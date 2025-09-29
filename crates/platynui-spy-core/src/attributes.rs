#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AttributeSet {
    None,
    #[default]
    Essential,
    Full,
}

#[derive(Debug, Clone, Default)]
pub struct AttributeConfig {
    pub set: AttributeSet,
    pub additional: Vec<String>,
}

impl AttributeConfig {
    pub fn new(set: AttributeSet, additional: Vec<String>) -> Self {
        let mut normalized = Vec::new();
        for attr in additional {
            let trimmed = attr.trim();
            if trimmed.is_empty() {
                continue;
            }
            if normalized
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(trimmed))
            {
                continue;
            }
            normalized.push(trimmed.to_string());
        }

        Self {
            set,
            additional: normalized,
        }
    }
}

pub const ESSENTIAL_ATTRIBUTES: &[&str] = &[
    "AutomationId",
    "Name",
    "ControlType",
    "ClassName",
    "FrameworkId",
    "BoundingRectangle",
    "IsEnabled",
    "IsOffscreen",
];
