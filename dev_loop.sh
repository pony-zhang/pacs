#!/bin/bash

# 自动化开发循环脚本
# 用于持续运行 claude 开发助手

set -e

# 配置参数
PROMPT_FILE="./prompt.md"
LOOP_DELAY=10  # 每次循环间隔（秒）
MAX_LOOPS=50   # 最大循环次数
LOG_FILE="./dev_loop.log"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "$LOG_FILE"
}

info() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')] INFO:${NC} $1" | tee -a "$LOG_FILE"
}

# Git 初始化（仅首次）
init_git_if_needed() {
    if [ ! -d ".git" ]; then
        log "检测到未初始化 Git 仓库，正在初始化..."
        git init -q
        git add .
        local commit_msg="Auto-commit: $(date '+%Y-%m-%d %H:%M:%S') [Initial]"
        git commit -m "$commit_msg" -q
        log "Git 仓库已初始化并完成首次提交 ✓"
    else
        log "Git 仓库已存在，跳过初始化"
    fi
}

# 自动提交当前更改
auto_commit() {
    # 检查是否有更改
    if ! git diff --quiet || ! git diff --cached --quiet; then
        git add .
        local commit_msg="Auto-commit: $(date '+%Y-%m-%d %H:%M:%S')"
        git commit -m "$commit_msg" -q
        log "自动提交完成: $commit_msg"
    else
        log "无文件变更，跳过提交"
    fi
}

# 检查必要文件
check_prerequisites() {
    log "检查运行环境..."

    if [ ! -f "$PROMPT_FILE" ]; then
        error "提示文件 $PROMPT_FILE 不存在！"
        exit 1
    fi

    if [ ! -d "docs" ]; then
        warn "docs 目录不存在，正在创建..."
        mkdir -p docs
    fi

    # 检查 claude 命令是否可用
    if ! command -v claude &> /dev/null; then
        error "claude 命令不可用，请确保已正确安装 Claude Code"
        exit 1
    fi

    # 检查 Git 是否可用
    if ! command -v git &> /dev/null; then
        error "git 命令不可用，请安装 Git"
        exit 1
    fi

    log "环境检查完成 ✓"
}

# 执行 claude 命令
run_claude() {
    local loop_num=$1
    log "开始第 $loop_num 轮开发循环..."

    info "执行命令: claude --dangerously-skip-permissions -p $PROMPT_FILE"

    # 执行 claude 命令并记录输出
    local start_time=$(date +%s)

    if claude --dangerously-skip-permissions -p "$PROMPT_FILE" 2>&1 | tee -a "$LOG_FILE"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log "第 $loop_num 轮开发完成，耗时: ${duration}秒 ✓"

        # 记录成功状态
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Loop $loop_num: SUCCESS (${duration}s)" >> "$LOG_FILE.success"
    else
        error "第 $loop_num 轮开发失败！"
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Loop $loop_num: FAILED" >> "$LOG_FILE.error"
    fi
}

# 显示统计信息
show_stats() {
    log "=== 开发循环统计 ==="

    local success_count=0
    local error_count=0

    if [ -f "$LOG_FILE.success" ]; then
        success_count=$(wc -l < "$LOG_FILE.success")
    fi

    if [ -f "$LOG_FILE.error" ]; then
        error_count=$(wc -l < "$LOG_FILE.error")
    fi

    info "成功循环: $success_count"
    info "失败循环: $error_count"
    info "总循环数: $((success_count + error_count))"

    if [ -d "docs" ]; then
        local doc_count=$(find docs -name "*.md" | wc -l)
        info "文档文件数: $doc_count"
    fi
}

# 清理函数
cleanup() {
    log "接收到中断信号，正在清理..."
    show_stats
    log "开发循环已停止"
    exit 0
}

# 主循环函数
main_loop() {
    log "开始自动化开发循环..."
    log "配置: 最大循环次数=$MAX_LOOPS, 循环间隔=${LOOP_DELAY}秒"
    log "日志文件: $LOG_FILE"

    # 设置信号处理
    trap cleanup SIGINT SIGTERM

    # 初始化 Git（仅一次）
    init_git_if_needed

    for ((i=1; i<=MAX_LOOPS; i++)); do
        log "=========================================="
        log "启动第 $i/$MAX_LOOPS 轮开发循环"

        run_claude "$i"

        # 每轮结束后自动提交
        auto_commit

        if [ $i -lt $MAX_LOOPS ]; then
            info "等待 ${LOOP_DELAY}秒后开始下一轮..."
            sleep "$LOOP_DELAY"
        fi
    done

    log "=========================================="
    log "所有 $MAX_LOOPS 轮开发循环已完成！"
    show_stats
}

# 显示帮助信息
show_help() {
    echo "自动化开发循环脚本（带 Git 自动提交）"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -d, --delay SECONDS 设置循环间隔（默认: 10秒）"
    echo "  -l, --loops COUNT   设置最大循环次数（默认: 50）"
    echo "  -s, --stats         只显示统计信息"
    echo "  -c, --clean         清理日志文件"
    echo ""
    echo "示例:"
    echo "  $0                  # 使用默认配置运行"
    echo "  $0 -d 60 -l 10      # 60秒间隔，最多10次循环"
    echo "  $0 -s               # 显示统计信息"
    echo "  $0 -c               # 清理日志文件"
}

# 清理日志文件
clean_logs() {
    log "清理日志文件..."
    rm -f "$LOG_FILE" "$LOG_FILE.success" "$LOG_FILE.error"
    log "日志文件已清理 ✓"
}

# 解析命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -d|--delay)
            LOOP_DELAY="$2"
            shift 2
            ;;
        -l|--loops)
            MAX_LOOPS="$2"
            shift 2
            ;;
        -s|--stats)
            show_stats
            exit 0
            ;;
        -c|--clean)
            clean_logs
            exit 0
            ;;
        *)
            error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 验证参数
if ! [[ "$LOOP_DELAY" =~ ^[0-9]+$ ]] || [ "$LOOP_DELAY" -lt 1 ]; then
    error "循环间隔必须是正整数"
    exit 1
fi

if ! [[ "$MAX_LOOPS" =~ ^[0-9]+$ ]] || [ "$MAX_LOOPS" -lt 1 ]; then
    error "最大循环次数必须是正整数"
    exit 1
fi

# 启动脚本
log "自动化开发循环脚本启动"
log "工作目录: $(pwd)"
check_prerequisites
main_loop
