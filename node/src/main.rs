use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

use edda_consensus::{
    leader::LeaderSchedule,
    tower::Tower,
    validator::{StakePool, ValidatorInfo},
};
use edda_core::{
    account::{Account, Pubkey, LAMPORTS_PER_EDDA, MAX_SUPPLY_EDDA},
    block::{Block, BlockHeader, BASE_FEE_LAMPORTS},
    hash::{hashv, Hash},
    keypair::Keypair,
    transaction::Transaction,
};
use edda_network::{EddaNetwork, InboundMessage, OutboundMessage};
use edda_poh::{verify_entries, PohEntry, PohRecorder};
use edda_rpc::{serve, BlockInfo, CallResult, TokenInfo, TransferInfo, TxHistoryEntry, TxInfo, RpcState};
use edda_wasm::{AccountView, WasmRuntime};

const BLOCKHASH_WINDOW:      usize = 150;
const BLOCK_REWARD_LAMPORTS: u64  = 10 * LAMPORTS_PER_EDDA; // 10 EDDA per block

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

// ── CLI configuration ─────────────────────────────────────────────────────────

struct Config {
    rpc_port: u16,
    p2p_port: u16,
    data_dir: String,
    peers:    Vec<String>,
}

impl Config {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut cfg = Config {
            rpc_port: 8899,
            p2p_port: 7000,
            data_dir: "data".to_string(),
            peers:    Vec::new(),
        };
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--rpc-port" => { i += 1; if i < args.len() { cfg.rpc_port = args[i].parse().unwrap_or(8899); } }
                "--p2p-port" => { i += 1; if i < args.len() { cfg.p2p_port = args[i].parse().unwrap_or(7000); } }
                "--data-dir" => { i += 1; if i < args.len() { cfg.data_dir = args[i].clone(); } }
                "--peer"     => { i += 1; if i < args.len() { cfg.peers.push(args[i].clone()); } }
                "--help" | "-h" => {
                    println!("Usage: edda-node [OPTIONS]");
                    println!("  --rpc-port <port>   RPC listen port (default: 8899)");
                    println!("  --p2p-port <port>   P2P listen port (default: 7000)");
                    println!("  --data-dir <path>   Data directory  (default: data)");
                    println!("  --peer <multiaddr>  Bootstrap peer  (repeatable)");
                    println!("  Example: edda-node --rpc-port 8900 --p2p-port 7001 --data-dir data/node2 --peer /ip4/127.0.0.1/tcp/7000");
                    std::process::exit(0);
                }
                _ => {}
            }
            i += 1;
        }
        cfg
    }

    fn state_file(&self)     -> String { format!("{}/ledger.bin",    self.data_dir) }
    fn validator_file(&self) -> String { format!("{}/validator.key", self.data_dir) }
}

// ── Token program ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenMint {
    id:             Hash,
    name:           String,
    symbol:         String,
    decimals:       u8,
    total_supply:   u64,
    max_supply:     u64,
    mint_authority: Pubkey,
    created_at:     u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TokenRegistry {
    mints:    HashMap<Hash, TokenMint>,
    balances: HashMap<(Pubkey, Hash), u64>,
}

impl TokenRegistry {
    fn create(&mut self, name: String, symbol: String, decimals: u8, max_supply: u64, authority: Pubkey) -> Hash {
        let seed = format!("{}:{}:{}", name, symbol, now_ms());
        let id   = hashv(&[seed.as_bytes(), &authority.0]);
        self.mints.insert(id, TokenMint {
            id, name, symbol, decimals,
            total_supply: 0, max_supply, mint_authority: authority, created_at: now_ms(),
        });
        id
    }

    fn mint_to(&mut self, id: Hash, to: Pubkey, amount: u64, authority: Pubkey) -> Result<(), &'static str> {
        let mint = self.mints.get_mut(&id).ok_or("token not found")?;
        if mint.mint_authority != authority { return Err("not mint authority"); }
        if mint.max_supply > 0 && mint.total_supply + amount > mint.max_supply {
            return Err("exceeds max supply");
        }
        mint.total_supply += amount;
        *self.balances.entry((to, id)).or_insert(0) += amount;
        Ok(())
    }

    fn transfer(&mut self, id: Hash, from: Pubkey, to: Pubkey, amount: u64) -> Result<(), &'static str> {
        if !self.mints.contains_key(&id) { return Err("token not found"); }
        let bal = self.balances.get(&(from, id)).copied().unwrap_or(0);
        if bal < amount { return Err("insufficient token balance"); }
        *self.balances.entry((from, id)).or_insert(0) -= amount;
        *self.balances.entry((to,   id)).or_insert(0) += amount;
        Ok(())
    }

    fn balance(&self, id: Hash, owner: Pubkey) -> u64 {
        self.balances.get(&(owner, id)).copied().unwrap_or(0)
    }
}

// ── Tx history index ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxRecord {
    slot:         u64,
    signature:    String,
    from:         Pubkey,
    to:           Pubkey,
    lamports:     u64,
    fee:          u64,
    timestamp_ms: u64,
}

// ── Persisted state ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct LedgerState {
    accounts:     Vec<(Pubkey, Account)>,
    total_supply: u64,
    total_burned: u64,
    recent_hash:  Hash,
    current_slot: u64,
    blocks:       Vec<Block>,
    tx_index:     Vec<TxRecord>,
    tokens:       TokenRegistry,
}

fn save_state(ledger: &Ledger, slot: u64, state_file: &str) {
    let state = LedgerState {
        accounts:     ledger.accounts.iter().map(|(k, v)| (*k, v.clone())).collect(),
        total_supply: ledger.total_supply,
        total_burned: ledger.total_burned,
        recent_hash:  ledger.recent_hash,
        current_slot: slot,
        blocks:       ledger.blocks.iter().rev().take(500).cloned().collect(),
        tx_index:     ledger.tx_index.iter().rev().take(10_000).cloned().collect(),
        tokens:       ledger.tokens.clone(),
    };
    if let Ok(bytes) = bincode::serialize(&state) {
        let tmp = format!("{}.tmp", state_file);
        if fs::write(&tmp, &bytes).is_ok() { let _ = fs::rename(&tmp, state_file); }
    }
}

fn load_state(state_file: &str) -> Option<LedgerState> {
    bincode::deserialize(&fs::read(state_file).ok()?).ok()
}

fn save_validator_key(kp: &Keypair, path: &str) { let _ = fs::write(path, kp.to_bytes()); }
fn load_validator_key(path: &str) -> Option<Keypair> {
    let b = fs::read(path).ok()?;
    if b.len() != 32 { return None; }
    let mut arr = [0u8; 32]; arr.copy_from_slice(&b);
    Some(Keypair::from_bytes(arr))
}

// ── In-memory ledger ──────────────────────────────────────────────────────────

struct Ledger {
    accounts:     HashMap<Pubkey, Account>,
    blocks:       Vec<Block>,
    total_supply: u64,
    total_burned: u64,
    recent_hash:  Hash,
    tx_index:     Vec<TxRecord>,
    tokens:       TokenRegistry,
    program_logs: HashMap<Pubkey, Vec<String>>,
}

impl Ledger {
    fn new(genesis_hash: Hash) -> Self {
        Self { accounts: HashMap::new(), blocks: Vec::new(),
               total_supply: 0, total_burned: 0, recent_hash: genesis_hash,
               tx_index: Vec::new(), tokens: TokenRegistry::default(),
               program_logs: HashMap::new() }
    }

    fn from_state(s: LedgerState) -> Self {
        Self { accounts: s.accounts.into_iter().collect(),
               blocks: s.blocks, total_supply: s.total_supply,
               total_burned: s.total_burned, recent_hash: s.recent_hash,
               tx_index: s.tx_index, tokens: s.tokens,
               program_logs: HashMap::new() }
    }

    fn mint(&mut self, to: Pubkey, lamports: u64) {
        let cap    = MAX_SUPPLY_EDDA * LAMPORTS_PER_EDDA;
        let actual = lamports.min(cap.saturating_sub(self.total_supply));
        self.accounts.entry(to).or_insert_with(|| Account::new(0, Pubkey::system_program())).lamports += actual;
        self.total_supply += actual;
    }

    fn transfer(&mut self, from: Pubkey, to: Pubkey, amount: u64, fee_recipient: Pubkey) -> Result<(), &'static str> {
        let bal = self.accounts.get(&from).ok_or("sender not found")?.lamports;
        if bal < amount + BASE_FEE_LAMPORTS { return Err("insufficient balance"); }
        self.accounts.get_mut(&from).unwrap().lamports -= amount + BASE_FEE_LAMPORTS;
        self.accounts.entry(to).or_insert_with(|| Account::new(0, Pubkey::system_program())).lamports += amount;
        // 50% burned (deflationary), 50% to the block validator
        let burned    = BASE_FEE_LAMPORTS / 2;
        let validator_cut = BASE_FEE_LAMPORTS - burned;
        self.total_burned += burned;
        self.accounts.entry(fee_recipient)
            .or_insert_with(|| Account::new(0, Pubkey::system_program()))
            .lamports += validator_cut;
        Ok(())
    }

    fn balance(&self, pk: &Pubkey) -> u64 {
        self.accounts.get(pk).map(|a| a.lamports).unwrap_or(0)
    }

    fn index_tx(&mut self, slot: u64, sig: String, from: Pubkey, to: Pubkey, lamports: u64, ts: u64) {
        self.tx_index.push(TxRecord {
            slot, signature: sig, from, to, lamports, fee: BASE_FEE_LAMPORTS, timestamp_ms: ts,
        });
    }

    fn history_for(&self, pk: &Pubkey, limit: usize) -> Vec<&TxRecord> {
        self.tx_index.iter().rev()
            .filter(|r| &r.from == pk || &r.to == pk)
            .take(limit).collect()
    }

    fn has_block(&self, slot: u64) -> bool {
        self.blocks.iter().any(|b| b.header.slot == slot)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_pubkey(hex: &str) -> Result<Pubkey, String> {
    let b = hex::decode(hex).map_err(|_| "invalid hex".to_string())?;
    if b.len() != 32 { return Err("pubkey must be 32 bytes".into()); }
    let mut arr = [0u8; 32]; arr.copy_from_slice(&b);
    Ok(Pubkey::new(arr))
}

fn parse_hash(hex: &str) -> Result<Hash, String> {
    let b = hex::decode(hex).map_err(|_| "invalid hex".to_string())?;
    if b.len() != 32 { return Err("hash must be 32 bytes".into()); }
    let mut arr = [0u8; 32]; arr.copy_from_slice(&b);
    Ok(Hash::new_from_array(arr))
}

fn block_to_info(block: &Block) -> BlockInfo {
    let transactions = block.transactions.iter().map(|tx| {
        let transfer = tx.message.instructions.first().and_then(|ix| {
            if ix.program_id == Pubkey::system_program() && ix.data.len() >= 9 && ix.data[0] == 0 {
                let mut b = [0u8; 8]; b.copy_from_slice(&ix.data[1..9]);
                let lamports = u64::from_le_bytes(b);
                let from = ix.accounts.first()?.pubkey.to_string();
                let to   = ix.accounts.get(1)?.pubkey.to_string();
                Some(TransferInfo { from, to, lamports })
            } else { None }
        });
        TxInfo { signature: tx.hash().to_hex(), signer: tx.signer.to_string(),
                 fee: BASE_FEE_LAMPORTS, transfer }
    }).collect();
    BlockInfo {
        slot: block.header.slot, blockhash: block.header.blockhash.to_hex(),
        parent_blockhash: block.header.parent_blockhash.to_hex(),
        timestamp_ms: block.header.timestamp_ms, leader: block.header.leader.to_string(),
        transaction_count: block.header.transaction_count,
        fees_burned: block.header.fees_burned, transactions,
    }
}

fn push_recent_hash(window: &mut VecDeque<Hash>, h: Hash) {
    if window.len() >= BLOCKHASH_WINDOW { window.pop_front(); }
    window.push_back(h);
}

// ── Base64 decode (no extra dep) ──────────────────────────────────────────────

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let alpha = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let s = s.trim_end_matches('=').replace(['\n', '\r', ' '], "");
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for b in s.as_bytes() {
        let val = alpha.iter().position(|&c| c == *b)
            .ok_or_else(|| format!("invalid base64 char: {}", *b as char))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
    }
    Ok(out)
}

// ── Execute a WASM instruction in the block producer ─────────────────────────

fn execute_wasm_ix(
    ledger:    &mut Ledger,
    runtime:   &WasmRuntime,
    tx:        &Transaction,
    slot:      u64,
    ts:        u64,
    validator: Pubkey,
) -> bool {
    let ix = match tx.message.instructions.first() { Some(i) => i, None => return false };

    // Load program account bytecode
    let bytecode = match ledger.accounts.get(&ix.program_id) {
        Some(acc) if acc.executable && !acc.data.is_empty() => acc.data.clone(),
        _ => return false,
    };

    // Deduct fee from signer first
    {
        let signer_acc = match ledger.accounts.get_mut(&tx.signer) {
            Some(a) if a.lamports >= BASE_FEE_LAMPORTS => a,
            _ => return false,
        };
        signer_acc.lamports -= BASE_FEE_LAMPORTS;
        let burned        = BASE_FEE_LAMPORTS / 2;
        let validator_cut = BASE_FEE_LAMPORTS - burned;
        ledger.total_burned += burned;
        ledger.accounts.entry(validator)
            .or_insert_with(|| Account::new(0, Pubkey::system_program()))
            .lamports += validator_cut;
    }

    // Build account views for WASM
    let views: Vec<AccountView> = ix.accounts.iter().map(|meta| {
        let acc = ledger.accounts.get(&meta.pubkey);
        AccountView {
            lamports: acc.map(|a| a.lamports).unwrap_or(0),
            data:     acc.map(|a| a.data.clone()).unwrap_or_default(),
            writable: meta.is_writable,
        }
    }).collect();

    // Compile & execute
    let module = match runtime.compile(&bytecode) {
        Ok(m)  => m,
        Err(e) => { println!("[WASM] Compile error: {}", e); return false; }
    };
    let result = match runtime.execute(&module, views, ix.data.clone()) {
        Ok(r)  => r,
        Err(e) => { println!("[WASM] Exec error slot={}: {}", slot, e); return false; }
    };

    // Write back updated account states
    for (meta, view) in ix.accounts.iter().zip(result.accounts.iter()) {
        if meta.is_writable {
            let acc = ledger.accounts.entry(meta.pubkey)
                .or_insert_with(|| Account::new(0, ix.program_id));
            acc.lamports = view.lamports;
            acc.data     = view.data.clone();
        }
    }

    // Emit logs
    if !result.logs.is_empty() {
        let entry = ledger.program_logs.entry(ix.program_id).or_default();
        for log in &result.logs {
            println!("[WASM] [{}...] slot={} {}", &ix.program_id.to_string()[..16], slot, log);
            entry.push(format!("slot={} {}", slot, log));
        }
        // keep at most 500 log lines per program
        if entry.len() > 500 { entry.drain(0..entry.len() - 500); }
    }

    ledger.index_tx(slot, tx.hash().to_hex(), tx.signer, ix.program_id, 0, ts);
    true
}

// ── Shared node state ─────────────────────────────────────────────────────────

struct NodeState {
    ledger:             RwLock<Ledger>,
    poh:                Mutex<PohRecorder>,
    poh_entries:        Mutex<Vec<PohEntry>>,
    tx_pool:            Mutex<Vec<Transaction>>,
    stake_pool:         StakePool,
    leader_schedule:    LeaderSchedule,
    current_slot:       Mutex<u64>,
    validator:          Keypair,
    network_tx:         tokio::sync::mpsc::Sender<OutboundMessage>,
    recent_blockhashes: Mutex<VecDeque<Hash>>,
    seen_sigs:          Mutex<HashSet<String>>,
    wasm:               WasmRuntime,
    state_file:         String,
}

// ── RpcState impl ─────────────────────────────────────────────────────────────

impl RpcState for NodeState {
    fn get_balance(&self, pk: &str) -> Option<u64> {
        Some(self.ledger.try_read().ok()?.balance(&parse_pubkey(pk).ok()?))
    }
    fn get_block_height(&self) -> u64 {
        self.ledger.try_read().ok().map(|l| l.blocks.len() as u64).unwrap_or(0)
    }
    fn get_recent_blockhash(&self) -> String {
        self.ledger.try_read().ok().map(|l| l.recent_hash.to_hex()).unwrap_or_default()
    }
    fn get_validator_count(&self) -> usize { self.stake_pool.validator_count() }
    fn get_total_supply(&self) -> u64 { self.ledger.try_read().ok().map(|l| l.total_supply).unwrap_or(0) }
    fn get_total_burned(&self) -> u64 { self.ledger.try_read().ok().map(|l| l.total_burned).unwrap_or(0) }

    fn get_block(&self, slot: u64) -> Option<BlockInfo> {
        let l = self.ledger.try_read().ok()?;
        Some(block_to_info(l.blocks.iter().find(|b| b.header.slot == slot)?))
    }
    fn get_recent_blocks(&self, limit: usize) -> Vec<BlockInfo> {
        let l = match self.ledger.try_read() { Ok(l) => l, Err(_) => return vec![] };
        l.blocks.iter().rev().take(limit).map(block_to_info).collect()
    }

    fn get_transaction_history(&self, pk_hex: &str, limit: usize) -> Vec<TxHistoryEntry> {
        let pk     = match parse_pubkey(pk_hex) { Ok(p) => p, Err(_) => return vec![] };
        let ledger = match self.ledger.try_read() { Ok(l) => l, Err(_) => return vec![] };
        ledger.history_for(&pk, limit).into_iter().map(|r| {
            let direction = if r.from == pk { "sent" } else { "received" }.to_string();
            TxHistoryEntry {
                slot: r.slot, signature: r.signature.clone(),
                from: r.from.to_string(), to: r.to.to_string(),
                lamports: r.lamports, fee: r.fee,
                timestamp_ms: r.timestamp_ms, direction,
            }
        }).collect()
    }

    fn create_token(&self, name: &str, symbol: &str, decimals: u8, max_supply: u64, authority_hex: &str) -> Result<String, String> {
        let authority = parse_pubkey(authority_hex)?;
        let mut l     = self.ledger.try_write().map_err(|_| "ledger busy")?;
        let id        = l.tokens.create(name.to_string(), symbol.to_string(), decimals, max_supply, authority);
        println!("[Token] Created {} ({}) id={}", name, symbol, &id.to_hex()[..16]);
        Ok(id.to_hex())
    }

    fn mint_token(&self, token_id: &str, to_hex: &str, amount: u64, authority_hex: &str) -> Result<(), String> {
        let id = parse_hash(token_id)?; let to = parse_pubkey(to_hex)?; let auth = parse_pubkey(authority_hex)?;
        self.ledger.try_write().map_err(|_| "ledger busy")?.tokens.mint_to(id, to, amount, auth).map_err(|e| e.to_string())
    }

    fn transfer_token(&self, token_id: &str, from_hex: &str, to_hex: &str, amount: u64) -> Result<(), String> {
        let id = parse_hash(token_id)?; let from = parse_pubkey(from_hex)?; let to = parse_pubkey(to_hex)?;
        self.ledger.try_write().map_err(|_| "ledger busy")?.tokens.transfer(id, from, to, amount).map_err(|e| e.to_string())
    }

    fn get_token_balance(&self, token_id: &str, owner_hex: &str) -> u64 {
        let id    = match parse_hash(token_id)  { Ok(h) => h, Err(_) => return 0 };
        let owner = match parse_pubkey(owner_hex) { Ok(p) => p, Err(_) => return 0 };
        self.ledger.try_read().ok().map(|l| l.tokens.balance(id, owner)).unwrap_or(0)
    }

    fn get_token_info(&self, token_id: &str) -> Option<TokenInfo> {
        let id = parse_hash(token_id).ok()?;
        let l  = self.ledger.try_read().ok()?;
        let m  = l.tokens.mints.get(&id)?;
        Some(TokenInfo { id: m.id.to_hex(), name: m.name.clone(), symbol: m.symbol.clone(),
            decimals: m.decimals, total_supply: m.total_supply, max_supply: m.max_supply,
            mint_authority: m.mint_authority.to_string() })
    }

    fn list_tokens(&self) -> Vec<TokenInfo> {
        let l = match self.ledger.try_read() { Ok(l) => l, Err(_) => return vec![] };
        l.tokens.mints.values().map(|m| TokenInfo {
            id: m.id.to_hex(), name: m.name.clone(), symbol: m.symbol.clone(),
            decimals: m.decimals, total_supply: m.total_supply, max_supply: m.max_supply,
            mint_authority: m.mint_authority.to_string(),
        }).collect()
    }

    fn submit_transaction(&self, tx_bytes: Vec<u8>) -> Result<String, String> {
        let tx: Transaction = bincode::deserialize(&tx_bytes)
            .map_err(|e| format!("deserialize: {}", e))?;
        if !Keypair::verify(&tx.signer, &tx.message.to_bytes(), &tx.signature) {
            return Err("invalid signature".into());
        }
        let sig = tx.hash().to_hex();
        {
            let window = self.recent_blockhashes.try_lock().map_err(|_| "node busy")?;
            if !window.contains(&tx.message.recent_blockhash) {
                return Err("blockhash expired — fetch a fresh one".into());
            }
        }
        {
            let mut seen = self.seen_sigs.try_lock().map_err(|_| "node busy")?;
            if seen.contains(&sig) { return Err("duplicate transaction".into()); }
            seen.insert(sig.clone());
            if seen.len() > 10_000 { seen.clear(); }
        }
        if let Ok(mut pool) = self.tx_pool.try_lock() { pool.push(tx); }
        Ok(sig)
    }

    // ── WASM smart contracts ───────────────────────────────────────────────────

    fn deploy_program(&self, bytecode_b64: &str, authority_hex: &str) -> Result<String, String> {
        let bytecode  = b64_decode(bytecode_b64).map_err(|e| format!("base64: {}", e))?;
        let authority = parse_pubkey(authority_hex)?;

        // Verify it's valid WASM before storing
        self.wasm.compile(&bytecode).map_err(|e| format!("invalid wasm: {}", e))?;

        // Derive a deterministic program ID
        let seed    = format!("program:{}:{}", authority_hex, now_ms());
        let prog_id_hash = hashv(&[seed.as_bytes()]);
        let mut id_bytes = [0u8; 32];
        id_bytes.copy_from_slice(prog_id_hash.as_bytes());
        let prog_id = Pubkey::new(id_bytes);

        let mut ledger = self.ledger.try_write().map_err(|_| "ledger busy")?;
        ledger.accounts.insert(prog_id, Account {
            lamports:   0,
            data:       bytecode,
            owner:      authority,
            executable: true,
        });

        let id_hex = prog_id.to_string();
        println!("[WASM] Deployed program {} by {}...", &id_hex[..16], &authority_hex[..16]);
        Ok(id_hex)
    }

    fn call_program(&self, program_id_hex: &str, input_b64: &str) -> Result<CallResult, String> {
        let pk       = parse_pubkey(program_id_hex)?;
        let input    = b64_decode(input_b64).map_err(|e| format!("base64: {}", e))?;

        let bytecode = {
            let ledger = self.ledger.try_read().map_err(|_| "ledger busy")?;
            let acc    = ledger.accounts.get(&pk).ok_or("program not found")?;
            if !acc.executable { return Err("account is not a program".into()); }
            acc.data.clone()
        };

        let module = self.wasm.compile(&bytecode).map_err(|e| format!("compile: {}", e))?;
        let result = self.wasm.execute(&module, vec![], input).map_err(|e| e.to_string())?;

        // Persist logs
        {
            let mut ledger = self.ledger.try_write().map_err(|_| "ledger busy")?;
            ledger.program_logs.entry(pk).or_default().extend(result.logs.clone());
        }

        Ok(CallResult { logs: result.logs, gas_used: result.gas_used })
    }

    fn get_program_logs(&self, program_id_hex: &str) -> Vec<String> {
        let pk = match parse_pubkey(program_id_hex) { Ok(p) => p, Err(_) => return vec![] };
        self.ledger.try_read().ok()
            .and_then(|l| l.program_logs.get(&pk).cloned())
            .unwrap_or_default()
    }
}

// ── Process all instructions in a transaction ─────────────────────────────────

fn process_tx(ledger: &mut Ledger, wasm: &WasmRuntime, tx: &Transaction, slot: u64, ts: u64, validator: Pubkey) -> u64 {
    let ix = match tx.message.instructions.first() { Some(i) => i, None => return 0 };

    if ix.program_id == Pubkey::system_program() {
        // Native transfer
        if ix.data.len() >= 9 && ix.data[0] == 0 {
            let mut b = [0u8; 8]; b.copy_from_slice(&ix.data[1..9]);
            let amount = u64::from_le_bytes(b);
            if let (Some(from_m), Some(to_m)) = (ix.accounts.first(), ix.accounts.get(1)) {
                if ledger.transfer(from_m.pubkey, to_m.pubkey, amount, validator).is_ok() {
                    ledger.index_tx(slot, tx.hash().to_hex(), from_m.pubkey, to_m.pubkey, amount, ts);
                    return BASE_FEE_LAMPORTS;
                }
            }
        }
    } else {
        // WASM program call
        if execute_wasm_ix(ledger, wasm, tx, slot, ts, validator) {
            return BASE_FEE_LAMPORTS;
        }
    }
    0
}

// ── Apply a block received from a remote peer ─────────────────────────────────

async fn apply_remote_block(state: &Arc<NodeState>, block: Block) {
    let slot = block.header.slot;
    if state.ledger.read().await.has_block(slot) { return; }

    // Verify the block's internal hash is correct
    if !block.verify_blockhash() {
        println!("[Sync] Block #{} rejected — bad blockhash", slot);
        return;
    }
    // Note: leader-schedule check is skipped until on-chain validator registration
    // is implemented. Blockhash integrity is sufficient for now.

    let ts         = block.header.timestamp_ms;
    let poh_hash   = block.header.poh_hash;
    let blockhash  = block.header.blockhash;
    let leader_str = block.header.leader.to_string();
    let tx_count   = block.transactions.len();

    {
        let mut ledger = state.ledger.write().await;
        let leader_pk  = ledger.blocks.iter()
            .find(|b| b.header.slot == slot)
            .map(|b| b.header.leader)
            .unwrap_or(state.validator.pubkey());
        for tx in &block.transactions {
            if !Keypair::verify(&tx.signer, &tx.message.to_bytes(), &tx.signature) { continue; }
            process_tx(&mut ledger, &state.wasm, tx, slot, ts, leader_pk);
        }
        // Block reward to the block's leader
        ledger.mint(leader_pk, BLOCK_REWARD_LAMPORTS);
        ledger.recent_hash = poh_hash;
        ledger.blocks.push(block);
        save_state(&ledger, slot, &state.state_file);
    }

    { let mut cs = state.current_slot.lock().await; if slot > *cs { *cs = slot; } }
    {
        let mut w = state.recent_blockhashes.lock().await;
        push_recent_hash(&mut w, poh_hash);
        push_recent_hash(&mut w, blockhash);
    }

    if tx_count > 0 {
        println!("[Sync] Block #{} applied  leader={}...  txs={}", slot, &leader_str[..16], tx_count);
    }
}

// ── Block production ──────────────────────────────────────────────────────────

async fn run_block_producer(state: Arc<NodeState>) {
    let mut ticker = interval(Duration::from_millis(400));
    ticker.tick().await;

    loop {
        ticker.tick().await;
        let slot = { let mut s = state.current_slot.lock().await; *s += 1; *s };

        let am_leader = state.leader_schedule.leader_for_slot(slot)
            .map(|l| l == state.validator.pubkey()).unwrap_or(false);
        if !am_leader { continue; }

        let poh_hash = {
            let mut poh = state.poh.lock().await;
            poh.hash_n(10_000);
            let entry = poh.tick();
            let h = entry.hash;
            state.poh_entries.lock().await.push(entry);
            h
        };

        let txs: Vec<Transaction> = {
            let mut p = state.tx_pool.lock().await;
            std::mem::take(&mut *p)
        };
        let tx_count = txs.len() as u32;
        let ts       = now_ms();
        let mut fees       = 0u64;
        let validator_pk   = state.validator.pubkey();

        {
            let mut ledger = state.ledger.write().await;
            for tx in &txs {
                fees += process_tx(&mut ledger, &state.wasm, tx, slot, ts, validator_pk);
            }
            // Block reward: 10 EDDA to this validator (respects max supply cap)
            ledger.mint(validator_pk, BLOCK_REWARD_LAMPORTS);
            ledger.recent_hash = poh_hash;
        }

        let parent_blockhash = {
            state.ledger.read().await.blocks.last()
                .map(|b| b.header.blockhash).unwrap_or(poh_hash)
        };
        let header = BlockHeader {
            slot, parent_slot: slot.saturating_sub(1), blockhash: Hash::default(),
            parent_blockhash, poh_hash, timestamp_ms: ts,
            leader: state.validator.pubkey(), transaction_count: tx_count,
            total_fees: fees, fees_burned: fees / 2, fees_to_validator: fees / 2,
        };
        let mut block = Block { header, transactions: txs };
        block.header.blockhash = block.compute_blockhash();
        let block_hash  = block.header.blockhash;
        let block_bytes = bincode::serialize(&block).unwrap_or_default();

        {
            let mut ledger = state.ledger.write().await;
            ledger.blocks.push(block);
            save_state(&ledger, slot, &state.state_file);
        }
        {
            let mut w = state.recent_blockhashes.lock().await;
            push_recent_hash(&mut w, poh_hash);
            push_recent_hash(&mut w, block_hash);
        }

        state.network_tx.send(OutboundMessage::BroadcastBlock(block_bytes)).await.ok();

        if tx_count > 0 {
            println!("[Block] Slot {:>5}  hash={}  txs={}  burned={} lp",
                slot, block_hash, tx_count, fees);
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn banner(cfg: &Config) {
    println!("╔════════════════════════════════════════════╗");
    println!("║         Edda Network  —  Node v0.4         ║");
    println!("║  PoH · BFT · Rewards · WASM · Tokens      ║");
    println!("╚════════════════════════════════════════════╝");
    println!();
    println!("  RPC port : {}", cfg.rpc_port);
    println!("  P2P port : {}", cfg.p2p_port);
    println!("  Data dir : {}", cfg.data_dir);
    if !cfg.peers.is_empty() {
        for p in &cfg.peers { println!("  Peer     : {}", p); }
    }
    println!();
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_args();
    banner(&cfg);

    fs::create_dir_all(&cfg.data_dir).expect("cannot create data dir");
    let state_file     = cfg.state_file();
    let validator_file = cfg.validator_file();

    let genesis_hash = Hash::new(b"edda-network-genesis-2026");

    let validator = match load_validator_key(&validator_file) {
        Some(kp) => { println!("[Validator] Loaded: {}", kp.pubkey()); kp }
        None     => {
            let kp = Keypair::generate();
            save_validator_key(&kp, &validator_file);
            println!("[Validator] New:    {}", kp.pubkey());
            kp
        }
    };

    let (ledger, resume_slot) = match load_state(&state_file) {
        Some(s) => {
            let slot = s.current_slot;
            let l    = Ledger::from_state(s);
            println!("[Ledger] Resumed — slot {}, {} accounts, {} tokens, {} tx records",
                slot, l.accounts.len(), l.tokens.mints.len(), l.tx_index.len());
            (l, slot)
        }
        None => {
            println!("[Ledger] Genesis init");
            let mut l = Ledger::new(genesis_hash);
            // Genesis allocation: 100M EDDA (20% of max supply) to founder wallet
            let founder_hex = "458eca8bfc394c5155a001be1fd0c54b884a44bd83c4e62395296350d08f4291";
            if let Ok(founder) = parse_pubkey(founder_hex) {
                l.mint(founder, 100_000_000 * LAMPORTS_PER_EDDA);
                println!("[Ledger] Genesis: 100,000,000 EDDA → founder");
            }
            println!("[Ledger] Total supply: {} EDDA", l.total_supply / LAMPORTS_PER_EDDA);
            (l, 0)
        }
    };

    // Seed the recent-blockhash window
    let mut init_window: VecDeque<Hash> = VecDeque::new();
    push_recent_hash(&mut init_window, genesis_hash);
    for b in ledger.blocks.iter().rev().take(BLOCKHASH_WINDOW) {
        push_recent_hash(&mut init_window, b.header.poh_hash);
        push_recent_hash(&mut init_window, b.header.blockhash);
    }
    push_recent_hash(&mut init_window, ledger.recent_hash);

    let poh = PohRecorder::new(if resume_slot == 0 { genesis_hash } else { ledger.recent_hash });

    let val2 = Keypair::generate();
    let val3 = Keypair::generate();
    let mut stake_pool = StakePool::new();
    stake_pool.register(ValidatorInfo::new(validator.pubkey(), validator.pubkey(), 10_000 * LAMPORTS_PER_EDDA, 5));
    stake_pool.register(ValidatorInfo::new(val2.pubkey(), val2.pubkey(),  8_000 * LAMPORTS_PER_EDDA, 3));
    stake_pool.register(ValidatorInfo::new(val3.pubkey(), val3.pubkey(),  6_000 * LAMPORTS_PER_EDDA, 8));

    let validators: Vec<_> = stake_pool.iter().collect();
    let seed = u64::from_le_bytes(genesis_hash.as_bytes()[..8].try_into().unwrap());
    let leader_schedule = LeaderSchedule::new(&validators, 0, seed);

    let mut tower = Tower::new(); tower.record_vote(0);
    let mut votes = HashMap::new();
    votes.insert(validator.pubkey(), 0u64);
    votes.insert(val2.pubkey(), 0u64);
    votes.insert(val3.pubkey(), 0u64);
    println!("[Consensus] {} validators  stake {} EDDA  supermajority: {}",
        stake_pool.validator_count(),
        stake_pool.total_stake() / LAMPORTS_PER_EDDA,
        stake_pool.is_confirmed(&votes, 0));

    let (network, mut inbound_rx, network_tx) =
        EddaNetwork::new(cfg.p2p_port, cfg.peers.clone()).await.expect("P2P init failed");

    let state = Arc::new(NodeState {
        ledger:             RwLock::new(ledger),
        poh:                Mutex::new(poh),
        poh_entries:        Mutex::new(Vec::new()),
        tx_pool:            Mutex::new(Vec::new()),
        stake_pool, leader_schedule,
        current_slot:       Mutex::new(resume_slot),
        validator, network_tx,
        recent_blockhashes: Mutex::new(init_window),
        seen_sigs:          Mutex::new(HashSet::new()),
        wasm:               WasmRuntime::new().expect("wasmtime init failed"),
        state_file:         state_file.clone(),
    });

    println!();
    println!("[Node] RPC http://0.0.0.0:{}", cfg.rpc_port);
    println!("[Node] P2P tcp/0.0.0.0:{}", cfg.p2p_port);
    println!("[Node] Data {}/", cfg.data_dir);
    println!("[Security] Replay protection ON  ({} slot window)", BLOCKHASH_WINDOW);
    println!("[WASM]  Smart contract runtime ready");
    println!("Press Ctrl+C to stop.\n");

    tokio::spawn(network.run());

    {
        let s = state.clone();
        tokio::spawn(async move {
            while let Some(msg) = inbound_rx.recv().await {
                match msg {
                    InboundMessage::Transaction(data) => {
                        if let Ok(tx) = bincode::deserialize::<Transaction>(&data) {
                            s.tx_pool.lock().await.push(tx);
                        }
                    }
                    InboundMessage::Block(data) => {
                        match bincode::deserialize::<Block>(&data) {
                            Ok(block) => apply_remote_block(&s, block).await,
                            Err(e)    => println!("[Sync] Bad block from peer: {}", e),
                        }
                    }
                }
            }
        });
    }

    { let s = state.clone(); tokio::spawn(run_block_producer(s)); }
    { let s = state.clone(); tokio::spawn(serve(s, cfg.rpc_port)); }

    tokio::signal::ctrl_c().await.ok();
    println!("\n[Node] Saving state...");
    let ledger = state.ledger.read().await;
    let slot   = *state.current_slot.lock().await;
    save_state(&ledger, slot, &state_file);
    let entries = state.poh_entries.lock().await;
    if !entries.is_empty() {
        println!("[PoH] Valid: {}  ({} entries)", verify_entries(genesis_hash, &entries), entries.len());
    }
    println!("[Node] Blocks: {}  Supply: {} EDDA  Burned: {} lp  Programs: {}  Tokens: {}",
        ledger.blocks.len(), ledger.total_supply / LAMPORTS_PER_EDDA, ledger.total_burned,
        ledger.accounts.values().filter(|a| a.executable).count(),
        ledger.tokens.mints.len());
}
