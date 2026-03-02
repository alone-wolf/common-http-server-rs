# http-socket

`http-socket` is a Rust library prototype for capability-negotiated client/server session setup over HTTP-oriented transports.

It currently focuses on:
- handshake negotiation (transport/version/capability contract)
- session state machine and transport attach/swap flow
- extension boundaries (auth/store/observability/middleware)
- Axum integration via `HttpSocketAxumLayer`

## Features

- default: `client`, `server`
- optional transport markers: `ws`, `sse`, `poll`
- integration: `axum` (enables `tower` + `server`)
- extension toggles: `tracing`, `metrics`

## Quick Start

Run the Axum demo server:

```bash
cargo run --example axum_http_socket_server --features axum
```

Run the demo client (new terminal):

```bash
cargo run --example http_socket_client -- http://127.0.0.1:4000
```

## Notes

- The crate is currently an early-stage `0.1.0` implementation.
- The built-in `ws`/`sse`/`poll` transport structs are lightweight handles for framework flow testing.
