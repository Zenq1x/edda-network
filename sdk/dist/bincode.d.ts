/**
 * Minimal bincode serializer that matches Rust's bincode default config:
 * - Little-endian integers
 * - u64 length prefix for sequences (Vec<T>, Vec<u8>)
 * - No padding, no alignment
 */
export declare class BincodeWriter {
    private chunks;
    writeU8(n: number): void;
    writeBool(b: boolean): void;
    writeU64(n: bigint): void;
    writeBytes(b: Uint8Array): void;
    /** Vec<u8>: 8-byte length prefix + raw bytes */
    writeByteVec(b: Uint8Array): void;
    /** Vec<T>: 8-byte count prefix + each item written by callback */
    writeSeq<T>(items: T[], writeItem: (w: BincodeWriter, item: T) => void): void;
    toBytes(): Uint8Array;
}
//# sourceMappingURL=bincode.d.ts.map