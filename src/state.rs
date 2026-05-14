use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoItem {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DockerSvc {
    pub name: String,
    pub cpu: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub today_usd: Option<f64>,
    pub today_tokens: Option<u64>,
    /// tokens used in the currently-active 5h block
    pub block_tokens: Option<u64>,
    /// projected token limit for the current block (from ccusage)
    pub block_limit: Option<u64>,
    pub raw: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Persisted {
    pub todos: Vec<Todo>,
    pub info: Vec<InfoItem>,
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub todos: Vec<Todo>,
    pub info: Vec<InfoItem>,
    pub docker: Vec<DockerSvc>,
    pub claude: ClaudeUsage,
    pub info_cursor: usize,
    pub quote_cursor: usize,
}

pub type SharedState = Arc<RwLock<AppState>>;

pub const QUOTES: &[&str] = &[
    "Make it work, then make it right.",
    "Small commits, big confidence.",
    "Naming is half the design.",
    "Boring tech ships.",
    "Read the error message.",
    "Optimise for the reviewer.",
    "Delete code; it's a liability.",
    "Cache invalidation is hard.",
];

pub fn state_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("wotwot");
    let _ = std::fs::create_dir_all(&p);
    p.push("state.json");
    p
}

pub async fn load() -> AppState {
    let path = state_path();
    let mut s = AppState::default();
    if let Ok(bytes) = tokio::fs::read(&path).await {
        if let Ok(p) = serde_json::from_slice::<Persisted>(&bytes) {
            s.todos = p.todos;
            s.info = p.info;
        }
    }
    s
}

pub async fn save(state: &AppState) -> Result<()> {
    let p = Persisted {
        todos: state.todos.clone(),
        info: state.info.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&p)?;
    tokio::fs::write(state_path(), bytes).await?;
    Ok(())
}
