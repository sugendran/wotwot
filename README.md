# wotwot

Tiny 55-col terminal dashboard built with [ratatui](https://ratatui.rs).

One binary, three roles:

- `wotwot run` — launches the TUI **and** a local HTTP API (default `127.0.0.1:47291`).
- `wotwot todo …` — CLI that talks to the running API to mutate the todo list.
- `wotwot info …` — CLI for the info stack (LIFO; the dashboard loops through it).

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

Point the CLI elsewhere with `WOTWOT_URL=http://host:port`.

State is persisted at `~/Library/Application Support/wotwot/state.json` (macOS).

## Quit

Press `q`, `Esc`, or `Ctrl-C` in the TUI.
