use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

// ── Public data types returned by the RPC ────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferInfo {
    pub from:     String,
    pub to:       String,
    pub lamports: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxInfo {
    pub signature: String,
    pub signer:    String,
    pub fee:       u64,
    pub transfer:  Option<TransferInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxHistoryEntry {
    pub slot:         u64,
    pub signature:    String,
    pub from:         String,
    pub to:           String,
    pub lamports:     u64,
    pub fee:          u64,
    pub timestamp_ms: u64,
    pub direction:    String, // "sent" | "received"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenInfo {
    pub id:           String,
    pub name:         String,
    pub symbol:       String,
    pub decimals:     u8,
    pub total_supply: u64,
    pub max_supply:   u64,
    pub mint_authority: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockInfo {
    pub slot:              u64,
    pub blockhash:         String,
    pub parent_blockhash:  String,
    pub timestamp_ms:      u64,
    pub leader:            String,
    pub transaction_count: u32,
    pub fees_burned:       u64,
    pub transactions:      Vec<TxInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallResult {
    pub logs:     Vec<String>,
    pub gas_used: u64,
}

// ── Trait that the node implements ────────────────────────────────────────────

pub trait RpcState: Send + Sync + 'static {
    fn get_balance(&self, pubkey_hex: &str) -> Option<u64>;
    fn get_block_height(&self) -> u64;
    fn get_recent_blockhash(&self) -> String;
    fn get_validator_count(&self) -> usize;
    fn get_total_supply(&self) -> u64;
    fn get_total_burned(&self) -> u64;
    fn submit_transaction(&self, tx_bytes: Vec<u8>) -> Result<String, String>;
    fn get_block(&self, slot: u64) -> Option<BlockInfo>;
    fn get_recent_blocks(&self, limit: usize) -> Vec<BlockInfo>;
    fn get_transaction_history(&self, pubkey_hex: &str, limit: usize) -> Vec<TxHistoryEntry>;
    // Token program
    fn create_token(&self, name: &str, symbol: &str, decimals: u8, max_supply: u64, authority_hex: &str) -> Result<String, String>;
    fn mint_token(&self, token_id: &str, to_hex: &str, amount: u64, authority_hex: &str) -> Result<(), String>;
    fn transfer_token(&self, token_id: &str, from_hex: &str, to_hex: &str, amount: u64) -> Result<(), String>;
    fn get_token_balance(&self, token_id: &str, owner_hex: &str) -> u64;
    fn get_token_info(&self, token_id: &str) -> Option<TokenInfo>;
    fn list_tokens(&self) -> Vec<TokenInfo>;
    // WASM smart contracts
    fn deploy_program(&self, bytecode_b64: &str, authority_hex: &str) -> Result<String, String>;
    fn call_program(&self, program_id_hex: &str, input_b64: &str) -> Result<CallResult, String>;
    fn get_program_logs(&self, program_id_hex: &str) -> Vec<String>;
}

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id:      Value,
    pub method:  String,
    #[serde(default)]
    pub params:  Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id:      Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code:    i32,
    pub message: String,
}

impl RpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0", id, result: None,
               error: Some(RpcError { code, message: message.into() }) }
    }
}

// ── RPC handler ───────────────────────────────────────────────────────────────

async fn health() -> StatusCode { StatusCode::OK }

async fn handle_rpc<S: RpcState>(
    State(state): State<Arc<S>>,
    Json(req): Json<RpcRequest>,
) -> Json<RpcResponse> {
    let id = req.id.clone();

    let resp = match req.method.as_str() {
        "getBalance" => {
            let pk = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            match state.get_balance(pk) {
                Some(l) => RpcResponse::ok(id, json!({ "lamports": l })),
                None    => RpcResponse::err(id, -32602, "account not found"),
            }
        }

        "getBlockHeight" => RpcResponse::ok(id, json!(state.get_block_height())),

        "getRecentBlockhash" => RpcResponse::ok(id, json!({ "blockhash": state.get_recent_blockhash() })),

        "getNetworkInfo" => RpcResponse::ok(id, json!({
            "validators":   state.get_validator_count(),
            "total_supply": state.get_total_supply(),
            "total_burned": state.get_total_burned(),
        })),

        "sendTransaction" => {
            let b64 = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            match BASE64.decode(b64) {
                Ok(bytes) => match state.submit_transaction(bytes) {
                    Ok(sig)  => RpcResponse::ok(id, json!({ "signature": sig })),
                    Err(msg) => RpcResponse::err(id, -32000, msg),
                },
                Err(_) => RpcResponse::err(id, -32602, "invalid base64"),
            }
        }

        // getTransactionHistory(pubkey_hex, limit?) -> Vec<TxHistoryEntry>
        "getTransactionHistory" => {
            let pk    = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            let limit = req.params.get(1).and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            let hist  = state.get_transaction_history(pk, limit.min(200));
            RpcResponse::ok(id, serde_json::to_value(hist).unwrap())
        }

        // createToken(name, symbol, decimals, max_supply, authority_hex) -> token_id
        "createToken" => {
            let p = &req.params;
            let name      = p.first().and_then(|v| v.as_str()).unwrap_or("");
            let symbol    = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let decimals  = p.get(2).and_then(|v| v.as_u64()).unwrap_or(9) as u8;
            let max_sup   = p.get(3).and_then(|v| v.as_u64()).unwrap_or(0);
            let authority = p.get(4).and_then(|v| v.as_str()).unwrap_or("");
            match state.create_token(name, symbol, decimals, max_sup, authority) {
                Ok(id_hex) => RpcResponse::ok(id, json!({ "token_id": id_hex })),
                Err(msg)   => RpcResponse::err(id, -32000, msg),
            }
        }

        // mintToken(token_id, to_hex, amount, authority_hex)
        "mintToken" => {
            let p = &req.params;
            let token_id  = p.first().and_then(|v| v.as_str()).unwrap_or("");
            let to        = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let amount    = p.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
            let authority = p.get(3).and_then(|v| v.as_str()).unwrap_or("");
            match state.mint_token(token_id, to, amount, authority) {
                Ok(())   => RpcResponse::ok(id, json!({ "success": true })),
                Err(msg) => RpcResponse::err(id, -32000, msg),
            }
        }

        // transferToken(token_id, from_hex, to_hex, amount)
        "transferToken" => {
            let p = &req.params;
            let token_id = p.first().and_then(|v| v.as_str()).unwrap_or("");
            let from     = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let to       = p.get(2).and_then(|v| v.as_str()).unwrap_or("");
            let amount   = p.get(3).and_then(|v| v.as_u64()).unwrap_or(0);
            match state.transfer_token(token_id, from, to, amount) {
                Ok(())   => RpcResponse::ok(id, json!({ "success": true })),
                Err(msg) => RpcResponse::err(id, -32000, msg),
            }
        }

        // getTokenBalance(token_id, owner_hex)
        "getTokenBalance" => {
            let token_id = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            let owner    = req.params.get(1).and_then(|v| v.as_str()).unwrap_or("");
            RpcResponse::ok(id, json!({ "balance": state.get_token_balance(token_id, owner) }))
        }

        // getTokenInfo(token_id)
        "getTokenInfo" => {
            let token_id = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            match state.get_token_info(token_id) {
                Some(t) => RpcResponse::ok(id, serde_json::to_value(t).unwrap()),
                None    => RpcResponse::err(id, -32602, "token not found"),
            }
        }

        // listTokens()
        "listTokens" => {
            RpcResponse::ok(id, serde_json::to_value(state.list_tokens()).unwrap())
        }

        // getBlock(slot) -> BlockInfo
        "getBlock" => {
            let slot = req.params.first().and_then(|v| v.as_u64()).unwrap_or(0);
            match state.get_block(slot) {
                Some(b) => RpcResponse::ok(id, serde_json::to_value(b).unwrap()),
                None    => RpcResponse::err(id, -32602, "block not found"),
            }
        }

        // getRecentBlocks(limit?) -> Vec<BlockInfo>
        "getRecentBlocks" => {
            let limit = req.params.first().and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let blocks = state.get_recent_blocks(limit.min(100));
            RpcResponse::ok(id, serde_json::to_value(blocks).unwrap())
        }

        // deployProgram(bytecode_base64, authority_hex) -> program_id
        "deployProgram" => {
            let bytecode  = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            let authority = req.params.get(1).and_then(|v| v.as_str()).unwrap_or("");
            match state.deploy_program(bytecode, authority) {
                Ok(id_hex) => RpcResponse::ok(id, json!({ "program_id": id_hex })),
                Err(msg)   => RpcResponse::err(id, -32000, msg),
            }
        }

        // callProgram(program_id_hex, input_data_base64) -> { logs, gas_used }
        "callProgram" => {
            let prog_id  = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            let input_b64 = req.params.get(1).and_then(|v| v.as_str()).unwrap_or("");
            match state.call_program(prog_id, input_b64) {
                Ok(r)    => RpcResponse::ok(id, serde_json::to_value(r).unwrap()),
                Err(msg) => RpcResponse::err(id, -32000, msg),
            }
        }

        // getProgramLogs(program_id_hex) -> [string]
        "getProgramLogs" => {
            let pid  = req.params.first().and_then(|v| v.as_str()).unwrap_or("");
            let logs = state.get_program_logs(pid);
            RpcResponse::ok(id, serde_json::to_value(logs).unwrap())
        }

        _ => RpcResponse::err(id, -32601, format!("method '{}' not found", req.method)),
    };

    Json(resp)
}

// ── Minimal base64 decoder ────────────────────────────────────────────────────

struct Base64;
const BASE64: Base64 = Base64;
impl Base64 {
    fn decode(&self, s: &str) -> Result<Vec<u8>, ()> {
        let alpha = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let s = s.trim_end_matches('=');
        let mut buf: u32 = 0;
        let mut bits = 0u32;
        for &b in s.as_bytes() {
            let val = alpha.iter().position(|&c| c == b).ok_or(())? as u32;
            buf = (buf << 6) | val;
            bits += 6;
            if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); }
        }
        Ok(out)
    }
}

// ── Server ────────────────────────────────────────────────────────────────────

pub async fn serve<S: RpcState>(state: Arc<S>, port: u16) {
    let app = Router::new()
        .route("/",       post(handle_rpc::<S>))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr     = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("RPC bind failed");
    println!("[RPC] Listening on http://{}", addr);
    axum::serve(listener, app).await.expect("RPC server error");
}
