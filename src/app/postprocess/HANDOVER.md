# 后处理进度这一屏 · 交接文档

（2026-10-15 写。读者：接手这一屏的人。写完时状态：**进度平滑与交互在 dev/无头环境全部实测通过；
真机上"CSS 动画不播"这个环境问题未解，但交互已改用 WAAPI 绕过**——见 §4。）

## 1. 这一屏是什么

后处理（给 G-code 上胶/校准）跑起来时弹的那一屏。文件：

| 文件 | 管什么 |
|---|---|
| `PostProcessModal.tsx` | 那一屏本体：跑着 hero / 问句红卡 / 结果四格 / 状态机（收尾、翻面） |
| `PostProcessModal.module.css` | 它的皮（原型 `frontend-lab/lab/mkp-postprocess/` 定稿的落地） |
| `useSmoothPercent.ts` | **进度条平滑**（本屏最复杂的一块，见 §2） |
| `steps.ts` | 12 步的中文名与**全局权重**（权重是体感，不是测量） |
| `usePostProcessRun.ts` | 数据源（Rust 侧事件通道的回写） |

## 2. 进度条的口径（`useSmoothPercent.ts`）

**一句话**：真实进度是"每 1% 一跳"的事件，条子要"一直在走"但不许撒谎。
做法 = **速率外推 + 上限封顶**（`want + rate × idle`，再封在 `want ± cap` 之间）。

| 常量 | 值 | 管什么 / 为什么 |
|---|---|---|
| `LOOKAHEAD_BASE` / `LOOKAHEAD_MAX` | 3 / 9 | 最多超前事件值几个点（不虚报的那道线）；等得越久放到 9 |
| `LOOKAHEAD_GROW_PER_SEC` | 1.5 | 每等 1 秒，上限多放开多少点 |
| `SPEED_EMA` / `SPEED_MIN_DT` | 0.45 / 0.1s | 瞬时速率的 EMA 与采样间隔（小文件 1~2 秒跑完，学慢了没用） |
| `CREEP_PER_SEC` | 3 | **没有任何速率信息**时的兜底爬升（开场那几步） |
| `APPROACH` | 1.2/秒 | 过程的比例项——**0.1 太陡**（开场"窜一下停住"）、1.2 是作者定的"均匀" |
| `CHASE_PER_SEC` | 12 | 追赶限速（**防"3 或 8 直接跳到 11"那种跳变**） |
| `MAX_CHASE_PER_SEC` | 80 | 追赶的绝对上限（**必须盖过小文件 68+ 点/秒的事件速度**，否则"60 多突然完成"） |
| `SETTLE_APPROACH` | 8/秒 | **收尾**冲线的比例系数（**不设点数限速**——"被限速拖着走"的手感作者否过） |
| `SETTLE_MS` | 150ms | 条子到 100 之后再停多久才让位给结果态 |
| `EPS` | 0.05 | 到位判据 |

**跨运行学习**（`STORAGE.clientPostProcessPace`，见 `storageKeys.ts`）：
`{ [stepId]: msPerMB }`，换步/收尾时按 MB 归一 EMA 学一格，开场拿它当先验（**缓存，不是底账**）。

**展示时长**（`PostProcessModal.tsx`）：`MIN_SHOW_MS = 1500`（整段下限）、
`SETTLE_MAX_MS = 8000`（等条子走完的兜底）、`LEAD_FADE_MS = 160`（收尾淡出时长）。

## 3. 交互（点击 / 按住 / 双击翻面）

- **单击 / 连击**：回弹一下（"给等待的人解压"）。
- **按住不放**：压到 `scale 0.972` + 暗 0.97，**一直保持到松手**（`pointerup` / `cancel` / `leave`）。
- **双击**：结果四格 ⇄ 100% hero 翻面。**判定是自己数的**（`DOUBLE_TAP_MS = 600`、
  `DOUBLE_TAP_SLOP = 24px`）——**不用系统 `dblclick`**：那个跟系统"双击速度"设置走，连击会被吞。
- **出场**：收尾切结果 = 淡入；双击翻面 = **轮播**（`startFlip`：旧块先上滑淡出 **130ms**，
  滑完才真翻；新块再由 `POP_KEYS` 从下弹上来，**240ms**——作者 2026-10-15：「上一个往上消失，
  下一个从下弹出来」→「速度有点慢，我点击快的时候他也要快」）。
  **快速连击**：`flipBusyRef` 一置位就直接翻（**不等滑出**）；`flipGenRef`（轮次号）让
  被"打断的旧轮次"回来时自动作废，不会把新一轮顶掉。

**⚠️ 这三样全部用 `element.animate()`（WAAPI）实现，不是 CSS 动画。** 原因见 §4。

**⚠️ 铁律（踩过三次）**：
1. **同一个元素上不许有两个 CSS `animation`**——`animation` 是同一属性，后者盖前者
   （"四格点了没效果"就是这么来的）。
2. 现在更简单：**这一屏别再加 CSS 动画**，一律 `element.animate()`。
3. 全站模态框**点背景不关**（`closeOnScrim` 默认 false，2026-10-15 作者定的口径），关走 Esc / 关闭键。

## 4. ★ 未解问题：真机上 CSS 动画不播（WAAPI 正常）

**现象**：作者真机（Tauri 的 WKWebView）上，这一屏所有 CSS `animation`/`transition` 效果
**一个都看不见**——点击回弹、按住过渡、收尾渐隐、翻面弹入，全部"一帧内跳变"。

**证据**：
- 拆视频帧 + 像素差量化（`/tmp/mkpvid` 那套脚本，见 §5）：切换处 diff 从 0 直跳 8.5/9，
  **无任何渐变**（若有 330ms 动画，40ms 一帧应看到 ~8 帧爬升）。
- 排查结果：**代码是新的**（dev server 5321 吐出的 CSS 里有 `pop-in`）、
  **生产构建也是对的**（`dist/assets/*.css` 里 `@keyframes _pop-in_...` 与 `animation:...!important` 都在、
  JS 里类名 `_popIn_...` 也在）、**`tauri dev` 也重启过**、**系统"减弱动态效果"开关是关的**。

**头号嫌疑**：`src/styles/global.css` 里那条全局降级——
`@media (prefers-reduced-motion: reduce) { * { animation: none !important; transition: none !important } }`。
**哪怕系统开关看着是关的，WKWebView 读到的未必一样**（这台机器上"按住缩放"可见——那是普通样式、
不受 `animation`/`transition` 降级影响，正好与"只有它看得见"吻合）。
我曾给那四样加了 `!important` 豁免（specificity 规则上应能赢 `*`），**真机上仍不播**。

**绕法（已实施，作者已确认"切换有效果了"）**：那三套交互动效全部改 **WAAPI**
（`element.animate()`，JS 调用、不过 CSS 级联）。
无头 WebKit 实测：点击 `getAnimations().length = 2`、按住 400ms `transform = 0.972`、松手回弹 ✓。

**⚠️ 改 WAAPI 时踩的第一个坑（务必留着这条经验）**：收尾"淡出"用了
`fill: 'forwards'` ⇒ 它把那一层**永久按在 opacity 0** 上；翻面回来时这一层被复用，
作者截图里那块就是"**空的**"。三处修法：① 两个 effect 开头一律
`el.getAnimations().forEach(a => a.cancel())`（**先清上一轮残留**）；② "弹上来"的
关键帧**不碰 `opacity`**（只动 `transform` —— 万一动画卡住也只错位置、不会"看不见"）；
③ 收尾"淡入"加 400ms 安全阀（没跑完就 `cancel()`，回到元素自身样式）。
**→ 以后加 WAAPI 动画：一律"先 cancel 旧动画"，且"可见性"类属性（opacity）不要赌动画一定能跑完。**

**⚠️ 第二个坑（同日）**：hero 那层与结果块那层是**同位置的条件渲染**，**不给 `key` 时 React 会复用
同一个 `<div>`** ⇒ "旧块滑出"与"新块弹入"作用在同一个元素上互相打断（作者："上一个还没消失，
下一个就进来了 / 乱了"）。修：两块各给 `key`（`heroBox` / `leadBox`）+ 滑出动画 `fill: 'forwards'`。
**→ 规则：同位置的"甲/乙条件渲染"若各自要独立动画，必须各给 `key`。**

**如果 WAAPI 在真机上也不播**（待作者确认），下一步按此排查：
1. **把诊断写进界面可见处**：在 `PostProcessModal` 里临时渲一行
   `matchMedia('(prefers-reduced-motion: reduce)').matches` + `getComputedStyle(hero).animationName`
   （dev 模式 HMR 立即生效，不用重编译）；
2. 若 `reduce` 为 **true** ⇒ 就是那条全局降级，**要么去掉那条 `!important`，要么把 `@media` 判定放宽**；
3. 若 `reduce` 为 false 且 WAAPI 也不动 ⇒ 查显卡/合成层（`will-change`、`transform` 在 WKWebView 的
   合成路径），换"纯 `opacity` 动画"试（最不容易被禁）；
4. **确认作者跑的到底是哪一份**：`ps aux | grep tauri`（dev，前端连 5321）
   vs `/Applications/SupportEase.app`（`ls -l` 看时间戳；那个是 **2026-10-06** 构建的旧包）。

## 5. 验证脚本（无头 Playwright，住在 `~/.npm/_npx/48b1ca104c3549f4/`）

| 脚本 | 验什么 |
|---|---|
| `jump.mjs` | 大跳变（3→8）平滑性、高速步跟得上 |
| `fast.mjs` | 小文件：done 时条子位置（应 ≥ 80，不再"60 多突然完成"） |
| `learn.mjs` | 跨运行学习（第一轮学表、第二轮开场按先验走） |
| `dbl.mjs` | 慢连击（520ms）/快连击（150ms）都翻得动 |
| `waapi.mjs` | 点击回弹、按住 0.972、松手回弹（WAAPI 版） |
| `frames.mjs` / `dense.mjs` / `diff.mjs` | **拆视频帧 + 像素差量化**（分析真机录屏用；mp4 放本地 http 服务喂给 WebKit） |

跑法：`cd ~/.npm/_npx/48b1ca104c3549f4 && node <脚本>.mjs`（需要 dev server 在 5321）。
