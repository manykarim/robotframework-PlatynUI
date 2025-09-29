use clap::ValueEnum;

/// Preset attribute sets that can be selected from the command line.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum AttributeSet {
    /// Emit no attributes at all.
    None,
    /// Emit a curated list of attributes that are commonly useful for UI automation.
    Essential,
    /// Emit every attribute captured for the node.
    Full,
}

impl Default for AttributeSet {
    fn default() -> Self {
        AttributeSet::Essential
    }
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
