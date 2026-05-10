#!/usr/bin/env python3
"""
生成 tread 三种模式的模拟终端截图
"""

from PIL import Image, ImageDraw, ImageFont
import os

# 终端配色
BG_COLOR = (12, 12, 12)       # 深黑背景
FG_COLOR = (204, 204, 204)    # 默认文字
DIM_COLOR = (100, 100, 100)   # 暗淡文字

# Log 模式颜色
LOG_TIMESTAMP = (128, 128, 128)
LOG_INFO = (0, 200, 0)
LOG_DEBUG = (0, 180, 180)
LOG_TRACE = (150, 150, 150)
LOG_WARN = (200, 180, 0)

# Comment 模式颜色
COMMENT_PREFIX = (100, 160, 100)
COMMENT_SUFFIX = (128, 128, 128)

# Minimal 模式颜色
MINIMAL_PROGRESS = (180, 180, 180)

LINE_HEIGHT = 26
PADDING = 16
CHAR_WIDTH = 13  # 近似等宽字符宽度

# 示例小说文本
SAMPLE_LINES = [
    "第一章 风雪夜归人",
    "窗外飘着鹅毛大雪，寒风呼啸着拍打着破旧的木窗。",
    "少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。",
]

def get_font(size=16):
    """尝试加载等宽字体，回退到默认"""
    font_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ]
    for path in font_paths:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                continue
    return ImageFont.load_default()

def create_screenshot(name, lines, width=700, height=140):
    """生成一张终端风格截图"""
    img = Image.new("RGB", (width, height), BG_COLOR)
    draw = ImageDraw.Draw(img)
    font = get_font(15)

    y = PADDING
    for item in lines:
        text = item.get("text", "")
        color = item.get("color", FG_COLOR)
        x = PADDING

        # 处理带颜色片段的行
        if isinstance(text, list):
            for segment in text:
                seg_text = segment.get("text", "")
                seg_color = segment.get("color", FG_COLOR)
                draw.text((x, y), seg_text, fill=seg_color, font=font)
                # 估算宽度（中文字符约2倍宽度）
                for ch in seg_text:
                    if ord(ch) > 127:
                        x += CHAR_WIDTH
                    else:
                        x += CHAR_WIDTH * 0.6
        else:
            draw.text((x, y), text, fill=color, font=font)

        y += LINE_HEIGHT

    # 保存
    os.makedirs("assets", exist_ok=True)
    path = f"assets/{name}.png"
    img.save(path)
    print(f"生成: {path}")
    return path

def main():
    font = get_font(15)

    # ── Log 模式 ──
    log_lines = [
        {
            "text": [
                {"text": "[2025-04-29 21:30:12] ", "color": LOG_TIMESTAMP},
                {"text": "INFO  ", "color": LOG_INFO},
                {"text": "窗外飘着鹅毛大雪，寒风呼啸着拍打着破旧的木窗。", "color": FG_COLOR},
            ]
        },
        {
            "text": [
                {"text": "[2025-04-29 21:30:13] ", "color": LOG_TIMESTAMP},
                {"text": "DEBUG ", "color": LOG_DEBUG},
                {"text": "少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。", "color": FG_COLOR},
            ]
        },
    ]
    create_screenshot("mode-log", log_lines, width=780, height=110)

    # ── Minimal 模式 ──
    minimal_lines = [
        {
            "text": [
                {"text": "[  2/342] ", "color": MINIMAL_PROGRESS},
                {"text": "窗外飘着鹅毛大雪，寒风呼啸着拍打着破旧的木窗。", "color": FG_COLOR},
            ]
        },
        {
            "text": [
                {"text": "[  3/342] ", "color": MINIMAL_PROGRESS},
                {"text": "少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。", "color": FG_COLOR},
            ]
        },
    ]
    create_screenshot("mode-minimal", minimal_lines, width=650, height=110)

    # ── Comment 模式 ──
    comment_lines = [
        {
            "text": [
                {"text": "// ", "color": COMMENT_PREFIX},
                {"text": "窗外飘着鹅毛大雪，寒风呼啸着拍打着破旧的木窗。", "color": FG_COLOR},
                {"text": " [Ch.1 | 0.6%]", "color": COMMENT_SUFFIX},
            ]
        },
        {
            "text": [
                {"text": "// ", "color": COMMENT_PREFIX},
                {"text": "少年裹紧了单薄的棉被，目光落在桌上那本泛黄的古籍。", "color": FG_COLOR},
                {"text": " [Ch.1 | 0.9%]", "color": COMMENT_SUFFIX},
            ]
        },
    ]
    create_screenshot("mode-comment", comment_lines, width=780, height=110)

    print("\n三种模式截图已生成到 assets/ 目录")

if __name__ == "__main__":
    main()
