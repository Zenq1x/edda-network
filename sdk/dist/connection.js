"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Connection = void 0;
// ── JSON-RPC 2.0 client ───────────────────────────────────────────────────────
class Connection {
    constructor(endpoint) {
        this.nextId = 1;
        this.url = endpoint;
    }
    async rpc(method, params = []) {
        const id = this.nextId++;
        const body = JSON.stringify({ jsonrpc: '2.0', id, method, params });
        const res = await fetch(this.url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body,
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}: ${res.statusText}`);
        }
        const json = await res.json();
        if (json.error) {
            throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
        }
        return json.result;
    }
    /** Get the lamport balance of an account */
    async getBalance(pubkey) {
        const res = await this.rpc('getBalance', [pubkey.toHex()]);
        return BigInt(res.lamports);
    }
    /** Get the current block height (slot number) */
    async getBlockHeight() {
        const res = await this.rpc('getBlockHeight');
        return BigInt(res);
    }
    /** Get the most recent blockhash — required to build a transaction */
    async getRecentBlockhash() {
        return this.rpc('getRecentBlockhash');
    }
    /** Get global network statistics */
    async getNetworkInfo() {
        const res = await this.rpc('getNetworkInfo');
        return {
            validators: res.validators,
            total_supply: BigInt(res.total_supply),
            total_burned: BigInt(res.total_burned),
        };
    }
    /**
     * Send a signed transaction to the network.
     * Returns the transaction signature (hash) as a hex string.
     */
    async sendTransaction(tx) {
        const b64 = tx.toBase64();
        const res = await this.rpc('sendTransaction', [b64]);
        return res.signature;
    }
    /** Check that the node is alive */
    async healthCheck() {
        const res = await fetch(this.url.replace(/\/$/, '') + '/health');
        return res.ok;
    }
}
exports.Connection = Connection;
//# sourceMappingURL=connection.js.map