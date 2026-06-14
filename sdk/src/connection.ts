import { Transaction } from './transaction';
import { PublicKey } from './publickey';

export interface NetworkInfo {
  validators:   number;
  total_supply: bigint;
  total_burned: bigint;
}

export interface BlockhashResult {
  blockhash: string;
}

// ── JSON-RPC 2.0 client ───────────────────────────────────────────────────────

export class Connection {
  private url:    string;
  private nextId: number = 1;

  constructor(endpoint: string) {
    this.url = endpoint;
  }

  private async rpc<T>(method: string, params: unknown[] = []): Promise<T> {
    const id = this.nextId++;
    const body = JSON.stringify({ jsonrpc: '2.0', id, method, params });

    const res = await fetch(this.url, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });

    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    }

    const json = await res.json() as {
      result?: T;
      error?:  { code: number; message: string };
    };

    if (json.error) {
      throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
    }

    return json.result as T;
  }

  /** Get the lamport balance of an account */
  async getBalance(pubkey: PublicKey): Promise<bigint> {
    const res = await this.rpc<{ lamports: number }>('getBalance', [pubkey.toHex()]);
    return BigInt(res.lamports);
  }

  /** Get the current block height (slot number) */
  async getBlockHeight(): Promise<bigint> {
    const res = await this.rpc<number>('getBlockHeight');
    return BigInt(res);
  }

  /** Get the most recent blockhash — required to build a transaction */
  async getRecentBlockhash(): Promise<BlockhashResult> {
    return this.rpc<BlockhashResult>('getRecentBlockhash');
  }

  /** Get global network statistics */
  async getNetworkInfo(): Promise<NetworkInfo> {
    const res = await this.rpc<{
      validators:   number;
      total_supply: number;
      total_burned: number;
    }>('getNetworkInfo');
    return {
      validators:   res.validators,
      total_supply: BigInt(res.total_supply),
      total_burned: BigInt(res.total_burned),
    };
  }

  /**
   * Send a signed transaction to the network.
   * Returns the transaction signature (hash) as a hex string.
   */
  async sendTransaction(tx: Transaction): Promise<string> {
    const b64 = tx.toBase64();
    const res  = await this.rpc<{ signature: string }>('sendTransaction', [b64]);
    return res.signature;
  }

  /** Check that the node is alive */
  async healthCheck(): Promise<boolean> {
    const res = await fetch(this.url.replace(/\/$/, '') + '/health');
    return res.ok;
  }
}
