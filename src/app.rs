use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

use crate::bookmark::{BookmarkEntry, Bookmarks};
use crate::config::DisplayMode;
use crate::reader::Book;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    ChapterList,
    GotoPercent,
    FavoriteList,
}

pub struct App {
    pub book: Book,
    pub bookmarks: Bookmarks,

    pub current_line: usize,
    pub sub_offset: usize,
    pub display_lines: usize,
    pub terminal_width: u16,
    pub mode: DisplayMode,

    pub input_mode: InputMode,
    pub search_input: String,
    pub last_search: String,
    pub search_highlight: Option<String>,
    pub goto_input: String,
    pub chapter_cursor: usize,
    pub favorite_cursor: usize,

    pub should_quit: bool,
    pub boss_key: bool,
    pub hidden: bool,

    pub status_message: Option<String>,
    pub status_until: Option<Instant>,

    pub custom_template: Option<String>,
}

impl App {
    pub fn new(book: Book, mode: DisplayMode, display_lines: usize) -> Self {
        let bookmarks = Bookmarks::load();
        let mut current_line = 0usize;
        let mut sub_offset = 0usize;
        let mut current_mode = mode;

        if let Some(entry) = bookmarks.get(&book.file_path) {
            current_line = entry.line_index.min(book.lines.len().saturating_sub(1));
            sub_offset = entry.sub_offset;
            current_mode = entry.mode;
        }

        App {
            book,
            bookmarks,
            current_line,
            sub_offset,
            display_lines: display_lines.clamp(1, 3),
            terminal_width: 80,
            mode: current_mode,
            input_mode: InputMode::Normal,
            search_input: String::new(),
            last_search: String::new(),
            search_highlight: None,
            goto_input: String::new(),
            chapter_cursor: 0,
            favorite_cursor: 0,
            should_quit: false,
            boss_key: false,
            hidden: false,
            status_message: None,
            status_until: None,
            custom_template: None,
        }
    }

    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.clear_status_if_expired();
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            if event::poll(Duration::from_millis(250))? {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key) => {
                            if self.hidden {
                                self.hidden = false;
                                continue;
                            }
                            match self.input_mode {
                                InputMode::Normal => self.handle_normal_key(key),
                                InputMode::Search => self.handle_search_key(key),
                                InputMode::ChapterList => self.handle_chapter_key(key),
                                InputMode::GotoPercent => self.handle_goto_key(key),
                                InputMode::FavoriteList => self.handle_favorite_key(key),
                            }
                        }
                        Event::Resize(w, _) => {
                            self.terminal_width = w;
                            self.sub_offset = 0;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit || self.boss_key {
                self.save_bookmark();
                break;
            }
        }
        Ok(())
    }

    fn clear_status_if_expired(&mut self) {
        if let Some(until) = self.status_until {
            if Instant::now() >= until {
                self.status_message = None;
                self.status_until = None;
            }
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, duration_secs: u64) {
        self.status_message = Some(msg.into());
        self.status_until = Some(Instant::now() + Duration::from_secs(duration_secs));
    }

    fn save_bookmark(&mut self) {
        self.bookmarks.set(
            &self.book.file_path,
            BookmarkEntry {
                line_index: self.current_line,
                sub_offset: self.sub_offset,
                mode: self.mode,
                last_accessed: None, // set() will fill in current timestamp
            },
        );
        self.bookmarks.save();
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        let content_width = self.estimate_content_width();
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Esc => self.boss_key = true,
            KeyCode::Char('h') => self.hidden = true,
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => {
                self.next_lines(1, content_width)
            }
            KeyCode::Char('k') | KeyCode::Up => self.prev_lines(1, content_width),
            KeyCode::Char(' ') | KeyCode::PageDown => {
                self.next_lines(self.display_lines, content_width)
            }
            KeyCode::Char('b') | KeyCode::PageUp => {
                self.prev_lines(self.display_lines, content_width)
            }
            KeyCode::Home => {
                self.current_line = 0;
                self.sub_offset = 0;
            }
            KeyCode::End => {
                self.current_line = self.book.lines.len().saturating_sub(1);
                self.sub_offset = 0;
            }
            KeyCode::Char('t') => self.mode = self.mode.next(),
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_input.clear();
            }
            KeyCode::Char('p') | KeyCode::Char(':') => {
                self.input_mode = InputMode::GotoPercent;
                self.goto_input.clear();
            }
            KeyCode::Char('n') if !self.last_search.is_empty() => {
                if let Some(idx) = self
                    .book
                    .search_forward(self.current_line, &self.last_search)
                {
                    self.current_line = idx;
                    self.sub_offset = 0;
                    self.search_highlight = Some(self.last_search.clone());
                } else {
                    self.set_status(format!("未找到: {}", self.last_search), 2);
                }
            }
            KeyCode::Char('N') if !self.last_search.is_empty() => {
                if let Some(idx) = self
                    .book
                    .search_backward(self.current_line, &self.last_search)
                {
                    self.current_line = idx;
                    self.sub_offset = 0;
                    self.search_highlight = Some(self.last_search.clone());
                } else {
                    self.set_status(format!("未找到: {}", self.last_search), 2);
                }
            }
            KeyCode::Char('g') => {
                if self.book.chapters.is_empty() {
                    self.set_status("无章节", 2);
                } else {
                    self.input_mode = InputMode::ChapterList;
                    self.chapter_cursor = self
                        .book
                        .chapters
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, ch)| ch.line_index <= self.current_line)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            KeyCode::Char('m') => {
                let added = self
                    .bookmarks
                    .toggle_favorite(&self.book.file_path, self.current_line);
                self.set_status(
                    if added {
                        "已收藏"
                    } else {
                        "已取消收藏"
                    },
                    2,
                );
            }
            KeyCode::Char('M') => {
                let favs = self.bookmarks.favorites(&self.book.file_path);
                if favs.is_empty() {
                    self.set_status("无收藏", 2);
                } else {
                    self.input_mode = InputMode::FavoriteList;
                    self.favorite_cursor = favs
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, &line)| line <= self.current_line)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.search_input.push(c),
            KeyCode::Backspace => {
                self.search_input.pop();
            }
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    self.last_search = self.search_input.clone();
                    if let Some(idx) = self
                        .book
                        .search_forward(self.current_line, &self.search_input)
                    {
                        self.current_line = idx;
                        self.sub_offset = 0;
                        self.search_highlight = Some(self.last_search.clone());
                    } else {
                        self.set_status(format!("未找到: {}", self.search_input), 2);
                    }
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_chapter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down
                if self.chapter_cursor + 1 < self.book.chapters.len() =>
            {
                self.chapter_cursor += 1;
            }
            KeyCode::Char('k') | KeyCode::Up if self.chapter_cursor > 0 => {
                self.chapter_cursor -= 1;
            }
            KeyCode::Enter => {
                if let Some(ch) = self.book.chapters.get(self.chapter_cursor) {
                    self.current_line = ch.line_index;
                    self.sub_offset = 0;
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('g') => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_goto_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => self.goto_input.push(c),
            KeyCode::Backspace => {
                self.goto_input.pop();
            }
            KeyCode::Enter => {
                if let Ok(percent) = self.goto_input.parse::<usize>() {
                    let pct = percent.min(100);
                    let total = self.book.lines.len();
                    if total > 0 {
                        self.current_line = ((pct * total) / 100).min(total - 1);
                        self.sub_offset = 0;
                    }
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_favorite_key(&mut self, key: KeyEvent) {
        let favs = self.bookmarks.favorites(&self.book.file_path);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down if self.favorite_cursor + 1 < favs.len() => {
                self.favorite_cursor += 1;
            }
            KeyCode::Char('k') | KeyCode::Up if self.favorite_cursor > 0 => {
                self.favorite_cursor -= 1;
            }
            KeyCode::Enter => {
                if let Some(&line) = favs.get(self.favorite_cursor) {
                    self.current_line = line;
                    self.sub_offset = 0;
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('M') => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn next_lines(&mut self, n: usize, content_width: usize) {
        for _ in 0..n {
            self.next_display_line(content_width);
        }
    }

    fn prev_lines(&mut self, n: usize, content_width: usize) {
        for _ in 0..n {
            self.prev_display_line(content_width);
        }
    }

    fn next_display_line(&mut self, content_width: usize) {
        if self.current_line >= self.book.lines.len() {
            return;
        }
        let wrapped = Book::wrap_line(&self.book.lines[self.current_line], content_width);
        if self.sub_offset + 1 < wrapped.len() {
            self.sub_offset += 1;
        } else if self.current_line + 1 < self.book.lines.len() {
            self.current_line += 1;
            self.sub_offset = 0;
        }
    }

    fn prev_display_line(&mut self, content_width: usize) {
        if self.sub_offset > 0 {
            self.sub_offset -= 1;
        } else if self.current_line > 0 {
            self.current_line -= 1;
            let wrapped = Book::wrap_line(&self.book.lines[self.current_line], content_width);
            self.sub_offset = wrapped.len().saturating_sub(1);
        }
    }

    fn estimate_content_width(&self) -> usize {
        let width = self.terminal_width as usize;
        if let Some(ref tmpl) = self.custom_template {
            let prefix = tmpl.split("{text}").next().unwrap_or(tmpl);
            let suffix = tmpl.split("{text}").nth(1).unwrap_or("");
            let prefix_width = prefix.width();
            let suffix_width = suffix.width();
            return width.saturating_sub(prefix_width + suffix_width).max(1);
        }
        match self.mode {
            DisplayMode::Log | DisplayMode::DockerLogs | DisplayMode::KubectlLogs => {
                let prefix_width = 29usize; // [YYYY-MM-DD HH:MM:SS] LEVEL
                width.saturating_sub(prefix_width).max(1)
            }
            DisplayMode::Minimal | DisplayMode::NpmInstall | DisplayMode::Pytest => {
                let progress_text =
                    format!("[{:5}/{:5}]", self.current_line, self.book.lines.len());
                let progress_width = progress_text.width();
                width.saturating_sub(progress_width).max(1)
            }
            DisplayMode::Comment => {
                let prefix = "// ";
                let prefix_width = prefix.width();
                let total_lines = self.book.lines.len();
                let progress_pct = if total_lines > 0 {
                    (self.current_line as f64 / total_lines as f64) * 100.0
                } else {
                    0.0
                };
                let current_chapter = self
                    .book
                    .chapters
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, ch)| ch.line_index <= self.current_line)
                    .map(|(i, _)| i + 1)
                    .unwrap_or(0);
                let suffix = format!(" [Ch.{} | {:.1}%]", current_chapter, progress_pct);
                let suffix_width = suffix.width();
                width.saturating_sub(prefix_width + suffix_width).max(1)
            }
            DisplayMode::GitLog => {
                let prefix_width = 14usize; // "abc1234 | "
                width.saturating_sub(prefix_width).max(1)
            }
        }
    }
}
