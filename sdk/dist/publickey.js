"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PublicKey = void 0;
exports.bytesToHex = bytesToHex;
exports.hexToBytes = hexToBytes;
class PublicKey {
    constructor(value) {
        if (typeof value === 'string') {
            if (value.length !== 64)
                throw new Error('PublicKey hex must be 64 characters');
            this.bytes = hexToBytes(value);
        }
        else {
            if (value.length !== 32)
                throw new Error('PublicKey must be 32 bytes');
            this.bytes = Uint8Array.from(value);
        }
    }
    toHex() {
        return bytesToHex(this.bytes);
    }
    toString() {
        return this.toHex();
    }
    equals(other) {
        return this.bytes.every((b, i) => b === other.bytes[i]);
    }
}
exports.PublicKey = PublicKey;
PublicKey.SYSTEM_PROGRAM = new PublicKey(new Uint8Array(32));
function bytesToHex(b) {
    return Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
}
function hexToBytes(hex) {
    if (hex.length % 2 !== 0)
        throw new Error('Invalid hex string');
    const out = new Uint8Array(hex.length / 2);
    for (let i = 0; i < out.length; i++) {
        out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return out;
}
//# sourceMappingURL=publickey.js.map