# tread 全局安装脚本 (Windows PowerShell)
# 自动完成：Rust 检测/安装 → 镜像配置 → 编译 → 安装 → 验证
#
# 用法：
#   cd tread
#   .\install.ps1
#
# 非交互模式：
#   $env:TREAD_MIRROR="yes"; .\install.ps1
#

$ErrorActionPreference = "Stop"

# ── 颜色 ──
$HasColor = $Host.UI.SupportsVirtualTerminal
function info($msg)  { if ($HasColor) { Write-Host "[tread] $msg" -ForegroundColor Cyan } else { Write-Host "[tread] $msg" } }
function ok($msg)    { if ($HasColor) { Write-Host "[tread] ✓ $msg" -ForegroundColor Green } else { Write-Host "[tread] ✓ $msg" } }
function warn($msg)  { if ($HasColor) { Write-Host "[tread] ! $msg" -ForegroundColor Yellow } else { Write-Host "[tread] ! $msg" } }
function err($msg)   { if ($HasColor) { Write-Host "[tread] ✗ $msg" -ForegroundColor Red } else { Write-Host "[tread] ✗ $msg" } }

# ── 1. 检测 Rust ──
info "检测 Rust 工具链..."
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    warn "未检测到 Rust 工具链，开始自动安装..."
    
    # 下载 rustup-init.exe
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupExe = "$env:TEMP\rustup-init.exe"
    
    info "正在下载 rustup-init.exe..."
    try {
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe -UseBasicParsing
    } catch {
        err "下载 rustup-init 失败，请检查网络连接"
        exit 1
    }
    
    info "运行 rustup 安装器（默认参数 -y）..."
    & $rustupExe -y
    Remove-Item $rustupExe -Force -ErrorAction SilentlyContinue
    
    # 重新加载环境变量
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        err "Rust 安装后仍未找到 cargo，请重新打开 PowerShell 后重试"
        exit 1
    }
    ok "Rust 安装完成: $(cargo --version)"
} else {
    ok "Rust 已安装: $(cargo --version)"
}

# ── 2. 配置 Cargo 镜像 ──
$cargoConfig = "$env:USERPROFILE\.cargo\config.toml"
$cargoConfigLegacy = "$env:USERPROFILE\.cargo\config"

if (Test-Path $cargoConfig -or Test-Path $cargoConfigLegacy) {
    ok "Cargo 配置已存在，跳过镜像配置"
    if ((Test-Path $cargoConfigLegacy) -and -not (Test-Path $cargoConfig)) {
        warn "检测到旧格式 config，建议迁移:"
        Write-Host "  Rename-Item '$cargoConfigLegacy' 'config.toml'"
    }
} else {
    $mirrorEnv = $env:TREAD_MIRROR
    $answer = ""
    if ($mirrorEnv) {
        if ($mirrorEnv -eq "yes" -or $mirrorEnv -eq "y" -or $mirrorEnv -eq "Y") {
            $answer = "y"
        } else {
            $answer = "n"
        }
    } else {
        info "是否要配置 Cargo 国内镜像以加速编译？"
        $input = Read-Host "  [Y/n]"
        if ([string]::IsNullOrWhiteSpace($input) -or $input -eq "y" -or $input -eq "Y") {
            $answer = "y"
        } else {
            $answer = "n"
        }
    }
    
    if ($answer -eq "y") {
        New-Item -ItemType Directory -Path "$env:USERPROFILE\.cargo" -Force | Out-Null
        @"
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"
replace-with = 'ustc'

[source.ustc]
registry = "git://mirrors.ustc.edu.cn/crates.io-index"
"@ | Out-File -FilePath $cargoConfig -Encoding utf8
        ok "已配置 USTC 镜像到 $cargoConfig"
        Write-Host "  镜像源: git://mirrors.ustc.edu.cn/crates.io-index"
    } else {
        info "跳过镜像配置，使用官方 crates.io"
    }
}

# ── 3. 编译安装 tread ──
Write-Host ""
info "开始编译 tread（release 模式，首次编译可能需几分钟）..."
cargo install --path . 2>&1
ok "编译安装完成"

# ── 4. 确保 cargo bin 在 PATH 中 ──
$cargoBin = "$env:USERPROFILE\.cargo\bin"
$pathDirs = $env:Path -split ";"
if ($pathDirs -notcontains $cargoBin) {
    info "将 $cargoBin 添加到用户 PATH..."
    $currentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    [System.Environment]::SetEnvironmentVariable("Path", "$currentPath;$cargoBin", "User")
    $env:Path = "$env:Path;$cargoBin"
    ok "已添加到用户 PATH（当前会话已生效）"
} else {
    ok "cargo bin 已在 PATH 中"
}

# ── 5. 验证 ──
Write-Host ""
info "验证安装..."
$treadCmd = Get-Command tread -ErrorAction SilentlyContinue
if ($treadCmd) {
    $treadPath = $treadCmd.Source
    ok "tread 已安装到: $treadPath"
    Write-Host ""
    Write-Host "────────────────────────────"
    tread --help
    Write-Host "────────────────────────────"
    Write-Host ""
    ok "全部完成！现在可以在任意目录运行："
    Write-Host ""
    Write-Host "  tread your-novel.txt"
    Write-Host "  tread your-novel.epub"
    Write-Host "  tread your-novel.mobi --mode comment --lines 2"
    Write-Host ""
    Write-Host "如果当前终端找不到 tread，请重新打开 PowerShell 后重试。"
} else {
    err "tread 未找到，安装可能出现问题"
    Write-Host ""
    Write-Host "请检查 $cargoBin 是否存在 tread.exe，或重新打开 PowerShell。"
    exit 1
}
