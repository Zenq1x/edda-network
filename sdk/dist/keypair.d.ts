import { PublicKey } from './publickey';
export declare class Keypair {
    readonly privateKey: Uint8Array;
    readonly publicKey: PublicKey;
    private constructor();
    /** Generate a fresh random Ed25519 keypair */
    static generate(): Keypair;
    /** Reconstruct a keypair from a 32-byte private key (hex or Uint8Array) */
    static fromPrivateKey(key: Uint8Array | string): Keypair;
    /** Sign arbitrary bytes — returns 64-byte Ed25519 signature */
    sign(message: Uint8Array): Uint8Array;
    /** Verify a signature against a public key */
    static verify(publicKey: PublicKey, message: Uint8Array, signature: Uint8Array): boolean;
    /** Export private key as hex (keep this secret!) */
    privateKeyHex(): string;
}
//# sourceMappingURL=keypair.d.ts.map