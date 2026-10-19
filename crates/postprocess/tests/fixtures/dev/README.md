# 开发台「真实后处理测试矩阵」的夹具

这些文件**只给工作台开发台用**（`src-tauri/src/workbench/app/devmatrix.rs`），
不进正式包，也不参与 `cargo test`。与 `tests/fixtures/real/` 同一条纪律：
**夹具只提供条件，不提供结论** —— 结局由真管线决定。

矩阵把每一档跑在**独立临时根**里（`target/dev-run/matrix/<档>-<时间戳>/`），
正式数据目录一个字节不动（见交接文档 `docs/DEV-REAL-POSTPROCESS-HANDOFF-2026-10-19.md` §8）。

| 文件 | 档 | 凭什么触发什么 |
| --- | --- | --- |
| `small-ok.gcode` | 小文件成功 | 机型头齐全 + 一小片模型 ⇒ 完整跑通（实测 101 ms，`error=null`） |
| `collide-tower.gcode` | 挪塔碰撞 | `; FEATURE: Support`（让擦料决策走 tower）+ 模型点落在塔框 22.96..57.04 里（X30 Y30）⇒ 第 6 步碰撞问句 |
| `broken-preset.toml` | 硬失败 | 故意不是 TOML ⇒ 加载预设那一步真失败（实测退出码 2 + `E_TOML_PARSE_001`） |
| `calib-zoffset-ext.gcode` | 校准件（跑完 / 问了没答） | **由仓库里那份 3MF 真切片而来**，挤出包围盒 119.511 × 16.063 ⇒ 命中 `zoffset-ext` 档；配合临时根里的 `origin.txt` 才认得出身份 |

**大文件成功**与**机型不匹配**不另放夹具：

- 大文件 = `tests/fixtures/real/.5926.1.gcode`（8.5 MB，被 `.gitignore` 掉，本机才有）；
- 机型不匹配 = 同一份大件 + `P1S` 系预设（机型名对不上 ⇒ 那一问）。

**取消**档不提供夹具（用大件），因为它要人在客户端点「停止」—— 那一"点"是这一档的全部意义。

## 两个反直觉的实测（别再踩）

1. **缺 `printer_model` 不是硬失败**。管线很宽容：`is_machine_mismatch` 第一条就放过
   "认不出机型"，照常跑完并判成功（实测 111 ms）。所以"硬失败"档不能拿缺机型头来造。
2. **`rib-tall-min.gcode` 单独跑不会碰撞**。它没有 `; FEATURE: Support` ⇒
   `decide_wiping` 判"这一趟不需要擦料" ⇒ `tower.use_towers=false`
   ⇒ `detect_tower_collision` 第一行就 `return`。`pipeline_tower_ask.rs` 能撞是因为
   它**手搭 IR**（直接置 `use_towers = true`）绕过了擦料决策 —— 真钩子入口做不到那一步。

## `calib-zoffset-ext.gcode` 是怎么来的（要重生成就这么做）

不用手搓图案 —— 本机装了 Bambu Studio 就能直接切：

```bash
/Applications/BambuStudio.app/Contents/MacOS/BambuStudio \
  --slice 0 \
  --outputdir /tmp/calib-slice \
  presets/assets/models/ZOffset_Calibration_ext.3mf
# ⇒ /tmp/calib-slice/plate_1.gcode，复制到本目录并改名
```

（`--export-3mf` 那个参数会报 "Unable to open the file" 并给退出码 243，**不影响** `--slice` 出 G-code。
另外它会在**当前目录**留一个 `result.json`（切片回执），切完删掉 —— 别让它混进仓库。）

切出来的件只说明"输入是真的"；**身份**还要靠矩阵在临时根里写的那行 `origin.txt`：

```
<临时根>/origin.txt          ← 一行 = 上面那份 3MF 的绝对路径
<临时根>/input/plate.gcode   ← 本目录这份的副本
```

`temp_root()` 取的是 **gcode 的上一级**，所以 `origin.txt` 放在 `input/` 的上一级。

★ 别拿这份件去跑别的档：它的包围盒是**特地**落在 `zoffset-ext` 的容差里的，
换个预设可能报 `E_CAL_MISMATCH_002`（那不是夹具坏了，是尺寸对不上档）。
