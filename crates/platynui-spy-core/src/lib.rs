pub mod attributes;
pub mod backend;
pub mod config;
pub mod filter;
pub mod model;
pub mod xpath;

pub use attributes::{AttributeConfig, AttributeSet, ESSENTIAL_ATTRIBUTES};
pub use backend::{load_tree, BackendError};
pub use config::{AppConfig, BackendKind};
#[cfg(target_os = "windows")]
pub use config::{Win32Config, Win32Root};
pub use filter::FilterConfig;
pub use model::{json_value_to_string, UiNode};
pub use xpath::{XPath, XPathParseError};

pub fn capture_tree(config: &AppConfig) -> Result<Option<UiNode>, BackendError> {
    let mut tree = load_tree(config)?;

    if let Some(xpath) = &config.xpath {
        let matches = xpath.select(&tree);
        if matches.is_empty() {
            return Ok(None);
        }

        tree = if matches.len() == 1 {
            matches.into_iter().next().expect("single match")
        } else {
            UiNode {
                name: "XPathMatches".to_string(),
                role: None,
                attributes: Default::default(),
                children: matches,
            }
        };
    }

    Ok(filter::filter_tree(&tree, &config.filter))
}
