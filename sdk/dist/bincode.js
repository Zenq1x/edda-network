"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.BincodeWriter = void 0;
/**
 * Minimal bincode serializer that matches Rust's bincode default config:
 * - Little-endian integers
 * - u64 length prefix for sequences (Vec<T>, Vec<u8>)
 * - No padding, no alignment
 */
class BincodeWriter {
    constructor() {
        this.chunks = [];
    }
    writeU8(n) {
        this.chunks.push(new Uint8Array([n & 0xff]));
    }
    writeBool(b) {
        this.writeU8(b ? 1 : 0);
    }
    writeU64(n) {
        const buf = new ArrayBuffer(8);
        new DataView(buf).setBigUint64(0, n, true);
        this.chunks.push(new Uint8Array(buf));
    }
    writeBytes(b) {
        this.chunks.push(Uint8Array.from(b));
    }
    /** Vec<u8>: 8-byte length prefix + raw bytes */
    writeByteVec(b) {
        this.writeU64(BigInt(b.length));
        this.writeBytes(b);
    }
    /** Vec<T>: 8-byte count prefix + each item written by callback */
    writeSeq(items, writeItem) {
        this.writeU64(BigInt(items.length));
        for (const item of items)
            writeItem(this, item);
    }
    toBytes() {
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
exports.BincodeWriter = BincodeWriter;
//# sourceMappingURL=bincode.js.map