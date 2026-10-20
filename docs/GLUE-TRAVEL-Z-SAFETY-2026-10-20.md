# 胶笔移动安全余量 · 权威结论（2026-10-20）

> **本文是唯一结论，不再重复讨论。** 以后遇到"Z 抬升 / 会不会撞 / 该调哪个参数"，先读 §0 和 §4。
> 实测样本：`MKP_support_test_models_20261020211150.gcode`（作者 A1，24 层，层高 0.2）。
> 代码基准：`crates/postprocess/src/postproc/pass1/glue_block.rs`。
> 已发生事故：**打印到高层时模型被撞倒（炒面）**，详见 §4.3。
> **2026-10-21 已按 §7 修复并落地**（段 1 改「G3 小螺旋 z-hop 1mm + 平飞 + 原位抬升」、
> 段 2 净空 2.8 → 5.0）；§1~§6 是修复前的实测与推导，保留作依据。

---

## 0. 三十秒速查

### 0.1 三个铁律

1. **判据永远是"笔尖离层顶多少"，不是"喷嘴 Z 是多少"。** 笔是伸缩的，同一喷嘴高度下，笔伸出与收起时笔尖相差 `3.7 + 8~10 ≈ 12~14mm`，看喷嘴会严重误判。
2. **笔伸出时，所有抬升都写成 `层顶 + z_offset + 常量`**。减掉 `z_offset` 后，笔尖余量恒等于那个常量，与笔伸多长无关。所以"笔越长越危险"只出现在**笔已伸出却按喷嘴高度走**的地方。
3. **顺序比数值重要。** 先抬后走 = 全程恒定余量；XY+Z 同一句斜插 = 余量沿途线性变化，起点往往就是最危险的。

### 0.2 一条万能公式

```
喷嘴 Z          = 层顶 + 余量 + (笔伸出 ? z_offset : 0)
笔尖 Z(伸出)    = 喷嘴 Z − z_offset
笔尖 Z(收起)    = 喷嘴 Z + 8~10
```

任何一行 G-code 都用这个式子反推，能立刻判断该段是否安全。

### 0.3 风险一览（**修复前**；层顶 12.8、`offset_z = 3.7` 的实机数据）

| 段落 | 危险对象 | 实际余量 | 判定 |
|---|---|---|---|
| 打印完 → 切笔位（`G1 X252 Z13.8`） | **喷嘴** | 起点 **0.0** → 终点 1.0（斜插） | ❌ 贴面滑行 120mm |
| 装笔横移（`X256/261/267`，Z 19.5） | 笔尖（伸出） | +3.0 | ⚠️ 偏紧 |
| 飞回涂胶点（`X127`，Z 19.3） | 笔尖（伸出） | **+2.8** | ⚠️ 最紧 |
| 落笔（`G1 Z16.25`） | 笔尖（伸出） | −0.05 | ✅ 正确压入 |
| 收笔 / 去卸笔（Z 21.5） | 笔尖（伸出） | +5.0 | ✅ 可接受 |
| 涂胶块内部段间跳转 | 笔尖（伸出） | +2.0 | ✅ 作者认可足够 |

---

## 1. 机构模型（一切推算的基准）

### 1.1 笔是伸缩机构，不是固定件

- **伸出（涂胶态）**：笔尖低于喷嘴，差值 = `toolhead.offset_z`（本机 **3.7**）
- **收起（打印/待机态）**：笔尖在喷嘴**上方**，行程约 **8~10mm**（作者实测，本文取 8 保守）
- 换笔动作由 `custom_mount_gcode` / `custom_unmount_gcode` 中的 `L801` / `L802` 完成

因此同一段路径，笔伸出和收起时的风险评估**完全不同**，这是本文最关键的前提。

### 1.2 三个 Z 量（不要混）

| 量 | 含义 | 本机值 | 落点 |
|---|---|---|---|
| `current_layer_height` | 当前层顶（来自切片器 `;Z_HEIGHT:`） | 例 12.8 | `pass1/mod.rs:311-321` |
| `last_layer_height` | 上一层顶 | 例 12.6 | 同上 |
| `toolhead.offset_z` | **笔尖离喷嘴的距离**（伸出时笔尖比喷嘴低多少） | **3.7** | `presets/delivery/mkp/presets/A1-fastv3.3.toml` `offset_z = 3.7` |
| `glue.z_offset` | 涂胶落笔的压入量 | **−0.05** | 同文件 `glue_z_offset = -0.05` |

> ⚠️ `offset_z` **同时**决定"落笔时笔尖贴面"和"抬笔时喷嘴该抬多高"。它只在漏加的地方不生效。

---

## 2. 全流程逐段公式 + 真实代码

### 2.1 打印完 → 切笔位（**核心病灶 ①**）

```529:544:crates/postprocess/src/postproc/pass1/glue_block.rs
    if custom_mount_gcode_has_retract(ir_data) {
        sink.push_str(&format!(
            "G1 X{} Z{} F{}",
            format_float(wipe_x),
            format_z(current_layer_height + 1.0),
            format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
        ));
    } else {
        sink.push_str(&format!(
            "G1 X{} Z{} E{} F{}",
            format_float(wipe_x),
            format_z(current_layer_height + 1.0),
            format_e(ir_data.gcode.mkp_retract),
            format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
        ));
    }
```

- `wipe_x = 252.0`（`presets/machines/A1.toml:44`；A1 mini 为 160）
- 公式：`喷嘴 = 层顶 + 1.0`，**故意不加 `z_offset`** —— 因为此时笔已收起，基准是喷嘴，这是**正确**的
- 问题在**语法**：`X` 与 `Z` 写在同一句，控制器做直线插补

**这一段的真实余量分布**（层顶 12.8，起点 X≈132.3，终点 X=252，横移 119.7mm，Z 只升 1.0）：

| 走到 | X | 喷嘴 Z | 喷嘴余量 | 笔尖（收起） |
|---|---|---|---|---|
| 0% | 132.3 | 12.80 | **0.00** | ≈20.8 |
| 25% | 162.2 | 13.05 | +0.25 | ≈21.1 |
| 50% | 192.1 | 13.30 | +0.50 | ≈21.3 |
| 75% | 222.1 | 13.55 | +0.75 | ≈21.6 |
| 100% | 252.0 | 13.80 | +1.00 | ≈21.8 |

**结论：起点余量为 0，喷嘴在层顶高度起跑，贴着刚打完的顶面滑行近 120mm。** 前 60mm 余量不到 0.5mm，毛刺、料头、轻微翘边必然擦到。与 `z_offset` 无关，调任何抬升参数都无效。

### 2.2 抬到装笔高度（RiseNozzle）

```554:563:crates/postprocess/src/postproc/pass1/glue_block.rs
    sink.push_str(crate::gcode::MARKER_RISING_NOZZLE);
    sink.push_str(&stage_comment_line(MkpStage::RiseNozzle, ""));
    let mut lift_height = ir_data.glue.z_lift_height;
    if current_layer_height < 3.0 {
        lift_height = ir_data.glue.z_lift_height_first_layers;
    }
    sink.push_str(&format!(
        "G1 Z{}",
        format_z(current_layer_height + ir_data.toolhead.z_offset + lift_height)
    ));
```

- 公式：`层顶 + z_offset + lift`（`lift` = 5.0，或 `层顶 < 3.0` 时 6.0）
- 笔尖（伸出）= 层顶 + lift → **余量恒为 lift，与笔长无关** ✅
- 此处没有 XY 移动，纯垂直抬升 ✅

### 2.3 装笔 L801

```421:426:crates/postprocess/src/postproc/pass1/glue_block.rs
        if ml.contains("L801") {
            let mut l801_z = current_layer_height + ir_data.toolhead.z_offset + 3.0;
            if current_layer_height < 3.0 {
                l801_z = current_layer_height + ir_data.toolhead.z_offset + 4.0;
            }
            sink.push_str(&format!("G1 Z{};L801", format_z(l801_z)));
        }
```

- `层顶 + z_offset + 3.0`（`<3.0` 时 `+4.0`）
- 这是**装笔动作高度**，笔在此处由收起变伸出
- 紧随其后的 `X256→261→267→256` 横移就发生在这个高度：笔尖余量 = 3.0

### 2.4 装好后飞回涂胶点（**核心病灶 ②**）

```433:440:crates/postprocess/src/postproc/pass1/glue_block.rs
    sink.push_str(";Toolhead Mounted");
    if !skip_final_z {
        sink.push_str(&format!(
            "G1 Z{}",
            format_z(last_layer_height + ir_data.toolhead.z_offset + 3.0)
        ));
    }
```

- 公式：**`上一层 + z_offset + 3.0`**（注意用 `last`，比 `current` 低 0.2）
- 随后 `G1 X{涂胶起点}` 在同一高度横移，这是**笔伸出状态下最长的一条横移**
- 笔尖余量 = 3.0 − (current − last) = **2.8**
- 全流程最小余量，且横穿整个模型区

### 2.5 落笔

```606:609:crates/postprocess/src/postproc/pass1/glue_block.rs
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
```

- `上一层 + z_offset + (−0.05)` → 笔尖压入上一层顶面 0.05mm ✅
- 顺序正确：先在 19.3 横移到点，再单独落 Z

### 2.6 涂胶结束收笔

```912:921:crates/postprocess/src/postproc/pass1/glue_block.rs
    let mut glue_finished_z =
        current_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_lift_height;
    if current_layer_height < 3.0 {
        glue_finished_z = current_layer_height
            + ir_data.toolhead.z_offset
            + ir_data.glue.z_lift_height_first_layers;
    }
    sink.push_str(&format!("G1 Z{}", format_z(glue_finished_z)));
    sink.push_str(&format!(";Lift-z:{}", format_z(glue_finished_z)));
```

- 与 §2.2 同公式（用 `current`），笔尖余量 = lift = 5.0
- **先抬 Z，再由 `custom_unmount_gcode` 横移到卸笔位** → 顺序正确，作者反馈"可以接受" ✅

### 2.7 去擦料塔

```940:948:crates/postprocess/src/postproc/pass1/glue_block.rs
        let offset_result = crate::postproc::offset::process_gcode_offset(
            "G1 X20 Y10.19",
            ir_data.wiping.wiper_x - tower_bbox.min_x,
            ir_data.wiping.wiper_y - tower_bbox.min_y,
            current_layer_height + 3.0,
            crate::gcode::OffsetMode::Normal,
            ir_data,
        )?;
```

- ⚠️ 这里的 `current_layer_height + 3.0` 是**作为 z 偏移参数**传入的，但源命令 `G1 X20 Y10.19` **没有 Z**，而 `process_gcode_offset` 只在 `has_z` 时才处理 Z（`offset.rs:136-139`）
- 结果：**这个"抬高到层顶+3"的意图从未生效**，实际高度继承上一行（收笔的 21.5）
- 目前安全，纯属继承值够高

---

## 3. 实机账本（本文件逐行验证）

### 3.1 四层抽样，公式全部命中

| 层顶 | `G1 X252 Z…` | 抬笔 `G1 Z…` | `;L801` | `;Toolhead Mounted` 后 | 落笔 `G1 Z…` |
|---|---|---|---|---|---|
| 0.6 | 1.6 | 10.3 | 8.3 | 7.1 | 4.05 |
| 2.8 | 3.8 | 12.5 | 10.5 | 9.3 | 6.25 |
| 3.0 | 4.0 | **11.7** | 9.7 | 9.5 | 6.45 |
| 12.8 | 13.8 | 21.5 | 19.5 | 19.3 | 16.25 |

验算（`offset_z = 3.7`）：

```
0.6 层： 0.6+1.0=1.6 │ 0.6+3.7+6.0=10.3 │ 0.6+3.7+4.0=8.3 │ 0.4+3.7+3.0=7.1 │ 0.4+3.7-0.05=4.05
2.8 层： 2.8+1.0=3.8 │ 2.8+3.7+6.0=12.5 │ 2.8+3.7+4.0=10.5 │ 2.6+3.7+3.0=9.3 │ 2.6+3.7-0.05=6.25
3.0 层： 3.0+1.0=4.0 │ 3.0+3.7+5.0=11.7 │ 3.0+3.7+3.0=9.7  │ 2.8+3.7+3.0=9.5 │ 2.8+3.7-0.05=6.45
12.8 层：12.8+1.0=13.8│ 12.8+3.7+5.0=21.5│ 12.8+3.7+3.0=19.5│ 12.6+3.7+3.0=19.3│ 12.6+3.7-0.05=16.25
```

> 全部为导出文件实测值，已逐行比对（0.6 层 5430/5433/5439/5447/5454，2.8 层 18767/18770/18776/18784/18791，
> 3.0 层 19675/19678/19684/19692/19699，12.8 层 43570/43573/43579/43587/43594）。

### 3.2 原文摘录（行号为该 gcode 文件）

**第 0.6 层**（`ev=1`，行 5428-5455）：

```gcode
;MKP_BEGIN swap ev=1 to=pen z=0.6
M106 S255.0
G1 X252.0 Z1.6 F42000.0
;Rising Nozzle a little
;MKP_STAGE: RiseNozzle ; 抬升喷嘴
G1 Z10.3
;Mounting Toolhead
G1 Z8.3;L801
G1 X256 F42000
G1 X261 F10000
G1 X267 F10000
G1 X256 F42000
G1 F42000
;Toolhead Mounted
G1 Z7.1
;MKP_END swap
;MKP_BEGIN paint ev=1 z=0.6
G1 X133.264 Y142.717
G1 Z4.05
```

**第 3.0 层**（`ev=13`，行 19673-19678）——阈值切换点，抬升从 6.0 掉到 5.0：

```gcode
;MKP_BEGIN swap ev=13 to=pen z=3.0
M106 S255.0
G1 X252.0 Z4.0 F42000.0
;Rising Nozzle a little
;MKP_STAGE: RiseNozzle ; 抬升喷嘴
G1 Z11.7
```

**第 12.8 层**（`ev=23`，行 43568-43594）——事故层：

```gcode
G1 X132.277 Y124.446 E-.38
;Pre-glue preparation
;MKP_BEGIN swap ev=23 to=pen z=12.8
M106 S255.0
G1 X252.0 Z13.8 F42000.0      ← 斜插：喷嘴余量 0 → 1.0
;Rising Nozzle a little
;MKP_STAGE: RiseNozzle ; 抬升喷嘴
G1 Z21.5
;Mounting Toolhead
G1 Z19.5;L801                 ← 装笔，笔变伸出
G1 X256 F42000                ← 笔尖余量 +3.0
G1 X261 F10000
G1 X267 F10000
G1 X256 F42000
;Toolhead Mounted
G1 Z19.3                      ← 笔尖余量 +2.8
;MKP_END swap
;MKP_BEGIN paint ev=23 z=12.8
G1 X127.392 Y146.589          ← 横穿 135mm，余量 2.8
G1 Z16.25                     ← 落笔压入 0.05
```

**收笔**（行 43607-43630）：

```gcode
;Glueing Finished
;MKP_STAGE: GlueEnd ; 涂胶结束
G1 Z21.5                      ← 先抬，笔尖余量 +5.0
;Lift-z:21.5
;MKP_END paint
;MKP_BEGIN swap ev=23 to=print z=12.8
;Unmounting Toolhead
G1 X256 F42000                ← 在 21.5 横移，笔仍伸出
G1 X261 F10000
G1 X273 F10000
G1 X256 F5000
G1 F42000
L802                          ← 这里才收笔
```

---

## 4. 风险清单

### 4.1 按余量排序

| 排名 | 段落 | 危险对象 | 余量 | 范围 | 可调性 |
|---|---|---|---|---|---|
| 1 | 打印完 → 切笔位 | **喷嘴** | **0 → +1.0** | 120mm 斜插 | 写死 `+1.0`，不受任何面板参数影响 |
| 2 | 飞回涂胶点 | 笔尖 | **+2.8** | 129mm 直线 | 写死 `+3.0` 且用 `last` |
| 3 | 装笔横移 | 笔尖 | +3.0 | ~40mm | 写死 `+3.0` |
| 4 | 收笔 / 去卸笔 | 笔尖 | +5.0 | 40mm | `glue_z_lift_height`（5~10） |
| 5 | 涂胶块内段间 | 笔尖 | +2.0 | 短 | `glue_inter_model_lift_height`（2~5），作者认可够用 |

### 4.2 为什么"调参数没用"

面板上能调的三个值只作用于 §2.2 / §2.6 / 段间跳转，**完全不影响排名 1~3**：

- 排名 1：`current_layer_height + 1.0`，硬编码
- 排名 2：`last_layer_height + z_offset + 3.0`，硬编码
- 排名 3：`current_layer_height + z_offset + 3.0`，硬编码

### 4.3 已发生事故

**2026-10-20，打印到高层时模型被撞倒（炒面）。** 机制：

- 模型越高，XY 截面越大，进入"危险走廊"的概率越高
- 低层时模型矮、面积小，段 1 的喷嘴贴面滑行可能刚好从空处经过，**问题被掩盖**
- 高层时同样的路径必然穿过实体，且模型细长、抗侧向力差，一碰即倒

候选病灶（两个都已具备"高层必然命中"的性质）：

1. **段 1**：喷嘴在 12.8 层顶起步，贴面滑行 120mm
2. **段 2**：笔尖只比层顶高 2.8mm，横穿 129mm

判别方法：看炒面时的现场——

- 模型顶部被**侧面刮倒**、喷嘴/热端有塑料残渣 → 段 1
- 模型顶部有**横向划痕/胶带痕迹**、笔架上有料 → 段 2

---

## 5. 参数对照表（面板 ↔ TOML ↔ 代码）

| 面板名 | TOML key | 本机值 | 范围 | 作用点 |
|---|---|---|---|---|
| Z 轴偏移 | `offset_z` | **3.7** | 0~20 | 落笔贴合 + 所有伸笔态抬升（加性） |
| 涂胶Z偏移 | `glue_z_offset` | −0.05 | — | 仅落笔压入量 |
| 涂胶Z轴抬升高度 | `glue_z_lift_height` | 5.0 | 5~10 | §2.2 抬笔、§2.6 收笔 |
| 前3层抬升高度 | `glue_z_lift_height_first_layers` | 6.0 | 6~10 | 条件 `层顶 < 3.0` 时**替换**上一项 |
| 涂胶区域间Z轴抬升高度 | `glue_inter_model_lift_height` | 2.0 | 2~5 | 涂胶块内段间/模型间跳转 |

参数定义：`presets/registry/param_registry.toml`；映射与 clamp：`crates/preset/src/build.rs:222-235`。

---

## 6. 三个必须记住的坑（已踩过，不要再争）

### 坑 1：`< 3.0` 是**Z 高度**，不是"前 3 层"

`glue_block.rs:557` 的条件是 `current_layer_height < 3.0`。层高 0.2 时实际是**前 14 层**都用 6.0。面板文案"前3层"与代码口径不一致。

**副作用**：3.0 层是断崖点——2.8 层抬到 12.5，3.0 层反而只到 11.7（层更高、抬得更低）。

### 坑 2：装笔后的横移用的是 `last`，不是 `current`

`glue_block.rs:437` 用 `last_layer_height`。12.8 层时 `G1 Z19.3` 对应的是 12.6 而不是 12.8，凭空少 0.2。

### 坑 3：`G1 X252 Z13.8` 不加 `z_offset` 是对的

很多人（包括本文初稿）会以为这里是漏了 `z_offset` 的 bug。实际此时**笔已收起**，基准是喷嘴，`+1.0` 是"喷嘴离层顶 1mm"，语义正确。真正的问题是斜插语法（§2.1），不是这个数值。

---

## 7. 已实施的修复（2026-10-20 ~ 10-21）

> §1~§6 是**修复前**的实测与推导，保留作为依据；本节记录落地结果。

### 7.1 改了什么

| 项 | 状态 | 实际改法 |
|---|---|---|
| 段 1 去切笔位 | ✅ 已改 | 原 `G1 X{wipe_x} Z{层顶+1.0}` 斜插 → ① `M204` + `G17` + `G3 Z{层顶+1.0} I.706 J.991 P1` 小螺旋脱层 ② 原位 `G1 Z{层顶+offset_z+3.0}` 抬到装笔高度 ③ `G1 X{wipe_x}` 高位平飞 |
| 段 2 飞回涂胶点 | ✅ 已改 | `last + offset_z + 3.0` → `extended_travel_z(ir, current)`；`mount_toolhead` 删掉 `last_layer_height` 入参，从根上消除 `last/current` 混用 |
| 装笔高度 L801 | ⏸ 未改 | 仍是 `层顶+offset_z+3.0`（`<3.0` 时 `+4.0`）—— 装笔机构的机械工作高度，改了可能装不到位 |
| 去擦料塔 | ⏸ 未改 | 依赖收笔的 21.5 继承，天然 ≥ 层顶+3.0，改动只会降低 |
| 面板参数 | **未新增** | 复用 `glue_z_lift_height` / `_first_layers`，与收笔同口径；作者把 5 改 7 时这两段同步变高，正是预期 |
| 段间 2.0 / 落笔 / 收笔 | 未动 | 作者认可现状 |

统一函数（`glue_block.rs`，`mount_toolhead` 之前）：

```rust
fn extended_travel_z(ir_data: &Ir, layer_z: f64) -> f64 {
    let lift_height = if layer_z < 3.0 {
        ir_data.glue.z_lift_height_first_layers
    } else {
        ir_data.glue.z_lift_height
    };
    layer_z + ir_data.toolhead.z_offset + lift_height
}
```

### 7.2 修复后的实际输出（层顶 12.8）

```gcode
;MKP_BEGIN swap ev=21 to=pen z=12.4
M106 S255.0
M204 S10000
G17
G3 Z13.4 I.706 J.991 P1 F42000.0    ← ① 小螺旋脱层：离开层顶 1.0
;Rising Nozzle a little
;MKP_STAGE: RiseNozzle ; 抬升喷嘴
G1 Z19.1                             ← ② 原位抬到装笔高度（= L801 高度）
G1 X252.0 F42000.0                   ← ③ 高位平飞（喷嘴余量 6.7）
;Mounting Toolhead
G1 Z19.1;L801                        ← 同高，不再先降 21.1 再降一次
G1 X256 F42000
...
;Toolhead Mounted
G1 Z21.1                             ← 笔伸出后抬到飞回净空（笔尖余量 5.0）
;MKP_END swap
;MKP_BEGIN paint ev=21 z=12.4
G1 X127.392 Y146.589                 ← 在 21.1 飞回
G1 Z15.85                            ← 落笔，未改
```

| 段落 | 修复前 | 修复后 |
|---|---|---|
| 段 1 平飞时喷嘴余量 | 0 → +1.0（斜插，起点贴面） | **+6.7（19.1，= 装笔高度）** |
| 切笔段 Z 往返 | 13.4→21.1→19.1→21.1（两上一下） | 13.4→**19.1**→19.1→21.1（只上不下） |
| 段 2 笔尖余量 | +2.8 | **+5.0** |
| 装笔横移 | +3.0 | +3.0（未改，机械高度，但不再往返） |

### 7.2.1 三个动作各自负责什么

| 动作 | 负责 | 不能省的理由 |
|---|---|---|
| ① 小螺旋 `G3 Z{层顶+1.0}` | **脱层**：把喷嘴从刚打完的表面拔起来 1mm | 直接原地抬会拉丝；直接斜插会从零净空贴面飞 |
| ② 原位 `G1 Z{装笔高度}` | **抬到平飞高度** | 只脱层 1mm 就横移，喷嘴全程贴着同层其它模型走 —— 等于把段 1 的问题换了个形式 |
| ③ `G1 X{wipe_x}` | **高位平飞** | 这一句起才允许长距离移动 |

**尺度**：螺旋只负责脱层，与切片器自己的 `G3 Z.8 I.706 J.991 P1 F42000`（层顶 0.6 → 0.8，只 z-hop 0.2）同量级；这里取 1.0。

**不要在螺旋里抬到装笔高度** —— 那是 8.7mm 的单圈螺旋，Z 轴按 20mm/s 限速要爬 0.44s，动作夸张；脱层之后没有粘连，一次普通的 `G1 Z` 就够了。

**I/J 分量照抄切片器**（`sqrt(.706² + .991²) = 1.217`，与项目 `SPIRAL_LIFT_RADIUS` 同半径），**不用 `I1.217 J0`** —— `J0` 那种写法在部分固件上会走不同的圆弧分支。

### 7.2.2 为什么装笔前后不再往返

历史写法是 `抬到 层顶+offset_z+5.0` → `降到 L801 的 层顶+offset_z+3.0` 装笔 → `再抬回 +5.0` 飞回（12.4 层就是 `21.1 → 19.1 → 21.1`）。中间那次下降没有意义：装笔前笔还没伸出，不需要 5.0 的笔尖净空。

现在 ② 直接抬到 **`L801` 的高度**，装笔那行变成同高空跑，装完之后才抬到 `+5.0` 飞回。省掉一次 2mm 下降。

### 7.3 防退化护栏

`crates/postprocess/tests/travel_clearance.rs`：跑真实 fixture 的 pass1+pass2 全链，逐行跟踪 Z，断言

0. `swap#1` 在 `;Mounting Toolhead` 之前必须先出现 `G3` 小螺旋（z-hop 1mm）—— 脱层前出现原地 `G1 Z` 直接判失败（防回归到拉丝写法）；螺旋之后在切笔位上空的原位抬升是允许的；
1. `swap#1` 在 `;Mounting Toolhead` 之前的任何 >20mm 横移，当前 Z ≥ **装笔高度**（`层顶 + offset_z + 3.0`，`< 3.0` 时 `+4.0`）—— 只脱层 1mm 就平飞会贴着同层其它模型走；
2. `;Toolhead Mounted` 之后第一条 Z ≥ `层顶 + offset_z + lift`（笔已伸出，管笔尖）；
3. `paint` 块**落笔之前**的 >20mm 横移（飞回涂胶点）同样 ≥ `层顶 + offset_z + lift`。

装笔机构动作（`;Mounting Toolhead` ~ `;Toolhead Mounted`）豁免。反空转：实测覆盖 10 个 swap#1 块（10 个全部走螺旋）、20 次长距离横移。

### 7.4 判据同步

- `tests/golden/42274.2.expected.gcode`、`42274.2.pass2.expected.gcode`：10 个换笔块改成「螺旋脱层 → 原位抬到装笔高度 → 平飞」（字节数 `532_561` / `572_233` 不变，只换了两句的位置与 Z 值），sha256 已登记进 `golden_integrity.rs`；
- `tests/end_to_end.rs`：与上游参考的整链比对**剔除换笔段**（`;Pre-glue preparation` → `;Glueing Started`）后仍逐字节相同。不覆盖参考物——那会让判据退化成自比自（2026-09-12 已踩过一次）。

**仍然不建议**：把 `z_offset` 再额外加成到 5.0/6.0 上。笔尖余量 = 常量，笔长已经 1:1 抵消，再加会让短笔浪费行程、长笔逻辑不可预期。

---

## 8. 代码索引

| 功能 | 位置 |
|---|---|
| 段 1 去切笔位（小螺旋 + 高位横移） | `crates/postprocess/src/postproc/pass1/glue_block.rs:541-576` |
| 统一净空函数 `extended_travel_z` | `glue_block.rs:406-419` |
| 螺旋半径常量 | `crates/postprocess/src/postproc/tower/constants.rs:38`（`SPIRAL_LIFT_RADIUS = 1.217`） |
| 装笔 L801 | `glue_block.rs:421-426` |
| 装后横移高度 | `glue_block.rs:433-440` |
| 落笔 | `glue_block.rs:606-609` |
| 收笔（GlueEnd） | `glue_block.rs:912-921` |
| 去擦料塔 | `glue_block.rs:940-948` |
| 主装配 | `glue_block.rs:495-576` |
| 段间微抬 | `glue_block.rs:104-139` |
| Z 偏移生效点 | `crates/postprocess/src/postproc/offset.rs:136-139` |
| 参数定义 | `presets/registry/param_registry.toml` |
| IR 映射 | `crates/preset/src/build.rs:73`、`:222-235` |
| 机型 wipeX | `presets/machines/A1.toml:44` |

---

## 9. 自查方法（换设备 / 换配置后 30 秒复核）

1. 在导出的 G-code 里找 `;MKP_BEGIN swap ev=… to=pen z=<层顶>`。
2. 往后看四行，取三个数：`X252` 那行的 Z、`RiseNozzle` 后的 Z、`;Toolhead Mounted` 后的 Z。
3. 套公式：

```
段1 终点喷嘴余量 = Z(段1) − 层顶                  应 ≥ 2.0
段1 起点喷嘴余量 = 上一行 Z − 层顶                通常为 0（此时全程贴着层顶飞）
抬笔笔尖余量   = Z(RiseNozzle) − 层顶 − offset_z   应为 5.0 或 6.0
装后笔尖余量   = Z(Mounted) − 层顶 − offset_z      若 < 3.0 即为高危
```

4. 若 `段1` 那一行同时含 `X` 和 `Z`，且上一行 Z 等于层顶 → **该段从零余量起步**，无论参数多大都不安全。

---

## 10. 一句话总结

> 系统里 6 段移动有 4 段算对了：抬笔、收笔、落笔、段间都在用"笔尖离层顶"的基准。**没设计的是"从刚打印完的位置飞到切笔位"和"装好笔飞回涂胶点"这两段**——前者是斜插从零余量起步，后者是硬编码 3.0 让笔尖只剩 2.8，且两者都在模型区上方横穿一百多毫米。装笔后的横移（+3.0）同属偏紧但没到失效。**2026-10-20 的高层炒面就是前两段之一造成的。**
