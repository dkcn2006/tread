use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};
use crate::config::DisplayMode;
use unicode_width::UnicodeWidthStr;
use chrono::Local;

pub fn render(frame: &mut Frame, app: &mut App) {
    app.terminal_width = frame.area().width;

    match app.input_mode {
        InputMode::ChapterList => render_chapter_list(frame, app),
        _ => {
            match app.mode {
                DisplayMode::Log => render_log(frame, app),
                DisplayMode::Minimal => render_minimal(frame, app),
                DisplayMode::Comment => render_comment(frame, app),
            }
            if app.input_mode == InputMode::Search {
                render_search_overlay(frame, app);
            }
        }
    }
}

fn render_log(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = 29usize; // [YYYY-MM-DD HH:MM:SS] LEVEL 
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut now = Local::now();
    let levels = ["INFO", "DEBUG", "TRACE", "WARN"];
    let level_colors = [Color::Green, Color::Cyan, Color::Gray, Color::Yellow];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let level_idx = (app.current_line + i) % 4;
            let level = levels[level_idx];
            let level_color = level_colors[level_idx];

            now = now + chrono::Duration::seconds(1);

            Line::from(vec![
                Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:5} ", level), Style::default().fg(level_color)),
                Span::raw(content),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(text_lines);
    frame.render_widget(paragraph, area);
}

fn render_minimal(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;

    let progress_text = format!(" [{}/{}]", app.current_line, app.book.lines.len());
    let progress_width = progress_text.width();
    let content_width = width.saturating_sub(progress_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut text_lines: Vec<Line> = lines
        .into_iter()
        .map(|s| Line::from(Span::raw(s)))
        .collect();

    if let Some(last) = text_lines.last_mut() {
        last.spans.push(Span::styled(
            progress_text,
            Style::default().fg(Color::DarkGray),
        ));
    }

    let paragraph = Paragraph::new(text_lines);
    frame.render_widget(paragraph, area);
}

fn render_comment(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix = "// ";
    let prefix_width = prefix.width();

    let total_lines = app.book.lines.len();
    let progress_pct = if total_lines > 0 {
        (app.current_line as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };

    let current_chapter = app
        .book
        .chapters
        .iter()
        .enumerate()
        .rev()
        .find(|(_, ch)| ch.line_index <= app.current_line)
        .map(|(i, _)| i + 1)
        .unwrap_or(0);

    let suffix = format!(" [Ch.{} | {:.1}%]", current_chapter, progress_pct);
    let suffix_width = suffix.width();
    let content_width = width.saturating_sub(prefix_width + suffix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut text_lines: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            let mut spans = vec![Span::styled(prefix, Style::default().fg(Color::DarkGray))];
            spans.push(Span::raw(content));
            if i == display_lines.saturating_sub(1) {
                spans.push(Span::styled(suffix.clone(), Style::default().fg(Color::DarkGray)));
            }
            Line::from(spans)
        })
        .collect();

    // Pad empty lines to maintain comment block consistency
    while text_lines.len() < display_lines {
        text_lines.push(Line::from(Span::styled(
            "//",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(text_lines);
    frame.render_widget(paragraph, area);
}

fn render_search_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let cursor = "_";

    let spans = vec![
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(&app.search_input),
        Span::styled(cursor, Style::default().fg(Color::Yellow)),
    ];

    let search_widget = Paragraph::new(Line::from(spans));
    let search_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    frame.render_widget(Clear, search_area);
    frame.render_widget(search_widget, search_area);
}

fn render_chapter_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let visible_count = area.height as usize;
    let total = app.book.chapters.len();
    let start = app.chapter_cursor.saturating_sub(visible_count.saturating_sub(1) / 2);
    let end = (start + visible_count).min(total);
    let start = end.saturating_sub(visible_count); // adjust if end clipped

    let items: Vec<ListItem> = app.book.chapters[start..end]
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let actual_idx = start + i;
            let prefix = if actual_idx == app.chapter_cursor { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(
                    prefix,
                    if actual_idx == app.chapter_cursor {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(&ch.title),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
