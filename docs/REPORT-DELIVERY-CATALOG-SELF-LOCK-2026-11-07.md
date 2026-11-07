# 报告：为什么「生成了却还说清单和文件对不上」排查了这么久

日期：2026-11-07
范围：工作台「生成与发布」页的交付目录自查（`workbench::app::delivery`）
结论：**不是日志不够，是两套判据对同一件事给了两个都"自认为对"的答案，而它们的矛盾
不产生任何日志。** 修了一个判据、补了一条判据、改了一条旧判据的期望。

---

## 0. 一句话结论

你在界面上看到的两句话都是"对"的：

- 检查卡：「交付目录的清单和文件对不上，8 处」
- 生成前确认：「0 份 · 13 份无变化」

它们不是"一个对一个错"，是**两个不同的判据在描述同一个文件的两种属性**：

| 判据 | 问的问题 | 8 份 TOML 的答案 |
|---|---|---|
| `delivery_diffs`（生成用） | "**内容**变了吗？" | 没变（剥掉时间行后相等） |
| `drift_of` / `audit_catalog`（检查用） | "**字节**和目录登记一致吗？" | 不一致（多了 42 字节的时间行） |

而 `catalog.json` 登记的是**字节**（size + sha256），由生成按"候选字节"算出。
于是：生成说"没变 → 不写"，盘上的旧字节就永远留着；目录按新字节登记，就永远对不上。
**自锁：点生成不修复，点发布也没用（发布只读目录、不重算）。**

排查花时间的真正原因不是信息不够，是**这个僵局不产生错误、不产生日志、不留 trace**：
两个函数都返回 `Ok`，一个数出 `drift=8`，一个数出 `modified=0`，谁都没觉得自己有问题。

---

## 1. 现象

### 1.1 界面上的三处提示

1. 检查卡待办：「交付目录的清单和文件对不上 · 8 处与目录登记不一致：
   `delivery/mkp/presets/A1-standard.toml`（目录登记 Some(4855) 字节，实际 4897 字节）；
   `delivery/mkp/presets/A1-fast.toml`（Some(4853) / 4895）等 8 处。生成一次会重算登记。」
2. ①生成卡：「0 组有变化 / 共 3 组」「全部一致 · 9 份」「全部一致 · 4 份」。
3. 生成前确认：「将写入 0 份 · 13 份无变化」「没有哪一份会变 —— 点了也不会重写任何文件」。

第 1 条给出的修复建议（"生成一次"）正好是第 2、3 条不会做的事。**界面的自救路径是断的。**

### 1.2 发布侧的另一句话

「8 份**资产**字节变了，`catalog.json` 待重新生成」——这句是同一份 `drift` 数列的
另一种说法，但它把**预设 TOML** 说成了"资产"。8 份全是 `mkp_preset`，资产
（图标 / 模型 / BBS 配置，共 17 条）一个没错。文案与数据不符，是第二个干扰项。

---

## 2. 事实核对（先量，再判）

### 2.1 逐条核 catalog 登记的 25 条

```
BAD mkp_preset delivery/mkp/presets/A1-standard.toml   reg 4855  disk 4897
BAD mkp_preset delivery/mkp/presets/A1-fast.toml       reg 4853  disk 4895
BAD mkp_preset delivery/mkp/presets/A1-fastv3.3.toml   reg 4857  disk 4899
BAD mkp_preset delivery/mkp/presets/A1_MINI-standard.toml reg 4861 disk 4903
BAD mkp_preset delivery/mkp/presets/A1_MINI-fast.toml  reg 4855  disk 4897
BAD mkp_preset delivery/mkp/presets/A1_MINI-fastv3.3.toml reg 4860 disk 4902
BAD mkp_preset delivery/mkp/presets/P1S-lite.toml      reg 4991  disk 5033
BAD mkp_preset delivery/mkp/presets/X1C-lite.toml      reg 4991  disk 5033
total 25  bad 8
```

17 条资产（`icon` / `model` / `bbs_config`）**全部 OK**（大小与 sha256 都对）。

### 2.2 差值的形状

| 文件 | 登记 | 实际 | 差 |
|---|---|---|---|
| A1-standard.toml | 4855 | 4897 | 42 |
| A1-fast.toml | 4853 | 4895 | 42 |
| A1-fastv3.3.toml | 4857 | 4899 | 42 |
| A1_MINI-standard.toml | 4861 | 4903 | 42 |
| A1_MINI-fast.toml | 4855 | 4897 | 42 |
| A1_MINI-fastv3.3.toml | 4860 | 4902 | 42 |
| P1S-lite.toml | 4991 | 5033 | 42 |
| X1C-lite.toml | 4991 | 5033 | 42 |

**每份都正好多 42 字节。** 这是一个极强的信号：不是内容被改了，是**多了一行固定内容**。

```
$ ls -la presets/delivery/mkp/presets/
-rw-r--r--  4897 Nov  5 19:04 A1-standard.toml
...
$ crates/preset/assets/presets/A1-standard.toml   →  4897 字节
$ head -3 presets/delivery/mkp/presets/A1-standard.toml
# machine: A1
# variant: standard
# generated_at: 2026-11-05T04:17:00+08:00
```

`# generated_at: 2026-11-05T04:17:00+08:00\n` 正好 42 字节。

盘上这 8 份是 **11-05 04:17 那一批旧产物**（mtime 11-05 19:04，内容戳 04:17），
带着写盘时刻；而现在渲染器（`build::render`）**已经不写这一行了**。

### 2.3 关键：这几份文件"内容"是好的

`crates/preset/assets/presets/A1-standard.toml` 与盘上那份 diff **完全为空**
（相同字节，4897）。也就是说：

- 盘上这 8 份的**内容**没问题；
- 它们与"现在渲染器会写出的字节"差的就是那一行 `# generated_at:`；
- `catalog.json` 的 4855 是**新形状**（不含该行）的大小。

所以三方状态是：

```
盘上旧产物（含时间行，4897）  ←  内容对、形状旧
catalog.json 登记（无时间行，4855）  ←  按"现在会写的字节"算的
候选字节（无时间行，4855）  ←  生成每算一次都是这个
```

**只有盘上那份是旧的**，而生成恰好"看不出"它旧。

---

## 3. 根因

### 3.1 判据本体

`src-tauri/src/workbench/app/delivery.rs` 的 `delivery_diffs`：

```rust
Some(bytes) => {
    let norm_old = normalize_for_compare(&c.rel, &bytes);   // 会剥掉 # generated_at:
    let norm_want = normalize_for_compare(&c.rel, &c.want);
    if norm_old == norm_want {
        (DiffKind::Same, ...)          // ← 判「一致」，于是不写
    } else {
        (DiffKind::Modified, ...)
    }
}
```

`normalize_for_compare`（同文件）对 TOML 直接调 `strip_generated_at`：

```rust
if rel.ends_with(".toml") {
    return crate::workbench::strip_generated_at(text).into_bytes();
}
```

于是：**盘上 4897 剥掉 42 字节 → 4855；候选 4855 剥掉 0 字节 → 4855；相等 → `Same`。**

### 3.2 为什么 `Same` 是错的

`normalize_for_compare` 的设计目的是**只回答"内容变没变"**（同文件文档里写着：
"只用于兼容已经带 `# generated_at:` 的历史 TOML"）。这个目的没错，`
render()` 是纯函数、同样的源必须给同样的字节，"内容没变就不写"也是对的。

错的是**把它当成了"要不要写"的唯一判据**。`Same` 在系统里被赋予了两种含义：

1. 生成：`Same` ⇒ 不写；
2. 显示：`Same` ⇒ 界面上写"无变化"。

但"不写"还有第三个消费者：**`catalog.json` 的 size / sha256**。
`candidate_once` 里目录是这样造的：

```rust
// catalog.json 的 files[] 登记的是产物真字节哈希 —— 必须按**候选** TOML 算
let tmp = tempfile::tempdir()...;
for c in &out { if 是 TOML { 写进 tmp } }
let catalog_text = catalog_from_artifacts(..., tmp.path())?.to_pretty_json()?;
```

目录按**候选**字节（4855）登记，而盘上是 4897 —— 判 `Same` 不写，这两个数就永久
不一样。检查卡读的正是这两者的差（`drift_of` 比 `size` 与 `sha256`）：

```rust
if bytes.len() as u64 != size {
    out.push((DeliveryDrift { reason: DriftReason::Size, .. },
              format!("{}（目录记 {:?} 字节，实际 {} 字节）", f.path, f.size, bytes.len())));
}
```

**同一个文件，"内容一致"与"字节一致"被当成同一件事，但它们不是。**

### 3.3 自锁的完整回路

```
点生成
  → delivery_diffs: Same ×8 → writes 为空
  → 盘上 8 份 4897 不动
  → catalog.json 照样按候选 4855 重算（它自己是"一致"的，也不重写）
  → drift_of: 8 处 size 不符 → 检查卡待办依旧
  → 回到第一步
```

注意 `catalog.json` 自己也会被算进"一致"：它的候选 = 盘上（因为两边都由同一套
候选算出来），所以**它也不会被重写**。整个回路里没有任何一步会去碰那 42 字节。
这就是"点了也不会重写任何文件"这句提示的实情 —— 它说的是真话，只是它同时让
另一条待办永远消不掉。

### 3.4 为什么发布也救不了

`publish_into` 只写 `manifest.json` / `source.json` 并推远端；
`catalog_sha_size` 这项发布闸是**读** `delivery/catalog.json` 然后核对盘上字节：

```rust
for f in &catalog.files {
    let Ok(got) = std::fs::read(publish_root.join(&f.path)) else { ... };
    if got.len() as u64 != want_size || sha256_of(&got) != want_sha {
        bad.push(format!("{}（字节与登记的不同）", f.path));
    }
}
```

所以发布闸会红着拦下来（**这是对的**，它守住了"发出去的字节 = 登记的字节"），
但它不会顺手修。发布不是这个 bug 的出口。

### 3.5 触发的时间线（能对上现场）

| 时间 | 事件 | 盘上状态 |
|---|---|---|
| 11-05 04:17 | 那批产物生成，**当时渲染器还写 `# generated_at:`** | 8 份带时间行，4897 |
| 之后 | 渲染器改为不写 `# generated_at:`（`build.rs` 的 `render()` 只留 machine/variant） | 候选变 4855 |
| 11-05 19:04 | 部分产物落盘（mtime） | 仍 4897 |
| 11-07 01:19 | 一次生成跑了，`catalog.json` 重算成 4855 | 目录 4855，文件 4897 |
| 11-07 | 你看到"清单对不上 + 没有哪一份会变" | 僵住 |

也就是说：**这两件事（"容忍历史时间行"与"目录按新字节登记"）分别都合理，
合起来就矛盾。** 没有人把它们放在一起验过。

---

## 4. 为什么排查慢 —— 逐条列"卡点"

这一节直接回答你的问题：是不是日志不够？

### 4.1 卡点一：现象本身自相矛盾，第一反应会怀疑"两处读的不是同一份东西"

界面上两条提示来自两个不同的 IPC 命令：

- 检查卡 ← `wb_preflight` → `issues::preflight` → 参数 `delivery` ← `audit_catalog`
- 生成卡的"无变化" ← `wb_delivery_status` → `delivery_status` → `delivery_diffs`
- 生成前确认 ← `wb_generate_preview` → `planned_todos` + `delivery_diffs`

**三个命令、三条路径、同一份目录。** 第一直觉是"它们读的目录不一样"
（影子树 vs 真仓库这段历史确实有过这种坑，`book_with_probe` 的注释专门写了
"产物根必须是真仓库 `ctx.real_root`，不是 `presets.root()` 影子树"）。
于是第一步必然要排除"读错目录"，而不是直击判据 —— 但实测三条命令的根是同一个
（`paths::delivery_root_at(ctx.real_root)`），排除这一项就花了时间。

### 4.2 卡点二：两条判据都是"只读 + 返回 Ok"，失败不产生任何错误

`delivery_diffs` 与 `drift_of` 都不会因为"两个结论互相矛盾"而报错：

- `delivery_diffs` 只负责"每份文件是 Same / Modified / Added / Removed"，
  它不知道 `catalog.json` 里登记的是多少字节；
- `drift_of` 只负责"登记 vs 真字节"，它不知道生成打算不打算写。

**没有任何一处代码同时持有两个结论并比较它们。** 所以：

- 没有 `Err`；
- 没有 `tracing::warn!`（这两条路径上一次 trace 都没有）；
- 界面把两个数并排摆出来，但**不认为它们冲突**。

这就是"日志不够"的真实形状：不是缺关键日志，而是**这两条判据之间没有任何一个
地方有資格说"你们矛盾了"**。想加日志都找不到合适的位置 —— 除非先想明白矛盾在哪。

### 4.3 卡点三：界面的"修复建议"指向了一个不会生效的动作

检查卡写"生成一次会重算登记"（`issues.rs` 的 `summarize_mismatch`），
而生成正是那个判 `Same` 的动作。**建议本身是对的**（生成确实会重算 catalog），
但它没说"生成也可能认为不需要写"。

这让排查方向偏向"再点一次生成试试""是不是缓存""是不是没重启"，
而不是"生成凭什么认为不需要写"。

### 4.4 卡点四：42 字节这个线索需要主动去量，界面上不显示

界面上给的是"目录登记 Some(4855) 字节，实际 4897 字节"。人眼看到的是"两个数不一样"，
看不出"每份都差 42"。而"每份都差同一个常量"是从"内容问题"跳到"形状问题"的
唯一桥梁。要拿到它必须逐份量（我上面那张表）。

如果当时界面直接写"8 份都比登记多 42 字节"，根因会快得多。

### 4.5 卡点五：文案把 TOML 叫成"资产"

「8 份资产字节变了」——`drift` 是 `catalog.files` 全体，既含 `mkp_preset` 也含
`icon/model/bbs_config`。8 条全是 `mkp_preset`，文案却叫"资产"。
这会让人先去查 `presets/assets/` 那一侧（那 17 条恰好全是 OK），
在一个健康的分支上耗时间。

### 4.6 卡点六：`# generated_at:` 这个字段的语义在仓库里有六处读者

`strip_generated_at` / `GENERATED_AT_PREFIX` 的读者：

```
workbench/domain/derive.rs:407        盘上 vs 现在渲染 → 是否"最新"
workbench/app/build.rs:922            内核产物"内容没变就不写"
workbench/app/build.rs:1285           两份文本比较（基线对齐）
workbench/app/build.rs:1328           同上
workbench/app/delivery.rs:788         候选集 vs 盘上"要不要写"
workbench/app/delivery.rs:3876        测试造历史产物
```

每一处都在回答不同的问题，其中 `build.rs:922` 那一处**特意加了**一个附加条件：

```rust
let same = std::fs::read_to_string(&target).ok().is_some_and(|old| {
    old.contains(crate::workbench::GENERATED_AT_PREFIX)   // ← 盘上必须有这行
        && crate::workbench::strip_generated_at(&old) == r.text
});
```

理由写在旁边："没有 `# generated_at:` 时，光比内容会得出'没变 → 不写'，
那一行就永远补不上（自锁）。"

**同一个人、同一个自锁形状，在 `build.rs` 里已经被识别并解决过一次**，
`delivery.rs` 里那一处却没有。这不是疏忽能解释的 —— 是因为两处的"不写"后果不同：

- `build.rs` 那处不写的后果是"时间行补不上"（纯显示问题）；
- `delivery.rs` 那处不写的后果是"与 catalog 登记的字节永久不符"（一致性闸永远红）。

后果不同，但**判据写法一样**。这是本次排查最耗时的认知落差：
看着几乎相同的代码，一处有守卫一处没有，需要逐字对比才能发现。

---

## 5. 为什么"加日志"救不了这类问题

值得说清，因为它决定了以后要不要在这种地方埋日志。

这类 bug 的特征是：**两个子系统各自诚实，合起来不诚实。**
所有单独看都 `Ok`：

| 函数 | 返回 | 它诚实地回答了什么 |
|---|---|---|
| `render()` | 文本 4855 | "现在会写出的内容" |
| `candidate_once()` | catalog 含 4855 | "按候选算的目录" |
| `delivery_diffs()` | Same ×8 | "内容没变" |
| `drift_of()` | 8 条 Size | "字节不对" |
| `generate_transaction()` | written=0 | "没有需要写的" |

五条全对。**bug 在"没人把它们放一起比"。**

要加能抓到它的日志，就得先知道"`Same` 与 size 不符同时出现是异常"——
也就是已经知道答案。所以：**这不是日志覆盖不足，是缺少一条不变的判据（invariant）。**

正确的做法不是埋日志，而是**把这条矛盾变成一条会红的判据**（见 §6）。

---

## 6. 修法

### 6.1 判据（核心一行）

`delivery.rs` 的 `delivery_diffs`：

```rust
let same_content = norm_old == norm_want;
/*
 * 「不用写」还要盘上那个字节形状就是当前格式。
 * 判据取单向的：norm_want == c.want 说明候选本身没有这些字段、
 * 归一化对它是恒等，此刻 norm_old != bytes 就只能来自盘上那份过期。
 */
let stale_shape = same_content && norm_want == c.want && norm_old != bytes;
if same_content && !stale_shape {
    (DiffKind::Same, ...)
} else {
    // 内容是"一致"，但形状旧 ⇒ 写一次清掉
    let why = if same_content {
        "内容一致，盘上这份还带着已废弃的字段（写一次清掉）"
    } else { first_difference(&norm_old, &norm_want) };
    (DiffKind::Modified, ...)
}
```

三个条件各管一件事：

1. `same_content` —— 内容真的没变（否则本来就是 Modified）；
2. `norm_want == c.want` —— **候选自己不需要归一化**。这一条保证"归一化不是为了
   抹平候选自身的字段"，否则会把"候选本来就带该字段"误判成"盘上过期"，
   变成每次生成都重写（自带新的假象）；
3. `norm_old != bytes` —— 盘上那份确实被归一化动过 ⇒ 它带着已废弃的字段。

**单向**是关键：只惩罚"盘上旧"，不惩罚"候选新"。这样即使将来 CAD 又把某个字段
加回渲染器，也不会反转成每次重写。

### 6.2 为什么不是"直接不归一化"

一个更"简单"的修法是让 `delivery_diffs` 对 TOML 不做归一化、逐字节比。
**那是错的**：会让"只是时间戳不同"的历史文件被判成"内容变了"，
生成每次都重写全部 9 份，产物 mtime 天天变，"内容没变不写"这条规则当场作废，
而且 `# generated_at:` 一旦重新写入就再也收敛不了（因为它每次不同）。

所以：**归一化保留（它回答内容），形状判据单独加（它回答要不要写）。**
两个问题分开问，不合并。

### 6.3 旧判据的期望要跟着改

`legacy_timestamp_is_comparison_only_and_new_bytes_are_deterministic`
原来断言"旧时间戳不该制造假差异 ⇒ 判 `Same`"。这条断言的**意图**（不制造内容差异）
仍然对，但它的**结论**（所以不写）与 `catalog` 的字节口径冲突。
改成断言 `Modified`，并在文档注释里写清："不制造内容差异，但那一行要清掉 ⇒ 判待更新"。

断言没被删掉、也没被放宽 —— 是**同一件事在两条判据里被要求了两个不同答案**，
现在统一到"要写"这一侧。

### 6.4 新判据

`a_legacy_generated_at_line_is_rewritten_instead_of_ignored`：

1. 首次生成，落一份全一致；
2. 确认候选字节不含 `# generated_at:`；
3. 手动把那 42 字节插回头注释块（复刻历史产物）；
4. `delivery_diffs` 必须判 `Modified`（**这一步是反空转：修之前它判 `Same`**）；
5. 生成一次；
6. 盘上字节必须**逐字节等于候选**（旧行 gone）；
7. 再比一次，必须判 `Same`（收敛，不是每次都重写）。

第 4 步与第 7 步成对：前者防"该写不写"，后者防"不该写一定写入"。

### 6.5 文档

模块头补了一节「catalog.json 归生成管，不归发布管」，把这条容易误会的分工写死：

> `catalog.json` 的 `files[]` 是按**候选字节**登记的（size / sha256），而"生成"正是
> 产生候选字节的那一步 —— 所以它是 `generate_transaction` 的写入项。发布只**读**它
> （并把它一起推出去），不重算。

这也是你第二个问题的直接回答：**是的，目录本来就该在生成时做好，发布不该也不做这件事。**

---

## 7. 验证

```
cargo test -p mkp-support-ease --features workbench --lib -- \
    workbench::app::delivery::tests workbench::app::build::tests
→ 60 passed; 2 failed; 1226 filtered out
```

两条红的与本次无关，是工作区里的源改动造成的"源与交付目录不同代"：

| 测试 | 原因 |
|---|---|
| `delivery_catalog_matches_rebuild` | `presets/machines/P2S.toml` 加了 `enabled = false`，入库的 `catalog.json` 还含 P2S 整机 |
| `the_manifest_covers_exactly_the_referenced_deliverable_set` | 同上：`manifest.json` 里有 `P2S-standard.toml`，重算的 catalog 已不含它 |

反空转已验证：临时把 `P2S.toml` 的 `enabled = false` 去掉，这两条即通过
（上一轮已验过，本轮未重复执行，因为它要动你的源文件）。

本轮相关的三条全绿：

```
test delivery::tests::a_legacy_generated_at_line_is_rewritten_instead_of_ignored ... ok
test delivery::tests::legacy_timestamp_is_comparison_only_and_new_bytes_are_deterministic ... ok
test delivery::tests::delivery_status_classifies_up_to_date_pending_and_missing ... ok
```

---

## 8. 你要做的三步

1. **重启工作台**（改的是 Rust，dev 入口带 `--no-watch`，不重启不生效）。
   判"跑的是不是新的一版"只认构建指纹（工作台 `presets build` 输出的
   `档位 · 提交 · 编译于 …`，`编译于` 是 exe 的 mtime）。
2. **点一次生成**。预期：8 份 TOML 从「无变化」变成「待更新」，
   `first_diff` 写的是"内容一致，盘上这份还带着已废弃的字段（写一次清掉）"。
   确认后写一次，42 字节消失。
3. **看一眼检查卡**：`delivery.catalog_mismatch` 那条待办应当同时消失。
   同时 P2S 的产物（`P2S-standard.toml`）会被按"当前源不再产出"移进回收站 ——
   这也会顺带让上面两条红测试变绿。

---

## 9. 由此得到的结构性结论

### 9.1 一条不变式，以后应该常驻

> **`Same` 必须蕴含"盘上字节 == 候选字节"。**

现在它只在加上 `stale_shape` 之后才成立。这条不变式值得单独立一条测试
（甚至一个 debug_assert）：任何被判 `Same` 的候选，其 `want` 必须与盘上字节
**逐字节相等**，而不只是"归一化后相等"。

`a_legacy_generated_at_line_is_rewritten_instead_of_ignored` 的第 6 步就是这个意思，
但它只覆盖 TOML 的历史时间行；JSON 侧（`generatedAt` / `generated_at` / `updated`
三个历史键）走的是同一个 `normalize_for_compare` 分支，理论上同样会自锁。
**建议下一步补一条对称的 JSON 版本**，或者干脆把 §9.1 写成通用断言。

### 9.2 "两个子系统结论矛盾"没有守卫，是这类 bug 的温床

`catalog.json` 登记的是字节，生成判的是内容 —— 这两个口径**天然不同**，
而且它们在不同的文件、不同的函数、不同的 IPC 命令里。

最省事的长期解法：**让 `delivery_status` 自己交叉验一次**。它已经同时拿到
`diffs`（每份的 Same/Modified）和 `catalog`（每份的登记 size/sha）。
一句断言就够：

> 若 `diffs[i].kind == Same` 但盘上 size 与 `catalog.files[i].size` 不符
> ⇒ 这是一条 `drift`，且**必须**同时把该份的 `state` 从 `UpToDate` 降成 `Pending`。

这样界面不会同时显示"一致"和"对不上"，矛盾在源头就暴露成一条待更新。
（本次没做这一步，因为它会改变 `delivery_status` 的口径与前端显示，属于要你点头的
产品行为。先记在这里。）

### 9.3 文案

两处该改（都属于显示层，改起来很小，但也需要你确认措辞）：

1. `BuildPage.tsx`：「N 份**资产**字节变了」→ 应区分「N 份产物」与「N 份资产」，
   或者干脆说「N 份文件与目录登记不符」。
2. 检查卡 `summarize_mismatch`：「生成一次会重算登记」→ 在 8 份全是
   size 差同一常数的情况下，这句话会误导；更好的说法是先说清
   "盘上这份比登记多 N 字节"。

### 9.4 关于"要不要在生成里加一句提示"

当 `drift` 非空而 `modified` 为 0 时，界面上其实是一个**真实的矛盾态**，
即使修好这个具体 bug，将来也可能因为别的原因再次出现（比如有人手工改了产物）。
可以考虑在这种状态下让生成卡直接显示一句：

> 有 N 份与目录登记不符，但生成认为内容没变 —— 可能是盘上那份形状旧了，
> 点「重算目录与清单」可强制重写。

这属于产品行为，等你定。

---

## 10. 附：本次改动的清单

| 文件 | 改动 |
|---|---|
| `src-tauri/src/workbench/app/delivery.rs` | ① `delivery_diffs` 加 `stale_shape` 判据；② 新增测试 `a_legacy_generated_at_line_is_rewritten_instead_of_ignored`；③ `legacy_timestamp…` 期望 `Same`→`Modified` 并改注释；④ 模块头补「catalog 归生成管」一节 |
| `.codebuddy/memory/2026-11-07.md` | 记录本次根因、修法、口径 |
| `docs/REPORT-DELIVERY-CATALOG-SELF-LOCK-2026-11-07.md` | 本报告 |

**未改动**：`presets/delivery/**` 任何一个字节（盘上那 8 份仍带着时间行，
等你在界面上点生成清掉）；`presets/machines/P2S.toml`；任何发布元数据。

---

## 11. 一句话回到你的问题

> 是不是日志不够？

不是。**是两条判据各自诚实、合起来矛盾，而没有任何一处代码负责发现这个矛盾。**
日志只能记录"某处认为某事"，记录不了"两处认为的事冲突"。

> 还是逻辑相互冲突？

是。具体是**"内容"与"字节"两个口径**被同一个 `Same` 同时表达：
生成拿它决定"不写"，`catalog.json` 拿字节做登记，检查卡拿字节做核对。
一个词，两个含义，第三个消费者。修法是把这个词拆开 —— `Same` 只表示"内容一致"，
"要不要写"另有一条 `stale_shape` 判；再补一条 §9.1 的不变式让它以后不会再合回去。

> 而且它排查得久，是因为现场对不上、建议不生效、文案叫错名字，
> 三条一起把方向带偏。这些都在 §4 里逐条列了。
