use crate::model::{json_value_to_string, UiNode};

#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    pub max_depth: Option<usize>,
    pub include_ancestors: bool,
    pub name_substring: Option<String>,
    pub role: Option<String>,
    pub attr_equals: Vec<(String, String)>,
}

impl FilterConfig {
    pub fn new(
        max_depth: Option<usize>,
        include_ancestors: bool,
        name_substring: Option<String>,
        role: Option<String>,
        attr_equals: Vec<(String, String)>,
    ) -> Self {
        let attr_equals = attr_equals
            .into_iter()
            .map(|(k, v)| (k, v.to_lowercase()))
            .collect();

        Self {
            max_depth,
            include_ancestors,
            name_substring,
            role,
            attr_equals,
        }
    }

    fn matches(&self, node: &UiNode) -> bool {
        if let Some(name) = &self.name_substring {
            if !node.name.to_lowercase().contains(name) {
                return false;
            }
        }

        if let Some(role) = &self.role {
            match &node.role {
                Some(node_role) if node_role.to_lowercase() == *role => {}
                Some(node_role) if node_role.eq_ignore_ascii_case(role) => {}
                _ => return false,
            }
        }

        for (key, value) in &self.attr_equals {
            let Some(actual) = node
                .attribute_value(key)
                .map(json_value_to_string)
                .map(|s| s.to_lowercase())
            else {
                return false;
            };

            if actual != *value {
                return false;
            }
        }

        true
    }
}

enum FilterOutcome {
    Include(UiNode),
    Promote(Vec<UiNode>),
    Exclude,
}

pub fn filter_tree(node: &UiNode, config: &FilterConfig) -> Option<UiNode> {
    match filter_internal(node, config, 0) {
        FilterOutcome::Include(node) => Some(node),
        FilterOutcome::Promote(children) => {
            if children.is_empty() {
                None
            } else {
                let mut clone = node.clone();
                clone.children = children;
                Some(clone)
            }
        }
        FilterOutcome::Exclude => None,
    }
}

fn filter_internal(node: &UiNode, config: &FilterConfig, depth: usize) -> FilterOutcome {
    if let Some(max_depth) = config.max_depth {
        if depth > max_depth {
            return FilterOutcome::Exclude;
        }
    }

    let mut filtered_children = Vec::new();
    for child in &node.children {
        match filter_internal(child, config, depth + 1) {
            FilterOutcome::Include(child_node) => filtered_children.push(child_node),
            FilterOutcome::Promote(mut promoted) => filtered_children.append(&mut promoted),
            FilterOutcome::Exclude => {}
        }
    }

    let matches_self = config.matches(node);

    if matches_self {
        let mut clone = node.clone();
        clone.children = filtered_children;
        FilterOutcome::Include(clone)
    } else if !filtered_children.is_empty() {
        if config.include_ancestors {
            let mut clone = node.clone();
            clone.children = filtered_children;
            FilterOutcome::Include(clone)
        } else {
            FilterOutcome::Promote(filtered_children)
        }
    } else {
        FilterOutcome::Exclude
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(name: &str, role: &str) -> UiNode {
        UiNode {
            name: name.to_string(),
            role: Some(role.to_string()),
            attributes: Default::default(),
            children: vec![],
        }
    }

    #[test]
    fn filters_on_name_and_role() {
        let mut root = node("root", "desktop");
        let mut child = node("Calculator", "window");
        child.children.push(node("Display", "text"));
        root.children.push(child.clone());
        root.children.push(node("Settings", "window"));

        let config = FilterConfig::new(
            Some(3),
            true,
            Some("calc".into()),
            Some("window".into()),
            vec![],
        );
        let filtered = filter_tree(&root, &config).expect("tree");
        assert_eq!(filtered.children.len(), 1);
        assert_eq!(filtered.children[0].name, "Calculator");
    }

    #[test]
    fn promotes_children_when_not_keeping_ancestors() {
        let mut root = node("root", "desktop");
        let mut group = node("Group", "group");
        let mut button = node("Submit", "button");
        button.attributes.insert("Name".into(), json!("Submit"));
        group.children.push(button.clone());
        root.children.push(group);

        let config = FilterConfig::new(None, false, None, Some("button".into()), vec![]);
        let filtered = filter_tree(&root, &config).expect("tree");
        assert_eq!(filtered.children.len(), 1);
        assert_eq!(filtered.children[0].name, "Submit");
    }

    #[test]
    fn attribute_filter_matches_case_insensitively() {
        let mut root = node("root", "desktop");
        let mut child = node("Calculator", "window");
        child
            .attributes
            .insert("AutomationId".into(), json!("CalcWindow"));
        root.children.push(child.clone());

        let config = FilterConfig::new(
            None,
            true,
            None,
            None,
            vec![("AutomationId".into(), "calcwindow".into())],
        );
        let filtered = filter_tree(&root, &config).expect("tree");
        assert_eq!(filtered.children.len(), 1);
    }
}
