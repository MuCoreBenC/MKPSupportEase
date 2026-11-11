#!/usr/bin/env bash
# 构建产物的定期清理（2026-11-11）
#
# ── 为什么会涨 ────────────────────────────────────────────────
# `target/debug` 曾经长到 **717 GiB**（`du` 只报 233G，APFS 稀疏文件让它少报）。
# 三个来源，都在 Rust 的正常行为里，不会自己回收：
#
#   1. `deps/`：**每跑一次 `cargo test` 就链接一个新的测试二进制**（100MB+/个），
#      旧 hash 的那份留着不动。改二十轮就攒二十个。
#   2. `incremental/`：增量编译缓存，每改一次攒一份，只增不减。
#   3. `build/`：编译脚本（`build.rs`）的产物，每个依赖一份。
#
# 涨到那个量级的后果不是"占磁盘"，是**每次链接要在 GB 级符号里做重定位** ——
# 那才是"改一行等两分钟"的真正来源。
#
# ── 规则 ─────────────────────────────────────────────────────
# 每次都是幂等的，随时可以跑：
#
#   * 3 天没动过的测试二进制（无扩展名、可执行）→ 删。它们对应的源码早变了，
#     留着也不会被复用。
#   * 3 天没动过的 `.dSYM` → 删（只影响符号化栈回溯）。
#   * `incremental/` 超 20G → 清空。cargo 会重建，代价是下一次编译多几秒。
#   * 整个 `target` 超 60G → 打印一句，提示可以 `cargo clean`。
#
# 用法：`./scripts/clean-target.sh`（或 `npm run clean:target`）
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=target/debug
[ -d "$TARGET" ] || { echo "还没有 target/，不用清"; exit 0; }

kb_of() { du -sk "$1" 2>/dev/null | awk '{print $1}'; }
mb() { awk -v k="$1" 'BEGIN { printf "%.1f GB", k/1048576 }'; }

before=$(kb_of "$TARGET")
echo "清理前：$(mb "$before")"

# 1) 旧的测试二进制：无扩展名、可执行、超过 3 天没动
n=0
while IFS= read -r -d '' f; do
  rm -f "$f"
  n=$((n + 1))
done < <(find "$TARGET/deps" -maxdepth 1 -type f -perm -111 -mtime +3 -print0 2>/dev/null)
[ "$n" -gt 0 ] && echo "  删掉 $n 个旧测试二进制"

# 2) 旧的 .dSYM
d=0
while IFS= read -r -d '' dir; do
  rm -rf "$dir"
  d=$((d + 1))
done < <(find "$TARGET" -maxdepth 3 -name "*.dSYM" -mtime +3 -print0 2>/dev/null)
[ "$d" -gt 0 ] && echo "  删掉 $d 个 .dSYM"

# 3) incremental 超过 20G 就清空
inc_kb=$(kb_of "$TARGET/incremental" || echo 0)
if [ "${inc_kb:-0}" -gt 20971520 ]; then
  rm -rf "$TARGET/incremental"
  echo "  incremental 已占 $(mb "$inc_kb")，清空（下次编译会重建）"
fi

after=$(kb_of "$TARGET")
echo "清理后：$(mb "$after")"

# 4) 总体还是太大就提醒（不自动删：那会让人以为下次构建坏了）
if [ "$after" -gt 62914560 ]; then
  echo
  echo "  还大于 60G。要彻底重来就：cargo clean（下次全量重编，约 10 分钟）"
fi
