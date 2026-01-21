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

## Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **Monero daemon** (`monerod`) running locally with RPC enabled
  - Default RPC port: `18081`
  - Must be running in mainnet or testnet mode
  - Should be fully synced for best results

## Quick Start

### 1. Clone and Build

```bash
git clone https://github.com/roundnews/MoneroWebCoordinator.git
cd MoneroWebCoordinator
cargo build --release
```

### 2. Configure

Copy the example configuration and edit it:

```bash
cp config.example.toml config.toml
```

Edit `config.toml` and set:
- Your Monero wallet address in `monerod.wallet_address`
- Verify `monerod.rpc_url` matches your local daemon
- Adjust `server.bind_addr` if needed (default: `0.0.0.0:8080`)

### 3. Run

```bash
cargo run --release
```

The server will start and listen for:
- WebSocket connections at `/ws` (default port 8080)
- Health checks at `/health`

### 4. Connect Browser Miners

Point your Web XMR Miner instances to:
```
ws://your-server:8080/ws
```

For production, use a reverse proxy (nginx, caddy) with TLS:
```
wss://your-domain.com/ws
```

## Configuration Reference

All configuration is in `config.toml` (see `config.example.toml` for full example).

### Server Settings

```toml
[server]
bind_addr = "0.0.0.0:8080"              # Listen address
ws_path = "/ws"                          # WebSocket endpoint
max_connections = 5000                   # Total connection limit
max_connections_per_ip = 20              # Per-IP limit
max_frame_bytes = 32768                  # Max WebSocket frame size
```

### Monerod Connection

```toml
[monerod]
rpc_url = "http://127.0.0.1:18081"       # Local monerod RPC
wallet_address = "YOUR_XMR_ADDRESS_HERE" # Your wallet for rewards
reserve_size = 8                         # Reserved bytes in template
rpc_timeout_ms = 5000                    # RPC timeout
```

**⚠️ Security:** Never expose monerod RPC to the public internet. The coordinator should run on the same machine or a trusted local network.

### Job Management

```toml
[jobs]
job_ttl_ms = 30000                       # Job validity period
template_refresh_interval_ms = 20000     # Template update frequency
stale_job_grace_ms = 10000               # Grace for old submissions
```

### Rate Limits

```toml
[limits]
submits_per_minute = 10                  # Block submission limit
shares_per_minute = 120                  # Share submission limit
messages_per_second = 20                 # Message rate limit
```

### Metrics (Optional)

```toml
[metrics]
enable = true                            # Enable Prometheus metrics
bind_addr = "127.0.0.1:9100"             # Metrics endpoint
path = "/metrics"                        # Metrics path
```

## Security Considerations

### Critical Security Rules

1. **Never expose monerod RPC publicly**
   - Keep monerod on localhost or private network only
   - Only the coordinator should communicate with monerod

2. **Run behind a reverse proxy in production**
   - Use nginx or caddy with TLS/SSL
   - Enable rate limiting at the proxy level
   - Set appropriate CORS policies

3. **Validate your configuration**
   - Use a real Monero address you control
   - Test with small hashrate before scaling up
   - Monitor for unusual submission patterns

4. **Rate limiting is enforced**
   - Per-session limits prevent abuse
   - Per-IP connection limits prevent flooding
   - Message size limits prevent memory exhaustion

### Network Architecture

```
Internet
   |
   v
[Reverse Proxy - nginx/caddy with TLS]
   |
   v
[Coordinator :8080] <--localhost--> [monerod :18081]
```

## Development

### Build

```bash
cargo build
```

### Run with debug logging

```bash
RUST_LOG=monero_web_coordinator=debug cargo run
```

### Run tests (when available)

```bash
cargo test
```

### Check for issues

```bash
cargo clippy
cargo fmt --check
```

## Architecture Details

### Component Overview

- **Config Module** (`src/config.rs`): TOML configuration loading and validation
- **Server Module** (`src/server.rs`): HTTP/WebSocket server using Axum
- **Error Module** (`src/error.rs`): Unified error types
- **Main** (`src/main.rs`): Application entry point and initialization

### Future Modules (Planned)

- **RPC Client**: Monerod communication and template management
- **Job Manager**: Work distribution and submission handling
- **Session Manager**: Miner session tracking and rate limiting
- **Metrics**: Prometheus metrics collection

### WebSocket Protocol

The coordinator will implement the Stratum-like protocol for browser miners:
- Job notifications with unique work assignments
- Share submissions with validation
- Block candidate forwarding to monerod

See the [Web XMR Miner POC](https://github.com/roundnews/web-xmr-miner-poc) for client-side implementation.

## Troubleshooting

### "Failed to read config file"

Create `config.toml` from the example:
```bash
cp config.example.toml config.toml
```

### "Connection refused" to monerod

Ensure monerod is running with RPC enabled:
```bash
monerod --rpc-bind-ip 127.0.0.1 --rpc-bind-port 18081
```

### Port already in use

Change `bind_addr` in `config.toml` to use a different port.

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- [Monero Project](https://getmonero.org)
- Built with [Axum](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs)
