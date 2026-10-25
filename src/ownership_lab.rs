//! # 第一课：所有权 (Ownership)、借用 (Borrowing)、生命周期 ('a) 与内存布局
//!
//! 结合移动端 MCP 的 SoM 控件解析与行情 Tick 解析场景，直观理解：
//! 1. 栈 (Stack) 与 堆 (Heap) 的内存开销对比 (`std::mem::size_of`)
//! 2. 所有权转移 (Move) vs 不可变借用 (`&T`) vs 可变借用 (`&mut T`)
//! 3. 生命周期标注 `'a`：为什么零拷贝 (Zero-Copy) 解析器在高频交易与公链节点中至关重要？

use std::mem::size_of;

/// 演示 `String`（堆分配，24 字节胖指针：ptr + len + cap）与 `&str`（16 字节切片引用：ptr + len）的内存布局差异
pub fn inspect_memory_layout() -> (usize, usize, usize) {
    let size_of_string = size_of::<String>();
    let size_of_str_slice = size_of::<&str>();
    let size_of_box_u64 = size_of::<Box<u64>>();
    (size_of_string, size_of_str_slice, size_of_box_u64)
}

/// 零拷贝控件节点视图：通过生命周期 `'a` 直接借用原始 XML/日志缓冲区中的字符串切片，
/// 避免在解析成百上千个 DOM 节点时发生任何堆内存分配 (`malloc`/`clone`)。
#[derive(Debug, PartialEq, Eq)]
pub struct ZeroCopyDomNode<'a> {
    pub tag: &'a str,
    pub label: &'a str,
    pub bounds: (u32, u32, u32, u32),
}

impl<'a> ZeroCopyDomNode<'a> {
    /// 从形如 `Button|买入做多 (Buy Long)|80,920,650,1020` 的单行文本中零拷贝解析出结构体
    pub fn parse_line(line: &'a str) -> Option<Self> {
        let mut parts = line.split('|');
        let tag = parts.next()?.trim();
        let label = parts.next()?.trim();
        let bounds_str = parts.next()?.trim();

        let mut coords = bounds_str.split(',').filter_map(|s| s.trim().parse::<u32>().ok());
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
    fn test_zero_copy_dom_parsing() {
        let raw_buffer = String::from("Button | 买入做多 (Buy Long) | 80,920,650,1020");
        // node.tag 和 node.label 直接指向 raw_buffer 堆内存中的字节区间，0 次新堆分配！
        let node = ZeroCopyDomNode::parse_line(&raw_buffer).expect("解析失败");
        assert_eq!(node.tag, "Button");
        assert_eq!(node.label, "买入做多 (Buy Long)");
        assert_eq!(node.bounds, (80, 920, 650, 1020));
        assert_eq!(node.center_point(), (365, 970));
    }

    /// 【实战练习 1】理解所有权转移与借用：
    /// 运行 `cargo test -- --ignored` 挑战此题！
    #[test]
    #[ignore]
    fn exercise_01_fix_ownership_borrow() {
        let goal = String::from("在 BTC/USDT 永续合约页限价 65000 买入 0.1 BTC");

        // 思考：如果一个函数只需要读取字符串内容，参数应该声明为 `String` 还是 `&str`？
        fn compute_goal_bytes(text: &str) -> usize {
            text.len()
        }

        let byte_len = compute_goal_bytes(&goal);
        // 因为使用了不可变借用 `&goal`，`goal` 的所有权依然保留在当前作用域！
        assert!(byte_len > 0);
        assert!(goal.contains("BTC/USDT"));
    }
}
