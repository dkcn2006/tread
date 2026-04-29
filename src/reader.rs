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

/// 简单 HTML → 纯文本转换（用于 mobi 内容清理）
static HTML_BR_RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"<br\s*/?>").unwrap()
);
static HTML_P_CLOSE_RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"</p>").unwrap()
);
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"</?[^>]+>").unwrap()
);

impl Book {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let canonical = std::fs::canonicalize(&path)?;
        let file_path = canonical.to_string_lossy().to_string();

        let ext = canonical.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let text = match ext.as_str() {
            "mobi" | "azw" | "azw3" => load_mobi(&path)?,
            "pdf" => load_pdf(&path)?,
            _ => {
                let raw = std::fs::read(&path)?;
                decode_text(&raw)
            }
        };

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

/// 解析 mobi/azw/azw3 文件，返回纯文本
fn load_mobi<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let m = mobi::Mobi::from_path(&path)?;
    let content = m.content_as_string();
    Ok(html_to_text(&content))
}

/// 解析 PDF 文件，返回纯文本
fn load_pdf<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let text = pdf_extract::extract_text(&path)?;
    Ok(clean_pdf_text(&text))
}

/// 清理 PDF 提取的文本：合并连续空行、去掉页码标记等
fn clean_pdf_text(text: &str) -> String {
    let mut result = String::new();
    let mut prev_blank = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // 跳过纯数字行（常见页码）
        if trimmed.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // 跳过常见的页眉页脚标记
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("www.")
        {
            continue;
        }

        if trimmed.is_empty() {
            if !prev_blank {
                result.push('\n');
                prev_blank = true;
            }
        } else {
            if !result.is_empty() && !prev_blank {
                result.push('\n');
            }
            result.push_str(trimmed);
            prev_blank = false;
        }
    }

    result
}

/// 简单但有效的 HTML → 纯文本转换
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // 1. <br>, <br/> → 换行
    text = HTML_BR_RE.replace_all(&text, "\n").to_string();

    // 2. </p> → 换行（段落分隔）
    text = HTML_P_CLOSE_RE.replace_all(&text, "\n").to_string();

    // 3. 去掉其余所有 HTML 标签
    text = HTML_TAG_RE.replace_all(&text, "").to_string();

    // 4. 解码常见 HTML 实体
    text = text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    // 5. 合并连续空行，保留单空行用于段落分隔
    let mut result = String::new();
    let mut prev_blank = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_blank {
                result.push('\n');
                prev_blank = true;
            }
        } else {
            if !result.is_empty() && !prev_blank {
                result.push('\n');
            }
            result.push_str(trimmed);
            prev_blank = false;
        }
    }

    result
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
