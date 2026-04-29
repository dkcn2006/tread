use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum DisplayMode {
    /// 伪装成服务器日志
    Log,
    /// 极简纯文本
    Minimal,
    /// 伪装成代码注释
    Comment,
}

impl DisplayMode {
    pub fn next(self) -> Self {
        match self {
            DisplayMode::Log => DisplayMode::Minimal,
            DisplayMode::Minimal => DisplayMode::Comment,
            DisplayMode::Comment => DisplayMode::Log,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::Log => "log",
            DisplayMode::Minimal => "minimal",
            DisplayMode::Comment => "comment",
        }
    }
}
