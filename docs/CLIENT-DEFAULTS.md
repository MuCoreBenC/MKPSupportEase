# 客户端偏好的出厂值

## 1. 只此一份

`presets/client-defaults.toml`。两段，三处消费者都 `include_str!` 它：

| 段 | 内容 | 消费者 | 用它做什么 |
|---|---|---|---|
| `[Prefs]` | 决定切出来 G-code 的 11 项 | 内核 `crates/postprocess/src/ir/types.rs` 的 `PrefsIr::default()` | 没被预设覆盖的那几格 |
| `[Prefs]` | 同上 | 客户端 `src-tauri/src/runtime/app_state.rs` 的 `PostprocPrefs::resolved()` | 设置页没填过的项显示什么 |
| `[Client]` | 界面偏好（主题 / 语言 / 切片器 / 引导 / 通知 / 显示方式 / 置顶 / 强制校准 / 预设页两项 / 自动清理 / 自动检查更新） | 客户端 `src-tauri/src/runtime/client_defaults.rs` | 盘上那一格是 `None` 时解成什么 |

改默认值 = 改这一个文件。**别在代码里再写字面量**：`postproc_prefs` 那条
`the_two_default_sets_agree`、`client_defaults` 的
`shipped_defaults_match_the_old_hardcoded_ones` 咬的就是「各处不许各写一套」。

## 2. 生效时机

它是**编译期**进二进制的：

- 改完要**重新构建客户端**才进包；
- 已经装好的客户端**不受影响** —— 默认值只决定「没有偏好文件时初值是多少」；
- 老用户想套用新默认值，在设置里点一次「恢复默认」（把 `postproc` 那 14 项整格写回 `null`）。
  主题 / 语言 / 切片器 / 引导 / 预设页那几项**没有**一键还原入口，改的是各自那一格。

工作台「默认配置」页顶上那句写的就是这件事，别删。

## 3. 哪些项能配

**两段都是可配的**，键表见 `src-tauri/src/workbench/app/defaults.rs` 的 `EDITABLE`：

- `[Prefs]` 10 项：涂胶、校准停留、校准抬升、收笔、塔位；
- `[Client]` 12 项：主题、界面语言、默认打开模型的软件、重新切片提示、完成通知、
  显示方式、等回答时置顶、强制校准模式、副本跟随云端、使用后复制脚本、自动清理执行记录、自动检查更新。

客户端里那些**没有偏好**的入口不在这份文件里，工作台也不摆（它们不是"能配的默认值"）：
备份与恢复 / 最近删除 / 导出日志 / 打开预设位置 / 更新渠道（不落盘）。
界面密度也没有 —— 它是按窗口宽度算的（`src/hooks/useDensity.ts`）。

调试动作那组（`debug_*`）同样不在这里：它有自己那颗「恢复默认」，住
`src/app/settings/DebugPrefsModal.tsx`。

## 4. 前端那两格怎么拿到出厂值

切片器（`mkp.A40.slicer`）与「保存后的重新切片提示」（`mkp.A40.guide.show`）住在 webview 的
localStorage，壳读不到。所以 `main.tsx` 挂 React **之前**问一次 `get_client_defaults`
（命令在 `src-tauri/src/ipc/prefs.rs`），把 `[Client]` 段那两格放进
`src/app/state/clientDefaults.ts`；`useSlicerPref` / `useGuidesPref` 在"没设过"时用它。
前端不另写一份默认值。

## 5. 与「出厂值只属于工作台」（AGENTS §1.6）的边界

§1.6 禁的是**预设参数的 `default` / `baseValue`** 被客户端接收与展示 ——
那几格是工作台算参数时的口径，可能是占位零。

这一份是**客户端偏好的初值**，不是预设参数、不进参数表、不参与 `presets generate`
的三层生成物。两者不冲突：前者归参数注册表，后者归这个文件。

## 6. 怎么改

两条路，选一条：

1. 工作台「默认配置」页 —— 写完进影子树，右上角「保存改动」才落 `presets/`；
   撤销 / 重做与其它页同一套栈。
2. 直接改 `presets/client-defaults.toml` —— 改完重编。

两条路都改同一份文件，不存在第二处要同步。**缺一个键就是读不出来**（不兜底）：
工作台那一页会报出是哪个键，客户端第一次用到时当场炸在启动日志里。
