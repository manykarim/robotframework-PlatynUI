use std::path::PathBuf;

use crate::attributes::AttributeConfig;
use crate::filter::FilterConfig;
use crate::xpath::XPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    File,
    #[cfg(target_os = "windows")]
    Win32,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub backend: BackendKind,
    pub input: Option<PathBuf>,
    pub filter: FilterConfig,
    pub attributes: AttributeConfig,
    pub xpath: Option<XPath>,
    #[cfg(target_os = "windows")]
    pub win32: Win32Config,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct Win32Config {
    pub root: Win32Root,
    pub process_id: Option<u32>,
    pub window_title: Option<String>,
    pub top_level_only: bool,
}

#[cfg(target_os = "windows")]
impl Default for Win32Config {
    fn default() -> Self {
        Self {
            root: Win32Root::Desktop,
            process_id: None,
            window_title: None,
            top_level_only: false,
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32Root {
    Desktop,
    Focused,
}

#[cfg(target_os = "windows")]
impl Default for Win32Root {
    fn default() -> Self {
        Self::Desktop
    }
}
