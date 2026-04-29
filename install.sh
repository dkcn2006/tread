#!/usr/bin/env bash
#
# tread 全局安装脚本
# 自动完成：PATH 配置 → 编译 → 安装 → 验证
#
# 用法：
#   cd /path/to/tread
#   ./install.sh
#

set -euo pipefail

# ── 颜色 ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}[tread]${NC} $1"; }
ok()    { echo -e "${GREEN}[tread] ✓${NC} $1"; }
warn()  { echo -e "${YELLOW}[tread] !${NC} $1"; }
err()   { echo -e "${RED}[tread] ✗${NC} $1"; }

# ── 1. 检测 shell，确定 rc 文件 ──
SHELL_NAME=$(basename "$SHELL")
case "$SHELL_NAME" in
    zsh)  RC_FILE="$HOME/.zshrc" ;;
    bash) RC_FILE="$HOME/.bashrc" ;;
    *)    RC_FILE="$HOME/.profile" ;;
esac
info "检测到 shell: $SHELL_NAME → rc 文件: $RC_FILE"

# ── 2. 检查 Rust 是否已安装 ──
if ! command -v cargo &> /dev/null; then
    echo ""
    err "未检测到 Rust 工具链（cargo）"
    echo ""
    echo "请先安装 Rust，推荐方式："
    echo ""
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "安装完成后，重新打开终端或执行："
    echo "  source \$HOME/.cargo/env"
    echo ""
    echo "然后再运行此脚本。"
    exit 1
fi

CARGO_VERSION=$(cargo --version)
ok "Rust 已安装: $CARGO_VERSION"

# ── 3. 持久化 PATH（将 cargo 加入 rc 文件） ──
CARGO_ENV_LINE='. "$HOME/.cargo/env"'

if [ -f "$HOME/.cargo/env" ]; then
    if ! grep -qF "$CARGO_ENV_LINE" "$RC_FILE" 2>/dev/null; then
        info "将 cargo 环境配置追加到 $RC_FILE"
        echo ""                              >> "$RC_FILE"
        echo "# ── Rust toolchain (added by tread) ──" >> "$RC_FILE"
        echo "$CARGO_ENV_LINE"               >> "$RC_FILE"
        ok "已写入 $RC_FILE，以后新终端自动可用"
    else
        ok "$RC_FILE 中已包含 cargo 配置"
    fi
else
    warn "~/.cargo/env 不存在，PATH 持久化可能不完整"
fi

# ── 4. 当前 shell 立刻生效 ──
info "当前 shell 加载 cargo 环境..."
# shellcheck source=/dev/null
. "$HOME/.cargo/env" 2>/dev/null || true
ok "环境已生效"

# ── 5. 编译安装 tread ──
echo ""
info "开始编译 tread（release 模式，首次编译可能需几分钟）..."
cargo install --path . 2>&1
ok "编译安装完成"

# ── 6. 验证 ──
echo ""
info "验证安装..."
if command -v tread &> /dev/null; then
    TREAD_PATH=$(command -v tread)
    ok "tread 已安装到: $TREAD_PATH"
    echo ""
    echo "────────────────────────────"
    tread --help
    echo "────────────────────────────"
    echo ""
    ok "全部完成！现在可以在任意目录运行："
    echo ""
    echo "  tread your-novel.txt"
    echo "  tread your-novel.txt --mode comment --lines 2"
else
    err "tread 未找到，可能安装路径不在 PATH 中"
    echo ""
    echo "请手动运行："
    echo "  source $RC_FILE"
    echo "然后重试此脚本。"
    exit 1
fi
