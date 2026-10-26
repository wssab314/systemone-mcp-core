//! # 第一课：所有权 (Ownership)、借用 (Borrowing)、生命周期 ('a) 与内存布局
//!
//! 结合移动端 GUI Agent 的 SoM (Set-of-Mark) 控件树解析与高吞吐日志流解析场景，直观理解：
//! 1. 栈 (Stack) 与 堆 (Heap) 的内存开销对比 (`std::mem::size_of`)
//! 2. 所有权转移 (Move) vs 不可变借用 (`&T`) vs 可变借用 (`&mut T`)
//! 3. 生命周期标注 `'a`：为什么零拷贝 (Zero-Copy) 解析器在 AI 推理网关与分布式存储引擎中至关重要？

use std::mem::size_of;

/// 演示 `String`（堆分配，24 字节胖指针：ptr + len + cap）与 `&str`（16 字节切片引用：ptr + len）的内存布局差异
pub fn inspect_memory_layout() -> (usize, usize, usize) {
    let size_of_string = size_of::<String>();
    let size_of_str_slice = size_of::<&str>();
    let size_of_box_u64 = size_of::<Box<u64>>();
    (size_of_string, size_of_str_slice, size_of_box_u64)
}

/// 零拷贝控件节点视图：通过生命周期 `'a` 直接借用原始 XML/UI Hierarchy 缓冲区中的字符串切片，
/// 避免在解析成百上千个 DOM 节点时发生任何堆内存分配 (`malloc`/`clone`)。
#[derive(Debug, PartialEq, Eq)]
pub struct ZeroCopyDomNode<'a> {
    pub tag: &'a str,
    pub label: &'a str,
    pub bounds: (u32, u32, u32, u32),
}

impl<'a> ZeroCopyDomNode<'a> {
    /// 从形如 `Button|Deploy Model (vLLM)|80,920,650,1020` 的单行文本中零拷贝解析出结构体
    pub fn parse_line(line: &'a str) -> Option<Self> {
        let mut parts = line.split('|');
        let tag = parts.next()?.trim();
        let label = parts.next()?.trim();
        let bounds_str = parts.next()?.trim();

        let mut coords = bounds_str
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok());
        let x1 = coords.next()?;
        let y1 = coords.next()?;
        let x2 = coords.next()?;
        let y2 = coords.next()?;

        Some(Self {
            tag,
            label,
            bounds: (x1, y1, x2, y2),
        })
    }

    /// 计算控件中心点像素坐标 `(center_x, center_y)`
    pub fn center_point(&self) -> (u32, u32) {
        let (x1, y1, x2, y2) = self.bounds;
        ((x1 + x2) / 2, (y1 + y2) / 2)
    }

    /// 计算控件边界框的像素面积 `(x2 - x1) * (y2 - y1)`
    /// 使用 `saturating_sub` 防止异常坐标 (`x2 < x1`) 导致无符号整数 `u32` 下溢 panic
    pub fn area(&self) -> u32 {
        let (x1, y1, x2, y2) = self.bounds;
        let width = x2.saturating_sub(x1);
        let height = y2.saturating_sub(y1);
        width.saturating_mul(height)
    }
}

/// 使用零成本迭代器链（Iterator Chain）批量解析并过滤掉面积小于 `min_area` 的 UI 噪点（如 1px 分割线）
pub fn parse_and_filter_som_nodes<'a>(
    raw_dump: &'a str,
    min_area: u32,
) -> Vec<(usize, ZeroCopyDomNode<'a>)> {
    raw_dump
        .lines()
        .filter_map(ZeroCopyDomNode::parse_line)
        .filter(|node| node.area() >= min_area && !node.label.is_empty())
        .enumerate()
        .map(|(idx, node)| (idx + 1, node))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_layout_sizes() {
        let (string_sz, str_sz, box_sz) = inspect_memory_layout();
        // 在 64 位架构 (Apple M3 Max aarch64) 下：
        // String = ptr(8) + len(8) + cap(8) = 24 bytes
        // &str   = ptr(8) + len(8)          = 16 bytes
        // Box<T> = ptr(8)                   = 8 bytes
        assert_eq!(string_sz, 24);
        assert_eq!(str_sz, 16);
        assert_eq!(box_sz, 8);
    }

    #[test]
    fn test_zero_copy_dom_parsing_and_area() {
        let raw_buffer = String::from("Button | Deploy Instance (A100-80G) | 80,920,650,1020");
        // node.tag 和 node.label 直接指向 raw_buffer 堆内存中的字节区间，0 次新堆分配！
        let node = ZeroCopyDomNode::parse_line(&raw_buffer).expect("解析失败");
        assert_eq!(node.tag, "Button");
        assert_eq!(node.label, "Deploy Instance (A100-80G)");
        assert_eq!(node.bounds, (80, 920, 650, 1020));
        assert_eq!(node.center_point(), (365, 970));
        // 宽 (650 - 80 = 570) * 高 (1020 - 920 = 100) = 57,000 px^2
        assert_eq!(node.area(), 57_000);
    }

    #[test]
    fn test_filter_out_tiny_noise_dividers() {
        let dump = "\
            Divider | | 0,500,1080,501\n\
            EditText | Model Endpoint Name | 80,520,650,610\n\
            Icon | NoiseDot | 10,10,12,12\n\
            Button | Launch Cluster | 80,920,650,1020";

        // 过滤掉面积小于 100 像素或无文本标签的分割线/噪点
        let valid_nodes = parse_and_filter_som_nodes(dump, 100);
        assert_eq!(valid_nodes.len(), 2);
        assert_eq!(valid_nodes[0].0, 1);
        assert_eq!(valid_nodes[0].1.label, "Model Endpoint Name");
        assert_eq!(valid_nodes[1].0, 2);
        assert_eq!(valid_nodes[1].1.label, "Launch Cluster");
    }

    #[test]
    fn test_ownership_borrow_semantics() {
        let goal = String::from("在云端 AI 控制台创建 Qwen-72B 推理集群并设置副本数为 4");

        fn compute_goal_bytes(text: &str) -> usize {
            text.len()
        }

        let byte_len = compute_goal_bytes(&goal);
        // 因为使用了不可变借用 `&goal`，`goal` 的所有权依然保留在当前作用域！
        assert!(byte_len > 0);
        assert!(goal.contains("Qwen-72B"));
    }
}
