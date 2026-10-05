> ⚠️ **历史快照（2026-10-05 加注）**：本文写于寻址改造之前，其中的 `presets/dist/`、
> `baseUrl` 同目录推导、release.json 位置等表述是**当时的现状记录**，不再是现行规则。
> 现行规则见 `docs/RESOURCE-ADDRESSING-ROADMAP.md` 与 `docs/PUBLISH-ARCHITECTURE.md` §2.2。

# 交接文档 · 资产交付链与图位分层（2026-10-03）

> **给新窗口/新会话的你**：这份文档是这一串工作的**完整交接**。读完这一篇 + 仓库根的
> `HANDOFF.md`（§0 进度 / §3.5 第三圈 / §5 纪律 / §6 判据清单），你就知道现在到哪了、
> 哪些规矩不能破、下一刀从哪儿下刀、怎么验证。
>
> 分支 `feat/client-assets-pipeline`（**本地，尚未推送**）：资产链这一串 = 8 个提交，
> 整支相对 `origin/main` 一共 15 个（见 §2）。工作区干净。

---

## 1. 这一串工作做了什么（一句话）

把"产品文件"这件事从**几处各写一套**（客户端硬编码表 + npm script 前置 + `public/` 残留）
收成**一条由台账驱动的交付链**：唯一源 → 台账（delivery）→ 对账式同步 → 客户端资源 → 装配进包；
并且让工作台与客户端**都只面向源数据**。

起点是作者看到的现象：「客户端 `npm run build` 会主动清掉工作台 dev 要用的那份资产 ⇒ 工作台全破图」。
后面几刀又把**消费端**收了进来：客户端取图改台账驱动 + 图位分层（版本 / 机型 / 品牌图），
工作台资产库从"登记表"变成**资产检查面板**（真实文件名 / 绝对路径 / SHA-256 / 尺寸 +
「在访达中显示」）。

## 2. 分支与提交

```
b40c813  资产域：整机图回台账（bundled 档）+ 资产根收敛 presets/assets     ← 上一刀的收尾
8114885  随包资产同步链：台账 delivery → 对账同步 → client-assets → vite 装配   ← 第一刀
6cd13ae  台账：增量之八 + §0 进度总览
1394a89  客户端取图改台账驱动 + 机型第二个图位                            ← 第二刀
7a98060  图位分层第一片：版本图位（缺则回落机型图）
823863a  图位分层第二片：品牌 logo 正式进资产体系
c9c673a  交接文档：资产交付链与图位分层（本文件初版）
本条      资产检查面板（第四刀）：wb_asset_inspect + wb_reveal_asset + 详情卡重排    ← hash 见 `git log -1`
```

**按纪律还没推送**（一整刀一个分支连续施工，最后统一 PR）。要推就
`git push -u origin feat/client-assets-pipeline`（推 feat 分支不需要 `ALLOW_PUSH_MAIN`）。

## 3. 现在的链路长什么样

```text
presets/assets.toml            登记（id / type / machineId / path / delivery）
presets/assets/**              文件唯一源（printer 图、图标、模型、BBS、品牌字标）
presets/machines/*.toml        机型：image / imageVariant / icon；[[versions]] 各自的 image
presets/brands.toml            品牌：logo（资产 id）
        │
        │  tools/assets/sync.mjs（对账式同步器，**唯一**搬字节的地方）
        ▼
client-assets/                 唯一随包交付根（生成物、不入库）
   assets/<台账 path>            ← delivery='bundled' 的那几份
   manifest.json                构建期派生账（path/sha256/bytes，无 generatedAt）
   GENERATED.md                 生成目录标记
        │
        │  tools/assets/plugin.mjs（vite 插件，唯一触发者）
        ├─ 客户端构建 ──► 装配 delivery='bundled' 进 dist/assets/<path>（不带哈希）
        ├─ 工作台构建 ──► 装配**全部有 path 的条目**（20 份）进 dist/assets/<path>
        ├─ 客户端 dev ──► /assets/* 读**交付根** client-assets/
        └─ 工作台 dev ──► /assets/* 直读**源** presets/assets/（不落盘）
```

## 4. 锁死的规矩（别改这几条，除非改产品规则本身）

1. **唯一源 = `presets/assets/**`；唯一登记 = `presets/assets.toml`。**
   业务数据（品牌 / 机型 / 版本）只引用**资产 id**，不引用路径、不引用文件名。
2. **`delivery` 是属性，不是目录。** 随包 / 云端是台账里的一栏，不是"哪个文件夹"。
3. **工作台永远面向源读；客户端永远面向 delivery 拿产物。**
   dev 阶段工作台读源、客户端读交付根（有意不同）；打包后两边都读自己 dist 里那份。
4. **同步只由 vite 插件触发**（`configResolved`）。npm script 里不再有前置 ——
   两个入口最后一定会变成两个行为定义。
5. **同步器五分支**（一个都不能少）：新增写 / SHA 同 skip / SHA 异覆盖 / 不再 bundled 删除 /
   源文件不存在 → **构建失败**。判据 = 源 SHA vs **目标文件实际 SHA**（目标即状态，不另立台账）。
   「删」按**期望集合**对账。
   **SHA skip 是同步器的正式语义，不是"优化 Vite"** —— 谁想删它，要改的是产品规则。
6. **URL 契约只有一个**：`/assets/<台账 path>`（后端 `workbench/app/assets.rs` 造，
   前端只 `encodeURI`）。
7. **图位分层（作者定）**：品牌图 → 机型图 → 版本图，**版本缺则回落机型图**。
   `imageVariant`（装了快拆件那张）**是硬件外观变体，不与版本混**，客户端选择层级不用它。
8. **随包资产在客户端包里只有一份**（URL 同形、不带哈希）—— 不要再造带哈希的副本。
9. `public/` 只剩 `public/bbs/**`（上游提取物、工作台不编辑、随包）。**不追求"所有 JSON 搬出 public"** ——
   判据是"每个目录职责明确"，不是扩展名。

## 5. 第四刀（已落地）：资产检查面板

**动机**：作者看着工作台资产库说「为什么右侧详细信息里没有文件的真实文件名、连路径也没有，
我还希望能用系统的文件管理器查看位置」。目标是把详情卡从"登记表"变成**资产检查面板**。

**字段清单（作者逐条给的）**：

| 字段 | 备注 |
|---|---|
| 资产名称 / 类型 / 资产 ID | 已有 |
| **真实文件名** | `a1.webp`（现在只有台账里的相对 path） |
| **源文件路径** | 绝对路径、可复制 |
| **文件大小 / 格式 / 尺寸** | 尺寸 = webp/svg/png 的像素或 viewBox（格式/尺寸目前后端一条都没读） |
| **SHA-256** | 有 `present` 时对源字节算 |
| **当前状态** | 已登记 · 已存在/不存在 · 随包/可下载 … |
| 交付 | `bundled` / `download` |
| **引用** | 谁在用（资产库已有反查 `wb_asset_usage`，详情卡要显示） |
| **【在访达中显示】** | 见下 |

**文件不存在时**：真实文件名照给，路径那一栏改成 **期望路径**，状态写「已登记 · 文件不存在」。

**MKP 预设（`mkPreset`，没有源 path）**：不要空着 ——
给 **产物文件**（`A1-fastv3.3.toml`）+ **产物路径**
（`presets/dist/mkp/presets/<机器>-<版本小写>.toml`，后端已有命名规则）+ 状态（已生成/待更新/未生成）。

**「在访达中显示」的做法（重要，别走偏）**：

- 新增**一条工作台命令**（客户端那条 `reveal_in_folder` 只认「我的文件」，
  路径先过用户根两道闸 —— 工作台要的是"打开 `presets/assets/` 并选中某文件"，是另一条）。
- **前端只传资产 id，路径由后端自己算**（`assets_root` + 台账 path / 产物路径）——
  不给前端传任意路径的机会。参照客户端那条的纪律：**只读、只开窗口、一个状态都不碰**。
- `tauri-plugin-opener` 已在依赖里，且插件在 Rust 侧调用**不需要**在
  `src-tauri/capabilities/workbench.json` 里开权限（客户端那条就是这么做的）。
- `mkPreset` 也要能定位（指向它的产物文件），产品一致性上说得通。

**当初建议的落点**（`wb_assets` 的 `AssetView` 加字段 + 详情卡重排 + `wb_reveal_asset(id)`）：
**字段表照做，但"加进列表"这一半没照做** —— 见下面「实际落点」。

### 实际落点（本条提交，与建议的差别 + 理由）

1. **重读数单开一条命令 `wb_asset_inspect(assetId)`（选中才问），不并进 `AssetView`**。
   理由：`sha256` / `bytes` 要读真实字节（模型实测 3.2 MB），而列表每次筛选 / 搜索词一变
   就重取（搜索框逐键触发）—— 并进去就是"每敲一个字读 4 MB"。它与 `wb_asset_usage`
   同一形状。前端的 effect **只依赖选中**（不依赖 list），换选中才重新问。
2. **`wb_reveal_asset(assetId)`**：前端只传 id，路径后端算（源根 + 台账 `path` / 产物路径）；
   只读、只开窗口、不碰状态；文件不在如实拒绝并附期望路径；界面那一侧按钮同时灰掉、
   原因写进 title（不给必被拒的按钮）。**插件只在 Rust 侧调，`capabilities` 不用改**。
3. **`mkPreset` 指向产物**：产物名走 `build::preset_file_name`（与生成端同一个函数）、
   `productPath` 相对仓库根、`absPath` 绝对；产物没生成 → 读数全空 + 期望路径 +
   状态说「未生成 · 产物文件不在」。
4. **尺寸读文件头**（png / webp / svg viewBox）；读不出如实 `None`。svg 的小数四舍五入
   （品牌字标 485.05 × 175.15 → 485 × 175）。
5. **演示桩（`?mock=1`）**：同形读数 + 「在访达中显示」**如实失败**（没有系统文件管理器）——
   探针断言的就是这句实话。

## 6. 深色模式那天：logo 的两色怎么落

作者 2026-10-03 明确：**logo 将来要两种颜色**（深色模式还没做）。

**今天为什么只有一色**：原 SVG 每个图元挂 `fill="currentColor"`，为换色设计；
但 `currentColor` 在 `<img src="...">` 里**会失效**（独立文档，解析不到外层 CSS 的 color）。
随包资产只能一色，所以固化了**深色**那版（浅色底用，实测只有 `BAMBU_LOGO_DARK` 在被引用，
`BAMBU_LOGO_LIGHT` 是死导出）。

**深色模式那天要做的事（数据模型不用改）**：

1. 台账加**第二条**资产 `bambu-lab-logo-light`（`path = 'brands/bambu-lab-logo-light.svg'`，
   `fill="#ffffff"` 那个版本）——**不是给同一条资产换个 fill**。
2. 客户端那条链 `品牌图 → 内置字标` 改成**按主题挑**：
   `brandLogoFor(theme)` → 查对应那条资产 id → `/assets/<path>`；
   内置兜底也按主题取 `BAMBU_LOGO_DARK` / `BAMBU_LOGO_LIGHT`。
3. 判据加一条：切主题时品牌图的 URL 换成了 `-light` 那条。

## 7. 怎么验证（全绿才算完）

```bash
# Rust（clean 是纪律：本地缓存会掩盖新 lint）
cargo fmt --all -- --check
cargo clean -p mkp-support-ease
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features workbench -- -D warnings
cargo test                                    # 期望 631 通过
cargo test --features workbench --lib          # 期望 478 / 146 / 77（第四刀 +5 条：检查面板）

# 前端
npx tsc -b
npm run lint                                  # eslint + stylelint
npm run build && npm run check:bundle && npm run check:zero-network
```

**四个探针**（都手工跑，用完关服务）：

```bash
# 客户端（4173）
npm run build
npx vite preview --port 4173 --strictPort &
node scripts/probes/presets.mjs http://localhost:4173/
node scripts/probes/home-flow.mjs http://localhost:4173/ --pick   # 含三条分层大图断言

# 工作台（4174；探针那份要带桩才量得到界面）
npm run build:workbench                          # 生产产物，体积闸
BUILD_WORKBENCH=1 NODE_ENV=development npx vite build --target esnext \
  --outDir node_modules/.cache/wb-probe/dist --emptyOutDir
npx vite preview --outDir node_modules/.cache/wb-probe/dist --port 4174 --strictPort &
node scripts/probes/asset-preview.mjs http://localhost:4174/workbench.html   # 含检查面板 6 条断言
node scripts/probes/workbench-build.mjs "http://localhost:4174/workbench.html?mock=1"
```

- `workbench-build.mjs` 的【二】**是已知预存红**（`button[title*="钉住"]`，那个按钮已删），
  停在它之前即可（【一】29 条全 `ok`）。
- 截图落 `tmp-shots/`。
- **别用 `npm run dev`（5321）做验收**（watcher 扫 `target/` 拖死）；要验 dev 行为就单独起它、
  `curl` 一下就关（本次就是这么验的）。

## 8. 这次踩过的坑（下一刀会再遇到）

| 坑 | 症状 / 处置 |
|---|---|
| **忘了重算内嵌 catalog** | `cargo test` 里 `embedded_matches_rebuild` 当场红。改 `presets/` 源后必须 `cargo run --bin gen-catalog`（本次真踩到一次） |
| **`vite preview` 的 SPA 回退** | 取不到的 `/assets/...` 返回 **`index.html` + HTTP 200** ⇒ 光看状态码查不出破图。判据只能量 `naturalWidth` |
| **客户端探针跑的是桩数据** | 真数据（`presets/`）只经 IPC 到 Tauri 真机，浏览器里是 `src/api/mock*` 那套。想验真数据链路要么改桩、要么用 `gen-catalog` + 交付根/装配的断言 |
| **`presets/dist/catalog.json` 与源不同步** | **不用手改**：它由工作台「生成 / 发布」重算（`write_catalog_json` 的调用者就那两个），发布收尾本来就会重算 |
| **测试夹具里的死文件名** | `testkit.rs` 的品牌曾写 `logo = 'bambu-logo.png'`（早就不存在的文件）。今天 `logo` 是资产 id、加载期校验 → 97 条测试红。**夹具数据要与真数据同形** |
| **真数据判据绝不能调写命令** | 测试进程里 `Ctx` = 真仓库。写命令只在真机 / 工作台里跑 |
| **判据条数变了要说清为什么** | 资产 28→29、image 档 4→5、随包身份 4→5、品牌图那条 —— 每处都写了理由。别为了让测试绿而改数 |
| **`replace_in_file` 偶发"参数缺失"** | 工具抽风时改用 `python3` 脚本改文件（同一处改动，别绕） |
| 注释里也有同样的字面量 | 反向测试改台账时 `replace` 打到了文件头注释里的示例行 —— 改真条目前先确认唯一性 |
| **`[class*="cardBody"]` 不止一张** | 工作台外壳把别的页的卡片也留在 DOM 里（实测 4 张）——`querySelector` 拿的第一张是机型页的。探针要按「资产检查」这块认卡（第四刀先红后绿的那一脚） |
| **重读数别挂进列表 DTO** | SHA-256 / 大小要读真实字节（模型 3.2 MB），而列表逐键重取 ⇒ 单开"选中才问"的读命令（见 §5 落点 1） |

## 9. 文件地图（改哪件事去看哪个文件）

| 文件 | 负责什么 |
|---|---|
| `tools/assets/sync.mjs` | 同步器：读台账 → 期望集合 → 五分支对账 → 写 `client-assets/` |
| `tools/assets/plugin.mjs` | vite 插件：触发同步、dev 中间件（分源）、build 装配（分集合） |
| `tools/assets/plugin.d.mts` | 上面那个的 TS 声明（`vite.config.ts` 引它） |
| `vite.config.ts` | 注册 `mkpAssets({ workbench })`（`--mode workbench` / `BUILD_WORKBENCH=1`） |
| `src/app/home/heroArt.ts` | 客户端取图：`assetUrlOf` + `pickArt`（回落链的唯一实现） |
| `src/app/home/useCatalog.ts` | 首页那三个读：`getMachines` + `getPresetFiles` + `getRuntimeCatalog`（后者出 `assets` / `brands`） |
| `src/api/contract.ts` | 客户端契约：`Machine.image/imageVariant`、`MachineVersion.image`、`CatalogAsset`、`RuntimeCatalogBrand` |
| `src/workbench/views/MachinesPage.tsx` | 机型图 / 图标 / 版本图三个素材格与选择器 |
| `src/workbench/views/AssetsPage.tsx` | 资产库页：列表 / 筛选 / 详情卡（**检查面板 + 「在访达中显示」**都在这一张卡上） |
| `src-tauri/src/presetdata/catalog.rs` | 机型与版本的字段、`MachineField` / `VersionField`、写回（值面 + 文档面两份，一起改） |
| `src-tauri/src/presetdata/mod.rs` | `check_asset_refs`（机型 / 版本 / 品牌三处引用校验都在这） |
| `src-tauri/src/workbench/app/assets.rs` | `wb_assets` 的 `AssetView` + **`wb_asset_inspect`**（检查面板读数 / 文件头尺寸读取器）+ **`wb_reveal_asset`**（在访达中显示） |
| `src/workbench/dev/mockBackend.ts` | 工作台演示桩（`?mock=1`）的检查面板读数 + 「在访达中显示」如实失败 |
| `src-tauri/src/workbench/app/machines.rs` | 工作台机型页 DTO（含 `VersionView.image`） |
| `scripts/probes/asset-preview.mjs` | 资产库判据（逐行走 `naturalWidth > 0` + **检查面板 6 条**：文件名 / 绝对路径 / SHA / 尺寸 / 在访达中显示 / 产物路径） |
| `scripts/probes/home-flow.mjs` | 首页判据；`--pick` 末尾有三条**分层大图**断言 |
| `HANDOFF.md` 增量之八～十一 | 这一串的正式登记（已提交） |

## 10. 还没做的（明确清单）

1. **深色模式的两色 logo**（见 §6）。
2. **`catalog` 24 份 vs 交付根 17 份**那条裂纹（`referenced_assets` 决定"发不发"，
   `delivery` 只有一票否决；客户端预设页会列出 4 份下不到的 0.2mm BBS）—— 独立一刀。
3. **BBS 页预设清单在真机没有端点**（只来自 serve 期 `/api/bbs/presets`）—— 独立一刀。
4. `presets/dist/` 那一版 catalog 的 definition 要等下一次工作台「生成 / 发布」才带上新字段。
5. 工作台浏览器桩没实现 `setVersionField` / `setMachineField`（既有缺口，演示里改不动字段）。
6. 分支**未推送**、**未开 PR**。

> **追记（2026-10-03，另一条分支 `feat/wb-brands-readability`）**：上面第 6 条已落地 ——
> 品牌图那一格连同整个「品牌」条目进了机型与版本页（`wb_set_brand_field` / `wb_add_brand`）；
> 那条分支从本分支上切出，两串可以分开 PR。
