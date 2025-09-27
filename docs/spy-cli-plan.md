# PlatynUI Spy CLI Implementation Plan

## Background analysis of the existing spy tool

The existing PlatynUI spy is an Avalonia desktop application backed by the `.NET` runtime. The `MainWindowViewModel` keeps an observable `TreeNode` hierarchy rooted in the shared `Desktop` instance so that UI elements from the currently active providers can be inspected. It also coordinates highlighting, search and selection state for nodes in the UI tree.【F:src/PlatynUI.Spy/ViewModels/MainWindowViewModel.cs†L23-L220】

Each visual node is represented by `TreeNode`, which wraps an `INode` from the runtime layer. `TreeNode` exposes the `LocalName`, derived description strings, lazily materialised children/attribute collections, and refresh helpers so the desktop UI stays in sync with the actual provider state.【F:src/PlatynUI.Spy/ViewModels/TreeNode.cs†L13-L151】 This structure allows the Avalonia tree view to expand on demand and re-query provider data when needed.

## Goals for the Rust command line spy

1. Provide a portable CLI alternative to the graphical spy that emits the UI tree to `stdout`.
2. Allow callers to scope the tree via filtering and formatting options so that the command can be piped into other tooling.
3. Structure the Rust implementation so additional backends (Win32 UIA, AT-SPI2, Accessibility API) can be plugged in later without redesigning the CLI surface.
4. Ship automated tests that exercise filtering, formatting and error handling against fixture data to keep the CLI stable.

## High-level architecture

- `crates/platynui-spy-cli`: a new Rust binary crate that exposes the CLI.
  - `args` module: defines the command line surface using `clap` and normalises filters.
  - `backend` module: trait-based abstraction returning a `UiNode` tree. Initial implementation is `FileBackend` which reads JSON snapshots exported by other tools; future backends can wrap platform-specific providers.
  - `filter` module: applies attribute, name and role predicates as well as depth limits. Filtering keeps ancestors of matching nodes to preserve the tree context.
  - `output` module: renders the filtered tree either as an ASCII tree (default) or as pretty-printed JSON. Optional flags control attribute rendering.
  - `main.rs`: wires everything together, selecting the backend, applying filters and printing the result. Errors are reported with context using `anyhow` for good CLI diagnostics.

Shared data model:
- `UiNode`: serialisable structure mirroring the runtime `INode` contract (name, role, arbitrary attributes, children). Serde-driven deserialisation keeps the CLI decoupled from specific providers while still supporting nested trees.

## Command line surface

| Option | Description |
| --- | --- |
| `--input <PATH>` | JSON snapshot describing a UI tree. Required for the file backend. |
| `--backend <mock|file>` | Backend selection. Defaults to `file` for now, leaving room for OS-specific backends. |
| `--format <tree|json>` | Output format (ASCII tree or JSON). Default: `tree`. |
| `--max-depth <N>` | Limit traversal depth (root depth = 0). |
| `--filter-name <PATTERN>` | Case-insensitive substring filter on the node name. |
| `--filter-role <ROLE>` | Exact match on the `role` field. |
| `--filter-attr <KEY=VALUE>` | Match arbitrary attribute key/value pairs (repeatable). |
| `--include-ancestors/--no-include-ancestors` | Keep ancestors of matching nodes (default: true). |
| `--show-attributes` | Include attribute summary in tree output. |

## Testing strategy

1. **Unit tests** (in `src/filter.rs` and `src/output.rs`):
   - Filtering includes matching descendants and prunes branches when no predicates hit.
   - Depth limits exclude deeper nodes.
   - ASCII formatter creates deterministic branch glyphs and attribute annotations.
2. **Integration tests** (in `tests/cli.rs` using `assert_cmd`):
   - Running the CLI against `tests/data/sample_tree.json` without filters prints the full tree.
   - Applying filters (role/name/attribute) reduces the output as expected while keeping ancestors when requested.
   - JSON output remains valid and pretty-printed.
   - Missing input file or malformed attribute filter surfaces human-readable errors.
3. **CI**: GitHub Actions workflow executes `cargo fmt --check`, `cargo clippy` and `cargo test` to keep the Rust crate healthy on every push/PR.

## Follow-up work (beyond this change)

- Implement platform backends that call into the existing runtime components via FFI or IPC, replacing the JSON snapshot dependency.
- Streamline JSON snapshot generation directly from the existing Avalonia spy so the CLI can share capture logic.
- Add optional output (e.g. CSV, XPath extraction) tailored to Robot Framework workflows.
