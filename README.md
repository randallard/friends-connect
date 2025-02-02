# Friends Connect

A real-time connection service that enables players to establish connections and exchange messages.

Better example repos:
- [ble-messenger](https://github.com/randallard/ble-messenger)
- [hello-friends-connect](https://github.com/randallard/hello-friends-connect)

## Features

- Create and join connections via unique links
- Real-time messaging between connected players
- WebSocket-based notifications
- Containerized deployment
- Kubernetes orchestration

## Prerequisites

- Rust 1.75 or later
- Cargo (Rust's package manager)
- Docker (for containerization)
- kubectl (for Kubernetes deployment)

## Local Development Setup

### Windows

1. Install Rust and Cargo:
   - Download and run [rustup-init.exe](https://rustup.rs/)
   - Follow the installation prompts
   - Open a new terminal and verify installation:
     ```bash
     rustc --version
     cargo --version
     ```

2. Clone the repository:
   ```bash
   git clone <repository-url>
   cd friends-connect
   ```

3. Build and run locally:
   ```bash
   cargo build
   cargo run
   ```

   The server will start at http://localhost:8080

### Linux

1. Install Rust and Cargo:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. Clone the repository:
   ```bash
   git clone <repository-url>
   cd friends-connect
   ```

3. Build and run locally:
   ```bash
   cargo build
   cargo run
   ```

   The server will start at http://localhost:8080

## Development Tools

- Run tests:
  ```bash
  cargo test
  ```

- Run specific test:
  ```bash
  cargo test test_name
  ```

- Run with logging:
  ```bash
  RUST_LOG=debug cargo run
  ```

## Docker Build

Build the container:
```bash
docker build -t friends-connect .
```

Run the container:
```bash
docker run -p 8080:8080 friends-connect
```

## Example Usage

1. Create a new connection:
```bash
curl -X POST http://localhost:8080/connections \
  -H "Content-Type: application/json" \
  -d '{"player_id":"player1"}'
```

2. Join an existing connection:
```bash
curl -X POST http://localhost:8080/connections/link/{LINK_ID}/join \
  -H "Content-Type: application/json" \
  -d '{"player_id":"player2"}'
```

3. Send a message:
```bash
curl -X POST http://localhost:8080/connections/{CONNECTION_ID}/messages \
  -H "Content-Type: application/json" \
  -d '{"player_id":"player1","content":"Hello!"}'
```

## Project Structure

- `src/` - Source code
  - `lib.rs` - Library interface
  - `main.rs` - Application entry point
  - `server.rs` - Server implementation
  - `connection.rs` - Connection management
- `static/` - Static web files
- `examples/` - Example code
- `tests/` - Integration tests

## License

MIT

## testing the leptos example
cd to examples/leptos_hello  
```powershell
wasm-pack test --chrome --headless
```
## Other Documentation

- [Development Plan](docs/development-plan.md) - Overall roadmap and development strategy
- [Connection Flow](docs/connecting-people-flow.mermaid) - Mermaid diagram showing the connection process 
- [Connection Guide](docs/connecting-to-people.md) - Detailed documentation on how connections work
- [Progress Log](docs/progress.md) - Development progress and updates