# 资产域①层：集中式资产定义与目录约定（b05 Task 8）

> 这一份记的是**形状与约定**（8.1–8.4），以及实现落在哪、判据盯什么（8.5–8.8）。
> 已裁决的三条（模型保留 / 名称与预览图暂缓 / 机型图用新版）记在
> `asset-inventory.md` §5，这里只引用不重复。

---

## 1. 数据形状（8.1）

`presets/assets.toml` —— **一份集中定义**，`[[assets]]` 数组表（与 `brands.toml` 同一写法）。

| 字段 | 必填 | 为什么是它 |
| --- | --- | --- |
| `id` | ✅ | 主键。**查表、反查、交付索引全用它**，不用路径 —— 路径会改，id 不改（与上游 `mkpPresetAssetId` 同一条理由）。**大小写不敏感唯一** |
| `type` | ✅ | 类型，见 §2 |
| `machineId` | ⬜ | 归属机型。可以没有（将来的公共素材）；有就必须是**真机型**（跨文件那条在 `Presets::check_cross_consistency` 里查） |
| `name` | ✅ | 给人看的名字。界面上的"这张图叫什么"不该靠文件名猜 |
| `path` | ✅ | 相对**资产根**的文件位置。**唯一一份路径** |
| `slicer` / `profile` | 仅切片器 | 见 §2、§3 |

**刻意不写的两类**：

- **不写 `fileName`**：旧仓同时存 `fileName` 与 `relativePath`，两处说的是同一件事，
  改名时必然有一处忘掉（doc §12.5 的清单里那两个字段就是这么来的）。
- **不写 `sha256` / `size`**：那是**交付物**的属性，发布时按真实字节算（Task 13.6）。
  写进①层就是第二份真相，而且它一定先过期。

## 2. 类型集合（8.2 / 8.3）

`image` · `icon` · `model` · `slicerProfile`

| 类型 | 说明 |
| --- | --- |
| `image` | 位图素材（机型图等）。与 `icon` 分开是**消费方式**不同：一处 `<img>`，一处当符号用 |
| `icon` | 矢量标记（实测是 `.svg`） |
| `model` | 模型本体（`.3mf`）。**保留** —— 裁决：名称与预览图**暂缓**，本轮不虚构元数据 |
| `slicerProfile` | 切片器预设。`slicer`（今天只有 `bbs`）是 G-3 留的**开放扩展维度**，`profile` 是该切片器下的档位（旧仓的 `category`，实测都是 `process`） |

**没有 `mkpPreset`**（doc §12.5）：那份路径由命名规则算出，登记一份就是冗余。
这不是一句注释 —— **enum 里没有这一档**，想登记得先改 enum，而改 enum 要过一次 review。

## 3. 目录约定（8.4）

```text
资产根 = <repo>/public/assets/          # 文件本体
定义   = <repo>/presets/assets.toml     # ①层，人改
URL    = /assets/<path>                 # vite 的 public/ 直通
```

三条决定与理由：

1. **在 `public/` 下**：这些文件要能被前端**按 URL 取**（机型图、图标、模型），
   而 vite 的 `public/` 是唯一"原样进产物、按路径直通"的目录。
2. **取子根 `assets/` 而不是 `public/` 本身**：`public/` 里还有 hero 图之类的**界面素材**，
   两类混在一层，`path` 就说不清"这条资产属于谁管"。约定：**我们管的资产全在
   `public/assets/` 里**，别的 `public/` 文件不许被 `assets.toml` 引用。
3. **`path` 永远相对资产根**，不相对定义文件所在目录 —— 定义（①层，`presets/`）与
   载荷（②层，`public/assets/`）本来就分家，路径的基准只该有一处。

根不存在时**按需建**（`paths::assets_root()`）：`resolve_in` 的第三道要比真实路径，
根不存在的话每条 `path` 都会解析失败。空目录靠 `public/assets/.gitkeep` 进库。

## 4. 实现落在哪、判据盯什么（8.5–8.8）

| 位置 | 是什么 |
| --- | --- |
| `presets/assets.toml` | 定义文件本身（现在是**空骨架**，形状与约定写在头部注释里） |
| `src-tauri/src/workbench/presets/assets.rs` | 模型 + 加载 + `add` / `write` + 判据 |
| `src-tauri/src/workbench/presets/mod.rs` | `Presets.assets` 接入 + 跨文件那条（归属机型必须存在） |
| `src-tauri/src/workbench/paths.rs` | `assets_root()` |
| `src-tauri/src/workbench/app/assets.rs` | `wb_assets`（**只读**）+ DTO |
| `src/workbench/api.ts` | `AssetKind` / `AssetView` / `AssetList` + `wb.assets()` + `assetUrl()` |

判据（每条都有反空转输入）：

| 判据 | 坏掉时的样子 |
| --- | --- |
| `ids_are_unique_case_insensitively` | `a1-image` 与 `A1-Image` 两台并存 ⇒ "某张图指向哪一条"取决于遍历顺序 |
| `path_must_stay_inside_the_asset_root` | `../` / 绝对路径 / `a/../../b` ⇒ 定义能把人指到仓库外 |
| `a_typo_in_a_key_is_loud` | `pth = '...'` ⇒ 静默少一个字段 |
| `slicer_fields_belong_to_slicer_profiles_only` | 切片器预设缺 `slicer` ⇒ 不知道该给谁读；非切片器带 `slicer` ⇒ 语义污染 |
| `the_machine_attribution_must_be_a_real_machine` | 归属一台不存在的机型 ⇒ 界面上只表现为"这台机型的图没了" |
| `a_zero_edit_roundtrip_is_byte_identical` | 保格式写回的底：没改过就必须逐字节相同 |
| `add_appends_one_block_and_writes_once` | `add` 与加载期**走同一套检查**（越界路径也要被拦） |
| `the_source_files_keep_lf_line_endings` | ①层 `.toml` 是 CRLF ⇒ 第一次保存整份被 `toml_edit` 改写成 LF（真踩过） |

## 5. 分工：Task 8 做了什么、剩下是谁的

| 事 | 谁 | 状态 |
| --- | --- | --- |
| 形状、类型、目录约定、数据层、读命令、前端类型 | Task 8 | ✅ 本轮 |
| **机型定义的 `image` / `icon` 改指资产 id**（原 8.6） | **Task 9** | ➡️ 顺序调整，理由见下 |
| 把文件搬进 `public/assets/`、为每份文件写条目（9.1/9.2） | Task 9 | ⏳ |
| BBS 9 份 JSON 与其元数据（9.3）、反查（9.4/9.5）、清理 `public/` 硬编码引用（9.6） | Task 9 | ⏳ |
| 「资产 id 能解析到真实文件」升级成校验层的一条 | Task 11.1 | ⏳ |
| 资产库界面（导入 / 预览 / 改名 / 删除 / 选择） | Task 14.6 | ⏳ |

**8.6 为什么要挪到 Task 9**：把机型定义的 `image` 从 `'a1.webp'` 改成资产 id，前提是
**那些 id 与文件已经存在**。先改引用、后搬文件，会造出一批"指向不存在资产"的引用 ——
而那正是 9.7 要拦的东西；判据一红，人分不清是"还没搬"还是"写错了 id"。
所以 Task 9 一次做完三件事：**搬文件 → 写条目 → 改引用**，每一步都能验。
