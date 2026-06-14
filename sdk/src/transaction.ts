import { PublicKey } from './publickey';
import { Keypair } from './keypair';
import { BincodeWriter } from './bincode';

export const LAMPORTS_PER_EDDA = 1_000_000_000n;
export const BASE_FEE_LAMPORTS = 5_000n;

// ── Instruction types ─────────────────────────────────────────────────────────

export interface AccountMeta {
  pubkey:     PublicKey;
  isSigner:   boolean;
  isWritable: boolean;
}

export interface Instruction {
  programId: PublicKey;
  accounts:  AccountMeta[];
  data:      Uint8Array;
}

// ── System program helpers ────────────────────────────────────────────────────

export class SystemProgram {
  static readonly PROGRAM_ID = PublicKey.SYSTEM_PROGRAM;

  /** Create a SOL-style transfer instruction */
  static transfer(params: {
    fromPubkey: PublicKey;
    toPubkey:   PublicKey;
    lamports:   bigint;
  }): Instruction {
    // data = [0x00, <lamports as u64 LE>]
    const data = new Uint8Array(9);
    data[0] = 0; // instruction index 0 = Transfer
    const view = new DataView(data.buffer);
    view.setBigUint64(1, params.lamports, true);

    return {
      programId: SystemProgram.PROGRAM_ID,
      accounts:  [
        { pubkey: params.fromPubkey, isSigner: true,  isWritable: true  },
        { pubkey: params.toPubkey,   isSigner: false, isWritable: true  },
      ],
      data,
    };
  }
}

// ── Serialization ─────────────────────────────────────────────────────────────

function serializeMessage(
  feePayer:        PublicKey,
  recentBlockhash: string,
  instructions:    Instruction[],
): Uint8Array {
  const w = new BincodeWriter();

  // fee_payer: Pubkey (32 bytes)
  w.writeBytes(feePayer.bytes);

  // recent_blockhash: Hash (32 bytes)
  const hashBytes = new Uint8Array(32);
  const hashHex   = recentBlockhash.replace(/^0x/, '');
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

export class Transaction {
  recentBlockhash = '';
  feePayer: PublicKey | null = null;

  private instructions: Instruction[] = [];
  private _signature: Uint8Array | null = null;
  private _signer:    PublicKey | null = null;

  add(ix: Instruction): this {
    this.instructions.push(ix);
    return this;
  }

  /** Sign the transaction with a Keypair. Must set feePayer + recentBlockhash first. */
  sign(keypair: Keypair): void {
    if (!this.feePayer)        throw new Error('feePayer not set');
    if (!this.recentBlockhash) throw new Error('recentBlockhash not set');

    const msgBytes = serializeMessage(
      this.feePayer,
      this.recentBlockhash,
      this.instructions,
    );

    this._signature = keypair.sign(msgBytes);
    this._signer    = keypair.publicKey;
  }

  /** Serialize the signed transaction to bincode bytes for sending via RPC */
  serialize(): Uint8Array {
    if (!this._signature || !this._signer || !this.feePayer) {
      throw new Error('Transaction must be signed before serializing');
    }

    const msgBytes = serializeMessage(
      this.feePayer,
      this.recentBlockhash,
      this.instructions,
    );

    const w = new BincodeWriter();

    // message (inline — bincode serializes structs field by field)
    w.writeBytes(msgBytes);

    // signature: Vec<u8>
    w.writeByteVec(this._signature);

    // signer: Pubkey (32 bytes)
    w.writeBytes(this._signer.bytes);

    return w.toBytes();
  }

  /** Base64-encode the serialized transaction (what sendTransaction RPC expects) */
  toBase64(): string {
    return Buffer.from(this.serialize()).toString('base64');
  }
}
