# Snipt Server

The server component of the Snipt application that provides API access to Snipt functionality.

## Overview

Snipt Server provides a REST API for accessing and managing snippets, allowing integration with other applications and remote access to your snippets. It's built using Tokio and Warp for high-performance asynchronous handling of requests.

## Features

- RESTful API for snippet management
- Authentication and authorization
- Secure storage and retrieval of snippets
- WebSocket support for real-time updates
- Cross-platform compatibility

## Usage

The server can be started through the Snipt CLI:

```bash
snipt server start
```

Or programmatically:

```rust
use snipt_server::Server;

async fn main() {
    let server = Server::new()
        .with_port(3000)
        .start()
        .await
        .expect("Failed to start server");
}
```

## API Documentation

When the server is running, API documentation is available at `/docs`.

## License

MIT 