let id = 1;

async function call<T>(method: string, params: unknown[] = []): Promise<T> {
  const res  = await fetch('/api/rpc', {
    method:  'POST',
    headers: { 'Content-Type': 'application/json' },
    body:    JSON.stringify({ jsonrpc: '2.0', id: id++, method, params }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result as T;
}

export interface TxHistoryEntry {
  slot: number; signature: string;
  from: string; to: string;
  lamports: number; fee: number;
  timestamp_ms: number; direction: 'sent' | 'received';
}

export interface TokenInfo {
  id: string; name: string; symbol: string;
  decimals: number; total_supply: number; max_supply: number;
  mint_authority: string;
}

export async function getBalance(pubkeyHex: string): Promise<bigint> {
  const r = await call<{ lamports: number }>('getBalance', [pubkeyHex]);
  return BigInt(r.lamports);
}

export async function getRecentBlockhash(): Promise<string> {
  const r = await call<{ blockhash: string }>('getRecentBlockhash');
  return r.blockhash;
}

export async function getBlockHeight(): Promise<number> {
  return call<number>('getBlockHeight');
}

export async function sendTransaction(base64Tx: string): Promise<string> {
  const r = await call<{ signature: string }>('sendTransaction', [base64Tx]);
  return r.signature;
}

export async function requestFaucet(pubkeyHex: string): Promise<bigint> {
  const r = await call<{ lamports: number }>('faucet', [pubkeyHex]);
  return BigInt(r.lamports);
}

export async function getTxHistory(pubkeyHex: string, limit = 50): Promise<TxHistoryEntry[]> {
  return call<TxHistoryEntry[]>('getTransactionHistory', [pubkeyHex, limit]);
}

export async function listTokens(): Promise<TokenInfo[]> {
  return call<TokenInfo[]>('listTokens');
}

export async function getTokenBalance(tokenId: string, ownerHex: string): Promise<number> {
  const r = await call<{ balance: number }>('getTokenBalance', [tokenId, ownerHex]);
  return r.balance;
}

export async function isNodeAlive(): Promise<boolean> {
  try { const r = await fetch('/api/rpc/health'); return r.ok; } catch { return false; }
}
