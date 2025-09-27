use std::collections::BTreeMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpyError {
    #[error("failed to parse ui tree: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("invalid name pattern: {0}")]
    InvalidPattern(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiNode {
    pub id: String,
    pub role: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub children: Vec<UiNode>,
}

impl UiNode {
    fn matches(&self, filter: &FilterCriteria, depth: usize) -> bool {
        if let Some(max_depth) = filter.max_depth {
            if depth > max_depth {
                return false;
            }
        }

        if !filter.roles.is_empty() {
            let node_role = if filter.ignore_role_case {
                self.role.to_lowercase()
            } else {
                self.role.clone()
            };

            let mut role_matches = false;
            for role in &filter.roles {
                if filter.ignore_role_case {
                    if node_role == *role {
                        role_matches = true;
                        break;
                    }
                } else if &self.role == role {
                    role_matches = true;
                    break;
                }
            }

            if !role_matches {
                return false;
            }
        }

        if let Some(ref pattern) = filter.compiled_name_pattern {
            let name = self.name.as_deref().unwrap_or("");
            if !pattern.is_match(name) {
                return false;
            }
        }

        for (key, expected_value) in &filter.properties {
            let Some(value) = self.properties.get(key) else {
                return false;
            };
            let matches = value
                .as_str()
                .map(|s| s == expected_value)
                .unwrap_or_else(|| value.to_string() == *expected_value);
            if !matches {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Default, Clone)]
pub struct FilterCriteria {
    pub roles: Vec<String>,
    pub name_pattern: Option<String>,
    pub compiled_name_pattern: Option<Regex>,
    pub properties: BTreeMap<String, String>,
    pub max_depth: Option<usize>,
    pub include_properties: bool,
    pub ignore_role_case: bool,
    pub ignore_name_case: bool,
}

impl FilterCriteria {
    pub fn new() -> Self {
        Self::default()
    }

    fn compile(&mut self) -> Result<(), SpyError> {
        if let Some(pattern) = &self.name_pattern {
            let regex_source = if self.ignore_name_case {
                format!("(?i){pattern}")
            } else {
                pattern.clone()
            };
            let regex = Regex::new(&regex_source)
                .map_err(|err| SpyError::InvalidPattern(err.to_string()))?;
            self.compiled_name_pattern = Some(regex);
        }
        if self.ignore_role_case {
            self.roles = self.roles.iter().map(|role| role.to_lowercase()).collect();
        }
        Ok(())
    }
}

pub fn load_ui_tree_from_str(raw: &str) -> Result<Vec<UiNode>, SpyError> {
    if raw.trim_start().starts_with('[') {
        Ok(serde_json::from_str::<Vec<UiNode>>(raw)?)
    } else if raw.trim().is_empty() {
        Ok(Vec::new())
    } else {
        let node = serde_json::from_str::<UiNode>(raw)?;
        Ok(vec![node])
    }
}

pub fn filter_tree(nodes: &[UiNode], criteria: &FilterCriteria) -> Result<Vec<UiNode>, SpyError> {
    let mut criteria = criteria.clone();
    criteria.compile()?;

    let filtered = nodes
        .iter()
        .filter_map(|node| filter_node(node, &criteria, 0))
        .collect();
    Ok(filtered)
}

fn filter_node(node: &UiNode, criteria: &FilterCriteria, depth: usize) -> Option<UiNode> {
    if let Some(max_depth) = criteria.max_depth {
        if depth > max_depth {
            return None;
        }
    }

    let mut filtered_children = Vec::new();
    if criteria.max_depth.map(|max| depth < max).unwrap_or(true) {
        filtered_children = node
            .children
            .iter()
            .filter_map(|child| filter_node(child, criteria, depth + 1))
            .collect();
    }

    let matches = node.matches(criteria, depth);
    if matches || !filtered_children.is_empty() {
        Some(UiNode {
            id: node.id.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            properties: node.properties.clone(),
            children: filtered_children,
        })
    } else {
        None
    }
}

pub fn format_tree_text(nodes: &[UiNode], include_properties: bool) -> String {
    let mut output = String::new();
    for (index, node) in nodes.iter().enumerate() {
        format_node_text(
            node,
            0,
            include_properties,
            index == nodes.len() - 1,
            &mut output,
            String::new(),
        );
    }
    output.trim_end().to_string()
}

fn format_node_text(
    node: &UiNode,
    depth: usize,
    include_properties: bool,
    is_last: bool,
    output: &mut String,
    prefix: String,
) {
    let connector = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}└── ")
    } else {
        format!("{prefix}├── ")
    };

    let label = match (&node.name, include_properties) {
        (Some(name), _) => format!(
            "[{role}] {name} <{id}>",
            role = node.role,
            name = name,
            id = node.id
        ),
        (None, _) => format!("[{role}] <{id}>", role = node.role, id = node.id),
    };
    output.push_str(&connector);
    output.push_str(&label);
    output.push('\n');

    if include_properties && !node.properties.is_empty() {
        let props_prefix = format!(
            "{prefix}{}",
            if depth == 0 {
                "    "
            } else if is_last {
                "    "
            } else {
                "│   "
            }
        );
        for (key, value) in &node.properties {
            output.push_str(&format!("{props_prefix}{key}: {value}\n"));
        }
    }

    let child_prefix = if depth == 0 {
        String::from("")
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };

    for (index, child) in node.children.iter().enumerate() {
        format_node_text(
            child,
            depth + 1,
            include_properties,
            index == node.children.len() - 1,
            output,
            child_prefix.clone(),
        );
    }
}

pub fn format_tree_json(nodes: &[UiNode]) -> Result<String, SpyError> {
    Ok(serde_json::to_string_pretty(nodes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Vec<UiNode> {
        serde_json::from_str(
            r#"
            [
                {
                    "id": "root",
                    "role": "window",
                    "name": "Calculator",
                    "properties": {"AutomationId": "rootWindow"},
                    "children": [
                        {
                            "id": "btn-1",
                            "role": "button",
                            "name": "One",
                            "properties": {"AutomationId": "num1Button"},
                            "children": []
                        },
                        {
                            "id": "btn-2",
                            "role": "button",
                            "name": "Two",
                            "properties": {"AutomationId": "num2Button"},
                            "children": []
                        },
                        {
                            "id": "panel",
                            "role": "group",
                            "name": "Memory",
                            "properties": {},
                            "children": [
                                {
                                    "id": "lbl",
                                    "role": "text",
                                    "name": "M+",
                                    "properties": {"Shortcut": "Ctrl+M"},
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
            "#,
        )
        .unwrap()
    }

    #[test]
    fn filters_by_role() {
        let tree = sample_tree();
        let mut criteria = FilterCriteria::new();
        criteria.roles = vec!["button".to_string()];
        let filtered = filter_tree(&tree, &criteria).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].children.len(), 2);
        assert!(
            filtered[0]
                .children
                .iter()
                .all(|node| node.role == "button")
        );
    }

    #[test]
    fn filters_by_name_regex() {
        let tree = sample_tree();
        let mut criteria = FilterCriteria::new();
        criteria.name_pattern = Some("^M".to_string());
        let filtered = filter_tree(&tree, &criteria).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].children.len(), 1);
        assert_eq!(filtered[0].children[0].name.as_deref(), Some("Memory"));
    }

    #[test]
    fn respects_property_filters() {
        let tree = sample_tree();
        let mut criteria = FilterCriteria::new();
        criteria
            .properties
            .insert("AutomationId".into(), "num2Button".into());
        let filtered = filter_tree(&tree, &criteria).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].children.len(), 1);
        assert_eq!(filtered[0].children[0].id, "btn-2");
    }
}
