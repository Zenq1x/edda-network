export declare class PublicKey {
    readonly bytes: Uint8Array;
    constructor(value: Uint8Array | string);
    static SYSTEM_PROGRAM: PublicKey;
    toHex(): string;
    toString(): string;
    equals(other: PublicKey): boolean;
}
export declare function bytesToHex(b: Uint8Array): string;
export declare function hexToBytes(hex: string): Uint8Array;
//# sourceMappingURL=publickey.d.ts.map