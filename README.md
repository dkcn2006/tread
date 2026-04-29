# tread

> **T**erminal **read** — 在终端里摸鱼看小说的终极方案。

## 这玩意儿是干嘛的？

为所有 **CLI vibe coding** 用户打造。

当你和 AI 在 terminal 里结对编程时——不管是 Claude Code、Aider、Cline 还是别的工具——难免会陷入漫长的等待：AI 在思考、在生成、在跑测试……屏幕上一大堆日志滚动，你的眼神却逐渐失焦。

这时候，你悄悄按两下键盘，终端底部不动声色地滑出一两行"日志"：

```
[2026-04-29 09:15:01] INFO  却说那贾雨村在金陵城中闲居无事...
```

同事路过，扫一眼你的屏幕，只看到平平无奇的服务器日志，然后放心地走开了。

**这就是 tread 存在的意义。**

## 伪装模式

`tread` 提供三种伪装形态，按 `t` 键循环切换：

| 模式 | 效果 | 适用场景 |
|------|------|---------|
| **Log** | `[时间戳] INFO  小说内容...` | 最常用，看起来像后端日志 |
| **Minimal** | `小说内容... [42/1205]` | 极简，像一条普通的命令输出 |
| **Comment** | `// 小说内容... [Ch.3 \| 2.1%]` | 像代码注释，适合前端项目 |

所有模式都只占终端 **1-3 行**，不进入 alternate screen，不刷屏，**隐蔽性拉满**。

## 环境配置

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 配置 Cargo 镜像（可选，国内用户推荐）

创建或编辑 `~/.cargo/config`：

```toml
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"
replace-with = 'ustc'

[source.ustc]
registry = "git://mirrors.ustc.edu.cn/crates.io-index"
```

国内常用镜像：
- **USTC**: `git://mirrors.ustc.edu.cn/crates.io-index`
- **TUNA**: `https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git`

### 3. 一键全局安装（推荐）

```bash
git clone https://github.com/dkcn2006/tread.git
cd tread
./install.sh
```

脚本会自动完成：
1. 检测 Rust 工具链
2. 将 cargo 配置持久化到 `~/.bashrc` / `~/.zshrc`
3. 编译 release 版本
4. 安装到 `~/.cargo/bin/`（全局可用）
5. 验证安装

> 首次编译需几分钟，取决于网络和设备性能。

### 3b. 手动编译（想自己控制安装位置）

```bash
git clone https://github.com/dkcn2006/tread.git
cd tread
cargo build --release
```

编译完成后，二进制位于 `target/release/tread`。手动加入 PATH：

```bash
# macOS / Linux
cp target/release/tread /usr/local/bin/
# 或加入 PATH
export PATH="$PATH:$(pwd)/target/release"
```

## 使用

项目仓库的 `txt/` 目录下已预置了示例小说（需自行放置你的 `.txt` 文件，该目录已被 `.gitignore` 忽略）。

```bash
# 基本用法
tread "txt/冰与火之歌一：权利的游戏.txt"

# 指定模式（log / minimal / comment）
tread "txt/冰与火之歌一：权利的游戏.txt" --mode comment

# 显示 2 行（默认 1 行）
tread "txt/冰与火之歌一：权利的游戏.txt" --lines 2
```

### 键位

| 按键 | 功能 |
|------|------|
| `j` / `↓` / `Enter` | 向下滚动一行 |
| `k` / `↑` | 向上滚动一行 |
| `Space` / `PageDown` | 向下翻一屏 |
| `b` / `PageUp` | 向上翻一屏 |
| `Home` | 跳到开头 |
| `End` | 跳到末尾 |
| `t` | 切换伪装模式 |
| `/` | 搜索 |
| `n` | 重复上次搜索 |
| `g` | 打开章节目录 |
| `q` | 正常退出并保存进度 |
| `Esc` | **老板键** — 清屏并立即退出 |

### 章节识别

自动识别 `第X章`、`Chapter X`、`卷X` 等常见章节标题格式，章节目录按 `g` 呼出。

### 书签

退出时自动保存阅读进度（行号、显示模式）到 `~/.config/terminal-read/bookmarks.json`。下次打开同一本书时自动续读。

### 编码

自动检测 UTF-8 / GBK / GB18030 / BIG5 等中文编码，无需手动转码。

## 技术栈

- [ratatui](https://github.com/ratatui/ratatui) — TUI 渲染（Inline viewport，不进入 alternate screen）
- [crossterm](https://github.com/crossterm-rs/crossterm) — 跨平台终端控制
- [clap](https://github.com/clap-rs/clap) — CLI 参数解析
- [encoding_rs](https://github.com/hsivonen/encoding_rs) — 中文编码检测

## License

MIT
