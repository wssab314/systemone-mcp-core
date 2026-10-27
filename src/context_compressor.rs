//! # 第四课：高频盘口级 UI 上下文增量差分压缩器 (`UiDeltaCompressor`)
//!
//! 对应路线图 **【同构方向 3：高频盘口 OrderBook Delta 差分 $\rightarrow$ 96% 脱水率的 UI/DOM Context Compression】**。
//!
//! ## 核心设计思想
//! 传统 GUI/Browser Agent 每一步都将完整的 60KB~200KB DOM/XML 树重复发给大模型，95% 以上的节点在相邻两帧之间根本没有变化。
//! 我们借鉴高频交易网关的 **OrderBook Snapshot + Incremental Delta 协议**：
//! 1. **零拷贝脱水 (Dehydration)**：借用 [`ZeroCopyDomNode<'a>`] 过滤掉不可见、无标签或面积过小的冗余布局容器。
//! 2. **结构锚点哈希定位 (`NodeAnchorKey`)**：以控件的 `(tag, bounds)` 作为空间结构键（实现 `Hash + Eq` Trait）。
//! 3. **帧间差分 (`UiDeltaPatch`)**：对比上一帧快照与当前帧：
//!    - 未发生变化的静态背景控件全部折叠为一行 `[UNCHANGED: N nodes]`；
//!    - 仅输出 `[ADDED]`、`[MODIFIED]`、`[REMOVED]` 增量指令，将上层 Agent 的 Prompt Token 消耗降低 90%~96%！

use std::collections::HashMap;
use std::fmt::Write;

use crate::ownership_lab::{parse_and_filter_som_nodes, ZeroCopyDomNode};

/// 控件在屏幕空间中的结构锚点键（类似于 OrderBook 中的 Price Level Key）
///
/// 通过派生 `Hash, PartialEq, Eq`，使该结构体可以直接作为 `HashMap` 的 Key。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeAnchorKey {
    pub tag: String,
    pub bounds: (u32, u32, u32, u32),
}

impl NodeAnchorKey {
    pub fn from_zero_copy(node: &ZeroCopyDomNode<'_>) -> Self {
        Self {
            tag: node.tag.to_string(),
            bounds: node.bounds,
        }
    }
}

/// 单条 UI 增量变更事件（对应 OrderBook 的单档位 Delta Event）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiDeltaEvent {
    /// 新增可交互元素（如新弹出的二次确认按钮或下拉菜单项）
    Added {
        som_id: usize,
        tag: String,
        label: String,
        center: (u32, u32),
    },
    /// 同一空间位置的控件文本/状态发生变更（如输入框填入内容、进度百分比更新）
    Modified {
        som_id: usize,
        tag: String,
        old_label: String,
        new_label: String,
        center: (u32, u32),
    },
    /// 原有控件消失（如骨架屏消失、弹窗被关闭）
    Removed {
        tag: String,
        old_label: String,
    },
}

/// 单帧压缩后的增量补丁包与脱水率统计指标
#[derive(Debug, Clone)]
pub struct UiDeltaPatch {
    pub unchanged_count: usize,
    pub events: Vec<UiDeltaEvent>,
    pub raw_bytes: usize,
    pub compressed_prompt: String,
}

impl UiDeltaPatch {
    /// 计算上下文体积压缩率（脱水比例，例如 `0.94` 代表省去了 94% 的字节）
    pub fn compression_ratio(&self) -> f64 {
        if self.raw_bytes == 0 {
            return 0.0;
        }
        let comp_len = self.compressed_prompt.len();
        if comp_len >= self.raw_bytes {
            0.0
        } else {
            1.0 - (comp_len as f64 / self.raw_bytes as f64)
        }
    }
}

/// 有状态的帧间 UI 增量差分压缩引擎
#[derive(Debug, Clone)]
pub struct UiDeltaCompressor {
    /// 过滤噪点控件的最小像素面积阈值
    min_area: u32,
    /// 上一帧的控件状态快照表：`NodeAnchorKey -> Label`
    prev_snapshot: HashMap<NodeAnchorKey, String>,
}

impl UiDeltaCompressor {
    pub fn new(min_area: u32) -> Self {
        Self {
            min_area,
            prev_snapshot: HashMap::new(),
        }
    }

    /// 重置快照缓存（例如发生页面级路由跳转时调用）
    pub fn reset(&mut self) {
        self.prev_snapshot.clear();
    }

    /// 输入当前帧原始 UI 树文本，输出脱水并差分后的 `UiDeltaPatch`，同时自动推进内部快照状态
    pub fn compress_frame(&mut self, raw_dump: &str) -> UiDeltaPatch {
        let raw_bytes = raw_dump.len();
        let valid_nodes = parse_and_filter_som_nodes(raw_dump, self.min_area);

        let mut current_snapshot = HashMap::with_capacity(valid_nodes.len());
        let mut events = Vec::new();
        let mut unchanged_count = 0usize;

        // 1. 遍历当前帧有效节点：识别 Unchanged / Modified / Added
        for (som_id, node) in &valid_nodes {
            let key = NodeAnchorKey::from_zero_copy(node);
            let center = node.center_point();

            match self.prev_snapshot.get(&key) {
                Some(old_label) if old_label == node.label => {
                    unchanged_count += 1;
                }
                Some(old_label) => {
                    events.push(UiDeltaEvent::Modified {
                        som_id: *som_id,
                        tag: node.tag.to_string(),
                        old_label: old_label.clone(),
                        new_label: node.label.to_string(),
                        center,
                    });
                }
                None => {
                    events.push(UiDeltaEvent::Added {
                        som_id: *som_id,
                        tag: node.tag.to_string(),
                        label: node.label.to_string(),
                        center,
                    });
                }
            }

            current_snapshot.insert(key, node.label.to_string());
        }

        // 2. 检查在当前帧中消失的旧节点 (Removed)
        for (old_key, old_label) in &self.prev_snapshot {
            if !current_snapshot.contains_key(old_key) {
                events.push(UiDeltaEvent::Removed {
                    tag: old_key.tag.clone(),
                    old_label: old_label.clone(),
                });
            }
        }

        // 3. 更新状态机快照为当前帧
        self.prev_snapshot = current_snapshot;

        // 4. 预分配内存生成极简 Delta Prompt（避免频繁堆重分配）
        let mut compressed_prompt = String::with_capacity(256);
        let _ = writeln!(compressed_prompt, "[UNCHANGED: {unchanged_count} nodes]");
        for ev in &events {
            match ev {
                UiDeltaEvent::Added {
                    som_id,
                    tag,
                    label,
                    center: (cx, cy),
                } => {
                    let _ = writeln!(
                        compressed_prompt,
                        "[ADDED] #{som_id:02} | {tag} | \"{label}\" | center=({cx}, {cy})"
                    );
                }
                UiDeltaEvent::Modified {
                    som_id,
                    tag,
                    old_label,
                    new_label,
                    center: (cx, cy),
                } => {
                    let _ = writeln!(
                        compressed_prompt,
                        "[MODIFIED] #{som_id:02} | {tag} | \"{old_label}\" -> \"{new_label}\" | center=({cx}, {cy})"
                    );
                }
                UiDeltaEvent::Removed { tag, old_label } => {
                    let _ = writeln!(
                        compressed_prompt,
                        "[REMOVED] {tag} | \"{old_label}\""
                    );
                }
            }
        }

        UiDeltaPatch {
            unchanged_count,
            events,
            raw_bytes,
            compressed_prompt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_style_ui_delta_compression() {
        let mut compressor = UiDeltaCompressor::new(100);

        // 构造包含大量静态背景节点与冗余分割线的第 1 帧（首帧全量建立快照）
        let mut frame1_lines = Vec::new();
        for i in 0..30 {
            frame1_lines.push(format!(
                "StaticCard | Cluster Node #{i:02} (Healthy) | 20,{y1},1060,{y2}",
                y1 = 100 + i * 40,
                y2 = 135 + i * 40
            ));
            // 掺入大量面积为 1px 的无效分割线噪点
            frame1_lines.push(format!("Divider | | 0,{y},1080,{y}", y = 99 + i * 40));
        }
        frame1_lines.push("EditText | Replica Count: 2 | 80,1400,650,1490".to_string());
        frame1_lines.push("Spinner | Loading Metrics... | 400,1520,680,1580".to_string());
        let frame1_raw = frame1_lines.join("\n");

        let patch1 = compressor.compress_frame(&frame1_raw);
        assert_eq!(patch1.unchanged_count, 0);
        assert_eq!(patch1.events.len(), 32); // 30 个静态卡片 + 1 个输入框 + 1 个加载转圈

        // 构造第 2 帧：30 个静态卡片完全未变，Spinner 消失，EditText 值变为 4，新增 Confirm 按钮
        let mut frame2_lines = Vec::new();
        for i in 0..30 {
            frame2_lines.push(format!(
                "StaticCard | Cluster Node #{i:02} (Healthy) | 20,{y1},1060,{y2}",
                y1 = 100 + i * 40,
                y2 = 135 + i * 40
            ));
            frame2_lines.push(format!("Divider | | 0,{y},1080,{y}", y = 99 + i * 40));
        }
        frame2_lines.push("EditText | Replica Count: 4 | 80,1400,650,1490".to_string());
        frame2_lines.push("Modal_Button | Confirm Scale-Up | 140,1600,590,1690".to_string());
        let frame2_raw = frame2_lines.join("\n");

        let patch2 = compressor.compress_frame(&frame2_raw);

        // 验证：30 个未变节点全部被折叠！
        assert_eq!(patch2.unchanged_count, 30);
        // 只有 3 个增量事件：1 个 Modified + 1 个 Added + 1 个 Removed
        assert_eq!(patch2.events.len(), 3);
        assert!(patch2.compressed_prompt.contains("[UNCHANGED: 30 nodes]"));
        assert!(patch2
            .compressed_prompt
            .contains("[MODIFIED] #31 | EditText | \"Replica Count: 2\" -> \"Replica Count: 4\""));
        assert!(patch2
            .compressed_prompt
            .contains("[ADDED] #32 | Modal_Button | \"Confirm Scale-Up\""));
        assert!(patch2
            .compressed_prompt
            .contains("[REMOVED] Spinner | \"Loading Metrics...\""));

        // 验证脱水率 >= 90%
        assert!(
            patch2.compression_ratio() >= 0.90,
            "期望压缩率 >= 90%，实际为 {:.1}%",
            patch2.compression_ratio() * 100.0
        );
    }
}
