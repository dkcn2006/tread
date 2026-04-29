use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub line_index: usize,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub lines: Vec<String>,
    pub lowercase_lines: Vec<String>,
    pub chapters: Vec<Chapter>,
    pub file_path: String,
}

static CHAPTER_PATTERNS: LazyLock<[Regex; 3]> = LazyLock::new(|| [
    Regex::new(r"^[\s]*第[\s]*[一二三四五六七八九十零百千万亿\d]+[\s]*[章回节卷篇][\s]*.*$").unwrap(),
    Regex::new(r"^(?i)[\s]*chapter[\s]*\d+.*$").unwrap(),
    Regex::new(r"^[\s]*[卷篇][\s]*[一二三四五六七八九十零百千万亿\d]+[\s]*.*$").unwrap(),
]);

impl Book {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let canonical = std::fs::canonicalize(&path)?;
        let file_path = canonical.to_string_lossy().to_string();
        let raw = std::fs::read(&path)?;
        let text = decode_text(&raw);
        let lines: Vec<String> = text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.is_empty() {
            return Err("文件内容为空或仅包含空白行".into());
        }
        let lowercase_lines: Vec<String> = lines.iter().map(|s| s.to_lowercase()).collect();
        let chapters = parse_chapters(&lines);
        Ok(Book {
            lines,
            lowercase_lines,
            chapters,
            file_path,
        })
    }

    pub fn wrap_line(line: &str, max_width: usize) -> Vec<String> {
        if line.is_empty() {
            return vec![String::new()];
        }
        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_width = 0usize;
        for ch in line.chars() {
            let w = ch.width().unwrap_or(0);
            if current_width + w > max_width && !current.is_empty() {
                result.push(current);
                current = String::new();
                current_width = 0;
            }
            current.push(ch);
            current_width += w;
        }
        if !current.is_empty() {
            result.push(current);
        }
        if result.is_empty() {
            result.push(String::new());
        }
        result
    }

    pub fn get_display_lines(
        &self,
        start: usize,
        sub_offset: usize,
        count: usize,
        max_width: usize,
    ) -> (Vec<String>, usize, usize) {
        let mut lines = Vec::new();
        let mut line_idx = start;
        let mut sub_idx = sub_offset;

        while lines.len() < count {
            if line_idx >= self.lines.len() {
                break;
            }
            let wrapped = Self::wrap_line(&self.lines[line_idx], max_width);
            if sub_idx < wrapped.len() {
                lines.push(wrapped[sub_idx].clone());
                sub_idx += 1;
            } else {
                line_idx += 1;
                sub_idx = 0;
            }
        }

        (lines, line_idx, sub_idx)
    }

    pub fn search_forward(&self, start: usize, query: &str) -> Option<usize> {
        let q = query.to_lowercase();
        let len = self.lines.len();
        if len == 0 {
            return None;
        }
        for offset in 1..=len {
            let idx = (start + offset) % len;
            if self.lowercase_lines[idx].contains(&q) {
                return Some(idx);
            }
        }
        None
    }
}

fn decode_text(raw: &[u8]) -> String {
    // UTF-8 validation
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string();
    }

    // BOM detection
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        if let Ok(s) = std::str::from_utf8(&raw[3..]) {
            return s.to_string();
        }
    }

    let encodings = [
        encoding_rs::GBK,
        encoding_rs::GB18030,
        encoding_rs::BIG5,
        encoding_rs::EUC_JP,
        encoding_rs::EUC_KR,
    ];

    for encoding in &encodings {
        let (cow, had_errors) = encoding.decode_without_bom_handling(raw);
        if !had_errors {
            return cow.to_string();
        }
    }

    // Final fallback: lossy UTF-8
    String::from_utf8_lossy(raw).to_string()
}

fn parse_chapters(lines: &[String]) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let display_width: usize = line.chars().map(|c| c.width().unwrap_or(0)).sum();
        if display_width > 60 {
            continue;
        }
        for pat in CHAPTER_PATTERNS.iter() {
            if pat.is_match(line) {
                chapters.push(Chapter {
                    title: line.clone(),
                    line_index: i,
                });
                break;
            }
        }
    }
    chapters
}
