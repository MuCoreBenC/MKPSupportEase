# public/bbs 里这四份 json 的来源与许可

这份目录**不是预设快照**。本仓刻意不打包那 285 个预设文件（`system/` 279 + `samples/` +
`index.json`）—— 预设清单是页面运行时读**本机 BBS 目录**得到的（见 `tools/dev-server/bbsFs.mjs`）。
这里只有**元数据**四份：页面靠它们把一份预设摊成参数表。

| 文件 | 大小 | 装的是什么 |
|---|---|---|
| `registry.json` | 335 KB | 702 条参数的定义（中文标签 / 单位 / 类型 / 枚举选项 / C++ 原类型） |
| `layout.json` | 15 KB | 参数面板的版面：5 页 / 组 / 有序 key |
| `defaults.json` | 29 KB | 出厂默认值（已归一成 BBS json 里的写法） |
| `icons.json` | 136 KB | 分组与原件的 SVG 源码 |

## 出处

由 `machine-motion/tools/bbs-extract.mjs` 从 **Bambu Studio** 提取：

- 参数定义与默认值 ← 源码 `src/libslic3r/PrintConfig.cpp` 的 `set_default_value(...)`
- 面板版面 ← 源码 `src/slicer/GUI/Tab.cpp` 的 `TabPrint::build()`
- 中文标签 ← 源码 `bbl/i18n/zh_CN/BambuStudio_zh_CN.po`
- 图标 ← 安装目录 `resources/images`

本仓这四份是**从试验场 `mkp-adaptive-console@e509255` 的 `public/bbs/` 原样抄来的**
（提取器产物，未做改动），对应的 BBS 版本记在同目录 `_sync.json`。

## 许可

**Bambu Studio 整体是 AGPL-3.0**（[官方对协议的表态](https://blog.bambulab.com/agpl-compliance-of-bambu-studio/)），
本仓也是 **AGPL-3.0**（见仓库根的 `LICENSE`）。同一个协议，所以这批派生数据**可以随本仓分发** ——
条件是保留来源与许可声明，这份文件就是那条声明。

两条要注意的：

1. 提取物只来自**开源那部分**（`PrintConfig.cpp` / `Tab.cpp` / `.po` / `resources/images`）。
   Bambu Studio 那个对接自家云服务的网络插件是独立分发的闭源件，我们一个字都没碰。
2. 将来若把本仓改成**闭源**分发，这条路要重做：那时不能再随包带这四份，得改成运行时
   从本机取、或自研一份等价表。**在 AGPL 之下不存在这个问题。**