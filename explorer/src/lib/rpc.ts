const RPC_URL = '/api/rpc';
let id = 1;

async function call<T>(method: string, params: unknown[] = []): Promise<T> {
  const res = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: id++, method, params }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result as T;
}

export interface TransferInfo { from: string; to: string; lamports: number; }
export interface TxInfo { signature: string; signer: string; fee: number; transfer: TransferInfo | null; }
export interface BlockInfo {
  slot: number; blockhash: string; parent_blockhash: string;
  timestamp_ms: number; leader: string;
  transaction_count: number; fees_burned: number;
  transactions: TxInfo[];
}
export interface NetworkInfo { validators: number; total_supply: number; total_burned: number; }

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

export const getNetworkInfo     = () => call<NetworkInfo>('getNetworkInfo');
export const getBlockHeight     = () => call<number>('getBlockHeight');
export const getRecentBlockhash = () => call<{ blockhash: string }>('getRecentBlockhash');
export const getBalance         = (pk: string) => call<{ lamports: number }>('getBalance', [pk]).then(r => r.lamports);
export const getBlock           = (slot: number) => call<BlockInfo>('getBlock', [slot]);
export const getRecentBlocks    = (limit = 20) => call<BlockInfo[]>('getRecentBlocks', [limit]);
export const getTxHistory       = (pk: string, limit = 50) => call<TxHistoryEntry[]>('getTransactionHistory', [pk, limit]);
export const listTokens         = () => call<TokenInfo[]>('listTokens');
export const getTokenInfo       = (id: string) => call<TokenInfo>('getTokenInfo', [id]);
export const getTokenBalance    = (tokenId: string, owner: string) => call<{ balance: number }>('getTokenBalance', [tokenId, owner]).then(r => r.balance);
