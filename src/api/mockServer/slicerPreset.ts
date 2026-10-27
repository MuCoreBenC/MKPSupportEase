/*
 * 演示用的**切片器配置正文**（`bbs_config` 那几份 `MKPProcess …json`）。
 *
 * # 为什么要在这儿造一份
 *
 * 真机上这几份的正文躺在下载区（`<appDataDir>/assets/bbs/Process/…`），读它要走 IPC。
 * 浏览器预览里**没有盘**，`readDownloadedText` 原来一律如实抛 —— 那条口径本身是对的
 * （MKP 的 toml 到现在也还是抛，见 `mock.ts` 那一段）。但工艺页左侧**默认就停在
 * 「MKP 工艺」那一档**，那一档不给正文就是个空壳：点开什么都没有、继承链第一步就断，
 * 作者要的「左侧默认先显示我们自己的那几份」在预览里根本看不见。
 *
 * 所以这里只给**切片器配置这一类**造正文，形状照真机那份
 * （`presets/assets/bbs/Process/0.4mm/MKPProcess A1 0.4 0.20.json`：
 * `name` / `inherits` / `from` / `print_extruder_variant` / `print_extruder_id` …），
 * `inherits` 指到 BBL 的标准档 —— 于是那一页能顺着它解析出继承链与基准，
 * 「改动 N 项」是**真算出来的**，不是编的一个数。
 *
 * ⚠️ 那些父档要能在**同一份预览**的清单里查到：浏览器预览里清单来自
 * `mockServer/bbsLocal.ts` 的演示集合（`0.20mm Standard @BBL A1` 那几份），
 * 这里 `BASE` 指的档名与它对齐。**往 `BASE` 里加机型时，那边也要跟着加一份**
 * （否则预览里那一份的父档会「不在清单里」—— 界面照实说，但看着像坏了）。
 *
 * # 与 MKP 的 toml 为什么区别对待
 *
 * toml 是"云端原件"，预览里没有就是没有 —— 照旧抛 NotImplementedError，这条线不放松：
 * 能造的是「这份 JSON 长什么样」（形状是死的、有依据），不能造的是「云端发过这份字节」。
 */

/**
 * 机型 + 喷嘴 → 继承的那份 BBL 标准档。
 *
 * 键是**文件名里那两个字段**（`MKPProcess <机型> <喷嘴> <层高>.json`），
 * 值是 BBS 系统预设里的档名。真机那九份的 `inherits` 就是这么指的
 * （0.2 喷嘴的走 `0.10mm Standard @BBL X 0.2 nozzle`，0.4 的走 `0.20mm Standard @BBL X`），
 * 所以预览里的继承链与桌面版一致。
 *
 * 这些档在装了 BBS 的机器上都有（系统预设覆盖全部机型）—— 万一某台机器的
 * 系统目录不全，BBS 页会照实给一句「父预设不在清单里」，不假装算得出来。
 */
const BASE: Record<string, string> = {
  'A1|0.2': '0.10mm Standard @BBL A1 0.2 nozzle',
  'A1|0.4': '0.20mm Standard @BBL A1',
  /*
   * ★ A1 mini 在 BBS 的档名里是**简称 `A1M`**（`0.20mm Standard @BBL A1M`），
   *   不是 `@BBL A1 mini`。第一版这里写的就是全名，于是那两份怎么也认不出机型
   *   （档名对不上 → 继承链断在第一步 → 机型号成 `unknown`）。作者截图里那句
   *   `0.20mm Standard @BBL A1M - 拷贝 111` 就是这一条的依据。
   */
  'A1 mini|0.2': '0.10mm Standard @BBL A1M 0.2 nozzle',
  'A1 mini|0.4': '0.20mm Standard @BBL A1M',
  'P1S|0.2': '0.10mm Standard @BBL P1S 0.2 nozzle',
  'P1S|0.4': '0.20mm Standard @BBL P1S',
  'P2S|0.2': '0.10mm Standard @BBL P2S 0.2 nozzle',
  'P2S|0.4': '0.20mm Standard @BBL P2S',
  'X1|0.2': '0.10mm Standard @BBL X1C 0.2 nozzle',
  'X1|0.4': '0.20mm Standard @BBL X1C',
}

/**
 * 文件名 → 一份工艺预设的正文。**认不出就返回 `null`**（调用方照旧如实抛）。
 *
 * 「认不出」包含两种情况：不是 `MKPProcess …` 的名字，或者机型/喷嘴的组合同 `BASE` 对不上
 * （目录里以后加了别的机型，这里没跟上）—— 两种都该走"没有字节"那条路，不许瞎造一份。
 */
export function slicerPresetText(fileName: string): string | null {
  /* 机型那一段用**非贪婪**：`MKPProcess A1 mini 0.2 0.10.json` 要切出 `A1 mini`，
     贪婪的 `.+` 会把后面两个数字也吞进去（`A1 mini 0.2`），BASE 就查不到了 */
  const m = /^MKPProcess (.+?) ([0-9](?:\.[0-9])?) ([0-9](?:\.[0-9]+)?)\.json$/.exec(fileName)
  if (m === null) return null
  const [, machine, nozzle, layer] = m
  const inherits = BASE[`${machine}|${nozzle}`]
  if (inherits === undefined) return null

  /*
   * 一项真预设是**几十个 key**，第一版这里只给了四五个 —— 于是界面照着那条
   * 「只识别到 N 个工艺参数，确认这是一份工艺预设？」的判据报了警（作者截图里那行橙字）。
   * 现在按 BBS 参数表里真实存在的几组铺开（层高 / 线宽 / 速度 / 外壳与填充），
   * 键名逐个对着 `public/bbs/registry.json` 核过（编一个表里没有的键，界面会改口说
   * 「有 N 项参数表里没登记」，同样是假警报）。
   *
   * 与基线不同的那几项是真的"MKP 自己的口径"（墙多一道、填充密一点、外壁慢一点），
   * 所以「改动 N 项」在预览里也是算出来的，不是随手填的数字。
   * 层高不进这里：档位名就是层高，写进去只会与基线相同。
   */
  const doc = {
    name: `MKPProcess ${machine} ${nozzle}`,
    inherits,
    from: 'User',
    instantiation: 'true',
    version: '2.0.0.83',
    print_settings_id: `MKPProcess ${machine} ${nozzle}`,
    print_extruder_variant: ['Direct Drive Standard'],
    print_extruder_id: ['1'],
    /* 层高 */
    layer_height: layer,
    initial_layer_print_height: layer,
    /* 线宽 */
    line_width: '0.42',
    initial_layer_line_width: '0.5',
    outer_wall_line_width: '0.42',
    inner_wall_line_width: '0.45',
    top_surface_line_width: '0.42',
    sparse_infill_line_width: '0.45',
    internal_solid_infill_line_width: '0.45',
    support_line_width: '0.42',
    /* 速度 */
    initial_layer_speed: '60',
    outer_wall_speed: '150',
    inner_wall_speed: '200',
    sparse_infill_speed: '180',
    top_surface_speed: '150',
    travel_speed: '500',
    /* 外壳与填充：MKP 自己的口径（基线通常是 2 道墙 / 15%） */
    wall_loops: '3',
    sparse_infill_density: '25%',
  }
  return JSON.stringify(doc, null, 2)
}
