use crate::model::{json_value_to_string, UiNode};

pub fn format_tree(node: &UiNode, show_attributes: bool) -> String {
    let mut lines = Vec::new();
    let label = node_label(node, show_attributes);
    lines.push(label);
    let last_index = node.children.len().saturating_sub(1);
    for (idx, child) in node.children.iter().enumerate() {
        write_child(
            child,
            "".to_string(),
            idx == last_index,
            show_attributes,
            &mut lines,
        );
    }
    lines.join("\n")
}

fn write_child(
    node: &UiNode,
    prefix: String,
    is_last: bool,
    show_attributes: bool,
    lines: &mut Vec<String>,
) {
    let connector = if is_last { "└── " } else { "├── " };
    lines.push(format!(
        "{}{}{}",
        prefix,
        connector,
        node_label(node, show_attributes)
    ));
    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
    let last_index = node.children.len().saturating_sub(1);
    for (idx, child) in node.children.iter().enumerate() {
        write_child(
            child,
            child_prefix.clone(),
            idx == last_index,
            show_attributes,
            lines,
        );
    }
}

fn node_label(node: &UiNode, show_attributes: bool) -> String {
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

    if show_attributes && !node.attributes.is_empty() {
        let attrs = node
            .attributes
            .iter()
            .map(|(k, v)| format!("{}={}", k, json_value_to_string(v)))
            .collect::<Vec<_>>()
            .join(", ");
        label.push_str(&format!(" {{{}}}", attrs));
    }

    label
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
    fn formats_ascii_tree_without_attributes() {
        let tree = sample_node();
        let formatted = format_tree(&tree, false);
        assert!(formatted.contains("Root [desktop]"));
        assert!(formatted.contains("└── Child [window]"));
        assert!(!formatted.contains("AutomationId"));
    }

    #[test]
    fn formats_ascii_tree_with_attributes() {
        let tree = sample_node();
        let formatted = format_tree(&tree, true);
        assert!(formatted.contains("Child [window] {AutomationId=CalcWindow, processId=1234}"));
    }
}
