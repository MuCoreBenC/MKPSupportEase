# P0 审计证据：MKPSupportEase × mkp-ssr

固定的 commit（两仓都是本机工作副本，不是 GitHub 网页读取）：

| 仓库 | 路径 | commit | 分支 | 工作区 |
|---|---|---|---|---|
| MKPSupportEase | `G:\project\MKPSupportEase` | `8bf3fcb` | `feat/b04-source-pipeline-plan` | 干净 |
| mkp-ssr | `G:\project\mkp-ssr` | `a2a47ca` | `main` | **有未提交改动**（`docs/HONEST-BOUNDARIES.md`、`make.bat`、`scripts/*`；不涉及 preset 链路） |

`INTEGRATION-PLAN.md` 写的是「GitHub 连接可读取」。更正一条：**mkp-ssr 就在本机**，
所以这一轮证据是读真文件 + 数真行数得到的，不是读网页。

---

## 1. 最重的一条：两份参数注册表已经分岔了

| | 我们 | mkp-ssr |
|---|---|---|
| 路径 | `presets/registry/param_registry.toml` | `crates/preset/assets/param_registry.toml` |
| 字节 | 56 902 | 56 738 |
| 行数 | 2 460 | 2 448 |
| sha256[:12] | `56bdcec93ad3` | `a837a25bd064` |
| `[[params]]` 条数 | **74** | **74** |
| key 集合 | — | **完全相同** |
| key 顺序 | — | **完全相同** |
| 正文 | — | **不同** |

74 条 key 一个不差、顺序也一样，但正文有实质差异。前四处（行号按各自文件）：

| 行 | 我们 | mkp-ssr | 性质 |
|---|---|---|---|
| 228 | `label = '装载胶笔 G-code'` | `label = '下笔 G-code'` | 文案，我们改过 |
| 261 | `label = '卸载胶笔 G-code'` | `label = '抬笔 G-code'` | 文案，我们改过 |
| 463 | `uiComponent = 'select'` | `uiComponent = 'segmented'` | **控件类型不同** |
| 1074+ | 多出一整块 `[[params.choices]]`（`value = 0.0` 关闭 / `value = 50.0` …） | 该处直接是 `[params.layout]` | **我们给这个参数加了枚举选项** |

`mkp-ssr` 侧那份是**编译进二进制**的：
`crates/preset/src/lib.rs:65` `pub const PARAM_REGISTRY_TOML: &str = include_str!("../assets/param_registry.toml");`，
解析入口 `crates/preset/src/registry.rs:162-172`（带反空转断言），
`crates/preset/src/registry.rs:282-290` 还断言 `params.len() >= 70` / `tabs.len() >= 8`。

**结论**：这正是 `INTEGRATION-PLAN.md` 第 4 节禁止的「两套注册表各自为权威源」，
而它**已经发生了**，不是风险预测。差异方向是单向的（我们这边更新），
所以唯一权威源定在 `MKPSupportEase/presets/registry/param_registry.toml`，
`mkp-ssr` 那份 `assets/param_registry.toml` 应当**由我们这边生成/同步过去**，不再手工维护。

复现方法（不需要脚本）：按 `[[params]]` 切块取每块第一行 `key = '...'` 比集合与顺序；
条数用 `rg -c "^\[\[params\]\]$"` 各文件；差异用逐行 zip 比对。

---

## 2. 内置预设 9 份 = 我们清单 10 个版本里的 9 个

`crates/preset/src/lib.rs:76-112` 的 `BUILTIN_PRESETS: &[(&str, &str)]`（**数组不是 map**，
元素是 `(文件名, include_str! 内容)`，表是手写的），9 项与我们 `presets/machines/*.toml`
的对应关系：

| 我们的清单 | 我们的 `presetFile` | mkp-ssr 内置文件名 | 对得上？ |
|---|---|---|---|
| A1/STANDARD | `A1.toml` | `A1-standard.toml` | 同一份配方，**名字两套** |
| A1/FAST | `A1F.toml` | `A1-fast.toml` | 同上 |
| A1/FASTV3.3 | `A1F_260628.toml` | `A1-fastv3.3.toml` | 同上 |
| A1_MINI/STANDARD | `A1M.toml` | `A1_MINI-standard.toml` | 同上 |
| A1_MINI/FAST | `A1MF.toml` | `A1_MINI-fast.toml` | 同上 |
| A1_MINI/FASTV3.3 | `A1MF_260628.toml` | `A1_MINI-fastv3.3.toml` | 同上 |
| **A2L/STANDARD** | **（空）** | **没有** | 它没有产物，也没有尺寸 |
| P1S/LITE | `P1.toml` | `P1S-lite.toml` | 同上 |
| P2S/STANDARD | `P2.toml` | `P2S-standard.toml` | 同上 |
| X1C/LITE | `X1.toml` | `X1C-lite.toml` | 同上 |

两条硬事实：

1. **10 − 1 = 9**：内置表不是随手挑的 9 份，它就是我们清单去掉 A2L。
   A2L 那台在我们这边是四处皆空（无尺寸、无产物、无机型差异），在消费端也就没有内置预设。
   两边对这台机器的判断一致 —— 这是一条很强的同源证据。
2. **命名口径是两套**：消费端是 `{机型}-{版本小写}.toml`，我们的 `presetFile` 是
   `A1F_260628.toml` 这种历史命名。而我们 `wb_generate` 写出去的文件名取的是
   **上游 manifest** 的 `mkp_preset.file_name`（`build.rs:193-197`，兜底 `{机型}_{版本}.toml`），
   也就是 `A1.toml` 那一套 —— **我们生成的产物名，消费端认不出**。

`sync_builtin_presets`（`src-tauri/src/builtin.rs:26`）把内置表同步到
`<数据根>/presets/builtin/`（`settings.rs:28` `PRESETS_BUILTIN_SUBDIR = "presets/builtin"`，
`settings.rs:210-211` 拼接）。策略是**内容相同就跳过（连 mtime 都不动）、不同才原子覆盖**
（`builtin.rs:33-42`），并**清掉目录里配方不认的 `.toml`**（`builtin.rs:49-60`）。

---

## 3. 产物格式：基本对上，两处差异

`mkp-ssr/crates/preset/assets/presets/A1-standard.toml`（99 行）前 9 行：

```toml
# uuid: f444aeaf-d447-453f-8871-cf7645c2d5df
# release_time: 2026-08-19 01:38:13
# machine: A1
# variant: standard
#胶笔配置

[toolhead]
offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移
speed_limit = 70 # 涂胶速度上限 (mm/s)
```

我们 `render()`（`app/build.rs:98-202`）生成的形状与它一致：头五行同序
（`build.rs:121-125`）、按 `section` 分段且段序取 `layout.order`（`build.rs:133-141`）、
内联表成员名取 `jsonKey`（`build.rs:166-169`）、多行 G-code 用三引号且注释在上一行
（`build.rs:179-182`）、行内注释取 `tomlComment`（`build.rs:205-215`）、
整数不写小数点（`build.rs:219-233` 的 `scalar`）。

**差异两处**：

1. `release_time`：我们写 RFC3339 UTC（`clock.rs:17-23` → `2026-09-23T..Z`），
   它是 `2026-08-19 01:38:13`（空格分隔、无时区）。
2. 文件名口径（见 §2 第 2 条）。

这两条都要在 P1 的契约里定死，否则消费端读到的是"格式对、名字不对"的产物。

---

## 4. 消费端读取链路（真实调用点）

依赖方向由 Cargo 强制单向：`mkp-ssr → {mkp-preset, mkp-pp}`，`mkp-preset → mkp-pp`，
`mkp-pp` 无本地依赖（`Cargo.toml:11` members = `crates/*` + `src-tauri`；
`crates/preset/Cargo.toml:25` `mkp-pp = { path = "../core" }`；
`src-tauri/Cargo.toml:42-43`）。理由写在 `crates/preset/Cargo.toml:3-6`。

`load_ir()`：`crates/preset/src/lib.rs:151-212`，签名
`(preset_path: &Path, gcode_text: Option<&str>) -> Result<mkp_pp::ir::Ir, PostprocError>`。
九步：读预设(159) → 规范机型名(163) → `validate_config`(165) → 载注册表(167) →
`validate_against_registry`(169-174) → 空机型拦截(176-180) → `build`(182) →
`preset_name`(184-187) → 归一 + 尺寸表命中(189-208) → `fill_machine_facts`(210)。

**产品代码只有三个调用点**，IPC 都隔一层：

| 调用点 | 用途 | 对应 IPC |
|---|---|---|
| `src-tauri/src/hook.rs:253` | 切片器钩子，**唯一喂 G-code 的** | — |
| `src-tauri/src/ir_view.rs:95` | IR 只读视图 | `commands.rs:461-471 preset_ir` |
| `src-tauri/src/preset_apply.rs:210` | **写回闸 3 的复检**（写完再过一遍内核校验） | `commands.rs:498-505 preset_apply`、`514-526 preset_edit_or_fork` |

注册表校验拒绝时统一是 `PostprocError::InvalidConfig`（`validate.rs:41-42`），两种话术：
`validate.rs:279-284`「不在允许的选项里」、`validate.rs:303-309`「超出区间」。
两条豁免：`validate.rs:299-301` **值为 0 视为未设置直接放过**、`:290-292` 没量程不校验。
区间支持 `机型:变体` 覆盖优先（`registry.rs:205-229`、`259-269`）。

**这条豁免值得记一笔**：我们这边把「0」当成一个真实的值（`0` 与"没这一行"是两件事），
而消费端校验把 `0.0` 当"未设置"跳过。两套语义不冲突（它只是少校验一项），
但 P1 写契约时要明确说出来，不能让人以为消费端会拦住 0。

---

## 5. 我们这边生成链路的真实完成度（含三条缺陷）

五条命令（全在 `app/build.rs`，`lib.rs:142-146` 注册）：

| 命令 | 行 | 真写盘 |
|---|---|---|
| `wb_preflight` | 75-83 | 不写 |
| `wb_preview_toml` | 408-416 | 不写 |
| `wb_generate` | 318-405 | **写** `dist-presets/presets/mkp/{file_name}`（373）+ 生成快照（378-381） |
| `wb_revert_preview` | 445-507 | 不写 |
| `wb_publish` | 540-612 | **写** `dist-presets/manifest.json`（599） |

`wb_generate` 自己**不落 `built.json`** —— 它把 `Patch::MarkBuilt` 交给前端
（`build.rs:392-397`），走 `wb_apply_draft`（`patch.rs:743-753`）再由 `wb_save`
落盘（`storage.rs:461-467`）。

阻断只有一条闸门：`issues::inspect` 的 `first_block()`（`build.rs:325-329` 与
`547-550`）。`Severity::Block` 全仓只有三种来法（`issues.rs:230-242` 显示条件成环、
`286-303` 值越界、`306-322` 枚举值不在选项里）。

**三条缺陷**（都有行号，都还没修）：

1. **我们输出的 manifest 里 `bundles` 是悬空引用**：`build.rs:588-599` 把
   `ctx.up.manifest.bundles()` 原样抄进去，而我们自己算的 `assets`
   （`build.rs:574-584`）**只含 `mkp_preset` 一类**，一条 `bbs_profile` 都没有。
   上游那份有专门的一致性检查拦这个（`upstream/manifest.rs:462-476`），
   **我们输出的这份没有过任何检查**。
2. **没有上游产物的版本永远进不了清单**：`build.rs:560-562` 在
   `mkp_preset == None` 时 `continue`。A2L/STANDARD 与任何新建版本都会被静默跳过。
3. **`channel` / `minimumClient` / `version` 直接透传上游**（`build.rs:588-599`）——
   上游一旦删掉，`wb_publish` 就没有这几个字段的来源。

另外 `paths::resolve_dist`（`paths.rs:73-75`）**全仓零调用点**，是死代码。

我们这边**没有任何一处**提到 IR / `load_ir` / 内置预设 / 消费端 —— 两仓目前在代码层面
零接触，唯一的耦合是产物文件的格式与名字。

---

## 6. 尺寸 / 别名 / 禁区：三类数据**逐字段等值**

补齐上一版列在「未查明」里的那件事。结论与注册表相反：**这三类完全一致，一处差异都没有**。

消费端这三份都嵌在 `mkp-pp` 里（`crates/core/assets/`）：
`machine_dimensions.json`（3 651 B，`machine_dims.rs:14` `include_str!`）与
`machine_catalog_extra.json`（1 990 B，顶层键 `aliasMap` / `forbiddenZones`，
`machine_dims.rs:133-138` 懒加载）。

### 6.1 机型尺寸：125 个字段全等

有尺寸的机型两边是**同一批 5 台**（A1 / A1_MINI / P1S / P2S / X1C），
没尺寸的两边都是 A2L（它在尺寸表里根本没有条目）。每台 25 个字段，
5 × 25 逐个比：**值不同 0，只在我们 0，只在消费端 0**。

比对时把 `camelCase` 与 `snake_case` 归一（我们 `bedSize.width`，它 `bed_size.width`），
数值按 f64 比。复现方法：两边各自拍平成点分键，键名归一后逐键比。

### 6.2 机型别名：23 条双向全覆盖

`aliasMap` 23 条（大写别名 → 规范机型 id）。逐条核对：
**消费端的 23 条我们全部认得；我们 `externalAliases` 里声明的，消费端一条不缺。**

| 机型 | 我们的 `externalAliases` | 消费端指向这台的别名 |
|---|---|---|
| A1 | `A1C` `A1F` | 上述 2 条 + `A1` |
| A1_MINI | `A1MINI` `A1M` `A1MC` `A1MQ` `A1MF` | 上述 5 条 + `A1_MINI` |
| A2L | （无） | `A2L` |
| P1S | `P1` `P1SC` `P1P` `P1C` | 上述 4 条 + `P1S` |
| P2S | `P2` | `P2` `P2S` |
| X1C | `X1` `X1S` `X1E` `X1 Carbon` `X1CARBON` | 上述 5 条（大写 `X1 CARBON`）+ `X1C` |

归一函数 `normalize_to_canonical`（`machine_dims.rs:148-153`）：
查不到别名**返回空串**，不是原样返回。

### 6.3 禁区：点集逐点相同

3 台（P1S / P2S / X1C）各 2 块多边形，**点集逐点相同**。结构差异只在容器：
我们是 `presets/forbidden_zones/{机型}.toml` 的 `[[zones]]`，
它是 `forbiddenZones[机型][]`，每块的键叫 `points`。

消费端填禁区的地方 `machine_dims.rs:157-170` `populate_forbidden_zones`：
**未命中只记 warn 不阻塞**（「禁区数据为空，跳过禁区填充（碰撞检测将不包含禁区）」）。
禁区丢了不会报错，只是碰撞检测少一块 —— 这条要进契约。

### 6.4 一个直接后果：A2L 在消费端**根本跑不通**

`load_ir` 第 8 步（`crates/preset/src/lib.rs:199-208`）：

```rust
if !has_machine_dimensions(&ir.machine.machine_type) {
    return Err(PostprocError::MissingMachine { path: format!(
        "…归一为 {:?}，但内置机型尺寸表里没有它 —— 别名表认识这个名字、\
         尺寸表却没有对应条目，属于内置数据不一致，请换一个机型）", …
```

别名表有 `A2L`、尺寸表没有 → **就算给 A2L 生成一份预设，`load_ir` 也会直接拒**。
消费端自己有测试钉着这个不一致（`crates/core/tests/machine_dims_must_exist.rs:6-7`
明写「6 个机型…而 `machine_dimensions.json` 只有 **5** 个条目 —— **没有 A2L**」）。

对我们的含义：A2L/STANDARD 现在被 `wb_generate` **静默跳过**（`build.rs:560-562`），
而那个"静默"恰好掩盖了真问题。正确做法二选一 —— 要么在尺寸页把 A2L 的尺寸补上，
要么生成时**明确阻断并说清原因**（「这台机器没有尺寸，消费端会拒掉整份配方」）。

### 6.5 与 §1 放在一起看

| 数据 | 两仓状态 | 说明 |
|---|---|---|
| 参数注册表 | **已分岔**（4 处实质差异） | 只有它我们改过 |
| 机型尺寸 | 逐字段等值（125/125） | 没人改过 |
| 机型别名 | 双向全覆盖（23 条） | 没人改过 |
| 禁区 | 点集逐点相同 | 没人改过 |

所以分岔不是结构问题，是**「谁改过谁没改」**的问题。这也说明：权威源定下来、
生成入口建好之后，其余三类的迁移**不需要任何数据修正**，只要把生成那一步接上。

---

## 7. 逐项迁移表（P0 结论，P1 才冻结）

| 项 | 权威源 | 处置 | 依据 |
|---|---|---|---|
| `param_registry.toml` | **我们** | mkp-ssr 那份改为从我们这边生成/同步；禁止手改 | §1 |
| 机型与版本清单 | **我们**（`presets/machines/*.toml`） | 消费端不维护清单 | §2 |
| 内置预设 9 份 | **我们生成** | `BUILTIN_PRESETS` 手写表改为按清单校验（缺一份要报错） | §2 |
| 产物文件名 | 待定案 | 必须定一套；我们现在依赖上游 manifest 的 `file_name`，这条依赖要断 | §2、§3 |
| `release_time` 格式 | 待定案 | 定一种；现在两边不同 | §3 |
| **机型尺寸** | **我们** | 已逐字段等值，只需建生成入口产出 `machine_dimensions.json` | §6.1 |
| **机型别名** | **我们** | 已双向全覆盖，同上（`aliasMap` 由 `externalAliases` 生成） | §6.2 |
| **禁区** | **我们** | 已逐点相同，同上（注意消费端丢禁区只 warn 不报错） | §6.3 |
| **A2L 的尺寸** | **我们** | 二选一：补尺寸，或生成时明确阻断（现在是静默跳过） | §6.4 |
| TOML → IR（`load_ir`） | **mkp-ssr** | 迁入我们仓库，**不另写同义转换器** | §4 |
| 后处理内核 `mkp-pp` | **mkp-ssr** | 整体迁入，依赖方向保持单向 | §4 |
| 写回复检（闸 3） | **mkp-ssr** | 复用它的做法：写完用 `load_ir` 复检一遍 | §4 |
| manifest 生成 | **我们** | 修那三条缺陷，并给它一条一致性检查 | §5 |
| `paths::resolve_dist` | — | 删（死代码） | §5 |

---

## 8. 本轮未查明 / 刻意没做

- `crates/preset/src/generate.rs` 与 `bin/gen_presets.rs` 具体生成什么、输入是哪份数据 —— 只确认了它们存在（`crates/preset/Cargo.toml:50-52`），没读实现。
- `mkp-ssr` 的 `src-tauri` 运行时用户预设目录扫描、只读/可写目录划分、以及"找不到就用内置兜底"是否静默 —— 本轮没读完。
- K-P1/K-P2/K-P3 三个写入风险标记的确切定义位置。
- 没有跑过任何一侧的测试，没有改动任何数据文件。

