use crate::state::{save, InfoItem, SharedState, Todo};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

pub const DEFAULT_PORT: u16 = 47291;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/state", get(get_state))
        .route("/todo", post(add_todo).get(list_todos))
        .route("/todo/:id", delete(rm_todo))
        .route("/todo/reorder", post(reorder_todos))
        .route("/info", post(push_info).get(list_info))
        .route("/info/:id", delete(rm_info))
        .route("/info/pop", post(pop_info))
        .with_state(state)
}

async fn get_state(State(s): State<SharedState>) -> Json<serde_json::Value> {
    let s = s.read().await;
    Json(serde_json::json!({
        "todos": s.todos,
        "info": s.info,
        "docker": s.docker,
        "claude": s.claude,
    }))
}

#[derive(Deserialize)]
struct TextBody {
    text: String,
}

async fn add_todo(State(s): State<SharedState>, Json(b): Json<TextBody>) -> Json<Todo> {
    let t = Todo {
        id: Uuid::new_v4().to_string(),
        text: b.text,
    };
    let mut g = s.write().await;
    g.todos.push(t.clone());
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(t)
}

async fn list_todos(State(s): State<SharedState>) -> Json<Vec<Todo>> {
    Json(s.read().await.todos.clone())
}

async fn rm_todo(State(s): State<SharedState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut g = s.write().await;
    let before = g.todos.len();
    // accept either id or 1-based index
    if let Ok(idx) = id.parse::<usize>() {
        if idx >= 1 && idx <= g.todos.len() {
            g.todos.remove(idx - 1);
        }
    } else {
        g.todos.retain(|t| t.id != id);
    }
    let removed = before != g.todos.len();
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(serde_json::json!({ "removed": removed }))
}

#[derive(Deserialize)]
struct ReorderBody {
    ids: Vec<String>,
}

async fn reorder_todos(
    State(s): State<SharedState>,
    Json(b): Json<ReorderBody>,
) -> Json<serde_json::Value> {
    let mut g = s.write().await;
    let mut new_order: Vec<Todo> = Vec::with_capacity(g.todos.len());
    for id in &b.ids {
        if let Some(pos) = g.todos.iter().position(|t| &t.id == id) {
            new_order.push(g.todos.remove(pos));
        }
    }
    // append leftovers in their original order
    new_order.extend(g.todos.drain(..));
    g.todos = new_order;
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(serde_json::json!({ "ok": true }))
}

async fn push_info(State(s): State<SharedState>, Json(b): Json<TextBody>) -> Json<InfoItem> {
    let i = InfoItem {
        id: Uuid::new_v4().to_string(),
        text: b.text,
    };
    let mut g = s.write().await;
    g.info.push(i.clone());
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(i)
}

async fn list_info(State(s): State<SharedState>) -> Json<Vec<InfoItem>> {
    Json(s.read().await.info.clone())
}

async fn pop_info(State(s): State<SharedState>) -> Json<serde_json::Value> {
    let mut g = s.write().await;
    let popped = g.info.pop();
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(serde_json::json!({ "popped": popped }))
}

async fn rm_info(State(s): State<SharedState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut g = s.write().await;
    let before = g.info.len();
    if let Ok(idx) = id.parse::<usize>() {
        if idx >= 1 && idx <= g.info.len() {
            g.info.remove(idx - 1);
        }
    } else {
        g.info.retain(|i| i.id != id);
    }
    let removed = before != g.info.len();
    let snap = g.clone();
    drop(g);
    let _ = save(&snap).await;
    Json(serde_json::json!({ "removed": removed }))
}
