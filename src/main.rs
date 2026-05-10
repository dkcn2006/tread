use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use ratatui::{backend::CrosstermBackend, Terminal, Viewport};
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
    file: Option<String>,

    /// Display mode: log, minimal, comment
    #[arg(short, long, value_enum, default_value = "log")]
    mode: DisplayMode,

    /// Number of display lines (1-3)
    #[arg(short, long, default_value = "1")]
    lines: usize,

    /// Show recent reading list
    #[arg(long)]
    recent: bool,

    /// Custom template string, e.g. "[{time}] {level} {text}"
    #[arg(long)]
    template: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.recent {
        return show_recent_list();
    }

    let file = args
        .file
        .ok_or("请指定文件路径，或使用 --recent 查看最近阅读列表")?;
    let book = Book::load(&file)?;
    let mut app = App::new(book, args.mode, args.lines);
    app.custom_template = args.template;

    enable_raw_mode()?;
    let stdout = io::stdout();

    // Panic hook: ensure terminal is restored even if app panics
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Clear(ClearType::All));
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        ratatui::TerminalOptions {
            viewport: Viewport::Inline(app.display_lines as u16),
        },
    )?;

    let boss_key = app.run(&mut terminal).is_ok_and(|_| app.boss_key);

    // Restore terminal
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    if boss_key {
        execute!(stdout, Clear(ClearType::All))?;
        print!("\r\n");
    } else {
        println!();
    }

    Ok(())
}

fn show_recent_list() -> Result<(), Box<dyn std::error::Error>> {
    use crate::bookmark::Bookmarks;
    use std::io::{self, Write};

    let bookmarks = Bookmarks::load();
    let recent = bookmarks.recent(10);

    if recent.is_empty() {
        println!("最近阅读列表为空");
        return Ok(());
    }

    println!("最近阅读：");
    for (i, (path, entry)) in recent.iter().enumerate() {
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        println!("  {}. {}  [第 {} 行]", i + 1, name, entry.line_index);
    }
    print!("\n输入序号打开 (直接回车取消): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(());
    }

    if let Ok(idx) = input.parse::<usize>() {
        if let Some((path, _)) = recent.get(idx.saturating_sub(1)) {
            let book = Book::load(path)?;
            let mut app = App::new(book, DisplayMode::Log, 1);

            enable_raw_mode()?;
            let stdout = io::stdout();

            let original_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), Clear(ClearType::All));
                original_hook(info);
            }));

            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::with_options(
                backend,
                ratatui::TerminalOptions {
                    viewport: Viewport::Inline(app.display_lines as u16),
                },
            )?;

            let boss_key = app.run(&mut terminal).is_ok_and(|_| app.boss_key);

            disable_raw_mode()?;
            let mut stdout = io::stdout();
            if boss_key {
                execute!(stdout, Clear(ClearType::All))?;
                print!("\r\n");
            } else {
                println!();
            }
        }
    }

    Ok(())
}
