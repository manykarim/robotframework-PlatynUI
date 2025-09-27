use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use spy_cli::{
    FilterCriteria, filter_tree, format_tree_json, format_tree_text, load_ui_tree_from_str,
};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "PlatynUI command line spy tool", long_about = None)]
struct Cli {
    /// Optional path to a JSON file containing a UI tree dump
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Restrict the output to nodes at or above this depth (root = 0)
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Only include nodes whose role matches one of the provided values
    #[arg(short, long = "role")]
    roles: Vec<String>,

    /// Only include nodes whose name matches the provided regular expression
    #[arg(short, long = "name-pattern")]
    name_pattern: Option<String>,

    /// Treat role filters as case-insensitive
    #[arg(long = "ignore-role-case")]
    ignore_role_case: bool,

    /// Treat name pattern filter as case-insensitive by wrapping it into (?i)
    #[arg(long = "ignore-name-case")]
    ignore_name_case: bool,

    /// Require nodes to contain these property key=value pairs
    #[arg(long = "property", value_parser = parse_property)]
    properties: Vec<(String, String)>,

    /// Include serialized properties in textual output
    #[arg(long = "include-properties")]
    include_properties: bool,

    /// Select the desired output format
    #[arg(long = "format", default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut buffer = String::new();
    if let Some(input_path) = cli.input {
        buffer = fs::read_to_string(input_path)?;
    } else {
        io::stdin().read_to_string(&mut buffer)?;
    }

    let mut criteria = FilterCriteria::new();
    criteria.max_depth = cli.max_depth;
    criteria.include_properties = cli.include_properties;
    criteria.ignore_role_case = cli.ignore_role_case;
    criteria.ignore_name_case = cli.ignore_name_case;
    for role in cli.roles {
        criteria.roles.push(role);
    }
    if let Some(pattern) = cli.name_pattern {
        criteria.name_pattern = Some(pattern);
    }
    for (key, value) in cli.properties {
        criteria.properties.insert(key, value);
    }

    let tree = load_ui_tree_from_str(&buffer)?;
    let filtered = filter_tree(&tree, &criteria)?;

    match cli.format {
        OutputFormat::Text => {
            let output = format_tree_text(&filtered, criteria.include_properties);
            println!("{}", output);
        }
        OutputFormat::Json => {
            let output = format_tree_json(&filtered)?;
            println!("{}", output);
        }
    }

    Ok(())
}

fn parse_property(raw: &str) -> Result<(String, String), String> {
    let (key, value) = raw
        .split_once('=')
        .ok_or_else(|| "properties must be expressed as key=value".to_string())?;
    if key.trim().is_empty() {
        return Err("property key must not be empty".into());
    }
    Ok((key.trim().to_string(), value.trim().to_string()))
}
