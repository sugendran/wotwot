mod cli;
mod collectors;
mod server;
mod state;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "wotwot", about = "Tiny terminal dashboard")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Launch the dashboard + API server
    Run {
        #[arg(long, default_value_t = server::DEFAULT_PORT)]
        port: u16,
    },
    /// Manage todos
    Todo {
        #[command(subcommand)]
        action: TodoCmd,
    },
    /// Manage info stack
    Info {
        #[command(subcommand)]
        action: InfoCmd,
    },
}

#[derive(Subcommand)]
enum TodoCmd {
    Add { text: Vec<String> },
    Rm { id: String },
    List,
    Reorder { ids: Vec<String> },
}

#[derive(Subcommand)]
enum InfoCmd {
    Push { text: Vec<String> },
    Pop,
    Rm { id: String },
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    match args.cmd.unwrap_or(Cmd::Run {
        port: server::DEFAULT_PORT,
    }) {
        Cmd::Run { port } => run_dashboard(port).await,
        Cmd::Todo { action } => match action {
            TodoCmd::Add { text } => cli::todo_add(text.join(" ")).await,
            TodoCmd::Rm { id } => cli::todo_rm(id).await,
            TodoCmd::List => cli::todo_list().await,
            TodoCmd::Reorder { ids } => cli::todo_reorder(ids).await,
        },
        Cmd::Info { action } => match action {
            InfoCmd::Push { text } => cli::info_push(text.join(" ")).await,
            InfoCmd::Pop => cli::info_pop().await,
            InfoCmd::Rm { id } => cli::info_rm(id).await,
            InfoCmd::List => cli::info_list().await,
        },
    }
}

async fn run_dashboard(port: u16) -> Result<()> {
    let initial = state::load().await;
    let shared = Arc::new(RwLock::new(initial));

    collectors::run(shared.clone()).await;

    let app = server::router(shared.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    tui::run(shared).await
}
