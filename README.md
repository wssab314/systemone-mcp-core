<div align="center">

# ⚡ systemone-mcp-core

### **Ultra-Low-Latency UI Skills, Delta Context Compression & System-1 Verification MCP Engine**
#### **基于 Rust 的极速 UI Skills 运行时、增量差分上下文压缩与 System-1 视觉门控引擎**

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust_1.95+-dea584?style=flat-square&logo=rust&logoColor=black" alt="Rust" />
  <img src="https://img.shields.io/badge/Context_Compression--96%25_Tokens-10b981?style=flat-square" alt="Compression" />
  <img src="https://img.shields.io/badge/System--1_Latency-%3C30ms-6366f1?style=flat-square" alt="Latency" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License" />
</p>

</div>

---

## 📖 Overview · 项目简介

**`systemone-mcp-core`** is a high-performance Rust execution core designed to eliminate the two biggest bottlenecks in modern GUI & Mobile AI Agents:
1. **Context Explosion**: Raw mobile XML / web DOM dumps (50KB–200KB per frame) burn thousands of prompt tokens on static layout wrappers.
2. **Autoregressive "Click-Look-Click" Deadlocks**: Traditional System-2 multimodal LLMs take `3s–5s` per micro-action and easily fall into infinite loops when unexpected popups or loading spinners appear.

**`systemone-mcp-core`** 是专为下一代 GUI 与移动端 AI Agent 打造的纯 Rust 高性能执行底座，旨在彻底解决当前 Agent 工程的两大核心痛点：
1. **上下文爆炸（Context Explosion）**：传统全量 XML/DOM 视图树每帧高达 50KB~200KB，95% 以上是无效布局容器与重复背景节点，极易撑爆 Token 预算并拖慢首字延迟（TTFT）。
2. **“点-看-点”慢速死循环**：传统云端大模型（System 2）每步微操作耗时 `3~5 秒`，且在遇到随机弹窗、骨架屏加载或操作未生效时极易产生幻觉并陷入死循环。

---

## 🏗️ Core Architecture · 核心模块架构

| Module · 模块文件 | Systems Primitive · 核心技术原语 | Description · 功能与工程实现（中英双语） |
| :--- | :--- | :--- |
| **[`src/ownership_lab.rs`](src/ownership_lab.rs)** | **Zero-Copy Lifetimes (`'a`)**<br/>零拷贝切片解析 | **Zero-Copy DOM / SoM Slice Parser**<br/>Parses raw UI hierarchy buffers using `&'a str` fat-pointer slices (`16B`) and lazy iterators, filtering out `<100px²` layout noise with **0 heap allocations**.<br/>利用 Rust 生命周期 `'a` 与 `&'a str` 胖指针切片直接借用原始缓冲区，零堆内存分配过滤无效微小噪点并计算点击中心点。 |
| **[`src/context_compressor.rs`](src/context_compressor.rs)** | **OrderBook-Style Delta Diff**<br/>高频盘口式增量差分 | **Stateful Frame-to-Frame UI Delta Compressor (`UiDeltaCompressor`)**<br/>Inspired by high-frequency exchange OrderBook delta streams: hashes UI nodes across consecutive frames, collapses unchanged nodes into `[UNCHANGED: N nodes]`, and emits only `[ADDED]`, `[MODIFIED]`, and `[REMOVED]` patches (**reducing prompt tokens by 90% ~ 96%**).<br/>借鉴高频交易盘口增量协议（Snapshot + Delta），对比前后两帧 UI 视图树，将未变化背景折叠为单行摘要，仅输出新增/修改/移除的增量锚点，**节省 90%~96% Token**。 |
| **[`src/protocol.rs`](src/protocol.rs)** | **Algebraic Data Types**<br/>代数枚举与强类型协议 | **Strongly-Typed `/v1/systemone` Protocol**<br/>Models single-forward-pass decision primitives — `noul` (Bernoulli binary guard), `choice` (closed-set anchor routing), and `score` (rubric progress evaluation) — via `#[serde(tag = "type")]`.<br/>使用 Rust 代数数据类型（`enum`）与 Serde 内部标签零歧义建模 `/v1/systemone` 三大单次前向传播原语（`noul` 二元概率守卫、`choice` 候选锚点路由、`score` 任务进度打分）。 |
| **[`src/gate_engine.rs`](src/gate_engine.rs)** | **Zero-Trust Verify Gate**<br/>置信度门控与防死循环熔断 | **Confidence Gate & Anti-Deadlock Circuit Breaker (`GateEngine`)**<br/>Enforces a 4-stage pipeline: loading spinner guard $\rightarrow$ popup auto-dismissal $\rightarrow$ confidence threshold gate (`>= 0.82`) $\rightarrow$ stagnant progress circuit breaker (`3` consecutive flat steps trigger System-2 fallback).<br/>实现四级确定性防护流水线：加载态守卫等待 $\rightarrow$ 意外弹窗毫秒级清道夫拦截 $\rightarrow$ 0.82 置信度门控 $\rightarrow$ 连续 3 步无进展硬件级熔断并唤醒 System-2 慢思考重规划。 |

---

## 🚀 Quick Start · 快速运行

```bash
# Run all unit tests (Zero-Copy Parser, Delta Compressor, Protocol & Gate Engine)
# 运行全部单元测试
cargo test

# Run the end-to-end interactive demo pipeline
# 运行端到端演练流水线
cargo run
```
