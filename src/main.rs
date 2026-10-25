pub mod gate_engine;
pub mod ownership_lab;
pub mod protocol;

use std::collections::HashMap;

use gate_engine::{ConfidenceGateEngine, GateConfig, LoopDecision};
use ownership_lab::{inspect_memory_layout, ZeroCopyDomNode};
use protocol::{PrimitiveResult, SystemOneResponse};

fn main() {
    println!("=========================================================================");
    println!("🦀 Rust_Chain_Lab | Step 01: Rust 内存布局与 System 1 (Laya-Vision) 决策状态机");
    println!("=========================================================================\n");

    // 1. 内存布局实验：验证栈上胖指针与引用切片字节大小
    let (string_sz, str_sz, box_sz) = inspect_memory_layout();
    println!(
        "[1] Apple M3 Max (aarch64) 内存布局实测:\n    • size_of::<String>()  = {} bytes (ptr + len + cap)\n    • size_of::<&str>()    = {} bytes (ptr + len, 零拷贝视图)\n    • size_of::<Box<u64>>() = {} bytes (堆指针)\n",
        string_sz, str_sz, box_sz
    );

    // 2. 零拷贝 SoM 候选控件树解析
    let raw_dom_dump = "\
        Tab | 合约 (Futures) | 210,1800,420,1920\n\
        EditText | 输入价格 (65000 USDT) | 80,520,650,610\n\
        EditText | 输入数量 (0.1 BTC) | 80,640,650,730\n\
        Button | 买入做多 (Buy Long) | 80,920,650,1020\n\
        Modal_Button | 二次确认弹窗-确认下单 | 140,1100,590,1190";

    println!("[2] 零拷贝解析移动端 SoM (Set-of-Mark) 候选锚点:");
    let candidates: Vec<(usize, ZeroCopyDomNode<'_>)> = raw_dom_dump
        .lines()
        .filter_map(ZeroCopyDomNode::parse_line)
        .enumerate()
        .map(|(idx, node)| (idx + 1, node))
        .collect();

    for (som_id, node) in &candidates {
        let (cx, cy) = node.center_point();
        println!(
            "    #{som_id}: [{tag}] \"{label}\" -> bounds={bounds:?}, tap_center=({cx}, {cy})",
            tag = node.tag,
            label = node.label,
            bounds = node.bounds
        );
    }
    println!();

    // 3. 模拟 System 1 (/v1/systemone) 单次前向传播闭环状态机
    println!("[3] 启动 System 1 (noul / choice / score) 置信度门控状态机演练:");
    let mut engine = ConfidenceGateEngine::new(GateConfig::default());

    // 构造模拟轨迹：加载等待 -> 弹窗拦截 -> 点击买入按钮(#4) -> 确认弹窗(#5) -> 旁路 API 201 完成
    let mut step1_loading = SystemOneResponse::new();
    step1_loading.insert("is_loading", PrimitiveResult::Noul { p_true: 0.89 });

    let mut step2_popup = SystemOneResponse::new();
    step2_popup.insert("is_loading", PrimitiveResult::Noul { p_true: 0.02 });
    step2_popup.insert("has_blocking_popup", PrimitiveResult::Noul { p_true: 0.92 });

    let raw_step3_json = r#"{
        "results": {
            "is_loading": { "type": "noul", "p_true": 0.03 },
            "has_blocking_popup": { "type": "noul", "p_true": 0.01 },
            "is_goal_completed": { "type": "noul", "p_true": 0.04 },
            "next_element_id": {
                "type": "choice",
                "probabilities": { "4": 0.91, "2": 0.05, "NONE": 0.04 },
                "top_choice": "4"
            },
            "progress_score": { "type": "score", "expected_score": 3.05 }
        }
    }"#;
    let step3_tap_buy =
        SystemOneResponse::from_json_str(raw_step3_json).expect("解析 /v1/systemone JSON 失败");

    let mut step4_confirm_modal = SystemOneResponse::new();
    step4_confirm_modal.insert("is_loading", PrimitiveResult::Noul { p_true: 0.01 });
    step4_confirm_modal.insert("has_blocking_popup", PrimitiveResult::Noul { p_true: 0.02 });
    step4_confirm_modal.insert("is_goal_completed", PrimitiveResult::Noul { p_true: 0.15 });
    step4_confirm_modal.insert(
        "next_element_id",
        PrimitiveResult::Choice {
            probabilities: HashMap::from([("5".to_string(), 0.96)]),
            top_choice: "5".to_string(),
        },
    );
    step4_confirm_modal.insert(
        "progress_score",
        PrimitiveResult::Score {
            expected_score: 4.10,
        },
    );

    let step5_done = SystemOneResponse::new();

    let trajectory = [
        ("Step 1 (骨架屏加载中)", &step1_loading, false),
        ("Step 2 (突发广告弹窗)", &step2_popup, false),
        ("Step 3 (表单就绪待下单)", &step3_tap_buy, false),
        ("Step 4 (二次确认弹窗)", &step4_confirm_modal, false),
        ("Step 5 (旁路捕获 HTTP 201)", &step5_done, true),
    ];

    for (label, resp, api_ok) in trajectory {
        let decision = engine.evaluate_step(resp, api_ok);
        match decision {
            LoopDecision::WaitLoading { sleep_ms } => {
                println!("    ⏳ {label} -> [noul 守卫拦截] 本地微睡眠 {sleep_ms}ms，0 Token 消耗");
            }
            LoopDecision::DismissPopup => {
                println!("    🛡️ {label} -> [noul 弹窗拦截] 本地极速点掉遮挡层，0 Token 消耗");
            }
            LoopDecision::ExecuteLocalAction {
                element_id,
                confidence,
            } => {
                println!(
                    "    ⚡ {label} -> [choice 门控通过 P={confidence:.2} >= 0.82] 本地 35ms 直接点击锚点 #{element_id}"
                );
            }
            LoopDecision::Completed { reason } => {
                println!("    ✅ {label} -> [任务闭环完成] 原因: {reason}");
            }
            LoopDecision::FallbackToSystemTwo { reason } => {
                println!("    🧠 {label} -> [回退云端 System 2] 原因: {reason}");
            }
        }
    }
}
