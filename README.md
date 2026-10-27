# systemone-mcp-core

Ultra-low-latency **UI Skills Runtime** & **OrderBook-style Delta Context Compression MCP Engine** powered by Rust and **System-1 (`Laya-Vision` / `Vev`)** vision primitives.

## Core Modules

- **Zero-Copy DOM / SoM Slice Parser (`src/ownership_lab.rs`)**: Leverages Rust lifetimes (`'a`), string slices (`&'a str`), and lazy iterator chains to parse mobile/web UI hierarchy nodes and filter out `<100px²` layout noise with zero heap allocations.
- **OrderBook-Style UI Delta Context Compressor (`src/context_compressor.rs`)**: Stateful frame-to-frame UI tree differ (`UiDeltaCompressor`). Collapses static background nodes into `[UNCHANGED: N nodes]` and emits only `[ADDED]`, `[MODIFIED]`, and `[REMOVED]` events—reducing prompt token consumption by **90% ~ 96%**.
- **Strongly-Typed `/v1/systemone` Protocol (`src/protocol.rs`)**: Models `noul` (Bernoulli probability), `choice` (closed-set routing), and `score` (rubric progress evaluation) primitives using Serde internally-tagged algebraic enums (`#[serde(tag = "type")]`).
- **Confidence Gate & Anti-Deadlock Circuit Breaker (`src/gate_engine.rs`)**: Sub-millisecond local state machine with loading guards, popup interception, confidence gating (`>= 0.82`), and stagnant-progress fallback to System-2 planners.

## Quick Start

```bash
cargo test
cargo run
```
