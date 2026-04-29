use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    Viewport,
};
use std::io;

use crate::app::App;
use crate::config::DisplayMode;
use crate::reader::Book;

mod app;
mod bookmark;
mod config;
mod reader;
mod ui;

#[derive(Parser, Debug)]
#[command(name = "tread")]
#[command(about = "A stealthy terminal TUI novel reader")]
struct Args {
    /// Path to the text file
    file: String,

    /// Display mode: log, minimal, comment
    #[arg(short, long, value_enum, default_value = "log")]
    mode: DisplayMode,

    /// Number of display lines (1-3)
    #[arg(short, long, default_value = "1")]
    lines: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let book = Book::load(&args.file)?;
    let mut app = App::new(book, args.mode, args.lines);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        ratatui::TerminalOptions {
            viewport: Viewport::Inline(app.display_lines as u16),
        },
    )?;

    app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    if app.boss_key {
        execute!(stdout, DisableMouseCapture, Clear(ClearType::All))?;
        print!("\r\n");
    } else {
        execute!(stdout, DisableMouseCapture)?;
        println!();
    }

    Ok(())
}
