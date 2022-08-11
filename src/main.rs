use std::io;

use clap::{App, Arg};
use portfolio::{Action, BuySell};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{BarChart, Block, Borders, Paragraph, Tabs, Text, Widget};
use tui::Terminal;

mod portfolio;
mod tuiutil;

use crate::tuiutil::event::{Event, Events};
use crate::tuiutil::TabsState;

struct TuiApp<'a> {
    tabs: TabsState<'a>,
}

fn get_actions_to_display(actions: &[Action]) -> Vec<Text> {
    let mut ret = Vec::new();
    for a in actions {
        match a.buysell {
            BuySell::Buy => ret.push(Text::styled(
                format!(
                    "{} {} {} -{:.2}$\n",
                    "BUY", a.amount, a.name, a.transaction_value
                ),
                Style::default().fg(Color::Red),
            )),
            BuySell::Sell => ret.push(Text::styled(
                format!(
                    "{} {} {} +{:.2}$\n",
                    "SELL", a.amount, a.name, a.transaction_value
                ),
                Style::default().fg(Color::Green),
            )),
        }
    }
    ret
}

fn main() -> anyhow::Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("portfolio-file").required(true).index(1))
        .get_matches();
    let portfolio_file = matches.value_of("portfolio-file").unwrap();

    let load_res = portfolio::load_portfolio(portfolio_file);
    match load_res {
        Ok(mut _original_portfolio) => {
            // println!("Original {:?}", _original_portfolio);
            let mut _target_portfolio = if _original_portfolio.donotsell {
                _original_portfolio.add_without_selling()
            } else {
                _original_portfolio.rebalance()
            };
            // println!("Rebalanced {:?}", _target_portfolio);

            let _actions = _original_portfolio.get_actions(&_target_portfolio);
            // println!("Actions {:?}", _actions);
            let _display_actions = get_actions_to_display(&_actions);

            let _original_alloc_data: Vec<(&str, u64)> = _original_portfolio.get_display_data();
            let _target_alloc_data: Vec<(&str, u64)> = _target_portfolio.get_display_data();

            let stdout = io::stdout().into_raw_mode()?;
            let stdout = MouseTerminal::from(stdout);
            let stdout = AlternateScreen::from(stdout);
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
                terminal.draw(|mut f| {
                    let size = f.size();
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                        .split(size);
                    Tabs::default()
                        .block(Block::default().borders(Borders::ALL).title("Tabs"))
                        .titles(&app.tabs.titles)
                        .select(app.tabs.index)
                        .style(Style::default().fg(Color::Cyan))
                        .highlight_style(Style::default().fg(Color::Yellow))
                        .render(&mut f, chunks[0]);
                    let allocations_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(chunks[1]);
                    match app.tabs.index {
                        0 => {
                            BarChart::default()
                                .block(
                                    Block::default()
                                        .title("Original Allocation (%)")
                                        .borders(Borders::ALL),
                                )
                                .data(&_original_alloc_data)
                                .bar_width(5)
                                .bar_gap(3)
                                .style(Style::default().fg(Color::Green))
                                .value_style(
                                    Style::default().bg(Color::Green).modifier(Modifier::BOLD),
                                )
                                .render(&mut f, allocations_chunks[0]);
                            BarChart::default()
                                .block(
                                    Block::default()
                                        .title("New Allocation (%)")
                                        .borders(Borders::ALL),
                                )
                                .data(&_target_alloc_data)
                                .style(Style::default().fg(Color::Red))
                                .bar_width(5)
                                .bar_gap(3)
                                .value_style(Style::default().bg(Color::Red))
                                .label_style(
                                    Style::default().fg(Color::Cyan).modifier(Modifier::ITALIC),
                                )
                                .render(&mut f, allocations_chunks[1]);
                        }
                        1 => {
                            let block = Block::default()
                                .borders(Borders::ALL)
                                .title_style(Style::default().modifier(Modifier::BOLD));
                            Paragraph::new(_display_actions.iter())
                                .block(block)
                                .alignment(Alignment::Left)
                                .render(&mut f, chunks[1]);
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
            eprintln!("{:?}", e);
        }
    }

    Ok(())
}
