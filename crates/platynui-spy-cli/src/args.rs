use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use thiserror::Error;

use crate::filter::FilterConfig;

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum BackendKind {
    File,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Tree,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "platynui-spy",
    about = "Inspect UI automation trees from the command line."
)]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = BackendKind::File)]
    pub backend: BackendKind,

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

    #[arg(long, default_value_t = true)]
    pub include_ancestors: bool,

    #[arg(long, default_value_t = false)]
    pub show_attributes: bool,
}

#[derive(Debug, Error)]
pub enum ArgsError {
    #[error("missing --input for file backend")]
    MissingInput,

    #[error("invalid attribute filter '{0}', expected key=value")]
    InvalidAttributeFilter(String),
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub backend: BackendKind,
    pub input: Option<PathBuf>,
    pub format: OutputFormat,
    pub filter: FilterConfig,
    pub show_attributes: bool,
}

impl Cli {
    pub fn build_config(&self) -> Result<AppConfig, ArgsError> {
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

        let filter = FilterConfig::new(
            self.max_depth,
            self.include_ancestors,
            name_filter,
            role_filter,
            attr_pairs,
        );

        if self.backend == BackendKind::File && self.input.is_none() {
            return Err(ArgsError::MissingInput);
        }

        Ok(AppConfig {
            backend: self.backend.clone(),
            input: self.input.clone(),
            format: self.format.clone(),
            filter,
            show_attributes: self.show_attributes,
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
