//! # 第三课：状态机设计、可变引用 (`&mut self`) 与 System 1 置信度门控引擎
//!
//! 完整实现基于 System 1 (`/v1/systemone`) 的本地高速决策闭环：
//! 1. **终点判定 (`is_goal_completed >= 0.88` 或旁路 API 确认)**
//! 2. **加载等待守卫 (`is_loading >= 0.75`)** -> 本地微睡眠重试，0 云端 Token 消耗
//! 3. **异常弹窗守卫 (`has_blocking_popup >= 0.80`)** -> 本地极速关闭遮挡弹窗
//! 4. **防死循环熔断器 (`ProgressTracker`)** -> 连续 N 步 `progress_score` 未增长则中断回退 System 2
//! 5. **置信度门控 (`confidence >= 0.82`)** -> 高置信度本地直接点击候选锚点，低置信度回退云端大模型

use crate::protocol::SystemOneResponse;

/// 单步决策循环的执行动作指令
#[derive(Debug, Clone, PartialEq)]
pub enum LoopDecision {
    /// 任务已达成（视觉确认或旁路网络嗅探确认）
    Completed { reason: &'static str },
    /// 页面正在加载（骨架屏/转圈），本地微睡眠等待后继续，不唤醒云端大模型
    WaitLoading { sleep_ms: u64 },
    /// 检测到非预期遮挡弹窗，触发本地快速关闭逻辑
    DismissPopup,
    /// System 1 高置信度命中候选元素，本地毫秒级直接执行动作
    ExecuteLocalAction {
        element_id: String,
        confidence: f64,
    },
    /// 遇到连续停滞或低置信度陌生界面，安全回退给云端 System 2 大模型
    FallbackToSystemTwo { reason: String },
}

/// 防死循环进度追踪器：监控 `score` 原语输出的期望得分是否停滞
#[derive(Debug, Default, Clone)]
pub struct ProgressTracker {
    history: Vec<f64>,
    min_delta: f64,
}

impl ProgressTracker {
    pub fn new(min_delta: f64) -> Self {
        Self {
            history: Vec::new(),
            min_delta,
        }
    }

    /// 记录最新一步的期望进度分
    pub fn record(&mut self, score: f64) {
        self.history.push(score);
    }

    /// 判断最近连续 `window` 步是否未取得显著进展（防原地打转死循环）
    pub fn is_stagnant(&self, window: usize) -> bool {
        if window < 2 || self.history.len() < window {
            return false;
        }
        let recent = &self.history[self.history.len() - window..];
        let first = recent[0];
        let max_in_window = recent
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        (max_in_window - first) < self.min_delta
    }
}

/// System 1 置信度门控阈值配置
#[derive(Debug, Clone)]
pub struct GateConfig {
    pub goal_completed_threshold: f64,
    pub loading_threshold: f64,
    pub popup_threshold: f64,
    pub choice_confidence_threshold: f64,
    pub stagnant_window: usize,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            goal_completed_threshold: 0.88,
            loading_threshold: 0.75,
            popup_threshold: 0.80,
            choice_confidence_threshold: 0.82,
            stagnant_window: 3,
        }
    }
}

/// System 1 本地决策状态机引擎
pub struct ConfidenceGateEngine {
    pub config: GateConfig,
    pub tracker: ProgressTracker,
}

impl ConfidenceGateEngine {
    pub fn new(config: GateConfig) -> Self {
        Self {
            config,
            tracker: ProgressTracker::new(0.15),
        }
    }

    /// 核心单步评估函数：输入 `/v1/systemone` 响应与旁路 API 状态，输出毫秒级路由决策
    pub fn evaluate_step(
        &mut self,
        resp: &SystemOneResponse,
        api_order_succeeded: bool,
    ) -> LoopDecision {
        // 1. 终点判定：旁路 API 已确认成功，或视觉 noul 高置信度判定完成
        if api_order_succeeded {
            return LoopDecision::Completed {
                reason: "旁路网络嗅探捕获到上游 API 201 成功响应",
            };
        }
        if resp.noul("is_goal_completed") >= self.config.goal_completed_threshold {
            return LoopDecision::Completed {
                reason: "System 1 noul(is_goal_completed) 达到完成阈值",
            };
        }

        // 2. 加载等待守卫
        if resp.noul("is_loading") >= self.config.loading_threshold {
            return LoopDecision::WaitLoading { sleep_ms: 180 };
        }

        // 3. 异常弹窗守卫
        if resp.noul("has_blocking_popup") >= self.config.popup_threshold {
            return LoopDecision::DismissPopup;
        }

        // 4. 防死循环熔断检查
        if let Some(score) = resp.score("progress_score") {
            self.tracker.record(score);
            if self.tracker.is_stagnant(self.config.stagnant_window) {
                return LoopDecision::FallbackToSystemTwo {
                    reason: format!(
                        "进度评分连续 {} 步停滞 (最新得分: {:.2})，触发防死循环熔断",
                        self.config.stagnant_window, score
                    ),
                };
            }
        }

        // 5. 置信度门控路由 (Confidence Gating)
        match resp.top_choice("next_element_id") {
            Some((best_id, confidence))
                if confidence >= self.config.choice_confidence_threshold && best_id != "NONE" =>
            {
                LoopDecision::ExecuteLocalAction {
                    element_id: best_id.to_string(),
                    confidence,
                }
            }
            Some((best_id, confidence)) => LoopDecision::FallbackToSystemTwo {
                reason: format!(
                    "System 1 候选元素 #{best_id} 置信度 ({confidence:.2}) 低于门控阈值 {:.2}",
                    self.config.choice_confidence_threshold
                ),
            },
            None => LoopDecision::FallbackToSystemTwo {
                reason: "缺失 next_element_id 路由结果".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PrimitiveResult;
    use std::collections::HashMap;

    #[test]
    fn test_high_confidence_local_execution() {
        let mut engine = ConfidenceGateEngine::new(GateConfig::default());
        let mut resp = SystemOneResponse::new();
        resp.insert("is_loading", PrimitiveResult::Noul { p_true: 0.03 });
        resp.insert("has_blocking_popup", PrimitiveResult::Noul { p_true: 0.01 });
        resp.insert("is_goal_completed", PrimitiveResult::Noul { p_true: 0.04 });
        resp.insert(
            "next_element_id",
            PrimitiveResult::Choice {
                probabilities: HashMap::from([("4".to_string(), 0.91)]),
                top_choice: "4".to_string(),
            },
        );
        resp.insert(
            "progress_score",
            PrimitiveResult::Score {
                expected_score: 3.05,
            },
        );

        let decision = engine.evaluate_step(&resp, false);
        assert_eq!(
            decision,
            LoopDecision::ExecuteLocalAction {
                element_id: "4".to_string(),
                confidence: 0.91
            }
        );
    }

    #[test]
    fn test_stagnant_progress_circuit_breaker() {
        let mut engine = ConfidenceGateEngine::new(GateConfig::default());
        let make_resp = |score: f64| {
            let mut r = SystemOneResponse::new();
            r.insert(
                "progress_score",
                PrimitiveResult::Score {
                    expected_score: score,
                },
            );
            r.insert(
                "next_element_id",
                PrimitiveResult::Choice {
                    probabilities: HashMap::from([("2".to_string(), 0.95)]),
                    top_choice: "2".to_string(),
                },
            );
            r
        };

        // 第 1、2 步正常执行，但进度一直卡在 2.0 左右
        assert!(matches!(
            engine.evaluate_step(&make_resp(2.00), false),
            LoopDecision::ExecuteLocalAction { .. }
        ));
        assert!(matches!(
            engine.evaluate_step(&make_resp(2.05), false),
            LoopDecision::ExecuteLocalAction { .. }
        ));
        // 第 3 步仍然卡在 2.02（增长 < 0.15），立即触发防死循环熔断并回退 System 2！
        let third = engine.evaluate_step(&make_resp(2.02), false);
        assert!(matches!(third, LoopDecision::FallbackToSystemTwo { .. }));
    }
}
