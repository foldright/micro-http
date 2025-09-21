# Micro Http

![Crates.io](https://img.shields.io/crates/l/micro-web) 
![Crates.io](https://img.shields.io/crates/v/micro-web)
[![Actions Status](https://github.com/foldright/micro-http/actions/workflows/ci.yml/badge.svg)](https://github.com/foldright/micro-http/actions)
[![Actions Status](https://github.com/foldright/micro-http/actions/workflows/clippy.yml/badge.svg)](https://github.com/foldright/micro-http/actions)

A lightweight, efficient, and modular HTTP server implementation built on top of tokio.

## Features

- Full HTTP/1.1 protocol support, HTTP/2 currently unsupported
- Asynchronous I/O using tokio
- Streaming request and response bodies
- Chunked transfer encoding
- Keep-alive connections
- Expect-continue mechanism
- Efficient memory usage through zero-copy parsing
- Structured logging with tracing

## Crates

This workspace contains the following crates:

- [micro-http](crates/http/README.md): Core HTTP protocol implementation
  - Zero-copy parsing
  - Streaming bodies
  - Full protocol compliance
  - [Example server](crates/http/examples/server.rs)

- [micro-web](crates/web/README.md): High-level web framework
  - Routing
  - Middleware support
  - Request/Response abstractions
  - [Getting started guide](crates/web/examples/getting_started.rs)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
micro-web = "0.1"
tokio = { version = "1", features = ["full"] }
```

See the [getting started example](crates/web/examples/getting_started.rs) for a complete working example.

## Performance

For performance benchmarks and comparisons, see (we are not in this benchmark yes, but we will):
- [Web Frameworks Benchmark](https://web-frameworks-benchmark.netlify.app/result?l=rust)

## Development

- [Deploy new version guide](deploy.md)

## MSRV

The Minimum Supported Rust Version is 1.90 😁

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
