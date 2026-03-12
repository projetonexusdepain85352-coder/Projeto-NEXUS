use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use sqlx::PgPool;
use std::io;
use tracing::info;

use crate::{
    db::{approve_model, list_models_pending_approval, reject_model},
    error::Result,
};

pub async fn run_approval_tui(pool: &PgPool) -> Result<()> {
    let mut models = list_models_pending_approval(pool).await?;
    if models.is_empty() {
        println!("Nenhum modelo aguardando aprovacao.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    let mut state = TableState::default();
    state.select(Some(0));
    let mut messages: Vec<String> = Vec::new();

    loop {
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Min(5),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(f.area());

            let header = Row::new(vec![
                Cell::from("Nome").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Dominio").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Docs").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Score").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from("Criado em").style(Style::default().add_modifier(Modifier::BOLD)),
            ])
            .style(Style::default().bg(Color::DarkGray));

            let rows: Vec<Row> = models
                .iter()
                .map(|m| {
                    let score = m
                        .benchmark_score
                        .map(|s| format!("{:.3}", s))
                        .unwrap_or("--".into());
                    let created = m.created_at.format("%Y-%m-%d %H:%M").to_string();
                    Row::new(vec![
                        Cell::from(m.name.clone()),
                        Cell::from(m.domain.clone()),
                        Cell::from(m.dataset_size.to_string()),
                        Cell::from(score),
                        Cell::from(created),
                    ])
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(30),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(20),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" NEXUS MTP -- Aprovacao "),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, chunks[0], &mut state);

            let help = ratatui::widgets::Paragraph::new(
                "  [up/down] Navegar   [a] Aprovar   [r] Rejeitar   [q] Sair",
            )
            .block(Block::default().borders(Borders::ALL).title(" Comandos "));
            f.render_widget(help, chunks[1]);

            let msg =
                ratatui::widgets::Paragraph::new(messages.last().cloned().unwrap_or_default())
                    .block(Block::default().borders(Borders::ALL).title(" Log "));
            f.render_widget(msg, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? && let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down => {
                    let i = state
                        .selected()
                        .map(|s| (s + 1).min(models.len().saturating_sub(1)))
                        .unwrap_or(0);
                    state.select(Some(i));
                }
                KeyCode::Up => {
                    let i = state.selected().map(|s| s.saturating_sub(1)).unwrap_or(0);
                    state.select(Some(i));
                }
                KeyCode::Char('a') => {
                    if let Some(idx) = state.selected() && idx < models.len() {
                        let m = &models[idx];
                        approve_model(pool, m.id).await?;
                        let msg = format!("OK '{}' APROVADO.", m.name);
                        info!("{}", msg);
                        messages.push(msg);
                        models.remove(idx);
                        if models.is_empty() {
                            state.select(None);
                        } else {
                            state.select(Some(idx.min(models.len() - 1)));
                        }
                    }
                }
                KeyCode::Char('r') => {
                    if let Some(idx) = state.selected() && idx < models.len() {
                        let m = &models[idx];
                        reject_model(pool, m.id).await?;
                        let msg = format!("XX '{}' REJEITADO.", m.name);
                        info!("{}", msg);
                        messages.push(msg);
                        models.remove(idx);
                        if models.is_empty() {
                            state.select(None);
                        } else {
                            state.select(Some(idx.min(models.len() - 1)));
                        }
                    }
                }
                _ => {}
            }
            if models.is_empty() {
                messages.push("Todos processados. [q] para sair.".into());
            }
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    Ok(())
}
