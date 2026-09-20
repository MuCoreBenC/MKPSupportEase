#!/bin/sh
# 闸门共用函数。被 pre-commit / pre-merge-commit / pre-push source 进来。
#
# 纯 POSIX sh，不依赖 node —— 钩子在任何 git 客户端里都要能跑，
# 不能假设 PATH 里有 node（GUI 客户端经常没有）。

GIT_DIR_ABS=$(git rev-parse --absolute-git-dir 2>/dev/null)
RELEASE_FLAG="$GIT_DIR_ABS/RELEASE_IN_PROGRESS"
BYPASS_LOG="$GIT_DIR_ABS/bypass.log"

# 用了逃生开关就往 .git/bypass.log 记一行。
# .git/ 不入库，所以这是本机痕迹；让它真的有用的是 npm run release 的前置检查 ——
# 发版时会把上次 tag 之后的绕过记录当着你的面复述一遍。
log_bypass() {
  [ -z "$GIT_DIR_ABS" ] && return 0
  printf '%s  %s  branch=%s  head=%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "$1" \
    "$(git symbolic-ref --short HEAD 2>/dev/null || echo '(detached)')" \
    "$(git rev-parse --short HEAD 2>/dev/null || echo '(none)')" \
    >>"$BYPASS_LOG" 2>/dev/null
  echo "  （已记入 .git/bypass.log，下次 npm run release 会复述）" >&2
}

# 发版流程中 release.mjs 会写这个标记，让闸①/①b 放行 main 上的提交与合并。
# 它只在脚本执行窗口内存在，脚本的 finally 负责删除 —— 比常开一个环境变量安全。
in_release() {
  [ -f "$RELEASE_FLAG" ]
}

# 从 package.json 读 version。不用 node、不用 jq，只用 sed。
# 取第一个 "version": "x.y.z"，package.json 里 dependencies 的版本号不会长这样（没有 version 键）。
pkg_version() {
  root=$(git rev-parse --show-toplevel 2>/dev/null) || return 1
  [ -f "$root/package.json" ] || return 1
  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$root/package.json" | head -n 1
}
