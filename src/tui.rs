use crate::state::{SharedState, QUOTES};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::{io, time::Duration};

const WIDTH: u16 = 55;

pub async fn run(state: SharedState) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let res = run_loop(&mut term, state).await;

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    res
}

async fn run_loop<B: ratatui::backend::Backend>(
    term: &mut Terminal<B>,
    state: SharedState,
) -> Result<()> {
    loop {
        let snap = state.read().await.clone();
        term.draw(|f| draw(f, &snap))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c')
                        if k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        return Ok(())
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, s: &crate::state::AppState) {
    let area = f.area();
    let w = WIDTH.min(area.width);
    let root = Rect {
        x: area.x,
        y: area.y,
        width: w,
        height: area.height,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // claude
            Constraint::Min(5),     // todos
            Constraint::Length(8),  // docker
            Constraint::Length(6),  // info / quote
        ])
        .split(root);

    draw_claude(f, chunks[0], s);
    draw_todos(f, chunks[1], s);
    draw_docker(f, chunks[2], s);
    draw_info(f, chunks[3], s);
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
}

fn draw_claude(f: &mut ratatui::Frame, area: Rect, s: &crate::state::AppState) {
    let mut lines: Vec<Line> = vec![];
    let c = &s.claude;
    if c.today_usd.is_none() && c.today_tokens.is_none() && c.raw.is_none() {
        lines.push(Line::from(Span::styled(
            "ccusage unavailable",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from("install: npm i -g ccusage"));
    } else {
        if let Some(u) = c.today_usd {
            lines.push(Line::from(format!("Today  ${:.2}", u)));
        }
        if let Some(t) = c.today_tokens {
            lines.push(Line::from(format!("Tokens {}", fmt_num(t))));
        }
        if lines.is_empty() {
            lines.push(Line::from("ccusage: data parsed but empty"));
        }
    }
    let p = Paragraph::new(lines)
        .block(block("claude code"))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn fmt_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn draw_todos(f: &mut ratatui::Frame, area: Rect, s: &crate::state::AppState) {
    let lines: Vec<Line> = if s.todos.is_empty() {
        vec![Line::from(Span::styled(
            "no todos — add via CLI",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        s.todos
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Line::from(vec![
                    Span::styled(
                        format!("{:>2}. ", i + 1),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(t.text.clone()),
                ])
            })
            .collect()
    };
    let p = Paragraph::new(lines)
        .block(block("todos"))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn draw_docker(f: &mut ratatui::Frame, area: Rect, s: &crate::state::AppState) {
    let lines: Vec<Line> = if s.docker.is_empty() {
        vec![Line::from(Span::styled(
            "no running containers",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        s.docker
            .iter()
            .take(area.height.saturating_sub(2) as usize)
            .map(|d| {
                let name = truncate(&d.name, 38);
                Line::from(vec![
                    Span::raw(format!("{:<38} ", name)),
                    Span::styled(
                        format!("{:>8}", d.cpu),
                        Style::default().fg(Color::Green),
                    ),
                ])
            })
            .collect()
    };
    let p = Paragraph::new(lines).block(block("docker"));
    f.render_widget(p, area);
}

fn draw_info(f: &mut ratatui::Frame, area: Rect, s: &crate::state::AppState) {
    let (title, text) = if !s.info.is_empty() {
        let idx = s.info_cursor.min(s.info.len() - 1);
        let item = &s.info[idx];
        (
            format!("info {}/{}", idx + 1, s.info.len()),
            item.text.clone(),
        )
    } else {
        let q = QUOTES
            .get(s.quote_cursor % QUOTES.len().max(1))
            .copied()
            .unwrap_or("");
        ("quote".to_string(), q.to_string())
    };
    let p = Paragraph::new(text)
        .block(block(&title))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
