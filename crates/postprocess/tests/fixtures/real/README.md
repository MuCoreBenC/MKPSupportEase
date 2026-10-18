# 真实 G-code 样例（本机跑，不入库）

这个目录是**斜肋高塔**那类真机问题的复现入口（2026-10-18，作者 `.5926.1.gcode`）。

## 大文件放这里，但不进 git

`*.gcode`（除 `rib-tall-min.gcode` 之外）已在 `crates/postprocess/tests/fixtures/real/.gitignore` 里忽略 ——
真文件通常 1~2MB，入库只会把仓库撑大，而且它带用户的模型内容。

要复现时，把切片器导出的那份拷进来即可：

```bash
cp "/var/folders/.../Metadata/.5926.1.gcode" \
   crates/postprocess/tests/fixtures/real/.5926.1.gcode
```

## 入库的那一份：`rib-tall-min.gcode`

从真文件裁出来的**最小复现**（几 KB），保留：

- `; max_z_height:`（塔高 —— 斜肋底座随它变大）；
- `; printable_area`（板子范围）；
- 若干带挤出的 `G1`（模型点云）；
- 不保留温度 / 风扇 / 抬 Z 的杂项。

判据 `collision::tests::the_real_rib_sample_moves_the_tower_off_the_points` 读它：
换成 rib 外墙 + 按这份文件里的塔高算框，自动摆塔给出的位置**不许压到那些点上**。

## 跑法

```bash
cargo test -p mkpse-postprocess --lib collision
```
