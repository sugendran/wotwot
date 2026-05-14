use crate::state::{ClaudeUsage, DockerSvc, SharedState};
use std::time::Duration;
use tokio::process::Command;

pub async fn run(state: SharedState) {
    let s1 = state.clone();
    tokio::spawn(async move {
        loop {
            let stats = collect_docker().await;
            {
                let mut g = s1.write().await;
                g.docker = stats;
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    let s2 = state.clone();
    tokio::spawn(async move {
        loop {
            let u = collect_claude().await;
            {
                let mut g = s2.write().await;
                g.claude = u;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    let s3 = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(6)).await;
            let mut g = s3.write().await;
            let n = g.info.len();
            if n > 0 {
                g.info_cursor = (g.info_cursor + 1) % n;
            }
            g.quote_cursor = (g.quote_cursor + 1) % crate::state::QUOTES.len().max(1);
        }
    });
}

async fn collect_docker() -> Vec<DockerSvc> {
    let out = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.CPUPerc}}",
        ])
        .output()
        .await;
    let Ok(out) = out else { return vec![] };
    if !out.status.success() {
        return vec![];
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines()
        .filter_map(|l| {
            let mut it = l.splitn(2, '\t');
            let name = it.next()?.trim().to_string();
            let cpu = it.next()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            Some(DockerSvc { name, cpu })
        })
        .collect()
}

async fn collect_claude() -> ClaudeUsage {
    // Try `ccusage --json` (npx fallback). Best effort; ignore failures.
    let attempts: Vec<(&str, Vec<&str>)> = vec![
        ("ccusage", vec!["--json"]),
        ("npx", vec!["-y", "ccusage", "--json"]),
    ];
    for (bin, args) in attempts {
        let out = Command::new(bin).args(&args).output().await;
        if let Ok(out) = out {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                let mut u = ClaudeUsage {
                    raw: Some(s.clone()),
                    ..Default::default()
                };
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                    // try common shapes
                    if let Some(today) = v.get("today") {
                        u.today_usd = today.get("totalCost").and_then(|x| x.as_f64());
                        u.today_tokens = today.get("totalTokens").and_then(|x| x.as_u64());
                    } else if let Some(arr) = v.get("daily").and_then(|x| x.as_array()) {
                        if let Some(last) = arr.last() {
                            u.today_usd = last.get("totalCost").and_then(|x| x.as_f64());
                            u.today_tokens = last.get("totalTokens").and_then(|x| x.as_u64());
                        }
                    }
                }
                return u;
            }
        }
    }
    ClaudeUsage::default()
}
