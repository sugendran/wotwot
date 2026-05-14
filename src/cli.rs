use anyhow::{anyhow, Context, Result};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

fn sock_path() -> std::path::PathBuf {
    crate::server::default_socket_path()
}

pub const AGENTS_GUIDE: &str = r#"# wotwot — guide for agents

wotwot is a tiny terminal dashboard the user keeps open while working.
You can drive two of its panes from the command line:

  - todos: an ordered checklist the user is working through
  - info:  a LIFO stack of short reminders that cycle every ~6s

Both speak markdown and ANSI escapes (bold, italic, inline `code`,
links, colours). Keep entries short — the pane is 55 columns wide.

## When to use which

Use **todo** for work the user must do or that you are about to do
on their behalf. Each todo represents one action. Remove it when it
is done; do not let stale items pile up.

Use **info** for short, time-bounded context the user should keep in
mind for the next while: deploy windows, oncall, who is waiting on
what, a meeting time, a number to remember. Push it; it disappears
when you pop it.

Do not mirror plan steps into todos — the user has their own task
tracker for that. Only surface items that genuinely belong on a
dashboard a human glances at.

## Commands

  wotwot todo add "<markdown text>"
  wotwot todo list                    # numbered, 1-based
  wotwot todo rm <index-or-uuid>
  wotwot todo reorder <uuid> <uuid>…  # listed ids float to the top,
                                      # the rest keep their relative order

  wotwot info push "<markdown text>"
  wotwot info list
  wotwot info pop                     # remove the top item
  wotwot info rm <index-or-uuid>

The CLI exits non-zero if the dashboard is not running.

## Formatting tips

  **bold**, *italic*, `code`, [link](https://…)
  ANSI escapes also work: $(printf '\033[1;31m…\033[0m')

Prefer plain markdown — it is easier to read and round-trips through
logs. Reserve colour for genuine urgency.

## Discovery

  WOTWOT_SOCK overrides the socket path. Default location:
    $XDG_RUNTIME_DIR/wotwot/wotwot.sock  (Linux)
    ~/Library/Caches/wotwot/wotwot.sock  (macOS, typical)
"#;

async fn request(method: &str, path: &str, body: Option<&serde_json::Value>) -> Result<String> {
    let sp = sock_path();
    let mut stream = UnixStream::connect(&sp)
        .await
        .with_context(|| format!("connect {}", sp.display()))?;

    let body_bytes = match body {
        Some(v) => serde_json::to_vec(v)?,
        None => Vec::new(),
    };

    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: wotwot\r\nConnection: close\r\nContent-Length: {}\r\n",
        body_bytes.len()
    );
    if body.is_some() {
        req.push_str("Content-Type: application/json\r\n");
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).await?;
    if !body_bytes.is_empty() {
        stream.write_all(&body_bytes).await?;
    }
    stream.flush().await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;

    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("malformed response"))?;
    let head = std::str::from_utf8(&buf[..split])?;
    let body = &buf[split + 4..];

    let status_line = head.lines().next().unwrap_or("");
    let code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);

    // very small responses, none chunked from axum here
    let body_str = String::from_utf8_lossy(body).to_string();
    if !(200..300).contains(&code) {
        return Err(anyhow!("HTTP {code}: {body_str}"));
    }
    Ok(body_str)
}

pub async fn todo_add(text: String) -> Result<()> {
    println!("{}", request("POST", "/todo", Some(&json!({ "text": text }))).await?);
    Ok(())
}

pub async fn todo_rm(id: String) -> Result<()> {
    println!("{}", request("DELETE", &format!("/todo/{id}"), None).await?);
    Ok(())
}

pub async fn todo_list() -> Result<()> {
    let body = request("GET", "/todo", None).await?;
    print_indexed(&body);
    Ok(())
}

pub async fn todo_reorder(ids: Vec<String>) -> Result<()> {
    println!(
        "{}",
        request("POST", "/todo/reorder", Some(&json!({ "ids": ids }))).await?
    );
    Ok(())
}

pub async fn info_push(text: String) -> Result<()> {
    println!("{}", request("POST", "/info", Some(&json!({ "text": text }))).await?);
    Ok(())
}

pub async fn info_pop() -> Result<()> {
    println!("{}", request("POST", "/info/pop", None).await?);
    Ok(())
}

pub async fn info_rm(id: String) -> Result<()> {
    println!("{}", request("DELETE", &format!("/info/{id}"), None).await?);
    Ok(())
}

pub async fn info_list() -> Result<()> {
    let body = request("GET", "/info", None).await?;
    print_indexed(&body);
    Ok(())
}

fn print_indexed(body: &str) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(arr) = v.as_array() {
            for (i, t) in arr.iter().enumerate() {
                println!(
                    "{:>2}. {}",
                    i + 1,
                    t.get("text").and_then(|x| x.as_str()).unwrap_or("")
                );
            }
            return;
        }
    }
    println!("{body}");
}
