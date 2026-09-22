# 资产瘦身（asset-slimdown）—— 完成小结

Task 1~5 全部完成并验证过。**Task 6（提交 + 重写历史）没动**，它不可逆，等你确认。

---

## 一、结果

`public/` 从 **1171.9 KB / 18 个文件** 降到 **366 KB / 6 个文件**，降幅 **69%**。

| | 之前 | 现在 |
|---|---|---|
| `public/` 合计 | 1171.9 KB | 366 KB |
| 文件数 | 18 | 6 |
| 前端依赖 | 含 `three` + `@types/three` | 去掉，`npm install` 少装 8 个包 |
| `public/` 根目录 | 4 个裸放的图 | 0 个（全进 `printers/bambu/` 与 `models/`） |
| 位图 PNG | 9 个 | 0 个（`src-tauri/icons/` 除外，那是 Tauri 打包要求的格式） |

留下的 6 个：

```
public/models/hero_pile.webp            32.1 KB   PageHome / PageCalib 的合影
public/models/hero_pile@2x.webp         73.2 KB
public/printers/bambu/a1.webp           93.5 KB
public/printers/bambu/a1mini.webp       82.5 KB
public/printers/bambu/a1mini-variant.webp 42.9 KB  ← printer-hero.png 转来的
public/printers/bambu/p1s.webp          41.8 KB
```

---

## 二、做了什么

**死链路（两侧一起删）** —— `getTestModels` / `get_test_models` 整条拿掉：
`contract.ts` 的 `TestModel` 接口与方法签名、`mock.ts` 的 93 行数据表、`bridge.ts` 的转发、
`index.ts` 的 re-export、Rust 的 `test_models()` / `#[tauri::command]` / `TestModel` 与 `Tag` 两个结构 /
`lib.rs` 的 `invoke_handler` 注册 / 那条序列化测试。`PageCalib.tsx:63` 的 `TEST_MODEL_ID` 保留 ——
它是「打开测试模型」按钮给 `openModel` 的 3mf id，与这条链路无关。

**死组件** —— `ExplodedHero.tsx`（354 行）+ `ExplodedHero.module.css` 删除，`three@0.186.0` 与
`@types/three` 从 `package.json` 移除。

**资产** —— 删 `plate_1~8.png`、`hero_fishtail.webp` 与 `@2x`、`test_models.glb`（machine-motion
那边已有一份）；`a1/a1mini/p1s.webp` 移进 `public/printers/bambu/`；
`printer-hero.png` 转 WebP 并改名 `a1mini-variant.webp`（它的语义是「a1mini 装了快拆件」）。

**品牌 logo** —— `public/bambulab.svg` 搬进 `src/app/assets/bambuLogo.ts`，做成参数化 data URI，
导出 `BAMBU_LOGO_DARK`（`#111111`，界面在用）与 `BAMBU_LOGO_LIGHT`（`#ffffff`，深色主题备用）。

---

## 三、验证

| 项 | 结果 |
|---|---|
| `npm run build`（tsc -b + vite build） | 通过 |
| `npm run lint`（eslint + stylelint） | 通过 |
| `cargo check --all-targets` | 通过 |
| `cargo test` | 18 passed / 0 failed |
| `cargo clippy --all-targets -- -D warnings` | exit 0，零告警 |
| `dist/` 产物 | 9 个文件，旧资产全部消失，只剩 4 张打印机图 + 2 张合影 + JS/CSS/html |
| 六个新资产 URL（打 `vite preview`） | 全部 `200` + `image/webp`，字节数与磁盘一致 |
| 六个旧路径 | 全部落到 `index.html`（`text/html`），确认已不存在 |
| 全仓 grep 旧路径 | 零残留 |
| 真浏览器跑一遍（品牌→A1→A1 mini→P1S→A2L→快拆版） | 每一步选中态都正确，**零 console 报错、零失败请求** |

**logo 的等价性是量出来的，不是看出来的**：把原 `bambulab.svg`（从 git 取回）与转写后的
markup 都用 ImageMagick 以 970×350 光栅化，`compare -metric AE` 结果 **0** ——
零个像素不同，两张 PNG 连字节数都一样（9824 B）。转写只做了两件等价变换：去掉
`xml:space`/`x`/`y`/`version` 这些无用属性；把 13 个图元各自的 `fill="currentColor"`
收成根元素上的一个 `fill`（fill 会继承，13 处原本同色）。

---

## 四、过程中发现 / 需要纠正 doc 的地方

**1. `replaceAll` 编不过。** 本仓库 tsconfig 的 lib 低于 es2021。改用 `replace(/__FILL__/g, …)`，
没有为一处调用去抬 lib。

**2. `fill="currentColor"` 是 13 处，doc 里写的 8 处是估的。** 已按实际处理。

**3. WebP 压缩比远好于预估，而且担心的色带风险不存在。** doc 里写「1074×1251 有渐变与软阴影，
q82 可能出色带」—— 打开图才发现它是**线稿风格的透视图**（大片纯白 + 细灰线 + 几块纯绿），
不是照片级渲染。260.2 KB → **42.9 KB**（−83%，doc 估的是 −73%），alpha 完整保留
（`srgba`/`alpha=Blend`），细线没糊。

**4. 删 `three` 不会让 bundle 变小。** doc 里那句「three 是本仓最大的前端依赖」在 `node_modules`
层面对，但 `ExplodedHero` 从未被挂载，three 早就被 tree-shake 掉了 —— bundle 前后都是
466 kB 左右。真正的收益在装包（少 8 个包）与仓库整洁，不在产物体积。
顺带一提，logo 内联让 JS 从 466.15 kB 涨到 469.16 kB（+3 kB，就是那份 markup），
换掉的是一次 HTTP 请求和 `public/` 里的 3 KB —— 这是划算的，但要说清它不是净减。

**5. 没能验到的那一项，如实说。** 想在浏览器里眼看大图逐档换图，但 `HeroFade` 所在的
`p.slot` 藏在卡片组的某一页里，脚本驱动的那几步没把它挂出来（页面上一个 `<img>` 都没出现，
也因此没有图片请求）。所以「换图视觉正确」这一条**我只验到了间接证据**：
资产 URL 全部 200、选中态全对、零报错零 404、logo 光栅化逐像素一致。
**建议你自己开一次首页把四档过一遍**（品牌 logo / A1 / A1 mini / A1 mini+快拆版 / P1S / A2L 回落），
这是唯一还没被机器确认的环节。

**6. 顺手发现一个与本轮无关的问题。** 页面上有个 `data-show="false"` 的「下一页」按钮仍在拦
指针事件 —— playwright 的真鼠标点击会被它吃掉（报 `intercepts pointer events`），
我最后改用 `dispatchEvent` 才点到选项。如果那块区域与真实可点元素重叠，用户的鼠标也会点空。
没查它的 CSS，不确定是否真会影响用户，**单独提一句给你判断**，本轮没动它。

---

## 五、Task 6 结果：走了 PR，没重写历史

提交在 `chore/asset-slimdown`（f5ab307，33 files，+668/−732），
PR：https://github.com/MuCoreBenC/MKPSupportEase/pull/8

**重写历史放弃了**，原因是查出来既走不通也不值得：

- `scripts/hooks/pre-push` 闸② **无条件**拒绝 push main，注释里自己写明
  「就算用 --no-verify 绕过这道本地闸，GitHub 的 main ruleset 也会拒」—— 服务端也拦
- tag `v0.0.1` 会被一起重写，重推要撞闸③+闸⑤，而服务端同样禁 tag 删除
- 收益只有约 1 MB：`size-pack` 1.93 MiB，16 个待清 blob 合计 1066.7 KB（每个只有 1 个版本）

对比 machine-motion 那次重写（41.86 → 17.17 MiB，省 24 MB）—— 这次为 1 MB 去临时拆掉
自己装的四道闸和服务端 ruleset，不划算。那 1 MB 留在历史里，工作区与以后的 clone 是干净的。

将来真要做，前置条件是：你去 repo settings 临时放开 main 与 tag 的 ruleset，
并接受 GitHub 不会按需 gc（旧对象在它自己跑 gc 前仍可按 SHA 取到）。

### 一条要撤回的判断

决策当时我还列了第四条理由：「`origin/fix/win-caption-buttons` 还在远端，旧 blob 从它仍可达，
就算 main 重写成功 GitHub 也不回收」。**这条是错的。**

那个分支在 GitHub 上早就不存在了（大概是 PR #7 合并时被「自动删除 head 分支」清掉的），
`git push origin --delete` 直接报 `remote ref does not exist`。我本地看到的
`origin/fix/win-caption-buttons` 只是个**过期的 remote-tracking ref** ——
`git fetch` 不带 `--prune` 永远不会清理它，于是它看起来像还在。`git fetch --prune` 之后
远端就只剩 `main` 与 `chore/asset-slimdown`。

结论不变（前三条理由足够），但这条判断作废，记在这里免得将来被当成事实引用。
顺带一条教训：**判断远端分支是否存在，要先 `git fetch --prune`**，否则读到的是本地缓存。

### 顺带澄清一件事

`origin/fix/win-caption-buttons` 也不需要「合并回主分支」—— 它的内容早就在 main 里。
PR #7 是 squash 合并，所以那 4 个原始提交（`1f37246` / `65e0ccf` / `f4f3005` / `2d81cf5`）
不出现在 main 的历史里（`git branch -r --merged main` 因此不列它），但两边的 tree
逐字节相同（`git diff origin/main origin/fix/...` 输出为空）。

## 六、仍然挂着的两件

**① 人眼过一遍首页六档**：品牌 logo / A1 / A1 mini / A1 mini+快拆版 / P1S / A2L 回落。
这是唯一没被机器确认的环节（原因见第四节第 5 条）。

**② 品牌 logo 要不要去掉**（你倾向去掉，本轮选了先不动）。补一条事实：logo 不是完全没用到。
`PageHome.tsx:204` 的大图槽位只在 `sel.model` 有值时才挂出来，所以**只选品牌**永远看不到
logo（你观察到的就是这个）；但选 **A2L / P2S / X1C** 这三个还没配图的机型时，槽位在、
`MODEL_ART` 没条目，`pickArt` 会回落到 logo，那时它真会显示（32% 透明度、四周内缩，
样式在 `HeroFade.module.css:44-67`）。所以去掉之前要先定那三个机型显示什么。

