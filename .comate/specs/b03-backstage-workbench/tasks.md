# B03 后厨工作台实施计划

> 顺序即依赖顺序。每个任务结束时工作台都应能启动且不崩，不留"要等下一个任务才能跑"的中间态。
> 任务 1 先做编译隔离，因为它是唯一一旦做错就会把后厨发给用户的环节；后面每个任务都在它的保护下进行。

- [x] Task 1: 工作台骨架与编译隔离
    - 1.1: 新建 `workbench.html` 与 `src\workbench\main.tsx`、`App.tsx`，先只渲染一句占位文本
    - 1.2: 改 `vite.config.ts`，`BUILD_WORKBENCH=1` 时才把 `workbench.html` 加进 `rollupOptions.input`；`server` 三条保持原样不动
    - 1.3: `src-tauri\Cargo.toml` 加 `[features] workbench = []`，默认 features 不含它
    - 1.4: 新建 `src-tauri\src\workbench\mod.rs`，整块挂 `#[cfg(feature = "workbench")]`；`lib.rs` 条件注册
    - 1.5: 新建 `tauri.workbench.conf.json` 声明工作台窗口，默认 `tauri.conf.json` 不含它
    - 1.6: `package.json` 加 `dev:workbench` / `build:workbench` / `tauri:workbench`
    - 1.7: 验收：默认构建产物里 `dist\workbench.html` 不存在，二进制里搜不到工作台命令名；带 feature 构建能开出工作台窗口

- [x] Task 2: 开发源数据骨架与字段定义
    - 2.1: 建 `workbench\` 目录结构（machines / bbs / capability / .draft / .trash / .snapshots）
    - 2.2: 工作台自己生成 `registry.json` 最小字段集（约 10 个：回抽长度、Z 偏移、喷嘴偏移 X/Y、装载/卸载 G-code 等），不从 `mkpse-presets` 搬运
    - 2.3: 生成 `fallback.json` 骨架
    - 2.4: 手工写 2 个机型基底（A1、P1）+ 3 个版本作为可操作样本，其中一个刻意留空以复现"未配置"状态
    - 2.5: `.gitignore` 加 `workbench\.draft\`，其余入库

- [x] Task 3: store.rs —— 读写与原子写
    - 3.1: 实现 `atomic_write`：写临时文件 → `sync_all` → 同盘 `rename`
    - 3.2: 实现机型 / 版本 / registry / fallback 的读取与解析
    - 3.3: 实现 `save_version`、`save_machine_base`，签名上拿不到 `Registry` 的可写引用
    - 3.4: 首次启动若 `workbench\` 为空：只创建空目录骨架 + 提示，**不写入任何配方**
    - 3.5: `registry.json` 损坏时进只读模式并明确报错，不允许在坏定义上继续
    - 3.6: 单元测试：原子写中断后原文件完整；空目录启动不产生配方文件

- [x] Task 4: resolve.rs —— 继承解析与 hash
    - 4.1: 定义 `Machine` / `Version` / `BbsBinding`，键用 `BTreeMap` 保证序列化顺序稳定
    - 4.2: 实现 `resolve`：版本覆盖优先、否则取机型基底、机型上没有则跳过
    - 4.3: 每个值带 `Origin`（Base / Override），供界面标来源
    - 4.4: `Effective.hash` = sha256(canonical_json(registry 指纹 + fallback 指纹 + values))
    - 4.5: 单元测试：字段顺序打乱后 hash 不变；改 registry 指纹后 hash 变；覆盖值与继承值各自的 Origin 正确

- [x] Task 5: 配方编辑页
    - 5.1: `App.tsx` 左侧机型/版本树，右侧工作区，`api.ts` 封装 invoke
    - 5.2: `Recipes.tsx` 按 section 分组渲染字段，按 `order` 排序
    - 5.3: 逐字段显示来源：继承值中性色 +「继承自 A1」，覆盖值高亮 +「本版覆盖」
    - 5.4: 单字段「清除覆盖」回到继承
    - 5.5: 机型基底编辑入口与版本编辑入口在界面上明确分开
    - 5.6: `registry.json` 只读展示：可查看、搜索、排序，无新增/删除/改类型/改区间入口

- [x] Task 6: 草稿（关掉再开还在那个状态）
    - 6.1: 编辑防抖 400ms，原子写 `.draft\{machine}__{version}.json`，含 `baseHash` 与 `savedAt`
    - 6.2: 打开时 `baseHash` 一致 → 恢复未保存状态 + 顶部「有未保存的修改」
    - 6.3: `baseHash` 不一致 → 不静默覆盖，让用户选保留草稿或丢弃
    - 6.4: 保存成功后删除草稿
    - 6.5: 界面上把「草稿已恢复」「配方已保存」「TOML 已生成」三件事分别指示，不合并
    - 6.6: 测试：写草稿后重启工作台，编辑内容仍在

- [x] Task 7: 克隆 / 重命名 / 移动 / 删除
    - 7.1: 克隆弹框预填「XX 副本」，可改名；重名当场提示，不自动加后缀
    - 7.2: 重命名版本（显示名可改，id 变更走同一套重名校验）
    - 7.3: 移动机型：保留覆盖值，移动前预览继承值变化，用橙/黄标「值变了」而非红色
    - 7.4: 移动后新机型上不存在的字段：覆盖值保留但标灰「新机型不适用」，不写进 TOML
    - 7.5: 删除进 `.trash\`，可还原，彻底删二次确认
    - 7.6: 进回收站的版本，其 TOML 立即从菜单中移除
    - 7.7: 并发保护：保存时比对 `baseHash`，不一致则拒绝并提示重新载入

- [x] Task 8: 参数矩阵与批量编辑
    - 8.1: `Matrix.tsx` 行=字段、列=版本，单元格显示值与来源
    - 8.2: 首次打开显示当前机型所有版本；另给「显示全部版本」入口
    - 8.3: 视图状态（筛选 / 展开）存 `.draft\matrix-view.json`，重开恢复
    - 8.4: 批量流程：显式勾选目标列 → 选字段 → 输入新值 → 预览影响范围 → 确认
    - 8.5: 改基底只改基底；改版本值则创建/更新该版本覆盖，不打破继承
    - 8.6: 跨机型批量走同一条显式勾列路径，不提供"一键应用到所有机型"
    - 8.7: 测试：批量改版本值后，未勾选版本的继承关系不变

- [x] Task 9: capability.rs —— 客户端能力定义（只读）
    - 9.1: 加载 `workbench\capability\client-*.json` 与 `support.json`
    - 9.2: 手工放一份对应当前客户端版本的能力定义作为首版输入
    - 9.3: 实现「字段是否被支持 / 值是否在范围与步进内」的校验函数
    - 9.4: 能力定义缺失或解析失败 → 返回明确的"无法校验兼容性"，不返回通过
    - 9.5: 代码路径上不提供任何写入能力定义的入口

- [x] Task 10: generate.rs —— 生成 MKP TOML
    - 10.1: 按 registry 的 `section` / `toml_key` 把有效配方序列化为 TOML
    - 10.2: 产物头部写 `source = "official"` 与 `preset_id`
    - 10.3: 生成前按 `support.json` 里每个在支持期内的客户端版本逐一校验
    - 10.4: 全部支持 / 部分支持 / 全不支持三档处理，算出该产物的 `minClientVersion`；全不支持则阻断
    - 10.5: 写临时目录 → 全部成功 → 整体 rename；任一步失败保留旧 TOML 并记失败原因
    - 10.6: 成功后写 `.snapshots\{machine}__{version}.json`，记这次用的 hash 与时间
    - 10.7: 测试：注入写失败，旧 TOML 字节不变

- [x] Task 11: 生成状态与生成入口
    - 11.1: 实现 `GenState` 四态判定（已生成 / 待生成 / 未配置 / 生成失败），判据是 hash 不是文件时间
    - 11.2: 全局配置（registry / fallback）变更后受影响版本自动转「待生成」
    - 11.3: 每个版本各自显示：上次成功生成时间、有无未生成修改、产物是否对应当前配方
    - 11.4: 状态词用「已生成」，全篇不出现「已同步」
    - 11.5: 「生成待更新项」为默认按钮，「全部重新生成」为次要入口
    - 11.6: 「恢复到上次成功生成时的配方」入口，恢复后仍需显式点生成
    - 11.7: 测试：没改的版本重复点生成，产物字节不变

- [x] Task 12: BBS 收录与三态
    - 12.1: `Resources.tsx` 两栏：TOML 栏与 BBS 栏，不共用编辑流程
    - 12.2: BBS 导入按钮：拷进 `workbench\bbs\`，原样存放不改字节；校验 JSON 可解析
    - 12.3: 机型默认 BBS 清单 + 版本 `Inherit` / `Own` 脱钩，脱钩后完全独立不做增删继承
    - 12.4: 三态标记：已分配 / 可选 / 仅归档
    - 12.5: 未分配 BBS 在清单里常显
    - 12.6: 首版以「导入按钮 + 空清单 + 三态标记」交付，不伪造样本数据

- [x] Task 13: 菜单与 Bundle
    - 13.1: `bundles.json` 编辑：一个 Bundle 含 0..N TOML + 0..N BBS，允许纯 TOML / 纯 BBS
    - 13.2: `catalog.json` 编辑：presetId / displayName / resource / standalone 分开维护
    - 13.3: presetId 唯一性校验；改 id 视作删旧建新并明确警告
    - 13.4: 改 displayName 不触及能力定义、不影响 presetId
    - 13.5: 改 resource 路径时自动更新引用；旧文件无人引用则提示可归档
    - 13.6: 菜单与 Bundle 分两份文件，不合并
    - 13.7: 测试：改显示名与上下架不引起任何 TOML 重新生成

- [x] Task 14: preflight.rs —— 出货检查
    - 14.1: 配方与产物类：未配置阻断、从未成功生成阻断、有未生成修改非阻断
    - 14.2: 引用完整性类：菜单/Bundle 引用不存在的资源、回收站版本仍被引用、presetId 重复 —— 全部阻断
    - 14.3: 兼容性类：能力定义缺失阻断；无任何支持期客户端支持某资源则阻断；部分版本不可用为非阻断并已标 `minClientVersion`
    - 14.4: 资源盘点类：未分配 BBS 一条轻提示，非阻断
    - 14.5: 「无法校验」归为阻断，不归为通过
    - 14.6: 检查结果按阻断 / 非阻断分组展示，阻断项未清空时发布按钮禁用

- [x] Task 15: 发布
    - 15.1: `Release.tsx` 把生成、出货检查、发布三个动作在界面上分开
    - 15.2: 发布对象只含 `presets\*.toml`、`bbs\*.json`、`catalog.json`、`bundles.json`
    - 15.3: 为每个交付文件计算 sha256 与 size 写进 `catalog.json`
    - 15.4: 写临时目录 → 全部成功 → 整体 rename；`catalog.json` 最后一个替换
    - 15.5: 开发配方 JSON 不进发布对象，代码路径上不可能被带出
    - 15.6: 测试：发布过程中断后，`dist-presets\catalog.json` 仍指向上一份完整资源

- [x] Task 16: 整体验收
    - 16.1: 默认构建：`npm run build` + `npm run tauri build`，确认产物无 `workbench.html`、无工作台命令
    - 16.2: 工作台构建：`npm run tauri:workbench build`，确认工作台窗口可用
    - 16.3: `npm run lint` 与 `cargo clippy`（两种 feature 组合各跑一次）通过
    - 16.4: 端到端走一遍：改配方 → 关掉重开（草稿在）→ 保存 → 生成待更新项 → 收录 BBS → 编菜单与套餐 → 出货检查 → 发布
    - 16.5: 复核四个动作在界面上确实分得开：保存 / 生成 / 发布 / 客户端更新
    - 16.6: 清理过程中产生的临时文件与试验数据
