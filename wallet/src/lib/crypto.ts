import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// Enable sync mode for browser
ed.etc.sha512Sync = (...msgs: Uint8Array[]) => sha512(
  msgs.reduce((a, m) => { const r = new Uint8Array(a.length + m.length); r.set(a); r.set(m, a.length); return r; }, new Uint8Array(0))
);

export function bytesToHex(b: Uint8Array): string {
  return Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
}

export function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}

export function generateKeypair(): { privateKey: string; publicKey: string } {
  const priv = ed.utils.randomPrivateKey();
  const pub  = ed.getPublicKey(priv);
  return { privateKey: bytesToHex(priv), publicKey: bytesToHex(pub) };
}

export function pubkeyFromPrivate(privHex: string): string {
  return bytesToHex(ed.getPublicKey(hexToBytes(privHex)));
}

// ── Bincode serialization ─────────────────────────────────────────────────────

class Writer {
  private chunks: Uint8Array[] = [];
  u8(n: number)      { this.chunks.push(new Uint8Array([n & 0xff])); }
  bool(b: boolean)   { this.u8(b ? 1 : 0); }
  u64(n: bigint)     { const v = new DataView(new ArrayBuffer(8)); v.setBigUint64(0, n, true); this.chunks.push(new Uint8Array(v.buffer)); }
  raw(b: Uint8Array) { this.chunks.push(b); }
  vec(b: Uint8Array) { this.u64(BigInt(b.length)); this.raw(b); }
  seq<T>(items: T[], fn: (w: Writer, t: T) => void) { this.u64(BigInt(items.length)); items.forEach(t => fn(this, t)); }
  bytes(): Uint8Array {
    const total = this.chunks.reduce((s, c) => s + c.length, 0);
    const out = new Uint8Array(total);
    let off = 0; for (const c of this.chunks) { out.set(c, off); off += c.length; }
    return out;
  }
}

interface AccountMeta { pubkey: Uint8Array; isSigner: boolean; isWritable: boolean; }
interface Instruction  { programId: Uint8Array; accounts: AccountMeta[]; data: Uint8Array; }

function serializeMessage(feePayer: Uint8Array, blockhash: string, ixs: Instruction[]): Uint8Array {
  const w = new Writer();
  w.raw(feePayer);
  const bh = new Uint8Array(32);
  const hex = blockhash.replace(/^0x/, '');
  for (let i = 0; i < 32; i++) bh[i] = parseInt(hex.slice(i * 2, i * 2 + 2) || '00', 16);
  w.raw(bh);
  w.seq(ixs, (w, ix) => {
    w.raw(ix.programId);
    w.seq(ix.accounts, (w, am) => { w.raw(am.pubkey); w.bool(am.isSigner); w.bool(am.isWritable); });
    w.vec(ix.data);
  });
  return w.bytes();
}

export function buildTransferTx(
  privHex:    string,
  toPubHex:   string,
  lamports:   bigint,
  blockhash:  string,
): string {
  const priv     = hexToBytes(privHex);
  const from     = ed.getPublicKey(priv);
  const to       = hexToBytes(toPubHex);
  const systemPg = new Uint8Array(32);

  const data = new Uint8Array(9);
  data[0] = 0;
  new DataView(data.buffer).setBigUint64(1, lamports, true);

  const ix: Instruction = {
    programId: systemPg,
    accounts:  [
      { pubkey: from, isSigner: true,  isWritable: true  },
      { pubkey: to,   isSigner: false, isWritable: true  },
    ],
    data,
  };

  const msgBytes = serializeMessage(from, blockhash, [ix]);
  const sig      = ed.sign(msgBytes, priv);

  const w = new Writer();
  w.raw(msgBytes);
  w.vec(sig);
  w.raw(from);
  return btoa(String.fromCharCode(...w.bytes()));
}
