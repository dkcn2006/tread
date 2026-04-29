use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    Terminal,
};
use std::time::Duration;

use crate::bookmark::{BookmarkEntry, Bookmarks};
use crate::config::DisplayMode;
use crate::reader::Book;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    ChapterList,
}

pub struct App {
    pub book: Book,
    bookmarks: Bookmarks,

    pub current_line: usize,
    pub sub_offset: usize,
    pub display_lines: usize,
    pub terminal_width: u16,
    pub mode: DisplayMode,

    pub input_mode: InputMode,
    pub search_input: String,
    pub last_search: String,
    pub chapter_cursor: usize,

    pub should_quit: bool,
    pub boss_key: bool,
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
            current_mode = DisplayMode::from_index(entry.mode);
        }

        App {
            book,
            bookmarks,
            current_line,
            sub_offset,
            display_lines: display_lines.max(1).min(3),
            terminal_width: 80,
            mode: current_mode,
            input_mode: InputMode::Normal,
            search_input: String::new(),
            last_search: String::new(),
            chapter_cursor: 0,
            should_quit: false,
            boss_key: false,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
        loop {
            terminal
                .draw(|frame| {
                    ui::render(frame, self);
                })
                .unwrap();

            if event::poll(Duration::from_millis(250)).unwrap() {
                if let Ok(Event::Key(key)) = event::read() {
                    match self.input_mode {
                        InputMode::Normal => self.handle_normal_key(key),
                        InputMode::Search => self.handle_search_key(key),
                        InputMode::ChapterList => self.handle_chapter_key(key),
                    }
                }
            }

            if self.should_quit || self.boss_key {
                self.save_bookmark();
                break;
            }
        }
    }

    fn save_bookmark(&mut self) {
        self.bookmarks.set(
            &self.book.file_path,
            BookmarkEntry {
                line_index: self.current_line,
                sub_offset: self.sub_offset,
                mode: self.mode.to_index(),
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
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => {
                self.next_lines(1, content_width)
            }
            KeyCode::Char('k') | KeyCode::Up => self.prev_lines(1, content_width),
            KeyCode::Char(' ') => self.next_lines(self.display_lines, content_width),
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
            KeyCode::Char('n') => {
                if !self.last_search.is_empty() {
                    if let Some(idx) = self.book.search_forward(self.current_line, &self.last_search)
                    {
                        self.current_line = idx;
                        self.sub_offset = 0;
                    }
                }
            }
            KeyCode::Char('g') => {
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
                    if let Some(idx) =
                        self.book.search_forward(self.current_line, &self.search_input)
                    {
                        self.current_line = idx;
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

    fn handle_chapter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.chapter_cursor + 1 < self.book.chapters.len() {
                    self.chapter_cursor += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.chapter_cursor > 0 {
                    self.chapter_cursor -= 1;
                }
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
        match self.mode {
            DisplayMode::Log => width.saturating_sub(29).max(1),
            DisplayMode::Minimal => width.saturating_sub(20).max(1),
            DisplayMode::Comment => width.saturating_sub(20).max(1),
        }
    }
}
