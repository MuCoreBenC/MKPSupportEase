# 资产瘦身（asset-slimdown）

把 `public/` 里的冗余图片清掉、按品牌归档、PNG 转 WebP、品牌 logo 内联成可换色的 data URI，
最后重写 Git 历史真正回收已推送到 GitHub 的旧 blob 空间。

---

## 一、三个疑问的直接回答

### 1. 项目推到云端了吗？

推了。

```
origin  https://github.com/MuCoreBenC/MKPSupportEase.git (fetch/push)
## main...origin/main   （无 ahead/behind）
```

**这决定了清理策略。** 一次普通的 `git rm` + commit 只是在历史顶端加一条"删除"记录，
被删文件的 blob 仍然完整躺在 `.git/objects` 和 GitHub 的仓库里。
仓库体积**不会降**，反而多一条提交。要真正回收，必须重写历史（见第五节 Task 6）。

### 2. 为什么 `dist` 里有一模一样的文件？`public` 为什么没被忽略？

不是重复放置，是**源 → 产物**的关系：

| 目录 | 角色 | Git 状态 | 为什么 |
|---|---|---|---|
| `public/` | **源资产**。你手工放进去的图 | **必须入库** | 忽略了，别人 clone 下来构建就没图 |
| `dist/` | **构建产物**。Vite 把 `public/` 原样拷贝 + 打包 JS | **已忽略**（`.gitignore:3`） | 产物可随时由源重新生成，入库没意义 |

`vite.config.ts` 没有配 `base` 也没有改 `publicDir`，所以 `public/` 在开发时以 `/` 为根被服务，
构建时**逐字节拷进 `dist/`**。`src-tauri/tauri.conf.json:7` 的 `frontendDist: "../dist"` 再把 `dist/` 打进安装包。

结论：`dist/` 的那份副本**完全不占仓库空间**，无需处理。本方案只动 `public/` 和引用它的代码。

> 附带后果（重要）：`public/` 下的文件**不经过 Vite 处理** —— 不加 hash、不被打包、不自动转格式。
> 所以改名或移动任何一个都会在运行时静默 404。这就是下面每一步都必须同步改代码引用的原因。

### 3. 能省多少？

`public/` 当前 18 个文件，合计 **1171.9 KB**：

| 文件 | 体积 | 处置 |
|---|---:|---|
| `printer-hero.png` | 260.2 KB | → 转 WebP（1074×1251） |
| `models/test_models.glb` | 248.5 KB | **删**（死代码） |
| `models/hero_fishtail@2x.webp` | 192.7 KB | **删** |
| `models/hero_fishtail.webp` | 75.9 KB | **删** |
| `a1.webp` | 93.5 KB | 移到 `printers/bambu/` |
| `a1mini.webp` | 82.5 KB | 移到 `printers/bambu/` |
| `models/hero_pile@2x.webp` | 73.2 KB | 保留（PageHome / PageCalib 在用） |
| `p1s.webp` | 41.8 KB | 移到 `printers/bambu/` |
| `models/hero_pile.webp` | 32.1 KB | 保留 |
| `models/plate_1..8.png` | 68.5 KB | **删**（8 个文件） |
| `bambulab.svg` | 3.0 KB | 内联进 TS，`public/` 下删除 |

- 直接删除：**585.6 KB**
- `printer-hero.png` → WebP 预计 **-190 KB** 左右（同画质 q82）
- 完工后 `public/` ≈ **393 KB**，降幅约 **66%**
- Git 历史重写后 `.git` 的实际降幅在 Task 6 用 `git count-objects -vH` 前后对比给出

---

## 二、现状清点：每张图谁在用

来源为全仓扫描（`src/**`、`index.html`、`src-tauri/**`、所有 CSS 与 CSS Module）。
**所有路径都是字面量字符串**，没有模板拼接、没有 `import.meta.glob`、没有 `new URL()`，
也没有任何一处走 ESM `import` —— 全部是 `public` 根绝对路径。

### 打印机大图（唯一的资产路径模块）

`src/app/heroArt.ts:22-31`

```ts
export const BRAND_ART: Record<string, string> = {
  bambu: '/bambulab.svg',
}

export const MODEL_ART: Record<string, ModelArt> = {
  a1: { plain: '/a1.webp' },
  a1mini: { plain: '/a1mini.webp', withVariant: '/printer-hero.png' },
  p1s: { plain: '/p1s.webp' },
  // a2l / p2s / x1c 暂缺图，自动回落到品牌 logo
}
```

消费链路：`PageHome.tsx:183` → `pickArt(sel)`（`heroArt.ts:34`，回落链 版本图 → 整机图 → 品牌 logo → null）
→ `useArtLayers(art)` → `HeroFade.tsx:60` 的 `src={layer.src}`，
且 `HeroFade.tsx:65` 有 `onError={() => onDrop(layer.id)}` —— 图挂了就把这一层丢掉，不会白屏。

### `models/hero_pile*`（保留，不动）

- `src/app/pages/PageHome.tsx:480-481`
- `src/app/pages/PageCalib.tsx:299-300`

两处都是 `src` + `srcSet="... 1x, ...@2x 2x"`。

### `models/hero_fishtail*` 与 `test_models.glb`（要删）

| 引用点 | 说明 |
|---|---|
| `ExplodedHero.tsx:93` | `'/models/test_models.glb'`，three.js GLTFLoader 加载 |
| `ExplodedHero.tsx:341-342` | WebGL 初始化失败时的兜底 `<img>`，用 fishtail 1x/2x |
| `src/api/mock.ts:111` | `testModels` 第 3 条的缩略图 |
| `src-tauri/src/ipc/mod.rs:173` | 同一张表的 Rust 镜像 |

**`ExplodedHero` 从未被挂载。** 全仓搜索 `ExplodedHero` 只命中它自己的 3 行（组件定义、
CSS Module import、内部一条 `console.warn`），没有任何页面 import 它。
连带地，`three@0.186.0` 与 `three/addons` 也**只有这个文件在用**。

### `models/plate_1..8.png`（要删）

| 图 | TS `src/api/mock.ts` | Rust `src-tauri/src/ipc/mod.rs` |
|---|---|---|
| plate_1 | :91 | :171 |
| plate_2 | :101 | :172 |
| plate_3 | **无人引用**（文件在，第 3 槽用的是 fishtail） | — |
| plate_4..8 | :121/:131/:141/:151/:161 | :174..:178 |

这些字符串目前**渲染不到屏幕上**：`getTestModels` 没有任何 UI 调用方
（只有 `contract.ts:79` 声明、`bridge.ts:66` 转发、`mock.ts:196` 实现），
而 `PageCalib.tsx:292-295` 的注释已经写明八张走马灯大卡被换成了一张合影：

> 原来这里是八张走马灯大卡（编号、标签、缩略图、用时 / 耗材、每张一个按钮）——
> 可八张卡打开的是同一个 3mf（TEST_MODEL_ID）…… 这一屏要做的决定只有一个：要不要打开。

所以 `testModels` 整条链路是走马灯下线后留下的遗留死数据。

---

## 三、目标产物形态

```
public/
├── printers/
│   └── bambu/
│       ├── a1.webp
│       ├── a1mini.webp
│       ├── a1mini-variant.webp   ← printer-hero.png 转换并改名而来
│       └── p1s.webp
└── models/
    ├── hero_pile.webp
    └── hero_pile@2x.webp

src/app/assets/bambuLogo.ts       ← 新增：参数化颜色的 logo data URI
```

`printer-hero.png` 顺手改名为 `a1mini-variant.webp`：它的语义是"a1mini 装了快拆件的整机"
（`heroArt.ts:28` 的 `withVariant`），`printer-hero` 这个名字既不说明机型也不说明变体。
既然文件已经要动，一次改到位，不留第二次改引用的机会。

---

## 四、实现细节

### 4.1 品牌 logo：为什么是「参数化 data URI」而不是两个 base64 常量

先说一个关键发现。`public/bambulab.svg` 的每个路径都是 `fill="currentColor"`：

```svg
<svg viewBox="0 0 485.05 175.15" ...>
  <polygon points="..." fill="currentColor"/>
  <path d="M240.56,112.15h-40.93..." fill="currentColor"/>
```

这意味着它**本来就是为换色设计的**。但 `currentColor` 在 data URI 里**会失效** ——
`<img src="data:...">` 是一个独立文档，`currentColor` 解析不到外层 CSS 的 `color`，
会退化成默认黑色。所以"转成 base64"和"要一个白色版本"这两个要求，
如果按字面做（两个写死颜色的 base64 字符串），会得到两份 3 KB 的重复 markup，
将来改 logo 得改两处。

同时，改成 React 内联 SVG 组件（那样 `currentColor` 就能用了）也不合适：
`HeroFade.tsx:60` 消费的是 `src` 字符串，`useArtLayers` 的淡入淡出、`onError` 掉层、
`data-kind` 分支全部建立在"图层是一个 `<img>`"之上。为一个 logo 改动画管线，代价不对等。

**取中间解**：markup 存一份，颜色作参数，运行时生成 data URI。SVG 走 `encodeURIComponent`
而不是 base64 —— 对文本型 SVG 更小（base64 固定 +33%，URI 编码只对特殊字符膨胀）。

新增 `src/app/assets/bambuLogo.ts`：

```ts
/**
 * Bambu Lab 字标。原文件是 public/bambulab.svg，为了不在 public 下裸放、
 * 也为了同一份 markup 能出黑白两色，搬进来做成参数化 data URI。
 *
 * 不用 base64：SVG 是文本，URI 编码比 base64 小（base64 固定 +33%）。
 * 不做成 React 组件：HeroFade 的图层管线消费的是 src 字符串（见 HeroFade.tsx:60），
 * 换成内联 <svg> 要动淡入淡出与 onError 掉层逻辑，代价不对等。
 *
 * markup 里的颜色统一用 __FILL__ 占位，由 logo() 替换 —— 所以改 logo 只改这一处。
 */
const MARKUP = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 485.05 175.15">...__FILL__...</svg>'

function toDataUri(fill: string): string {
  return `data:image/svg+xml,${encodeURIComponent(MARKUP.replaceAll('__FILL__', fill))}`
}

/** 深色底用 */
export const BAMBU_LOGO_LIGHT = toDataUri('#ffffff')
/** 浅色底用（当前界面用这个） */
export const BAMBU_LOGO_DARK = toDataUri('#111111')
```

> `MARKUP` 的实际内容是把 `public/bambulab.svg` 全文压成一行、
> 把 8 处 `fill="currentColor"` 换成 `fill="__FILL__"`、
> 并删掉 `xml:space`/`x`/`y`/`version` 这些无用属性。实现时逐字节从原文件转写，不重绘。

`heroArt.ts` 改为：

```ts
import { BAMBU_LOGO_DARK } from './assets/bambuLogo'

export const BRAND_ART: Record<string, string> = {
  bambu: BAMBU_LOGO_DARK,
}

export const MODEL_ART: Record<string, ModelArt> = {
  a1: { plain: '/printers/bambu/a1.webp' },
  a1mini: {
    plain: '/printers/bambu/a1mini.webp',
    withVariant: '/printers/bambu/a1mini-variant.webp',
  },
  p1s: { plain: '/printers/bambu/p1s.webp' },
  // a2l / p2s / x1c 暂缺图，自动回落到品牌 logo
}
```

`BAMBU_LOGO_LIGHT` 本轮不接入任何页面 —— 你要求"到时候用得上"，所以它作为导出常量备着。
它会被 `tsc` 保留（有 export），但因为无人 import，Vite 打包时会被 tree-shake 掉，不进 bundle。

**Tauri CSP 已确认放行**：`tauri.conf.json:28` 是 `"csp": null`，没有 `img-src` 限制，
`data:` URI 在 WebView 里可正常加载。

### 4.2 PNG → WebP

工具已就位：`ImageMagick 7.1.2-Q16-HDRI`（`C:\Program Files\ImageMagick-7.1.2-Q16-HDRI\magick.exe`）。
`sharp-cli` 和 `cwebp` 都不在，不额外装。

```powershell
# 先确认是否带 alpha，决定要不要 -background none
magick identify -format '%[channels] %w x %h\n' public/printer-hero.png

magick public/printer-hero.png -quality 82 -define webp:method=6 `
       public/printers/bambu/a1mini-variant.webp
```

q82 + method=6 是这类产品渲染图的常规取值（肉眼无损、压缩率接近上限）。
转换后**人眼比对一次**再删源文件 —— 大图有渐变和软阴影，WebP 有色带风险，这一步不能省。

### 4.3 死数据链路的处置

删掉 plate 与 fishtail 后，`testModels` 的 `image` 字段全部悬空。`contract.ts:47` 的
`image: string` 是必填，不能直接删字段。两条路：

**方案 A（推荐）：整条 `getTestModels` 链路删除。** 走马灯已下线（`PageCalib.tsx:292-295` 自己写明了），
无 UI 调用方，TS 与 Rust 还要手工保持两张表同步。涉及：

- `src/api/contract.ts` — 删 `TestModel` 接口（:39-54）与 `MkpApi.getTestModels`（:79）
- `src/api/mock.ts` — 删 `testModels` 数组（:75-167 含注释）与 `getTestModels` 实现（:196-198）
- `src/api/bridge.ts` — 删 `getTestModels` 转发（:66）
- `src-tauri/src/ipc/mod.rs` — 删 `test_models()`（:154-）、`TestModel`/`Tag` 结构、对应 `#[tauri::command]` 与 `invoke_handler` 注册

**方案 B（保守）：留表，`image` 全置 `''`。** 改动小，但留下一张谁都不读、
却仍要 TS/Rust 双边同步的表 —— 下一次有人改它时同样要面对这个问题。

### 4.4 `ExplodedHero` 与 `three` 依赖

`ExplodedHero.tsx`（354 行）+ `ExplodedHero.module.css` 从未被挂载，
`three@0.186.0` 是它的专属依赖。删掉组件后 `three` 可一并从 `package.json` 移除。

这超出了"清理图片"的原始范围，列为**独立任务并标注需你确认**。
不删组件也能完成资产清理 —— 但那样 `ExplodedHero.tsx:93` 和 `:341-342` 会指向三个已删文件，
留下三处必然 404 的死引用。所以「删 glb + fishtail」与「删组件」实际上绑定：
只删图不删组件，等于把坏引用留在库里。

### 4.5 Git 历史重写

`git filter-repo` **未安装**，但 `python 3.10.11` 在，`pip install git-filter-repo` 即可。

顺序很关键：**先把普通清理提交做完，再重写历史**。这样 `--invert-paths` 指定的旧路径
（`public/a1.webp` 等）只存在于历史提交里，不会误伤新路径（`public/printers/bambu/a1.webp`）。

```powershell
# 0. 兜底备份：一个 bundle 含全部 ref，出事可 clone 回来
git bundle create ../MKPSupportEase-backup-20260922.bundle --all
git count-objects -vH      # 记下 size-pack，作为对比基线

# 1. 重写（在清理提交已完成之后）
git filter-repo --force --invert-paths `
  --path public/a1.webp `
  --path public/a1mini.webp `
  --path public/p1s.webp `
  --path public/bambulab.svg `
  --path public/printer-hero.png `
  --path public/models/test_models.glb `
  --path public/models/hero_fishtail.webp `
  --path public/models/hero_fishtail@2x.webp `
  --path public/models/plate_1.png --path public/models/plate_2.png `
  --path public/models/plate_3.png --path public/models/plate_4.png `
  --path public/models/plate_5.png --path public/models/plate_6.png `
  --path public/models/plate_7.png --path public/models/plate_8.png

# 2. filter-repo 会按设计删掉 origin，重新加回
git remote add origin https://github.com/MuCoreBenC/MKPSupportEase.git
git count-objects -vH      # 与基线对比

# 3. 强推（需你逐条确认，见下）
git push --force origin main
```

---

## 五、边界条件与风险

| 风险 | 说明 | 处置 |
|---|---|---|
| **强推是不可逆操作** | 所有提交 SHA 改变，GitHub 上的 `main` 被覆盖 | 执行前必须拿到你的明确确认；先做 bundle 备份 |
| **另一台机器必须重新 clone** | 你在 macOS 和 Windows 两边测（见 PR #7 的 Windows 修复），另一台的本地 `main` 与重写后的历史无共同祖先，`git pull` 会炸 | 重写完成后，另一台**删掉目录重新 clone**，不要试图 merge/rebase |
| **已合并的 PR #1~#7 快照失效** | GitHub PR 页面的 diff 引用旧 SHA，重写后显示为 orphan | 不可避免。PR 正文与评论保留，diff 可能显示异常 |
| **tag 与其他分支** | 若存在 tag 或远端分支，filter-repo 会一并重写，需同步强推 | 执行前 `git tag -l` + `git branch -r` 清点 |
| **WebP 色带** | 1074×1251 的渲染图含渐变与软阴影，q82 可能出色带 | 转换后肉眼比对；不满意升到 q90 或 `-define webp:lossless=true` |
| **`public/` 不经 Vite** | 改名/移动没有编译期检查，运行时静默 404 | 每个移动都全仓 grep 旧路径确认零残留；最后 `npm run build` + 实跑一次 |
| **`hero_pile` 的 `srcSet`** | `PageHome.tsx:480-481`、`PageCalib.tsx:299-300` 两处路径不变 | 本方案不动它们，但构建后要确认两张图仍在 `dist/models/` |
| **Rust 与 TS 双表** | `mock.ts` 与 `ipc/mod.rs` 是同一份数据的两个副本 | 方案 A 两边一起删；任何一边漏改，`npm run build` 不会报错，只有运行时才暴露 |
| **`.gitignore` 无需改动** | `dist` 已在 `:3` 正确忽略，`public` 必须入库 | 不动 `.gitignore` |

---

## 六、数据流（改动后）

```
用户在 MachinePicker 选 brand / model / variant
        ↓  Selection
pickArt(sel)                                  heroArt.ts:34
        ├─ MODEL_ART[model].withVariant  →  '/printers/bambu/a1mini-variant.webp'
        ├─ MODEL_ART[model].plain        →  '/printers/bambu/{a1|a1mini|p1s}.webp'
        ├─ BRAND_ART[brand]              →  'data:image/svg+xml,...'  ← 不再走网络
        └─ null（什么都没选）
        ↓  HeroArt { src, kind }
useArtLayers(art)  →  ArtLayer[]（新层淡入 + 旧层淡出）
        ↓
HeroFade  <img src={layer.src} onError={掉层}>     HeroFade.tsx:60,65
```

品牌 logo 从"一次 HTTP 请求 + 可能的 404 掉层"变成"随 JS bundle 同步到达"，
`onError` 对它不再有机会触发 —— 这是内联的附带收益。

---

## 七、决策已定（2026-09-22）

**① `getTestModels` 死链路 → 走方案 A：整条删。**
走马灯早已下线（`PageCalib.tsx:292-295` 自己写明了），无 UI 调用方，
留着还要手工维持 TS 与 Rust 两张表同步。

定方案时又扫出两个 doc 原先没列到的引用点，一并删：

- `src/api/index.ts:5` —— 桶文件里把 `TestModel` 也 re-export 了
- `src-tauri/src/lib.rs:60` —— `invoke_handler` 里注册着 `ipc::get_test_models`
- `src-tauri/src/ipc/mod.rs:47` 的 `Tag` 结构 —— 全仓只有 `TestModel.tag`（:63 / :192）在用，可一并删
- `src-tauri/src/ipc/mod.rs:268` 的测试 `test_models_serialize_with_contract_names` —— 跟着一起删

**注意别误删**：`PageCalib.tsx:63` 的 `TEST_MODEL_ID = 'test-models'` 是「打开测试模型」按钮
传给 `openModel` 的那个 3mf id（:199 在用），与 `getTestModels` 无关，**保留**。

**② `ExplodedHero` → 删。** 连带 `ExplodedHero.module.css` 与 `three@0.186.0` 依赖。
组件从未被挂载，`three` 是它的专属依赖。不删的话，`ExplodedHero.tsx:93` 与 `:341-342`
会指向三个已删资产，留下 3 处必然 404 的死引用。

**③ 强推** —— 你已选择重写历史。执行到那一步我会单独停下来向你确认一次，
并提醒你另一台机器必须重新 clone。


---

## 八、预期结果

- `public/` 从 1171.9 KB 降到约 393 KB（-66%），文件数 18 → 6
- 打印机图按 `printers/{品牌}/` 归档，为多品牌预留；`public/` 根目录不再有裸放的图
- 品牌 logo 一份 markup 出黑白两色，`BAMBU_LOGO_LIGHT` 备用待接入
- 仓库内不再有 PNG 位图资产（`src-tauri/icons/` 除外 —— 那是 Tauri 打包要求的固定格式，不能动）
- `.git` 实际体积降幅以 `git count-objects -vH` 前后数据为准
- `npm run build` 通过、`cargo check` 通过、应用实跑无 404
