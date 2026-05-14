# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- Build: `cargo build` (release: `cargo build --release`)
- Run dashboard: `cargo run -- run` (headless: `cargo run -- run --headless`)
- Lint / format: `cargo clippy --all-targets`, `cargo fmt`
- Tests: `cargo test`; single test: `cargo test <name>` (e.g. `cargo test --lib state::tests::round_trip`)
- Install locally: `cargo install --path .`

## Architecture

Single Rust binary (`wotwot`) with three roles dispatched by `clap` in `src/main.rs`:

1. **`run`** — boots a `tokio` runtime that does three things concurrently:
   - Binds a Unix domain socket (path from `WOTWOT_SOCK` / `--sock`, default under `$XDG_RUNTIME_DIR/wotwot/`, mode `0600`) and serves an `axum` router over it via a hand-rolled `serve_uds` loop in `main.rs` (hyper-util `auto::Builder`, no TCP).
   - Spawns periodic **collectors** (`src/collectors.rs`) that shell out to `npx -y ccusage` and `docker stats --no-stream`, parse their output, and write into shared state. Missing CLIs degrade gracefully — the pane shows "unavailable".
   - Renders the **TUI** (`src/tui.rs`, `ratatui` + `crossterm`) unless `--headless`. Four stacked panes (claude / todos / docker / info) in a fixed 55-col layout.
2. **`todo …` / `info …`** — CLI subcommands in `src/cli.rs` that connect to the same Unix socket as an HTTP client and call the REST endpoints. They do **not** touch state files directly; the running `run` instance is the single writer.
3. **`agents`** — prints a static markdown guide for AI agents.

### State

`src/state.rs` defines `AppState` (todos, info stack, cached collector output) wrapped as `SharedState = Arc<RwLock<AppState>>`. Persisted as JSON via `dirs::data_dir()` (macOS: `~/Library/Application Support/wotwot/state.json`). Only the `todo`/`info` mutations persist; collector output is in-memory.

### HTTP API (over UDS)

Routes live in `src/server.rs`: `GET /state`, `POST/GET /todo`, `DELETE /todo/:id`, `POST /todo/reorder`, `POST/GET /info`, `POST /info/pop`, `DELETE /info/:id`. The CLI in `cli.rs` is the canonical client — match its request shape when adding endpoints.

### Info stack semantics

LIFO: `push` prepends, `pop` removes the top, and the TUI pane loops through entries. Falls back to a built-in quote list when empty.

## Conventions

- The TUI fills the available terminal width; pane heights are constrained in `tui.rs`, widths flow from `f.area()`.
- New collectors should follow the pattern in `collectors.rs`: spawn the external command, parse defensively, write into `SharedState`, and never panic on missing binaries.
- New CLI subcommands belong in `cli.rs` and should round-trip through the HTTP API rather than reading/writing state directly.
