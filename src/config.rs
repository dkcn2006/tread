use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum DisplayMode {
    /// 伪装成服务器日志
    Log,
    /// 极简纯文本
    Minimal,
    /// 伪装成代码注释
    Comment,
    /// 伪装成 git log
    GitLog,
    /// 伪装成 npm install 输出
    NpmInstall,
    /// 伪装成 pytest 测试输出
    Pytest,
    /// 伪装成 docker logs
    DockerLogs,
    /// 伪装成 kubectl logs
    KubectlLogs,
}

impl DisplayMode {
    pub fn next(self) -> Self {
        match self {
            DisplayMode::Log => DisplayMode::Minimal,
            DisplayMode::Minimal => DisplayMode::Comment,
            DisplayMode::Comment => DisplayMode::GitLog,
            DisplayMode::GitLog => DisplayMode::NpmInstall,
            DisplayMode::NpmInstall => DisplayMode::Pytest,
            DisplayMode::Pytest => DisplayMode::DockerLogs,
            DisplayMode::DockerLogs => DisplayMode::KubectlLogs,
            DisplayMode::KubectlLogs => DisplayMode::Log,
        }
    }
}

impl FromStr for DisplayMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "log" => Ok(DisplayMode::Log),
            "minimal" => Ok(DisplayMode::Minimal),
            "comment" => Ok(DisplayMode::Comment),
            "gitlog" => Ok(DisplayMode::GitLog),
            "npm_install" | "npminstall" | "npm" => Ok(DisplayMode::NpmInstall),
            "pytest" => Ok(DisplayMode::Pytest),
            "dockerlogs" | "docker" => Ok(DisplayMode::DockerLogs),
            "kubectllogs" | "kubectl" => Ok(DisplayMode::KubectlLogs),
            _ => Err(format!("unknown display mode: {}", s)),
        }
    }
}
