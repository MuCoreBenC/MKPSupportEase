# 旧名 → 新身份映射表（b05 Task 4）

> **这份表是一次性迁移输入，不是规范。** 有效期到「B 套名字从仓里消失」为止：
> 基线夹具改名（Task 5）、机型定义的 `presetFile` 删除（G-2 / Task 16.3）、
> 四处硬编码名单更新（Task 5.5）之后，这份表就只剩审计价值 ——
> **不进入任何长期代码路径**，也不许有代码反过来依赖它。
>
> 规范是 `docs/ARCHITECTURE.md` §10：文件名 = `<机型 id>-<版本 id 小写>.toml`，
> 由 `preset::preset_file_name` 唯一算出。**这份表的每一行都是它的一个实例。**

---

## 1. 九对映射（4.1）

| 旧云端名（B 套） | 机型 id | 版本 id | 新文件名（A/C 套） | 字节数（两侧相同） | sha256 前 16 位（两侧相同） |
| --- | --- | --- | --- | --- | --- |
| `A1.toml` | `A1` | `standard` | `A1-standard.toml` | 4299 | `0B19FEAA67A77D2F` |
| `A1F.toml` | `A1` | `fast` | `A1-fast.toml` | 4297 | `522EB884C820BCAE` |
| `A1F_260628.toml` | `A1` | `fastv3.3` | `A1-fastv3.3.toml` | 4300 | `3426FD38DDC1389F` |
| `A1M.toml` | `A1_MINI` | `standard` | `A1_MINI-standard.toml` | 4306 | `26D1AAD549BC76C6` |
| `A1MF.toml` | `A1_MINI` | `fast` | `A1_MINI-fast.toml` | 4300 | `115E061F72014F76` |
| `A1MF_260628.toml` | `A1_MINI` | `fastv3.3` | `A1_MINI-fastv3.3.toml` | 4304 | `F4E51D8F889637E8` |
| `P1.toml` | `P1S` | `lite` | `P1S-lite.toml` | 4362 | `BED23FB6B45E680A` |
| `P2.toml` | `P2S` | `standard` | `P2S-standard.toml` | 4366 | `DF96BDA82AA41FDB` |
| `X1.toml` | `X1C` | `lite` | `X1C-lite.toml` | 4362 | `17CA04C3C51035B3` |

**这九对是同一批文件的两个名字。** 两侧（`crates/postprocess/tests/fixtures/presets/`
的旧名 vs `crates/preset/assets/presets/` 的新名，以及旧仓 `mkp-ssr` 那份）**逐字节相同**：
sha256 九对全部相等、字节数逐对相等，**连 `release_time` 都不差**（配方顶层写死了时间戳，
正是为了让"重新生成逐字节相同"成立）。

所以这次迁移**不改任何一个参数值**，只改名字。硬防线（九份产物 sha256 不变）在任何一步
都不许让路：`gen-presets --check`（产物 vs 配方）与 `gen-presets --baseline`（产物 vs 基线）
就是用它守的。

## 2. 与 `key.rs` 的四元组核对（4.2）

`crates/preset/tests/key.rs:32-57` 的 `klc0_every_fixture_has_the_key_we_expect` 里有一张
**硬编码的四元组表**（旧名、机型 id、版本 id、新文件名），九条。逐条与本表对照：

- 九条的**顺序与内容完全一致**（同一份数据在代码里被钉了一次，图表各一份）；
- 那条判据不只读表：它取 `fixtures().len() == 9`（反空转），再逐份
  ① 按旧名找到文件，② 用 `PresetKey::of(read_preset_from_bytes(…))` 读出身份，
  ③ 断身份与四元组一致，④ 断 `key.file_name()` 与四元组里的新名一致；
- 也就是说：**表要是抄错一个字，这条判据会红**，而它的红指向的是"钥匙或文件名不对"。
  本表因此不是唯一一份映射，但它与那份是同一份 —— 改一处必须改另一处，
  这种"两处真相"的代价在 Task 5 之后会随旧名一起消失（那时只剩身份与算法）。

## 3. B 套名字怎么读（4.3，只作历史解释）

| 记号 | 读作 | 例 |
| --- | --- | --- |
| 裸机器代号 | 该机的**标准版**（`standard`） | `A1.toml`、`P2.toml` |
| `M` | **mini** —— 机型档位，不是版本 | `A1M` = `A1_MINI`；但 `P1`/`P2`/`X1` 里的数字不是 |
| `F` | **快拆**（fast）—— 版本档位 | `A1F` = A1 的 `fast` |
| `_260628` | **开源版日期**（2026-06-28）—— 那一版对应的开源版本 | `A1F_260628` = A1 的 `fastv3.3` |
| 组合 | 机器代号 + 版本档位 + 日期，**按位置拼** | `A1MF_260628` = A1_MINI 的 `fastv3.3` |

两条要点：

1. **B 套不可算。** 看到 `A1MF_260628` 推不出"它是 `A1_MINI` 的 `fastv3.3`" ——
   `M` 是机型档位、`F` 是版本档位、日期后缀还要查对照表才知道对应哪个版本 id。
   解码要么带一张表、要么靠文件头的 `# machine:` / `# variant:`（`pair_by_head` 走的就是后一条路）。
   这正是弃用它的原因：**文件名不再承担业务身份**（`docs/ARCHITECTURE.md` §10.1）。
2. **日期后缀不是版本语义。** `_260628` 说的是"那一版开源版是哪天"，不是"这是第几个版本"。
   新规则里版本由 id 表达（`fastv3.3`），日期不进文件名。

## 4. B 套名字今天还留在哪（4.4 的收尾清单）

| 位置 | 现状 | 何时消失 |
| --- | --- | --- |
| `crates/postprocess/tests/fixtures/presets/*.toml`（9 份） | 基线夹具仍用旧名 | Task 5.3 改名 |
| `crates/postprocess/tests/fixtures/ir/build9/*.json`（9 份） | 派生 IR 夹具沿用旧名 | Task 5.4 改名 |
| `presets/machines/*.toml` 的 `presetFile` | 悬空的旧名字面量 | G-2 → Task 16.3 删除 |
| 四处硬编码名单 | `pipeline_wiping_source.rs`、`build9.rs`、`registry_branch_diff.rs`、`crates/preset/src/build.rs` | Task 5.5 更新 |
| `src/api/mock.ts`、`src/app/constants/postProcess.ts` | 旧前端的演示数据与示例命令行 | 随旧前端退役；**不是**本轮管辖范围 |

**迁移完成的判据**：上方五类里前四类都变成新名或消失，且九份产物的 sha256 与 doc §2.4
的表仍然一致。到那时这份文档归档，标记失效 —— 不删，因为它是"为什么曾经有 B 套名字"的
唯一一份解释。
