use std::collections::HashMap;

use systemone_mcp_core::context_compressor::UiDeltaCompressor;
use systemone_mcp_core::gate_engine::{ConfidenceGateEngine, GateConfig, LoopDecision};
use systemone_mcp_core::ownership_lab::{inspect_memory_layout, parse_and_filter_som_nodes};
use systemone_mcp_core::protocol::{PrimitiveResult, SystemOneResponse};

fn main() {
    println!("=========================================================================");
    println!("🦀 systemone-mcp-core | 极速 UI Skills & Delta 上下文压缩 MCP 引擎");
    println!("=========================================================================\n");

    // 1. 内存布局实验：验证栈上胖指针与引用切片字节大小
    let (string_sz, str_sz, box_sz) = inspect_memory_layout();
    println!(
        "[1] Apple M3 Max (aarch64) 内存布局实测:\n    • size_of::<String>()  = {} bytes (ptr + len + cap)\n    • size_of::<&str>()    = {} bytes (ptr + len, 零拷贝视图)\n    • size_of::<Box<u64>>() = {} bytes (堆指针)\n",
        string_sz, str_sz, box_sz
    );

    // 2. 零拷贝 SoM 候选控件树解析（自动过滤 1px 分割线与噪点）
    let raw_dom_dump = "\
        Divider | | 0,480,1080,481\n\
        Tab | AI Compute Instances | 210,1800,420,1920\n\
        EditText | Model ID (Qwen2.5-72B-Instruct) | 80,520,650,610\n\
        EditText | Replica Count (4x H100) | 80,640,650,730\n\
        Button | Deploy Endpoint | 80,920,650,1020\n\
        Modal_Button | Confirm Provisioning | 140,1100,590,1190";

    println!("[2] 零拷贝解析移动端 SoM (Set-of-Mark) 候选锚点 (自动过滤 <100px² 噪点):");
    let candidates = parse_and_filter_som_nodes(raw_dom_dump, 100);

    for (som_id, node) in &candidates {
        let (cx, cy) = node.center_point();
        let area = node.area();
        println!(
            "    #{som_id}: [{tag}] \"{label}\" -> bounds={bounds:?}, area={area}px², tap_center=({cx}, {cy})",
            tag = node.tag,
            label = node.label,
            bounds = node.bounds
        );
    }
    println!();

    // 3. 演练同构方向 3：OrderBook 级增量 UI 上下文压缩器 (UiDeltaCompressor)
    println!("[3] 演练 UiDeltaCompressor (帧间增量差分压缩):");
    let mut compressor = UiDeltaCompressor::new(100);
    let mut f1 = Vec::new();
    let mut f2 = Vec::new();
    for i in 0..40 {
        let card = format!(
            "StaticCard | Worker Node #{i:02} (Ready, 80GB VRAM) | 20,{y1},1060,{y2}",
            y1 = 100 + i * 35,
            y2 = 130 + i * 35
        );
        let div = format!("Divider | | 0,{y},1080,{y}", y = 99 + i * 35);
        f1.push(card.clone());
        f1.push(div.clone());
        f2.push(card);
        f2.push(div);
    }
    f1.push("EditText | Replica Count: 2 | 80,1550,650,1640".to_string());
    f1.push("Spinner | Fetching GPU Quota... | 400,1660,680,1720".to_string());

    f2.push("EditText | Replica Count: 4 | 80,1550,650,1640".to_string());
    f2.push("Modal_Button | Confirm Provisioning | 140,1750,590,1840".to_string());

    let _patch1 = compressor.compress_frame(&f1.join("\n"));
    let patch2 = compressor.compress_frame(&f2.join("\n"));
    println!(
        "    • 原始第 2 帧大小: {} bytes -> 增量压缩后大小: {} bytes (脱水率: {:.1}%)",
        patch2.raw_bytes,
        patch2.compressed_prompt.len(),
        patch2.compression_ratio() * 100.0
    );
    for line in patch2.compressed_prompt.lines() {
        println!("      {line}");
    }
    println!();

    // 4. 模拟 System 1 (/v1/systemone) 单次前向传播闭环状态机
    println!("[4] 启动 System 1 (noul / choice / score) 置信度门控状态机演练:");
    let mut engine = ConfidenceGateEngine::new(GateConfig::default());

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
    let step3_tap_deploy =
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
        ("Step 2 (突发通知弹窗)", &step2_popup, false),
        ("Step 3 (参数就绪待部署)", &step3_tap_deploy, false),
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
