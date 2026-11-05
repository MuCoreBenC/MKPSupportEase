# 擦料塔两小节 + 官方塔包围盒 + 两处预览显示（2026-11-04）

> 作者原话（图 1）：「他为什么会把那个模型的支撑啊，识别成内墙啊……还有那擦料塔这么显示正确吗？
> 官方的擦料塔是怎么样的」。
> 这一轮只做**显示与事实**，不动任何涂胶/塔的几何：切片坐标、Z、挤出量一个字节没改。

---

## 1. 模型段的支撑被预览画成「内墙」

### 根因

MKP 打擦料塔时自己写了一条 `; FEATURE: Inner wall`（`postproc/pass2/mod.rs:1095`，那条是**故意的**，
作者要的就是预览里那个黄）。而切片器原文回到模型段时**不重发** `; FEATURE:` ——
Bambu 只在 feature 变化时才写。于是从塔块到本层结束的所有模型走线都挂在了「内墙」名下：

| 文件 | `; FEATURE: Inner wall` |
| --- | --- |
| 切片器原文 | 250 |
| MKP 成品（修复前） | 802 |
| 差 | **552** ＝ 同一份文件里 `Tower_Layer_Gcode` 的块数（552），逐字对上 |

实测样例：`MKP_support_test_models_20261104155043` 的 46.014mm 层，
原文 83412 之前最后一条 FEATURE 是 `Support`，成品 125511 之前最后一条是塔块的 `Inner wall`。

### 修法

塔块收尾（`;Leaving Wiping Tower` → `; LAYER_HEIGHT: …` 之后）补一条
`; FEATURE: <切片器当前那一档>`，位置正好是 Bambu 自己会写的那一处。
「切片器当前那一档」由 `Ctx::slicer_feature` 记（只认原文里的 `; FEATURE:` 行；
MKP 塔块自己写的那条走 `write_line`，不经过记录点，不会污染）。

- 塔自身仍是 `Inner wall`（黄的那个不动）。
- 判据：同一份样本重跑后 `Inner wall` = **887** = 637（塔层）+ 250（原文），模型段不再混进来。

### 为什么整链判据要多一把手

`end_to_end` / `process_with_ir` 的口径是「除**有意偏离**外与 `mkp-sr` 参考物逐字节相同」。
这条新增行是本侧有意加的，而参考物由来源仓库生成、本仓无法复现（覆盖它＝把判据变自比自）。
所以按既有做法加了一份剔除器 `postproc::marks::strip_tower_feature_restore`
（认法只认 MKP 自己的锚：`;Leaving Wiping Tower` → `; LAYER_HEIGHT:` → `; FEATURE:` 三行紧邻，
切片器原生的 FEATURE 前面没有这个组合，不会误伤）。

---

## 2. 切片器自己的擦料塔（Prime tower）

### 之前为什么看不见

整条链上只有**排除**：`postproc/disk/collision.rs` 判碰撞时跳开它，`printtime.rs:1279` 有个名字映射。
**没有任何地方数过它** —— 所以报告页既没有「开/关」，也没有位置与耗材。

### 新增 `postproc/prime_tower.rs`

只读事实，不参与变换；随 `HeaderFacts` 一起出门（开跑前就能定）：

| 字段 | 来源 |
| --- | --- |
| `enabled` | 头部 `; enable_prime_tower` |
| `layers` | 含塔块的层数 |
| `height` | 塔块内 Z 极差 |
| `filament_grams` | E 总量 × 2.4053mm² × 1.24g/cm³ ÷ 1000 |
| `bbox` | 塔块内**自带 X 或 Y 的挤出**点的并集 |

★ **包围盒不能直接取「块内所有挤出点」**：多色换料时切片器把吐料也归在 `Prime tower` 段里，
那些 `G1 E18 F324` 只有 E、沿用上一条移动的坐标 —— 而上一条常是床外擦嘴位 `G1 X-48.2`。
直接并集会得到 `X[-48.2, 267.0]`（横跨整板）的假框。只认自带 X/Y 的挤出即可；
`parse_xyze` 不匹配负数，床外点天然被排除。

实测（`gcode_history/2026-11-04/62192.0_20261104_214017_original.gcode`，开了官方塔）：

```text
层数 697 · E 13575.5mm · 并集 X[33.41, 72.35] Y[185.14, 223.13]（38.9 × 38.0 mm）
首层带外扩（33.4/72.3），塔身稳定在 X[36.63, 69.14] Y[188.33, 219.93]
耗材 40.5 g —— 切片器面板同盘显示 40.82 g（口径对上）
```

对照：`62192.0_20261104_214242`（官方塔关）扫出 0 层、`enabled=false`。

### 报告页怎么摆

- 组标题仍是**一个**「擦料塔」（作者：「分成两框」不是拆两个组），组内两小节：
  **MKP 擦料塔**（原来那 4 行）与**切片软件擦料塔**（开关 / 层数·高度 / 耗材 / 位置）。
- 俯视图上多画一个**纯矩形**（`CollisionMap` 的 `.slicerTower`，切片器那个绿，
  与 MKP 塔的红色判据框、模型的灰轮廓都分得开），图例补一条。
  形状不固定 —— 按该盘实测包围盒实时定位，不套 `TowerCoreSvg`。
- 老记录没有 `slicerTower` ⇒ 只显示 MKP 那一节，不假装官方塔是「关」。

---

## 3. 开了热床预涂胶之后，切片器预览里多一条「高处的擦嘴长条」

### 实测（`62192.0_20261104_214242.gcode`）

- `;Bed Glueing Started` → `;Bed Glueing Finished` 之间 **86 行，挤出 0 行**（全是空走）。
- 整段 prep+glue 里**唯一**的挤出行是锚点：
  `G1 X100 Y100 E0.001 F3000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility`，
  在 `G1 Z7.9` 之后 —— 也就是**一边横穿热床、一边在 7.9mm 高处挤出** 100mm。
  切片器预览画出来的那条「又长又高的擦嘴条」就是它。

### 改法（只动这一行的写法）

拆成两步，语义一个字节没变（终点还是 (100,100)、E 的目标值还是 0.001）：

```text
G1 X100 Y100 F3000
G1 E0.001 F3000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility
```

那 100mm 从此是**不挤出的空走**，预览里不再有那条横在半空的条。
**涂胶本身的坐标、Z、路径一行未改。**

### 还没验证的一格（留给下一轮）

> 这条修改的正确性目前只有「G-code 语义不变」这一半证据；
> 「Bambu 预览里是否真的不再出现那条条」必须在切片器里开一次热床预涂胶看。
> 若锚点变成零长度后预览连这个点都不画了（概率低），备选是把锚点挪到涂胶面高度
> （`z_offset + glue_z_offset + GLUE_Z_MARGIN`），而不是继续留在 7.9。

---

## 4. 验收

```text
cargo fmt --all                                      ✅
cargo test -p mkpse-postprocess                      ✅（含新增 prime_tower 3 条）
cargo test -p mkp-support-ease --features workbench archive   ✅
cargo clippy -p mkpse-postprocess --all-targets -D warnings   ✅
cargo clippy -p mkp-support-ease --features workbench -D warnings ✅
npm run build / tsc -b --force / lint / check:i18n / 其余六道 ✅
npm test                                             ✅ 57 passed
```
