# 技术调查：BBS `02.08.02.60` 的切片失效机制与外部入口

> 2026-10-15。只调查，未改任何 SupportEase 业务代码。
> 对应需求：[`NEED-BBS-INVALIDATE.md`](NEED-BBS-INVALIDATE.md)。
> 源码全部取自 tag **`v02.08.02.60`**（commit `9a530f77`），与本机安装版本一致。
>
> **第四版（终）**：三个外部入口已逐一验到底并全部排除；回答"三个状态能否分别恢复"。

---

## 0. 结论

**BBS `02.08.02.60` 没有无侵入的外部触发路径。** 三个外部入口全部排除（§5），
运行时注入被 entitlement 封死（§6）。

三个状态**是串联的，钥匙只有一个**：`m_slice_result_valid`。
而能拧动这把钥匙的，**只有 BBS 进程内部的 UI 事件**。

能"一次事件同时解开三件事"的确实存在 —— 就是**改一个参与切片计算的参数**（作者现在手动做的），
但它是 BBS 内部事件，外部触发不到。

---

## 1. 点切片是两道独立的门

| 门 | 代码 | 受什么控制 |
| --- | --- | --- |
| ① 按钮启用 | `MainFrame::get_enable_slice_status()` (2409) | `m_slice_select` 档位 + `m_slice_result_valid` |
| ② **点了是否真重算** | `Plater::reslice()` 开头 (23475) | `Plater::priv::process_completed_with_error` |

作者实测「Slice all 档按钮可点但点了不切片」⇒ 门①开了、门②没开。

```cpp
// MainFrame.cpp:2409-2456
if (m_slice_select == eSliceAll) { enable = true; }        // 切全部盘 → 永远启用
else if (m_slice_select == eSlicePlate) {
    if (current_plate->is_slice_result_valid()) enable = false;
    else if (!current_plate->can_slice())       enable = false;
}
```

```cpp
// Plater.cpp:23468-23480
void Plater::reslice() {
    if (p->process_completed_with_error == p->partplate_list.get_curr_plate_index()) {
        reset_gcode_toolpaths();
        return;                                            // ★ 什么都不做
    }
```

---

## 2. ★ 三个状态能否分别恢复（回答作者的核心问题）

### 状态 A：`Plater::priv::process_completed_with_error`（int，`Plater.cpp:7132`）

- 谁设：后处理失败/停止后，`update_background_process()` 里 11043 设为**当前盘索引**。
- **能不能恢复：能。** 唯一清除点 `Plater.cpp:11070`：
  ```cpp
  else if (!background_process.empty() && !background_process.running()) {
      if (cur_plate->can_slice()) { update_slice_print_status(..., true); process_completed_with_error = -1; }
      else                        { update_slice_print_status(..., false); process_completed_with_error = idx; }
  }
  ```
- **但有前提**：必须先让 `background_process.finished() && is_slice_result_valid()` 这个
  "双真"不再成立（见下），否则走的是上面的分支①（只更新按钮、**不动标记**）。
- 还要有一次 `update_background_process()` 被调用（后台定时器 `Plater.cpp:7331` 会周期跑，
  但它同样会撞上双真 ⇒ **等再久也不会自己好**）。

⇒ **A 能恢复，但必须 B 先变成 false。**

### 状态 B：`PartPlate::m_slice_result_valid`（bool）

- **能不能恢复：能**（置 false 即可，`PartPlate.cpp:3543`）。
- **但入口全是 BBS 内部 UI 事件**。本版本全部调用点（12 处）：
  `PartPlate.cpp` 的 `214`(set_bed_type，`if old!=new`)、`252`(set_print_seq)、`2340`(clear)、
  `3543`(定义)、`3666`(置真)、`5659`/`5691`/`5699`/`5762`/`5801`(PartPlateList)、`6845`(invalid_all_slice_result)、`6959`。
  **没有一个是"读盘 / 收消息"触发。**
- `PartPlateList::invalid_all_slice_result()`（6840）是最像"一把全解开"的入口 ——
  它遍历所有盘置 false。但它的**唯一调用点**（`Plater.cpp:24466`）只在
  `opt_key == "printer_technology"`（打印技术 FFF↔SLA 切换）时触发，**不是正常操作**。

⇒ **B 能恢复，但钥匙只在 BBS 内部。**

### 状态 C：旧的切片结果与 G-code toolpaths

- `reset_gcode_toolpaths()` 清的是**显示**；真正被替换要靠下一次**真的**切片跑完。
- 作者观察"切到其他盘再切回来，显示的还是后处理之前的原本" ⇒ 与 B 没变 false
  完全一致（切盘走的是 `update_restart_background_process`，同样撞上双真分支①）。

⇒ **C 是 B 的下游，B 不动它就不动。**

### 2.1 串联关系

```
B (m_slice_result_valid = false)
   └─→ 定时器/update_background_process 走到分支②
          └─→ A (process_completed_with_error = -1)
                 └─→ reslice() 不被拦 ⇒ 真的重算 ⇒ C 被覆盖
```

★ **唯一的钥匙是 B。A 和 C 都是它的下游，没有独立的恢复入口。**

⇒ 所以"一个事件同时解开三件事"确实存在：**改一个参与切片计算的参数**。
它不是外部事件，是 BBS 内部事件 —— 这正是作者现在手动做的事。

---

## 3. 失效函数本身（回答"它还做了什么"）

`PartPlate.cpp:3543-3554`：
```cpp
void PartPlate::update_slice_result_valid_state(bool valid)
{
    m_slice_result_valid = valid;
    if (valid) m_slice_percent = 100.0f;
    else       m_slice_percent = -1.0f;
}
```
**纯 setter**，只写 bool + float，无任何事件通知。按钮刷新是 MainFrame 侧自己重算的。
⇒ 若真走"改内存"路，**至少动这两个成员**，不能只改一个 bool，且要等定时器跑一轮。

---

## 4. 为什么 SupportEase 的 TOML 变化不会传播

1. **BBS 不认识这些参数**：`user/<id>/process/MKPProcess A1 mini.json` 所有顶层键里
   没有任何 MKP / offset / pen / glue 字段。
2. **它连读都没读过**：偏移住 `presets-local/*.toml`，是钩子命令行 `--Toml` 的字符串入参。
3. **没有读盘重放**：`strings` 整个二进制，`filesystemwatcher` / `preset reload` /
   `externally modified` 零命中；`BambuStudio.conf` 里也没有切片结果时间戳。

⇒ 改我们的 TOML ⇒ 没有参数面板值变化 ⇒ 走不到 §2-B 那些调用点 ⇒ 钥匙不动 ⇒ 门②不解。

---

## 5. ★ 三个外部候选入口，逐一验到底，全部排除

### 5.1 URL scheme `bambustudioopen://` —— **只能下载远程模型，不能打开本地工程**

`GUI_App::MacOpenURL`（`GUI_App.cpp:7661-7704`）：

```cpp
if (!url.empty() && boost::starts_with(url, "bambustudioopen://")) {
    auto input_str_arr = split_str(url.ToStdString(), "bambustudioopen://");
    ...
    std::string decoded_url = url_decode(download_origin_url);
    // 正式版还有域名白名单（makerworld / public-cdn.bblmw.com / amazonaws / aliyuncs），
    // 不在白名单弹「This file is not from a trusted site, do you want to open it anyway?」
    download_file_url = sanitize_download_url(download_file_url);

    // ★★ 只处理 http:// 与 https://
    if (!download_file_url.empty() && (starts_with(download_file_url, "http://") ||
                                       starts_with(download_file_url, "https://"))) {
        request_model_download(download_file_url);
    }
}
```

⇒ 格式是 `bambustudioopen://<urlencoded>`，但**只接受 http(s)**；
本地路径、file:// 一律不处理。**这条路对"重开当前工程"无效。**

### 5.2 打开文档（`open -a BambuStudio xxx.3mf`）—— **会起第二个 BBS 实例**

`GUI_App::MacOpenFiles`（`GUI_App.cpp:7706-7722`）：
```cpp
bool single_instance = app_config->get("app", "single_instance") == "true";
if (m_post_initialized && !single_instance) {
    bool has3mf = false; ...
    if (has3mf) { start_new_slicer(names); return; }      // ★ 新进程
}
```
本机 `BambuStudio.conf`：`app.single_instance = False`
⇒ 有 3mf 时走 `start_new_slicer` ⇒ **启动第二个 BBS 实例** ⇒ 违反需求 §3
（"不启动另一个 BBS 进程"）。改成 true 才有可能 —— 那是改用户配置，且需重启生效。

（附带：`recent_projects.001` 确实能拿到最近工程路径，例如
`/Users/wzy/Downloads/千夏3mf2.3mf` —— 路径不再是障碍，障碍是上面这条。）

### 5.3 CLI —— 另起进程，且没有 `--export-gcode`

`--help` 全文 100 行：有 `--slice` / `--datadir` / `--load-settings` / `--outputdir`；
`instance`/`ipc`/`socket`/`server`/`port` 零命中；`--pipe` 是**出向**进度；
导出只有 3mf / png / settings / slicedata / stl / stls。
⇒ 管不了眼前那扇窗，也大概率不触发后处理钩子。

---

## 6. 运行时注入 —— 本机被封

| 手段 | 结论 | 证据 |
| --- | --- | --- |
| lldb attach | **不可行** | entitlements 只有 `cs.allow-unsigned-executable-memory` + `cs.disable-library-validation`，**无 `get-task-allow`**；SIP enabled |
| DYLD 注入 | **不可行** | 无 `cs.allow-dyld-environment-variables`，Hardened Runtime 会剥掉 |
| 静态定位 | 可行 | 符号表未 strip（412735 条） |
| 改安装包 / 重签名 | 不做 | 需求 §3 排除 |

---

## 7. 卡在哪一层（按需求 §6 逐条）

- **实际失效状态由谁控制**：`PartPlate::m_slice_result_valid`（bool，钥匙）与
  `Plater::priv::process_completed_with_error`（int，点切片的实际门槛）。二者串联（§2.1）。
- **为什么 TOML 变化不传播**：§4 三处断点。BBS 根本不知道这个参数存在。
- **哪种入口能真正触发**：BBS 内部一次"参与切片计算的参数赋值"（等价于作者手改板型）。
  A/C 是 B 的下游，没有独立入口。
- **是否适用当前版本**：源码取自 `v02.08.02.60`，与本机一致。
- **最小可行实现及风险**：§8。
- **无法无侵入时卡在哪**：**卡在 `m_slice_result_valid` 只有 BBS 内部 UI 事件能改。**
  三个外部入口（URL scheme / 打开文档 / CLI）已逐一排除，运行时注入被 entitlement 封死。

---

## 8. 剩下能做的（都还没实施）

| # | 做法 | 能否零手动 | 风险 / 卡点 |
| --- | --- | --- | --- |
| P1 | **UI 自动化**：替用户做一次无害赋值（改一个参与切片的参数再改回，或切盘再切回） | 能 | 要辅助功能权限；脆（BBS 升级即废）；点错有风险。**唯一能真正自动化的路** |
| P2 | 停止时退出码退 0（假装成功） | 能 | ★ 违反需求 §3 最后一条；且用户可能直接拿未处理的 G-code 去打印。**不建议** |
| P3 | 如实报告限制，在「已停止」那屏给一句操作指引 | 否 | 零风险 |

（已排除：切盘法、Slice all 档、`bambustudioopen://`、`open -a …3mf`、CLI、内存注入。）

---

## 8.5 ★ 转向：屏幕引导方案的可行性验证（2026-10-15 实测）

方向变了：不再试图自动失效，而是 **SupportEase 识别 BBS 窗口 → 悬浮箭头指向目标 → 用户自己点**。
不模拟点击、不碰 BBS 内存。下面是第一阶段"能不能定位"的实测结果。

**实测环境**：BBS 在跑（pid 42952），窗口 `name = "* 千夏3mf2"`（`*` = 有未保存改动），
`position = {0, 30}`，`size = {1440, 786}`。

| 能力 | 结果 | 证据 |
| --- | --- | --- |
| 读窗口名 / 位置 / 大小 | ✅ **可以** | `get {name, position, size} of window 1` → `* 千夏3mf2` / `0,30` / `1440x786` |
| 读菜单栏 | ✅ 可以 | `count of menu bar items of menu bar 1` = 8 |
| 读窗口内控件 role + **屏幕坐标** | ✅ **可以** | 窗口直系 78 个元素，逐个拿到 `AXButton` / `AXStaticText` / `AXCheckBox` / `AXTextField` / `AXImage` 与 `position`+`size`（示例：`2 AXButton 10,105 18x18`、`22 AXCheckBox 82,452 70x24`） |
| 读控件的**语义标识**（title / identifier / description） | ❌ **基本没有** | 子元素 properties 里 `titl = missing value`、`name = missing value`、`axds = ""` ⇒ **拿不到"这个按钮是切片"这种信息**，只有坐标和类型 |
| 定位"切片"按钮本身 | ⚠️ **存疑** | 右上角区域只见 `AXImage 1178,68 5x24`（像下拉箭头），**未见 AXButton** ⇒ 切片按钮可能是自绘控件（`m_slice_btn` / `SideButton`），未进 AX 树 |

### 踩到的坑（下次别再踩）

- `every button of entire contents` 语法在 System Events 里不成立（报 -1700 不能转 specifier）。
- **不要在 AppleScript handler（子程序）里遍历传入的元素列表** —— 会丢 specifier 上下文，
  role/position 全部取不到（我第一轮就是这么误判成"BBS 不暴露控件"）。**要直接在 `tell` 块里 repeat。**
- `properties of 元素` 里有 `NSImage` 时转 text 会炸（报 -1700），**别取 properties**，只取 role/position/size。

### 下一步（需要作者在场，人工做）

用 **`/Applications/Xcode.app/Contents/Applications/Accessibility Inspector.app`**
指向 BBS 的「切片」按钮，看它：① 是否在 AX 树里；② 有没有 AXIdentifier / 标题。
这一步决定引导方案能不能做到"指得准"，我无法代做（要鼠标悬停）。

若切片按钮不在 AX 树里 ⇒ 退化方案：窗口矩形 + 相对偏移（脆，改版即偏），或截图图像识别。

### 8.5.1 ★★ 定位算法跑通了（Rust 侧探针实测，仓库外 `/tmp/axprobe`）

**结论：不用固定偏移，改成"在当前窗口里遍历找特征元素"** ——
于是窗口缩放、左栏拖宽**都不影响**（x 是实时读出来的）。

两次实测（同一 BBS 进程 pid 42952，窗口被改成两种尺寸）：

| 窗口 | 缩略图#1 相对窗口 | 缩略图#2 相对窗口 |
| --- | --- | --- |
| `1440×786`（osascript 那次） | (61, **115**) | (200, **117**) |
| `1000×652`（Rust 探针这次） | (32, **115**) | (142, **117**) |

- ★ **相对 y 稳定不变**（115 / 117）—— 垂直方向可以当特征；
- **相对 x 会变**（61→32、200→142）—— 所以 x **必须实时读**，不能用偏移去算；
- 判据：`size == 50×50` 且 `相对窗口 y ∈ [100,140]` ⇒ 稳定命中 2 个。

**两个坑（写进实现时注意）**：
1. **`AXRole` 不是每次都取得到**（探针里 [5]/[8] 的 role 是空的，而 [2]/[11] 的 `AXStaticText` 能取到）
   ⇒ **别用 role 当筛选条件，用尺寸 + 相对 y**。
2. `CFStringGetCStringPtr` 对某些 CFString 返回 null（窗口 `AXTitle` 就取成空了）
   ⇒ 要 `CFStringGetLength` + `CFStringGetCString` 兜底。

**§8.5 里那个"双锚点校验"作废** —— 两个缩略图同在左栏，左栏拖宽时它们一起平移、间距仍对得上，
自检会**误通过**。真正的解法就是上面这个"遍历特征元素"。

### 8.5.2 还剩一个待确认：哪个是「板子类型」

遍历稳定得到 **2 个** 50×50 缩略图。按作者第一次给的坐标（`200,147`，对应 1440 窗口下的
**x 较大的那个**）⇒ **#2（x 较大）= 板型缩略图**，#1 多半是耗材色块。
★ 这是一次观测得出的，真做时要再确认一次（指错了箭头就指到耗材上）。

### 8.5.3 引导窗口落地（2026-10-15）

**形态**：一扇**透明无边框**的小窗（**376×256**），贴在「板子类型」控件**右侧**
（右边放不下就翻到左侧），`always_on_top`，同时把 BBS 用 `open -a` 抬到最前 ——
用户要看的是 BBS 里那个控件，卡片浮在它上面就行。

**窗口 ≠ 卡片**（这是第三轮改的重点，作者原话"阴影被窗口截断了，你可以把窗口做大，
卡片气泡小一点，边缘、阴影都显示完整"）：

| 数 | 值 | 干什么用 |
| --- | --- | --- |
| `CARD_W` × `CARD_H` | 340 × 224 | 气泡本体（`CARD_H` 是**量出来的**，见下） |
| `SHADOW_PAD` | 16 | 四边留给 CSS 阴影的空 —— 贴着窗口边会被**切掉** |
| `ARROW_PAD` | 20 | 箭头那一竖条：半对角线 8.5 + 箭尖外那圈阴影 ≈ 9 + 余量 |
| `CARD_GAP` | 22 | 卡片离控件的空。★★ **必须 ≥ `ARROW_PAD`**：窗口左边 = 控件右边再往左让一条
箭头竖条，`CARD_GAP < ARROW_PAD` 时窗口就**压到控件身上** ⇒ 那一块虽然透明、但**照样吃点击**，
用户点不到他要点的控件 |

⇒ 窗口 `20 + 340 + 16 = 376` × `16 + 224 + 16 = 256`；两档 `side` 只是**卡片在窗口里的
左/右不同**，窗口本身同样大。卡片竖直位置：窗口顶 = `target.y - 16` ⇒ 卡片顶正好落在
`target.y`；箭头中心 = 窗口顶下 `16 + 25 = 41` ⇒ `top: 35px`（目标 50×50）。

**"看不见 / 不够透明 / 被截断"是四层原因叠出来的**（都记下来，别再踩）：

1. ★★★ **`body` 被铺了不透明渐变**：引导窗口走的是**同一个 `index.html` + 同一份
   `global.css`**，而 `global.css` 给 `body` 铺了一层渐变（那是**主窗口**的底 —— 主窗口
   自己也 `transparent`，画面全靠它撑着，删不得）⇒ 引导窗整扇被铺满、白箭头画在白底上。
   判据：`main.tsx` 认了 `?guide=1` 就往 `html` 打 `data-guide`，
   `global.css` 里 `html[data-guide] body { background: none }` 收口。
   （`macOSPrivateApi: true` + tauri 的 `macos-private-api` feature 本来就在，不是那一层的问题。）
2. ★★ **原生窗口阴影按"窗口矩形"画**：透明无边框窗留着系统阴影 ⇒ 卡片之外的透明区
   能看到一块**灰底**（作者原话"不够透明，看到灰灰的"）⇒ 建窗时加 **`.shadow(false)`**，
   立体感交给 CSS。
3. ★★ **白箭头压不住白底**：切片器左栏本来就是白的，纯白三角摆上去等于没画。
   改成"**白方块转 45°**"：跟卡片共用同一块背景 + 同一圈 `1px rgb(0 0 0 / 10%)` 发丝描边，
   转出来的那半块压在卡片上（`z-index: 1`）⇒ 箭头**无论背后什么颜色**都看得见，接缝还是连的。
   ★ 去掉不需要那两条边要用 **`border-*-width: 0`**，不能用 `border-top: 0` 简写
   （简写会把颜色重置成 `currentcolor`）。
4. ★★ **阴影得挂在 `.root` 上**（卡片自己 `box-shadow: none`）：`filter: drop-shadow()`
   跟着"卡片 ∪ 箭头"的**轮廓**走；挂在卡片上的话那圈影子会摊到箭头那一竖条上，看着像
   "不够透明、灰灰的"。**同时**必须留出上面那张表里的 `SHADOW_PAD`——否则影子被窗口硬切。

**★★★ "被别的软件盖住了"要问系统层序，不问 AX 的焦点**（2026-10-15 作者："其他软件
盖住了 BBS，它还是这个还是一样的显示……有没有可能我们知道 BBS 的层级在哪……这是系统
的事情吗"）：

- **是系统的事，而且系统给了只读接口**：`CGWindowListCopyWindowInfo(OnScreenOnly)` 返回
  **按层序从前往后排好的**窗口列表，每项带 **归属 pid / 层级 / 矩形** ——
  读这三样**不需要任何授权**（要读窗口名字/内容才需要"屏幕录制"）；
- ★ **AX 的 `AXFocusedApplication` 答的是"焦点"不是"层级"**：窗口压上来、焦点没变时它
  毫无反应（作者遇到的正是这个）。而且它要打给别的进程；
- 判据（`src-tauri/src/zorder.rs`）：找到切片器那扇**普通窗**（`ownerPID==切片器` 且
  `layer==0`），看它**前面**有没有「别的 App + `layer==0` + 与它重叠**两成以上**」的。
  ★ 只算 `layer==0`：我们自己的卡片是浮层（`always_on_top` ⇒ layer 3），天然不算"盖住"；
  小浮窗压到角上也不算（`COVERED_ENOUGH = 0.2`）。
  ★ `OnScreenOnly` 顺带答了"最小化 / 隐藏"（列表里根本没有），还把窗口矩形一起带出来。
- ★★ **"贴着 BBS 的层级"字面上做不到**：同一层级里谁在上面由"哪个 App 是活跃 App"决定，
  而卡片必须在 BBS 之上才看得见 ⇒ 只能用浮层。**等效效果 = BBS 被盖住就把卡片收起来。**

**★★★ 切页 / 换界面：窗口一动不动，我们一条信号都收不到**（2026-10-15 作者："我切换到
别的页面，它还是一样显示；但把窗口最大化/改一下尺寸，它就不显示，尺寸改回去又显示"）：

- 现象是**缓存的指纹**：窗口没动 ⇒ 一直命中"平移免 AX"的缓存 ⇒ 从没重新读过 ⇒
  发现不了控件已经没了；改尺寸 ⇒ 绕开缓存 ⇒ 真读一次 ⇒ 收起来；尺寸改回 ⇒ 又命中缓存。
- ★★★ 但**光去掉缓存也不够**：切页时窗口没挪没缩、焦点没变、也没最小化 ⇒
  "变没变"那个键**一样都不变** ⇒ 根本不会去重读。所以加了**定期回看一眼**：
  - `LOCATE_TTL = 2s`：缓存只在 2s 内有效（拖动那几跳照样免 AX = 省；过期就真读 = 准）；
  - 闸门里挂 `attempt_is_fresh()`：**过期就不许早退**，必须真去看一眼 ——
    这是"切页了"唯一的发现途径；
  - ★ `LAST_ATTEMPT`（成没成都记）与 `LAST_LOCATE`（只有成功才记）**分开**：
    控件找不到时缓存永远过期，不分开记就会每 700ms 重扫一遍控件树；
  - ★ 卡片**没开着**时一般不必校验（真该露脸时键一定会变：被压住→回最前 / 最小化→还原）
    —— **但有一个例外**，见下条。

**★★★ 唯一"没有信号"的隐藏理由：控件没了（切页）**（2026-10-15 作者："切换到别的页面，
它确实消失了；但我**切换回来，它就没出现**"）：

隐藏卡片有好几个理由，**只有"控件没了"这一档没有任何信号**：

| 为什么收起来 | 怎么知道它该回来了 |
| --- | --- |
| 被别的 App 盖住 | 层序变了 ⇒ **键变** ⇒ 立刻重算 |
| 最小化 / 隐藏 | 窗口从屏上消失 ⇒ **键变** ⇒ 同上 |
| 切片器退了 | pid 没了 ⇒ **键变** ⇒ 同上 |
| ★ **控件没了（切页）** | **什么都不会变** —— 窗口没挪没缩、焦点没变 |

⇒ 单独记一个 `TARGET_LOST` 标记（`decide` 里"指不了"时置位、指得上时清掉），
闸门因此变成「**卡片开着** 或 **控件找不到**」且「刚刚没看过」才去看一眼。
★ 第一版把"卡片没开着就不校验"一刀切了 —— 于是切回来时**再也没人把它接回来**。
（"收起来是因为被压住/最小化"那几档照旧**不校验**，它们有信号。）

**★★★ 省电版（作者："那个卡片多等一会再跟上也没关系，怎么样节省就怎么样做吧"）**：

- `TICK` 350ms → **700ms**；
- ★★ **每跳只发一次本地调用**：`zorder` 顺手给了窗口矩形 ⇒ "变没变"的键**不用去问切片器**
  （AX 是同步 IPC，问它才是贵的）。**AX 只在 CG 问不到时才读**；
- ★★ **拖动时一个 AX 调用都不发**：控件在窗口里的**相对位置是固定的** ⇒ 缓存
  「(窗口矩形, 控件矩形)」这一对，窗口**只平移**时新坐标 = 旧坐标 + 位移（纯算术）；
  只有**尺寸变了**（拉大缩小 / 换布局）才重新读 AX。★ 这与 `locate` 挑控件的判据
  （按**相对窗口**的 y 落区间）是同一个假设，不会打架。

**★★★ AX 的活绝不许落在主线程上**（2026-10-15 作者："一直转圈、鼠标不变手、
悬停提示也出不来"）：每一次 AX 调用都是**同步 IPC 打到切片器那个进程**去；切片器一忙
（正在切片 / 自绘重绘），一次读就能卡到 AX 的默认消息超时（数秒）。落在主线程上就是
**整屏转圈 + 光标不再更新 + tooltip 出不来**。所以 `ipc::guide::sync` 拆成两段：

- **第一段 `decide()`**：读 AX、拿主意 —— 谁来叫就在谁的线程上跑，**绝不在主线程**；
- **第二段 `apply()`**：`set_position` / 建窗 / 销毁 —— 只有这几句回主线程，里面
  **一次 AX 读都没有**。

配套：AX 回调（它**本来就在主线程上**，observer 的 source 挂在主 run loop）只往一条
短命线程上甩；细定时回读那条线程**就地**跑第一段。★ 状态键里的矩形**取整**再比，
否则子点抖动会让每一跳都判成"变了"、每 350ms 重扫一次控件树。
★ 重挂监听带 **3s 节流**（`AXObserverAddNotification` 同样是打给切片器的同步 IPC）。

**★★★ 拖动跟随不能只靠 AX 通知**（2026-10-15 作者："拖动了它没有跟随"）：
切片器是 **Qt 应用**，实测**不发** `AXWindowMoved` —— 拖动窗口时一条通知都没有
（而最小化 / 还原 / 换焦点那几条它是发的，那几条仍然走通知）。所以：

- **位置这一条**由 `ax_watch` 里一条**细定时回读**兜底（`TICK = 350ms`，只在卡片可能
  露脸的期间跑，点完「知道了」就停）；
- 这次回读**很便宜**：`ipc::guide::sync` 第一步先算一个"状态键"——
  **窗口矩形 + 最前是谁 + 最小化了没有**（`ax::SlicerState`，一次读齐），
  **没变就直接返回**，一次也不重扫控件树。重扫只在"真变了"时发生，也就是用户真动了窗口。
- ★ 所以这不是"频繁的轮询"：它不重扫、不查表、`kill(pid,0)` 探活也不起子进程。

**只有一步**：卡片一次说完，命令只有 `locate_bed_type_control` / `guide_show` / `guide_close`
三条，URL 上不带 `step`。文案（**作者原文，别再自作主张改回去**）：

> **MKP 预设参数已修改**
> 修改后的 MKP 预设参数将在重新切片后生效。请在 Bambu Studio 中重新切片，以生成应用最新参数的 G-code。
> 如果点击「切片」后没有重新切片，或「切片」按钮呈灰色无法点击，请先切换到另一块打印板，再选回原来的打印板，然后重新切片。
> ［知道了］

标题上挂 tooltip 交代机制：`PartPlate::m_slice_result_valid`。
★ 起作用的是"换成不同值"那一下（BBS 源码里那个 `if (old != new)`），换回来只是还原；
分步的代价是"下一步"要**关掉窗口重建**，中间那一瞬卡片没了 —— 比不引导还差。
★ JSX 里**每段要写成一整行**：折行会被折成一个空格，中文标点后面就多出一个空格。

**★★★ 「建不出引导窗口」那颗雷（`close()` vs `destroy()`）**：作者点第二次「查看引导」时
报的就是这句。原因不是窗口建不出来，是**旧的那扇还没死**：

- `close()` 只是**请求**关闭 —— 它往主线程排一条 Close，主线程走到"关窗处理器 → 销毁"
  再排一条 Destroy；
- 我们紧接着拿**同一个 label** 再建一扇，那条 Create 排在 Close 后面、**排在 Destroy 前面**；
- Create 执行时旧窗还在管理器里 ⇒ "a webview with label `guide` already exists"。

⇒ 收旧的改用 **`destroy()`**（直接排 Destroy，与后面的 Create **同一条队列、先进先出**，
旧窗先没、新窗再造）。见 `ipc::guide::destroy_guide`。

**尺寸怎么定的**：`CARD_H` 是**贴着实测排版**给的（Chromium 340 宽下文案要 203），
多留一行防真机 WKWebView 断行差一行 ⇒ 224。透明窗**不穿透点击**，多高一寸就多挡一寸
别人的点击，所以不留富余；但多出来的高度在卡片里表现为一段空白（按钮被 `flex:1` 顶到
底边），不是透明死区。**改文案要回头复核这个数。**

**验收怎么量的**（临时探针，跑完即删）：`omitBackground` 截图 + canvas 读像素 ——
① 卡片四周的留白 ≥ `SHADOW_PAD`（箭头侧 ≥ `ARROW_PAD`）；② **窗口最外一圈的 alpha
必须 ≤ 3**（8bit 最后一级；上一版卡片贴边时这里是几百 ⇒ 一眼看出"被切"）；
③ 箭头那一竖条在卡片以外只剩阴影尾巴；④ 每段 `textContent` 里中文标点后面没有空格。

### 8.5.4 ★★★ 深色模式：卡片跟**用户那一档**，不跟系统（2026-10-15）

**症状**（作者："接下来把它做一个深色模式"）：这张卡片**没有深色** —— 更准确地说，
它只认**系统**深浅，与设置里「外观 → 主题」选的那一档**无关**：选「深色」而
macOS 还是浅色时，卡片是白的。

**根因（一条链，缺的那一环在窗口上）**：
- 卡片的深色原来写的是 `@media (prefers-color-scheme: dark)` —— 那是它当时**唯一
  拿得到的信号**，不是它想那样写；
- 因为**引导窗口不跑 `appState`**（`main.tsx` 里它只挂 `GuideCard`，为的是"慢一拍
  就指不到东西"）⇒ 主窗口那条"读 AppState 的 theme 格 → 落 `<html data-theme>`"
  在那扇窗里**根本不会发生** ⇒ `[data-theme='dark']` 那套 token 它一次也没吃到。

**改法（三处，判据只有一份）**：
1. 新增 `src/app/state/theme.ts` = **主题解析的唯一出处**（`prefersDark` /
   `resolveDark` / `applyTheme`），`appState.ts` 改成从它 import（原来那句解析私有的
   在这个模块里，引导窗口拖不起它）；
2. `main.tsx` 引导分支：`await api.getTheme()` → `applyTheme(pref)`（读不到就 `null`
   = 跟随系统）→ **再**挂卡片；
3. `GuideCard.module.css`：`@media (prefers-color-scheme: dark)` → **`[data-theme='dark']`**
   （与全站同一个判据），并补 `.err` 深色（浅色那支 `#d70015` 压深底几乎读不出）。

★ **别再用 `@media (prefers-color-scheme: dark)` 当深色判据**：它答的是"系统此刻
是什么"，不是"用户选的是哪一档"。全站判据只有一个 —— `<html data-theme>`。

**验收**（临时探针，跑完即删）：**故意让两者相反**，量卡片**实际渲染的像素**：

| 系统 | 档位 | `<html data-theme>` | 卡片背景 |
|---|---|---|---|
| 浅 | `dark` | dark | `rgb(44,44,46)` ← 原来这里是白 |
| 深 | `light` | light | `rgb(255,255,255)` |
| 深 / 浅 | `system` / `null` | 跟系统 | 跟系统 |

八组全绿；两档截图落 `tmp-shots/guide-dark-*.png`。

★ 顺带把事实核了一遍（别再重做）：**客户端主窗口的深色早就做完了**（`tokens.css`
两套值 + `applyTheme` + Rust 的 `theme` 格与单测 + `bridge.ts` + 设置页「外观」那一档，
A06/A07 两刀还收过各页覆盖）。**唯一没有深色的是工作台**（`src/workbench/`：独立入口
+ 自己两份只有浅色的 token）。

### 8.6 ★★ Windows 落地：**能做，而且比 macOS 还省**（2026-10-19 实测）

作者：「接下来做这一个，看看 Windows 的能不能做」。结论：**能**——窗口层三件事
（定位 / 层序 / 最小化）Win32 全有公开只读 API，**不需要任何授权**（macOS 的
辅助功能那一关在 Windows 上不存在）；「板子类型」控件**也找得到**，而且比 macOS
那边干净：**它是真实子窗口，`EnumChildWindows` 直接枚举到，连 UIA / COM 都不用**。

实测环境：BBS `02.08.02.60` @ Windows，**125% 缩放**（DPI 120），主窗 class
**`wxWindowNR`**（wxWidgets），标题 `* <工程名> - BambuStudio`。

| 能力 | 结果 | 证据 |
| --- | --- | --- |
| 按 pid 找主窗 | ✅ | `EnumWindows` + `GetWindowThreadProcessId`（探针 `scripts/probes/uia-probe.ps1`） |
| 读窗口矩形 / 可见 / 最小化 | ✅ | `GetWindowRect` / `IsWindowVisible` / `IsIconic`（★ 本进程要先 `SetProcessDPIAware`，否则坐标被虚拟化） |
| 找到「板子类型」缩略图 | ✅ | 两个 **60×60**（物理）的子窗口，class **`Static`**，相对窗口 y = **150 / 154**（`scripts/probes/childwin-probe.ps1`） |
| 读子窗屏幕坐标 | ✅ | `GetWindowRect` 对子窗直接给屏幕坐标 |
| 层序 / 被谁盖住 | ✅ | `EnumWindows` 本来就按 **z 序从前往后**返回，判据与 §8.5 的 CG 版逐条对应（见 `zorder.rs` 的 `mod win` 头注释） |
| 抬窗 | ⚠️ 有限制 | `SetForegroundWindow` 有前台限制，失败时照实 warn、不硬来（与 macOS「抬不动就静静过去」同口径） |

**判据换算（别用 macOS 那组数）**：DPI 120 ÷ 96 = 1.25，物理 60×60 / y 150~154
折算成逻辑 = **48×48 / y 120~123** —— 与 macOS（50×50 / y 115~117）同族但数值不同。
⇒ Windows 的挑选判据单独一套（`ax.rs` 的 `mod win`）：**class `Static` + 正方形 +
边长 48±6 + y ∈ [100,145]，命中取 x 较大**。多出的 class 过滤是必须的：窗口里还有
class `wxWindowNR` 的方形子窗口（50×50 @ y=510、63×63 @ y=374），不过滤必指错。

**踩到的坑（实机验证抓出来的，下次别再踩）**：
1. ★★ **同 pid 下有别的"有标题"的顶层窗**：实测一扇 **20×20** 的小窗在 z 序里排在
   主框前面 —— 主窗挑选不能"z 序第一个有标题的就当"，要**可见 + class `wxWindowNR`
   + 面积最大**（见 `ax.rs` `win::main_window_of` 的注释）。
2. ★ **DPI 感知是进程级的**：给用户的 exe 靠 tauri 清单没问题；cargo test 那种裸进程
   默认不感知，`GetWindowRect` 会给虚拟化坐标 ⇒ `ax.rs` / `zorder.rs` 的入口都有
   `SetProcessDPIAware` 的 `Once` 兜底。
3. **多开**：BBS 会起多个同名进程（实测三份）——认进程要挑"真有主窗"的那个；
   层序判定里**别的实例（另一个 pid）照算"盖住"**，那是真实处境。

**实现落点**（与 macOS 侧逐层对称，前端零改动）：
- `ax.rs` `mod win`：`slicer_pid`（Toolhelp 快照按进程名 `bambu-studio.exe` 认）、
  `locate` / `state` / `raise`；
- `zorder.rs` `mod win`：`EnumWindows` 的 z 序算遮挡比（同一个 `COVERED_ENOUGH`，
  提成了 `zorder::COVERED_ENOUGH` 公共常量）；
- `win_watch.rs`（新）：跟随信号 = **700ms 一次本地回读**，没有 `SetWinEventHook` ——
  `decide` 的闸门本来每跳只比一个本地键（`EnumWindows` 全在本地，不打给切片器进程），
  换推送只快几百毫秒，不值多养一条消息泵线程（省电口径是作者 2026-10-15 定的）；
- `ipc/guide.rs`：四处 `cfg` 分派放开 Windows；
- 实机验证：`cargo test -- --ignored`（`ax::win::tests::live_*`，本机跑着 BBS 才有意义）。

**还没在真界面里点过一遍**（Rust 侧单测与实机探针都绿了，但「查看引导 → 卡片贴着
板型控件」这条端到端要开着 tauri 应用人工看一眼 —— 透明窗口在 Windows 上的表现
（阴影 / 点击穿透）是 §8.5.3 那四层坑的 Windows 版，没有实测过）。

### 8.7 ★★ 认"我刚才切的那一份"：记父进程（2026-10-20 作者定）

**问题**：BBS 会多开。改完参数要引导用户回**我刚才切的那一份**去重新切片，
而"窗口排在最前那份"会挑错（切着 A 份、手上点着 B 份 / 几份最大化叠在一起）。

**答案：不用猜 —— 钩子是被那一趟切片的 BBS 拉起来的。**

| 环节 | 落点 |
| --- | --- |
| 拿父进程号 | `src-tauri/src/parent.rs`（Windows：Toolhelp 快照读 `th32ParentProcessID`；macOS/Linux：`getppid()`；1 号不算 —— 那是被界面启动的） |
| 记进记录 | `archive::Meta::slicer_pid`（**只加不改**：旧记录没有 ⇒ `null` ⇒ 退回老逻辑；成功 / 失败 / 取消都记） |
| 读回来 | `archive::latest_slicer_pid(root, accept)`：日期目录倒序 + 同名时刻倒序，**只认 `accept` 认下的号**（进程号会失效 ⇒ 最近那条死了就落到上一条还活着的）；最多翻 30 条 |
| 引导层用它 | `ax::slicer_pid()` = **记住的那份**（校验它这会儿还是切片器）→ 否则 z 序最前那份；`locate` / `raise` / 回读**都走同一个** `slicer_pid()`（认不同一份 = 摆出去指 A、下一跳盯 B，卡片照样被收） |
| 省 | 扫记录要读盘 ⇒ 缓存 30s（回读是 700ms 一跳，不能每跳扫） |

★ **为什么"记住的那份"优先于"最前那份"**：用户要的是"我刚切的那盘"；
只有当那份已经不在了（切片器重启 / 关掉）才该退到眼前那份。

**§8 里 P1（自动点击切板）作者 2026-10-20 明确不做** —— 维持"只指不代劳"：
点错等于真的替用户改了板子，位置靠控件识别、BBS 一升级就废，还要抢前台。

## 9. 待办 / 未查证

- `Plater::update(flags)` 带 `FORCE_BACKGROUND_PROCESSING_UPDATE` 的触发源未逐一排查
  （若存在某个"无害"的 UI 动作带这个标志，理论上可直接解开门② —— 目前没找到）。
- `Print::invalidate_all_steps()` 一族未逐行核对（按 §1/§2 它不直接参与这两道门）。
- P1 的可行性实验**没跑**（会弹 macOS 辅助功能授权框，需作者在场）。
