//! # 第二课：代数枚举 (`enum`)、模式匹配 (`match`) 与 `serde` 强类型协议解析
//!
//! 在 `Laya-Vision` / `Vev` 的 `/v1/systemone` 接口中，单次前向传播返回三种异构的决策原语：
//! - `noul`：伯努利二元真假概率 $P(\text{true}) \in [0.0, 1.0]$
//! - `choice`：封闭集多选一概率分布与 `top_choice`
//! - `score`：量表评估期望得分（如 1.0 ~ 5.0）
//!
//! 通过 Rust 的 `enum` 结合 `#[serde(tag = "type", rename_all = "lowercase")]`（内部标签枚举），
//! 编译器能够自动根据 JSON 中的 `"type": "noul" | "choice" | "score"` 字段完成零样板代码的强类型反序列化。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// `/v1/systemone` 请求端的问题原语定义（支持混合打包多个问题单次前向推理）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum QuestionSpec {
    Noul {
        id: String,
        question: String,
    },
    Choice {
        id: String,
        question: String,
        options: Vec<String>,
    },
    Score {
        id: String,
        question: String,
        rubric: HashMap<String, String>,
    },
}

/// System 1 三大决策原语的返回结果枚举（Internally Tagged Enum）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PrimitiveResult {
    /// 伯努利二元概率原语：返回命题为真的校准概率 `p_true`
    Noul { p_true: f64 },
    /// 封闭集多选一路由原语：返回每个候选编号的概率分布及最高概率选项
    Choice {
        probabilities: HashMap<String, f64>,
        top_choice: String,
    },
    /// 量表评估原语：返回根据 Rubric 计算出的期望得分
    Score { expected_score: f64 },
}

/// 单次 `/v1/systemone` 批量评估响应包
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemOneResponse {
    pub results: HashMap<String, PrimitiveResult>,
}

impl SystemOneResponse {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    /// 直接从 `/v1/systemone` 返回的 JSON 字符串反序列化为强类型结构体
    pub fn from_json_str(raw_json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw_json)
    }

    pub fn insert(&mut self, question_id: impl Into<String>, result: PrimitiveResult) {
        self.results.insert(question_id.into(), result);
    }

    /// 提取指定 `question_id` 的 `noul` 概率值；若不存在或类型不匹配则返回 `0.0`
    pub fn noul(&self, question_id: &str) -> f64 {
        match self.results.get(question_id) {
            Some(PrimitiveResult::Noul { p_true }) => *p_true,
            _ => 0.0,
        }
    }

    /// 提取指定 `question_id` 的 `choice` 最优选项及其置信度 `(top_choice, confidence)`
    pub fn top_choice(&self, question_id: &str) -> Option<(&str, f64)> {
        match self.results.get(question_id) {
            Some(PrimitiveResult::Choice {
                probabilities,
                top_choice,
            }) => {
                let confidence = probabilities.get(top_choice).copied().unwrap_or(0.0);
                Some((top_choice.as_str(), confidence))
            }
            _ => None,
        }
    }

    /// 提取指定 `question_id` 的 `score` 期望得分
    pub fn score(&self, question_id: &str) -> Option<f64> {
        match self.results.get(question_id) {
            Some(PrimitiveResult::Score { expected_score }) => Some(*expected_score),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_one_primitives_extraction() {
        let mut resp = SystemOneResponse::new();
        resp.insert("is_loading", PrimitiveResult::Noul { p_true: 0.03 });
        resp.insert(
            "next_element_id",
            PrimitiveResult::Choice {
                probabilities: HashMap::from([
                    ("1".to_string(), 0.02),
                    ("4".to_string(), 0.91),
                    ("NONE".to_string(), 0.07),
                ]),
                top_choice: "4".to_string(),
            },
        );
        resp.insert(
            "progress_score",
            PrimitiveResult::Score {
                expected_score: 3.05,
            },
        );

        assert!((resp.noul("is_loading") - 0.03).abs() < 1e-6);
        assert_eq!(resp.top_choice("next_element_id"), Some(("4", 0.91)));
        assert_eq!(resp.score("progress_score"), Some(3.05));
    }

    #[test]
    fn test_serde_json_deserialization_from_v1_systemone() {
        let raw_payload = r#"{
            "results": {
                "is_loading": { "type": "noul", "p_true": 0.03 },
                "has_blocking_popup": { "type": "noul", "p_true": 0.01 },
                "is_goal_completed": { "type": "noul", "p_true": 0.04 },
                "next_element_id": {
                    "type": "choice",
                    "probabilities": {
                        "1": 0.01, "2": 0.02, "3": 0.02,
                        "4": 0.91, "5": 0.01, "6": 0.02, "NONE": 0.01
                    },
                    "top_choice": "4"
                },
                "progress_score": { "type": "score", "expected_score": 3.05 }
            }
        }"#;

        let resp = SystemOneResponse::from_json_str(raw_payload).expect("JSON 反序列化应成功");
        assert!((resp.noul("is_loading") - 0.03).abs() < 1e-6);
        assert!((resp.noul("has_blocking_popup") - 0.01).abs() < 1e-6);
        assert_eq!(resp.top_choice("next_element_id"), Some(("4", 0.91)));
        assert_eq!(resp.score("progress_score"), Some(3.05));
    }
}
