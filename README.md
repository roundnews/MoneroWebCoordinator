# Monero Web Coordinator

A Rust-based mining coordinator that enables browser WASM miners to contribute hashpower to coordinated solo mining against your own monerod.

## Status

🚧 **Under Development** - Initial implementation in progress.

## Overview

This coordinator works with the [Web XMR Miner POC](https://github.com/roundnews/web-xmr-miner-poc) to turn temporary, opt-in test mining activity into real solo mining in a responsible way.

### Key Features

- Maintains fresh Monero block templates via local daemon RPC
- Assigns collision-free work to many browser miners using the template's reserved region
- Accepts submissions and forwards only valid block candidates to monerod
- Operates safely under untrusted traffic (rate limiting, backpressure, isolation)
- Never exposes monerod RPC to the Internet

## Architecture

```
[Browser Miner (WASM)] <--WSS/443--> [Coordinator (Rust)] <--HTTP localhost--> [monerod RPC :18081]
```

## License

MIT
