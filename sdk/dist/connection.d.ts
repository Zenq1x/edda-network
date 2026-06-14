import { Transaction } from './transaction';
import { PublicKey } from './publickey';
export interface NetworkInfo {
    validators: number;
    total_supply: bigint;
    total_burned: bigint;
}
export interface BlockhashResult {
    blockhash: string;
}
export declare class Connection {
    private url;
    private nextId;
    constructor(endpoint: string);
    private rpc;
    /** Get the lamport balance of an account */
    getBalance(pubkey: PublicKey): Promise<bigint>;
    /** Get the current block height (slot number) */
    getBlockHeight(): Promise<bigint>;
    /** Get the most recent blockhash — required to build a transaction */
    getRecentBlockhash(): Promise<BlockhashResult>;
    /** Get global network statistics */
    getNetworkInfo(): Promise<NetworkInfo>;
    /**
     * Send a signed transaction to the network.
     * Returns the transaction signature (hash) as a hex string.
     */
    sendTransaction(tx: Transaction): Promise<string>;
    /** Check that the node is alive */
    healthCheck(): Promise<boolean>;
}
//# sourceMappingURL=connection.d.ts.map