# Rust Spy Tool Implementation Plan

## 1. Background and Existing Capabilities
- The repository already ships a graphical spy utility built with Avalonia that is launched via the `PlatynUI.Spy` desktop application entry point.【F:README.md†L55-L81】【F:src/PlatynUI.Spy/Program.cs†L1-L20】
- The GUI tool focuses on interactively inspecting running applications; a command-line complement is missing for scripted pipelines or headless environments.【F:README.md†L55-L81】

## 2. Objectives for the Command-Line Spy Tool
- Provide a Rust-based binary (`spy-cli`) that can be executed in CI or local shells without a window system.
- Accept serialized UI tree dumps (JSON) via stdin or a file path and emit a filtered tree to stdout for downstream tooling.
- Offer flexible filtering primitives so the tool can be chained with Robot Framework suites or debugging scripts.

## 3. Data Model and Parsing Strategy
- Parse UI hierarchies into a `UiNode` structure containing `id`, `role`, optional `name`, optional `properties`, and recursive `children` nodes.
- Support both single-root (`{ ... }`) and forest (`[{ ... }]`) JSON payloads to accommodate different exporters.
- Preserve property order deterministically using `BTreeMap` to yield stable textual output for tests and diff tooling.

## 4. Filtering Capabilities
- Role filtering (`--role <value>`) with optional case-insensitive comparison for interoperability across platforms.
- Name matching via regular expressions with optional case folding (`--name-pattern`, `--ignore-name-case`).
- Arbitrary property matchers supplied as `--property key=value` pairs that ensure exact matches against node metadata.
- Depth limiting (`--max-depth`) to truncate noisy hierarchies while still showing ancestor context.
- Filtering retains ancestors of matching nodes to preserve navigational context even when the ancestor itself fails the predicate.

## 5. Output Modes
- Text mode (default) renders an ASCII tree (`├──`, `└──`) with optional property sections when `--include-properties` is passed.
- JSON mode (`--format json`) prints the filtered tree back as formatted JSON for machine consumers or regression snapshots.

## 6. Error Handling
- Invalid JSON payloads or regex patterns bubble up as actionable error messages while still exiting with a non-zero status code.
- Improper `--property` arguments (missing `=`) are rejected by clap before the tool attempts to parse the payload.

## 7. Testing Strategy
- **Unit tests** exercise the filtering engine: role filtering, regex name matching, property matching, and depth truncation.
- **Integration tests** (via `assert_cmd`) run the compiled binary against a fixture UI tree to verify end-to-end behaviour for text output, JSON mode, and input validation failure paths.
- The CI workflow runs `cargo fmt`, `cargo clippy`, and `cargo test` to keep style and lints enforced alongside functional coverage.

## 8. Delivery Steps
1. Scaffold the Rust crate and define the data model and filter engine inside `src/lib.rs`.
2. Implement the CLI front end with clap argument parsing in `src/main.rs`.
3. Add fixtures and integration tests under `tests/` to validate CLI behaviour.
4. Document the plan (this file) and reference it from contributor documentation if necessary.
5. Add a dedicated GitHub Actions workflow to build, lint, and test the crate on every push and pull request.
