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

static CHAPTER_PATTERNS: LazyLock<[Regex; 4]> = LazyLock::new(|| {
    [
        Regex::new(r"^[\s]*第[\s]*[一二三四五六七八九十零百千万亿\d]+[\s]*[章回节卷篇][\s]*.*$")
            .unwrap(),
        Regex::new(r"^(?i)[\s]*chapter[\s]*\d+.*$").unwrap(),
        Regex::new(r"^[\s]*[卷篇][\s]*[一二三四五六七八九十零百千万亿\d]+[\s]*.*$").unwrap(),
        Regex::new(
            r"^[\s]*(楔子|序章|序[言篇]|前言|引子|引言|正文|番外|后记|尾声|后序|跋)[\s]*.*$",
        )
        .unwrap(),
    ]
});

/// 简单 HTML → 纯文本转换（用于 mobi 内容清理）
static HTML_BR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<br\s*/?>").unwrap());
static HTML_P_CLOSE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</p>").unwrap());
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</?[^>]+>").unwrap());

impl Book {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let canonical = std::fs::canonicalize(&path)?;
        let file_path = canonical.to_string_lossy().to_string();

        let ext = canonical
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let text = match ext.as_str() {
            "mobi" | "azw" | "azw3" => load_mobi(&path)?,
            "pdf" => load_pdf(&path)?,
            "epub" => load_epub(&path)?,
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

    pub fn search_backward(&self, start: usize, query: &str) -> Option<usize> {
        let q = query.to_lowercase();
        let len = self.lines.len();
        if len == 0 {
            return None;
        }
        for offset in 1..=len {
            let idx = (start + len - offset) % len;
            if self.lowercase_lines[idx].contains(&q) {
                return Some(idx);
            }
        }
        None
    }
}

/// 解析 EPUB 文件，返回纯文本
fn load_epub<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let mut doc = epub::doc::EpubDoc::new(&path)?;
    let mut full_text = String::new();

    let spine = doc.spine.clone();
    for idref in spine.iter() {
        if let Ok(content) = doc.get_resource(idref) {
            let html = decode_text(&content);
            let text = html_to_text(&html);
            if !text.is_empty() {
                if !full_text.is_empty() {
                    full_text.push('\n');
                }
                full_text.push_str(&text);
            }
        }
    }

    if full_text.is_empty() {
        return Err("EPUB 文件内容为空".into());
    }

    Ok(full_text)
}

/// 解析 mobi/azw/azw3 文件，返回纯文本
fn load_mobi<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let m = mobi::Mobi::from_path(&path)?;
    let content = m.content_as_string()?;
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
    text = text
        .replace("&lt;", "<")
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

pub(crate) fn parse_chapters(lines: &[String]) -> Vec<Chapter> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_line_empty() {
        assert_eq!(Book::wrap_line("", 10), vec![String::new()]);
    }

    #[test]
    fn test_wrap_line_ascii() {
        assert_eq!(
            Book::wrap_line("hello world", 5),
            vec!["hello".to_string(), " worl".to_string(), "d".to_string()]
        );
    }

    #[test]
    fn test_wrap_line_chinese() {
        // 中文字符占 2 列宽，"你好世界" 总宽 8
        assert_eq!(
            Book::wrap_line("你好世界", 4),
            vec!["你好".to_string(), "世界".to_string()]
        );
    }

    #[test]
    fn test_wrap_line_mixed() {
        // "ab中文" 宽 = 1+1+2+2 = 6
        assert_eq!(
            Book::wrap_line("ab中文", 4),
            vec!["ab中".to_string(), "文".to_string()]
        );
    }

    #[test]
    fn test_wrap_line_exact_fit() {
        // "hello" 宽 5，恰好等于 max_width，不应再拆
        assert_eq!(Book::wrap_line("hello", 5), vec!["hello".to_string()]);
    }

    fn make_book(lines: Vec<&str>) -> Book {
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let lowercase_lines = lines.iter().map(|s| s.to_lowercase()).collect();
        Book {
            lines,
            lowercase_lines,
            chapters: vec![],
            file_path: "/tmp/test.txt".to_string(),
        }
    }

    #[test]
    fn test_search_forward_basic() {
        let book = make_book(vec!["Hello world", "Rust is great", "Hello again"]);
        assert_eq!(book.search_forward(0, "rust"), Some(1));
        assert_eq!(book.search_forward(1, "hello"), Some(2));
    }

    #[test]
    fn test_search_forward_wrap() {
        let book = make_book(vec!["first", "second", "third"]);
        // 从第 2 行搜索 "first"，应回绕到第 0 行
        assert_eq!(book.search_forward(2, "first"), Some(0));
    }

    #[test]
    fn test_search_forward_no_match() {
        let book = make_book(vec!["alpha", "beta", "gamma"]);
        assert_eq!(book.search_forward(0, "zzz"), None);
    }

    #[test]
    fn test_search_forward_empty_book() {
        let book = Book {
            lines: vec![],
            lowercase_lines: vec![],
            chapters: vec![],
            file_path: "/tmp/empty.txt".to_string(),
        };
        assert_eq!(book.search_forward(0, "x"), None);
    }

    #[test]
    fn test_search_backward_basic() {
        let book = make_book(vec!["Hello world", "Rust is great", "Hello again"]);
        assert_eq!(book.search_backward(2, "rust"), Some(1));
        assert_eq!(book.search_backward(1, "hello"), Some(0));
    }

    #[test]
    fn test_search_backward_wrap() {
        let book = make_book(vec!["first", "second", "third"]);
        // 从第 0 行反向搜索 "third"，应回绕到第 2 行
        assert_eq!(book.search_backward(0, "third"), Some(2));
    }

    #[test]
    fn test_search_backward_no_match() {
        let book = make_book(vec!["alpha", "beta", "gamma"]);
        assert_eq!(book.search_backward(0, "zzz"), None);
    }

    #[test]
    fn test_parse_chapters_chinese() {
        let lines = vec!["第一章 入门", "这是正文内容", "第二章 进阶", "更多正文"];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "第一章 入门");
        assert_eq!(chapters[0].line_index, 0);
        assert_eq!(chapters[1].title, "第二章 进阶");
        assert_eq!(chapters[1].line_index, 2);
    }

    #[test]
    fn test_parse_chapters_english() {
        let lines = vec!["Chapter 1 - Intro", "Some text here", "chapter 2 - More"];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[1].title, "chapter 2 - More");
    }

    #[test]
    fn test_parse_chapters_skips_long_lines() {
        let long_line = "a".repeat(80);
        let lines = vec![
            "第一章",
            &long_line, // 超过 60 列宽
            "第二章",
        ];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        // 中间的长行即使包含 "第一章" 也不应被识别（这里没有，只是测长度过滤）
        // 但这里要确保 "第一章" 和 "第二章" 正常识别
        assert_eq!(chapters.len(), 2);
    }

    #[test]
    fn test_parse_chapters_width_filter() {
        let long_title = format!("{}{}", "第".repeat(20), "章 这是一个超长的章节标题");
        let lines = vec!["第一章 正常", &long_title, "第二章 正常"];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        assert_eq!(chapters.len(), 2);
        assert!(chapters.iter().all(|ch| ch.title != long_title));
    }

    #[test]
    fn test_parse_chapters_volume() {
        let lines = vec!["卷一 开篇", "一些内容", "篇二 续集"];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "卷一 开篇");
        assert_eq!(chapters[1].title, "篇二 续集");
    }

    #[test]
    fn test_parse_chapters_special_titles() {
        let lines = vec![
            "楔子",
            "一些内容",
            "序章 黑暗降临",
            "正文",
            "番外 另一个故事",
            "后记",
            "尾声",
        ];
        let lines: Vec<String> = lines.into_iter().map(|s| s.to_string()).collect();
        let chapters = parse_chapters(&lines);
        let titles: Vec<&str> = chapters.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(
            titles,
            vec![
                "楔子",
                "序章 黑暗降临",
                "正文",
                "番外 另一个故事",
                "后记",
                "尾声"
            ]
        );
    }

    #[test]
    fn test_load_utf8_fixture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("sample_utf8.txt");
        let book = Book::load(&path).expect("should load utf8 fixture");
        assert!(book.lines.len() > 5);
        assert!(!book.chapters.is_empty());
        let titles: Vec<&str> = book.chapters.iter().map(|c| c.title.as_str()).collect();
        assert!(titles.contains(&"楔子"));
        assert!(titles.contains(&"第一章 风雪夜归人"));
    }

    #[test]
    fn test_load_gbk_fixture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("sample_gbk.txt");
        let book = Book::load(&path).expect("should load gbk fixture");
        assert!(book.lines.len() > 5);
        assert!(!book.chapters.is_empty());
        // 内容应正确解码为中文
        assert!(book.lines.iter().any(|l| l.contains("大雪")));
    }
}
