"use strict";
/**
 * Example: send 1 EDDA from Alice to Bob using the SDK.
 * Run with: npx ts-node src/example.ts   (while edda-node is running)
 */
Object.defineProperty(exports, "__esModule", { value: true });
const index_1 = require("./index");
async function main() {
    const connection = new index_1.Connection('http://127.0.0.1:8899');
    // 1. Check node is alive
    const alive = await connection.healthCheck();
    console.log('Node alive:', alive);
    // 2. Network stats
    const info = await connection.getNetworkInfo();
    console.log('Validators :', info.validators);
    console.log('Supply     :', info.total_supply / index_1.LAMPORTS_PER_EDDA, 'EDDA');
    console.log('Burned     :', info.total_burned, 'lamports');
    // 3. Generate keypairs
    const alice = index_1.Keypair.generate();
    const bob = index_1.Keypair.generate();
    console.log('\nAlice:', alice.publicKey.toHex());
    console.log('Bob  :', bob.publicKey.toHex());
    // 4. Get blockhash (required to prevent replay attacks)
    const { blockhash } = await connection.getRecentBlockhash();
    console.log('\nBlockhash:', blockhash);
    // 5. Build + sign transaction
    const tx = new index_1.Transaction();
    tx.feePayer = alice.publicKey;
    tx.recentBlockhash = blockhash;
    tx.add(index_1.SystemProgram.transfer({
        fromPubkey: alice.publicKey,
        toPubkey: bob.publicKey,
        lamports: index_1.LAMPORTS_PER_EDDA, // 1 EDDA
    }));
    tx.sign(alice);
    console.log('\nTransaction size:', tx.serialize().length, 'bytes');
    console.log('Base64 preview  :', tx.toBase64().slice(0, 60) + '...');
    // 6. Broadcast (Alice needs on-chain balance — this shows the full API flow)
    try {
        const sig = await connection.sendTransaction(tx);
        console.log('\nTransaction sent!');
        console.log('Signature:', sig);
    }
    catch (e) {
        // Expected: Alice has no on-chain balance yet (she was just generated)
        console.log('\nSend result:', e instanceof Error ? e.message : String(e));
        console.log('(Alice needs funds first — use the faucet or genesis account)');
    }
    // 7. Block height
    const height = await connection.getBlockHeight();
    console.log('\nCurrent block height:', height.toString());
}
main().catch(console.error);
//# sourceMappingURL=example.js.map