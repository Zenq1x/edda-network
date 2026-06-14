import { PublicKey } from './publickey';
import { Keypair } from './keypair';
export declare const LAMPORTS_PER_EDDA = 1000000000n;
export declare const BASE_FEE_LAMPORTS = 5000n;
export interface AccountMeta {
    pubkey: PublicKey;
    isSigner: boolean;
    isWritable: boolean;
}
export interface Instruction {
    programId: PublicKey;
    accounts: AccountMeta[];
    data: Uint8Array;
}
export declare class SystemProgram {
    static readonly PROGRAM_ID: PublicKey;
    /** Create a SOL-style transfer instruction */
    static transfer(params: {
        fromPubkey: PublicKey;
        toPubkey: PublicKey;
        lamports: bigint;
    }): Instruction;
}
export declare class Transaction {
    recentBlockhash: string;
    feePayer: PublicKey | null;
    private instructions;
    private _signature;
    private _signer;
    add(ix: Instruction): this;
    /** Sign the transaction with a Keypair. Must set feePayer + recentBlockhash first. */
    sign(keypair: Keypair): void;
    /** Serialize the signed transaction to bincode bytes for sending via RPC */
    serialize(): Uint8Array;
    /** Base64-encode the serialized transaction (what sendTransaction RPC expects) */
    toBase64(): string;
}
//# sourceMappingURL=transaction.d.ts.map