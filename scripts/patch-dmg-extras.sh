#!/bin/bash
# 给 tauri 产出的 dmg 追加「安装说明.txt + 终端快捷方式」，并把图标摆成传统布局。
#
# 为什么：应用未签名，macOS 会拦"浏览器下载"的第一次打开（提示"已损坏"）——
# 让 dmg 自己带着解法：说明文档（能直接打开）+ 系统终端的快捷方式（指向
# Apple 签名、不会被拦的系统应用），下载的人照着文档粘一条命令即可。
# ★ 刻意**不放可执行脚本**：脚本和应用一样被隔离拦下，形同虚设。
#
# 2026-11-06：作者反馈"图标乱放"。根因是 Tauri 那边没有 `bundle.dmg` 配置，
# 默认 48pt 图标 + 自动网格；这里又事后 cp 两个文件进去，Finder 就把四个一起乱排。
# 所以本脚本现在做三件事：
#   ① 拷两个辅助文件；
#   ② 铺背景图（`.background/`，Finder 用 144dpi 的 TIFF 渲染 retina）；
#   ③ 用 AppleScript 把四个图标摆到固定落点（应用左、应用程序右、两个辅助在下面）。
#
# ★ 布局的 AppleScript 必须**挂在 /Volumes 下**跑：Finder 只认挂载点上的卷才会写
#   `.DS_Store`。早先挂在临时目录里，`set background picture` 不报错但什么也没存下。
#
# 用法：scripts/patch-dmg-extras.sh <dmg 路径>
# 前提：macOS（hdiutil + Finder）。布局失败只告警，不让一次发版因为外观挂掉。
set -euo pipefail

DMG="$1"
EXTRAS="$(cd "$(dirname "$0")/../src-tauri/installer-extras" && pwd)"
WORK="$(mktemp -d)"
MNT=""

cleanup() {
  if [ -n "$MNT" ]; then hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true; fi
  rm -rf "$WORK"
}
trap cleanup EXIT

# UDZO（压缩只读）写不进去 —— 先转成可写，装完 extras 再转回去
hdiutil convert "$DMG" -format UDRW -o "$WORK/rw.dmg" -quiet
# **不加 -nobrowse、不指定 mountpoint、也不加 -quiet**：要让它挂在 /Volumes 下
# （Finder 才接管），而且挂载点是从 hdiutil 的输出里读出来的 —— 加 -quiet 就没得读了。
# ★ 用**制表符**切字段：挂载点里可能有空格（重名时会是 `/Volumes/SupportEase 2`），
#   按空白切 `$NF` 只会拿到最后那个 "2"，接着 `cp` 就报 "cp: 2: No such file or directory"。
MNT="$(hdiutil attach "$WORK/rw.dmg" | awk -F'\t' '/\/Volumes\// {print $NF; exit}')"
[ -n "$MNT" ] || { echo "挂载失败：$DMG" >&2; exit 1; }
VOLNAME="$(basename "$MNT")"

cp "$EXTRAS/安装说明.txt" "$MNT/"

# 「终端」快捷方式：指向系统自带的 Terminal.app（Apple 签名，不会被拦）
# ★ 先删掉同名项再建：这个脚本会**重跑**（已经补过包的 dmg 再打一遍），
#   而 `ln -s` 撞上"已存在的符号链接指向目录"时会往**目标目录里**再建一个链接
#   —— 报 "Operation not permitted"，看上去像权限问题，其实是钻进了链接。
rm -f "$MNT/终端"
for term in "/System/Applications/Utilities/Terminal.app" "/Applications/Utilities/Terminal.app"; do
  if [ -d "$term" ]; then
    ln -s "$term" "$MNT/终端"
    break
  fi
done

# 背景：PNG 给老 Finder 兜底，TIFF 带 144dpi ⇒ retina 下不糊
mkdir -p "$MNT/.background"
cp "$EXTRAS/dmg-background.png" "$MNT/.background/background.png"
cp "$EXTRAS/dmg-background.tiff" "$MNT/.background/background.tiff"

# 布局（坐标与 `installer-extras/dmg-background.svg` 里画的箭头一致，改一处就要改两处）
if ! osascript <<APPLESCRIPT
on run
	tell application "Finder"
		tell disk "$VOLNAME"
			open
			set x0 to 200
			set y0 to 140
			set w to 660
			set h to 458
			tell container window
				set current view to icon view
				set toolbar visible to false
				set statusbar visible to false
				set the bounds to {x0, y0, x0 + w, y0 + h}
			end tell
			set opts to the icon view options of container window
			tell opts
				set icon size to 96
				set text size to 13
				set arrangement to not arranged
			end tell
			try
				set background picture of opts to file ".background:background.tiff"
			on error
				set background picture of opts to file ".background:background.png"
			end try
			-- ★ 两者**同一 y**：Finder 的标签跟着图标框走，单独抬图标会让标签差一截；
			--   要对齐就对齐"标签"，图标靠同 y 自然齐（2026-11-06 作者反馈）
			set position of item "SupportEase.app" of container window to {210, 132}
			set position of item "Applications" of container window to {450, 132}
			set position of item "安装说明.txt" of container window to {253, 300}
			set position of item "终端" of container window to {407, 300}
			close
			open
			delay 1
			tell container window
				set statusbar visible to false
				set the bounds to {x0, y0, x0 + w - 10, y0 + h - 10}
			end tell
		end tell
		delay 1
		tell disk "$VOLNAME"
			tell container window
				set statusbar visible to false
				set the bounds to {x0, y0, x0 + w, y0 + h}
			end tell
		end tell
		-- 等 Finder 真的把 .DS_Store 写出来，最多 15s
		set waited to 0
		repeat while waited < 15
			delay 1
			set waited to waited + 1
			if (do shell script "[ -f " & quoted form of ("$MNT/.DS_Store") & " ]; echo \$?") = "0" then exit repeat
		end repeat
		tell disk "$VOLNAME" to close
	end tell
end run
APPLESCRIPT
then
  echo "::warning::dmg 布局脚本没跑通（应用/说明都在，只是图标位置与背景可能没生效）"
fi

sync
sleep 2
hdiutil detach "$MNT" -quiet
MNT=""

hdiutil convert "$WORK/rw.dmg" -format UDZO -o "$WORK/final.dmg" -quiet
mv "$WORK/final.dmg" "$DMG"
echo "dmg 已追加：安装说明.txt、终端快捷方式、背景与固定布局"
