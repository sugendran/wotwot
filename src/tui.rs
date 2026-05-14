use crate::state::{SharedState, QUOTES};
use ansi_to_tui::IntoText;
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
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Terminal,
};
use std::{io, time::Duration};

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
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
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
    let root = f.area();

    let docker_rows = 2 + s.docker.len().max(1) as u16; // borders + at least 1 line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),            // claude (3 lines + borders + gauge)
            Constraint::Min(5),               // todos
            Constraint::Length(docker_rows),  // docker — fits all rows
            Constraint::Length(6),            // info / quote
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
    let c = &s.claude;

    // outer block + inner area
    let outer = block("claude code");
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    if c.today_usd.is_none() && c.today_tokens.is_none() && c.block_tokens.is_none() {
        let p = Paragraph::new(vec![
            Line::from(Span::styled(
                "ccusage unavailable",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("needs: npx (Node) on PATH"),
        ]);
        f.render_widget(p, inner);
        return;
    }

    // top line: today's $ + tokens
    let mut header = String::new();
    if let Some(u) = c.today_usd {
        header.push_str(&format!("Today ${:.2}", u));
    }
    if let Some(t) = c.today_tokens {
        if !header.is_empty() {
            header.push_str("   ");
        }
        header.push_str(&format!("Tokens {}", fmt_num(t)));
    }

    // bar label
    let (used, limit, ratio) = match (c.block_tokens, c.block_limit) {
        (Some(t), Some(l)) if l > 0 => (t, Some(l), (t as f64 / l as f64).min(1.0)),
        (Some(t), _) => (t, None, 0.0),
        _ => (0, None, 0.0),
    };
    let label = if let Some(l) = limit {
        format!("{} / {} ({:.0}%)", fmt_num(used), fmt_num(l), ratio * 100.0)
    } else if c.block_tokens.is_some() {
        format!("{} (no limit)", fmt_num(used))
    } else {
        "no active block".to_string()
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // "5h block" label
            Constraint::Length(1), // gauge
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(header), rows[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("5h block ", Style::default().fg(Color::DarkGray)),
            Span::raw(label),
        ])),
        rows[1],
    );

    let bar_colour = match (ratio * 100.0) as u32 {
        0..=59 => Color::Green,
        60..=84 => Color::Yellow,
        _ => Color::Red,
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(bar_colour))
        .ratio(if limit.is_some() { ratio } else { 0.0 })
        .label(""); // label printed above so we control formatting
    f.render_widget(gauge, rows[2]);
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
    let text: Text = if s.todos.is_empty() {
        Text::from(Line::from(Span::styled(
            "no todos — add via CLI",
            Style::default().fg(Color::DarkGray),
        )))
    } else {
        let mut out = Text::default();
        let last = s.todos.len() - 1;
        for (i, t) in s.todos.iter().enumerate() {
            let prefix = Span::styled(
                format!("{:>2}. ", i + 1),
                Style::default().fg(Color::Yellow),
            );
            let body = render_rich(&t.text);
            // first body line gets the index prefix; subsequent lines indent 4
            let mut body_lines = body.lines.into_iter();
            if let Some(first) = body_lines.next() {
                let mut spans = vec![prefix];
                spans.extend(first.spans);
                out.lines.push(Line::from(spans));
            } else {
                out.lines.push(Line::from(prefix));
            }
            for rest in body_lines {
                let mut spans = vec![Span::raw("    ")];
                spans.extend(rest.spans);
                out.lines.push(Line::from(spans));
            }
            if i != last {
                out.lines.push(Line::from(""));
            }
        }
        out
    };
    let p = Paragraph::new(text)
        .block(block("todos"))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

/// Render todo/info text as styled Text:
/// - ANSI escapes (\x1b[...) → ansi-to-tui
/// - otherwise → tui-markdown (handles plain text fine)
fn render_rich(s: &str) -> Text<'static> {
    if s.contains('\x1b') {
        if let Ok(t) = s.as_bytes().into_text() {
            return t;
        }
    }
    let md = tui_markdown::from_str(s);
    // tui-markdown returns Text<'a> borrowing from `s`; convert to owned.
    let owned: Vec<Line<'static>> = md
        .lines
        .into_iter()
        .map(|l| {
            let spans = l
                .spans
                .into_iter()
                .map(|sp| Span::styled(sp.content.into_owned(), sp.style))
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect();
    Text::from(owned)
}

fn draw_docker(f: &mut ratatui::Frame, area: Rect, s: &crate::state::AppState) {
    let lines: Vec<Line> = if s.docker.is_empty() {
        vec![Line::from(Span::styled(
            "no running containers",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        // borders=2, cpu col=8, gap=1
        let name_w = (area.width as usize).saturating_sub(2 + 8 + 1).max(1);
        s.docker
            .iter()
            .map(|d| {
                let name = truncate(&d.name, name_w);
                Line::from(vec![
                    Span::raw(format!("{:<width$} ", name, width = name_w)),
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
    let p = Paragraph::new(render_rich(&text))
        .block(block(&title))
        .wrap(Wrap { trim: false });
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
