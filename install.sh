#!/usr/bin/env bash
#
# tread 全局安装脚本
# 真正的一键安装：Rust → 镜像 → 编译 → 安装 → 验证
#
# 非交互模式（CI/无头环境）：
#   TREAD_MIRROR=yes ./install.sh   # 自动配置镜像
#   TREAD_MIRROR=no  ./install.sh   # 跳过镜像
#
# 用法：
#   cd /path/to/tread
#   ./install.sh
#

set -euo pipefail

# ── 检测操作系统 ──
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    err "检测到 Windows 环境，请使用 PowerShell 安装脚本："
    echo "  .\\install.ps1"
    exit 1
fi

# ── 颜色 ──
if [ -n "${NO_COLOR:-}" ]; then
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
elif [ -t 1 ] && command -v tput &> /dev/null && tput colors &> /dev/null && [ "$(tput colors)" -ge 8 ]; then
    RED=$(tput setaf 1)
    GREEN=$(tput setaf 2)
    YELLOW=$(tput setaf 3)
    BLUE=$(tput setaf 4)
    NC=$(tput sgr0)
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

info()  { printf "${BLUE}[tread]${NC} %s\n" "$1"; }
ok()    { printf "${GREEN}[tread] ✓${NC} %s\n" "$1"; }
warn()  { printf "${YELLOW}[tread] !${NC} %s\n" "$1"; }
err()   { printf "${RED}[tread] ✗${NC} %s\n" "$1"; }

# ── 1. 检测 shell，确定 rc 文件 ──
SHELL_NAME=$(basename "$SHELL")
case "$SHELL_NAME" in
    zsh)  RC_FILE="$HOME/.zshrc" ;;
    bash) RC_FILE="$HOME/.bashrc" ;;
    *)    RC_FILE="$HOME/.profile" ;;
esac
info "检测到 shell: $SHELL_NAME → rc 文件: $RC_FILE"

# ── 2. 自动安装 Rust（如未安装） ──
if ! command -v cargo &> /dev/null; then
    warn "未检测到 Rust 工具链，开始自动安装..."
    echo ""
    echo "  下载 rustup 安装器..."

    # 下载 rustup-init
    RUSTUP_URL="https://sh.rustup.rs"
    if command -v curl &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf "$RUSTUP_URL" | sh -s -- -y
    elif command -v wget &> /dev/null; then
        wget -qO- "$RUSTUP_URL" | sh -s -- -y
    else
        err "需要 curl 或 wget 来下载 Rust 安装器，请先安装其中之一。"
        exit 1
    fi

    # 加载 rustup 环境（安装后立即生效）
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env" 2>/dev/null || true

    ok "Rust 安装完成: $(cargo --version)"
else
    ok "Rust 已安装: $(cargo --version)"
fi

# ── 3. 确保 cargo 环境在 rc 文件中 ──
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

# ── 5. 配置 Cargo 镜像（国内加速） ──
CARGO_CONFIG="$HOME/.cargo/config.toml"
CARGO_CONFIG_LEGACY="$HOME/.cargo/config"

# 检查是否已配置镜像
if [ -f "$CARGO_CONFIG" ] || [ -f "$CARGO_CONFIG_LEGACY" ]; then
    ok "Cargo 配置已存在，跳过镜像配置"
    # 提示迁移旧格式的 config
    if [ -f "$CARGO_CONFIG_LEGACY" ] && [ ! -f "$CARGO_CONFIG" ]; then
        warn "检测到旧格式 $CARGO_CONFIG_LEGACY，建议迁移:"
        echo "  mv $CARGO_CONFIG_LEGACY $CARGO_CONFIG"
    fi
else
    # 支持环境变量控制非交互模式
    if [ -n "${TREAD_MIRROR:-}" ]; then
        if [ "$TREAD_MIRROR" = "yes" ] || [ "$TREAD_MIRROR" = "y" ] || [ "$TREAD_MIRROR" = "Y" ]; then
            answer="y"
        else
            answer="n"
        fi
    elif [ -t 0 ]; then
        # 有交互式 stdin，询问用户（默认否）
        info "是否要配置 Cargo 国内镜像以加速编译？"
        printf "  [y/N] "
        read -r answer || true
        if [ -z "$answer" ]; then
            answer="n"
        fi
    else
        # 无交互式 stdin，保守起见跳过镜像
        info "非交互环境，跳过 Cargo 镜像配置（如需启用请设置 TREAD_MIRROR=yes）"
        answer="n"
    fi

    if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then
        mkdir -p "$HOME/.cargo"
        cat > "$CARGO_CONFIG" << 'EOF'
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"
replace-with = 'ustc'

[source.ustc]
registry = "git://mirrors.ustc.edu.cn/crates.io-index"
EOF
        ok "已配置 USTC 镜像到 $CARGO_CONFIG"
        echo "  镜像源: git://mirrors.ustc.edu.cn/crates.io-index"
        echo "  如需更换其他镜像，可手动编辑该文件。"
    else
        info "跳过镜像配置，使用官方 crates.io"
    fi
fi

# ── 6. 编译安装 tread ──
echo ""
info "开始编译 tread（release 模式，首次编译可能需几分钟）..."
cargo install --path . 2>&1
ok "编译安装完成"

# ── 7. 确保 ~/.cargo/bin 在 PATH 中 ──
# cargo install 默认装到 ~/.cargo/bin，但如果用户 PATH 没配好可能找不到
_has_cargo_bin_in_rc=false

if [ -f "$RC_FILE" ]; then
    # 检查 rc 文件中是否已有 cargo env source 或 cargo/bin PATH 配置
    if grep -qE '(\.cargo/env|\.cargo/bin)' "$RC_FILE" 2>/dev/null; then
        _has_cargo_bin_in_rc=true
    fi
fi

if ! $_has_cargo_bin_in_rc; then
    info "将 ~/.cargo/bin 追加到 $RC_FILE"
    echo ""                                      >> "$RC_FILE"
    echo "# ── cargo bin PATH (added by tread) ──" >> "$RC_FILE"
    echo 'export PATH="$HOME/.cargo/bin:$PATH"'  >> "$RC_FILE"
    ok "已追加到 $RC_FILE"
    # 当前 shell 也立即生效
    export PATH="$HOME/.cargo/bin:$PATH"
else
    ok "$RC_FILE 中已包含 cargo bin PATH 配置"
fi

# ── 8. 验证 ──
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
    echo "  tread your-novel.epub"
    echo "  tread your-novel.mobi --mode comment --lines 2"
    echo ""
    echo "如果当前终端找不到 tread，请执行："
    echo "  source $RC_FILE"
else
    err "tread 未找到，安装可能出现问题"
    echo ""
    echo "请检查 ~/.cargo/bin 是否存在 tread 二进制，或手动执行："
    echo "  source $RC_FILE"
    exit 1
fi
