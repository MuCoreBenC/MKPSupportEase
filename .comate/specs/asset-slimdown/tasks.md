# 资产瘦身（asset-slimdown）—— 任务清单

按 `doc.md` 拆，两个决策点已定为「都删」。

顺序是刻意的：**先把代码里的引用清干净、再动文件、最后才重写历史**。
反过来做会在中间状态留下一堆 404，而且 `git filter-repo` 要求普通提交先落地
（它按旧路径清历史，新路径必须已经存在于最新提交里，否则会把新文件一起清掉）。

Task 1~5 是可逆的普通改动，随时可停。**Task 6 不可逆**，会单独向你确认。

---

- [x] Task 1: 删掉 getTestModels 整条死链路（TS 侧）
    - 1.1: `src/api/contract.ts` —— 删 `TestModel` 接口（:39-54）与 `MkpApi.getTestModels`（:79）
    - 1.2: `src/api/mock.ts` —— 删 `testModels` 数组（:75-167，含那段讲 plate_N.png 来历的注释）与 `getTestModels` 实现（:196-198），并从 :1 的 import 里摘掉 `TestModel`
    - 1.3: `src/api/bridge.ts` —— 删 `getTestModels` 转发（:66）
    - 1.4: `src/api/index.ts` —— 从 :5 的 re-export 列表里摘掉 `TestModel`
    - 1.5: 确认 `PageCalib.tsx:63` 的 `TEST_MODEL_ID` **原样保留**（它是 openModel 的 3mf id，:199 在用，与本链路无关）
    - 1.6: `npm run build` 通过（tsc 会把漏改的引用全指出来）

- [x] Task 2: 删掉 get_test_models 整条死链路（Rust 侧）
    - 2.1: `src-tauri/src/ipc/mod.rs` —— 删 `test_models()`（:154-）与 `#[tauri::command] get_test_models`（:245-246）
    - 2.2: 同文件删 `TestModel` 结构（:54-）与 `Tag` 结构（:47-）—— 全仓只有 `TestModel.tag` 在用，已确认无其他引用
    - 2.3: 同文件删测试 `test_models_serialize_with_contract_names`（:268-）
    - 2.4: `src-tauri/src/lib.rs` —— 从 `invoke_handler` 里删 `ipc::get_test_models`（:60）
    - 2.5: `cargo check` 与 `cargo test` 通过；`cargo clippy` 无新增告警（尤其确认没留下 unused import）

- [x] Task 3: 删掉 ExplodedHero 与 three 依赖
    - 3.1: 删 `src/app/components/ExplodedHero.tsx`
    - 3.2: 删 `src/app/components/ExplodedHero.module.css`
    - 3.3: `package.json` 移除 `three` 依赖，并检查有无 `@types/three`
    - 3.4: `npm install` 更新 `package-lock.json`
    - 3.5: 全仓 grep `three`、`ExplodedHero`、`GLTFLoader` 确认零残留
    - 3.6: `npm run build` 通过，并记一下 bundle 体积变化（three 是本仓最大的前端依赖）

- [x] Task 4: 删资产 + 按品牌归档 + PNG 转 WebP
    - 4.1: 删 `public/models/plate_1.png` ~ `plate_8.png`（8 个）
    - 4.2: 删 `public/models/hero_fishtail.webp` 与 `hero_fishtail@2x.webp`
    - 4.3: 删 `public/models/test_models.glb`（248.5 KB；machine-motion 那边已有一份，不会丢）
    - 4.4: 建 `public/printers/bambu/`，把 `a1.webp` / `a1mini.webp` / `p1s.webp` 移进去
    - 4.5: `magick identify -format '%[channels]'` 先看 `printer-hero.png` 有没有 alpha，再转成 `public/printers/bambu/a1mini-variant.webp`（q82 + method=6）
    - 4.6: **肉眼比对** WebP 与原 PNG：这张 1074×1251 有渐变与软阴影，有色带风险；不满意就升 q90
    - 4.7: 比对通过后删 `public/printer-hero.png`
    - 4.8: 确认 `public/models/hero_pile.webp` 与 `hero_pile@2x.webp` **原样保留**（PageHome:480-481 / PageCalib:299-300 在用）

- [x] Task 5: 品牌 logo 内联成可换色 data URI
    - 5.1: 新建 `src/app/assets/bambuLogo.ts`：把 `public/bambulab.svg` 全文压成一行做 `MARKUP`，8 处 `fill="currentColor"` 换成 `fill="__FILL__"`，删掉 `xml:space`/`x`/`y`/`version` 这些无用属性 —— **逐字节从原文件转写，不重绘**
    - 5.2: 写 `toDataUri(fill)`：走 `encodeURIComponent` 不走 base64（SVG 是文本，URI 编码更小），导出 `BAMBU_LOGO_DARK`（`#111111`）与 `BAMBU_LOGO_LIGHT`（`#ffffff`）
    - 5.3: 注释写明为什么不用 base64、为什么不做成 React 内联 SVG 组件（`HeroFade.tsx:60` 消费的是 `src` 字符串）
    - 5.4: `src/app/heroArt.ts` —— `BRAND_ART.bambu` 改成 `BAMBU_LOGO_DARK`，`MODEL_ART` 三条路径改成 `/printers/bambu/...`
    - 5.5: 删 `public/bambulab.svg`
    - 5.6: `npm run build` 通过；实跑一次，逐项确认：选 bambu 但不选机型 → 出黑色 logo；选 a1 / a1mini / p1s → 出对应整机图；a1mini 选了版本 → 出 `a1mini-variant.webp`；选 a2l / p2s / x1c → 回落到 logo
    - 5.7: 确认 console 无 404（`public/` 不经 Vite，改名漏改只有运行时才暴露）

- [ ] Task 6: 提交与重写历史（不可逆，执行前单独确认）
    - 6.1: `git status` 核对改动面，确认没夹带无关文件
    - 6.2: 建分支提交 Task 1~5（按仓库既有 PR 流程走，不直接推 main）
    - 6.3: `git bundle create ../MKPSupportEase-backup-20260922.bundle --all` 做兜底备份
    - 6.4: `git count-objects -vH` 记下 `size-pack` 基线；`git tag -l` + `git branch -r` 清点会被一起重写的 ref
    - 6.5: **停下来向你确认**：强推会改掉所有提交 SHA、覆盖 GitHub 上的 main、让 PR #1~#7 的 diff 变成 orphan，且**另一台机器必须删目录重新 clone**
    - 6.6: `pip install git-filter-repo`
    - 6.7: `git filter-repo --force --invert-paths` 清掉 16 个旧路径（3 张打印机图旧位置 + bambulab.svg + printer-hero.png + test_models.glb + hero_fishtail 两张 + plate_1~8）
    - 6.8: `git remote add origin ...`（filter-repo 按设计会删掉 remote），`git count-objects -vH` 与基线对比
    - 6.9: `git push --force origin main`

---

## 验收口径

- `public/` 从 1171.9 KB 降到约 393 KB，文件数 18 → 6
- `npm run build` + `cargo check` + `cargo test` 全绿
- 应用实跑：首页选机型换图正常、校准页那张合影正常、console 零 404
- `.git` 体积降幅以 `git count-objects -vH` 前后实测数为准，不预先承诺

## 不在本轮范围

- `.gitignore` 不动（`dist` 已在 `:3` 正确忽略，`public` 必须入库）
- `dist/` 不处理（构建产物，重新 build 就是新的）
- `src-tauri/icons/` 下那些 PNG 不动 —— Tauri 打包要求的固定格式
- machine-motion 的 `test-models-sandbox` 提交（那边 Task 7.5 还等你点头）
