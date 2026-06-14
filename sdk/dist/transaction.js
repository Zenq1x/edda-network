"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Transaction = exports.SystemProgram = exports.BASE_FEE_LAMPORTS = exports.LAMPORTS_PER_EDDA = void 0;
const publickey_1 = require("./publickey");
const bincode_1 = require("./bincode");
exports.LAMPORTS_PER_EDDA = 1000000000n;
exports.BASE_FEE_LAMPORTS = 5000n;
// ── System program helpers ────────────────────────────────────────────────────
class SystemProgram {
    /** Create a SOL-style transfer instruction */
    static transfer(params) {
        // data = [0x00, <lamports as u64 LE>]
        const data = new Uint8Array(9);
        data[0] = 0; // instruction index 0 = Transfer
        const view = new DataView(data.buffer);
        view.setBigUint64(1, params.lamports, true);
        return {
            programId: SystemProgram.PROGRAM_ID,
            accounts: [
                { pubkey: params.fromPubkey, isSigner: true, isWritable: true },
                { pubkey: params.toPubkey, isSigner: false, isWritable: true },
            ],
            data,
        };
    }
}
exports.SystemProgram = SystemProgram;
SystemProgram.PROGRAM_ID = publickey_1.PublicKey.SYSTEM_PROGRAM;
// ── Serialization ─────────────────────────────────────────────────────────────
function serializeMessage(feePayer, recentBlockhash, instructions) {
    const w = new bincode_1.BincodeWriter();
    // fee_payer: Pubkey (32 bytes)
    w.writeBytes(feePayer.bytes);
    // recent_blockhash: Hash (32 bytes)
    const hashBytes = new Uint8Array(32);
    const hashHex = recentBlockhash.replace(/^0x/, '');
    for (let i = 0; i < Math.min(32, hashHex.length / 2); i++) {
        hashBytes[i] = parseInt(hashHex.slice(i * 2, i * 2 + 2), 16);
    }
    w.writeBytes(hashBytes);
    // instructions: Vec<Instruction>
    w.writeSeq(instructions, (w, ix) => {
        // program_id
        w.writeBytes(ix.programId.bytes);
        // accounts: Vec<AccountMeta>
        w.writeSeq(ix.accounts, (w, am) => {
            w.writeBytes(am.pubkey.bytes);
            w.writeBool(am.isSigner);
            w.writeBool(am.isWritable);
        });
        // data: Vec<u8>
        w.writeByteVec(ix.data);
    });
    return w.toBytes();
}
// ── Transaction ───────────────────────────────────────────────────────────────
class Transaction {
    constructor() {
        this.recentBlockhash = '';
        this.feePayer = null;
        this.instructions = [];
        this._signature = null;
        this._signer = null;
    }
    add(ix) {
        this.instructions.push(ix);
        return this;
    }
    /** Sign the transaction with a Keypair. Must set feePayer + recentBlockhash first. */
    sign(keypair) {
        if (!this.feePayer)
            throw new Error('feePayer not set');
        if (!this.recentBlockhash)
            throw new Error('recentBlockhash not set');
        const msgBytes = serializeMessage(this.feePayer, this.recentBlockhash, this.instructions);
        this._signature = keypair.sign(msgBytes);
        this._signer = keypair.publicKey;
    }
    /** Serialize the signed transaction to bincode bytes for sending via RPC */
    serialize() {
        if (!this._signature || !this._signer || !this.feePayer) {
            throw new Error('Transaction must be signed before serializing');
        }
        const msgBytes = serializeMessage(this.feePayer, this.recentBlockhash, this.instructions);
        const w = new bincode_1.BincodeWriter();
        // message (inline — bincode serializes structs field by field)
        w.writeBytes(msgBytes);
        // signature: Vec<u8>
        w.writeByteVec(this._signature);
        // signer: Pubkey (32 bytes)
        w.writeBytes(this._signer.bytes);
        return w.toBytes();
    }
    /** Base64-encode the serialized transaction (what sendTransaction RPC expects) */
    toBase64() {
        return Buffer.from(this.serialize()).toString('base64');
    }
}
exports.Transaction = Transaction;
//# sourceMappingURL=transaction.js.map