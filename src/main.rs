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
use crate::settings::Settings;

mod app;
mod bookmark;
mod config;
mod reader;
mod settings;
mod ui;

#[derive(Parser, Debug)]
#[command(name = "tread")]
#[command(about = "A stealthy terminal TUI novel reader")]
struct Args {
    /// Path to the text file
    file: Option<String>,

    /// Display mode: log, minimal, comment
    #[arg(short, long, value_enum)]
    mode: Option<DisplayMode>,

    /// Number of display lines (1-3)
    #[arg(short, long)]
    lines: Option<usize>,

    /// Show recent reading list, or open the N-th recent file directly
    #[arg(long, num_args = 0..=1, default_missing_value = "0")]
    recent: Option<usize>,

    /// Forget/remove a file from bookmarks and recent list
    #[arg(long)]
    forget: Option<String>,

    /// Custom template string, e.g. "[{time}] {level} {text}"
    #[arg(long)]
    template: Option<String>,

    /// Use a named template preset from config
    #[arg(long)]
    template_name: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let settings = Settings::load();

    if let Some(path) = args.forget {
        return forget_bookmark(&path);
    }

    if let Some(n) = args.recent {
        return show_recent_list(n, &settings);
    }

    let file = args
        .file
        .ok_or("请指定文件路径，或使用 --recent 查看最近阅读列表")?;

    let mode = args
        .mode
        .or_else(|| settings.default_mode.as_ref().and_then(|m| m.parse().ok()))
        .unwrap_or(DisplayMode::Log);

    let lines = args.lines.or(settings.display_lines).unwrap_or(1);

    let template = if let Some(name) = args.template_name {
        settings
            .find_template(&name)
            .map(|t| t.to_string())
            .or_else(|| {
                eprintln!("错误: 配置中未找到模板预设 '{}'", name);
                std::process::exit(1);
            })
    } else {
        args.template
    };

    if let Some(ref tmpl) = template {
        if !tmpl.contains("{text}") {
            eprintln!("错误: --template 必须包含 {{text}} 占位符，否则正文无法显示");
            std::process::exit(1);
        }
    }

    let book = Book::load(&file)?;
    let mut app = App::new(book, mode, lines);
    app.custom_template = template;
    run_app(app)
}

fn run_app(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
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

fn forget_bookmark(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::bookmark::Bookmarks;
    use std::path::Path;

    let mut bookmarks = Bookmarks::load();
    let canonical = std::fs::canonicalize(path)?;
    let key = canonical.to_string_lossy().to_string();
    bookmarks.remove(&key);
    bookmarks.save();
    println!(
        "已从书签中移除: {}",
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
    );
    Ok(())
}

fn show_recent_list(n: usize, _settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    use crate::bookmark::Bookmarks;
    use std::io::{self, Write};

    let bookmarks = Bookmarks::load();
    let recent = bookmarks.recent(10);

    if recent.is_empty() {
        println!("最近阅读列表为空");
        return Ok(());
    }

    // 如果直接传了序号，跳过交互
    if n > 0 {
        if let Some((path, _)) = recent.get(n.saturating_sub(1)) {
            let book = Book::load(path)?;
            let app = App::new(book, DisplayMode::Log, 1);
            return run_app(app);
        } else {
            eprintln!("序号 {} 超出范围 (1-{})", n, recent.len());
            std::process::exit(1);
        }
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
            let app = App::new(book, DisplayMode::Log, 1);
            return run_app(app);
        }
    }

    Ok(())
}
