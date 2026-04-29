use crate::config::DisplayMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkEntry {
    pub line_index: usize,
    pub sub_offset: usize,
    pub mode: DisplayMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmarks {
    entries: HashMap<String, BookmarkEntry>,
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
        }
    }

    pub fn load() -> Self {
        let path = Self::bookmark_path();
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Bookmarks>(&content) {
                Ok(bm) => bm,
                Err(_) => Self::new(),
            },
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

    pub fn set(&mut self, file_path: &str, entry: BookmarkEntry) {
        self.entries.insert(file_path.to_string(), entry);
    }

    fn bookmark_path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("terminal-read")
            .join("bookmarks.json")
    }
}
