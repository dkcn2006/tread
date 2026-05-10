use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};
use crate::config::DisplayMode;
use chrono::Local;
use rand::seq::SliceRandom;
use rand::Rng;
use unicode_width::UnicodeWidthStr;

fn highlight_spans(text: &str, highlight: Option<&str>) -> Vec<Span<'static>> {
    let hl = match highlight {
        Some(h) if !h.is_empty() => h,
        _ => return vec![Span::raw(text.to_string())],
    };
    let lower_text = text.to_lowercase();
    let lower_hl = hl.to_lowercase();
    let mut spans = Vec::new();
    let mut last_end = 0usize;
    for (start, part) in lower_text.match_indices(&lower_hl) {
        if start > last_end {
            spans.push(Span::raw(text[last_end..start].to_string()));
        }
        let end = start + part.len();
        spans.push(Span::styled(
            text[start..end].to_string(),
            Style::default().fg(Color::Yellow),
        ));
        last_end = end;
    }
    if last_end < text.len() {
        spans.push(Span::raw(text[last_end..].to_string()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

pub fn render(frame: &mut Frame, app: &mut App) {
    app.terminal_width = frame.area().width;

    if app.hidden {
        render_hidden(frame);
        return;
    }

    match app.input_mode {
        InputMode::ChapterList => render_chapter_list(frame, app),
        InputMode::FavoriteList => render_favorite_list(frame, app),
        _ => {
            if let Some(ref tmpl) = app.custom_template {
                render_custom_template(frame, app, tmpl);
            } else {
                match app.mode {
                    DisplayMode::Log => render_log(frame, app),
                    DisplayMode::Minimal => render_minimal(frame, app),
                    DisplayMode::Comment => render_comment(frame, app),
                    DisplayMode::GitLog => render_git_log(frame, app),
                    DisplayMode::NpmInstall => render_npm_install(frame, app),
                    DisplayMode::Pytest => render_pytest(frame, app),
                    DisplayMode::DockerLogs => render_docker_logs(frame, app),
                    DisplayMode::KubectlLogs => render_kubectl_logs(frame, app),
                }
            }
            if app.input_mode == InputMode::Search {
                render_search_overlay(frame, app);
            } else if app.input_mode == InputMode::GotoPercent {
                render_goto_overlay(frame, app);
            } else if app.status_message.is_some() {
                render_status_overlay(frame, app);
            }
        }
    }
}

fn render_hidden(frame: &mut Frame) {
    let area = frame.area();
    let blank = Paragraph::new(vec![Line::from(""); area.height as usize]);
    frame.render_widget(blank, area);
}

fn render_log(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = 29usize;
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut now = Local::now();
    let mut rng = rand::thread_rng();
    let levels = ["INFO", "DEBUG", "TRACE", "WARN"];
    let level_colors = [Color::Green, Color::Cyan, Color::Gray, Color::Yellow];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let level_idx = rng.gen_range(0..4);
            let level = levels[level_idx];
            let level_color = level_colors[level_idx];
            now += chrono::Duration::seconds(1);

            let mut spans = vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{:5} ", level), Style::default().fg(level_color)),
            ];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
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
        .map(|s| Line::from(highlight_spans(&s, app.search_highlight.as_deref())))
        .collect();

    if let Some(last) = text_lines.last_mut() {
        last.spans.push(Span::styled(
            progress_text,
            Style::default().fg(Color::DarkGray),
        ));
    }

    frame.render_widget(Paragraph::new(text_lines), area);
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
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            if i == display_lines.saturating_sub(1) {
                spans.push(Span::styled(
                    suffix.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Line::from(spans)
        })
        .collect();

    while text_lines.len() < display_lines {
        text_lines.push(Line::from(Span::styled(
            "//",
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_git_log(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = 14usize; // "abc1234 | "
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut rng = rand::thread_rng();
    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let hash = format!("{:07x}", rng.gen::<u32>());
            let mut spans = vec![Span::styled(
                format!("{} | ", hash),
                Style::default().fg(Color::DarkGray),
            )];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_npm_install(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefixes = [
        ("npm info ", Color::Green),
        ("npm warn ", Color::Yellow),
        ("npm ERR ", Color::Red),
        ("npm ok ", Color::Cyan),
    ];

    let prefix_width = prefixes.iter().map(|(p, _)| p.width()).max().unwrap_or(9);
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let text_lines: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            let (pfx, color) = prefixes[i % prefixes.len()];
            let mut spans = vec![Span::styled(pfx.to_string(), Style::default().fg(color))];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_pytest(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = "test_read.py::test_1234 PASSED ".width();
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut rng = rand::thread_rng();
    let results = [("PASSED", Color::Green), ("FAILED", Color::Red)];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let test_name = format!("test_{:03}", rng.gen_range(0..1000));
            let (result, color) = results[rng.gen_range(0..2)];
            let mut spans = vec![
                Span::styled(
                    "test_app.py::".to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", test_name), Style::default()),
                Span::styled(format!("{} ", result), Style::default().fg(color)),
            ];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_docker_logs(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = 38usize; // "[2026-04-28 14:32:01] [app] INFO  "
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut now = Local::now();
    let mut rng = rand::thread_rng();
    let levels = ["INFO", "DEBUG", "TRACE", "WARN"];
    let level_colors = [Color::Green, Color::Cyan, Color::Gray, Color::Yellow];
    let modules = ["app", "db", "api", "worker", "cache"];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let level_idx = rng.gen_range(0..4);
            let level = levels[level_idx];
            let level_color = level_colors[level_idx];
            let module = modules.choose(&mut rng).unwrap();
            now += chrono::Duration::seconds(1);

            let mut spans = vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("[{}] ", module),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(format!("{:5} ", level), Style::default().fg(level_color)),
            ];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_kubectl_logs(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;
    let prefix_width = 42usize; // "2026-04-28T14:32:01Z INFO  [pod-xxx] "
    let content_width = width.saturating_sub(prefix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut now = Local::now();
    let mut rng = rand::thread_rng();
    let levels = ["INFO", "DEBUG", "TRACE", "WARN"];
    let level_colors = [Color::Green, Color::Cyan, Color::Gray, Color::Yellow];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let timestamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let level_idx = rng.gen_range(0..4);
            let level = levels[level_idx];
            let level_color = level_colors[level_idx];
            let pod = format!("pod-{:04x}", rng.gen::<u16>());
            now += chrono::Duration::seconds(1);

            let mut spans = vec![
                Span::styled(
                    format!("{} ", timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{:5} ", level), Style::default().fg(level_color)),
                Span::styled(format!("[{}] ", pod), Style::default().fg(Color::Magenta)),
            ];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_custom_template(frame: &mut Frame, app: &App, tmpl: &str) {
    let area = frame.area();
    let width = area.width as usize;
    let display_lines = app.display_lines;

    let parts: Vec<&str> = tmpl.split("{text}").collect();
    let prefix = parts.first().copied().unwrap_or("");
    let suffix = parts.get(1).copied().unwrap_or("");
    let prefix_width = prefix.width();
    let suffix_width = suffix.width();
    let content_width = width.saturating_sub(prefix_width + suffix_width);

    let (lines, _, _) = app.book.get_display_lines(
        app.current_line,
        app.sub_offset,
        display_lines,
        content_width.max(1),
    );

    let mut now = Local::now();
    let mut rng = rand::thread_rng();
    let levels = ["INFO", "DEBUG", "TRACE", "WARN"];
    let modules = ["app", "db", "api", "worker", "cache"];

    let text_lines: Vec<Line> = lines
        .into_iter()
        .map(|content| {
            let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let level = levels[rng.gen_range(0..4)];
            let module = modules.choose(&mut rng).unwrap();
            let trace_id = format!("{:08x}", rng.gen::<u32>());
            now += chrono::Duration::seconds(1);

            let expanded_prefix = prefix
                .replace("{time}", &timestamp)
                .replace("{level}", level)
                .replace("{module}", module)
                .replace("{trace_id}", &trace_id);
            let expanded_suffix = suffix
                .replace("{time}", &timestamp)
                .replace("{level}", level)
                .replace("{module}", module)
                .replace("{trace_id}", &trace_id);

            let mut spans = vec![Span::raw(expanded_prefix)];
            spans.extend(highlight_spans(&content, app.search_highlight.as_deref()));
            spans.push(Span::raw(expanded_suffix));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(text_lines), area);
}

fn render_status_overlay(frame: &mut Frame, app: &App) {
    if let Some(ref msg) = app.status_message {
        let area = frame.area();
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };
        let spans = vec![Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        )];
        frame.render_widget(Clear, status_area);
        frame.render_widget(Paragraph::new(Line::from(spans)), status_area);
    }
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

fn render_goto_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let cursor = "_";

    let spans = vec![
        Span::styled(":", Style::default().fg(Color::Yellow)),
        Span::raw(&app.goto_input),
        Span::styled(cursor, Style::default().fg(Color::Yellow)),
    ];

    let widget = Paragraph::new(Line::from(spans));
    let goto_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    frame.render_widget(Clear, goto_area);
    frame.render_widget(widget, goto_area);
}

fn render_list_overlay(
    frame: &mut Frame,
    items: impl Iterator<Item = (usize, String)>,
    cursor: usize,
    area: Rect,
) {
    let visible_count = area.height as usize;
    let total = items.size_hint().1.unwrap_or(0);
    let start = cursor.saturating_sub(visible_count.saturating_sub(1) / 2);
    let end = (start + visible_count).min(total);
    let start = end.saturating_sub(visible_count);

    let list_items: Vec<ListItem> = items
        .skip(start)
        .take(end - start)
        .map(|(actual_idx, text)| {
            let prefix = if actual_idx == cursor { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(
                    prefix,
                    if actual_idx == cursor {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(text),
            ]))
        })
        .collect();

    let list = List::new(list_items);
    frame.render_widget(list, area);
}

fn render_chapter_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let items = app
        .book
        .chapters
        .iter()
        .enumerate()
        .map(|(i, ch)| (i, ch.title.clone()));
    render_list_overlay(frame, items, app.chapter_cursor, area);
}

fn render_favorite_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let favs = app.bookmarks.favorites(&app.book.file_path);
    let items = favs.iter().enumerate().map(|(i, &line)| {
        let text = app.book.lines.get(line).cloned().unwrap_or_default();
        let label = if text.len() > 40 {
            format!("{}...", &text[..40])
        } else {
            text
        };
        (i, format!("第{}行: {}", line + 1, label))
    });
    render_list_overlay(frame, items, app.favorite_cursor, area);
}
