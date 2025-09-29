use std::collections::HashSet;

use platynui_spy_core::{
    json_value_to_string, AttributeConfig, AttributeSet, UiNode, ESSENTIAL_ATTRIBUTES,
};

pub fn format_tree(node: &UiNode, attributes: &AttributeConfig) -> String {
    let mut lines = Vec::new();
    let label = node_label(node, attributes);
    lines.push(label);
    let last_index = node.children.len().saturating_sub(1);
    for (idx, child) in node.children.iter().enumerate() {
        write_child(
            child,
            "".to_string(),
            idx == last_index,
            attributes,
            &mut lines,
        );
    }
    lines.join("\n")
}

fn write_child(
    node: &UiNode,
    prefix: String,
    is_last: bool,
    attributes: &AttributeConfig,
    lines: &mut Vec<String>,
) {
    let connector = if is_last { "└── " } else { "├── " };
    lines.push(format!(
        "{}{}{}",
        prefix,
        connector,
        node_label(node, attributes)
    ));
    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
    let last_index = node.children.len().saturating_sub(1);
    for (idx, child) in node.children.iter().enumerate() {
        write_child(
            child,
            child_prefix.clone(),
            idx == last_index,
            attributes,
            lines,
        );
    }
}

fn node_label(node: &UiNode, config: &AttributeConfig) -> String {
    let mut label = if node.name.is_empty() {
        "<unnamed>".to_string()
    } else {
        node.name.clone()
    };

    if let Some(role) = &node.role {
        if !role.is_empty() {
            label.push_str(&format!(" [{}]", role));
        }
    }

    let attrs = selected_attributes(node, config);
    if !attrs.is_empty() {
        let rendered = attrs
            .into_iter()
            .map(|(key, value)| format!("{}={}", key, json_value_to_string(value)))
            .collect::<Vec<_>>()
            .join(", ");
        label.push_str(&format!(" {{{}}}", rendered));
    }

    label
}

fn selected_attributes<'a>(
    node: &'a UiNode,
    config: &'a AttributeConfig,
) -> Vec<(&'a String, &'a serde_json::Value)> {
    match config.set {
        AttributeSet::Full => node.attributes.iter().collect(),
        AttributeSet::None | AttributeSet::Essential => {
            let mut order = Vec::new();
            let mut seen = HashSet::new();
            let iter: Box<dyn Iterator<Item = &str>> = match config.set {
                AttributeSet::None => Box::new(config.additional.iter().map(|s| s.as_str())),
                AttributeSet::Essential => Box::new(
                    ESSENTIAL_ATTRIBUTES
                        .iter()
                        .copied()
                        .chain(config.additional.iter().map(|s| s.as_str())),
                ),
                AttributeSet::Full => unreachable!(),
            };

            for key in iter {
                if seen.insert(key.to_ascii_lowercase()) {
                    order.push(key.to_string());
                }
            }

            let mut results = Vec::new();
            for target in order {
                if let Some((key, value)) = node
                    .attributes
                    .iter()
                    .find(|(attr, _)| attr.eq_ignore_ascii_case(&target))
                {
                    results.push((key, value));
                }
            }
            results
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_node() -> UiNode {
        UiNode {
            name: "Root".into(),
            role: Some("desktop".into()),
            attributes: Default::default(),
            children: vec![UiNode {
                name: "Child".into(),
                role: Some("window".into()),
                attributes: [
                    ("AutomationId".into(), json!("CalcWindow")),
                    ("processId".into(), json!(1234)),
                ]
                .into_iter()
                .collect(),
                children: vec![],
            }],
        }
    }

    #[test]
    fn formats_ascii_tree_with_essential_attributes() {
        let tree = sample_node();
        let config = AttributeConfig::new(AttributeSet::Essential, Vec::new());
        let formatted = format_tree(&tree, &config);
        assert!(formatted.contains("Root [desktop]"));
        assert!(formatted.contains("└── Child [window]"));
        assert!(formatted.contains("AutomationId=CalcWindow"));
        assert!(!formatted.contains("processId"));
    }

    #[test]
    fn formats_ascii_tree_with_full_attributes() {
        let tree = sample_node();
        let config = AttributeConfig::new(AttributeSet::Full, Vec::new());
        let formatted = format_tree(&tree, &config);
        assert!(formatted.contains("Child [window] {AutomationId=CalcWindow, processId=1234}"));
    }

    #[test]
    fn appends_additional_attributes() {
        let tree = sample_node();
        let config = AttributeConfig::new(AttributeSet::Essential, vec!["processId".into()]);
        let formatted = format_tree(&tree, &config);
        assert!(formatted.contains("processId=1234"));
    }

    #[test]
    fn supports_none_attribute_set_with_overrides() {
        let tree = sample_node();
        let config = AttributeConfig::new(AttributeSet::None, vec!["AutomationId".into()]);
        let formatted = format_tree(&tree, &config);
        assert!(formatted.contains("AutomationId=CalcWindow"));
        assert!(!formatted.contains("processId"));
    }
}
