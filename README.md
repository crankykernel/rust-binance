# My Rust Binance Client Library

Use it or don't, or use it for the examples.

This is an extraction from my larger trading library, but I'm trying to extract
the Binance client code out in a re-usable way.

## Examples

### Binance Futures Public WebSocket

```
cargo run --example futures-ws
```

### Command Line Order Tool

```
API_KEY=... API_SECRET=... cargo run --example binance
```