# 图片引导：交接（2026-10-16）

> 这一条线是「**改 .3mf 默认打开方式**」那一处的图片引导：**客户端那个弹窗里的图 + 图上那几个框**。
> 本文 = 现在到哪了 + **下一刀做什么** + 不许违背的口径 + 判据在哪。
> 相关：`docs/DATA-ARCHITECTURE.md`（归属表里"标注源稿"那一行）· `HANDOFF.md`（增量账）· `docs/DOC-INDEX.md`。

---

## 0. 一句话现状

客户端：设置页 →「去系统里改…」→ 弹窗左边一张系统截图、右边状态句 + 两步，图上两个框指着
「打开方式」下拉与「全部更改…」；**图可点开放大**（可滚轮 / 双指缩放、还能拖）、**图取不到就整块留空**。

工作台：**标注台**（`src/workbench/views/AnnotatePage.tsx`）在图上拖框、调框的样子、把**源稿写进仓库**。

---

## 1. 接手先读：六条口径（改动前先确认没违背）

| # | 口径 | 落点 |
|---|---|---|
| 1 | 图**住资产根 + 进台账**（不是 `src/app/assets/`）：`presets/assets/guides/` + `presets/assets.toml` 两条 `type = 'guide'`、`delivery = 'bundled'` | `openWithGuide.ts` 头注 |
| 2 | 客户端**只认资产 id** → `catalog.assets[]` → `path` → `/assets/<path>`；换图 / 换裁切**这个文件一个字不用改** | `openWithGuide.ts` |
| 3 | 标注 = **一份数据、两张图共用**：`marks` 是**百分比**，浅 / 深两张同尺寸同裁切；`GuideImage.tsx` 画 | `GuideImage.tsx` 头注 |
| 4 | **源稿**（能再编辑的那一份）住**图旁边**：`presets/assets/guides/<对>.anno.json`，跟仓库走、**发布排除** | `src-tauri/src/workbench/app/annotation.rs` |
| 5 | 图**一像素装饰都不加**；**不摆"点一下放大"那种提示**；动效 / 缩放一律 **WAAPI 或内联 transform + rAF** | `GuideImage.module.css` / `.tsx` |
| 6 | 框的线 / 圆角 / 编号**视觉大小恒定**（`calc(… / var(--zoom-k, 1))`），线画在 `box-shadow` 上（`border` 会被引擎钳到最小 1px） | 同上 |

---

## 2. 下一刀 ①：**图片按需下载**（原定方向）

作者原话（2026-10-16）：「图片……后续按需下载……到那一页的时候就下载下来」。
现在的状态是 `delivery = 'bundled'` —— **图随程序本体走**（客户端不下载、不更新）。

### 要做的三步

1. **台账去掉那两条的 `delivery = 'bundled'`**（`presets/assets.toml` 的两条 guide）——
   不写 = 进交付集合 = **按需下载**。
2. **`cargo run --bin gen-catalog`** 重生成快照（`embedded_matches_rebuild` 会盯着）。
3. **客户端补一条"读下载区图片"的口子**：今天下载区的东西**只有文本走 IPC**（图片没有）；
   取图那一步（`openWithGuide.ts` 的 `assetUrlOf`）要变成"先看下载区有没有 → 没有就按需下载 → 再给 URL"。

### 会红、且是设计好的判据（照它说的核对台账，别直接改数字）

- `workbench/app/assets.rs`：`the_real_identity_states_match_the_model`（现在 **bundled 7** ⇒ 变 5）
- `scripts/check-bundle.mjs`：**随包装配 7 份 ⇒ 变 5 份**
- `runtime/catalog.rs` 那两处条数判据（`files[]` 会多出这两条）

### 坑（都踩过或明确过）

- **老客户端 + 新目录 = 破图**：0.0.6 的包里没有那两个字节，而新目录说"有"。
  ⇒ 这一刀**必须配版本发布**（bump + tag），不能只 `publish:presets`。
- **`marks` 一个字都不用改**（坐标是百分比、与来源无关）；但**换图 / 重裁之后要重量坐标**。
- 客户端那侧别忘了 `docs/DATA-ARCHITECTURE.md` **归属表**里这两条的身份要改（现在是"随程序本体"）。

---

## 3. 下一刀 ②：**"一组"（套装）的感觉** —— 2026-10-16 作者新提的

作者原话：

> 「它们本质上是同一套图的两个，那我就希望它不是这种分开的，他需要是**套装的感觉**，
>   这样他的框选的才能说是对应两个的，那不然你现在是怎么区分的呢？
>   为什么这两个他们用到的那个线框是一样的？为什么我切换到别的这个线框就没了？
>   所以说，我觉得最好还是做这种套装的感觉比较好，或者说是**组合的感觉，就是用同样的框的就是一组**」

### 现在是怎么区分的（答那两个"为什么"）

- **一组 = 一份源稿**。`open-with.anno.json` 里写着 `source.images`（它画在哪几张上，2026-10-16 由 `light/dark` 泛化成列表）
  ⇒ 那几张就是"一组"；**框画在源稿上**，所以那一组里每张看到的框必然一样。
- **切到别的图框就没了** = 那张图**没有源稿**（不是漏画）。任何一张图第一次打开都是空的 ——
  拖一下就为它（或它那一组）建一份源稿。
- "哪两张是一组"的约定：**把 id / 文件名里 `-light` / `-dark` 那一段抹掉**就是组名
  （Rust `annotation::pair_stem`、工作台 `pairOf(key)` **两处必须一致**；客户端那份 `marks` 是源码常量，`LIGHT_ID` / `DARK_ID` 写死）。

### 已经做的一小步（2026-10-16 本轮）

- 标注台的选图改成**按组摆**：成组的收进 `<optgroup>`（组名 = 把「（浅色）」抹掉），
  标签写着「（一组 2 张 · 共用一份标注）」；单张的平摆。
- 页脚点名同组那一张：「… · 同组还有：`guides/open-with-dark.png`（共用这份标注）」。
- 探针 `scripts/probes/annotate.mjs` 盯着这两条。

### 三件裁决（2026-10-16 作者已拍）

| 问题 | 裁决 |
|---|---|
| 一组可能**不止两张**吗？ | **已泛化为 N 张**：`AnnoSource` 由 `{light,dark}` 改成 `source.images: string[]`。★ 因为**还没发布、没有兼容包袱**，所以**原地改形状、代次仍是 1**，不写 v1→v2 迁移（作者：「不用管它，我们现在都还没有发布」）。四处同步：Rust `annotation.rs` / `api.ts` / `mockBackend.ts` / 真源稿。 |
| **组名 / 组内次序**从哪来？ | **仍靠约定**（一份源稿 = 一组，`pairOf` 抹浅深后缀）；组内次序 = `images` 数组顺序。台账加 `groupId` 字段属"数据归属"，暂不做。 |
| 客户端那一侧要不要也"套装"？ | **不动**：客户端**不读源稿**（源稿不入交付、不进包）；`openWithGuide.ts` 的 `LIGHT_ID` / `DARK_ID` 与 `marks` 是源码常量，就是"这一组"的客户端表达。 |

---

## 4. 判据与探针（哪条守哪条）

| 判据 / 探针 | 守什么 |
|---|---|
| `annotation::a_pair_shares_one_source_document` | 浅 / 深**共用一份源稿**（抹浅深后缀的落点口径） |
| `annotation::the_source_document_is_neither_an_asset_nor_a_delivery_item` | 源稿**不进资产、不进交付目录** |
| `annotation::the_real_source_document_parses_and_passes_checks` | 仓库里那份**真源稿**读得出、过校验 |
| `annotation::check_rejects_broken_documents` | 坏值（负坐标 / 出界 / 重号 / 样式离谱）**整次拒绝** |
| `catalog::bundled_assets_…` + `runtime::ui_catalog_backfills_…` | **随包那份只兜底**（② 旧 / 缺也不丢图） |
| `scripts/probes/guide-image.mjs`（~70 条） | 版式（图左文右 / 等比 / 完整 / 不滚）、放大层（FLIP 进出、**滚轮与双指缩放**、默认大小、Esc 只收一层、遮罩底独立、卸层不闪）、**伪造 404 整块留空** |
| `scripts/probes/annotate.mjs` | 框落点逐值、拖动 / 撤销 / 拉边、**一组（optgroup + 同组那一张）**、样式跟着变、**写入仓库 → 换图读回**、导出带 style；**编辑体验那一批**：数字格退格/删除**不删框**、删框 ⌘Z **一定找回**、撤销/重做两颗按钮、**保存后仍能撤销**、「放弃改动」可点且**本身可撤销**、冲突时**两版并排预览**（repo/local 两层）再选 |

---

## 5. 已知的坑（踩过的，别再踩）

- **② 的写入时机**：启动只铺空位；**每次运行第一次进「预设」页**会拿云端那份对齐一次（"指纹不一样就采用"，**没有先后**）。
  ⇒ 手工往 `<appDataDir>/catalog.json` 里塞东西**只是临时**（那次对齐会打回），随包的那类由 `catalog_for_ui` 兜底解决。
- **`getComputedStyle` 量不到空格写法的颜色**（`rgb(16 24 32 / 42%)` ⇒ 空串）—— 要脚本量一眼的颜色用逗号 `rgba()`。
- **`border-width` 有 1px 下限**；**`el.closest('div')` 会拿错 `.frame`/`.zoomLayer`**（探针里认类名片段更稳）。
- **改 `.mark` 的样式变量名 / 缺省值要改两处**：客户端 `GuideImage.module.css` 与工作台 `AnnotatePage.module.css`（import 不到彼此，靠同名同缺省守）。
- 换图 / 重裁之后**必须重量坐标**（附：`docs/GUIDE-...` 的 canvas 扫描法在 `openWithGuide.ts` 头注里；标注台拖一遍更快）。

---

## 6. 落点清单（这一条线涉及的文件）

```
src/app/guide/GuideImage.tsx        客户端那个弹窗：图 + 框 + 放大层（FLIP / 缩放 / 空态）
src/app/guide/GuideImage.module.css 版式、框与编号（CSS 变量）、防闪的两条
src/app/guide/openWithGuide.ts      取图（只认资产 id）+ marks（导出的那一段）+ 量法记录
src/components/modal/Modal.tsx      加了一个 overlay 槽（"从框里浮起来"那一层）
src/workbench/views/AnnotatePage.tsx  标注台：拖框 / 一组 / 样式 / 源稿读写
src/workbench/views/AnnotatePage.module.css  与客户端同名的样式变量
src-tauri/src/workbench/app/annotation.rs    源稿的读写 + 四条判据
presets/assets/guides/open-with.anno.json    真源稿（两个框）
presets/assets.toml                  两条 guide 的登记（下一刀要改 delivery）
scripts/probes/guide-image.mjs       客户端那一屏的常驻探针
scripts/probes/annotate.mjs          标注台的常驻探针
```

---

## 7. 下一步（作者点头就做，按建议次序）

1. **按需下载那一刀**（§2）—— 它挡着"用户端看不到新图"这件事的正路（配版本发布一起走）。
2. **发布链**：bump 版本 + 工作台「发布」+ `npm run publish:presets`（把新目录推上云端，老用户才会看到那两张图）。
3. §3 那三件裁决（一组能不能超过两张 / 组名从哪来 / 客户端要不要动）—— 建议都先维持现状，等真需求。
