"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.Keypair = void 0;
const ed = __importStar(require("@noble/ed25519"));
const sha512_1 = require("@noble/hashes/sha512");
const publickey_1 = require("./publickey");
// Enable synchronous signing (required for Node.js with @noble/ed25519 v2)
ed.etc.sha512Sync = (...msgs) => (0, sha512_1.sha512)(msgs.reduce((acc, m) => {
    const merged = new Uint8Array(acc.length + m.length);
    merged.set(acc);
    merged.set(m, acc.length);
    return merged;
}, new Uint8Array(0)));
class Keypair {
    constructor(privateKey, publicKey) {
        this.privateKey = privateKey;
        this.publicKey = new publickey_1.PublicKey(publicKey);
    }
    /** Generate a fresh random Ed25519 keypair */
    static generate() {
        const priv = ed.utils.randomPrivateKey();
        const pub = ed.getPublicKey(priv);
        return new Keypair(priv, pub);
    }
    /** Reconstruct a keypair from a 32-byte private key (hex or Uint8Array) */
    static fromPrivateKey(key) {
        const priv = typeof key === 'string'
            ? Uint8Array.from(Buffer.from(key, 'hex'))
            : key;
        const pub = ed.getPublicKey(priv);
        return new Keypair(priv, pub);
    }
    /** Sign arbitrary bytes — returns 64-byte Ed25519 signature */
    sign(message) {
        return ed.sign(message, this.privateKey);
    }
    /** Verify a signature against a public key */
    static verify(publicKey, message, signature) {
        return ed.verify(signature, message, publicKey.bytes);
    }
    /** Export private key as hex (keep this secret!) */
    privateKeyHex() {
        return Buffer.from(this.privateKey).toString('hex');
    }
}
exports.Keypair = Keypair;
//# sourceMappingURL=keypair.js.map