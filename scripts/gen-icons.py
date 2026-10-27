#!/usr/bin/env python3
"""
把作者的 SVG 母版套上 **macOS squircle** 蒙版，导出 1024 母版。

# 为什么要有这个脚本

Tauri 默认生成的图标用的是**普通圆角**（border-radius），而 macOS Big Sur 起规范是
**squircle 超椭圆**（连续曲率）。作者 2026-10-15：

> 我的 Dock 栏有图标呀，但是那个图标我感觉它的圆角好像不自然，没有平滑过渡呢…
> 人家的好像都有平滑过渡的

肉眼差别：普通圆角在「直边 → 圆弧」处有一个**转折点**，squircle 是顺滑弯进去的。
Dock 那种 64~128px 的尺寸下并排看，差别一眼可见。

# 输入 / 输出

- 输入 = `src-tauri/icons/source/icon-source.svg`（**作者提供的母版**，1024×1024、
  方角满幅；图形与配色都在那儿，这个脚本**一个字都不改它**）。
- 输出 = `src-tauri/icons/source/icon-1024.png`（套好 squircle 的 1024 母版）。

# 用法

    python3 scripts/gen-icons.py
    npx tauri icon src-tauri/icons/source/icon-1024.png

第二条让 Tauri 自己栅格化各尺寸（`icon.icns` / `icon.ico` 各档都由官方工具生成，
**不手算**）。要改图形改 SVG；要改圆角改这个脚本的 `SQUIRCLE_N`。别再手改 `icons/*.png`。

# 依赖

`cairosvg`（把 SVG 渲染成位图）。缺了它就报错提示怎么装 —— **不退化**：
用 PIL 模拟 SVG 渲染会丢掉 vector 的锐利边缘（我们正在修的恰恰是"边缘"这件事）。
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"
SRC_SVG = ICONS / "source" / "icon-source.svg"
OUT = ICONS / "source" / "icon-1024.png"          # macOS / 通用（squircle）
OUT_WIN = ICONS / "source" / "icon-1024-win.png"  # Windows（方角满幅，图形更大）

CANVAS = 1024
INSET = 100                # macOS 官方口径：内容区四周各留 100
SIDE = CANVAS - INSET * 2  # ⇒ 内容区 824×824
SQUIRCLE_N = 5.0           # 超椭圆指数：2 = 正圆、∞ = 方角，5 最像 Apple 那条

# ★★★ 母版是「方角满幅、图形顶边」的画法（作者那份 SVG 从 x=238 到 x=795，
# 竖直方向几乎到顶到边）。直接套 squircle 会把**四角连图形一起切掉** ——
# 第一版就是这样，笔尖戳出下边、锯片齿咬到右边。
# ⇒ 先把整幅（底 + 图形一起）缩到 `ART_SCALE`，再套蒙版。缩的是**整幅**而不是只缩图形：
#   底板必须跟着缩，否则缩完图形会比底板小、四周露一圈透明缝。
#
# ★★★ 2026-10-16：**0.86 → 0.78**。作者：「你是不是应该帮我把这个中间的这个图案再缩小一点呢？
# 我感觉它现在离那个边缘好靠近啊」。
#
# 量过才改的（母版 1024）：图形**横向只占 48%**、**纵向占 66.7%** —— 因为「笔 + 锯片」
# 是个**细长斜放**的组合，纵向才是它顶到边的那一头（笔帽在上、笔尖在下）。
# 0.86 时笔画离上 164 / 离下 177（在 824 的内容区里各占 2 成），看着就是"上下都贴边"。
# 0.78 之后这两处余量提到约 2.6 成，与 Apple 自家图标（图形大致占内容区 6~7 成、
# 四周留一圈明显的空）同量级。
ART_SCALE = 0.78

# ★★★ 2026-10-16：**Windows 要单独一档**（作者：「我在 Windows 上那个图标小了一圈」）。
#
# 原因不是"图形占比算错了"，而是**两个平台的显示机制不同**：
#   · macOS 把我们给的方图**裁成 squircle** ⇒ 四角被吃掉，图形显得"顶满"；
#   · Windows **原样方角铺满**（任务栏 / 桌面快捷方式 / 标题栏）⇒ 同一份图里
#     图形四周那圈留白在 Windows 上**看得见**，于是"小了一圈"。
# ⇒ 同一份母版给两边用必然有一边不合身。Windows 那支要**放大**：图形占满到
#   `WIN_ART_SCALE`（0.94，几乎顶边 —— Windows 的图标本来就不留那么多边）。
# ★ 而且 Windows **不套 squircle**（方角满幅才是那边的规范；套了圆角在任务栏上
#   会显得"缺角"，那不是 Windows 图标的样子）。
WIN_ART_SCALE = 0.94


def squircle_mask(size: int) -> Image.Image:
    """
    超椭圆蒙版（|x/a|^n + |y/a|^n ≤ 1，居中在 824 的内容区里，四周留 100 透明）。

    ★ 用公式而不是 `ImageDraw.rounded_rectangle`：后者是**圆弧**，正是作者说
    "不自然、没有平滑过渡"的那个形状。超椭圆的曲率连续，没有转折点。
    ★ 逐行 `d.line` 而不是逐像素 `putpixel`：1024² 逐点太慢，而且这条式子每行
    只需解一次（对称，左右同时铺）。
    """
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    a = SIDE / 2
    c = size / 2
    for py in range(size):
        t = abs((py + 0.5 - c) / a)
        if t >= 1:
            continue
        xr = a * (1 - t**SQUIRCLE_N) ** (1 / SQUIRCLE_N)
        d.line([(c - xr, py), (c + xr, py)], fill=255)
    return mask


def render_svg() -> Image.Image:
    """把母版 SVG 渲染成 1024 的 RGBA 位图。"""
    try:
        import cairosvg
    except ImportError:
        print(
            "缺 cairosvg —— 它负责把母版 SVG 渲染成位图。\n"
            "  macOS:  brew install cairo  &&  pip3 install cairosvg\n"
            "（**不用 sips 退化**：我们要修的正是「边缘」，而 sips 的 svg 支持会丢锐利度）",
            file=sys.stderr,
        )
        raise SystemExit(1)

    png = cairosvg.svg2png(url=str(SRC_SVG), output_width=CANVAS, output_height=CANVAS)
    if png is None:
        print("SVG 渲染失败", file=sys.stderr)
        raise SystemExit(1)
    import io

    return Image.open(io.BytesIO(png)).convert("RGBA")


def scaled_art(art: Image.Image, scale: float) -> Image.Image:
    """
    把「底 + 图形」整幅缩到 `scale`，四周用**母版的底板色**补满。

    ★ 缩的是整幅而不是只缩图形：底板必须跟着缩，否则缩完图形会比底板小、
    四周露一圈透明缝。缩完空出来的地方用母版左上角那个色补上。
    """
    if scale == 1.0:
        return art
    side = round(CANVAS * scale)
    small = art.resize((side, side), Image.LANCZOS)
    spread = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    spread.paste(small, ((CANVAS - side) // 2, (CANVAS - side) // 2))
    plate = art.getpixel((4, 4))
    return Image.alpha_composite(Image.new("RGBA", (CANVAS, CANVAS), plate), spread)


def main() -> int:
    if not SRC_SVG.exists():
        print(f"找不到母版 {SRC_SVG}", file=sys.stderr)
        return 1

    art = render_svg()
    if art.size != (CANVAS, CANVAS):
        art = art.resize((CANVAS, CANVAS), Image.LANCZOS)

    OUT.parent.mkdir(parents=True, exist_ok=True)

    # ① macOS / 通用：套 squircle（方图会被 AppKit 再裁一次，所以自己先裁好）
    mask = squircle_mask(CANVAS)
    mac = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    mac.paste(scaled_art(art, ART_SCALE), (0, 0), mask)
    mac.save(OUT)
    print(f"[图标] macOS 版已写出 {OUT}（squircle，图形占 {ART_SCALE:.0%}）")

    # ② Windows：**方角满幅、不套蒙版**，图形也放大一档（见 WIN_ART_SCALE 那段）
    win = scaled_art(art, WIN_ART_SCALE).convert("RGB")  # ico 不吃 alpha 底，转 RGB 更稳
    win.save(OUT_WIN)
    print(f"[图标] Windows 版已写出 {OUT_WIN}（方角满幅，图形占 {WIN_ART_SCALE:.0%}）")

    print("下一步：npx tauri icon src-tauri/icons/source/icon-1024.png")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
