# 第五刀交接：应用内更新（2026-10-05）

> 作者 2026-10-05 拍板：**"以后下载要像 Trae / WorkBuddy 那样"** —— 标题栏常驻小图标 +
> 环形进度、点开是详情面板（下载 / 暂停 / 取消）、下载完出现「重启并安装」，点了就重启。
> 这一刀把它做出来，顺带修掉两个真机问题。
>
> **一次提完**（作者 2026-10-05 追加纪律：不再按"又发现一个问题"切一刀，见 §6）。

---

## 1. 这一刀做完的事

| # | 事 | 在哪 |
|---|---|---|
| 1 | **`release.json` 加可选 `asset` 格**（安装包名 / 地址 / 大小 / sha256） | `src-tauri/src/runtime/release_info.rs` |
| 2 | **更新内核**：流式下载 + 进度 + 暂停 / 继续 / 取消 + 验大小 / 验 SHA + 解压 | `src-tauri/src/runtime/updater.rs`（新） |
| 3 | **装上去并重启**：退出本进程 → 后台脚本替换 `.app` → 重新拉起 → **下次启动读账** | 同上 + `src-tauri/src/ipc/update.rs`（新） |
| 4 | **六个命令**：`update_info` / `start_update` / `pause_update` / `resume_update` / `cancel_update` / `install_update` | `ipc/update.rs` |
| 5 | **修 bug：「查看更新」点了没反应** → 走 `open_url` 命令（webview 没 opener 权限，`<a>` 必然打不开） | `ipc/update.rs` + `PageSettings.tsx` |
| 6 | **标题栏常驻指示器**（环形进度 + 详情面板 + 「重启并安装」） | `src/app/components/UpdateIndicator.tsx`（新） |
| 7 | **mock 夹具能触发整套界面**（**不用真发一版**就能验） | `src/api/mock.ts` |
| 8 | **工作台发版自动打 `.app.zip` 并上传**，写进 `release.json` 的 `asset` | `workbench/app/release_tx.rs` |

**没有引入任何新依赖**：打包 `ditto`、解压 `unzip` 都是 macOS 自带。

---

## 2. 状态机（界面只有这一种形状要认）

```text
Idle ──start──→ Downloading ⇄ Paused
                    │
                 Ready ──install──→ 退出 → 替换 → 重启 → 下次启动读 update-result.json
                    │
                 Failed / Cancelled
```

- **进度是事件**（`software-update-progress`，每 120ms 一次）；
- **状态是快照**（`updateInfo` 一次问全：状态 + 有无新版 + 资产 + 上次安装结果）；
  事件与快照**是同一个 `UpdateState` 类型** —— 界面不会有两套渲染。

---

## 3. 两个"退出进程之后"的事实（本进程看不见，只能下次说）

| 事实 | 怎么办 |
|---|---|
| **正在运行的 `.app` 换不掉**（代码段在执行） | 先 spawn 后台脚本（`sleep 1`）→ 再 `app.exit(0)` |
| **替换成没成，本进程已经退出了，看不见** | 脚本把结果写 `<appData>/run/update-result.json`，**下次启动**由 `updateInfo` 读出来说清 |

判据 `update_result_is_read_on_next_launch` 钉住"账读得回来"，坏档报 `CORRUPTED` 不静默当"没装过"。

---

## 4. 三个"没有它就不成立"的前置条件

1. **这一版必须带 `asset`** —— 发布方先具备，客户端才会用；没有就退回"打开下载页"
   （0.0.2 / 0.0.3 的行为仍然成立，`a_release_without_an_asset_still_parses` 钉住）。
2. **必须是 `.app.zip`** —— 解压即换，不用挂载 dmg、不用管理员密码。
3. **必须是 macOS** —— 替换 `/Applications/…` 与 `open` 重新拉起都是 macOS 形状。

---

## 5. 怎么自己触发这套界面（**不用真发一版**）

```bash
npm run dev            # 浏览器里加 ?mock=1
```

- `src/api/mock.ts` 的 `MOCK_NEW_VERSION`（当前 `'0.0.2'`）决定"有新版本"还是"已是最新"：
  **改成与 `MOCK_APP_VERSION` 相同就切到「已是最新版本」**；
- mock 的下载是**假进度**（180ms 一档、24 档走完），暂停 / 继续 / 取消 / 「重启并安装」全都点得到；
- 想验"点了没反应"那个 bug 是否修好：mock 下 `openUrl` 只记不跳（真机才开浏览器）。

真机验：装上带这一刀的版本 → 设置页「在应用内下载」→ 标题栏出现环 → 点开面板能暂停 → 下完出现「重启并安装」。

---

## 6. 这一刀顺带定的纪律（作者 2026-10-05）

> **一轮只提一次代码 PR**：手上攒的改动全部一起提，不按"又发现一个问题"切一刀。
> **发版固定 +2 个 PR**（版本号 + `release.json`），中间不插别的。
> 端到端验收放在**发版之后**做，不在发版前插验证轮次。

违反它就会看到 #32 / #33 那种碎片 —— 那次是两个真机问题各切了一刀。

---

## 7. 还没做的（明确的欠账，别替我"顺手做了"）

| 欠账 | 为什么现在不做 |
|---|---|
| **Windows / Linux 的应用内更新** | 第一阶段只发 macOS；替换路径与重新拉起都是 macOS 形状 |
| **断点续传 / 多线程下载** | 几十 MB 单流够用；真要慢再说 |
| **安装包签名与公证** | 作者 2026-10-04 裁定暂不做；Gatekeeper 警告写进 Release 正文 |
| **Gitee 上建 Release / 传附件** | 作者 2026-10-05 裁定"发布链只走 GitHub，Gitee 自己 `git push` 镜像"；代码已具备（PR #32），但**发版时选 gitee 才会走** |
| **国内源默认切 Gitee** | 要等 Gitee 上的 `release.json` 跟到最新一版；没跟之前切过去 = 把用户往"查不到更新"的地方带 |

---

## 8. 判据（默认 338 / workbench 624）

新增：

- `runtime::updater`：`percent_is_honest_about_unknown_totals`（总量未知给 0，**不给 100**）/
  `size_mismatch_is_refused` / `sha256_is_checked_only_when_given` /
  `the_unpacked_app_is_the_only_one`（多了不许自己挑）/
  `unpack_is_checked_by_exit_code`（★ 只认退出码，不解析 zip 结构）/
  `update_result_is_read_on_next_launch` / `the_install_script_replaces_and_reopens_and_records`；
- `runtime::release_info`：`a_release_without_an_asset_still_parses` /
  `a_downloadable_asset_is_offered_to_the_client` / `an_unusable_asset_url_is_refused`（`file:` 拒）。

**写盘纪律的第二处逃生口**：`updater::download_into` 里的 `File::create` ——
安装包几十 MB，`fsx::atomic_write` 是"整个字节数组进内存"，流式写不能走它。
代码里写了理由与**退役条件**（一旦下载改成"先落 .part 再 atomic rename"就换掉）。

---

## 9. 发版时这一刀会多做什么

`release_tx` 在上传 dmg 之后多两步（**失败不挡发版**）：

```text
构建 dmg → 上传 dmg
       → ditto -c -k --sequesterRsrc --keepParent SupportEase.app SupportEase_<版本>_<arch>.app.zip
       → 上传 zip → release.json 里写 asset{name,url,size,sha256}
```

打不出 zip / 传不上去 ⇒ **如实说"这一版退回「打开下载页」"**，`release.json` 里没有 `asset`，
客户端照旧退回那条路（比整个发版失败好）。
