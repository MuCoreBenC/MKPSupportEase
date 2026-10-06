#!/bin/bash
# 给 tauri 产出的 dmg 追加「安装说明.txt + 终端快捷方式」。
#
# 为什么：应用未签名，macOS 会拦"浏览器下载"的第一次打开（提示"已损坏"）——
# 让 dmg 自己带着解法：说明文档（能直接打开）+ 系统终端的快捷方式（指向
# Apple 签名的系统应用，也不会被拦），下载的人照着文档粘一条命令即可。
# ★ 刻意**不放可执行脚本**：脚本和应用一样被隔离拦下，形同虚设。
#
# 用法：scripts/patch-dmg-extras.sh <dmg 路径>
# 前提：macOS（hdiutil）。
set -euo pipefail

DMG="$1"
EXTRAS="$(cd "$(dirname "$0")/../src-tauri/installer-extras" && pwd)"
WORK="$(mktemp -d)"
MNT="$WORK/mnt"
cleanup() {
  hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# UDZO（压缩只读）写不进去 —— 先转成可写，装完 extras 再转回去
hdiutil convert "$DMG" -format UDRW -o "$WORK/rw.dmg" -quiet
mkdir -p "$MNT"
hdiutil attach "$WORK/rw.dmg" -mountpoint "$MNT" -nobrowse -quiet
cp "$EXTRAS/安装说明.txt" "$MNT/"

# 「终端」快捷方式：指向系统自带的 Terminal.app（Apple 签名，不会被拦）
for term in "/System/Applications/Utilities/Terminal.app" "/Applications/Utilities/Terminal.app"; do
  if [ -d "$term" ]; then
    ln -s "$term" "$MNT/终端"
    break
  fi
done

hdiutil detach "$MNT" -quiet
hdiutil convert "$WORK/rw.dmg" -format UDZO -o "$WORK/final.dmg" -quiet
mv "$WORK/final.dmg" "$DMG"
echo "dmg 已追加：安装说明.txt、终端快捷方式"
