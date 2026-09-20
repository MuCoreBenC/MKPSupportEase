# MKP 后处理 · 3D 展示模型资产契约

> **contract v3** ｜ 2026-09-14 ｜ 取代 v2
> 对接方：machine-motion（上游模型/资产提供方）｜ 消费方：MKP 后处理桌面端
> 变更规则：机器可读产物的**字段**变化才升版本号，字段只增不删。文档措辞调整不升版本。

---

## 0. 契约分层：数据是规格，文档是解释

这是 v3 唯一真正重要的一节。

上游一共交三样东西，**只有这三样是规格**：

| 产物 | 层次 | 内容 |
| --- | --- | --- |
| `manifest.json` | 索引层 | 哪台机器有 3D、版本号、文件名、校验值 |
| `{model}.variant.json` | 视觉档案层 | 材质档定义、机位、fov、曝光、色调映射、环境配方、默认角度、兜底图映射 |
| `.glb` | 几何层 | 几何 + 烘好的 PBR 材质（含材质档） |

**README、回执、本契约文档都不是规格**，它们只负责解释"为什么这么定"和"怎么产出"，给人看、给以后查账用。

### 0.1 铁律

> **应用侧不得硬编码任何来自文档、回执或聊天记录的数值。**
> **本契约文档里出现的所有数字都是示例，不是规格。**

反例（v3 明确禁止）：

```
✗ 「README 说 FOV 是 28，所以我这里写 28」
✗ 「回执说 opacity 是 0.15，所以我复制一个 0.15」
✗ 「契约 §6 的示例里 envMapIntensity 是 1.0，所以我写死 1.0」
```

正确：

```
✓ variant.json 说 28，所以我消费 28
✓ variant.json 没给这个字段，所以我用应用内置默认值，并在开发模式 warn
```

这条为什么重要：上游改了标定（换了棚灯、改了曝光），只要重出 `variant.json` + 升版本，用户端下次刷新就对了，**MKP 不需要改代码、不需要发版**。一旦某个数值被抄进 MKP 的源码，这条链路就断了，而且断得很安静。

### 0.2 双向的，不只约束 MKP

同一条铁律反过来也成立：**上游不要依赖 MKP 的文档来决定标定值**。上游标定完就把结果写进 `variant.json`，不需要问"MKP 那边曝光设的是多少"。曝光就是你给的那个数。

唯一的例外是 §12 的**钳制范围** —— 那是安全阀，不是审美参数。

---

## 1. v3 相对 v2 改了什么

| # | 变更 | 原因 |
| --- | --- | --- |
| 1 | 新增 **`{model}.variant.json` 视觉档案层**，把 v2 里散在文档正文的渲染参数（fov / 曝光 / tone mapping / 环境配方 / 材质档映射）全部收进数据文件 | 下游不应依赖回执与文档 |
| 2 | v2 §5.2 那张"应用侧固定的渲染参数"表**作废** —— 那正是"把数值写在文档里"的错误示范 | 同上 |
| 3 | 材质档 → `MATERIAL_PROFILE` 的映射从"交付说明里写明"改成 `variant.json` 的 `sourceProfile` 字段 | 同上 |
| 4 | 环境配方从独立 `envRecipe` 文件并入 `variant.json`（仍可用 `$ref` 指向共享文件） | 少一个要对齐的文件 |
| 5 | 新增 §12：**应用侧的校验与钳制规则**（缺字段、越界、未知枚举各自怎么处理） | 让"消费数据"这件事可落地、可预期 |
| 6 | `variant.json` 纳入不可变 + 版本化 + sha256 校验，与 glb 同等对待 | 它现在是规格的一部分 |

v2 的其余条款（材质档双档、坐标约定、体积预算、兜底图规格、缓存策略）**全部保留**，只是数值化的部分改成从 `variant.json` 读。

---

## 2. 交付物清单（首批：A1 mini 一台，含笔架件）

| 交付物 | 数量 | 必需 | 层次 |
| --- | --- | --- | --- |
| `manifest.json` 条目 | 1 条 | 是 | 索引 |
| `{model}.variant.json` | 1 份 | 是 | 视觉档案 |
| `.glb`（几何 + 两套材质档） | 1 份几何 | 是 | 几何 |
| 静态兜底图（展示档角度） | 1 张 | 是 | 资源 |
| 静态兜底图（官方档角度） | 1 张 | 否，建议给 | 资源 |
| 环境贴图（仅当不用程序化环境时） | ≤1 张，全机型共用 | 否 | 资源 |

---

## 3. 存放位置：复用现有云端资源仓库

**不新建渠道。** 应用已经在从现有云端仓库拉取预设（`.toml`）、测试模型、校准模型。3D 展示模型是第四类资源，走同一条链路，只多一个目录：

```
<现有云端资源仓库>/
  presets/            已有
  test-models/        已有
  calibration/        已有
  showcase-models/    ← 新增
      manifest.json
      a1-mini-1.0.0.glb
      a1-mini-1.0.0.variant.json
      a1-mini-showcase.webp
      a1-mini-official.webp
      env/studio-v1.env.webp        仅当不用程序化环境
```

> 仓库准确地址、目录规则、是否鉴权 = **MKP 侧待填**（§16-1）。上游先按上面的相对路径组织，前缀由应用拼。
> 应用侧约束：必须匿名 HTTPS GET 直下、路径稳定、不重定向到需登录页。

---

## 4. 文件不可变

应用**下载一次就永久缓存**，缓存键 = `machineCode + version + variant`，命中缓存时**完全不发网络请求**（连 HEAD 都不发）。

> **`.glb` 或 `.variant.json` 任一内容有改动，必须升 `version` 并改文件名。禁止原地覆盖同名文件。**

`variant.json` 现在是规格的一部分，所以它和 glb 一样：版本化文件名、在 `manifest.json` 里登记 `sha256`、下载后校验。**只改标定不改几何，也要升 version** —— 反正 `variant.json` 只有几 KB，重下几乎无成本，而 glb 命中缓存不会重下（形态 A 下 glb 文件名不变时应用会复用旧几何；若几何未变，允许 glb 文件名保持上一个 version，用 `glbVersion` 字段声明，见 §10）。

命名：
- `a1-mini-1.0.0.glb`
- `a1-mini-1.0.0.variant.json`
- 双文件材质形态：`a1-mini-showcase-1.0.0.glb` / `a1-mini-official-1.0.0.glb`

---

## 5. 材质档（展示档 / 官方档）

### 5.1 背景：材质不在 glb 里

上游回执 r1 §5 已经说清：上游的 glb 只带**几何 + 材质名**，观感来自上游应用侧一套材质档案（`MATERIAL_PROFILE`：颜色 / 粗糙度 / 金属度 / 环境反射强度逐档标定），**运行时按材质名整份替换**，而且不止一套。

所以「标准 PBR」这条要求在上游那边直接满足的是"CAD 原始灰白件"，不是那台看起来对的机器。

### 5.2 要交两套

| variant key | 中文名 | 语义 | 用途 |
| --- | --- | --- | --- |
| `showcase` | **展示档** | 上游摄影棚里那套已标定材质，即上游画面里那台机器的观感 | 首页默认展示 |
| `official` | **官方档** | 贴近真机官方外观/官方配色的那套 | 用户切换查看真机配色 |

对应上游哪一档 `MATERIAL_PROFILE`，写进 `variant.json` 的 `sourceProfile` 字段（**不是写在交付说明里**）。

**几何只交一份**，两种形态，优先 A：

| 形态 | 做法 | 体积 | 切换 |
| --- | --- | --- | --- |
| **A（推荐）** | 单个 glb + Khronos 官方扩展 **`KHR_materials_variants`** 装两套材质 | 几何一份 + 两套材质数值（无贴图时几百字节） | 零下载、瞬间切换 |
| B（兜底） | 两个 glb，各自完整 | 几何重复，约翻倍 | 切换需二次下载 |

> `KHR_materials_variants` 是 Khronos 正式扩展，three.js `GLTFLoader` 原生支持。§7 的"禁用非白名单扩展"**不包含它**，明确允许并推荐。

### 5.3 材质必须烘进 glb

每档每个材质给出数值化 PBR，烘进 `pbrMetallicRoughness`：`baseColorFactor`（含 alpha，透明件要给对）、`metallicFactor`、`roughnessFactor`、`emissiveFactor`（如有）。**材质名保持不变**，方便双方对照排查。

**无贴图是允许的**，纯数值 PBR 完全可以。§7 里贴图那几条是**上限**，不是要求。

环境反射烘不回去，那部分由应用负责 —— 见 §6。

---

## 6. 观感一致性：环境 / IBL 与渲染参数

**回答回执 r1 §6-Q2：应用侧的灯光包含环境贴图（IBL），不是只有几盏方向光。** 金属件发黑的问题不会出现。

v2 曾在文档里列了一张"应用侧固定的渲染参数"表（fov / 曝光 / tone mapping）。**v3 作废这张表** —— 它正是"把规格写在文档里"的错误做法。这些值现在全部由 `variant.json` 供给，应用照单消费。

责任划分：

| 谁 | 负责什么 |
| --- | --- |
| 上游 | 环境配方、相机 fov、曝光、tone mapping、色彩空间、`envMapIntensity`、默认展示角度 —— 全部写进 `variant.json` |
| 应用 | 按 `variant.json` 生成环境、设置渲染器、摆机位；页面级的东西（地面阴影、背景渐变、UI）自己定 |

环境两种给法，优先第一种：

| 方案 | 做法 | 下载体积 |
| --- | --- | --- |
| **① 程序化（推荐）** | `variant.json` 里给出摄影棚构造参数（几盏面光的位置/尺寸/色温/强度，或声明基于 `RoomEnvironment` 及改动），应用用 `PMREMGenerator` 运行时生成 | **0 字节** |
| ② 环境贴图 | 一张 ≤ 512×256 等距柱状图（`.webp` / `.hdr`），放 `showcase-models/env/`，**全机型共用一张** | ≤ 60KB，一次下载 |

---

## 7. 模型文件要求

| 项 | 要求 |
| --- | --- |
| 格式 | `.glb`（glTF 2.0 二进制）。`.stl / .obj / .3mf / .fbx / .step` 一律不接受 |
| 几何压缩 | **Draco**（必需，未压缩约 7MB 过不了线） |
| 单文件大小 | 推荐 ≤ 4MB，**硬上限 8MB**（超限应用直接拒绝下载） |
| 三角面数 | 推荐 ≤ 80k，硬上限 150k |
| Draw call | ≈ 合并同材质网格后的不同材质数。推荐 ≤ 25，硬上限 40 |
| 材质 | 数值化 PBR，见 §5.3 |
| 贴图（上限，非要求） | 单张 ≤ 1024²，总数 ≤ 4 张。没有贴图不算不合格 |
| 允许的扩展 | `KHR_draco_mesh_compression`、`KHR_materials_variants`、`KHR_texture_basisu` |
| 禁用 | 透射/折射 `KHR_materials_transmission`、非白名单扩展 |
| 动画 / 骨骼 / 变形 | 不要 |
| 相机 / 灯光节点 | 不要打包进 glb（机位与灯光由 `variant.json` 给） |
| 地面 / 底座 / 阴影平面 | 不要建进模型（地面阴影由应用画，否则旋转时阴影跟着转） |
| 交付范围 | **含笔架配件** `printrig_beam / printrig_head / printrig_pen` |

---

## 8. 坐标 / 朝向 / 尺度

1. **单位**：1 glTF unit = 1 米（上游 `MM_PER_GLTF_UNIT = 1000` 已印证）。
2. **上方向**：**+Y 朝上**。上游现为 CAD 的 Z-up，需要一次转轴。
3. **正面朝向**：机器正面朝 **+Z**。转轴后实测正脸朝哪，用旋转对到 +Z，**不要用 `yawDefault` 去补朝向**（`yawDefault` 只管"好看的角度"，不管纠正建模朝向）。
4. **原点**：模型**底面中心**落在 `(0,0,0)`。上游现在的原点由成型区左前角、喷嘴尖端两个标定常量推出，需要重定。
5. **变换烘死**：所有网格 transform 应用到顶点（scale=1 / rotation=0 / position=0）。
6. **360° 全周可见**：只允许绕 Y 旋转，四面都会被看到。背面必须有几何、不能有反面（法线朝内）。
7. **底面封闭**。未验证前 `pitchLock` 填 `true`。
8. 合并成一件后，**运动部件摆在好看的中位姿态**（床居中、横梁中位、刀头居中），并在 `variant.json` 的 `notes` 里说明摆位。

---

## 9. 静态兜底图

**透明底 + 不烘阴影**（两条绑定，一起定的）。

| 项 | 要求 |
| --- | --- |
| 背景 | 完全透明 |
| 阴影 | 不烘任何地面/接触阴影 |
| 格式 | `.webp`，质量 ≥ 80 |
| 体积 | ≤ 400KB |
| 尺寸 | 长边 1200~2000px |
| 裁切 | 紧贴 alpha 真实边界，四周留约 3% 空白 |
| 角度 | 与该档的 `yawDefault` 一致 |
| 套数 | 展示档 1 张（必需）+ 官方档 1 张（建议） |

为什么必须透明且不带阴影：首页背景是浅色渐变底，图片直接贴上去；烘死的阴影换底色会露灰边，且会和应用画的地面阴影叠成两层。

---

## 10. `manifest.json`（索引层，schema 3）

只负责"有哪些机器、文件在哪、校验值是多少"。**不放任何视觉参数。**

```json
{
  "schema": 3,
  "updatedAt": "2026-09-14",
  "models": [
    {
      "machineCode": "A1_MINI",
      "displayName": "A1 mini",
      "version": "1.0.0",
      "variantFile": "a1-mini-1.0.0.variant.json",
      "variantSha256": "3b7e…",
      "glbVersion": "1.0.0",
      "file": "a1-mini-1.0.0.glb",
      "bytes": 2148576,
      "sha256": "9f2c…",
      "triangles": 62340,
      "includes": ["printrig_beam", "printrig_head", "printrig_pen"],
      "license": "",
      "author": ""
    }
  ]
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schema` | 是 | 固定 `3` |
| `machineCode` | 是 | 与后处理脚本机型代号**逐字符一致**（首批 = `A1_MINI`）。大写 + 下划线。这是唯一关联键 |
| `displayName` | 是 | 人读名称，仅日志用 |
| `version` | 是 | 本条目版本。`variant.json` 或 glb 任一变化就升 |
| `variantFile` / `variantSha256` | 是 | 视觉档案文件名与校验值 |
| `glbVersion` | 是 | glb 自身的版本。**几何没变时可以保持旧值**，这样只改标定不会让用户重下几何 |
| `file` / `bytes` / `sha256` / `triangles` | 形态 A 必填 | glb 文件与实测值 |
| `variantFiles` | 形态 B 必填 | 每个材质档一个 glb，各自 `file`/`bytes`/`sha256`/`triangles` |
| `includes` | 否 | 含哪些配件，用于"关于/致谢"标注 |
| `license` / `author` | 否 | 版权登记，口径见 §16-2 |

`bytes` / `sha256` / `triangles` / `variantSha256` **必须由打包脚本自动生成**（上游已确认能做）。

---

## 11. `{model}.variant.json`（视觉档案层，schema 1）

**这份文件就是"那台机器长什么样"的唯一规格。** 下面的数值全部是示例。

```json
{
  "schema": 1,
  "machineCode": "A1_MINI",
  "version": "1.0.0",
  "defaultVariant": "showcase",
  "variants": [
    {
      "key": "showcase",
      "label": "展示",
      "sourceProfile": "MATERIAL_PROFILE.studio",
      "yawDefault": 18,
      "fallbackImage": "a1-mini-showcase.webp"
    },
    {
      "key": "official",
      "label": "官方",
      "sourceProfile": "MATERIAL_PROFILE.stock",
      "yawDefault": 22,
      "fallbackImage": "a1-mini-official.webp"
    }
  ],
  "camera": {
    "fov": 28,
    "distanceFactor": 2.6,
    "heightFactor": 0.62,
    "targetHeightFactor": 0.45
  },
  "render": {
    "toneMapping": "ACESFilmic",
    "exposure": 1.0,
    "outputColorSpace": "srgb",
    "envMapIntensity": 1.0
  },
  "environment": {
    "type": "procedural",
    "recipe": "studio-v1",
    "base": "RoomEnvironment",
    "lights": [
      { "kind": "area", "pos": [0.6, 1.2, 0.8], "size": [1.2, 0.8], "color": "#ffffff", "intensity": 3.2 },
      { "kind": "area", "pos": [-0.9, 0.9, -0.4], "size": [0.9, 0.9], "color": "#e8f0f8", "intensity": 1.4 }
    ]
  },
  "interaction": { "pitchLock": true, "pitchRangeDeg": 0 },
  "notes": "床居中、横梁中位、刀头居中；含笔架三件"
}
```

### 字段说明

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schema` | 是 | 固定 `1`（`variant.json` 自己的 schema，与 manifest 的独立） |
| `machineCode` / `version` | 是 | 必须与 `manifest.json` 对应条目一致，不一致按损坏处理 |
| `defaultVariant` | 是 | 首页默认档，必须存在于 `variants[].key` 中 |
| `variants[].key` | 是 | `showcase` / `official`。与 glb 里 `KHR_materials_variants` 的名字一致 |
| `variants[].label` | 是 | 界面上显示的中文短标签，应用直接用，**不自己翻译** |
| `variants[].sourceProfile` | 是 | 对应上游哪一档 `MATERIAL_PROFILE`。只用于查账排查 |
| `variants[].yawDefault` | 是 | 该档的默认展示角度（度） |
| `variants[].fallbackImage` | 是 | 该档的静态兜底图 |
| `camera.fov` | 是 | 透视相机纵向视场角（度） |
| `camera.distanceFactor` | 是 | 机位距离 = 模型高度 × 该系数 |
| `camera.heightFactor` | 是 | 机位高度 = 模型高度 × 该系数 |
| `camera.targetHeightFactor` | 是 | 视点高度 = 模型高度 × 该系数 |
| `render.toneMapping` | 是 | 枚举：`ACESFilmic` / `AgX` / `Neutral` / `Linear` / `None` |
| `render.exposure` | 是 | `toneMappingExposure` |
| `render.outputColorSpace` | 是 | 枚举：`srgb` / `srgb-linear` |
| `render.envMapIntensity` | 是 | 统一乘到所有材质上 |
| `environment.type` | 是 | 枚举：`procedural` / `texture` |
| `environment.recipe` | 是 | 配方标识，仅用于日志与缓存标记 |
| `environment.base` | type=procedural 时必填 | 基底，如 `RoomEnvironment` / `none` |
| `environment.lights` | type=procedural 时必填 | 面光数组：位置、尺寸、颜色、强度 |
| `environment.file` | type=texture 时必填 | 环境图相对路径，`env/` 下，全机型共用 |
| `interaction.pitchLock` | 是 | `true` = 锁死上下旋转 |
| `interaction.pitchRangeDeg` | 是 | `pitchLock=false` 时的上下范围（度），锁死时填 `0` |
| `notes` | 否 | 给人看的说明（摆位、含哪些件等）。**应用不解析这个字段** |

`camera` 用"系数 × 模型高度"而不是绝对坐标，是为了让同一套构图规则在不同尺寸的机型上都成立 —— 以后加大机型不需要重新调参。

> 上游若认为某个应该由数据驱动的东西还不在这份 schema 里（例如地面阴影的强度、背景色建议），**提出来加字段，不要写在 README 里让 MKP 去抄**。

---

## 12. 应用侧的校验与钳制

"消费数据"不等于"无条件照做"。为了让这条链路可预期，应用侧按下面的规则处理：

| 情况 | 处理 |
| --- | --- |
| `variant.json` 缺失 / 非法 JSON / `schema` 不认识 | **整条降级**：该机型按"没有 3D"处理，用静态图。不猜、不半读 |
| `variantSha256` 校验不过 | 同上，并丢弃缓存文件 |
| `machineCode`/`version` 与 manifest 不一致 | 按损坏处理，整条降级 |
| **必填字段缺失** | 整条降级（而不是用默认值硬撑）。必填就是必填 |
| **可选字段缺失** | 用应用内置默认值，开发模式 warn 一条 |
| 未知枚举值（如 `toneMapping: "Foo"`） | 用应用默认枚举 + warn，**不整条降级**（不至于因为一个拼写错误让 3D 全没了） |
| **数值越界** | 钳制到安全区间 + warn。安全区间见下 |
| 出现 schema 里没有的字段 | 忽略 + 开发模式 warn（向前兼容，方便上游先发字段后升 schema） |

安全区间（这是**安全阀**，不是审美参数；上游给的值只要在区间内就原样生效）：

| 字段 | 区间 | 越界原因 |
| --- | --- | --- |
| `camera.fov` | 15 ~ 60 | 超出会严重变形或几乎正交 |
| `camera.distanceFactor` | 1.2 ~ 6.0 | 太近穿模，太远变成一个点 |
| `render.exposure` | 0.2 ~ 3.0 | 全黑或全白 |
| `render.envMapIntensity` | 0 ~ 4 | 同上 |
| `environment.lights[].intensity` | 0 ~ 20 | 同上 |
| `variants[].yawDefault` | 任意，取模 360 | 无风险 |
| `interaction.pitchRangeDeg` | 0 ~ 30 | 再大会看到没建模/未封闭的底部 |

被钳制过的值会在开发模式打印"实际生效值 vs 上游给定值"，方便上游一眼看出自己给超了。

> `devicePixelRatio` 上限、是否开实时阴影、按需渲染这些属于**应用性能策略**，不由上游数据控制。这不是"抄文档"，是应用自己的职责边界。

---

## 13. 应用侧承担什么（上游不用管）

- 按 `variant.json` 生成环境、设置渲染器、摆机位
- 地面阴影（CSS 绘制）、页面背景、UI 排版
- 旋转交互：仅左右拖动改 yaw，**无平移、无缩放**，释放后惯性衰减，双击复位
- 材质档切换 UI：平面文字开关，标签直接用 `variants[].label`
- 下载、进度、SHA-256 校验、落盘缓存、旧版本清理
- 清单缓存：内置一份兜底清单，联网时后台刷新；首页渲染永不等待网络
- 性能策略：按需渲染（静止零帧）、`devicePixelRatio` 上限、不开实时阴影
- 降级：见 §12

---

## 14. 对回执 r1 六问的答复

**Q1 云端仓库地址 / 目录规则 / 是否鉴权？** 仍待 MKP 内部给出（§16-1）。已定约束：匿名 HTTPS GET 直下、路径稳定、目录名用 `showcase-models/`。上游先按相对路径组织。

**Q2 应用侧灯光是否包含 IBL？** 包含。且 v3 把它彻底数据化：环境配方在 `variant.json` 里，应用照着生成，见 §6 / §11。金属件发黑不会发生。

**Q3 兜底图要不要透明底？阴影烘不烘？** 透明底 + 不烘阴影，见 §9。

**Q4 兜底图体积上限？** ≤ 400KB，WebP 质量 ≥ 80，长边 1200~2000px。

**Q5 机身皮肤再分发授权谁确认？** MKP 侧确认，上游不承担这个判断。降级路径见 §16-2：授权无结论前只上兜底图、清单不写条目，首页照常工作。

**Q6 `A1_MINI` 是否逐字符一致？** 是，应用侧取值为 `A1_MINI`。多机型权威代号表由 MKP 侧从后处理脚本导出后附上（§16-3）。

**回执 §4 的四条协商全部接受**：兜底图体积放宽、透明底与阴影一起定、8MB 硬上限保留且推荐值放宽到 4MB、交付含笔架件写明（`manifest.includes`）。

---

## 15. 上游交付自检清单

模型：
- [ ] `.glb`，能在任意 glTF 预览器打开
- [ ] Draco 已压，≤ 4MB（硬上限 8MB）；面数 ≤ 80k；draw call ≤ 25
- [ ] +Y 朝上、正脸朝 +Z、底面中心在原点、transform 已烘死
- [ ] 多件已合并，运动部件在好看的中位姿态
- [ ] 绕一圈无空洞、无反面；底面封闭
- [ ] 无相机、无灯光、无地面/阴影平面、无动画
- [ ] 两套材质档已烘进 glb（形态 A 用 `KHR_materials_variants`，档名与 `variant.json` 的 `key` 一致）

数据：
- [ ] `variant.json` 合法 JSON，必填字段齐全，`machineCode`/`version` 与 manifest 一致
- [ ] `defaultVariant` 存在于 `variants[].key`
- [ ] 所有数值在 §12 安全区间内（超了会被钳制，观感就不是你标定的那个了）
- [ ] `sourceProfile` 填了真实的档名
- [ ] `manifest.json` 合法 JSON，`schema: 3`
- [ ] `bytes` / `sha256` / `triangles` / `variantSha256` 全部脚本自动生成
- [ ] `version` 是新的，文件名带 version，没有覆盖旧文件
- [ ] 几何未变时 `glbVersion` 保持旧值（省用户一次下载）

图：
- [ ] 每档一张兜底图，透明底、不带阴影、WebP q≥80、≤400KB、长边 1200~2000、alpha 紧贴 + 3% 留白
- [ ] 图的角度与该档 `yawDefault` 一致

---

## 16. 仍需 MKP 侧确认（不阻塞上游开工）

1. **云端资源仓库准确地址、目录规则、是否鉴权。** 定了补进 §3。
2. **机身皮肤再分发授权。** 结论出来前走降级路径：
   - 只提交兜底图，`manifest.json` **不写该机型条目** → 首页显示静态图，一切正常，只是没有 3D
   - 授权明确后补条目，用户端下次刷新清单即可获得 3D，无需重装
   - 笔架件（MKP 自研）单独确认一次
3. **多机型权威代号表**，从后处理脚本导出。首批只需 `A1_MINI`。
4. **是否在"关于软件"标注模型来源与作者**（`license` / `author` / `includes` 三个字段）。

---

## 变更记录

| 版本 | 日期 | 变更 |
| --- | --- | --- |
| v1 | 2026-09-14 | 首版 |
| v2 | 2026-09-14 | 依回执 r1 修订：新增材质档（展示/官方）与 `KHR_materials_variants`；明确应用侧提供环境/IBL；兜底图定为透明底 + 不烘阴影 + ≤400KB；模型推荐体积放宽到 4MB；draw call 口径写清；贴图改为上限；清单升 schema 2 |
| **v3** | 2026-09-14 | **契约分层**：新增 `{model}.variant.json` 视觉档案层，把渲染参数、环境配方、材质档映射、默认角度全部数据化；作废 v2 §5.2 那张写在文档里的固定参数表；确立铁律"文档只解释、数据才是规格，应用不得硬编码文档数值"；`variant.json` 纳入版本化 + sha256 校验；manifest 升 schema 3 并拆出 `glbVersion`（几何未变可免重下）；新增校验与钳制规则（§12） |
