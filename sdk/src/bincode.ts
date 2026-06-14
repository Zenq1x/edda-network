/**
 * Minimal bincode serializer that matches Rust's bincode default config:
 * - Little-endian integers
 * - u64 length prefix for sequences (Vec<T>, Vec<u8>)
 * - No padding, no alignment
 */
export class BincodeWriter {
  private chunks: Uint8Array[] = [];

  writeU8(n: number): void {
    this.chunks.push(new Uint8Array([n & 0xff]));
  }

  writeBool(b: boolean): void {
    this.writeU8(b ? 1 : 0);
  }

  writeU64(n: bigint): void {
    const buf = new ArrayBuffer(8);
    new DataView(buf).setBigUint64(0, n, true);
    this.chunks.push(new Uint8Array(buf));
  }

  writeBytes(b: Uint8Array): void {
    this.chunks.push(Uint8Array.from(b));
  }

  /** Vec<u8>: 8-byte length prefix + raw bytes */
  writeByteVec(b: Uint8Array): void {
    this.writeU64(BigInt(b.length));
    this.writeBytes(b);
  }

  /** Vec<T>: 8-byte count prefix + each item written by callback */
  writeSeq<T>(items: T[], writeItem: (w: BincodeWriter, item: T) => void): void {
    this.writeU64(BigInt(items.length));
    for (const item of items) writeItem(this, item);
  }

  toBytes(): Uint8Array {
    const total = this.chunks.reduce((s, c) => s + c.length, 0);
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of this.chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  }
}
