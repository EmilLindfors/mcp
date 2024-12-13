# mcp.rs

A Rust implementation of the Model Context Protocol (MCP), providing a standardized way for AI models to access external context and resources.

## Quickstart 

### Client
```
# List resources
cargo run --bin client list-resources

# Read a specific file
cargo run --bin client read-resource -u "file:///path/to/file"

# Use a prompt
cargo run --bin client get-prompt -n "code_review" -a '{"code": "fn main() {}", "language": "rust"}'

# Call a tool
cargo run --bin client -- --server "http://127.0.0.1:3000" call-tool --name "file_system" --args '{\"operation\": \"read_file\", \"path\": \"Config.toml\"}'

# Set log level
cargo run --bin client set-log-level -l "debug"

# Use SSE transport
cargo run --bin client -t sse -s http://localhost:3000 list-resources
```

### Server
```
# Run with test config
cargo run --bin server -- --config "../servers/test.json"
```

## Overview

mcp.rs is a high-performance, type-safe Rust implementation of the Model Context Protocol, designed to enable seamless communication between AI applications and their integrations. It provides a robust foundation for building MCP servers that can expose various types of resources (files, data, APIs) to AI models.

## Features

- **Multiple Transport Types**:
  - Standard Input/Output (stdio) transport for CLI tools
  - HTTP with Server-Sent Events (SSE) for web integrations
  - Extensible transport system for custom implementations

- **Resource Management**:
  - File system resource provider
  - Resource templating support
  - Real-time resource updates
  - Resource subscription capabilities

- **Flexible Configuration**:
  - YAML/JSON configuration files
  - Environment variable overrides
  - Command-line arguments
  - Sensible defaults

- **Security**:
  - Built-in access controls
  - Path traversal protection
  - Rate limiting
  - CORS support

## Installation

Add mcp.rs to your project's `Cargo.toml`:

```toml
[dependencies]
mcp = "0.1.0"
```

## Quick Start

1. Create a basic MCP server:

```rust
use mcp_rs::error::McpError;
use mcp_rs::server::config::ServerConfig;
use mcp_rs::server::McpServer;

#[tokio::main]
async fn main() -> Result<(), McpError> {
    // Create server with default configuration
    let mut server = McpServer::new(ServerConfig::default());
    
    // Run the server
    server.run().await
}
```

2. Configure via command line:

```bash
# Run with stdio transport
mcp-server -t stdio

# Run with SSE transport on port 3000
mcp-server -t sse -p 3000

# Enable debug logging
mcp-server -l debug
```

3. Or use a configuration file:

```yaml
server:
  name: "my-mcp-server"
  version: "1.0.0"
  transport: sse
  port: 3000

resources:
  root_path: "./resources"
  allowed_schemes:
    - file
  max_file_size: 10485760

security:
  enable_auth: false
  allowed_origins:
    - "*"

logging:
  level: "info"
  format: "pretty"
```

## Architecture

mcp.rs follows a modular architecture:

- **Transport Layer**: Handles communication between clients and servers
- **Protocol Layer**: Implements the MCP message format and routing
- **Resource Layer**: Manages access to external resources
- **Configuration**: Handles server settings and capabilities

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `transport` | Transport type (stdio, sse) | stdio |
| `port` | Server port for network transports | 3000 |
| `log_level` | Logging level | info |
| `resource_root` | Root directory for resources | ./resources |

## API Reference

For detailed API documentation, run:
```bash
cargo doc --open
```

## Examples (TODO)

Check out the `/examples` directory for:
- Basic server implementation
- Custom resource provider
- Configuration examples
- Integration patterns

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting pull requests.

### Development Requirements

- Rust 1.70 or higher
- Cargo
- Optional: Docker for containerized testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test
```

## License

This project is licensed under the [MIT License](LICENSE).

## Acknowledgments

This implementation is based on the Model Context Protocol specification and inspired by the reference implementation.

## Contact

- Issue Tracker: [GitHub Issues](https://github.com/EmilLindfors/mcp/issues)
- Source Code: [GitHub Repository](https://github.com/EmilLindfors/mcp)