use std::fmt;

use thiserror::Error;

use crate::model::{json_value_to_string, UiNode};

#[derive(Debug, Error)]
pub enum XPathParseError {
    #[error("XPath must start with '/' and contain at least one segment")]
    MissingRoot,
    #[error("unclosed predicate in segment '{0}'")]
    UnclosedPredicate(String),
    #[error("invalid predicate '{0}', expected @attribute='value'")]
    InvalidPredicate(String),
    #[error("invalid quoted value in predicate '{0}'")]
    InvalidQuotedValue(String),
}

#[derive(Debug, Clone)]
pub struct XPath {
    segments: Vec<PathSegment>,
}

impl XPath {
    pub fn parse(raw: &str) -> Result<Self, XPathParseError> {
        if !raw.starts_with('/') {
            return Err(XPathParseError::MissingRoot);
        }

        let mut segments = Vec::new();
        for part in raw.split('/').skip(1) {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            segments.push(PathSegment::parse(trimmed)?);
        }

        if segments.is_empty() {
            return Err(XPathParseError::MissingRoot);
        }

        Ok(Self { segments })
    }

    pub fn select(&self, root: &UiNode) -> Vec<UiNode> {
        let mut current: Vec<&UiNode> = vec![root];
        for (index, segment) in self.segments.iter().enumerate() {
            let mut next = Vec::new();
            for node in &current {
                if index == 0 && segment.matches(node) {
                    next.push(*node);
                }
                for child in &node.children {
                    if segment.matches(child) {
                        next.push(child);
                    }
                }
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }

        current.into_iter().map(|node| node.clone()).collect()
    }
}

#[derive(Debug, Clone)]
struct PathSegment {
    name: Option<String>,
    predicates: Vec<Predicate>,
}

impl PathSegment {
    fn parse(raw: &str) -> Result<Self, XPathParseError> {
        let mut name = String::new();
        let mut predicates = Vec::new();
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '[' => {
                    let mut predicate = String::new();
                    let mut depth = 1usize;
                    while let Some(next) = chars.next() {
                        match next {
                            '[' => {
                                depth += 1;
                                predicate.push(next);
                            }
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                predicate.push(next);
                            }
                            _ => predicate.push(next),
                        }
                    }

                    if depth != 0 {
                        return Err(XPathParseError::UnclosedPredicate(raw.to_string()));
                    }

                    predicates.push(Predicate::parse(predicate.trim())?);
                }
                _ => name.push(ch),
            }
        }

        let name = name.trim();
        let name = if name.is_empty() || name == "*" {
            None
        } else {
            Some(name.to_string())
        };

        Ok(Self { name, predicates })
    }

    fn matches(&self, node: &UiNode) -> bool {
        if let Some(name) = &self.name {
            if !node.name.eq_ignore_ascii_case(name) {
                return false;
            }
        }

        for predicate in &self.predicates {
            if !predicate.matches(node) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
enum Predicate {
    NameEquals(String),
    RoleEquals(String),
    AttributeEquals(String, String),
}

impl Predicate {
    fn parse(raw: &str) -> Result<Self, XPathParseError> {
        let trimmed = raw.trim();
        if !trimmed.starts_with('@') {
            return Err(XPathParseError::InvalidPredicate(raw.to_string()));
        }
        let body = &trimmed[1..];
        let (lhs, rhs) = body
            .split_once('=')
            .ok_or_else(|| XPathParseError::InvalidPredicate(raw.to_string()))?;
        let lhs = lhs.trim();
        let value = parse_quoted(rhs.trim())
            .ok_or_else(|| XPathParseError::InvalidQuotedValue(raw.to_string()))?;
        match lhs.to_ascii_lowercase().as_str() {
            "name" => Ok(Predicate::NameEquals(value)),
            "role" => Ok(Predicate::RoleEquals(value)),
            attr => Ok(Predicate::AttributeEquals(attr.to_string(), value)),
        }
    }

    fn matches(&self, node: &UiNode) -> bool {
        match self {
            Predicate::NameEquals(expected) => node.name.eq_ignore_ascii_case(expected),
            Predicate::RoleEquals(expected) => node
                .role
                .as_ref()
                .map(|role| role.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
            Predicate::AttributeEquals(key, expected) => node
                .attributes
                .iter()
                .find(|(attr, _)| attr.eq_ignore_ascii_case(key))
                .map(|(_, value)| json_value_to_string(value))
                .map(|actual| actual.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
        }
    }
}

fn parse_quoted(raw: &str) -> Option<String> {
    if !raw.starts_with('\'') || !raw.ends_with('\'') || raw.len() < 2 {
        return None;
    }
    let inner = &raw[1..raw.len() - 1];
    Some(inner.to_string())
}

impl fmt::Display for XPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments = self
            .segments
            .iter()
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>()
            .join("/");
        write!(f, "/{segments}")
    }
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}")?;
        } else {
            write!(f, "*")?;
        }
        for predicate in &self.predicates {
            write!(f, "[{}]", predicate)?;
        }
        Ok(())
    }
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::NameEquals(value) => write!(f, "@name='{value}'"),
            Predicate::RoleEquals(value) => write!(f, "@role='{value}'"),
            Predicate::AttributeEquals(key, value) => write!(f, "@{key}='{value}'"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_tree() -> UiNode {
        UiNode {
            name: "Desktop".into(),
            role: Some("desktop".into()),
            attributes: Default::default(),
            children: vec![UiNode {
                name: "Calculator".into(),
                role: Some("window".into()),
                attributes: [("AutomationId".into(), json!("Calc"))]
                    .into_iter()
                    .collect(),
                children: vec![UiNode {
                    name: "Display".into(),
                    role: Some("text".into()),
                    attributes: Default::default(),
                    children: vec![],
                }],
            }],
        }
    }

    #[test]
    fn parses_simple_xpath() {
        let path = XPath::parse("/Desktop/Calculator").expect("parse");
        let matches = path.select(&sample_tree());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Calculator");
    }

    #[test]
    fn matches_by_attribute_predicate() {
        let path = XPath::parse("/Desktop/*[@AutomationId='Calc']").expect("parse");
        let matches = path.select(&sample_tree());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Calculator");
    }

    #[test]
    fn rejects_missing_root() {
        let err = XPath::parse("Calculator").expect_err("missing root");
        assert!(matches!(err, XPathParseError::MissingRoot));
    }
}
