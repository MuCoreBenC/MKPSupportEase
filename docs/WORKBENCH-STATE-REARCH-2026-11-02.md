# 工作台状态模型重构报告（2026-11-02）

> 作者裁定：「不要去做这种小小的修修补补了……我要的是整个这个系统，整个架构的重构，破坏性的重构。」
> 本报告只做判定，**不含任何代码改动**。重构在报告获准后另开分支实施。
>
> **修订记录**：2026-11-02 第二轮审查后补了第四部分之二（§4.2 硬约束）与刀序重排；
> 审查指出了报告的三处空白 —— 崩溃恢复、外部冲突、旁路写入入口，现已逐条落到代码行号。

> 一句话结论：**徽章不是坏了，是它依赖的「保存前的值」根本不存在。**
> 工作台把「已保存状态」与「编辑状态」合并成了同一棵会动的树（影子树），而这棵树的初值
> 副本（`base_root`）明明已经躺在盘上，却**只有一个人用、而且不是用来算值的**。
> 修法不是改徽章，是把已经有的那份基准接上，然后让影子树这条**双重身份的链退场**。

> 归因结论（有 Git 证据）：**这是 2026-10-31 那次重构引入的回归，不是旧 bug。**
> `58f4295`（10-01）引入 `rest_at` 时，`open_at` 读的是真源，徽章正确；
> `3babb63`（10-31）把 `open_at` 改成读影子树，而 `rest_at` **一行没改**。
> 所以本次重构不是修新毛病，是**把 10-31 漏掉的那一半补齐**。详见 §4.2.1。

---

## 第一部分 · 现状与问题

### 1.1 现在有四个东西在同时扮演「状态」

| 名字 | 住在哪 | 谁在写 | 现在被当成什么 |
|---|---|---|---|
| `presets/`（真仓库） | 仓库根 | 只有 `wb_save` / `wb_generate` / `wb_publish` | 「已保存状态」 |
| 影子树 `shadow_root` | `<workbench>/.draft/presets/` | 所有 `wb_*` 写命令 | **既当「编辑态」又当「已保存态」** |
| 基准快照 `base_root` | `<workbench>/.draft/base/` | `settle_shadow` / `wb_save` / `wb_discard` | 「影子是从哪份真源铺出来的」（只用于三方对账） |
| 内存草稿 `Ctx.draft` | 内存 + `.draft/book.json` | `wb_apply_draft` 然后立刻清空 | 「值编辑的临时层」（**已基本空转**） |

关键代码位置：

- `src-tauri/src/workbench/domain/txn.rs:883` `shadow_root()`
- `src-tauri/src/workbench/domain/txn.rs:909` `base_root()`
- `src-tauri/src/workbench/app/mod.rs:335-336` `Ctx.shadow_root` 的注释：「所有编辑写这里」
- `src-tauri/src/workbench/app/mod.rs:501` `Ctx::open_at`：`Presets::load_from(&shadow_root)` ——
  **会话的 `presets` 根永远指影子树**（注释：「读影子树，不读真源」，并记着 2026-11-01 那桩回归）

于是「当前值」和「已保存值」都被解析成同一个东西：`ctx.presets`。

### 1.2 徽章为什么消失 / 为什么显示上一个值

徽章在两处渲染，两处都读同一个后端字段 `Cell.rest`：

- `src/workbench/views/ParamsPage.tsx:2221` `{cell.dirty && cell.rest !== null && cell.editable && !dep && (<button className={s.pRevert}>↩ {String(cell.rest)}</button>)}`
- `src/workbench/views/ParamDetail.tsx:266` 同一枚（右栏详情）

`Cell.rest` 与 `Cell.dirty` 都由 `derive.rs` 算出：

| 代码 | 内容 |
|---|---|
| `derive.rs:922` | `rest: self.rest_at(col, key)` |
| `derive.rs:944-961` | `dirty_of()`：先看 `draft.pending(...)`（**已经是死代码**，见下），再 `match (now, self.rest_at(...)) { (Some(now), Some(rest)) => now != &rest, (Some(_), None) => true, ... }` |
| `derive.rs:963-969` | `rest_at()`：`self.presets.registry.param(key)?.machine_variants.get(&variant_key(...)).cloned()` |

而 `self.presets` 就是影子树（§1.1），写入顺序又是**先落影子、再拍快照**：

```text
src-tauri/src/workbench/app/mod.rs:1965  let written = storage::apply_draft_now(&mut ctx.presets, ...)?;  // 写进影子
src-tauri/src/workbench/app/mod.rs:1973  ctx.draft = Draft::default();                                    // 草稿随即清空
```

于是 `rest_at()` 读到的就是**刚刚被写进去的那一份**：

| 作者的操作 | 影子里 `machine_variants["A1:STANDARD"]` | `rest` 读到 | `dirty` | 界面 |
|---|---|---|---|---|
| 打开时 70（未钉） | 无 | `None` | false | 无徽章（正确） |
| 70 → 65 | 65 | **65** | **false** | **徽章不出现** |
| 65 → 60 | 60 | **60**… 实际是上一笔写下的 65（详见下） | true | **显示 65** |

> 补充说明第二行：`65 → 60` 这一笔在渲染前，`rest_at` 读到的是影子在**这一笔写入之前**的内容
> ——也就是 65。所以在界面上它就是「上一个值」，而不是「保存前的值 70」。
> 作者实测的两条与这条链**逐条对上**，不是偶发。

`dirty_of` 里那条 `self.draft.pending(...)` 分支 (`derive.rs:941-947`) 是 2026-10-31 之前
「值先攒在草稿里」时代留下的判据；值改成立刻写影子之后它永远为 `false`，
注释（`derive.rs:928-936`）已经写明过这一点，但真正该改的 `rest_at` 没跟着改。

### 1.3 事务为什么变成「整文件回滚」

`run_ctx` 给每条编辑命令拍两次**影子树**快照：

```text
src-tauri/src/workbench/app/mod.rs:793   let shot = match edit { Some(_) => Some(txn::snapshot(&ctx.shadow_root)?), None => None };
src-tauri/src/workbench/app/mod.rs:809   let shot_after = txn::snapshot(&ctx.shadow_root)?;
src-tauri/src/workbench/app/mod.rs:812   ctx.txn.record(label, ctx.edit_key.take(), &shot_before, &shot_after, &ctx.shadow_root)
```

`Txn::record` 把 before/after 的**整份字节**存进 payload（`txn.rs:645` 起）；撤销是

```text
src-tauri/src/workbench/domain/txn.rs:840-848  let back: Vec<(String, Option<Vec<u8>>)> = top.files.iter().map(|(rel, before, _)| (rel.clone(), before.clone())).collect();
                                               apply_bytes(shadow, &back)?;
```

即：**撤销 = 把整份文件写回上一版字节**。这在「改一格值」上是 56 KB 的 registry 整体覆盖
（`txn.rs` 头注释自陈：「参数台一次改值 = 56 KB 的 registry 两份」）。
它功能上正确（可逆性是构造出来的），但与作者要的「撤销只撤销上一步**操作**」在粒度上不是一回事：
它撤销的是**文件**，不是**那一格**。

### 1.4 「放弃」回到的是磁盘现状，不是打开那一刻

```text
src-tauri/src/workbench/app/mod.rs:2205-2233  discard_in()
  ctx.draft = Draft::default();
  ctx.flush();
  txn::mirror(&ctx.real_root, &ctx.shadow_root)?;   // ← 从真仓库镜像过来
  txn::mirror(&ctx.real_root, &base_root)?;
  ctx.txn.clear();
  ctx.reload_from_disk()?;
```

`mirror(real → shadow)` 的语义是「让影子等于**真仓库现在的内容**」。作者要的是
「让工作树等于**打开时的内容**」。两者只在一种情况下不同，但那种情况恰恰是最危险的：

> 开会话到点放弃之间，`presets/` 被外部改过（`git checkout`、AI 直接改文件、另一个终端跑
> `presets publish`）。此时「放弃」会把那些外部改动一起变成工作台的状态，
> 而用户以为自己只是作废了自己的编辑。

`build.rs:602` 的 `require_shadow_in_sync`（生成前拦「影子与真源不一致」）与
`drafts_at_risk()`（`mod.rs:1157` 起的 reload 前盘点）都是在这条「放弃 = 对齐真源」的
假设上长出来的补丁。

### 1.5 最关键的发现：基准快照已经有了，只是没接上

`base_root` 的存在理由，在 `txn.rs:886-908` 与 `mod.rs:369-382` 里写得很清楚：

| 影子 vs 基准 | 真源 vs 基准 | 判定 |
|---|---|---|
| 变了 | 没变 | 用户的草稿 |
| 没变 | 变了 | 外部修改 |
| 变了 | 变了 | 冲突 |

—— 它**本来就是用来回答「相对基准改了什么」的**。但全仓只有三处读它，全是「对账」用途：

- `settle_shadow`（`mod.rs:388`）判「影子里有没有用户的草稿」
- `drafts_at_risk`（`mod.rs:1163` 起）判 reload 会不会丢东西
- 没有任何一处用它算 `rest`

**所以本次重构的实质不是「新造一个基准概念」，而是「把已经有的基准接到值域上」。**
这直接支持作者的判断：影子树可以退场 —— 因为「不随编辑变化的参照」并不需要靠影子树来实现，
它已经独立存在了。

### 1.6 客户端为什么对，工作台为什么错

客户端只有一条链，且「已保存」是**一份编辑碰不到的数据**：

| 位置 | 内容 |
|---|---|
| `src/app/params/useParams.ts:1632` | `savedValueOf = saved.get(cellKey) ?? (editingLocal ? fileValues.get(key) : undefined) ?? combo?.baseByKey.get(key) ?? ''` |
| `src/app/params/useParams.ts:1642` | `valueOf = draft[key] ?? savedValueOf(key)` |
| `src/app/params/ParamRow.tsx:399-413` | `{(dirty || badTyping) && !locked && !readonly && <button className={s.restore}>↩ {savedText}</button>}`，`savedText = valueText(def, savedValue, tr)` |

`saved` 只由保存动作写；`draft` 只由编辑写。两个 map 互不覆盖，所以
「保存前」与「现在」**天然是两个东西**，徽章不需要任何额外判据。

工作台的区别只有一条：**它把 `saved` 和 `draft` 合并成了一棵会动的树**。
其余（React 组件、渲染、乐观层）都无关。

### 1.7 保留 / 废弃清单

| 机制 | 判定 | 理由 |
|---|---|---|
| `txn::snapshot` / `diff` / `changed_files` / `apply_bytes` / `mirror` | **保留** | 都是「以字节为准」的中性工具，与谁是谁的参照无关 |
| `txn::commit`（原子写 + 失败 `apply_bytes` 回滚 + 删除进 `.trash/<stamp>/`） | **保留，不重写** | 第 10 条判据（不得半提交）的能力**已经在里面**（`txn.rs:645-706`） |
| `base_root` 作为「打开时快照」 | **保留并升格** | 本次重构唯一的「已保存状态」来源 |
| `shadow_root` 作为「会话的 `presets` 根」 | **废弃** | 双重身份就是全部病根（§1.2、§1.5） |
| `Ctx.draft` 的值覆盖层 | **废弃** | 值已直接写树，它空转；`dirty_of` 里那条分支是死代码 |
| `Txn` 的整文件 payload | **改形态** | 改为「操作」：一条命令的类型化描述 + 反向量；撤销重放操作而不是回写字节 |
| `shadow_diff` | **改名** `draft_diff` | 语义从「真源 ↔ 影子」变成「基准 ↔ 工作」；前端渲染（`DiffView` / `build::line_pairs`）一字不用改 |
| `settle_shadow` 的三方对账 | **简化** | 影子退场后只剩「基准 ↔ 工作 ↔ 真源」两两比，不再有「影子里住了谁」的问题 |
| `require_shadow_in_sync`（`build.rs:602`） | **降级** | 它拦的是「影子与真源不一致」，退场后并入 `require_clean` |
| `mockBackend` 的 `captureShadow/restoreShadow` | **保留行为、改名** | 演示桩的语义本来就是「首改前深拷贝、放弃时还原」，与目标模型一致 |

---

## 第二部分 · 目标架构

### 2.1 三个状态，三条操作

```text
        ┌──────────────┐   打开工作台时整份拷一份（只读，编辑永不碰）
        │ presets/     │ ────────────────────────────────► BASE
        │ 真仓库       │                                    <workbench>/.session/<sid>/base/
        └──────────────┘                                              │
               ▲                                                      │ 放弃 = 用 BASE 覆盖 WORK
               │ 保存 = commit(WORK → presets/)                 （增的删掉、删的补回、改的改回）
               │ 成功后 BASE := WORK（新一代基准）                    ▼
               └──────────────────────────────────────────── WORK
                                                            <workbench>/.session/<sid>/work/
                                                            会话的 presets 根永远指这里
```

| 操作 | 定义 | 不做什么 |
|---|---|---|
| **打开** | `BASE = snapshot(presets/)`；`WORK = copy(BASE)` | 不判「上次会话有没有草稿」（见 §2.4） |
| **编辑** | 只写 `WORK` | 绝不碰 `BASE`、绝不碰 `presets/` |
| **比较** | `BASE` ↔ `WORK` | 唯一的「改了吗」判据 |
| **保存** | ① 先整份备份 `presets/` 到 `.trash/<stamp>/pre`；② `commit(WORK → presets/)`；③ 成功 ⇒ `BASE := WORK` | 失败时不留半提交（复用 `txn::commit` 的 `apply_bytes` 回滚） |
| **放弃** | `mirror(BASE → WORK)`；`clear 撤销栈` | **不读 `presets/` 当前内容** —— 否则就不是「打开那一刻」 |
| **撤销** | 反放最后一条操作的逆向量（写 `WORK`） | 不回写整文件、不碰 `BASE` |

三件事的边界写得比代码还清楚：

```text
放弃 ≠ 重新加载    放弃回到「我来的时候」，重新加载才回到「磁盘现在」
撤销 ≠ 放弃        撤销退一步，放弃退到底
保存 ≠ 生成        保存只把 WORK 写进 presets/，不推进 publishSerial（AGENTS.md §4）
```

### 2.2 「保存前的值」从哪来（徽章的正确来源）

`rest_at()` 改成读 **BASE** 的同一层钉值：

```rust
// 目标形态（不是最终代码，只是形状）
fn rest_at(&self, col: &ResolvedCol<'_>, key: &str) -> Option<Value> {
    self.base.registry.param(key)?
        .machine_variants.get(&variant_key(col.level, &col.key_of_level())).cloned()
}
```

于是三条判据自动成立，`ParamsPage` / `ParamDetail` 的 JSX **一个字都不用改**：

| 情况 | BASE | WORK | `dirty` | 徽章 |
|---|---|---|---|---|
| 打开时 70（未钉） | 无 | 70 | false | 不显示（正确：它没钉过，没有「保存前的值」可回） |
| 70 → 65 | 无 | 65 | true | 显示 `↩ 70`（**回退到出厂/继承**，见 §4 判据 1 注） |
| 70 → 65 → 60 | 无 | 60 | true | 仍显示 `↩ 70` |
| 恢复 70 | 无 | 70 | false | 消失 |
| 已钉 `A1:STANDARD = -1` → 改 -0.7 | -1 | -0.7 | true | `↩ -1` |

> **判据 1 的实现细节（必须先定）**：`A1:STANDARD` 上原值是 **70 = `defaultValue`**，
> 而 `machineVariants` 是空的 —— 登记表里写着「空表是刻意的：九条版本覆盖的值都等于
> defaultValue，写了也没有信号」（`presets/registry/param_registry.toml:2418-2425`）。
> 所以「回到 70」的正确动作是**删键**（挂回继承），徽章展示 `↩ 70` 的取值需要走
> 「BASE 那一层 → 没有就退回 `defaultValue`」这条回落链。
> 这决定了 `Cell.rest` 的语义要从 `Option<Value>` 细分成三档：
> `None`（BASE 没钉、也没默认可退）/ `Some(v)`（BASE 钉着）/ `Fallback(v)`（BASE 没钉，退到出厂）。
> 报告按这个口径写判定；落地时严格照它，不许在 UI 层再猜一次。

### 2.3 Draft State 覆盖全部编辑种类

作者要求「不区分操作类型」，所以每一种编辑都要能回答「BASE 里是什么」：

| 编辑种类 | 写进 WORK 的动作 | BASE 里的对应物 | 放弃时怎么回 | 比较时的表现 |
|---|---|---|---|---|
| 参数值 | `Presets::apply_values` 写 `WORK/registry/*.toml` | 同文件 | `mirror(BASE→WORK)` | 值不同 |
| 参数定义（改名/说明/分组/顺序） | `set_param_meta` + `write_back` | 同文件 | 同上 | 文件不同 |
| 分组结构（改名/换位/新增/删除） | `set_section_meta` / 块级改写 | 同文件 | 同上 | 文件不同 |
| 新增文件（机型/版本/套餐/资产） | 新建 `WORK/**/*.toml` | **不存在** | `mirror` 会删掉它 | `state = added` |
| 删除文件 | 删 `WORK/` 下那份 | **存在** | `mirror` 会补回来 | `state = removed` |
| 改名 | 删 + 增（两次文件操作） | 旧名存在、新名不存在 | `mirror` 双向都回 | 一 `removed` 一 `added` |
| 排序 | 改 `layout_schema.toml` / 机型文件里的 `order` | 同文件 | `mirror` | 文件不同 |
| 标注源稿（`wb_anno_write`） | 写 `WORK/assets/**/*.anno.json` | 同文件 | `mirror` | 文件不同 |

**这张表就是「不区分操作类型」的兑现方式**：放弃不需要知道用户做了什么，
只要 `mirror(BASE → WORK)`，七种编辑一起回退。

### 2.4 文件快照与提交机制（作者点名的难点）

#### 为什么必须在盘上，不能只在内存

作者的要求里有一条绕不过去的：**编辑期间有真实文件被删、被改名、被覆盖，放弃要能恢复内容和目录状态。**
而工作台正常工作时，`WORK` 里真的有文件被删除、被新建 —— 它们不是「内存里的标记」。
所以 `BASE` **必须是一份完整的、盘上的、可校验的快照**。

#### BASE 的形状

```text
<workbench>/.session/<sid>/
├── base/                       # 打开那一刻的完整内容（rel → bytes）
│   ├── registry/param_registry.toml
│   ├── layout_schema.toml
│   ├── machines/A1.toml
│   └── …
├── base.manifest.json          # 每份文件的 rel + sha256 + size + 是否文本
└── session.json                # sid / 打开时间 / 由哪一份真源铺出（real_root / 真源 stamp）
```

- 复用 `txn::snapshot` 的 `rel → Entry` 形状（`txn.rs:405`），**不复制 git 元数据**
  （`.git` 从来不在 `presets/` 里，这点现成成立）。
- `manifest.json` 里的 `sha256` 是**放弃前判完整性**用的：BASE 坏了 ⇒ 不许悄悄用真源顶替
  （这条规矩 `settle_shadow` 已经立过，`mod.rs:380-386`，照抄）。
- 只保留**最近一个** sid（工作台是单窗口单会话工具，不做多会话并发）。

#### 保存的原子性

`txn::commit`（`txn.rs:645-706`）已经有：

1. 逐份 `atomic_write`（`fsx::atomic`）；
2. 任何一份写失败 ⇒ `apply_bytes(real, &undo)` **把已经写进去的全回滚**；
3. 删除的文件 rename 进 `.trash/<stamp>/`（可人工捞回）。

**第 10 条判据「不得半提交」不需要新写代码**，只要保持「一次 `commit`、不拆成多次」。
要新增的只有一处：commit 之前把 `presets/` **整份**也留一份到 `.trash/<stamp>/pre`
（当前只留「被删掉的那几份」）。这是为了「保存中途断电 / 磁盘满」时人能整体回到保存前。

#### 放弃的原子性

`mirror(BASE → WORK)` 的现有实现（`txn.rs:546`）已经是三步：先删 `WORK` 里 BASE 没有的 →
再写 BASE 里有的 → 最后清空目录。**但它是顺序执行的，中途失败会留下半新半旧**。
目标形态要求：

```text
1. WORK → WORK.tmp 之外的做法不可取（目录可能在别处被引用）
2. 实际做法：先把 BASE 整份 copy 到 WORK.reset/，再把 WORK.reset/ 换名成 WORK/
   —— 与 atomic_write 同一个手法（rename 是原子的）
```

这一条写进迁移的第二刀，判据是「放弃中途 kill -9，重开工作台仍是打开那一刻或完整编辑态，
不能是半新半旧」。

### 2.5 撤销的粒度

作者：「撤销只撤销上一步操作」。现在的实现撤销的是「文件字节」，功能可逆但粒度不对。目标：

```text
Op {
  kind: SetValue { level, owner, key, before: Option<Value>, after: Option<Value> }
      | SetParamMeta { key, before: ParamMetaEdit, after: ParamMetaEdit }
      | Structural { rel: Vec<String>, … }   // 增删改名这类仍然按文件算，但只列这一次的文件
      | …
  label: String            // 撤销按钮上那句人话
  at: Instant              // 合并窗口（同格 5 秒）
}
```

- 撤销 `SetValue` ⇒ 只把那一格写回 `before`（`apply_values` 一条）；
- 撤销 `Structural` ⇒ 只回放这一次动过的那几个文件（不是整棵树）；
- 两者都以 `WORK` 为目标，**永不碰 BASE**。

### 2.6 「上次会话未保存」怎么办

影子树退场后，重启工作台时 `WORK` 里可能还留着上次的编辑。三种可选，报告推荐 C：

| 方案 | 行为 | 问题 |
|---|---|---|
| A 每次打开都丢弃 | `WORK := BASE := presets/` | 用户关掉窗口 = 悄悄丢东西，违背「保存才算」 |
| B 永远保留 | 上次未保存的编辑原样呈现 | 与「打开时」的基准语义打架（此时 BASE 该是哪个？） |
| **C（推荐）** | 保留 `session.json`：重开时若 `WORK ≠ BASE` ⇒ **沿用上次的 BASE**，并按「一条未保存改动」呈现；用户「放弃」回上次打开那一刻，「保存」写成新基准 | 需要持久化 `session.json`（已有），且要判「磁盘在关窗期间被改过」 |

C 的三方对账与现在的 `settle_shadow` 同构，只是把「影子 / 基准 / 真源」换成
「WORK / BASE / presets 现在」。冲突判定沿用现成的 `Snapshot::reconcile`（`txn.rs:307`）。

### 2.7 迁移：老影子树怎么处理

首次以新版打开时：

1. 若 `.draft/presets/` 存在，用现有 `settle_shadow` 判一次它是不是用户的草稿；
2. 是草稿 ⇒ 把 `.draft/presets/` 整体作为新会话的 `WORK` 收下，把 `.draft/base/` 作为 `BASE`，
   按「一条未保存改动」呈现（**不丢用户的编辑**）；
3. 不是草稿 ⇒ 直接删 `.draft/`（含 `book.json`，它的内容已由 WORK 承担）；
4. 之后 `.draft/` 整族废弃，`.session/` 取而代之。

`WORKBENCH_DIRS`（`paths.rs:44`）要把 `.session` 加进去，`.draft` 留一版用于清理。

---

## 第三部分 · 改动清单

> 行号以 2026-11-02 工作区为准。表的最后一列是「不改会怎样」，用来判断能不能省。

### 3.1 后端（Rust）

| 模块 / 文件 | 计划 | 不改会怎样 |
|---|---|---|
| `domain/derive.rs:963` `rest_at` | 改读 `self.base`（新增字段）而非 `self.presets`；返回值细分为「钉着 / 退到出厂 / 无」三档 | 徽章继续显示上一个值 |
| `domain/derive.rs:944` `dirty_of` | 删掉 `draft.pending` 分支（死代码）；判据只留 `now != rest` 与「原先没钉、现在钉上」 | 每格多一次无用查表；语义两处各说一套 |
| `domain/derive.rs:126` `Book::new` | 签名从 `(presets, committed, draft)` 变为 `(work, base, committed)`；`draft` 参数退场 | `Book` 构造点遍布（见 3.4），必须一次改完 |
| `domain/derive.rs:537-538` | `dirty_count` 与 `save` 状态改从 `BASE↔WORK` 派生 | 状态栏「未保存 N」说谎 |
| `app/mod.rs:310-348` `Ctx` | `shadow_root` → `work_root`；新增 `base: Presets`（BASE 的内存文档）与 `base_root` | 没有 BASE 内存面，`rest_at` 无处可读 |
| `app/mod.rs:388` `settle_shadow` | 简化为 `settle_session`：只判 `WORK/BASE/presets`，不再维护「影子里住着谁」 | 「上次未保存」的语义继续靠影子树，删不掉它 |
| `app/mod.rs:490` `open_at` | `BASE ← snapshot(presets/)`；`WORK ← copy(BASE)`；`Presets::load_from(&work_root)` | 会话根还是影子树 = 边改边落盘的老病 |
| `app/mod.rs:558` `refresh_unsaved_files` | 比较对象从 `real ↔ shadow` 改为 `base ↔ work` | 外部改 `presets/` 会被算成「我的未保存改动」 |
| `app/mod.rs:780-835` `run_ctx` | 撤销改记**操作**而非字节（见 §2.5）；`shot` 机制退场 | 撤销继续整文件回滚 |
| `app/mod.rs:1100` `wb_reload` | 语义拆成两条：`reload`（回到磁盘现在）/ `discard`（回到打开那一刻） | 两者继续混在一条命令里 |
| `app/mod.rs:1163` `drafts_at_risk` | 改比 `base ↔ work`；「外部改了 presets」单独成一档提示 | reload 拦错人 |
| `app/mod.rs:1940-2020` `wb_apply_draft` | 保留「写 WORK」；删掉 `ctx.draft = Draft::default()` 之后那一串为草稿服务的代码 | 草稿层空转，多一次无用写盘 |
| `app/mod.rs:2149` `save_in` | commit 前整份留底到 `.trash/<stamp>/pre`；成功后 `BASE := WORK`（同时更新 `self.base`） | 断电时无法整体回退 |
| `app/mod.rs:2205` `discard_in` | `mirror(BASE → WORK)`；**一个字节都不读 `real_root`** | 放弃退到「磁盘现在」，不是「打开那一刻」 |
| `app/mod.rs:2249` `wb_shadow_diff` | 改名 `wb_draft_diff`，比较 `base ↔ work`（`domain/shadow_diff.rs:79` 同步改） | 「未保存改动」框读错参照 |
| `app/mod.rs:2257/2281` `wb_txn_undo/redo` | 目标从 `shadow_root` 改为 `work_root` | 撤销写到已废弃的目录 |
| `domain/txn.rs:645` `commit` | 保留；新增 `pre` 整份留底 | 见上 |
| `domain/txn.rs:708` `Txn` | payload 形态改为 `Op`（见 §2.5），`MAX_OPS` 与合并窗口保留 | 撤销粒度不对 |
| `domain/txn.rs:883/909` | `shadow_root()` 退场、`base_root()` 改到 `.session/<sid>/base`；新增 `work_root()` | 路径概念少一个，少一处能读错 |
| `domain/shadow_diff.rs` | 文件改名 `draft_diff.rs`；`ShadowDiff` 结构里的 `shadowRoot` 字段改 `workRoot` | 前端类型名说谎 |
| `app/storage.rs:248` `apply_draft_now` | 保留（它写的就是 `ctx.presets` = WORK） | — |
| `app/storage.rs:272` `save` | 退场（保存只剩 `commit`）；`book.json` 快照整体退场 | 两套「保存」语义并存 |
| `app/build.rs:602` `require_shadow_in_sync` | 删掉（`require_clean` 已足够：BASE↔WORK 有改动就不许生成） | 多一道永远为真的检查 |
| `app/build.rs:579` / `1630` | 生成/发布后的 `mirror(real → shadow)` 改 `mirror(real → WORK)` **且** `BASE := WORK` | 生成后 WORK 停在旧代，下一次保存会把刚生成的产物覆盖回去 |
| `app/publish_tx.rs:442` | `ctx.draft.is_clean()` 判据改为 `BASE↔WORK` 无改动 | 发布闸读空转的草稿层 |
| `workbench/mod.rs:86-110` `load_session_presets` | 改读 `work_root`（并把 `base` 一并装进 Ctx） | 这条路径也会读到影子树 |
| `workbench/paths.rs:44` `WORKBENCH_DIRS` | 加 `.session` | 目录不被创建/清理 |
| `bin/presets.rs:144/178`、`bin/clear-stale.rs:43` | 认 `work_root` / `base_root` 新位置；`--draft` 改指 WORK | CLI 写到废弃目录 |

### 3.2 前端（TypeScript）

| 文件 | 计划 | 不改会怎样 |
|---|---|---|
| `src/workbench/views/ParamsPage.tsx:2221` | **不改** | — （`Cell.rest` 语义修好，徽章自然对） |
| `src/workbench/views/ParamDetail.tsx:266` | **不改** | — |
| `src/workbench/views/BatchEdit.tsx:36/136` | 只改注释（`rest` 语义变化） | 注释说谎 |
| `src/workbench/api.ts:860-880` `ShadowFileDiff` / `ShadowDiff` | 改名 `DraftFileDiff` / `DraftDiff`，`shadowRoot` → `workRoot` | 类型名与新语义脱节 |
| `src/workbench/api.ts:2465` `shadowDiff()` | 改 `draftDiff()`（IPC 名同步） | — |
| `src/workbench/editQueue.ts` | 保留（乐观层就是「Draft 显示层」，与目标模型同构）；`patchRow` 里 `rest` 不要被乐观值改写（**现在就没改，要钉住**） | 乐观层一旦改写 `rest`，徽章又变「上一个值」 |
| `src/workbench/App.tsx:564/593` | `save` / `discard` 的注释与 toast 文案按新语义改（「回到打开时的状态」） | 文案继续叫「拉回磁盘原状」 |
| `src/workbench/dev/mockBackend.ts:337/346/360` | `captureShadow/restoreShadow` → `captureBase/restoreBase`；新增 `wb_draft_diff` 返回空清单 | 演示模式与真机不一致 |
| `src/workbench/views/HistoryModal.tsx` / `ShadowDiffModal.*` | 只改名与文案（「盘上 ↔ 影子」→「打开时 ↔ 现在」） | 用户看到的解释是错的 |

### 3.3 明确不动

| 区域 | 判定 |
|---|---|
| `src/app/**`（客户端全部代码） | **一个字不改**（作者明令） |
| `crates/postprocess/**`、`crates/preset/**` | 不动（内核与预设库与状态模型无关） |
| `presets/**`（源数据） | 不动 |
| `presets/delivery/**`、`catalog.generated.json` | 不动（生成物，由 `presets generate` / `publish` 管） |

### 3.4 `Book::new` 的调用面（必须一次改完，不能分批）

`Book::new(&presets, committed, draft)` 在下列位置出现（写作前已 grep）：

- `app/mod.rs`：`build::book_with_probe` 内部、`view_of`、`diff_draft`、`book_view` 一族、多处在 `#[cfg(test)]`
- `app/build.rs`：生成前预演、`generate_with`
- `app/publish_tx.rs`：发布闸
- `app/storage.rs`：`save` 里那一段
- `domain/derive.rs` 自身与其测试

**所以「`Book` 换签名」必须是单独一刀，且这一刀之后所有测试一起跑**，
不允许拆成「一半旧一半新」的中间态（那会编译不过）。

---

## 第四部分 · 回归判据

> 每条都写三件事：**怎么测**（哪一层、造什么场景）、**现在为什么不过**、**改完怎么算过**。
> 前三条是作者实测；四~八条是作者给的例子；九、十条是持久性与原子性。

### 判据表

| # | 判据 | 怎么测 | 现在为什么不过 | 改完怎么算过 |
|---|---|---|---|---|
| 1 | 70 → 65：显示 `↩ 70` | 会话打开干净仓库；`wb_apply_draft` 把 `A1:STANDARD` 的 `toolhead.speed_limit` 设 65；读 `desk` 里那一格 | `rest_at` 读影子，已是 65 ⇒ `dirty=false`、`rest=65` | `cell.dirty === true` 且 `cell.rest` 表示 70（`Fallback(70)` 档，见 §2.2 注） |
| 2 | 70 → 65 → 60：仍显示 `↩ 70` | 同上再设 60 | `rest` 读到上一笔 65 | `cell.rest` 仍是 70 |
| 3 | 恢复 70：徽章消失 | 再把那格写回 70（= 删键挂回继承） | `dirty` 判「与影子不同」，影子本来就是 70 ⇒ **可能为真也可能为假，取决于写法** | `cell.dirty === false`，`rest` 不再参与渲染 |
| 4 | 新增文件后放弃：新增文件消失 | 会话里 `wb_add_version`（或 `wb_copy_recipe` 新建文件）；`wb_discard`；比 `WORK` 与 `BASE` | `discard` 从 `real→shadow` 镜像，**恰好也能过**（因为真源里没有这份）；`presets/` 被外部改过时才露馅 | 放弃后 `WORK` 与 `BASE` 逐字节相同，且新增文件不存在 |
| 5 | 删除文件后放弃：文件恢复 | `wb_remove_version` 后 `wb_discard` | 同上，偶然能过 | 放弃后文件回来了，内容与 BASE 逐字节相同 |
| 6 | 改名后放弃：恢复原文件名 | 改机型/套餐名后 `wb_discard` | 同上 | 新名不存在、原名存在且内容一致 |
| 7 | 排序后放弃：恢复原排序 | 分组换位（`wb_set_section_meta` 改 order）后 `wb_discard` | 同上 | `layout_schema.toml` 与 BASE 逐字节相同 |
| 8 | 参数与文件操作混合后放弃：整体恢复 | 一次会话里连做 1 + 4 + 5 + 6 + 7，再放弃 | 同上；且 `txn` 只清栈不回滚内存（`reload_from_disk` 兜住） | `WORK` 与 `BASE` 全等；`desk` 派生与「刚打开」逐字段相同 |
| 9 | 保存后重新打开：保存内容成为新基准 | 改值 + 保存；`drop_ctx()` 后重开 | `save_in` 已 `mirror(real → base)`，**现在也能过**；要防的是新实现漏掉「`BASE := WORK`」与「`self.base` 重装」 | 重开后 `dirty=false`、`rest` 等于保存后的值、徽章不显示 |
| 10 | 保存失败：不得半提交 | 注入第 N 份文件写失败（现有测试手法：只读目录 / 临时改成目录） | `txn::commit` **已有** `apply_bytes(real, &undo)` 回滚（`txn.rs:687-690`）；新实现要保住它并补 `.trash/pre` | `presets/` 逐字节等于保存前；`WORK` 仍保有用户的编辑；界面报错不报成功 |

### 新增判据（作者没列、但重构必须钉住）

| # | 判据 | 为什么必须有 |
|---|---|---|
| 11 | 外部改 `presets/` 后点「放弃」：回到**打开那一刻**，不是外部那份 | 这正是 §1.4 的病；不钉就会退回「对齐真源」 |
| 12 | 「未保存 N」不把外部改动算进来 | `refresh_unsaved_files` 换参照后最容易错的一处 |
| 13 | 放弃中途 `kill -9`：重开不得是半新半旧 | §2.4 的原子性 |
| 14 | 撤销一条值编辑：只把那一格写回，其他格（同文件内）不动 | §2.5 的粒度 |
| 15 | 老 `.draft/presets/` 有草稿时首次升级：编辑不丢 | §2.7 的迁移 |

### 现有测试的处置

下列测试与新语义冲突或需要改写（`src-tauri/src/workbench/app/mod.rs` 的 `#[cfg(test)]`）：

| 测试 | 处置 |
|---|---|
| `an_edit_writes_only_the_shadow_tree_and_lands_in_the_transaction`（:2712） | 改名 `…_only_the_work_tree_…`，断言目标换 `work_root` |
| `a_session_opened_from_disk_edits_the_shadow_until_save`（:2781） | 改名 `…_edits_the_work_tree_until_save`；**语义不变**（这一条本来就对） |
| `a_group_move_from_a_fresh_session_only_touches_the_shadow`（:2856） | 改名；断言「真仓库两份文件逐字节不动」这一半保留 |
| `moving_a_group_lands_in_the_transaction_and_reorders_the_shadow`（:2937） | 改名；补断言「`BASE` 未被改动」 |
| `e2e_first_open_lays_down_shadow_and_base`（:2483） | 改成 `…_lays_down_base_and_work` |
| `e2e_after_discard_reopening_is_clean`（:2560） | 保留，语义加强：放弃后 `WORK == BASE` |
| `shadow_edits_are_counted_only_when_they_are_the_users_drafts`（:2318）等 4 条 `reconcile` 测试 | **保留不动**（`Snapshot::reconcile` 是中性工具，三方对账仍要它） |
| `a_value_changed_back_and_forth_is_visible_each_time`（:3369） | 保留；补断言「`rest` 在 5 → 7 期间一直是 BASE 的值」 |
| `editing_does_not_touch_the_disk`（:3327） | 保留（它测的是 book.json 快照；`storage::save` 退场后这条要跟着改或删） |
| `reload_refuses_to_silently_drop_unsaved_drafts`（:2644） | 保留，判据换 `base ↔ work` |

---

## 第四部分之二 · 实施前必须钉死的硬约束（2026-11-02 审查后补）

> 审查意见的结论是「方向正确，但还不是实施细节已闭环」。这一节把每一处空白都落到代码行号上，
> 并给出必须先写进代码的规则。**下列五条里，A / B / C 是已经存在的真实缺陷，D / E 是会在新模型下变形的东西。**

### 4.2.1 先回答「混乱是重构中引入，还是重构后才有」

有 Git 证据，不是推断：

| 提交 | 日期 | 做了什么 | 后果 |
|---|---|---|---|
| `58f4295` | 2026-10-01 | 引入 `rest_at`，读 `self.presets`；同时 `open_at` 是 `Presets::load_from(self.presets.root())` | 读的是**真源** ⇒ 徽章正确 |
| `3babb63` | 2026-10-31 | `open_at` 改成 `Presets::load_from(&shadow_root)`；**`rest_at` 一行没改** | 从这天起 `rest_at` 读**被编辑过的树** ⇒ 徽章失效 |

`git merge-base --is-ancestor 3babb63 58f4295` 返回 **NO**（10-31 不在 10-01 的祖先链上），
即 `rest_at` 先诞生、shadow 后接上。

**判定：这是重构过程中引入的回归，分界点 2026-10-31。** 不是旧 bug，也不是重构完才出问题。
所以本次重构不是「修一个新毛病」，而是**把 10-31 那次改动漏掉的那一半补齐**
（当时改了会话根，没改依赖会话根的所有读者）。

### 4.2.2 漏洞 A：保存中断会让「已保存」与「基准」分裂 —— **真实缺陷**

`src-tauri/src/workbench/app/mod.rs:2165-2196` 是**四步**，不是一步：

```text
① txn::commit(real, shadow, trash, stamp)?          // 磁盘已换成新内容
② tracing::info!("保存：事务已提交")
③ ctx.draft = Draft::default(); ctx.txn.clear();
④ txn::mirror(&ctx.real_root, &txn::base_root(...))?  // 基准才推进
⑤ ctx.reload_from_disk()?
```

若 ① 成功、④ 之前崩溃（断电 / 被 `?` 中断）：

- 磁盘 = **新内容**
- `BASE` = **旧内容**
- 下次打开 `settle_shadow` 看到「WORK ≠ BASE」⇒ 判「用户有未保存草稿」
- 用户点**放弃** ⇒ `WORK := BASE` ⇒ **刚保存成功的东西被退回去了**

现有代码用注释（`mod.rs:2181-2190`）说明了「必须放在 commit 成功之后」的**顺序**理由，
但没有解决「两步之间崩溃」的问题。**顺序对 ≠ 流程原子。**

**必须补：提交阶段机。** 写进 `<session>/session.json`：

```text
idle
  → prepared             WORK 已整份快照到 <session>/staging/（供回滚）
  → real_committed       presets/ 已换完（commit 返回 Ok 的那一刻）
  → baseline_advanced    BASE := WORK 完成
  → idle
```

启动时按 stage 恢复：

| 读到 stage | 磁盘 | BASE | 该做什么 |
|---|---|---|---|
| `idle` | — | — | 正常开新会话 |
| `prepared` | **未动** | 旧 | 删 `staging/`，报「上次保存未完成」（零写入，安全） |
| `real_committed` | **已换** | 旧 | **补完** `BASE := WORK`（幂等），不打扰用户 |
| `baseline_advanced` | 已换 | 新 | 清 `staging/`，正常 |

这一条不解决，「原子 rename × N 次」仍然只是 N 次原子，不是一次原子。

### 4.2.3 漏洞 B：目录替换不是原子的 —— **真实缺陷**

`src-tauri/src/workbench/domain/txn.rs:546-579` 的 `mirror()`：

```rust
for rel in &have { if !want_set.contains(rel) { std::fs::remove_file(&path); } }   // ① 先删
for rel in &want {
    std::fs::create_dir_all(parent)?;
    if same_file(...) { continue; }
    let _ = std::fs::remove_file(&to);          // ② 删旧的
    std::fs::copy(&from, &to)?;                 // ③ 复制
}
```

三个问题：

1. **非原子**：中途失败 ⇒ 目标目录半新半旧；
2. **②③ 之间有窗口**：`remove_file` 成功、`copy` 失败 ⇒ **那份文件直接没了**（`let _ =` 吞掉了删除错误）；
3. **没有回滚**：`mirror` 全程没有一个字节的 undo 记录（对照 `commit` 有 `apply_bytes` 回滚）。

而「放弃」在新模型下就是 `mirror(BASE → WORK)` —— 用这个函数实现「放弃」，
等于把「用户最需要可靠」的那一步建在最不可靠的机制上。

**必须补：**

- 放弃改成**整目录换名**，与 `atomic_write` 同一个手法：
  `BASE → WORK.reset/`（整份拷完后校验 sha256）→ `rename(WORK, WORK.old)` → `rename(WORK.reset, WORK)` → 删 `WORK.old`。
  任一步失败 ⇒ `WORK` 仍是完整的旧版本。
- 平台约束先说清：`rename` 要求同卷（`<session>/base` 与 `<session>/work` 天然同卷）；
  Windows 上目标存在时 `rename` 会失败，所以先 `rename` 到 `.old` 再删（而不是直接覆盖）；
  实现前先确认 `WORK/` 里没有被打开的文件句柄（工作台不长期持有写句柄，这一点现成成立）。
- 崩溃后残留的 `WORK.reset` / `WORK.old` ⇒ 启动时按「哪个是完整的」判定后清理，不许留在那儿让下一次放弃再踩一次。

### 4.2.4 漏洞 C：外部修改与「重启续草稿」的冲突 —— **真实缺陷（已出过一次事故）**

`src-tauri/src/workbench/app/build.rs:585-596` 的注释是作者自己写的病历：

> `Ctx::open` 发现影子非空且与磁盘不一致时，只把会话标成 `recovered` 就照旧**读影子**……
> 于是「外部改了源、工作台影子是旧拷贝」时，生成会拿旧影子渲染、再写回真仓库 ——
> **2026-10-31 A1 三份 0.7 被写回 0.8 就是这条链。**

现在用 `require_shadow_in_sync` 拦住了**生成**，但：

- 它只拦生成/发布，**不拦保存**；
- 它把「状态模型不一致」转化成了「功能不可用」（有未保存改动就不许生成）；
- 它没有回答「重启时该用哪一份」。

**必须补：会话身份与冲突三选。** `session.json` 除 stage 外再记：

```json
{ "sessionId": "...", "openedAt": "...", "diskStamp": "<presets/ 的整份指纹>" }
```

启动时比 `diskStamp` 与当前 `presets/` 指纹：

| WORK vs BASE | 磁盘 vs diskStamp | 判定 | 行为 |
|---|---|---|---|
| 相同（没改过） | 相同 | 干净 | 直接开，`BASE := 磁盘` |
| 相同 | **不同** | 只是外部改过 | **静默**开新会话（用户没有工作要保） |
| **不同**（有草稿） | 相同 | 续上次会话 | 恢复 WORK/BASE，标「一条未保存改动」 |
| **不同** | **不同** | **冲突** | **不许静默覆盖**：三选「保留我的 / 用磁盘的 / 另存一份」 |

第三条是关键：**「放弃」回到的是这段未保存会话的基线，不是「每次启动时磁盘的样子」。**
这条必须写进界面文案，否则用户会以为是后者。

### 4.2.5 漏洞 D：结构操作 —— 放弃侧已经够了，撤销侧不要过度设计

- **放弃侧**：`mirror(BASE → WORK)` 天然覆盖「新建的删掉、删掉的补回、改名双向回、排序改回」，
  **不需要为每种操作写回退代码**。这是新模型比 typed-op 更强的地方。
- **撤销侧**：审查意见说得对 —— 「保留受影响路径的快照通常比只存一个抽象操作名更可靠」。
  所以报告 §2.5 的 typed `Op` **降级为可选**：优先做「一条操作 = 它动过的那几个路径的 before/after 字节」，
  `label` 只用于显示。真正的价值是**撤销不再整棵树重写**，不是「操作类型」本身。
- **必须钉住的**：改名 = 删 + 增两条记录（自然可逆）；排序改动必须落到 `layout_schema.toml`
  这类真实文件上（若排序存在内存里，放弃就回不去）。

### 4.2.6 漏洞 E：所有写入入口的归属 —— 必须在表里逐条认领

工作台的真正风险不是「编辑按钮写错地方」，而是**有第二条写入路径**。现状：

| 入口 | 现在写谁 | 现在的保护 | 新模型下的归属（必须钉死） |
|---|---|---|---|
| 参数值编辑 | 影子 | — | `WORK` |
| 参数定义 / 分组 / 排序 | 影子 | — | `WORK` |
| 文件增删改名（`wb_copy_recipe` 等） | 影子 | — | `WORK` |
| 标注源稿 | 影子 | — | `WORK` |
| 保存 | 真仓库 | `txn::commit` | `presets` + `BASE`（阶段机） |
| 放弃 | 影子 | `mirror(real→shadow)` | **`WORK`（从 BASE）** |
| **生成** | **真仓库，然后 `mirror(real→shadow)`**（`build.rs:579`） | `require_clean` + `require_shadow_in_sync` | **`WORK` 里的 `delivery/`** |
| **发布** | **真仓库**（`build.rs:1630` 也 `mirror` 回来） | `require_clean` | 仍只认**已保存**那一版（`require_clean`） |
| 重新加载 | 替换会话 | `drafts_at_risk` 盘点 | 显式「新会话」：`BASE := 磁盘` |

**其中「生成」是唯一必须改变行为的一项。** 现在是「写真仓库、再 mirror 回影子」，
等于让生成绕过状态模型；新模型下它应当基于 `WORK` 渲染、产物落进 `WORK/delivery/`，
保存时与其它编辑一起提交。这同时解掉「改了一个值没保存就不能生成」这个不合理限制。

发布保持 `require_clean`（发出去的东西必须有人审过），这一条不需要改。

### 4.2.7 八条硬约束（建议直接写进 `AGENTS.md`）

| 规则 | 必须保证 | 由谁钉住 |
|---|---|---|
| 编辑 | 所有工作台编辑只写 `WORK` | 代码评审 + 断言「真仓库 mtime 未变」的测试 |
| 基线 | `BASE` 只在**打开新会话**或**保存成功**后推进 | 阶段机 |
| 放弃 | 只从 `BASE` 恢复，**一个字节都不读** `presets/` | 判据 11 |
| 保存 | `WORK → presets → BASE` 有可恢复的提交阶段 | 阶段机 + 判据 10 |
| 外部变化 | 磁盘与会话基线不一致时**不许静默覆盖** | 冲突三选 + 判据 12 |
| 结构操作 | 新建/删除/改名/排序都属于草稿状态 | 判据 4-8 |
| 旁路写入 | 生成/发布/刷新不得绕过状态模型 | 入口归属表 |
| 撤销 | 操作失败或恢复失败时不留半完成状态 | 目录换名 + 校验 |

### 4.2.8 验收必须组合，不能只测「改参数后放弃」

审查意见的最后一条是对的：单独修好徽章不能证明状态模型稳定。判据表（第四部分）要加**组合场景**：

| # | 组合场景 | 钉住什么 |
|---|---|---|
| 16 | 改参数 + 改名 + 删文件 + 新增 + 排序，然后**关掉工作台再打开**，再放弃 | 会话延续 + 结构操作一起回退 |
| 17 | 保存到一半强杀（在 `real_committed` 之后、`baseline_advanced` 之前） | 阶段机补完，不出现「保存了却提示有草稿」 |
| 18 | 工作台开着时外部 `git checkout` 改了 `presets/`，用户有草稿 | 冲突三选，不许静默覆盖 |
| 19 | 放弃过程中强杀 | `WORK` 要么是完整旧版、要么是完整编辑态，不许半新半旧 |
| 20 | 生成时用户有未保存改动 | 生成基于 `WORK`，产物进 `WORK/delivery/`，不写真仓库 |

---

## 第五部分 · 迁移步骤（分刀）

> 每刀结束**必须**：`cargo fmt` ✓ / 双 feature `clippy -D warnings` ✓ /
> `cargo test -p mkp-support-ease --lib --features workbench` ✓ / 前端四道（若动了前端）。
> 每刀给一个「刀后回滚点」= 该刀的 commit，出问题 `git revert` 那一笔即可。

> **刀序已按 §4.2 审查意见重排**：机制类（原子换名、阶段机、冲突判定）**先于**语义类（放弃、换根）。
> 先让下层的「写」变可靠，再让上层的「谁写谁」变正确 —— 否则每一刀都建在 `mirror` 的非原子上。

| 刀 | 做什么 | 红→绿的判据 | 回滚点 |
|---|---|---|---|
| **0** | 只加不删：`paths::work_root()` / `session_dir()`；`Ctx` 加 `base: Presets` 与 `base_root`（仍指老位置）；`session.json` 骨架（`stage` / `diskStamp` / `sessionId`，此刻没人读） | 全绿（纯基建，行为零变化） | 这一笔 |
| **1** | `rest_at` 改读 `base`（`base` 目前还是 `.draft/base`，语义已对）；`dirty_of` 删 draft 分支 | **判据 1 / 2 / 3 红转绿**；其余全绿 | 这一笔 |
| **2** | **`txn::mirror` 换成「拷到 `.reset/` + 校验 sha256 + 整目录换名」**；补删除失败的报错（不再 `let _ =`）；启动时清理残留 `.reset` / `.old`。此时**语义不变**，只是底层变可靠 | **判据 19**；`txn` 现有 mirror 单测全绿 | 这一笔 |
| **3** | 放弃改 `mirror(BASE → WORK)`，去掉 `real → shadow` 那一句 | **判据 4-8、11 红转绿**；新增断言「放弃期间 `presets/` 零读写」 | 这一笔 |
| **4** | 会话根 `.draft/presets` → `.session/<sid>/work`；`settle_shadow` → `settle_session`；加迁移逻辑（§2.7） | 判据 9 / 15；`open_at` 相关 5 条测试改名后全绿 | 这一笔 |
| **5** | **保存阶段机**：`prepared → real_committed → baseline_advanced` 写进 `session.json`；`save_in` 按阶段拆；启动恢复（§4.2.2 表） | **判据 10 / 17** | 这一笔 |
| **6** | 外部冲突三选：启动时比 `diskStamp` 与 `presets/` 指纹（§4.2.4 表）；`drafts_at_risk` 改比 `base ↔ work` | **判据 12 / 18** | 这一笔 |
| **7** | 撤销改「受影响路径的 before/after 字节」（§4.2.5：不做 typed `Op`，`label` 只用于显示） | **判据 14**；`txn` 单测改写 | 这一笔 |
| **8** | **生成改写入 `WORK/delivery/`**（不再写真仓库再 mirror 回来）；删 `require_shadow_in_sync` | **判据 20**；`build.rs` 生成相关测试 | 这一笔 |
| **9** | 清理：`storage::save` / `book.json` 快照 / `shadow_diff` → `draft_diff`（后端 + 前端 + mock 三处）、`.draft` 族残留、CLI（`bin/presets.rs` / `bin/clear-stale.rs`）、`publish_tx` 判据、文档与 MEMORY 同步 | 全量验证集 + `npm run check:artifacts` | 这一笔 |

**约束**：

- 第 4 刀之前不删任何老代码（老路径仍能用），这样每一刀都能单独 revert。
- `Book::new` 换签名放在第 4 刀内一起做（§3.4），因为它是编译期硬依赖。
- **刀 2 与刀 5 不许省**。这两刀是「不再出现状态混乱」的全部保证；省掉它们，
  其余七刀只是把「基线错了」换成「保存中断后错了」「放弃中途坏了」。
- 刀 1 可以**立刻做**并单独验证（它只改 `rest_at` 的读取源），徽章会当场正确；
  但必须知道：**刀 1 之后徽章对了，不代表状态模型稳了** —— 刀 2/5/6 才管后者。

---

## 第六部分 · 提交待办与风险清单

> 作者第一条消息要求「提交一次，不用推送」，随后把「先出报告」定为最终计划。
> **本轮一个字节都没提交。** 下面是提交前必须先处理的事。

### 6.1 工作区现状

`git status --porcelain` 共 **105 项**（84 项已跟踪修改 + 21 项未跟踪），分四类：

| 类别 | 数量级 | 能否直接提交 |
|---|---|---|
| 源码改动（`src/**`、`src-tauri/src/**`、`crates/**`） | 主体 | 可以，但要先过验证集 |
| **三层生成物**（`presets/delivery/**`、`crates/preset/assets/presets/*`、`src-tauri/src/runtime/catalog.generated.json`） | 十余份 | **不可以随手提交**（AGENTS.md §3.5）：要么整批 `presets generate` + `gen-catalog`，要么 `git restore` |
| 文档（`HANDOFF.md`、`docs/**`） | 数份 | 可以 |
| **垃圾 / 畸形** | 见下 | 必须先处理 |

### 6.2 三个必须先处理的东西

1. **`presets/src-tauri/`（未跟踪）** —— 畸形嵌套目录，里面是
   `presets/src-tauri/src/runtime/catalog.generated.json`（199872 字节），
   与真正的 `src-tauri/src/runtime/catalog.generated.json` 同名但 `revision` 不同。
   看起来是某次命令在 `presets/` 下设了 `CARGO_TARGET_DIR` 或工作目录写错造成的。
   **判定：删除，不提交。**
2. **`tmp-scroll-probe.mjs`（未跟踪，仓库根）** —— 临时探针，按 MEMORY 的
   「探针跑完删」规矩，**判定：删除**。
3. **`scripts/stale-translations.mjs`、`src-tauri/src/bin/clear-stale.rs`、
   `src/workbench/views/{ComponentSlot,GroupPicker,SectionDrawer,SectionList,SectionPicker}.*`、
   `src/workbench/views/sectionId.ts`、`src/components/scroll/`、
   `src/app/postprocess/autoCopy.ts`、`docs/HANDOFF-ARCHIVE-2026-11-01.md`** ——
   这些是**真代码**（MEMORY 里有对应条目），**判定：要提交**。

### 6.3 提交前必跑

```text
npm run check:artifacts     # 闸⑧：三层生成物一致性（红就先看它给的两条路）
npm run check:i18n
npx tsc -b --force
npm run lint
cargo fmt --all --check
cargo clippy --all-targets --features workbench -D warnings
```

### 6.4 风险（按严重度）

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| R1 | 生成物与源不同源，被一起提交 | `check:artifacts` 红、发布线脏 | 提交前单独跑一次闸；只 stage 白名单 |
| R2 | 第 3 刀（换会话根）与 `Book::new` 换签名同时落地，`git revert` 粒度粗 | 出问题回退代价大 | 第 3 刀内部再拆两个 commit（先换根、再换签名） |
| R3 | 「放弃」语义从「对齐真源」变成「回到打开时」，会让 `build.rs` / `publish_tx` 里那些「影子必须与真源同步」的假设失效 | 生成/发布可能拿着一份「打开时」的状态去渲染 | 第 5 刀统一收口 `require_clean`，判据 11/12 钉住 |
| R4 | `editQueue` 的乐观层如果被改成同时改写 `cell.rest` | 徽章退回「上一个值」 | 判据 1/2 的断言写在**后端派生**层，前端探针只做补充 |
| R5 | 老 `.draft/presets/` 的迁移丢用户编辑 | 数据丢失（最严重） | §2.7 第 2 步「是草稿就收下」，判据 15 钉住 |

---

## 附录 A · 一句话回答作者的三个问题

**Q：影子树到底有没有必要？**

> 现在的影子树没有必要，而且它是全部病根。它承担了两件事：「编辑态」和「已保存状态的参照」。
> 前者必须有（编辑要能真的写文件）；后者**已经由 `base_root` 独立承担了**，只是没人用它。
> 所以正确做法是：**留下编辑树，删掉「影子树」这个概念**，把 `base_root` 升格为唯一的
> 「打开时快照」，并且明确「放弃 = 从快照还原，不读磁盘现在」。

**Q：为什么客户端对？**

> 客户端的 `saved` 是一份编辑碰不到的数据（`useParams.ts:1632`），`draft` 是另一份；
> 工作台把这两份合并成一棵会动的树，于是「保存前」被编辑覆盖了。

**Q：这次重构多大？**

> 后端 25 处（第三部分表 3.1），前端 9 处（表 3.2，其中徽章两处**不改**），
> 现有测试 10 条要改名/改断言、4 条原样保留。分 **10 刀**（第五部分已按 §4.2 重排），
> 每刀可单独回滚。客户端、内核、交付产物一个字不动。
> 其中**刀 2（原子换名）与刀 5（保存阶段机）不许省** —— 它们才是「不再出现状态混乱」的保证。

**Q：这是重构中引入的，还是重构完才出现？**

> 重构中引入，有 Git 证据。`58f4295`（2026-10-01）落 `rest_at` 时 `open_at` 读真源、徽章正确；
> `3babb63`（2026-10-31）把 `open_at` 改成读影子树，`rest_at` 没跟着改。
> `git merge-base --is-ancestor 3babb63 58f4295` 返回 NO，即「shadow 后接上、rest_at 先存在」。
> 分界点就是 10-31。详见 §4.2.1。

**Q：按这份方案做完，还会不会再乱？**

> 只按第一版报告做，**会**。三个真实缺陷必须先修：
> ① 保存是四步、不是一步，①→④ 之间崩溃会让「磁盘已保存」与「BASE 未推进」分裂，
>    用户点放弃就把刚存的东西退回去（§4.2.2）；
> ② `mirror` 是逐个 `remove_file + copy`、失败无回滚，而「放弃」正要用它（§4.2.3）；
> ③ 外部改过 `presets/` 时，现在只拦生成、不拦保存，也没有重启冲突规则（§4.2.4，
>    2026-10-31 的 0.7→0.8 事故就是这条链）。
> 这三条修完 + 八条硬约束落地，才有资格说「不再乱」。

## 附录 B · 不改会怎样（留一句给未来的自己）

徽章不是「一个 UI bug」，它是**架构在界面上留下的一处读数**。
把徽章单独修好（比如在 `editQueue` 里缓存一个 `rest`）只能让它看起来对，
而「放弃回到磁盘现状而不是打开时」「撤销整文件回滚」「`reload` 与 `discard` 混成一条」
这三件事会继续以别的形式冒出来 —— 作者已经连报了三次「徽章」，正是这个意思。
