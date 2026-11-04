# 资产怎么来 · 以后加新机型怎么生成

> 这份文档回答一个问题：**界面上的那些图（引导图 / 机型图 / 品牌 logo）是谁生成的、放在哪、以后加了新机型要怎么补。**
> 只写怎么做；为什么这样分（三层生成物）见 `AGENTS.md §1` / `§3.5`。

## 1. 两类图，别混

| 类别 | 是什么 | 在哪 | 谁给它登记 |
|---|---|---|---|
| **「打开方式」引导图** | 教用户在系统里怎么把本程序设成 `.3mf` 的打开方式（浅 / 深两版） | `presets/assets/guides/` | `presets/assets.toml` |
| **机型图** | 每个机型一张外观图（卡片 / 详情用） | `presets/assets/printers/` | `presets/assets.toml` |
| **品牌 logo** | 厂商标识 | `presets/assets/brands/` | `presets/assets.toml` |

工作台标注台那套（PS 式源稿 + 渲染成品）也在 `presets/assets/` 下，属于**引导图/标注**这一类，
源稿与成品一起入库。

## 2. 目录长什么样

```text
presets/assets/                  ← 唯一源（入库）
├── guides/
│   ├── open-with-light.rendered.webp     ← 成品（界面实际用这个）
│   ├── open-with-dark.rendered.webp
│   ├── open-with.anno.json               ← 标注源稿（工作台标注台读写）
│   └── source/                           ← 原图
├── printers/
│   └── a1.webp / p1s.webp / …
└── brands/
    └── bambu-lab-logo.svg
presets/assets.toml              ← 台账：path / type / delivery
client-assets/                   ← 生成物（**不入库**，删了会自动重建）
```

`client-assets/` 里那份 `GENERATED.md` 是给读的人看的，程序只读 `manifest.json` 与 `assets/**`。

## 3. 怎么生成（不需要记命令）

同步**只由 vite 插件触发**（`tools/assets/plugin.mjs` → `tools/assets/sync.mjs`）：

```text
presets/assets.toml (delivery 的唯一输入) + presets/assets/** (文件唯一源)
        │
        │  npm run build / build:workbench / npx vite build 都会先经过
        ▼
client-assets/assets/<与 URL 同形的树>
```

对账语义是「目标文件本身就是状态」：期望有、目标没有 ⇒ 写入；两边 SHA 不同 ⇒ 覆盖；
SHA 相同 ⇒ skip；目标有、期望没有 ⇒ **删除**。所以**永远不用手工删 `client-assets/`**，
把台账改对再构建一次就同步了。

## 4. 加一个新机型要做什么

1. 把机型图放进 `presets/assets/printers/<机型>.webp`（推荐 webp，小；与现有图同尺寸比例）。
2. 在 `presets/assets.toml` 里加一行登记：

   ```toml
   [entries."printers/<机型>.webp"]
   path = "printers/<机型>.webp"
   type = "printer"
   delivery = "bundled"
   ```

3. 跑一次构建（`npm run build`），`client-assets/` 会自己多出这一份。
4. 跑 `npm run check:artifacts` —— 三层生成物的闸。

**只改 `client-assets/` 是白改**：它是产物，下一次构建会被台账覆盖回去。

## 5. 以后要换引导图

改 `presets/assets/guides/` 里的**成品**（`.rendered.webp`）；源稿 `source/*.png` 与
`open-with.anno.json` 一起留着，标注台还能再编辑。台账里的 path 不用动 —— 名字没变，
对账会按 SHA 判定「改过 ⇒ 覆盖」。

## 6. 判据在哪

| 守什么 | 判据 |
|---|---|
| 台账与目录一份不差、文件字节与台账一致 | `crates/preset/tests/` 里 `builtin_presets_match_dir` 同族的资产断言 + `npm run check:artifacts` |
| 三层生成物不许「顺手带上」 | `scripts/check-artifact-boundary.mjs`（`npm run check:artifacts`） |
| 客户端能解析内置预设（含 `mkp_source` 语义） | `builtin_presets_are_readable_by_the_client_without_legacy_header` |
