use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiNode {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub children: Vec<UiNode>,
}

impl UiNode {
    pub fn attribute_value(&self, key: &str) -> Option<&Value> {
        self.attributes.get(key)
    }
}

pub fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "<invalid>".to_string())
        }
    }
}
