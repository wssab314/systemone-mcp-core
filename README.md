# systemone-mcp-core

Low-latency UI execution runtime, stateful DOM/XML delta compressor, and confidence-gated state machine in Rust, designed for Model Context Protocol (MCP) agents operating over `/v1/systemone` single-forward-pass primitives (`noul`, `choice`, `score`).

## Architecture & Modules

- **`src/ownership_lab.rs` — Zero-Copy DOM / SoM Slice Parser**
  Parses raw UI hierarchy lines using borrowed string slices (`&'a str`) and lazy iterators (`ZeroCopyDomNode<'a>`). Filters out non-interactive layout noise (`< 100px²`) and computes bounding-box center coordinates without heap allocations.
- **`src/context_compressor.rs` — Stateful UI Delta Compressor**
  Implements frame-to-frame tree diffing (`UiDeltaCompressor`) inspired by incremental order-book feeds. Computes structural fingerprints per node, collapses unchanged nodes into a single summary line (`[UNCHANGED: N nodes]`), and emits only `[ADDED]`, `[MODIFIED]`, and `[REMOVED]` patches.
- **`src/protocol.rs` — `/v1/systemone` Wire Protocol**
  Strongly-typed Serde representation (`#[serde(tag = "type")]`) for single-forward-pass vision primitives:
  - `noul`: Bernoulli binary guard probability (`p_yes`).
  - `choice`: Closed-set Set-of-Mark (SoM) anchor selection (`selected_id`, `confidence`).
  - `score`: Multi-dimensional rubric evaluation for task progress tracking (`normalized_score`).
- **`src/gate_engine.rs` — Confidence Gate & Circuit Breaker**
  Deterministic state machine (`GateEngine`) enforcing pre-action loading checks, modal/popup interception, confidence thresholding (`>= 0.82`), and stagnant-progress circuit breaking (escalating to a System-2 planner after `N` consecutive steps without score improvement).

## Usage

```bash
# Run unit test suite
cargo test

# Run end-to-end pipeline verification
cargo run
```

## License

MIT
