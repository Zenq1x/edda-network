import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { PublicKey } from './publickey';

// Enable synchronous signing (required for Node.js with @noble/ed25519 v2)
ed.etc.sha512Sync = (...msgs) => sha512(
  msgs.reduce((acc, m) => {
    const merged = new Uint8Array(acc.length + m.length);
    merged.set(acc); merged.set(m, acc.length);
    return merged;
  }, new Uint8Array(0))
);

export class Keypair {
  readonly privateKey: Uint8Array;
  readonly publicKey: PublicKey;

  private constructor(privateKey: Uint8Array, publicKey: Uint8Array) {
    this.privateKey = privateKey;
    this.publicKey  = new PublicKey(publicKey);
  }

  /** Generate a fresh random Ed25519 keypair */
  static generate(): Keypair {
    const priv = ed.utils.randomPrivateKey();
    const pub  = ed.getPublicKey(priv);
    return new Keypair(priv, pub);
  }

  /** Reconstruct a keypair from a 32-byte private key (hex or Uint8Array) */
  static fromPrivateKey(key: Uint8Array | string): Keypair {
    const priv = typeof key === 'string'
      ? Uint8Array.from(Buffer.from(key, 'hex'))
      : key;
    const pub = ed.getPublicKey(priv);
    return new Keypair(priv, pub);
  }

  /** Sign arbitrary bytes — returns 64-byte Ed25519 signature */
  sign(message: Uint8Array): Uint8Array {
    return ed.sign(message, this.privateKey);
  }

  /** Verify a signature against a public key */
  static verify(publicKey: PublicKey, message: Uint8Array, signature: Uint8Array): boolean {
    return ed.verify(signature, message, publicKey.bytes);
  }

  /** Export private key as hex (keep this secret!) */
  privateKeyHex(): string {
    return Buffer.from(this.privateKey).toString('hex');
  }
}
