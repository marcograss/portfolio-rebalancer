use std::io;

use clap::{Arg, Command};
use portfolio::{Action, BuySell};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Line, Span};
use tui::widgets::{BarChart, Block, Borders, Paragraph, Tabs};
use tui::Terminal;

mod portfolio;
mod tuiutil;

use crate::tuiutil::event::{Event, Events};
use crate::tuiutil::TabsState;

struct TuiApp<'a> {
    tabs: TabsState<'a>,
}

fn get_actions_to_display(actions: &[Action]) -> Vec<Line> {
    let mut ret = Vec::new();
    for a in actions {
        match a.buysell {
            BuySell::Buy => ret.push(Line::from(Span::styled(
                format!(
                    "{} {} {} -{:.2}$\n",
                    "BUY", a.amount, a.name, a.transaction_value
                ),
                Style::default().fg(Color::Red),
            ))),
            BuySell::Sell => ret.push(Line::from(Span::styled(
                format!(
                    "{} {} {} +{:.2}$\n",
                    "SELL", a.amount, a.name, a.transaction_value
                ),
                Style::default().fg(Color::Green),
            ))),
        }
    }
    ret
}

fn main() -> anyhow::Result<()> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::new("portfolio-file").required(true).index(1))
        .get_matches();
    let portfolio_file = matches.get_one::<String>("portfolio-file").unwrap();

    let load_res = portfolio::load_portfolio_from_file(portfolio_file);
    match load_res {
        Ok(mut original_portfolio) => {
            // println!("Original {:?}", _original_portfolio);
            let mut target_portfolio = if original_portfolio.donotsell {
                original_portfolio.add_without_selling()
            } else {
                original_portfolio.rebalance()
            };
            // println!("Rebalanced {:?}", _target_portfolio);

            let actions = original_portfolio.get_actions(&target_portfolio);
            // println!("Actions {:?}", _actions);
            let display_actions = get_actions_to_display(&actions);

            let original_alloc_data: Vec<(&str, u64)> = original_portfolio.get_display_data();
            let target_alloc_data: Vec<(&str, u64)> = target_portfolio.get_display_data();

            let stdout = io::stdout().into_raw_mode()?;
            let stdout = MouseTerminal::from(stdout);
            let stdout = stdout.into_alternate_screen()?;
            let backend = TermionBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            terminal.hide_cursor()?;

            let events = Events::new();

            // App
            let mut app = TuiApp {
                tabs: TabsState::new(vec!["Original/New Allocations", "Actions"]),
            };

            // Main loop
            loop {
                terminal.draw(|f| {
                    let size = f.size();
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                        .split(size);
                    let t = Tabs::new(app.tabs.titles.iter().copied().map(Line::from).collect())
                        .block(Block::default().borders(Borders::ALL).title("Tabs"))
                        .select(app.tabs.index)
                        .style(Style::default().fg(Color::Cyan))
                        .highlight_style(Style::default().fg(Color::Yellow));
                    f.render_widget(t, chunks[0]);
                    let allocations_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(chunks[1]);
                    match app.tabs.index {
                        0 => {
                            let b1 = BarChart::default()
                                .block(
                                    Block::default()
                                        .title("Original Allocation (%)")
                                        .borders(Borders::ALL),
                                )
                                .data(&original_alloc_data)
                                .bar_width(5)
                                .bar_gap(3)
                                .style(Style::default().fg(Color::Green))
                                .value_style(
                                    Style::default()
                                        .bg(Color::White)
                                        .add_modifier(Modifier::BOLD),
                                );
                            f.render_widget(b1, allocations_chunks[0]);
                            let b2 = BarChart::default()
                                .block(
                                    Block::default()
                                        .title("New Allocation (%)")
                                        .borders(Borders::ALL),
                                )
                                .data(&target_alloc_data)
                                .style(Style::default().fg(Color::Red))
                                .bar_width(5)
                                .bar_gap(3)
                                .value_style(Style::default().bg(Color::White))
                                .label_style(
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::ITALIC),
                                );
                            f.render_widget(b2, allocations_chunks[1]);
                        }
                        1 => {
                            let block = Block::default().borders(Borders::ALL);
                            let p = Paragraph::new(display_actions.clone())
                                .block(block)
                                .alignment(Alignment::Left);
                            f.render_widget(p, chunks[1]);
                        }
                        _ => {}
                    }
                })?;
                if let Event::Input(input) = events.next()? {
                    match input {
                        Key::Char('q') => {
                            break;
                        }
                        Key::Right => app.tabs.next(),
                        Key::Left => app.tabs.previous(),
                        _ => {}
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{e:?}");
        }
    }

    Ok(())
}
