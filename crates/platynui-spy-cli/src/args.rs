use std::path::PathBuf;

#[cfg(target_os = "windows")]
use clap::Args;
use clap::{ArgAction, Parser, ValueEnum};
use platynui_spy_core::{
    AppConfig, AttributeConfig, AttributeSet, BackendKind, FilterConfig, XPath, XPathParseError,
};
#[cfg(target_os = "windows")]
use platynui_spy_core::{Win32Config, Win32Root};
use thiserror::Error;

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum BackendArg {
    File,
    #[cfg(target_os = "windows")]
    Win32,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Tree,
    Json,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum AttributeSetArg {
    None,
    Essential,
    Full,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, Default)]
pub enum Win32RootArg {
    #[default]
    Desktop,
    Focused,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Args, Default)]
#[command(next_help_heading = "Windows UI Automation")]
pub struct Win32CliOptions {
    #[arg(long, value_enum, default_value_t = Win32RootArg::Desktop)]
    pub root: Win32RootArg,

    #[arg(long)]
    pub process_id: Option<u32>,

    #[arg(long)]
    pub window_title: Option<String>,

    #[arg(long, action = ArgAction::SetTrue)]
    pub top_level_only: bool,
}

#[cfg(target_os = "windows")]
impl Win32CliOptions {
    fn normalized_window_title(&self) -> Option<String> {
        self.window_title
            .as_ref()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty())
    }

    fn has_win32_args(&self) -> bool {
        self.process_id.is_some()
            || self.normalized_window_title().is_some()
            || self.top_level_only
            || !matches!(self.root, Win32RootArg::Desktop)
    }

    fn to_config(&self) -> Win32Config {
        let root = match self.root {
            Win32RootArg::Desktop => Win32Root::Desktop,
            Win32RootArg::Focused => Win32Root::Focused,
        };

        Win32Config {
            root,
            process_id: self.process_id,
            window_title: self.normalized_window_title(),
            top_level_only: self.top_level_only,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "platynui-spy",
    about = "Inspect UI automation trees from the command line."
)]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = BackendArg::File)]
    pub backend: BackendArg,

    #[arg(long)]
    pub input: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Tree)]
    pub format: OutputFormat,

    #[arg(long)]
    pub max_depth: Option<usize>,

    #[arg(long)]
    pub filter_name: Option<String>,

    #[arg(long)]
    pub filter_role: Option<String>,

    #[arg(long = "filter-attr")]
    pub filter_attrs: Vec<String>,

    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_include_ancestors")]
    pub include_ancestors: bool,

    #[arg(
        long = "no-include-ancestors",
        action = ArgAction::SetTrue,
        conflicts_with = "include_ancestors"
    )]
    pub no_include_ancestors: bool,

    #[arg(long, value_enum, default_value_t = AttributeSetArg::Essential)]
    pub attribute_set: AttributeSetArg,

    #[arg(long = "attribute")]
    pub attribute_keys: Vec<String>,

    #[arg(long)]
    pub xpath: Option<String>,

    #[cfg(target_os = "windows")]
    #[command(flatten)]
    pub win32: Win32CliOptions,
}

#[derive(Debug, Error)]
pub enum ArgsError {
    #[error("missing --input for file backend")]
    MissingInput,

    #[error("invalid attribute filter '{0}', expected key=value")]
    InvalidAttributeFilter(String),

    #[error("--include-ancestors and --no-include-ancestors cannot be used together")]
    ConflictingAncestorFlags,

    #[error("invalid XPath expression: {0}")]
    InvalidXPath(#[from] XPathParseError),

    #[cfg(target_os = "windows")]
    #[error("win32-specific options require --backend win32")]
    Win32OptionsWithoutBackend,
}

#[derive(Debug, Clone)]
pub struct CliConfig {
    pub capture: AppConfig,
    pub format: OutputFormat,
}

impl Cli {
    pub fn build_config(&self) -> Result<CliConfig, ArgsError> {
        let attr_pairs = self
            .filter_attrs
            .iter()
            .map(|raw| parse_attr(raw))
            .collect::<Result<Vec<_>, _>>()?;

        let name_filter = self
            .filter_name
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let role_filter = self
            .filter_role
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let include_ancestors = match (self.include_ancestors, self.no_include_ancestors) {
            (true, true) => return Err(ArgsError::ConflictingAncestorFlags),
            (true, false) => true,
            (false, true) => false,
            (false, false) => true,
        };

        let filter = FilterConfig::new(
            self.max_depth,
            include_ancestors,
            name_filter,
            role_filter,
            attr_pairs,
        );

        let attribute_set = match self.attribute_set {
            AttributeSetArg::None => AttributeSet::None,
            AttributeSetArg::Essential => AttributeSet::Essential,
            AttributeSetArg::Full => AttributeSet::Full,
        };
        let attributes = AttributeConfig::new(attribute_set, self.attribute_keys.clone());

        let xpath = match &self.xpath {
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(XPath::parse(trimmed).map_err(ArgsError::from)?)
                }
            }
            None => None,
        };

        let backend = match self.backend {
            BackendArg::File => BackendKind::File,
            #[cfg(target_os = "windows")]
            BackendArg::Win32 => BackendKind::Win32,
        };

        if matches!(backend, BackendKind::File) && self.input.is_none() {
            return Err(ArgsError::MissingInput);
        }

        #[cfg(target_os = "windows")]
        if !matches!(backend, BackendKind::Win32) && self.win32.has_win32_args() {
            return Err(ArgsError::Win32OptionsWithoutBackend);
        }

        #[cfg(target_os = "windows")]
        let win32 = if matches!(backend, BackendKind::Win32) {
            self.win32.to_config()
        } else {
            Win32Config::default()
        };

        let capture = AppConfig {
            backend,
            input: self.input.clone(),
            filter,
            attributes,
            xpath,
            #[cfg(target_os = "windows")]
            win32,
        };

        Ok(CliConfig {
            capture,
            format: self.format.clone(),
        })
    }
}

fn parse_attr(raw: &str) -> Result<(String, String), ArgsError> {
    let (key, value) = raw
        .split_once('=')
        .ok_or_else(|| ArgsError::InvalidAttributeFilter(raw.to_string()))?;
    let key = key.trim();
    let value = value.trim();

    if key.is_empty() || value.is_empty() {
        return Err(ArgsError::InvalidAttributeFilter(raw.to_string()));
    }

    Ok((key.to_string(), value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn base_cli() -> Cli {
        Cli {
            backend: BackendArg::File,
            input: Some(PathBuf::from("tree.json")),
            format: OutputFormat::Tree,
            max_depth: None,
            filter_name: None,
            filter_role: None,
            filter_attrs: Vec::new(),
            include_ancestors: false,
            no_include_ancestors: false,
            attribute_set: AttributeSetArg::Essential,
            attribute_keys: Vec::new(),
            xpath: None,
            #[cfg(target_os = "windows")]
            win32: Win32CliOptions::default(),
        }
    }

    #[test]
    fn conflicting_ancestor_flags_are_rejected() {
        let mut cli = base_cli();
        cli.include_ancestors = true;
        cli.no_include_ancestors = true;

        let err = cli.build_config().expect_err("conflict");
        assert!(matches!(err, ArgsError::ConflictingAncestorFlags));
    }

    #[test]
    fn parse_attr_accepts_valid_pairs() {
        let pair = parse_attr("AutomationId=MainWindow").expect("pair");
        assert_eq!(pair, ("AutomationId".to_string(), "MainWindow".to_string()));
    }

    #[test]
    fn parse_attr_rejects_invalid_pairs() {
        let err = parse_attr("no-equals").expect_err("error");
        assert!(matches!(err, ArgsError::InvalidAttributeFilter(_)));
    }

    #[test]
    fn whitespace_xpath_is_ignored() {
        let mut cli = base_cli();
        cli.xpath = Some("   ".into());

        let config = cli.build_config().expect("config");
        assert!(config.capture.xpath.is_none());
    }

    #[test]
    fn invalid_xpath_reports_error() {
        let mut cli = base_cli();
        cli.xpath = Some("Calculator".into());

        let err = cli.build_config().expect_err("invalid xpath");
        assert!(matches!(err, ArgsError::InvalidXPath(_)));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn win32_specific_options_require_backend() {
        let mut cli = base_cli();
        cli.win32.process_id = Some(42);

        let err = cli
            .build_config()
            .expect_err("win32 options should require backend");
        assert!(matches!(err, ArgsError::Win32OptionsWithoutBackend));
    }
}
