use std::fs;
use std::path::PathBuf;

use thiserror::Error;

#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "windows")]
use win32::load_win32_tree;

#[cfg(target_os = "windows")]
use uiautomation::Error as Win32AutomationError;

use crate::args::{AppConfig, BackendKind};
use crate::model::UiNode;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("missing input for file backend")]
    MissingInput,

    #[error("failed to read UI tree from {path:?}")]
    ReadFailure {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse UI tree from {path:?}")]
    ParseFailure {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[cfg(target_os = "windows")]
    #[error("failed to capture Windows UI Automation tree: {source}")]
    WindowsAutomation {
        #[from]
        source: Win32AutomationError,
    },

    #[cfg(target_os = "windows")]
    #[error("no Windows UI element matched the provided selectors: {selectors}")]
    WindowsTargetNotFound { selectors: String },
}

pub fn load_tree(config: &AppConfig) -> Result<UiNode, BackendError> {
    match config.backend {
        BackendKind::File => {
            let path = config.input.clone().ok_or(BackendError::MissingInput)?;
            read_file_tree(path)
        }
        #[cfg(target_os = "windows")]
        BackendKind::Win32 => load_win32_tree(config),
    }
}

fn read_file_tree(path: PathBuf) -> Result<UiNode, BackendError> {
    let content = fs::read_to_string(&path).map_err(|source| BackendError::ReadFailure {
        path: path.clone(),
        source,
    })?;

    serde_json::from_str(&content).map_err(|source| BackendError::ParseFailure { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[test]
    fn reads_tree_from_json_file() {
        let mut file = NamedTempFile::new().expect("temp file");
        let data = json!({
            "name": "root",
            "children": [
                {"name": "child"}
            ]
        });
        serde_json::to_writer_pretty(file.as_file_mut(), &data).expect("write");

        let tree = read_file_tree(file.path().to_path_buf()).expect("tree");
        assert_eq!(tree.name, "root");
        assert_eq!(tree.children.len(), 1);
    }
}
