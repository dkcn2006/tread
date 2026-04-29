# tread - Code Plan

## 项目概述

**tread** 是一个伪装性极强的终端 TUI 小说阅读器，用 Rust 实现。核心设计目标：只占终端 1-2 行，看起来像普通的终端输出（日志 / 纯文本 / 代码注释），适合摸鱼场景。

## 架构总览

```
┌──────────────────────────────────────────────────────┐
│                     main.rs                          │
│          CLI 解析 → 终端初始化 → 启动 App             │
└──────────┬───────────────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────────────────┐
│                     app.rs                           │
│        App 状态机 + 事件循环 + 键盘事件分发             │
│   ┌─────────┬──────────────┬─────────────────┐       │
│   │ Normal  │   Search     │  ChapterList    │       │
│   │  模式   │    模式      │     模式         │       │
│   └─────────┴──────────────┴─────────────────┘       │
└──────┬──────────┬──────────────┬─────────────────────┘
       │          │              │
       ▼          ▼              ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│  ui.rs    │ │ reader.rs │ │bookmark.rs│
│ 三种渲染  │ │ 文本加载   │ │ 进度持久化 │
│  模式     │ │ 编码/分页  │ │           │
└───────────┘ └───────────┘ └───────────┘
       │
       ▼
┌───────────┐
│ config.rs │
│ 模式枚举  │
└───────────┘
```

## 技术选型

| 用途 | 依赖 | 版本 | 说明 |
|------|------|------|------|
| TUI 框架 | `ratatui` | 0.29 | 终端 UI 渲染，使用 Inline viewport 不进入 alternate screen |
| 终端控制 | `crossterm` | 0.28 | 跨平台终端 raw mode、事件监听 |
| CLI 解析 | `clap` | 4 (derive) | 声明式命令行参数解析 |
| 序列化 | `serde` + `serde_json` | 1 | 书签 JSON 持久化 |
| 编码检测 | `encoding_rs` | 0.8 | 处理 GBK/GB2312/BIG5 等中文 txt 编码 |
| 中文宽度 | `unicode-width` | 0.2 | 正确计算中文字符的终端显示宽度 |
| 配置目录 | `dirs` | 6 | 跨平台获取 `~/.config/` 路径 |
| 时间 | `chrono` | 0.4 | 日志模式中的时间戳生成 |
| 正则 | `regex` | 1 | 章节标题识别 |

## 文件详细设计

---

### 1. `Cargo.toml` - 项目配置

```toml
[[bin]]
name = "tread"          # 二进制名称，直接用 tread 命令运行
path = "src/main.rs"
```

关键决策：
- `edition = "2021"` 而非 2024，确保依赖兼容性
- 二进制命名为 `tread`（terminal + read 的缩写），简短好记

---

### 2. `src/config.rs` - 显示模式枚举

**职责**：定义三种伪装模式的枚举，以及模式间的切换逻辑。

```rust
pub enum DisplayMode {
    Log,      // 伪装成服务器日志
    Minimal,  // 极简纯文本
    Comment,  // 伪装成代码注释
}
```

**核心方法**：

| 方法 | 功能 |
|------|------|
| `next()` | 循环切换到下一个模式：Log → Minimal → Comment → Log |
| `to_index()` / `from_index()` | 模式与数字互转，用于书签序列化 |
| `label()` | 返回模式名称字符串 |

**设计决策**：
- 同时派生 `ValueEnum`（供 clap CLI 解析）和 `Serialize/Deserialize`（供书签存储）
- 模式切换用 `match` 而非数组索引，编译期保证穷举

---

### 3. `src/reader.rs` - 文本加载与分页引擎

**职责**：文件读取、编码检测、章节解析、智能折行、搜索。这是整个阅读器的数据层核心。

#### 数据结构

```rust
pub struct Chapter {
    pub title: String,       // 章节标题文本
    pub line_index: usize,   // 该章节在 Book.lines 中的起始索引
}

pub struct Book {
    pub lines: Vec<String>,      // 全书所有非空行（已 trim）
    pub chapters: Vec<Chapter>,  // 识别到的章节列表
    pub file_path: String,       // 文件的 canonical 路径（用作书签 key）
}
```

#### 核心函数

**`Book::load(path)` - 加载文件**
```
读取原始字节 → decode_text() 编码检测 → 按行分割并过滤空行 → parse_chapters() 章节识别
```

**`decode_text(raw)` - 编码检测链**
```
UTF-8 验证 → BOM 检测 → 依次尝试 GBK/GB18030/BIG5/EUC-JP/EUC-KR → 最终 lossy UTF-8
```
设计决策：优先尝试 UTF-8（大多数现代文件），然后逐个尝试常见中文编码，`encoding_rs` 的 `had_errors` 标志用于判断是否匹配。

**`parse_chapters(lines)` - 章节识别**
```
正则匹配三种模式：
1. 第X章/节/回/卷（中文数字+阿拉伯数字）
2. Chapter X（英文，不区分大小写）
3. 卷X / 篇X
```
过滤条件：行显示宽度 ≤ 60 列（排除碰巧包含"第X章"的正文段落）。

**`Book::wrap_line(line, max_width)` - 智能折行**
```
逐字符遍历 → 用 UnicodeWidthChar 计算每个字符的显示宽度 → 累计超过 max_width 时断行
```
关键点：中文字符占 2 列宽，英文/数字占 1 列宽。这里不能用字符数，必须用显示宽度。

**`Book::get_display_lines(start, sub_offset, count, max_width)` - 获取显示行**
```
从 start 行的 sub_offset 子行开始 → 逐行 wrap → 收集到 count 行为止
返回 (显示行列表, 下一个书行索引, 下一个子行偏移)
```
这个函数是连接 Book 数据和 UI 渲染的桥梁。

**`Book::search_forward(start, query)` - 前向搜索**
```
从 start 行向后扫描 → 到达末尾后从头回绕 → 返回第一个匹配行的索引
```
搜索为大小写不敏感（`.to_lowercase()`）。

---

### 4. `src/bookmark.rs` - 阅读进度持久化

**职责**：将阅读进度以 JSON 格式保存到磁盘，下次打开同一文件时自动恢复。

#### 数据结构

```rust
pub struct BookmarkEntry {
    pub line_index: usize,  // 书行索引
    pub sub_offset: usize,  // 折行后的子行偏移
    pub mode: usize,        // 上次使用的显示模式
}

pub struct Bookmarks {
    entries: HashMap<String, BookmarkEntry>,  // 文件路径 → 书签
}
```

#### 存储路径

```
~/.config/terminal-read/bookmarks.json
```
使用 `dirs::config_dir()` 获取跨平台配置目录，自动创建父目录。

#### 核心流程

```
启动时：Bookmarks::load() → 读取 JSON → 反序列化 → 查找当前文件的书签
退出时：Bookmarks::set() → 更新 HashMap → Bookmarks::save() → 序列化写入 JSON
```

设计决策：
- 用 canonical 文件路径作为 key，避免相对路径导致的书签匹配失败
- `load()` 在任何错误情况下都返回空 Bookmarks（不阻塞启动）
- `save()` 的错误被 `let _ =` 忽略（摸鱼工具不需要把错误暴露给用户）

---

### 5. `src/ui.rs` - 三种伪装模式的渲染层

**职责**：根据当前 App 状态，用 ratatui 渲染对应的伪装 UI。

#### 渲染调度

```rust
pub fn render(frame, app) {
    match app.input_mode {
        ChapterList => render_chapter_list(),  // 章节目录优先级最高
        _ => match app.mode {
            Log     => render_log(),
            Minimal => render_minimal(),
            Comment => render_comment(),
        }
    }
}
```

#### 模式 1：`render_log()` - 日志伪装

```
[2026-04-28 14:32:01] INFO  却说那贾雨村在金陵城中闲居无事...
[2026-04-28 14:32:02] DEBUG 因向甄士隐一一道来,原来雨村...
```

实现细节：
- 时间戳用 `chrono::Local::now()`，每行递增 1 秒（模拟真实日志）
- 日志级别按行号 `% 4` 循环选取 INFO/DEBUG/TRACE/WARN
- 级别用不同颜色：INFO=绿, DEBUG=青, TRACE=灰, WARN=黄
- 时间戳占 29 字符宽，小说内容填满剩余宽度
- 前缀用 `DarkGray` 色，文本内容用默认色

#### 模式 2：`render_minimal()` - 极简模式

```
却说那贾雨村在金陵城中闲居无事因向甄士隐一一道来 [3/1205]
```

实现细节：
- 进度指示器 `[当前行/总行数]` 放在最后一行末尾
- 进度文字用 `DarkGray` 色，不抢眼
- 内容宽度 = 终端宽度 - 进度文字宽度

#### 模式 3：`render_comment()` - 代码注释伪装

```
// 却说那贾雨村在金陵城中闲居无事,
// 因向甄士隐一一道来,原来雨村...   [Ch.3 | 0.2%]
```

实现细节：
- 每行前缀 `"// "`（3 字符），用 `DarkGray` 色
- 最后一行末尾追加 `[Ch.X | X.X%]`（章节号 + 百分比进度）
- 空行也显示 `"//"`（维持注释块的视觉一致性）
- 内容宽度 = 终端宽度 - 前缀宽度 - 后缀宽度

#### 搜索模式覆盖

三种模式都有相同逻辑：当 `InputMode::Search` 时，弹出最后一行替换为搜索栏：
```
/搜索关键词_
```
黄色 `/` 前缀 + 输入文本 + 黄色光标 `_`。

#### `render_chapter_list()` - 章节目录

```
> 第一回 甄士隐梦幻识通灵
  第二回 贾夫人仙逝扬州城
  第三回 贾雨村夤缘复旧职
```

- 当前选中项前显示 `> ` 标记，用黄色高亮
- 支持滚动：当选中项超出可视范围时自动滚动

---

### 6. `src/app.rs` - 应用状态机与事件循环

**职责**：管理全部应用状态，驱动事件循环，分发键盘输入。

#### 状态机

```
              ┌──────────┐
     ┌────────│  Normal  │────────┐
     │  '/'   └────┬─────┘  'g'   │
     ▼             │              ▼
┌──────────┐       │       ┌──────────────┐
│  Search  │       │       │ ChapterList  │
└────┬─────┘       │       └──────┬───────┘
     │ Enter/Esc   │              │ Enter/Esc
     └─────────────┼──────────────┘
                   │
            'q' or Esc
                   │
                   ▼
              ┌──────────┐
              │   Exit   │
              └──────────┘
```

#### App 结构体

```rust
pub struct App {
    // 数据
    pub book: Book,              // 加载的书籍内容
    bookmarks: Bookmarks,        // 书签管理器

    // 阅读状态
    pub current_line: usize,     // 当前书行索引
    pub sub_offset: usize,       // 折行子行偏移
    pub display_lines: usize,    // 显示行数（1-3）
    pub terminal_width: u16,     // 终端宽度（每帧从 frame.area() 同步）
    pub mode: DisplayMode,       // 当前伪装模式

    // 输入状态
    pub input_mode: InputMode,   // 当前输入模式
    pub search_input: String,    // 搜索框输入缓冲
    pub last_search: String,     // 上次搜索词（用于 'n' 重复搜索）
    pub chapter_cursor: usize,   // 章节列表中的光标位置

    // 退出标志
    pub should_quit: bool,       // 正常退出
    pub boss_key: bool,          // 老板键退出（需清屏）
}
```

#### 初始化流程

```
App::new(book, mode, display_lines)
    │
    ├── Bookmarks::load()                    // 从磁盘加载书签
    ├── 查找当前文件的书签                      // bookmarks.get(&book.file_path)
    │   ├── 有 → 恢复 line_index / sub_offset / mode
    │   └── 无 → 从头开始，使用 CLI 指定的 mode
    └── 构造 App 实例
```

#### 事件循环

```rust
pub fn run(&mut self, terminal) {
    loop {
        terminal.draw(|frame| {
            self.terminal_width = frame.area().width;  // 每帧同步终端宽度
            ui::render(frame, self);
        });
        if event::poll(250ms) {                          // 等待输入
            if let Key(key) = event::read() {
                match self.input_mode {                  // 分发到对应 handler
                    Normal     => handle_normal_key(),
                    Search     => handle_search_key(),
                    ChapterList => handle_chapter_key(),
                }
            }
        }
        if should_quit || boss_key {
            save_bookmark();                             // 退出前保存进度
            break;
        }
    }
}
```

设计决策：`event::poll(250ms)` 而非阻塞等待，确保 UI 能定期刷新（日志模式的时间戳需要更新）。

#### 逐显示行滚动（重要）

> **踩坑记录**：早期版本中 `j`/`k` 按**段落（书行）**整行跳转，但 txt 小说中一个段落往往很长，经终端折行后会产生多个显示行。按一次 `j` 就跳过整个段落，用户感知到"跳过了好几行内容"。

**解决方案**：导航必须基于**显示行（display line）**而非书行（book line）。

核心滚动方法：

```rust
/// 前进一个显示行
fn next_display_line(&mut self, content_width: usize) {
    let wrapped = Book::wrap_line(&self.book.lines[self.current_line], content_width);
    if self.sub_offset + 1 < wrapped.len() {
        // 当前段落还有未显示的折行 → 只前进一个子行
        self.sub_offset += 1;
    } else if self.current_line + 1 < self.book.lines.len() {
        // 当前段落已显示完毕 → 进入下一个段落
        self.current_line += 1;
        self.sub_offset = 0;
    }
}

/// 后退一个显示行
fn prev_display_line(&mut self, content_width: usize) {
    if self.sub_offset > 0 {
        // 当前段落内回退一个子行
        self.sub_offset -= 1;
    } else if self.current_line > 0 {
        // 回到上一段落的最后一个子行
        self.current_line -= 1;
        let wrapped = Book::wrap_line(&self.book.lines[self.current_line], content_width);
        self.sub_offset = wrapped.len().saturating_sub(1);
    }
}
```

关键实现细节：
- `content_width` 由 `handle_normal_key()` 根据 `terminal_width` 减去最大前缀宽度（~32 字符，对应日志模式）估算
- `terminal_width` 在每帧渲染时从 `frame.area().width` 同步，确保终端窗口缩放后滚动行为仍然正确
- `next_lines(n)` 和 `prev_lines(n)` 内部循环调用 `next_display_line()` / `prev_display_line()`，保证翻页（空格键）也是精确的

#### 键位映射

**Normal 模式**：

| 按键 | 处理函数/逻辑 |
|------|--------------|
| `j` / `↓` / `Enter` | `next_lines(1)` - 前进一行 |
| `k` / `↑` | `prev_lines(1)` - 后退一行 |
| `空格` | `next_lines(display_lines)` - 翻一屏 |
| `Home` | 跳到文件开头 |
| `End` | 跳到文件末尾 |
| `t` | `mode = mode.next()` - 切换伪装模式 |
| `/` | 进入 Search 模式，清空搜索缓冲 |
| `n` | 用 `last_search` 重复搜索下一个 |
| `g` | 进入 ChapterList 模式，光标定位到当前章节 |
| `q` | 设置 `should_quit = true` |
| `Esc` | 设置 `boss_key = true`（老板键） |
| `Ctrl+C` | 设置 `should_quit = true` |

**Search 模式**：

| 按键 | 逻辑 |
|------|------|
| 字符 | 追加到 `search_input` |
| `Backspace` | 删除最后一个字符 |
| `Enter` | 执行搜索 → 跳转到匹配行 → 回到 Normal |
| `Esc` | 取消搜索 → 回到 Normal |

**ChapterList 模式**：

| 按键 | 逻辑 |
|------|------|
| `j` / `↓` | 光标下移 |
| `k` / `↑` | 光标上移 |
| `Enter` | 跳转到选中章节 → 回到 Normal |
| `Esc` / `q` / `g` | 关闭目录 → 回到 Normal |

---

### 7. `src/main.rs` - 程序入口

**职责**：CLI 参数解析、终端初始化/恢复、启动 App、处理退出。

#### CLI 参数

```
tread <FILE> [--mode log|minimal|comment] [--lines 1|2]
```

使用 `clap` derive 宏自动生成参数解析和 `--help`。

#### 终端初始化策略

```rust
// 关键设计：使用 Inline viewport 而非 AlternateScreen
terminal::enable_raw_mode()?;
Terminal::with_options(backend, TerminalOptions {
    viewport: Viewport::Inline(display_lines as u16),  // 只占 1-2 行
});
```

**为什么不用 AlternateScreen？**
- AlternateScreen 会清空整个终端并切换到新缓冲区，退出后恢复 —— 这个"闪烁"非常可疑
- Inline viewport 只在终端底部追加指定行数，看起来就像一条普通命令的输出，极为隐蔽

#### 退出处理

```
正常退出 (q):      disable_raw_mode → println!() 换行 → 结束
老板键退出 (Esc):  disable_raw_mode → Clear(All) 清屏 → MoveTo(0,0) → 结束
```

老板键会清空整个终端内容并把光标移到左上角，看起来就像刚打开了一个新终端窗口。

---

## 数据流总结

```
txt 文件
    │
    ▼ Book::load()
┌─────────────┐
│  raw bytes  │──── decode_text() ────▶ UTF-8 String
└─────────────┘                              │
                                             ▼ 按行分割、过滤空行
                                      ┌─────────────┐
                                      │ Vec<String>  │ (Book.lines)
                                      └──────┬──────┘
                                             │
                              ┌──────────────┼──────────────┐
                              ▼              ▼              ▼
                       parse_chapters  get_display_lines  search_forward
                              │              │              │
                              ▼              ▼              ▼
                       Vec<Chapter>    渲染用的行      匹配行索引
                              │              │              │
                              └──────────────┼──────────────┘
                                             │
                                             ▼
                                      ┌─────────────┐
                                      │   ui.rs     │ 渲染到终端
                                      └─────────────┘
```

## 关键设计决策汇总

| 决策 | 选择 | 原因 |
|------|------|------|
| Viewport 模式 | Inline（非 AlternateScreen） | 隐蔽性：看起来像普通命令输出 |
| 编码检测策略 | 依次尝试 UTF-8 → GBK → GB18030 → BIG5 | 覆盖绝大多数中文 txt 小说 |
| 字符宽度计算 | `unicode-width` crate | 正确处理中文占 2 列宽的情况 |
| 书签 key | 文件 canonical 路径 | 避免 `./a.txt` 和 `a.txt` 被当作不同文件 |
| 事件轮询 | 250ms timeout | 平衡响应速度和 CPU 占用 |
| 章节识别 | 正则 + 宽度过滤 (≤60) | 避免正文中碰巧包含"第X章"的长句被误判 |
| 老板键 | Esc 清屏 + 光标归零 | 一键消失，不留痕迹 |
| 逐显示行滚动 | 基于 sub_offset 的子行级导航 | 长段落折行后按 j/k 不会跳过内容（见 app.rs 踩坑记录） |
