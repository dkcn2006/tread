#!/usr/bin/env python3
"""
生成一个模拟的 asciinema demo.cast，展示 tread 的核心使用流程。
用法:
    python3 scripts/generate-asciinema.py
    # 然后本地播放:
    asciinema play assets/demo.cast
"""

import json
import sys
import os

# ANSI 颜色代码
DIM = "\x1b[2m"
GRAY = "\x1b[90m"
GREEN = "\x1b[32m"
CYAN = "\x1b[36m"
YELLOW = "\x1b[33m"
MAGENTA = "\x1b[35m"
RESET = "\x1b[0m"
CLEAR = "\x1b[H\x1b[J"
HIDE_CURSOR = "\x1b[?25l"
SHOW_CURSOR = "\x1b[?25h"


def write_cast(path: str):
    lines = []

    # 头部
    header = {
        "version": 2,
        "width": 90,
        "height": 24,
        "timestamp": 1715000000,
        "env": {"SHELL": "/bin/zsh", "TERM": "xterm-256color"},
    }
    lines.append(json.dumps(header))

    def out(delay: float, text: str):
        lines.append(json.dumps([delay, "o", text]))

    def key(delay: float, text: str):
        lines.append(json.dumps([delay, "i", text]))

    t = 0.0

    # 开场: 清屏 + 提示符
    out(t, CLEAR)
    t += 0.2
    out(t, f"{GREEN}~/novels{RESET} $ ")

    # 输入命令
    cmd = "tread 冰与火之歌.txt --mode log"
    for ch in cmd:
        key(0.03, ch)
        out(0.0, ch)
    key(0.2, "\r")
    out(0.0, "\n")

    t += 0.5

    # 日志模式输出 (Inline viewport 效果)
    novel_lines = [
        "窗外飘着鹅毛大雪，寒风呼啸着拍打着破旧的木窗。",
        "少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。",
        "他的手指轻轻抚过书页，眼中闪烁着坚定的光芒。",
        "门外传来一阵急促的脚步声，打破了夜晚的宁静。",
    ]

    out(t, HIDE_CURSOR)
    for i, content in enumerate(novel_lines):
        timestamp = f"2025-05-10 14:{30+i:02d}:00"
        level = ["INFO", "DEBUG", "TRACE", "WARN"][i % 4]
        colors = [GREEN, CYAN, GRAY, YELLOW]
        color = colors[i % 4]
        prefix = f"[{GRAY}{timestamp}{RESET}] {color}{level:5}{RESET}  "
        out(0.25, prefix + content + "\n")

    t += 0.5

    # 按 t 切换 Minimal 模式
    key(0.5, "t")
    out(0.0, "\n")
    for i, content in enumerate(novel_lines):
        progress = f" [{i+5:5}/{len(novel_lines)+10:5}]"
        out(0.15, content + f"{DIM}{progress}{RESET}\n")

    t += 0.3

    # 按 t 切换 Comment 模式
    key(0.5, "t")
    out(0.0, "\n")
    for i, content in enumerate(novel_lines):
        suffix = f" [Ch.1 | {((i+5)*100//20):.1f}%]"
        out(0.15, f"{GRAY}// {RESET}{content}{DIM}{suffix}{RESET}\n")

    t += 0.3

    # 按 / 搜索
    key(0.5, "/")
    out(0.0, f"\n{YELLOW}/{RESET}")
    for ch in "大雪":
        key(0.08, ch)
        out(0.0, ch)
    key(0.2, "\r")
    out(0.0, "\n")
    out(0.1, f"[{GRAY}2025-05-10 14:31:00{RESET}] {GREEN}INFO {RESET}  {YELLOW}窗外飘着鹅毛{RESET}，寒风呼啸着拍打着破旧的木窗。\n")

    t += 0.5

    # 按 n 下一个匹配
    key(0.5, "n")
    out(0.0, "\n")
    out(0.1, f"[{GRAY}2025-05-10 14:31:01{RESET}] {CYAN}DEBUG{RESET}  门外传来一阵急促的脚步声，打破了夜晚的宁静。\n")

    t += 0.3

    # 按 h 隐藏
    key(0.5, "h")
    out(0.0, "\n")
    # 隐藏后显示空白行
    for _ in range(3):
        out(0.1, "\n")

    t += 0.3
    key(0.5, "j")
    out(0.0, "\n")
    # 恢复显示
    out(0.1, f"[{GRAY}2025-05-10 14:31:02{RESET}] {GREEN}INFO {RESET}  少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。\n")

    t += 0.5

    # 按 q 退出
    key(0.5, "q")
    out(0.0, "\n")
    out(0.1, SHOW_CURSOR)
    out(0.1, f"{GREEN}~/novels{RESET} $ ")

    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"生成: {path}")


if __name__ == "__main__":
    os.makedirs("assets", exist_ok=True)
    write_cast("assets/demo.cast")
