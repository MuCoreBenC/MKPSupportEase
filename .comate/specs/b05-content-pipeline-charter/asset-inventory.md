# 旧仓资产盘点与迁移清单（b05 Task 7）

> **只盘点，不搬运。** 盘点对象是 `local-reference/mkpse-presets/`（旧仓的一份只读拷贝）。
> 本文件只回答「有什么、归谁、还用不用、迁不迁」，**不动任何文件**：
> 搬运是 Task 9 的事，删除是各 Task 各自的事（G-3 的 Orca 在 Task 16.4）。
>
> 口径与证据等级写在 §0；`assets/` 六个子目录见 §1；顶层 `models/` 三种形态见 §2；
> 迁移清单见 §3；只读证据与"我们这边还引用它吗"见 §4。

---

## 0. 口径与证据

- **体量/份数**：`Get-ChildItem -Recurse -File` 实测（2026-09-24）。
- **归属机型与类型**：取旧仓自己的 `manifest.json`（72 条资产，含 `resourceType` / `machineId` /
  `category` / `sha256` / `size`）与 `content/asset_usage.json`（旧仓自己那份"谁在用它"反向索引）。
- **完整性**：拿 `manifest.json` 的 72 条 sha256 **逐份核过拷贝里的实际字节** ——
  **72/72 全对，0 缺 0 不符**（见 §4）。清单里 54 条 `image` = 3 avatars + 29 faq + 3 icons
  + 11 machines + 8 models，逐一对得上。
- **「我们这边还引用它吗」**：在 `MKPSupportEase` 仓里 grep 过（排除 `local-reference/`），
  结论见 §4 —— 代码里对旧仓那批路径**零命中**，唯一沾边的是机型定义里的**裸文件名**。

## 1. `assets/` 六个子目录（7.2 / 7.3 / 7.5）

| 子目录 | 份数 | 体量 | 归属 | 旧仓内谁在用 | 结论 |
| --- | --- | --- | --- | --- | --- |
| `avatars/` | 3 | 14.1 KB | 无（关于页作者头像） | `about.json` → `authors` | **舍弃**（D-4：about 不纳入） |
| `faq/` | 29 | 5,372.7 KB | 无 | `faq.json` | **舍弃**（D-4：FAQ 不纳入） |
| `icons/machine/` | 3（+1 残留） | 1.1 KB（+374 B） | A1 + A2L / A1_MINI / P1S + P2S + X1C | `asset_usage` → `machine/*` | **保留 3 份 .svg**；`cantilever.svg.orig` 舍弃（`.orig` 残留，不在 manifest） |
| `machines/` | 11 | 247.5 KB | 5 台归我们（a1 / a1_mini / p1s / p2s / x1c），6 台无归属 | `asset_usage` → `machine/{A1,A1_MINI,P1S,P2S,X1C}` | **保留 5 份**；另外 6 份（`anycubic_kobra_x`、`creality_sparkx_i7`、`k1c`、`k2c`、`s1c`、`voron24`）**舍弃** —— 无引用，且归属机型不在我们的机型集合里 |
| `models/` | 8 | 30.9 KB | 无（模型预览图） | `model_copy.json` 的 `models[].image` | **舍弃**（D-4：`model_copy` 不纳入）。**空档见 §5** |
| `qr/` | 8 | **0 B** | 无 | 无 | **整目录舍弃**（7.3 成立：8 份全是 `*.png.gitkeep` 占位符，0 字节，没有一张真图） |

## 2. 顶层 `models/`：三种形态（7.4）

同一批模型有三种形态并存，实测把关系钉住了：

| 文件 / 目录 | 体量 | sha256 前 16 位 | 是什么 |
| --- | --- | --- | --- |
| `models/MKP_support_test_models.3mf` | 3,317,913 B | `8486199771D639A0` | 模型（多模型拼盘） |
| `models/MKP_support_test_models.zip` | 3,317,913 B | `8486199771D639A0` | **与上面逐字节相同**，只是扩展名不同 |
| `models/Precise_Calibration.3mf` | 385,969 B | `9D7B373D3AA90FE2` | 模型（独一份，没有第二种形态） |
| `models/ZOffset_Calibration.3mf` | 290,327 B | `773B65B0DCDF0947` | 模型 |
| `models/ZOffset_Calibration.zip` | 290,327 B | `773B65B0DCDF0947` | **与上面逐字节相同** |
| `models/ZOffset_Calibration/` | 17 份 1.7 MB | `BC3447438F7F8ED4`（内部文件哈希汇总） | **同一份的展开**：17 个条目与那个 `.3mf` 里的 17 个条目一一对应（`[Content_Types].xml` / `3D/` / `Metadata/` / `_rels/`） |

**定案（7.4）：只交付 `.3mf`。**
`.3mf` 本身就是 OPC 包（zip 容器），所以 `.zip` 是同一份字节的另一个扩展名、解压目录是同一份内容的
展开 —— 三选一没有信息损失。保留三份 `.3mf`（合计 3.99 MB），舍弃两个 `.zip` 与那个解压目录
（合计 5.31 MB）。

**一个必须记下的事实**：这三份 `.3mf` **不在 `manifest.json` 的 72 条里** —— 旧仓自己没把它们当
"客户端要下载的资产"，是 `content/model_copy.json` 的 `models[].modelFile` 按**裸文件名**引用的。
也就是说：它们是给模型库页面用的**模型本体**，交付路径与 BBS/预设那套不同。
（三个名字去重后正好是这三份；8 个模型条目共 3 个 `modelFile` 取值。）

## 3. 迁移清单（7.6）

| # | 对象 | 份数 / 体量 | 处置 |
| --- | --- | --- | --- |
| 1 | `assets/machines/{a1,a1_mini,p1s,p2s,x1c}.webp` | 5 / 84.8 KB | **直接保留** → 图片资产，归属 5 台机型。附注：与 `public/printers/bambu/` 里那几张**不是同一份文件**（体量差约 4 倍），Task 9 迁移时要定留哪一份 |
| 2 | `assets/icons/machine/{a1,a1_mini,p1s}.svg` | 3 / 1.1 KB | **直接保留** → 图标资产。归属是**多对一**：`a1`→A1+A2L、`a1_mini`→A1_MINI、`p1s`→P1S+P2S+X1C |
| 3 | `models/{MKP_support_test_models,Precise_Calibration,ZOffset_Calibration}.3mf` | 3 / 3.99 MB | **直接保留** → 模型资产 |
| 4 | BBS 9 份 JSON（`presets/bbs/Process/**`） | 9 / 13.8 KB | **需转换**：按新结构重写成一集中定义（doc §12.4 已定，G-3 纳入） |
| 5 | 套餐 5 份（`source/bundles/*_default.toml`） | 5 / 0.7 KB | **需转换**：一文件一条 → 一份集中定义（doc §12.4；Task 10） |
| 6 | `source/assets/*.toml` | 18 / 3.2 KB | **需转换**：砍掉 9 份 `mkp_preset` 条目（doc §12.5），其余 9 条 BBS 元数据按新结构重写 |
| 7 | `assets/faq/` 29 份 | 29 / 5,372.7 KB | **舍弃**（D-4） |
| 8 | `assets/avatars/` 3 份 | 3 / 14.1 KB | **舍弃**（D-4：about） |
| 9 | `assets/models/model_*.webp` 8 份 | 8 / 30.9 KB | **舍弃**（D-4：`model_copy`）。**空档见 §5** |
| 10 | `assets/machines/` 那 6 张非归属机图 | 6 / 162.8 KB | **舍弃**（无引用、不属于我们支持的机型） |
| 11 | `assets/qr/` 8 个占位符 | 8 / 0 B | **舍弃**（整目录，无真图） |
| 12 | `assets/icons/machine/cantilever.svg.orig` | 1 / 374 B | **舍弃**（`.orig` 残留；不在 manifest，且与 `p1s.svg` 不同字节） |
| 13 | `models/*.zip` ×2 + `models/ZOffset_Calibration/` 目录 | 19 / 5.31 MB | **舍弃**（同一份字节的另外两种形态） |
| 14 | `presets/orca/**` | 3 / 2.7 KB | **舍弃**（G-3 已定：删除现有 Orca 文件；落地在 Task 16.4，此处只登记。3 份 = 2 个 JSON + 1 个 `.keep`） |
| 15 | `content/**`（17 份 282.4 KB）与 `source/` 里未提及的部分 | 17 | **不迁**：`content/` 是旧版**构建产物**，`source/` 里未列出的模块（about / faq / theme / event / notification / model_copy 等）按 D-4 不纳入 |

保留与转换合计约 **4.08 MB**（机器图 84.8 KB + 图标 1.1 KB + 模型 3.99 MB + BBS/套餐/资产元数据 17.7 KB），
舍弃合计约 **10.89 MB**。

## 4. 只读证据 / 「我们这边还引用它吗」（7.7）

**只读证据（拷贝没有 `.git`，所以不能靠版本控制证明，改用旧仓自己的账）：**

- `manifest.json` 的 **72 条 sha256 逐份核过：0 缺、0 不符** —— 这份拷贝与旧仓发布时的字节一致；
- 全仓文件 mtime 一致（`2026-09-14 19:26`，复制时的落盘时间），没有"后来又被改过"的孤点；
- 本次盘点只跑了读操作（`Get-ChildItem` / `Get-FileHash` / `Select-String` / `ConvertFrom-Json`），
  **没有写过、没有删过、没有移过**任何文件；Task 5 之后本仓新增的 judges 也把这条约束钉在
  `local-reference/` 之外（它本来就在 `.gitignore` 里，不入库）。

**我们这边还引用它吗：**

| 旧仓资产 | 我们仓里的引用 |
| --- | --- |
| `assets/machines/*.webp` | 机型定义的 `image` 是**裸文件名**（`a1.webp` / `p1s.webp` / …）—— 引用的正是这些名字，但**仓里没有对应文件**（悬空，doc §2.7） |
| `assets/icons/machine/*.svg` | 机型定义的 `icon` 是**裸名**（`a1` / `p1s` / …）—— 同上 |
| `assets/faq`、`assets/avatars`、`assets/models`、`assets/qr`、`models/*` | 代码里对它们的路径**零命中** |
| 我们自己的图 | 在 `public/printers/bambu/`（4 张，与旧仓同名不同文件）与 `public/models/hero_pile.webp`；引用点在 `src/app/heroArt.ts`、`PageHome` / `PageCalib` |

## 5. 两条要你知道的空档（不扩大范围，只记事实）

1. **模型预览图与模型名在被舍弃的那一侧。** `content/model_copy.json` 里每个模型有
   `title` / `subtitle` / `description` / `time` / `weight` / `tag` / `image` / `modelFile`。
   D-4 把 `model_copy` 列为不纳入，于是按规则**舍弃**；但保留的三份 `.3mf` 取出后是
   **没有名字、没有预览图**的 —— 将来做"模型库"界面时，这一块要么从 `model_copy` 里按需取
   （那就等于部分推翻了 D-4），要么由人重新命名。**这是取舍，不是遗漏**，所以写在这里。
2. **同一用途两份图。** 我们机型定义指向旧仓那几张（21.7 KB 起），而当前界面显示的是
   `public/printers/bambu/` 那几张（95.7 KB 起）—— 不是同一份文件。Task 9 搬运时要人看一眼
   定哪份（分辨率高的那张未必是想要的那张）。
