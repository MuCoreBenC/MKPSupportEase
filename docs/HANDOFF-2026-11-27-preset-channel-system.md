# 交接：版本化预设渠道与本地来源分组（2026-11-27）

> 分支：`fix/report-calibration-plate`  
> 状态：**施工中，未发布**；`publishSerial` 仍是 22。  
> 本文是换设备继续用的交接件；提交后以 `git log` / 本文件为准。

## 1. 这次要解决什么

用户目标不是“让新程序兼容旧预设”，而是建立一套长期交付系统：

1. 0.0.18 旧客户端继续用 0.0.18 的预设，不被新预设破坏；
2. 0.0.19 及以后每个客户端版本用匹配自己的预设；
3. 旧版本可以只发**预设数据补丁**（例如 0.0.18.1），不必发安装包；
4. 新客户端拿到旧预设、旧客户端拿到新预设，都要在下载/导入/列表阶段说清楚，不能等后处理才炸；
5. 本地预设按来源分组，同名、不同来源、不同发布代不要挤在一起。

已确认的产品取舍：

- 新程序**不强行兼容旧预设**；不自动转换、不自动补字段、不自动删字段。
- 旧预设权威字节留在 `v0.0.18` tag；当前工作树不把旧预设塞回 `presets/delivery` 之外的地方。
- 旧客户端改不了代码，只能靠服务端目录的签名/`minClient` 和它自己的 `readable` 闸挡住新数据。
- 0.0.18.1 只是**数据发布标签**，不是客户端版本；旧线补丁只能改已有键的值，不能加/删键或改类型。
- 不新开分支做投递；全部在 main 分支的目录里；tag 只作快照/记录，不能作为旧客户端更新地址。

## 2. 关键事实（换设备前必须知道）

### 2.1 旧客户端读的是写死的地址，不是 tag

0.0.18 安装包构建时注入的是：

```text
.../main/presets/delivery/source.json
```

它不读 git tag，也不会自动去 0.0.18.1 的新 tag 找数据。新 tag 对旧客户端不可见。

因此：

- 旧线要收数据补丁，必须继续更新 `presets/delivery`；
- 0.0.19+ 新构建改为读 `presets/channels/index.json`；
- 如果 0.0.19 数据写回 `presets/delivery`，0.0.18 会看到新数据（或被签名闸整次拒绝），做不到“旧客户端只看旧数据”。

### 2.2 catalog 用本地还是云端

运行时读的是**本地已释放的那份** catalog（应用数据根里的 `catalog.json`），云端是更新来源：

1. 客户端解析当前源，取云端 catalog；
2. 校验结构/版本/兼容矩阵；
3. 通过后才替换本地那份；
4. 随包 `catalog.generated.json` 只是兜底。

本地开发源也走同一套：下载 → 校验 → 释放，不是直接读云端字节。

### 2.3 序号不负责兼容

`publishSerial` 只回答“这个渠道的第几代”。兼容性由三层回答：

1. 渠道的 `minClient` / `maxClientExclusive` 范围；
2. `presetSchemaSignature` / `presetContractEpoch`；
3. 真实历史解析器矩阵。

没发布的本地数据也可以有本地序号（0 或本地递增），但不能冒充正式渠道。

### 2.4 兼容分级（2026-11-27 用户确认）

不是所有交付文件都值得做同一套细粒度兼容：

| 层级 | 文件 | 判据 |
|---|---|---|
| A · 细粒度 | MKP 预设 `.toml`、切片器预设 `.json` | 结构签名 / 契约代次 / 真实历史解析器矩阵；下载、导入、本地列表都要提前说清 |
| B · 简单版本 | 3MF 模型、图标、引导图、品牌图等静态资产 | 只看渠道 `minClient` + catalog 的 SHA/大小；**不跑**逐份历史解析器矩阵 |
| C · 随包 | `bundled` 资产 | 不进下载集合；随程序包走，不参与渠道兼容 |

当前代码已经按这个方向收窄：

- `presetSchemaSignature` / `presetContractEpoch` 只写给 `kind = mkp_preset` 的 TOML；
- `ensure_preset_readable`、兼容矩阵、导入探测都只处理 MKP 预设；
- 3MF/模型/图标走普通 `CatalogFile`，只有 SHA、size、`minClient`；
- 切片器 `.json` 的细粒度兼容仍是待补项：不能拿 MKP 的 TOML 解析器套它，要按它自己的 JSON 结构/切片器版本单独定。

**不要**把 3MF 这种大资产塞进历史解析器矩阵；那既测不出“预设是否兼容”，又会拖慢发布。

## 3. 当前渠道模型

```text
presets/
├── delivery/                         ← 0.0.18 旧线（旧安装包写死的地址）
├── legacy/0.0.18/delivery/           ← 旧线权威快照（从 v0.0.18 tag 恢复）
├── channels/
│   ├── index.json                    ← 0.0.19+ 新客户端读它选渠道
│   └── 0.0.19/
│       ├── assets/                   ← 迁移期临时复制，见“遗留问题”
│       └── delivery/
│           ├── catalog.json
│           ├── manifest.json
│           ├── source.json
│           └── mkp/presets/*.toml
```

`presets/channels/index.json` 当前登记：

- `legacy-0.0.18`：`presets/delivery`，范围 `[0.0.1, 0.0.19)`；
- `0.0.19`：`presets/channels/0.0.19/delivery`，范围 `[0.0.19, 0.0.20)`。

0.0.20 以后：在索引加一条 `0.0.20` 渠道即可；没有专属渠道时按 `fallbackTo` 回落。客户端不会自动跳到新渠道。

## 4. 本地/未发布数据怎么用

### 4.1 本地服务

只给一个 TOML 文件不够；客户端要读 `source.json` / 索引。最小可用结构：

```text
本地服务/
├── channels/index.json
└── channels/dev-0.0.19/
    └── delivery/
        ├── source.json
        ├── catalog.json
        ├── manifest.json
        └── mkp/presets/*.toml
```

- 渠道 id 用 `dev-0.0.19`，不要冒充正式 `0.0.19`；
- `publishSerial` 可以是 0 或本地递增；
- `minClient` 写当前程序版本；
- 客户端设置 → 数据源 → 自定义，指向 `channels/index.json`；
- 0.0.18 不读索引，不会误拿这个开发渠道。

如果只服务一份数据，也可以直接指向 `source.json`，不建索引；此时它就是一个单渠道，靠 `minClient` + 当前解析器探测判断兼容性。

### 4.2 未发布分支

分支不会自动被发现：

- 自定义源必须指到该分支的 `source.json` 或索引；
- tag 只是快照，不能作为旧客户端更新地址；
- 未发布分支没有正式账本/序号，应使用 `dev-*` 渠道和本地账本；
- 旧客户端不会因为“分支里有文件”就自动切换。

### 4.3 本地测试源（开发）

客户端 debug 构建支持 `MKPSE_PRESET_SOURCE_URL` 覆盖，但当前代码仍是按“交付根/`source.json`”解析，**尚未把本地 index 的自动识别接到这个覆盖入口**。目前最稳的是直接指向 `source.json`，或后续补 `presets dev` 命令。

## 5. 已完成的改动

### 5.1 数据迁移

- `presets/delivery` 已从 `v0.0.18` tag 原样恢复；`git diff v0.0.18 -- presets/delivery` 为 0。
- 当前 0.0.19 数据已复制到 `presets/channels/0.0.19/delivery`。
- `presets/legacy/0.0.18/delivery` 保存旧线权威快照。
- `presets/channels/index.json` 已建。
- `gen-catalog` 默认写 `presets/channels/0.0.19/delivery`，并拒绝重建 `legacy-0.0.18`，避免当前 schema 覆盖旧线。
- 0.0.19 渠道 catalog 带 `channelId: "0.0.19"`，`minClient: 0.0.19`，`publishSerial: 22`。

### 5.2 客户端渠道解析

- `runtime::source` 增加渠道索引模型、版本范围选择、`fallbackTo`、渠道相对路径校验、`bootstrap_candidates` 识别 `index.json`。
- `workbench/bootstrap.json` 默认地址改为 `presets/channels/index.json`；0.0.18 已发布包不受影响。
- `Catalog.channel_id` 可选字段；旧客户端忽略。
- `SourceIdentity` 增加可选 `channel`；`folder_key()` 让渠道参与本地文件夹，但 `key()` 仍兼容旧来源认亲。
- 选择规则：先按 `[minClient, maxClientExclusive)`；没有精确渠道时按最新下界回落；`fallbackTo` 显式一跳。

### 5.3 本地来源文件夹

- `branch_dir::folder_for` 不再对 `NO_REF` 返回平铺；自定义源用 `host-port` / repo 路径生成一层文件夹。
- 本地文件 DTO/契约增加 `folder`；本地列表按 folder 排序并在表里画组头。
- 同一来源、不同发布代用 `(vN)` 后缀；同代仍 `-2`。
- 旧平铺文件不迁移，归“未分文件夹”组。
- 四语文案补了 `presets.tree.localUnfiled` / `localIncompatible`。

### 5.4 CLI / 发布器（部分）

- `presets` CLI 支持 `--channel ID`，默认 `0.0.19`；`legacy-0.0.18` 必须显式。
- `workbench::paths::delivery_root_at` 读取 `MKPSE_PRESET_CHANNEL`，CLI 可写渠道目录。
- `gen-catalog` 写渠道；`audit` 的结构规则表会向上找到 `presets/structure-signatures.toml`。
- `presets check --channel 0.0.19` 现在 18 项通过，只剩“工作区脏”这一项预期红。

## 6. 没完成 / 遗留问题

1. **每渠道独立序号与账本**：目前 `publish-serial.json` 仍是单交付根；需要按渠道拆，禁止跨渠道混用。
2. **legacy 值补丁工具**：旧 schema 不能用当前生成器重建；需要 `presets/legacy/0.0.18/` 源 + 只改已有键值的工具 + 旧解析器矩阵。
3. **`--data-only`**：尚未真正实现；CLI 帮助里已不宣称可用。
4. **工作台 UI/发布页**仍按单交付根；CLI 的渠道参数只先通了路径。
5. **渠道资产布局（临时）**：`source.json.filesRoot` 目前只认 `.` / `..`，所以 `0.0.19` 渠道暂时带一份约 5.7MB 的 `assets`，随本次分支提交以保证换设备可运行；最终应改 resolver/audit 支持共享 `presets/assets`，不要让渠道长期复制大资产。
6. **渠道索引校验**：范围重叠、循环回落、路径越界还缺完整测试/发布闸。
7. **切片器 `.json` 细粒度兼容**：当前逐份矩阵只覆盖 MKP `.toml`；切片器 profile 需要按自己的 JSON 结构/切片器版本单独做，不能套 TOML 解析器。
8. **本地测试源 index 入口**：`MKPSE_PRESET_SOURCE_URL` 还不能自动识别本地 index；建议补 `presets dev --channel dev-0.0.19`。
9. **文档/全量测试/发布**：未做；工作区改动包含用户原有 AMS/后处理改动，不能粗暴覆盖。

## 7. 当前进度估计

- 数据迁移与 legacy 恢复：约 85%
- 客户端渠道解析：约 70%
- 本地来源文件夹/UI：约 80%
- 发布器/每渠道账本/legacy 补丁：约 30%
- 测试/文档/全量回归：约 25%

整体约 55%。**没有发布，没有推进序号。** 本文随当前分支一起提交，供换设备继续。

## 8. 换设备继续的检查顺序

1. 先看本文件 + `docs/PRESET-COMPATIBILITY.md`；
2. `git status` 确认沿用当前分支；
3. `cargo check --workspace --all-targets`；
4. `npm run presets -- check --channel 0.0.19`（当前只剩 git/clean 红）；
5. `node node_modules/typescript/bin/tsc -b`；
6. `node node_modules/vitest/vitest.mjs run src/app/presets/presetTree.test.ts`；
7. 继续做：per-channel ledger → legacy 补丁工具 → `presets dev` → 发布闸 → 全量测试。

## 9. 回滚

- 旧线原始快照：`.scratch/legacy-0.0.18/`（本机；不入库）；
- 仓库内权威快照：`presets/legacy/0.0.18/`；
- `presets/delivery` 与 `v0.0.18` tag 逐字节一致；
- 迁移前的 0.0.19 数据已复制到 `presets/channels/0.0.19/`。

## 10. 用户问题与结论（备查）

### 10.1 旧客户端为什么不能直接看新 tag

旧安装包的默认地址是构建时写死的，tag 不会改变它的请求地址；要“不装新包收补丁”，只能继续更新它读的旧目录。

### 10.2 0.0.20 怎么选

0.0.19+ 读同一个 index；0.0.20 命中自己的范围；没有专属渠道时按 `fallbackTo` 回落。旧 0.0.19 继续用 0.0.19，不会被 0.0.20 数据影响。

### 10.3 未发布本地服务有没有序号

有本地渠道序号，但没有正式发布账本；序号不代表兼容性。兼容性看 `minClient` + schema/contract + 矩阵。

### 10.4 为什么“能解析不等于兼容”

字段删/改/类型变化可能解析成功但取默认值，造成静默换值；必须用真实旧 tag 解析器跑矩阵，不能只用新解析器模拟。

### 10.5 本地预设为什么按来源分一层

同名文件来自不同数据源或不同发布代时，平铺会挤在一起；一层来源文件夹 + `(vN)` 后缀是简单且可回退的方案，旧文件不迁移。
