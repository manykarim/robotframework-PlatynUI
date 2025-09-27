pub mod args;
pub mod backend;
pub mod filter;
pub mod model;
pub mod output;

use anyhow::Context;
use clap::Parser;

use args::{Cli, OutputFormat};
use backend::BackendError;
use filter::filter_tree;

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run_with_args(cli)
}

fn run_with_args(cli: Cli) -> anyhow::Result<()> {
    let config = cli.build_config().map_err(anyhow::Error::from)?;
    let tree = backend::load_tree(&config).map_err(|err| match err {
        BackendError::MissingInput => anyhow::Error::new(err),
        BackendError::ReadFailure { path, source } => {
            anyhow::Error::new(source).context(format!("failed to read input {:?}", path))
        }
        BackendError::ParseFailure { path, source } => {
            anyhow::Error::new(source).context(format!("failed to parse UI tree from {:?}", path))
        }
    })?;

    if let Some(filtered) = filter_tree(&tree, &config.filter) {
        match config.format {
            OutputFormat::Tree => {
                let rendered = output::format_tree(&filtered, config.show_attributes);
                println!("{rendered}");
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&filtered)
                    .context("failed to serialise filtered tree as JSON")?;
                println!("{json}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn run_with_sample_tree_outputs_text() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let sample = format!("{manifest_dir}/tests/data/sample_tree.json");
        let cli = Cli::parse_from(["platynui-spy", "--input", &sample, "--format", "tree"]);

        run_with_args(cli).expect("run");
    }

    #[test]
    fn run_with_json_output_is_valid_json() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let sample = format!("{manifest_dir}/tests/data/sample_tree.json");
        let cli = Cli::parse_from([
            "platynui-spy",
            "--input",
            &sample,
            "--format",
            "json",
            "--filter-role",
            "window",
        ]);

        run_with_args(cli).expect("run");
    }
}
