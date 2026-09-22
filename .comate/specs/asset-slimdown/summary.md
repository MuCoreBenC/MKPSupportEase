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

## 五、Task 6 等你确认（不可逆）

改动已全部落在工作区，**还没提交**。`git status` 干净可读：17 个删除、8 个修改、
2 个新增目录（`public/printers/`、`src/app/assets/`）。

要真正回收已推送到 GitHub 的那些旧图，下一步是重写历史。执行前你要知道：

- 所有提交 SHA 会变，GitHub 上的 `main` 被**强制覆盖**
- PR #1~#7 的 diff 会变成 orphan（正文与评论留着，diff 可能显示异常）
- **另一台机器必须删掉目录重新 clone**，`git pull` 会因为没有共同祖先而炸
- 我会先 `git bundle` 全量备份，并在强推前把 `git count-objects -vH` 的前后数据摆给你看

另外两件要你定的小事：

- `tsconfig.app.tsbuildinfo` 是**被跟踪的构建产物**，每次 build 都会变脏。本轮照原样提交，
  但它本该进 `.gitignore` —— 要不要顺手收拾？
- `git status` 里还有两个**不是本轮**的未跟踪目录：`.comate/specs/win-chrome-interactions/`、
  `.comate/specs/windows-caption-bar/`。我不会碰。

说「继续」我就走 Task 6；想先自己看一眼界面也行，改动都在工作区等着。
