# EDDA Network

High-performance Layer 1 blockchain — 65,000 TPS, 400ms finality, near-zero fees.

## Links

- **Website** — [eddachain.com](https://eddachain.com)
- **Wallet** — [wallet.eddachain.com](https://wallet.eddachain.com)
- **Explorer** — [explorer.eddachain.com](https://explorer.eddachain.com)
- **RPC** — [rpc.eddachain.com](https://rpc.eddachain.com)

## Overview

| | |
|---|---|
| Consensus | Proof of History + Tower BFT |
| Smart contracts | WebAssembly (WASM) |
| Max supply | 500,000,000 EDDA |
| Block reward | 10 EDDA |
| Fee model | 50% burned · 50% to validator |
| Signatures | Ed25519 |
| Networking | libp2p (Gossipsub + Kademlia DHT) |

## Repo Structure

```
crates/
  edda-consensus/   — Leader schedule, PoH, Tower BFT
  edda-node/        — P2P networking, RPC server
  edda-runtime/     — WASM contract execution
wallet/             — Next.js wallet (wallet.eddachain.com)
explorer/           — Next.js block explorer (explorer.eddachain.com)
edda website/       — Landing page (eddachain.com)
```

## License

MIT
