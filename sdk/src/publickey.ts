export class PublicKey {
  readonly bytes: Uint8Array;

  constructor(value: Uint8Array | string) {
    if (typeof value === 'string') {
      if (value.length !== 64) throw new Error('PublicKey hex must be 64 characters');
      this.bytes = hexToBytes(value);
    } else {
      if (value.length !== 32) throw new Error('PublicKey must be 32 bytes');
      this.bytes = Uint8Array.from(value);
    }
  }

  static SYSTEM_PROGRAM = new PublicKey(new Uint8Array(32));

  toHex(): string {
    return bytesToHex(this.bytes);
  }

  toString(): string {
    return this.toHex();
  }

  equals(other: PublicKey): boolean {
    return this.bytes.every((b, i) => b === other.bytes[i]);
  }
}

export function bytesToHex(b: Uint8Array): string {
  return Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
}

export function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) throw new Error('Invalid hex string');
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}
