use anyhow::{Context, Result};
use serde_json::json;

fn base() -> String {
    std::env::var("WOTWOT_URL")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{}", crate::server::DEFAULT_PORT))
}

pub async fn todo_add(text: String) -> Result<()> {
    let r = reqwest::Client::new()
        .post(format!("{}/todo", base()))
        .json(&json!({ "text": text }))
        .send()
        .await
        .context("POST /todo")?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn todo_rm(id: String) -> Result<()> {
    let r = reqwest::Client::new()
        .delete(format!("{}/todo/{}", base(), id))
        .send()
        .await?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn todo_list() -> Result<()> {
    let r = reqwest::Client::new()
        .get(format!("{}/todo", base()))
        .send()
        .await?;
    let v: serde_json::Value = r.json().await?;
    if let Some(arr) = v.as_array() {
        for (i, t) in arr.iter().enumerate() {
            println!(
                "{:>2}. {}",
                i + 1,
                t.get("text").and_then(|x| x.as_str()).unwrap_or("")
            );
        }
    }
    Ok(())
}

pub async fn todo_reorder(ids: Vec<String>) -> Result<()> {
    let r = reqwest::Client::new()
        .post(format!("{}/todo/reorder", base()))
        .json(&json!({ "ids": ids }))
        .send()
        .await?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn info_push(text: String) -> Result<()> {
    let r = reqwest::Client::new()
        .post(format!("{}/info", base()))
        .json(&json!({ "text": text }))
        .send()
        .await?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn info_pop() -> Result<()> {
    let r = reqwest::Client::new()
        .post(format!("{}/info/pop", base()))
        .send()
        .await?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn info_rm(id: String) -> Result<()> {
    let r = reqwest::Client::new()
        .delete(format!("{}/info/{}", base(), id))
        .send()
        .await?;
    println!("{}", r.text().await?);
    Ok(())
}

pub async fn info_list() -> Result<()> {
    let r = reqwest::Client::new()
        .get(format!("{}/info", base()))
        .send()
        .await?;
    let v: serde_json::Value = r.json().await?;
    if let Some(arr) = v.as_array() {
        for (i, t) in arr.iter().enumerate() {
            println!(
                "{:>2}. {}",
                i + 1,
                t.get("text").and_then(|x| x.as_str()).unwrap_or("")
            );
        }
    }
    Ok(())
}
