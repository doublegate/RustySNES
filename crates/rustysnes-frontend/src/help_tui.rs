//! The interactive ratatui terminal help browser (`rustysnes help --interactive`).
//!
//! Native-only, behind the default-on `help-tui` feature. A minimal first cut: a left-pane topic
//! list + a right-pane body, navigable with the arrow keys / `j`/`k`, `q` to quit. The topic
//! text comes from [`crate::cli::topic_text`] so the TUI and the static `help <topic>` output
//! never drift.
//!
//! v0.1.0: the renderer is a small, self-contained loop (the deep RustyNES help-browser polish —
//! search, scrollback, intra-doc links — is a TODO for the implementation phase).

use std::io;

use ratatui::Terminal;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::crossterm::{execute, terminal};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use std::io::Stdout;

use crate::cli::{TOPICS, topic_text};

/// Run the interactive help browser. Restores the terminal on exit (even on error).
///
/// # Errors
/// Returns an [`io::Error`] if the terminal cannot be put into / out of raw mode or drawn to.
pub fn run() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend).map_err(io::Error::other)?;

    let result = event_loop(&mut term);

    // Always restore, even if the loop errored.
    terminal::disable_raw_mode()?;
    execute!(term.backend_mut(), terminal::LeaveAlternateScreen)?;
    term.show_cursor()?;
    result
}

fn event_loop(term: &mut Terminal<ratatui::backend::CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut selected = 0usize;
    loop {
        let body = topic_text(TOPICS[selected]).unwrap_or("(no text)");
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
                .split(f.area());

            let items: Vec<ListItem> = TOPICS.iter().map(|t| ListItem::new(*t)).collect();
            let mut state = ListState::default();
            state.select(Some(selected));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Topics"))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            f.render_stateful_widget(list, chunks[0], &mut state);

            let para = Paragraph::new(Text::raw(body))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("RustySNES help — {}", TOPICS[selected])),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(para, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1) % TOPICS.len();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = (selected + TOPICS.len() - 1) % TOPICS.len();
                }
                _ => {}
            }
        }
    }
    Ok(())
}
