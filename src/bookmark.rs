use crate::config::DisplayMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkEntry {
    pub line_index: usize,
    pub sub_offset: usize,
    pub mode: DisplayMode,
    /// Unix timestamp in seconds
    pub last_accessed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmarks {
    entries: HashMap<String, BookmarkEntry>,
    favorites: HashMap<String, Vec<usize>>,
}

impl Default for Bookmarks {
    fn default() -> Self {
        Self::new()
    }
}

impl Bookmarks {
    pub fn new() -> Self {
        Bookmarks {
            entries: HashMap::new(),
            favorites: HashMap::new(),
        }
    }

    pub fn load() -> Self {
        let path = Self::bookmark_path();
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str::<Bookmarks>(&content).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self) {
        let path = Self::bookmark_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub fn get(&self, file_path: &str) -> Option<&BookmarkEntry> {
        self.entries.get(file_path)
    }

    pub fn toggle_favorite(&mut self, file_path: &str, line_index: usize) -> bool {
        let list = self.favorites.entry(file_path.to_string()).or_default();
        if let Some(pos) = list.iter().position(|&x| x == line_index) {
            list.remove(pos);
            false
        } else {
            list.push(line_index);
            list.sort();
            true
        }
    }

    pub fn favorites(&self, file_path: &str) -> Vec<usize> {
        self.favorites.get(file_path).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, file_path: &str, mut entry: BookmarkEntry) {
        entry.last_accessed = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
        self.entries.insert(file_path.to_string(), entry);
    }

    pub fn recent(&self, n: usize) -> Vec<(&String, &BookmarkEntry)> {
        let mut items: Vec<_> = self.entries.iter().collect();
        items.sort_by_key(|(_, e)| std::cmp::Reverse(e.last_accessed));
        items.into_iter().take(n).collect()
    }

    fn bookmark_path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("terminal-read")
            .join("bookmarks.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_serde_roundtrip() {
        let mut bm = Bookmarks::new();
        bm.set(
            "/home/user/novel.txt",
            BookmarkEntry {
                line_index: 42,
                sub_offset: 3,
                mode: DisplayMode::Log,
                last_accessed: Some(1000),
            },
        );
        bm.set(
            "/home/user/another.epub",
            BookmarkEntry {
                line_index: 100,
                sub_offset: 0,
                mode: DisplayMode::Comment,
                last_accessed: Some(2000),
            },
        );
        bm.toggle_favorite("/home/user/novel.txt", 10);
        bm.toggle_favorite("/home/user/novel.txt", 20);

        let json = serde_json::to_string_pretty(&bm).unwrap();
        let restored: Bookmarks = serde_json::from_str(&json).unwrap();

        let entry1 = restored.get("/home/user/novel.txt").unwrap();
        assert_eq!(entry1.line_index, 42);
        assert_eq!(entry1.sub_offset, 3);
        assert_eq!(entry1.mode, DisplayMode::Log);

        let entry2 = restored.get("/home/user/another.epub").unwrap();
        assert_eq!(entry2.line_index, 100);
        assert_eq!(entry2.mode, DisplayMode::Comment);

        assert_eq!(restored.favorites("/home/user/novel.txt"), vec![10, 20]);
    }

    #[test]
    fn test_bookmark_get_missing() {
        let bm = Bookmarks::new();
        assert!(bm.get("/nonexistent.txt").is_none());
    }

    #[test]
    fn test_bookmark_overwrite() {
        let mut bm = Bookmarks::new();
        bm.set(
            "/a.txt",
            BookmarkEntry {
                line_index: 1,
                sub_offset: 0,
                mode: DisplayMode::Minimal,
                last_accessed: None,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        bm.set(
            "/a.txt",
            BookmarkEntry {
                line_index: 99,
                sub_offset: 2,
                mode: DisplayMode::Log,
                last_accessed: None,
            },
        );

        let entry = bm.get("/a.txt").unwrap();
        assert_eq!(entry.line_index, 99);
        assert_eq!(entry.sub_offset, 2);
        assert_eq!(entry.mode, DisplayMode::Log);
        assert!(entry.last_accessed.is_some());
    }

    #[test]
    fn test_bookmark_json_format() {
        // 验证 JSON 输出包含字段名和枚举字符串值
        let mut bm = Bookmarks::new();
        bm.set(
            "/test.txt",
            BookmarkEntry {
                line_index: 0,
                sub_offset: 0,
                mode: DisplayMode::Comment,
                last_accessed: Some(0),
            },
        );
        let json = serde_json::to_string(&bm).unwrap();
        assert!(json.contains("\"line_index\":0"));
        assert!(json.contains("\"mode\":\"Comment\""));
    }

    #[test]
    fn test_favorite_toggle() {
        let mut bm = Bookmarks::new();
        assert!(bm.toggle_favorite("/a.txt", 10)); // add 10
        assert!(bm.toggle_favorite("/a.txt", 5)); // add 5, sorted
        assert_eq!(bm.favorites("/a.txt"), vec![5, 10]);
        assert!(!bm.toggle_favorite("/a.txt", 10)); // remove 10
        assert_eq!(bm.favorites("/a.txt"), vec![5]);
    }

    #[test]
    fn test_bookmark_recent_order() {
        let mut bm = Bookmarks::new();
        bm.set(
            "/old.txt",
            BookmarkEntry {
                line_index: 1,
                sub_offset: 0,
                mode: DisplayMode::Minimal,
                last_accessed: None,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        bm.set(
            "/mid.txt",
            BookmarkEntry {
                line_index: 3,
                sub_offset: 0,
                mode: DisplayMode::Minimal,
                last_accessed: None,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        bm.set(
            "/new.txt",
            BookmarkEntry {
                line_index: 2,
                sub_offset: 0,
                mode: DisplayMode::Minimal,
                last_accessed: None,
            },
        );

        let recent = bm.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].0, "/new.txt");
        assert_eq!(recent[1].0, "/mid.txt");
    }
}
