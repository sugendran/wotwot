use anyhow::{anyhow, Context, Result};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

fn sock_path() -> std::path::PathBuf {
    crate::server::default_socket_path()
}

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
