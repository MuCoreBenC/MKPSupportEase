# 本地官方源测试服务

**一句话**：把"官方云端源"临时换成本机 HTTP 服务，客户端**逻辑一套不变** ——
还是走 catalog / manifest / 寻址规则 / SHA 校验 / 下载管道，换的只是"去哪儿取"。

它解决的痛点：**「我只是想测更新功能，为什么每次都要真的发布到云端？」**

---

## 起

```bash
npm run dev:test-update          # 夹具 v1 + 服务 8787 + tauri dev（源指向它）
npm run preset-source:dev        # 只起服务（默认 fixtures/v1）
npm run preset-source:make       # 从真交付根派生 v1 / v2 两代夹具
npm run preset-source:sync       # 改过夹具里的 TOML 之后重算 sha256 / size / revision
```

`dev:test-update` 做了三件事：夹具不在就先派生 → 起服务并等就绪 → 用
`MKPSE_PRESET_SOURCE_URL=http://127.0.0.1:8787` 启动 `tauri dev`。

★ 那个环境变量**只在 debug 构建里认**（`runtime/source.rs` 的 `debug_source_override`，
`#[cfg(debug_assertions)]`）。release 里这段代码整个不存在，
`scripts/check-release-source.mjs` 与"启动零网络"两条判据都不受影响。

不跑 `dev:test-update` 也可以：`npm run preset-source:dev`，再把
`http://127.0.0.1:8787` 填进**设置 → 高级设置 → 预设数据源 → 自定义地址**
（保存即生效，不用重启）。

也可以完全不碰终端：**工作台「设置 → 本地测试源（开发）」**那颗按钮跑的就是
`npm run preset-source:dev`（起 / 停 / 看状态）。它**只起服务**，不碰你的客户端 ——
地址自己填进客户端「设置 → 高级设置 → 预设数据源 → 自定义地址」（保存即生效，不用重启）。
日志打在起工作台的那个终端里 —— 它只是替你敲了这条命令，没有第二套日志窗口。

那颗按钮上有一组**三选一**（换源 = 换源重启，一次点击一件事）：

| 端哪份 | `--root` / 服务根 | 客户端填的地址 |
| --- | --- | --- |
| 夹具 v1 | `fixtures/v1`（默认） | `http://127.0.0.1:8787` |
| 夹具 v2 | `fixtures/v2` | `http://127.0.0.1:8787` |
| **当前交付** | **预设根**（`presets/`） | `http://127.0.0.1:8787/delivery` |

★ 第三项端的是**预设根**、地址要带 `/delivery` —— 因为真交付的 `source.json` 写着
`filesRoot: ".."`，而 `catalog.path` 本身带 `delivery/` 段（相对预设根）。
地址那一串由后端按这个规矩给（界面照抄，不用记事）。手敲命令时也一样：
`PRESET_TEST_ROOT=<repo>/presets npm run preset-source:dev`，地址填 `<地址>/delivery`。

★ 端哪一份由 `PRESET_TEST_ROOT` 交下去（工作台那颗按钮换的就是它）：
`--root` 参数 → `PRESET_TEST_ROOT` → 默认 `fixtures/v1`，先认参数。

★ 8787 上已经有东西时（上一轮留下的服务、你手动起的一份），它会**起之前先探端口**，
把占用者（谁、PID）摆出来并给一颗「停掉它」；停掉之后「启动」才解禁。

★ 想连客户端的 `tauri dev` **一起**起（它会自动把源指过来、不用手填地址）：那是上面
那条 `npm run dev:test-update` —— 它靠 `MKPSE_PRESET_SOURCE_URL` 注入，所以必须把
客户端 dev 一起管起来；工作台那颗按钮不干那件事（作者 2026-10-08 裁决：
「这不就是单开一个服务吗？我自己输入这个地址就可以」）。

---

## 夹具里有什么

`fixtures/v1` 与 `fixtures/v2` **是派生产物**（`make-fixtures.mjs` 从
`presets/delivery/` 算出来，`fixtures/.gitignore` 挡住它们）：手写两份 `catalog.json`
必然与真目录漂移，而漂了之后这个源就不是"真形状"，测出来的结论也不作数。

两代的差别只有两处（够演示"官方发新版了"）：

| | `# release_time` | `A1-fast.toml` 的 `offset_y` |
| --- | --- | --- |
| `v1` | 2026-10-08 | 26.3 |
| `v2` | 2026-10-15 | 26.8 |

`source.json` 改成 `filesRoot: "."`（夹具自己就是一个交付根），
`files` 只留两份 MKP 预设 —— `structureSignature` / `minClientVersion` /
机型 / 注册表**一律照抄真目录**，所以"这一代读不读得懂"的判定不会平白翻脸。

---

## 想测的几条链（都要重启一次或点「检查更新」）

```text
① 本地只有 v1            → 云端表那几行是「已下载」，没有「新版本」
② 把源换成 v2            → 云端表出现「新版本」（revision 变了）
③ 点「下载」             → 落一份新的「我的预设」（名字带 2026-10-15），
                           旧的那份一个字节不动；baseline 里多一份
④ 参数页「恢复默认值」   → 以**这份预设对应的官方基准**为准（不是出厂值）
⑤ 把两份我的预设拉进「参数对比」 → 同一项并排看，改哪一份都行
⑥ 设置页改地址并保存     → 立刻生效，不用重启
```

> 换第三版：改 `fixtures/vX/mkp/presets/*.toml`（发布日 + 参数值），
> `npm run preset-source:sync -- --root fixtures/vX --at 2026-10-22T00:00:00Z`，
> 重启。`revision` 一变，客户端"检查更新"就看得见。

---

## 边界

- **不做浏览器 / Tauri 两套 mock**：客户端只有一套逻辑，测试源只是"另一个地址"。
  浏览器预览那套假后端（`src/api/mock.ts`）是另一件事（它没有文件系统）。
- **不进依赖树**：只用 `node:http` / `node:fs` / `node:crypto`。
- **只端夹具根底下的文件**（防穿越），并带 `cache-control: no-store`（改了立刻见效）。
- **别把正式源地址换成本地**：`MKPSE_PRESET_SOURCE_URL` 只在 debug 构建里读，
  但手填进设置页的那个自定义地址 release 也认 —— 那是给开发/排查用的后门，
  填错了在设置页点内置源就能切回去。
