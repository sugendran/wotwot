# wotwot

Tiny terminal dashboard built with [ratatui](https://ratatui.rs). Resizes
to fit whatever space your terminal gives it.

One binary, three roles:

- `wotwot run` — launches the TUI **and** a local HTTP API served over a
  Unix domain socket (no ports, no network). Default path is
  `$XDG_RUNTIME_DIR/wotwot/wotwot.sock` (or a cache-dir / tmp fallback),
  created mode `0600`.
- `wotwot todo …` — CLI that talks to the running API to mutate the todo list.
- `wotwot info …` — CLI for the info stack (LIFO; the dashboard loops through it).

Override the socket path with `--sock <path>` (on `run`) or `WOTWOT_SOCK=<path>`
(everywhere). Run `wotwot run --headless` to skip the TUI — useful for
daemonising under launchd/systemd.

## Install

```sh
# from source (only supported method right now)
git clone https://github.com/sugendran/wotwot.git
cd wotwot
cargo install --path .
```

That puts the `wotwot` binary in `~/.cargo/bin`. Make sure that's on
your `PATH`.

Or, without installing globally:

```sh
cargo run --release -- run
```

### Optional dependencies

Per-pane requirements (each is optional — the dashboard still renders
without them, the pane just shows an "unavailable" hint):

| Pane | Needs |
|---|---|
| claude code | `npx` (Node.js) on PATH — pulled in transparently via `npx -y ccusage` |
| docker | the `docker` CLI, with the daemon running |

## Panes

```
+-----------------------------+
|        claude code          |  ccusage --json (best effort)
+-----------------------------+
|           todos             |  numbered list
+-----------------------------+
|          docker             |  docker stats --no-stream
+-----------------------------+
|       info / quote          |  info stack, falls back to quotes
+-----------------------------+
```

## CLI

```sh
wotwot run                       # start dashboard

wotwot todo add "buy milk"
wotwot todo list
wotwot todo rm 1                 # by 1-based index or uuid
wotwot todo reorder <id1> <id2>  # listed ids move to front

wotwot info push "deploy at 3pm"
wotwot info list
wotwot info pop                  # remove top
wotwot info rm 2
```

Point the CLI at a different socket with `WOTWOT_SOCK=<path>`.

State is persisted at `~/Library/Application Support/wotwot/state.json` (macOS).

## Agents

`wotwot agents` prints a markdown guide for AI agents on how (and when)
to use the todo and info commands. Pipe it into your agent's system
prompt.

## Quit

Press `q`, `Esc`, or `Ctrl-C` in the TUI.
