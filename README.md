# systemone-mcp-core

High-speed **System-1 (`Laya-Vision` / `Vev`)** confidence gate state machine and zero-copy **Set-of-Mark (SoM)** DOM node parser in Rust for low-latency Mobile GUI Agents.

## Key Features

- **Zero-Copy DOM / SoM Slice Parser (`ownership_lab.rs`)**: Leverages Rust lifetimes (`'a`) and string slices (`&'a str`) to parse mobile UI hierarchy nodes and compute tap centers with zero heap allocations.
- **Strongly-Typed `/v1/systemone` Protocol (`protocol.rs`)**: Models `noul` (Bernoulli probability), `choice` (closed-set routing), and `score` (rubric progress evaluation) primitives using Serde internally-tagged algebraic enums (`#[serde(tag = "type")]`).
- **Confidence Gate & Anti-Deadlock Circuit Breaker (`gate_engine.rs`)**: Sub-millisecond local state machine with loading guards, popup interception, confidence gating (`>= 0.82`), and stagnant-progress fallback to System-2 planners.

## Quick Start

```bash
cargo test
cargo run
```
