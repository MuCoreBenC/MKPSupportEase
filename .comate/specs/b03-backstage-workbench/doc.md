# B03 工作台与客户端资源管理

> 本稿第二次整份重写，以「工作台直接读取仓库开发数据、不内置开发配方、不做工作台配方 OTA」为准。
>
> 相对上一稿的三处纠正（旧稿错了，这里不做兼容）：
> 1. 客户端安装包内置的**不是**配方快照，而是**客户端能力定义**；开发配方永远不进客户端。
> 2. 取消「批次目录整批取」模型，改为**菜单/资源目录 OTA + 每资源兼容性声明**——否则改菜单也要升级软件。
> 3. 预设身份用 **presetId**，不用文件路径。
>
> 术语全篇固定，不再混称"配方"或"预设"：
>
> | 术语 | 是什么 | 谁编辑 | 客户端看得到吗 |
> |---|---|---|---|
> | 开发配方 JSON | 参数源数据（机型基底 + 版本覆盖） | 工作台 | **永远看不到** |
> | 客户端能力定义 | 客户端支持哪些机型/版本/字段及其约束 | 客户端侧产出，工作台**只读** | 随软件安装包内置 |
> | MKP TOML | 由开发配方生成的交付文件 | 生成物，不手编 | 按菜单下载 |
> | BBS JSON | 外部资源，工作台只收录 | 外部 | 按菜单下载 |
> | 菜单/资源目录 JSON | 决定客户端能看到、能下载什么 | 工作台 | 走 OTA |
> | Bundle JSON | 资源组合（0..N TOML + 0..N BBS） | 工作台 | 走 OTA |

---

## 1. 目标与核心原则

B03 是开发者工作台：维护开发源数据、生成客户端预设文件、管理交付资源、控制客户端最终能看到和下载什么。

1. 工作台是开发工具，不是客户端。两者独立入口、独立编译、只有工作台进不了用户的包。
2. 工作台**直接读写开发仓库里的数据**，不内置一份开发配方，也不做工作台配方 OTA。
3. 客户端不接触开发配方 JSON。
4. 资源在仓库里 ≠ 客户端能看到。
5. **保存 ≠ 生成 ≠ 发布 ≠ 客户端更新**，四个动作分开。

---

## 2. 系统结构

```text
                    开发者
                      │
                      ▼
              B03 开发者工作台  ←──只读──  客户端能力定义
                      │                    （客户端侧产出）
         ┌────────────┼────────────┐
         ▼            ▼            ▼
    开发配方 JSON   BBS JSON    菜单 / Bundle
         │            │            │
         ▼            │            │
     生成 TOML        │            │
         │            │            │
         └────────────┼────────────┘
                      ▼
                  发布目录（云端 / 开源直链）
                      │
             ┌────────┴────────┐
             ▼                 ▼
         客户端 OTA         首次安装
             │                 │
             └────────┬────────┘
                      ▼
                   MKP 客户端
          ┌───────────┼───────────┐
          ▼           ▼           ▼
      能力定义     菜单目录     资源文件
    （随软件）    （走 OTA）   TOML / BBS
```

**开发配方 JSON 那条线到"生成 TOML"就断了，它本身绝不过河。**

---

## 3. 数据与文件（绝对路径）

### 3.1 开发源数据（工作台直接读写，不进客户端包）

```
g:\project\MKPSupportEase\workbench\
  registry.json                        字段定义（本稿只读展示，不开放编辑）
  fallback.json                        回退登记表
  machines\A1.json                     机型基底：基础参数
  machines\A1\versions\standard.json   版本：只写相对基底的差异
  machines\A1\versions\quickswap.json
  bbs\BBS-01.json                      收录的 BBS（原样存放，不改一个字节）
  catalog.json                         菜单 / 资源目录
  bundles.json                         Bundle 套餐
  .draft\                              未保存编辑（原子写，重开恢复）
  .trash\                              回收站
  .snapshots\A1__standard.json         上次成功生成时的配方快照
```

### 3.2 客户端能力定义（**只读输入**）

```
g:\project\MKPSupportEase\workbench\capability\
  client-0.1.0.json
  client-0.2.0.json
  support.json                         哪些客户端版本在支持期内 → 出货检查按这几个校验
```

单个文件：

```json
{
  "clientVersion": "0.2.0",
  "catalogSchemaVersion": 2,
  "machines": ["A1", "A1_MINI", "P1S"],
  "fields": [
    { "key": "toolhead.mkp_retract", "valueType": "float", "min": -50, "max": 50, "step": 0.1 },
    { "key": "toolhead.z_offset",    "valueType": "float", "min": -2,  "max": 2,  "step": 0.01 }
  ]
}
```

**实现边界（不假装已解决）**：这几个文件只能由客户端侧产出——只有客户端知道自己支持什么。工作台**只读、不生成、不修改**它们。首版先手工放一份对应当前客户端的定义；理想形态是客户端构建时导出。**工作台不凭空编一个能力定义**，缺文件时出货检查直接报"无法校验兼容性"，而不是假装通过。

### 3.3 发布目录（交付文件）

```
g:\project\MKPSupportEase\dist-presets\
  catalog.json                         菜单 / 资源目录（OTA 入口）
  bundles.json
  presets\a1-fast-default.toml
  bbs\BBS-01.json
```

注意与 vite 的 `dist\` 分开：`dist\` 是客户端前端产物，`dist-presets\` 是交付资源，两者互不相干。

### 3.4 工作台代码

```
g:\project\MKPSupportEase\workbench.html                       第二个 vite 入口
g:\project\MKPSupportEase\src\workbench\main.tsx
g:\project\MKPSupportEase\src\workbench\App.tsx                左机型树 + 右工作区
g:\project\MKPSupportEase\src\workbench\api.ts
g:\project\MKPSupportEase\src\workbench\pages\Recipes.tsx      开发配方编辑
g:\project\MKPSupportEase\src\workbench\pages\Matrix.tsx       参数矩阵 / 批量
g:\project\MKPSupportEase\src\workbench\pages\Resources.tsx    预设 / 资源管理（TOML + BBS，两栏分开）
g:\project\MKPSupportEase\src\workbench\pages\Catalog.tsx      菜单与 Bundle
g:\project\MKPSupportEase\src\workbench\pages\Release.tsx      生成 / 出货检查 / 发布
g:\project\MKPSupportEase\src\workbench\workbench.css

g:\project\MKPSupportEase\src-tauri\src\workbench\mod.rs        命令注册
g:\project\MKPSupportEase\src-tauri\src\workbench\model.rs     数据结构
g:\project\MKPSupportEase\src-tauri\src\workbench\store.rs     读写 / 原子写 / 草稿 / 回收站
g:\project\MKPSupportEase\src-tauri\src\workbench\resolve.rs   继承解析 / 有效配方 / hash
g:\project\MKPSupportEase\src-tauri\src\workbench\capability.rs 能力定义加载（只读）
g:\project\MKPSupportEase\src-tauri\src\workbench\generate.rs  TOML 生成
g:\project\MKPSupportEase\src-tauri\src\workbench\catalog.rs   菜单 / Bundle
g:\project\MKPSupportEase\src-tauri\src\workbench\preflight.rs 出货检查
g:\project\MKPSupportEase\src-tauri\tauri.workbench.conf.json  工作台窗口配置
```

### 3.5 修改

| 文件 | 改什么 |
|---|---|
| `g:\project\MKPSupportEase\vite.config.ts` | `BUILD_WORKBENCH=1` 时才把 `workbench.html` 加进 `rollupOptions.input` |
| `g:\project\MKPSupportEase\package.json` | 加 `dev:workbench` / `build:workbench` / `tauri:workbench` |
| `g:\project\MKPSupportEase\src-tauri\Cargo.toml` | 加 `[features] workbench = []`（默认不含） |
| `g:\project\MKPSupportEase\src-tauri\src\lib.rs` | `#[cfg(feature = "workbench")]` 注册工作台命令与窗口 |
| `g:\project\MKPSupportEase\.gitignore` | 忽略 `workbench\.draft\`；其余入库（仓库开源，任何人都能来后厨看） |

---

## 4. 编译隔离（只把客户端给用户）

不是藏起来，是**根本没编进去**，三道各自独立。

**① 前端**

```ts
// vite.config.ts
const WITH_WORKBENCH = process.env.BUILD_WORKBENCH === '1'

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      input: WITH_WORKBENCH
        ? { index: 'index.html', workbench: 'workbench.html' }
        : { index: 'index.html' },
    },
  },
  // server 三条保持原样，不动（strictPort / 5321 / open:false 都是刻意的）
})
```

**② 后端**——默认 feature 不含 `workbench`，`cargo build` 连编译都不碰这些文件。

```rust
// src-tauri/src/lib.rs
#[cfg(feature = "workbench")]
mod workbench;

pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(feature = "workbench")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        workbench::list_machines,
        workbench::read_effective,
        workbench::save_version,
        workbench::generate_stale,
        workbench::preflight,
        // …
    ]);

    builder.run(tauri::generate_context!()).expect("error while running");
}
```

**③ 窗口**——只在 `tauri.workbench.conf.json` 里声明。

```
npm run tauri build              → 客户端（给用户）
npm run tauri:workbench build    → 客户端 + 工作台（只给你）
```

验收判据不是"看一眼配置"：默认产物里 `dist\workbench.html` 不存在，且二进制里搜不到工作台命令名。

---

## 5. 稳定 ID：身份、显示名、路径三件事

```json
{
  "presetId": "a1-fast-default",
  "displayName": "A1 快拆版默认预设",
  "resource": "presets/a1-fast-default.toml",
  "minClientVersion": "0.2.0"
}
```

| 内容 | 含义 | 是否固定 |
|---|---|---|
| `presetId` | 预设的稳定身份 | 原则上不变 |
| `displayName` | 客户端显示名称 | 随时可改 |
| `resource` | 实际资源路径 | 可调，但必须同步更新引用 |

由此：

- 改显示名 → **不需要**改客户端能力定义，也不需要升级软件。
- 换 TOML 路径 → 只更新 `resource`，presetId 不动。
- **文件名变了不等于新预设**；客户端按 presetId 认身份。
- 删除资源或改变其身份 → 必须先处理菜单/Bundle 里的引用（出货检查阻断）。

客户端能力定义里**不枚举预设文件名**，所以新增一个预设不需要升级客户端。

实现上：`presetId` 在菜单里唯一，改 id 视作"删旧 + 建新"并明确警告——客户端会把它当成另一个预设。

---

## 6. 机型与版本（稀疏继承）

```
A1
├── 机型基底   （喷嘴偏移 X=168, Y=-2, Z=0.10）
├── 标准版     （不覆盖 → 三个值全继承）
└── 快拆版     （只覆盖 Z=0.15 → X/Y 继承）
```

- 未覆盖字段继承机型基底；覆盖字段用自己的值。
- 改机型基底，只影响继承该字段的版本。
- 改版本参数 = 创建/更新该版本的覆盖，**不动基底**。
- 移动版本到别的机型：保留已有覆盖值，**移动前预览继承值变化**。

```
移动：A1 快拆版 → P1
  喷嘴偏移 X   继承值   168 → 175      ⚠ 值变了
  喷嘴偏移 Y   继承值   -2  → -1       ⚠ 值变了
  Z 偏移       本版覆盖  0.15（保留）
```

用橙/黄标"值变了"，不用红色——这不是错误。

**开发仓库里存在某个版本 ≠ 它上架给客户端。** 机型/版本的开发定义与菜单是否展示，是两件事。

### 开发配方的历史

由 git 管，工作台**不造第二套历史系统**。工作台只额外记一份"上次成功生成时的配方快照"，用途有两个：判状态（第 8 节）、以及"当前改乱了想回到上次能出货的那份"。**恢复快照 ≠ 自动重新生成 TOML**，恢复后仍要显式点生成。

---

## 7. 工作台升级不覆盖开发成果

这条要落到代码上，不是承诺：

- 工作台**不携带任何 seed 开发配方**，升级时没有"默认值"可以回填。
- 首次启动若 `workbench\` 为空：只创建空目录骨架 + 提示"这里还没有开发数据"，**不写入任何配方**。
- 所有写入路径按类型分开：`save_version` 拿不到 `Registry` 的可写引用，所以任何版本级编辑在类型上就不可能改到全局字段定义。
- 迁移逻辑（如果将来有）一律**读旧→写新文件**，不原地覆盖，旧文件留在 `.trash\`。

---

## 8. 状态是比出来的

```
当前有效配方（机型基底 + 版本覆盖 + 全局配置指纹，序列化后算 sha256）
        vs
.snapshots\ 里记的上次成功生成时那个 sha256
```

| 状态 | 判据 |
|---|---|
| 已生成 | 两个 hash 相同 |
| 待生成 | hash 不同 |
| 未配置 | 从没生成过且版本下一个字段都没写（配方本上有名字、下面一行没写） |
| 生成失败 | 上次成功的 TOML 原样保留 + 一条失败原因，状态仍是"待生成" |

**判据是内容 hash，不是文件修改时间**——时间变了、哈希变了，用户端就会莫名其妙要更新。

字段定义、回退规则这类全局配置一改，hash 输入里的指纹跟着变 → 所有受影响的版本自动转"待生成"，不需要手工标记。

界面上每个版本各自显示：上次成功生成时间 / 有没有未生成的修改 / 当前产物是否对应当前配方。全店最近生成时间只作辅助。

**不使用"已同步"**，因为四个阶段必须分得开：

```
配方存了  ≠  TOML 生成了  ≠  发布云端了  ≠  客户端更新了
   ↑              ↑
 工作台只管这两个 → 状态词用「已生成」
```

---

## 9. 关键实现

### 9.1 数据结构

```rust
// model.rs
#[derive(Serialize, Deserialize)]
pub struct Machine {
    pub id: String,                          // "A1"
    pub display_name: String,
    pub base: BTreeMap<String, Value>,       // 机型基底
    pub default_bbs: Vec<String>,            // 机型默认 BBS 清单
}

#[derive(Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub display_name: String,
    pub machine_id: String,
    pub overrides: BTreeMap<String, Value>,  // 只写相对基底的差异
    pub bbs: BbsBinding,
}

pub enum BbsBinding {
    Inherit,          // 继承机型默认
    Own(Vec<String>), // 脱钩 → 完全独立一份，不做增删式继承
}
```

`BTreeMap` 不是随手选的：序列化顺序必须稳定，否则 hash 会因字段顺序抖动而误报"待生成"。

### 9.2 有效配方与 hash

```rust
// resolve.rs
pub fn resolve(reg: &Registry, m: &Machine, v: &Version) -> Effective {
    let mut values = BTreeMap::new();
    for f in &reg.fields {
        let (val, origin) = match v.overrides.get(&f.key) {
            Some(x) => (x.clone(), Origin::Override),
            None => match m.base.get(&f.key) {
                Some(x) => (x.clone(), Origin::Base),
                None => continue,            // 该字段在这个机型上不适用
            },
        };
        values.insert(f.key.clone(), ValueOrigin { value: val, origin });
    }
    // 全局配置必须进 hash，否则改字段定义不会让产物过期
    let payload = json!({
        "registry": reg.fingerprint(),
        "fallback": reg.fallback_fingerprint(),
        "values": &values,
    });
    Effective { hash: sha256(canonical_json(&payload)), values }
}
```

界面上逐字段显示来源：继承值中性色 +「继承自 A1」，覆盖值高亮 +「本版覆盖」，可一键清除覆盖回到继承。

### 9.3 原子写

所有写盘走同一条路径——开发配方、草稿、TOML、菜单都一样。

```rust
// store.rs
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().ok_or(Error::NoParent)?;
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{}.tmp", uuid()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;            // 先落盘再改名，断电也不会留半个文件
    }
    fs::rename(&tmp, path)?;      // 同盘 rename 是原子的
    Ok(())
}
```

生成一个版本：**全部写到临时目录 → 全部成功 → 整体 rename 到正式位置**。任何一步失败，旧 TOML 原样在那儿，只多一条失败记录。

### 9.4 只生成待更新

```rust
pub fn plan(all: &[(Machine, Version)]) -> Vec<String> {
    all.iter()
       .filter(|(m, v)| !matches!(state_of(m, v), GenState::Generated { .. }))
       .map(|(m, v)| format!("{}__{}", m.id, v.id))
       .collect()
}
```

「生成待更新项」是默认按钮，「全部重新生成」是次要入口。没改的产物字节不变，用户端不会莫名要更新。

### 9.5 生成流程（含能力校验）

```
1. 读当前有效开发配方
2. 解析机型基底 + 版本覆盖
3. 按客户端能力定义校验：字段是否被支持、值是否在范围/步进内
4. 生成 TOML（写临时目录）
5. 全部成功 → 原子替换旧产物
6. 写快照（记这次用的 hash）
```

第 3 步的校验对象是 `support.json` 里列出的**每一个**在支持期内的客户端版本，结果分三档：

- 全部支持 → 该产物的 `minClientVersion` 取支持期内最低那个版本
- 部分支持 → `minClientVersion` 取最低的那个支持它的版本，并在界面标明"0.1.0 用不了这个预设"
- 全不支持 → **生成阻断**，提示"需要先升级客户端并提供新的能力定义"

这就是"必须先升级客户端，再发布依赖该能力的资源"的机械判据，不靠自觉。

### 9.6 草稿（关掉再开还在那个状态）

编辑 → 防抖 400ms → 原子写 `workbench\.draft\{machine}__{version}.json`：

```json
{
  "baseHash": "<打开时的配方 hash>",
  "overrides": { "toolhead.z_offset": 0.15 },
  "savedAt": "2026-09-22T14:31:02Z"
}
```

重开时 `baseHash` 与当前配方一致 → 恢复未保存状态 + 顶部一条「有未保存的修改」；不一致（这份配方在别处被改过）→ **不静默覆盖**，让你选保留草稿还是丢弃。保存后草稿删除。

**草稿恢复 ≠ 配方已保存 ≠ TOML 已生成**，三件事各有指示。

### 9.7 克隆 / 删除 / 批量

- **克隆**：弹框预填「快拆版 副本」，可直接确认也可改名；重名当场提示，不自动加后缀。
- **删除**：进 `.trash\`，可还原，彻底删二次确认。进回收站的版本，它的 TOML **立即从菜单里移除**，不能还算有效交付物。
- **批量**：勾选目标列 → 选字段 → 输入新值 → 预览影响范围 → 确认。**改基底只改基底；改版本值则创建/更新覆盖**，绝不因为勾了某些版本就无意打破它们的继承。跨机型同理，必须显式勾列，没有"一键应用到所有机型"。
- **矩阵视图**：首次打开 = 当前机型所有版本；之后恢复上次筛选与展开状态；另给「显示全部版本」入口。

### 9.8 字段定义（本稿只看）

`registry.json` 可查看、搜索、排序，**不开放新增/删除字段、改类型、改区间**。理由：它决定 TOML 生成、校验、参数范围和回退规则，现在做成完整编辑器等于在工作台里再造一个配置系统。

首版 `registry.json` **由工作台自己生成**一份最小字段集（约 10 个代表性字段：回抽长度、Z 偏移、喷嘴偏移 X/Y、装载/卸载 G-code 等），不从 `mkpse-presets` 整份搬运。

---

## 10. 预设 / 资源管理（TOML 与 BBS 分开）

同一个页面，两栏，**不共用编辑流程**：

| | MKP TOML | BBS JSON |
|---|---|---|
| 来源 | 由开发配方生成 | 外部导入 |
| 能改吗 | 只能重新生成 | 只能收录、归档、分配，不改内容 |
| 有生成状态吗 | 有（已生成/待生成/未配置/失败） | 没有，只有"在库/不在库" |
| 校验什么 | 字段与值是否被客户端支持 | 文件是否存在、JSON 是否可解析 |

BBS 三态，逐瓶指定，**不因为"仓库里有"就默认"该带上"**：

| 状态 | 含义 |
|---|---|
| 已分配 | 这个版本实际会交付 |
| 可选 | 允许用户额外选择下载 |
| 仅归档 | 在仓库里，不参与交付，**不进菜单 ⇒ 客户端完全看不到也下不了** |

仓库是开源的，任何人都能看见它躺在那儿；但客户端里点不到、下不了。这两件事不矛盾。

未分配的 BBS 在工作台清单里常显，出货检查只给**一条轻提示，不阻断**——有些 BBS 本来就是备用或归档。

**如实登记**：`G:\project\mkpse-next-v3\mkpse-presets\presets\bbs\` 当前实测只有 `Process\` 与 `.keep`，没有实际 BBS 文件。首版交付「导入按钮 + 空清单 + 三态标记」，不伪造样本。

---

## 11. 菜单与 Bundle（分开管理）

**Bundle 定义内容；菜单决定可见性。** 两份文件，不塞进一个越来越大的 JSON。

`bundles.json`——组合任意，0..N TOML + 0..N BBS：

| Bundle | TOML | BBS |
|---|---|---|
| A1 标准版套餐 | a1-standard | BBS-01 |
| A1 快拆版套餐 | a1-fast-default | BBS-02、BBS-03 |
| 纯预设套餐 | a1-standard | 无 |
| 纯 BBS 套餐 | 无 | BBS-04 |

`catalog.json`——客户端的唯一入口：

```json
{
  "catalogSchemaVersion": 2,
  "generatedAt": "2026-09-22T14:30:00Z",
  "presets": [
    {
      "presetId": "a1-fast-default",
      "machine": "A1",
      "version": "quickswap",
      "displayName": "A1 快拆版默认预设",
      "resource": "presets/a1-fast-default.toml",
      "sha256": "…",
      "size": 3820,
      "minClientVersion": "0.2.0",
      "standalone": true
    }
  ],
  "bbs": [
    { "bbsId": "BBS-01", "displayName": "…", "resource": "bbs/BBS-01.json",
      "sha256": "…", "minClientVersion": "0.1.0", "standalone": false }
  ],
  "bundles": [
    { "bundleId": "a1-fast-full", "displayName": "A1 快拆版套餐",
      "presets": ["a1-fast-default"], "bbs": ["BBS-02", "BBS-03"],
      "minClientVersion": "0.2.0" }
  ]
}
```

客户端读取规则（**不静默忽略**）：

```
读 catalog.json
  ├─ catalogSchemaVersion 不支持 → 整份拒绝，保留上一份可用目录 + 提示升级软件
  └─ 支持 → 逐项过滤 minClientVersion > 自己的条目
             若过滤掉了 N 项 → 明确显示「有 N 项需要更新软件才能使用」
```

所以"静默更新"和"提醒升级"不是两套规则，是同一条规则的两个分支：兼容范围内静默换，超出范围就明确说，绝不静默吃下不认识的字段。

**云端只改菜单就能新增、隐藏、调整预设与套餐，不必升级客户端**——前提是这些资源用到的字段都在老客户端的能力范围内（由 9.5 的校验保证）。

---

## 12. 出货检查

```
配方与产物
  ✗ A2L 标准版：未配置任何参数                              阻断
  ✗ a1-fast-default：TOML 从未成功生成                      阻断
  ⚠ a1-standard：有未生成的修改                             非阻断
引用完整性
  ✗ 菜单引用 presetId=a1-old，资源文件不存在                 阻断
  ✗ Bundle「A1 快拆版套餐」引用 BBS-09，仓库中没有            阻断
  ✗ 回收站里的版本仍被菜单引用                               阻断
  ✗ presetId 重复                                          阻断
兼容性
  ✗ workbench\capability\ 为空，无法校验兼容性                阻断
  ✗ a1-fast-default 用到 toolhead.mustard，支持期内无客户端支持 阻断
  ⚠ a1-fast-default 在 0.1.0 上不可用（已标 minClientVersion） 非阻断
资源盘点
  ⚠ 有 2 个 BBS 文件未分配到任何版本                          非阻断，轻提示
```

**"无法校验"归为阻断，不归为通过**——否则兼容性就是靠运气。

---

## 13. 发布

发布是独立动作，与编辑、保存、生成分开。发布对象只有交付文件：

- `presets\*.toml`
- `bbs\*.json`
- `catalog.json`
- `bundles.json`

**开发配方 JSON 不在发布对象里**，它是开发源数据。（仓库开源，所以别人能在仓库里看到它——但那是"来后厨参观"，不是客户端交付通道。）

发布写盘同样原子化：先写临时目录，全部成功再整体 rename；`catalog.json` **最后一个**替换，保证客户端不会读到"目录说有、文件还没到"的中间态。

---

## 14. 客户端侧（本稿不实现，只对接）

**首次安装拿到**：客户端能力定义 + 必要的本地基础配置。**没有开发配方。**

之后读云端 `catalog.json`，展示当前可用的机型、版本、预设、套餐；用户选了再按 `resource` 下载 TOML / BBS 并校验 sha256。

**OTA 能更新**：菜单/资源目录、已有能力能理解的 TOML、BBS、Bundle。
**OTA 不能替代软件升级**：新字段需要客户端新增解析/校验/处理能力时，必须先升级客户端。

客户端自己的规则（官方文件只读、改动落临时文件、临时文件关闭即弃、不可重命名官方文件、用户副本可重命名但自动避让官方名、官方更新时旧版归档、官方更新绝不碰用户副本）属消费端，本稿只提供产物身份信息：

```toml
source = "official"
preset_id = "a1-fast-default"
```

用户副本由客户端自己标 `source = "custom"` + 自己的 uuid + `basedOn`。官方更新只作用于 `source = official`。

### 一处刻意的不对称

| 场景 | 未保存的编辑 |
|---|---|
| 后厨（工作台） | **自动保留**，关掉再开还在那个状态 |
| 柜台（客户端） | **不保留**，关闭即弃 |

方向相反是有意的：后厨是你自己的工作台，中断了要能接着干；客户端是顾客，没存就等于没要。

---

## 15. 边界与异常

| 情况 | 处理 |
|---|---|
| 生成中途失败（磁盘满、写入失败、进程中断） | 临时目录丢弃，旧 TOML 原样保留，记失败原因，状态仍"待生成" |
| 草稿 `baseHash` 与当前配方不符 | 不静默覆盖，让你选保留或丢弃 |
| 版本移到新机型后某字段在新机型上不存在 | 覆盖值保留在 JSON 里但标灰「新机型不适用」，不写进 TOML（不静默丢数据） |
| 能力定义缺失或解析失败 | 生成与发布都阻断，明确报"无法校验兼容性" |
| `registry.json` 损坏 | 工作台只读模式启动 + 明确报错，不允许在坏的字段定义上生成 |
| 改了 `presetId` | 视作删旧建新，明确警告"客户端会当成另一个预设" |
| 改了 `resource` 路径 | 自动更新菜单引用；旧文件若无人引用，提示可归档 |
| 同一版本在两处被并发编辑 | 保存时比对 `baseHash`，不一致则拒绝并提示重新载入（不做锁） |
| 客户端读不懂 `catalogSchemaVersion` | 整份拒绝，保留上一份可用目录 + 提示升级软件 |
| 客户端遇到 `minClientVersion` 更高的条目 | 该条目不展示，但**明确告知有 N 项需要升级**，不静默隐藏 |

---

## 16. 预期结果

1. 工作台是同仓库里的第二个窗口；**默认构建产物里没有它的页面、也没有它的命令**，给用户的包只有客户端。
2. 工作台直接读写 `workbench\` 下的开发数据，**不内置开发配方**，升级工作台不会覆盖你的开发成果。
3. 机型基底 + 版本覆盖的继承关系在界面上随时看得见来源；克隆、重命名、移动归属、批量编辑都有预览与确认。
4. 状态由**内容 hash** 比出来，不看文件时间；默认只生成待更新项，没改的产物字节不变。
5. 所有写盘原子化；生成失败绝不破坏上一次成功的产物；`catalog.json` 最后替换，客户端读不到中间态。
6. 生成时按**客户端能力定义**校验，自动算出每个资源的 `minClientVersion`；全不支持则阻断生成。
7. TOML 与 BBS 分栏管理；BBS 三态（已分配/可选/仅归档）；未上架的文件客户端完全不可见。
8. 菜单与 Bundle 分开：改显示名、上下架、调整套餐都只动菜单，**不需要升级客户端**。
9. 客户端在兼容范围内静默更新；超出范围明确提示升级软件，绝不静默吃下不认识的字段。
10. 四个动作在界面上分得清：保存 / 生成 / 发布 / 客户端更新。

---

## 17. 明确不在本稿范围

- 客户端的下载、校验、应用、归档、用户副本管理
- 帮用户把旧 TOML 的修改迁移到新版（用户自己拿两份 TOML 逐项对比即可，不在工作台重复建设）
- 字段定义的完整编辑器（新增/删除字段、改类型、改区间）
- 工作台内置开发配方与工作台配方 OTA（**已明确不做**）
- 开发配方的第二套历史系统（git 已经在管）
- 能力定义的**生成**——它只能由客户端侧产出，工作台只读
- 云端发布通道的具体形式（本稿只产出 `dist-presets\`，仓库本身就是开源直链）
