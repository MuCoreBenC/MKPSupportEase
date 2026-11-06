# 本地官方源测试服务

**一句话**：把"官方云端源"临时换成本机 HTTP 服务，客户端**逻辑一套不变** ——
还是走 catalog / manifest / 寻址规则 / SHA 校验 / 下载管道，换的只是"去哪儿取"。

它解决的痛点：**「我只是想测更新功能，为什么每次都要真的发布到云端？」**

---

## 起

```bash
npm run preset-source:dev        # 只起服务（端仓库里的 presets/）
npm run dev:test-update          # 服务 + tauri dev（源自动指过来）
```

不跑 `dev:test-update` 也可以：起服务后把 `http://127.0.0.1:8787/delivery` 填进
**设置 → 高级设置 → 预设数据源 → 自定义地址**（保存即生效，不用重启）。

也可以完全不碰终端：**工作台「设置 → 本地测试源（开发）」**那颗按钮跑的就是
`npm run preset-source:dev`（起 / 停 / 看状态）。它**只起服务**，不碰你的客户端 ——
地址自己填进客户端，日志打在起工作台的那个终端里。

★ `MKPSE_PRESET_SOURCE_URL` **只在 debug 构建里认**（`runtime/source.rs` 的
`debug_source_override`，`#[cfg(debug_assertions)]`）。release 里这段代码整个不存在，
`scripts/check-release-source.mjs` 与"启动零网络"两条判据都不受影响。

---

## 端的是哪一份（2026-10-30 起只有一项）

| 端点 | `--root` / 服务根 | 客户端填的地址 |
| --- | --- | --- |
| **当前交付** | **预设根**（`presets/`，测试模式开着 = 沙箱那份） | `http://127.0.0.1:8787/delivery` |

★ 端的是**预设根**、地址要带 `/delivery` —— 因为真交付的 `source.json` 写着
`filesRoot: ".."`，而 `catalog.path` 本身带 `delivery/` 段（相对预设根）。
地址那一串由后端按这个规矩给（界面照抄，不用记事）。手敲命令时也一样：
`PRESET_TEST_ROOT=<repo>/presets npm run preset-source:dev`。

★ 端哪一份由 `PRESET_TEST_ROOT` 交下去（工作台那颗按钮换的就是它）：
`--root` 参数 → `PRESET_TEST_ROOT` → 默认 `<repo>/presets`，先认参数。

★ 8787 上已经有东西时（上一轮留下的服务、你手动起的一份），它会**起之前先探端口**，
把占用者（谁、PID）摆出来并给一颗「停掉它」；停掉之后「启动」才解禁。

### 发布序号自动 +1

`PRESET_TEST_BUMP_SERIAL=1`（或 `--bump-serial`，工作台按钮默认带）时，返回的
`catalog.json` 里 `publishSerial` 往前拨一格、`publishedAt` 换成此刻 ——
**只改响应，盘上一个字节不动**。

为什么要拨：客户端手上那份目录和这份源同号、内容却不同 ⇒ 逐文件判据给
`Inconsistent`（"那一代目录不再唯一"），看着像坏了。拨一格就成了正常的
"云端发了下一版"，和真发布一模一样 —— **判据一个字都没放宽**，只是这份模拟源
自称"刚发过一版"。

★ 所以：**改完东西 → 重新生成一次 → 客户端点「检查更新」**，就能看见新版。
不用发布、不用提交、不用派生任何副本。

---

## 本地发布：测「软件更新」也不发版（2026-11-06）

上面那条链只演**预设数据**。演**软件自己升级**（下载 → 校验 → 解包 → 替换 → 重启）
以前只有一条路：真发一版 —— 打不可退的 tag、传包、写清单、等镜像，二十分钟起步。

现在：

```bash
npm run tauri:build                   # 编一份真的 .app（已编过就跳过）
npm run local-release                 # 把最新的 .app.zip 变成"一份本地发布"
npm run preset-source:dev -- --local-release
```

`local-release` 做三件事（全部落在 `target/local-release/`，git 看不见）：

1. 把 `target/release/bundle/macos/*.app.zip`（没有就从 `.app` / `.dmg` 里现打）拷过去；
2. 算 size 与 SHA-256；
3. 写 `release-preview.json`，`asset.url` 指向 `http://127.0.0.1:8787/delivery/<那支 zip>`。

服务端带 `--local-release` 时对 `/delivery/<名字>` **先查这份目录**，查到就顶掉仓库里
那一份 —— **只改响应，`presets/delivery/` 一个字节不动**。

★ **版本号默认取"包里那个版本的下一个预览号"**（`0.0.10-preview.2` → `preview.3`）。
不这么干的话界面上一张卡都不会出现：客户端只摆**比当前构建新**的候选，而
`APP_VERSION` 是编进二进制的，包里那个号与当前构建一模一样。
包里真正的版本仍是 `preview.2`，装完重启后界面也还是它 —— **这是本地测试的真相**，
要"装完显示新号"只能真编一版新号。想手写号：`--version 0.0.11-preview.1`。

★ 想演正式通道：`--channel stable` 会写 `release.json`（那份会**主动通知**，
只测安装链的话用缺省的 preview）。

★ `npm run dev:test-update` 也会自动带上这份覆盖（它本来就是"测更新"那条命令）。

---

## 和「夹具」的区别（那套已经删了）

以前有 `fixtures/v1`、`fixtures/v2` 两代派生产物（`make-fixtures.mjs` 从真交付根算），
作用是演"云端发了下一版"。2026-10-30 作者裁决：**不需要** ——
端真产物 + 序号 +1 更真，还不会和源漂移、也没有第二套目录要同步。

---

## 想测的几条链（都要重启一次或点「检查更新」）

```text
① 本地已是最新        → 云端表那几行是「已下载」，没有「新版本」
② 改点东西再生成      → 云端表出现「新版本」（revision / serial 前进）
③ 点「下载」          → 落一份新的「我的预设」，旧的那份一个字节不动
④ 参数页「恢复默认值」→ 以**这份预设对应的官方基准**为准（不是出厂值）
⑤ 两份预设拉进「参数对比」→ 同一项并排看，改哪一份都行
⑥ 设置页改地址并保存  → 立刻生效，不用重启
```

---

## 边界

- **不做浏览器 / Tauri 两套 mock**：客户端只有一套逻辑，测试源只是"另一个地址"。
  浏览器预览那套假后端（`src/api/mock.ts`）是另一件事（它没有文件系统）。
- **不进依赖树**：只用 `node:http` / `node:fs`。
- **只端根底下的文件**（防穿越），并带 `cache-control: no-store`（改了立刻见效）。
- **别把正式源地址换成本地**：`MKPSE_PRESET_SOURCE_URL` 只在 debug 构建里读，
  但手填进设置页的那个自定义地址 release 也认 —— 那是给开发/排查用的后门，
  填错了在设置页点内置源就能切回去。
