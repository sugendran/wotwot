mod cli;
mod collectors;
mod server;
mod state;
mod tui;

use anyhow::Result;
use axum::Router;
use clap::{Parser, Subcommand};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tower::Service;

async fn serve_uds(listener: UnixListener, app: Router) {
    loop {
        let (stream, _addr) = match listener.accept().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let io = TokioIo::new(stream);
        let app = app.clone();
        let svc = hyper::service::service_fn(move |req: hyper::Request<Incoming>| {
            let mut app = app.clone();
            async move { app.call(req).await }
        });
        tokio::spawn(async move {
            let _ = ConnBuilder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await;
        });
    }
}

#[derive(Parser)]
#[command(name = "wotwot", about = "Tiny terminal dashboard")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Launch the dashboard + API server (Unix domain socket)
    Run {
        /// Override the socket path (defaults to $WOTWOT_SOCK or
        /// <runtime-dir>/wotwot/wotwot.sock).
        #[arg(long)]
        sock: Option<std::path::PathBuf>,
        /// Run the API + collectors without the TUI (useful for daemonising).
        #[arg(long)]
        headless: bool,
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
    /// Print a guide for AI agents on how to drive todos/info
    Agents,
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
        sock: None,
        headless: false,
    }) {
        Cmd::Run { sock, headless } => run_dashboard(sock, headless).await,
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
        Cmd::Agents => {
            print!("{}", cli::AGENTS_GUIDE);
            Ok(())
        }
    }
}

async fn run_dashboard(sock: Option<std::path::PathBuf>, headless: bool) -> Result<()> {
    let initial = state::load().await;
    let shared = Arc::new(RwLock::new(initial));

    collectors::run(shared.clone()).await;

    let path = sock.unwrap_or_else(server::default_socket_path);
    if path.exists() {
        // stale socket from a previous run
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let listener = tokio::net::UnixListener::bind(&path)?;
    // only the current user should be able to talk to us
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    let app = server::router(shared.clone());
    let path_for_cleanup = path.clone();
    tokio::spawn(async move {
        serve_uds(listener, app).await;
        let _ = std::fs::remove_file(&path_for_cleanup);
    });

    let res = if headless {
        eprintln!("wotwot: listening on {}", path.display());
        tokio::signal::ctrl_c().await.ok();
        Ok(())
    } else {
        tui::run(shared).await
    };
    let _ = std::fs::remove_file(&path);
    res
}
