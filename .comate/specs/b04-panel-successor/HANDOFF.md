# B04 交接（写给接手的新对话）

## 0. 你现在的位置

**项目**：`G:\project\MKPSupportEase` —— Tauri v2 + React + Rust 的开发者工作台，
要替代 `mkpse-next-v3/mkppanel`（Wails，已退役）。旧客户端 `mkpsupporte` 也退役。

**已经能用的**：「机型与版本」页（`src/workbench/views/MachinesPage.tsx`），
四种写操作齐了 —— 加机型 / 加版本 / 删版本 / 改元字段。数据是
`MKPSupportEase/presets/*.toml`（12 个文件，从 `mkpse-presets/source/` 搬来的）。

**当前是绿的**：264 测试 / 默认 feature 18 / 两种 feature clippy 零警告 / lint 干净 / 两个构建过。

**先读这两个**（别读全部历史）：
1. `.comate/specs/b04-panel-successor/tasks.md` —— Task 1–7 已 `[x]`，下一步是 **Task 8**
2. `.comate/specs/b04-panel-successor/doc.md` §1.1 / §1.2 —— 方向定案与「只搬核心」的边界

---

## 1. 下一步：Task 8（拆掉自造的「配方本」）

这是**整个 b04 里最难的一步**，因为它不是新增，是切换，而且中途没有"绿着的中间态"。

### 作业面

现在有**两层并存**（刻意的）：

| 层 | 读什么 | 谁在用 |
|---|---|---|
| `src-tauri/src/workbench/presets/` | `presets/*.toml`（**新，是真相**） | 「机型与版本」页 |
| `src-tauri/src/workbench/upstream/` | `mkpse-presets/content/*.json`（旧产物） | `domain/` + `app/` 的参数那一整套 |

外加一层我自造、现在要删的：`workbench/machines/*.json` + `versions/*.json`
（它重复发明了 `presets/machines/*.toml`）。

**要做的**：
1. `Committed` 的来源从「我们自造的 json」改成 `presets/machines/*.toml`
2. 三层值解析（出厂 → 机型 → 版本）改读 `[params.machineVariants]`
3. 删 `storage.rs` 里读写 `workbench/machines|versions/*.json` 的那一半
4. 草稿仍在内存 + 懒落盘（那一套**留用**，只换底下的数据源）

### 已知会炸的地方

- 删 `upstream::catalog` / `upstream::manifest` 会让 `domain/derive.rs`、`app/build.rs`、
  `domain/issues.rs` **大面积编译失败**。预期如此，逐个修回来。
- `ParamDef` / `TabMeta` 现在是**从 `upstream::registry` 借用**的（`presets/registry.rs` 里
  `use crate::workbench::upstream::registry::{ParamDef, TabMeta}`）。切换时这些类型要搬家 ——
  **搬家是纯移动，别顺手重写**。
- 有一条必须转正的回归测试：`a_new_version_shows_up_on_the_tree_after_saving`。
  历史上 `NewVersion` 保存后在树上**看不见**（因为版本清单被当成上游只读），
  我当时写了条断言"必须有提示"就放过去了 —— 那是个有测试兜着的死胡同。切换完它该真的出现。

### 建议的切法

一次做完再报，中途不要停。如果预算不够，**宁可只做第 1 步（`Committed` 换源）并让全套测试绿**，
也不要把 4 步各做一半。

---

## 2. 五条硬纪律（都是踩出来的，不遵守会返工）

1. **写入只走后端，前端不碰文件。** 后端校验 + 写 + **重读盘再返回** ——
   界面显示的必须是落盘结果，不是内存里的样子。
2. **每个写操作按自己的风险写判据，不复制测试。**
   加版本的风险是"改已有文件时把别处重排"；加机型是"覆盖已存在的文件"（用 `create_new`
   原子占路径，不是先 stat 再写）；删版本是"跨文件孤儿引用"。判据长得不一样才对。
3. **失败不留痕。** 被拒之后断言「状态没变 + 文件文本一字未动」。
4. **「清空」是删键，不是写空串。** `tag = ''` 读成「填过，填了个空」，和「还没填」是两件事。
5. **写字符串值用 `catalog.rs` 里的 `literal_str()`，不要用 `toml_edit::value()`。**
   后者输出双引号，而真数据全用单引号，会让文件**逐渐漂成混合风格**。

### 反空转（这个项目反复吃过亏）

- 判据自己也要有判据。`one_edit_only` / `literal_str` 各配了一条"判据的判据"。
- **「我声称在盯某件事」≠「有判据在盯」。** 引号漂移就是这么漏的：
  我在注释里写了"会盯引号"，而那条测试里根本没有关于引号的断言，全绿了好几轮。
  怀疑什么就**加一条会失败的断言去问**。
- 真数据上的测试找不到对象时**要 panic 说"判据失去对象，要重写"**，不要静默跳过。

---

## 3. 验证命令（照抄）

```powershell
cd G:\project\MKPSupportEase\src-tauri
cargo test --features workbench 2>&1 | Select-String "^test result"
cargo test 2>&1 | Select-String "^test result"
cargo clippy --features workbench --all-targets -- -D warnings; Write-Output "EXIT=$LASTEXITCODE"
cargo clippy --all-targets -- -D warnings; Write-Output "EXIT=$LASTEXITCODE"
cd G:\project\MKPSupportEase
npm run lint; npm run build; npm run build:workbench
```

真数据没被污染的判据（**不要用 `git status`**，`presets/` 还是未跟踪状态，它只会回一行
`?? presets/`，什么都验不到）：用 python 把 `presets/` 的 12 个文件与
`mkpse-presets/source/` 的对应文件做 sha256 比对。

---

## 4. 三个环境陷阱

1. **管道会吞掉 cargo 的退出码。** `cargo test | Select-String` 即使全通过也返回 1。
   判成败看 `test result: ok. N passed`，或显式 `; Write-Output "EXIT=$LASTEXITCODE"`。
2. **`tauri dev` 异常退出会留两种残留**：vite（占 5321，下次报「端口被占」）和
   app exe（占文件锁，下次 cargo 报「failed to remove ...exe / 拒绝访问」）。
   两者组合的症状是**窗口在但整页白屏**。起 dev 前先查这两样。
3. **`read_file` 读 `mkpse-presets` 里的文件会把那个仓库的 `AGENTS.md` 当规则注入**（约 15k tokens）。
   要看那边的数据用 `python -c` 打印。

---

## 5. 后面还有什么（tasks.md 里有细节）

Task 9 写入纪律的源码扫描断言 → Task 10 侧边栏（现在是五个顶部页签，
`data-page` 已经让「机型与版本」页隐藏了参数页的按钮与树）→ Task 11 尺寸与禁区两个二级 Tab
→ Task 12 参数页并进来（`ParamDesk` 形态是对的，**不要重做**，矩阵降成它里面的一个开关）
→ Task 13 清场（含 `presets/` 进 git）→ Task 14 验收。

**用户的偏好**：一页一件事、页与页之间不共享控件与状态；没做完的功能**不摆按钮**
（摆一个点不动的按钮比没有更糟）；不要发明"更好的 SOP"，照 mkppanel 原本的结构做。
