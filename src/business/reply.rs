//! 插件回复归一化（评审建议 5）：把 handle() 的返回值变成出站消息列表。
//!
//! v1：list[str]，每个字符串一条 text 段消息（存量语义不变）。
//! v2：str → 一条单 text 段消息；list → **一条**消息（扁平段列表，元素为
//!     单键 dict，键见 SEGMENT_KINDS；str 元素宽容视为 text 段）；
//!     None / "" / [] → 空（不归我管，链继续）。
//! 段为 protojson 形态，与入站 ctx["segments"] 同构，宿主原样序列化进
//! SendMessageRequest.segments——发图/引用/at 不再需要 send_action 逃生舱。
//! 嵌套树刻意不支持（提案 §7）：嵌套归适配器，段列表保持扁平。

use std::error::Error;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use serde_json::Value;

use super::Message;

/// 段类型键（adapter.proto 的 Segment oneof），v2 段 dict 的唯一键必须在此集合内。
pub(crate) const SEGMENT_KINDS: [&str; 9] = [
    "text",
    "at",
    "reply",
    "image",
    "face",
    "voice",
    "video",
    "file",
    "extension",
];

/// 单 text 段消息。
pub(crate) fn text_message(s: &str) -> Message {
    vec![serde_json::json!({ "text": { "text": s } })]
}

/// 校验扁平段列表：每个元素是恰好一个已知键、内容为对象的 dict。
/// 与 from_python 解耦：无 Python 依赖，纯逻辑可单测。
pub(crate) fn validate_segments(segments: &[Value]) -> Result<(), String> {
    for (i, seg) in segments.iter().enumerate() {
        let Some(obj) = seg.as_object() else {
            return Err(format!("第 {i} 段不是对象"));
        };
        if obj.len() != 1 {
            return Err(format!("第 {i} 段应恰好一个键，实际 {}", obj.len()));
        }
        let (kind, body) = obj.iter().next().expect("len == 1 已检查");
        if !SEGMENT_KINDS.contains(&kind.as_str()) {
            return Err(format!("第 {i} 段类型未知: {kind}"));
        }
        if !body.is_object() {
            return Err(format!("第 {i} 段 {kind} 的内容不是对象"));
        }
    }
    Ok(())
}

/// handle() 返回值 → 出站消息列表，按插件 API 版本解释。
pub(super) fn from_python(
    py: Python<'_>,
    ret: &Bound<'_, PyAny>,
    api_version: u8,
) -> Result<Vec<Message>, Box<dyn Error>> {
    if api_version < 2 {
        let strings: Vec<String> = ret.extract()?;
        return Ok(strings.iter().map(|s| text_message(s)).collect());
    }
    if ret.is_none() {
        return Ok(Vec::new());
    }
    if let Ok(s) = ret.extract::<String>() {
        return Ok(if s.is_empty() {
            Vec::new()
        } else {
            vec![text_message(&s)]
        });
    }
    let Ok(list) = ret.downcast::<PyList>() else {
        return Err(format!(
            "返回类型 {} 不支持（v2 可返回 str / list[段] / None）",
            ret.get_type()
        )
        .into());
    };
    if list.is_empty() {
        return Ok(Vec::new());
    }
    // dict 元素走 json 字符串中转（同 ctx.segments 注入的手法：serde_json::Value
    // 无 IntoPyObject）；str 元素宽容视为 text 段——AI 生成的插件最常写错的形状。
    let json_mod = py.import("json")?;
    let mut segments: Vec<Value> = Vec::with_capacity(list.len());
    for item in list.iter() {
        if let Ok(s) = item.extract::<String>() {
            segments.push(serde_json::json!({ "text": { "text": s } }));
            continue;
        }
        if item.downcast::<PyDict>().is_err() {
            return Err(
                format!("段元素类型 {} 不支持（应为 dict 或 str）", item.get_type()).into(),
            );
        }
        let text: String = json_mod.call_method1("dumps", (&item,))?.extract()?;
        segments.push(serde_json::from_str(&text)?);
    }
    validate_segments(&segments).map_err(|e| format!("段列表不合法: {e}"))?;
    Ok(vec![segments])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_accepts_known_single_key_objects() {
        let segs = vec![
            json!({ "text": { "text": "看这个" } }),
            json!({ "image": { "url": "https://x/y.png" } }),
            json!({ "reply": { "messageId": "m1" } }),
        ];
        assert!(validate_segments(&segs).is_ok());
    }

    #[test]
    fn validate_rejects_unknown_key_and_shapes() {
        assert!(
            validate_segments(&[json!({ "bogus": {} })]).is_err(),
            "未知键"
        );
        assert!(
            validate_segments(&[json!({ "text": {}, "at": {} })]).is_err(),
            "双键"
        );
        assert!(
            validate_segments(&[json!("纯字符串")]).is_err(),
            "裸字符串段"
        );
        assert!(
            validate_segments(&[json!({ "image": "https://x" })]).is_err(),
            "内容非对象"
        );
        assert!(validate_segments(&[json!({})]).is_err(), "零键");
    }

    #[test]
    fn text_message_shape_matches_inbound_schema() {
        assert_eq!(
            text_message("早"),
            vec![json!({ "text": { "text": "早" } })]
        );
    }
}
