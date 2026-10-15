/*
 * 把生成的板子坐标画成 SVG。
 * `src/calib/*.generated` 那两份坐标数据**不复制**：它是共享资产（263 KB），照旧直接 import。
 */

import type { CSSProperties } from 'react'
import * as precise from '../../calib/precise-calibration.generated'
import * as zoffset from '../../calib/zoffset-calibration.generated'
import s from './CalibPlate.module.css'

const PLATES = { zoffset, precise }

export type PlateModel = keyof typeof PLATES

/*
 * 探索仓的口味修正 —— 产物（`src/calib/*.generated.ts`）是工作台生成的共享资产，
 * 不动它；要改的颜色一律覆盖在导入这一层。这个口子就是为这件事开的。
 *
 * ★ 2026-10-13（作者：「绿色的选中的方块，有点不好看，你看看怎么加一点渐变」）——
 * 选中那一格从**平涂纯色**改成**一条浅渐变**：
 *
 *   平涂的绿在这套界面里读作"一个色块"，没有体积；实物上那格是贴片（微微鼓起、
 *   上面受光），所以给它一条三档渐变：一头一档亮、中间回到本色、另一头一档暗。
 *   三档都走配色变量（`--calib-sel-top` / `--calib-sel-fill` /
 *   `--calib-sel-bottom`），**两套主题各自定值**；两个新变量没给时整条退化成纯色
 *   —— 也就是改之前的样子，不会把哪套配色调坏。
 *
 *   渐变本体是 `<defs>` 里的一条 `<linearGradient>`（见下面组件），单位是
 *   `objectBoundingBox`：每条 path 各自铺满自己那一块，XY 板那些 0.5mm 的线
 *   也吃得到（横线只有 0.5mm 高，那点明暗刚好是"一根有厚度的线"）。
 *
 *   ※ 这一轮的渐变**在第四轮删掉了**（作者：「这个选中的就不要渐变色了吧」）——
 *     这段只留着当来路，别再照着加回去。
 *
 * ★ 2026-10-13（作者：「都不好呀，算了，去掉吧，去掉这些乱七八糟的效果了，
 *   就用最纯粹的……而且是左上和右下的渐变，不是上和下」）——
 *   · 选中格上那层"水"（圆角轮廓 + 球面 + 里侧暗 + 外沿亮线 + 两颗高光）**整层删掉**，
 *     连同 `--calib-water-*` 一整套变量；`lab/water-angle/` 那六套方案到此为止。
 *   · 这一格只留**一条渐变**，轴从"上→下"改成 **"左上→右下"**
 *     （`x1/y1 = 0,0` → `x2/y2 = 1,1`）。
 *
 * ★ 2026-10-13 第三轮（作者：「我知道了，我们直接去掉悬停算了」「这个中性灰很明显，
 *   它就比别的少了一圈。你可以自己去量像素」「不要悬停的了」）—— 两件事：
 *
 *   ① **选中那一格换成中性灰**（绿被否；抽卡页 `lab/pick-ink/` 里挑的就是那一档）：
 *      浅色 `#48484a` / 深色 `#98989d`，对角渐变的三档跟着换成灰 —— **结构一点没动**，
 *      还是"一条左上→右下 + 一圈边"。
 *   ② **悬停整条删掉**：`--calib-hover-*`、`data-hover`、指针进出那两个监听、
 *      CSS 里那两条规则全删。两条原因，都记着别再捡回来：
 *      · 浅色下悬停是**一整块近黑** `#48484a`，比"选中"还重 —— 层级是反的
 *        （"指针停在这"不该压过"已确认"）；
 *      · 中性灰的选中色**正是** `#48484a`，和悬停撞成同一个值，两个状态分不开。
 *      指针划过仍有反馈：命中层 `.hit` 是 `cursor: pointer`。
 *
 *   ★ **"少一圈"那件事（记下来，别再犯）**：选中格那圈**描边色必须能在板面上看见**。
 *     量过了：产物里 11 个格子全是 10×10mm、间距 1mm、y 都是 9.118 —— **几何一模一样**；
 *     `stroke-width: 0.18` 是居中描边（每边外扩 0.09）。邻格的边 `#aeaeb2` 在浅色板面
 *     `#e6e6e6` 上看得见 ⇒ 邻格视觉 10.18mm；选中格那圈一度改成**白色**，白边压在
 *     `#e6e6e6` 上等于没有 ⇒ 只剩 10.0mm，于是"少了一圈"。
 *     **浅色用 `#aeaeb2`（与邻格同色）、深色用 `#c7c7cc`**，两个主题都对得上。
 *
 * ★ 2026-10-13 第四轮（作者：「那这个就不用渐变色了吧，还要渐变吗？这个选中的就不要渐变色了吧」
 *   「还有 XY 的也把它做了呀，XY 的你看看用什么颜色，反正去掉悬停了，那他也没必要用绿色了」）
 *   —— 两件事：
 *
 *   ① **选中格回到纯色**：`<defs>` 里那条 `<linearGradient>`、`--calib-sel-paint`、
 *      `--calib-sel-top` / `-bottom`、CSS 里 `.selTop / .selMid / .selBottom` 三个色标
 *      一起删掉 —— 现在就 `--calib-sel-fill` 一个值（第二轮那条"左上→右下"作废）。
 *   ② **XY 板也换中性灰**（原来是与 Z 同一族的苹果绿）：浅色 `#48484a`、深色 `#c7c7cc`。
 *      · 深色比 Z 板那一格再亮一档（Z 是 `#98989d`）：XY 要跳出来的是**一条 0.5mm 的线**，
 *        而线自己已经是 `#8e8e93`，只提一档根本看不出来；
 *      · `--calib-sel-width: 0`：0.5mm 的线上再压一条 0.2 的描边，就是"给线加了圈毛边"。
 *
 * ★ 2026-10-13 第五轮（作者：「我觉得描边还是有点发虚……深色模式还可以接受，浅色模式有点发虚。
 *   换个颜色重新试一下」「X、Y 的几乎看不出来，换个颜色太不明显了，现在深色模式的也是啊，
 *   几乎看不出区别」）—— 两个数：
 *
 *   ① **Z 板浅色的选中边 `#aeaeb2` → `#8e8e93`**：0.18mm 的浅灰压在深灰填充（`#48484a`）上，
 *      抗锯齿之后读成"一层薄雾"；落到 systemGray 就实了。深色那边作者说"可以接受"，不动。
 *   ② **XY 板把明度拉到两头**：浅色 `#48484a` → `#1c1c1e`、深色 `#c7c7cc` → `#f2f2f7` ——
 *      同色相只差一档明度，在 0.5mm（屏幕约 3px）的线上根本读不出来，只能靠明度。
 *
 * ★ 2026-10-13 第七轮（作者：「先把 Z 的选好，X、Y 等会再说」）——
 *   Z 板的选中色在 `lab/pick-gray/`（中性色十档）里定稿：**浅色 = 03「轻」**
 *   （`#aeaeb2` + 边 `#6e6e73`，比第四轮那档深灰轻得多）、**深色 = 08「偏重」**
 *   （`#98989d` + 边 `#c7c7cc`，正好就是原来那档，值没动）。
 *   两个号不一样是正常的：浅色那边要"压深"、深色那边要"提亮"，合适的位置本来就不对称。
 *
 *   XY 板两块**暂时保持现状**（同一轮里作者的下一句是「X、Y 等会再说」）：它在真机上
 *   是 0.5mm 的线（屏幕约 3px），**灰阶深浅在那么细的线上本来就读不出来** ——
 *   要解决得换办法（线加粗 / 给它一个不同的标记），不是继续调灰。
 */
const VAR_PATCHES: Record<string, Record<string, string>> = {
  /*
   * ① XY 板整套（2026-10-13，作者：「现在的颜色不好看」→ 抽卡定 **01**，
   *   见 lab/xy-plate/）。
   *
   *   选中那条原来是**饱和红实心** `#ff3b30`。红在这套界面里已经绑死了
   *   「危险 / 失败」（--danger、.btnDanger、后处理失败态），而"点选涂胶最均匀的
   *   那一条"是个中性偏正面的判断 —— 不是报错。每点一次都像在做一件危险的事。
   *   后来换成果绿，第四轮又换**中性灰** —— 作者：「反正去掉悬停了，那他也没必要用绿色了」。
   *
   *   · 选中 → 中性色，**明度拉到两头**（浅色近黑 #1c1c1e / 深色近白 #f2f2f7，见下）
   *   · 22 条线 → 退到 systemGray #8e8e93 —— **这一手最关键**：原来 #b8b8b8 太亮，
   *     22 条一起抢注意力；退一档之后板子安静下来，选中那条才有地方跳。
   *   · 悬停 → **2026-10-13 第三轮整条删掉**（见文件头；原来定的是 03「只提亮、不染色」）
   *
   * ★ 2026-10-13 第九轮**定稿：02「只把 22 条线退掉」**。作者原话：
   *   「02 只把 22 条线退掉……不管了，你改不好了，就用这一个了，把这个做回去」。
   *
   *   ⇒ 选中色**一个字没动**（浅色 #1c1c1e / 深色 #f2f2f7，还是第五轮那条"明度拉到两头"），
   *     只把 22 条**没被选中**的线各再退一档：浅色 #8e8e93 → #c7c7cc、深色 → #4a4a4c。
   *     思路是"让底和其余的线一起让位"，而不是继续在那 2.5px 上折腾选中色。
   *     （深色尤其需要这一手：原来 21 条线是 #8e8e93，比选中那条的近白还抢眼。）
   *
   *   为什么最后没上加宽 —— 抽卡页里试了两种（同色加宽 / 加宽那圈涂板面色的"凿空"），
   *   量完几何就明白这条路在这块板上走不通：
   *   · 11 条横线只在 **x 3.55~20.34**（板子左侧、右端逐条缩短成阶梯），
   *     11 条竖线只在 **x 13.05~53.55 / y 33.5~50**（左下角）—— **两者几乎不相交**，
   *     最近的一处只差 0.22mm（真机 1.1px）；"加宽的线盖住别的线"在这里几乎不存在；
   *   · 于是加宽唯一的效果就是"那条线变粗"，读起来是**浮在上面**
   *     （作者：「它都已经加宽了之后，它都已经压在了那些上面」）。把加宽拆成一条独立的
   *     底层"底衬"重画一遍也没用 —— 无头 Chrome 实测 DOM 层序确实是最底
   *     （plate → selPad → mark → glyph），但**没有可挡的东西**。
   *   整段来路（含两次"沉底"尝试的截图与量到的坐标）记在 `lab/xy-plate/HANDOVER.md`，
   *   别再走一遍。
   */
  'precise/深色': {
    '--calib-plate-fill': '#2c2c2e',
    /* 第九轮定稿 02：22 条没被选中的线从 #8e8e93 退到 #4a4a4c
       （原来比选中那条的近白还抢眼 —— 见上面 ① 那段） */
    '--calib-mark-fill': '#4a4a4c',
    '--calib-glyph-fill': '#c7c7cc',
    /*
     * **明度直接拉到近白**（#f2f2f7）：这块板要跳出来的是**一条 0.5mm 的线**，
     * 而 22 条线全是同一个中灰 #8e8e93 —— 同色相、只差一档明度，在 3px 的线上
     * 根本读不出来（作者第五轮：「X、Y 的几乎看不出来……现在深色模式的也是」）。
     * 干脆不靠色相，把明度拉到两头。
     * 宽度给 0：0.5mm 的线上再压一条 0.2 的描边，读起来就是"给线加了圈毛边"（老账）。
     */
    '--calib-sel-fill': '#f2f2f7',
    '--calib-sel-width': '0',
  },
  'precise/浅色': {
    '--calib-plate-fill': '#e9e9ec',
    /* 同深色那条：第九轮定稿 02，22 条线从 #8e8e93 退到 #c7c7cc */
    '--calib-mark-fill': '#c7c7cc',
    '--calib-glyph-fill': '#3a3a3c',
    /* 同深色那条：明度拉到近黑（板面 #e9e9ec、22 条线 #c7c7cc）；宽度 0 同上 */
    '--calib-sel-fill': '#1c1c1e',
    '--calib-sel-width': '0',
  },

  /*
   * ② Z 板整套（2026-10-13，作者：「那做一下 z 的抽卡」→ 定 **02**，
   *   见 lab/z-plate/）。这里并掉了原先那条 2026-10-12 的单点补丁
   *   （`'zoffset/深色': { '--calib-sel-fill': '#b0b0b5' }` —— 作者当时：
   *   「这个格子的白色我不是很喜欢」，把选中的纯白 #e3e3e3 降成中亮灰）。
   *
   *   **Z 板与 XY 板的问题不是一回事**：
   *   · 它可点的是 **11 个格子**（role `pad`），不是线 —— 一排横过去，
   *     "选中要跳出来"本来就不难，难的是**格子立不立得起来**；
   *   · 深色下 板面 #3b3b3b 与 格子 #3a3a3c **只差 1**，11 个可选格几乎隐形，
   *     全靠 0.18 的描边撑着 ⇒ 格子提到 #4a4a4e 才算"看得见"。
   *
   *   · 格子 → #4a4a4e（+ 描边 #6e6e73）—— 这一手和 XY 那边"线退一档"同等要紧
   *   · 选中 → **2026-10-13 第三轮换成中性灰**（原来是与 XY 同一族的苹果绿）
   *   · 选中格的**描边必须留着**：邻格有边（0.18，画在轮廓之外），
   *     选中格没边的话视觉上会退进去那么多 ⇒ **看起来小一圈**
   *     （作者：「选中的比别的看起来小」「中间的格子描边记得加回来」）。
   *     实测两者几何都是 10×10、屏幕 41.62px —— 一模一样，纯粹是描边在撑。
   *     **第三轮又栽上这上面一次**（描边改成白色 ⇒ 在浅色板面上看不见 ⇒ 又"少一圈"），
   *     所以两套主题的描边现在都挑"在各自板面上看得见的"那个色 —— 见文件头。
   *   · 悬停 → **2026-10-13 第三轮整条删掉**（见文件头）
   */
  'zoffset/深色': {
    /*
     * ★★★ 2026-10-15 第十轮（作者：「这背景11个方块啊，还有选中的那一个亮着的，看着不舒服」）
     *   —— **整套换成「格子沉下去、选中只提一档」，并把这两支灰的蓝去掉。**
     *
     *   老账（作者截图按像素量的实际值）：页底 `#232323`、板面 `#3B3B3B`、
     *   格 `#4A4A4E`、选中 `#98989D`。两条病都在数上：
     *     · 11 个格比页底**亮 39**（74 对 35）⇒ 中间那一片是"一整块中灰墙"，
     *       不是"11 个格子"（作者说的"不舒服"就是它）；
     *     · 选中**是全屏最亮的东西**（152）⇒ 每点一下都像打开一盏灯。
     *   另外这两支灰是**苹果蓝灰**：格 B−R = +4、选中 B−R = +5
     *   （与 tokens 那条"面族去蓝"同源，这里当时漏了）。
     *
     *   新的一档（值出自 `tmp-shots/plate-cards.png` 那次抽卡，作者挑的是"选中仍是一块面"这一路）：
     *     · **格 `#333333`（比板面还暗 8）+ 细边 `#4C4C4C`** ⇒ 11 个格读成"压出来的坑"，
     *       整块板是一个物件，中间不再是一片亮。
     *       ★★ 但**必须仍比页底（`#232323` = 35）亮** —— 值 0 那一格里那个"圆点"
     *       是 pad 路径的**洞**，露的就是页底（见文件头）；压到页底附近 = 把点抹掉。
     *     · **选中 `#5E5E5E` + 边 `#9C9C9C`（0.18）** ⇒ 只提一档：
     *       "选中"改由「比邻格亮 + 一圈边」说，不再是那块 152 的亮板。
     *       ★ 边照旧留着 —— "选中的看起来小一圈"那笔老账还在（见文件头）。
     *     · 两支灰全部 R=G=B，**不带蓝**。
     *
     *   ★ 历史（别捡回来）：`--calib-pad-fill #4a4a4e` / `--calib-pad-stroke #6e6e73`
     *     是 2026-10-13 那几轮"格子立不起来"时的解法（当时板面与格**只差 1**，只能靠提亮）；
     *     选中 `#98989d` + 边 `#c7c7cc` 是第七轮在 `lab/pick-gray/` 十档里挑的 08「偏重」
     *     —— 那是在"格已经提亮过"的前提下挑的，**这一轮把格压下去，那档"偏重"就过亮了**。
     */
    '--calib-pad-fill': '#333333',
    '--calib-pad-stroke': '#4c4c4c',
    '--calib-glyph-fill': '#c7c7cc',
    '--calib-sel-fill': '#5e5e5e',
    '--calib-sel-stroke': '#9c9c9c',
    '--calib-sel-width': '0.18',
  },
  'zoffset/浅色': {
    /*
     * ★★★ 2026-10-15 浅色这一档**改了三版**才落定，三版的来路都记在这儿（别再走一遍）。
     *
     * ── 第一版：格 `#F2F2F4`、选中 `#C7C7CC` + 边 `#8E8E93`
     *    作者：「选中的看起来是凹下去的…因为那个阴影会导致那些没选中的凸起来」。
     *    ★ 板子上**没有任何阴影**（没有 box-shadow 也没有 drop-shadow）—— 他说的"阴影"
     *      其实是**那圈比面更暗的边**。凹凸只由两组明暗关系决定：
     * ```text
     *   面比周围暗 + 边比面暗   ⇒ 读成"凹"（一个坑）
     *   面比周围亮 + 边比面重   ⇒ 读成"凸/浮起来"（一块被点亮的板）
     * ```
     *      那一版两条都占着"凹"（格 242 比页底 251 暗、选中 199 更暗、边 174/142 全比面暗）
     *      ⇒ 一排全是坑、选中是最深的坑。★ 而**深色那版是反的**（面比页底亮、边比面亮）
     *      ⇒ 同一件事在两套主题里方向相反 = 那个 bug 的根。
     *
     * ── 第二版：格 `#ECECEF`、选中 `#FFFFFF` + 边 `#6E6E73`（"选中=最亮那块"）
     *    作者：「现在这种效果我不是很喜欢，颜色搭配得不是很好。这一个没选中的颜色，
     *    感觉这种颜色不好」⇒ **一整条灰 track 被否了**（他那句"再深一点"原来是落在
     *    "这一排别那么灰"上，不是在说选中那块）。
     *
     * ── 第三版（**现在这版**）：**把"深"和"凹凸"分开**——
     *    · **格 `#F7F7F7` + 细边 `#C6C6C6`**：近白纸面、中性（R=G=B，不带蓝）、
     *      不再是一整条灰，靠一圈很轻的边分开；
     *    · **选中 `#3A3A3A` 实心（面与边同色）**：这不是"凹"也不是"凸"，
     *      是**被填实的那一格**（扁平设计里"已选=填满"，像勾选/分页点那样），
     *      也是作者最初那句"选中那个再深一点"的直接答案。
     *      面与边同色 ⇒ 那一格读成一块**实心块**，没有"面/边"两组关系可以被读成凹凸。
     *
     *   ★★ 老约束一条没破：**那圈边不能是白**（浅色板面浅，白边压上去等于没有 ⇒
     *   "少一圈"，第三轮栽过）、**不能和填充太近**（0.18 抗锯齿后读成薄雾 ⇒ "发虚"）。
     *   ★ 值 0 那格那个"圆点"是**洞**（露页底）——作者明确说了"那个是故意的，别管它"。
     *
     *   ★ 历史（别捡回来）：`pad #ffffff` 与选中 `#aeaeb2` + 边 `#6e6e73` 是
     *   2026-10-13 第七轮在 `lab/pick-gray/` 十档里挑的 03「轻」—— 那是**在"格是纯白"
     *   的前提下**挑的"和邻格关系更近"的一档，这一轮整族都换了，那档不再适配。
     */
    '--calib-pad-fill': '#f7f7f7',
    '--calib-pad-stroke': '#c6c6c6',
    '--calib-sel-fill': '#3a3a3a',
    '--calib-sel-stroke': '#3a3a3a',
    '--calib-sel-width': '0.18',
  },
}

interface CalibPlateProps {

  model: PlateModel
  /** 配色套名，省略则用工作台里存下的那一套 */
  theme?: string
  label: string
  /** 热区被点（含键盘 Enter / Space）；不传就是只读 */
  onPick?: (id: string) => void
  /** 当前选中的热区，多选就传多个 id */
  picked?: readonly string[]
  /** 给热区的可读名字，用于 aria-label */
  hitLabel?: (id: string) => string
}

export default function CalibPlate({
  model,
  theme,
  label,
  onPick,
  picked,
  hitLabel,
}: CalibPlateProps) {
  const plate = PLATES[model]
  const vars = {
    ...(plate.THEMES[theme ?? plate.ACTIVE_THEME] ?? plate.THEME),
    ...(VAR_PATCHES[`${model}/${theme ?? plate.ACTIVE_THEME}`] ?? {}),
  } as Record<string, string>
  /* 悬停整条删掉之后，产物里那三个 `--calib-hover-*` 已经没有任何规则吃它 ——
     别把死变量挂在 DOM 上，顺手摘掉 */
  for (const k of Object.keys(vars)) if (k.startsWith('--calib-hover-')) delete vars[k]
  const [x, y, w, h] = plate.VIEW_BOX
  const on = (id: string) => Boolean(picked?.includes(id))

  // 命中层：产物里的热区 bbox 四边各外扩 HIT_GROW —— 那些梯子只有 0.5mm 宽，
  // 靠轮廓本身根本点不中，导出的 SVG 也是另铺一层 .hit 矩形来解决的。
  const useRects = Boolean(onPick) && plate.HIT_SHAPE === 'rect'
  const grow = plate.HIT_GROW

  return (
    <div
      className={s.wrap}
      data-model={plate.MODEL}
      // --plate-mm 是这块板的 mm 宽度：乘上列上给的共用 px/mm，两块板才一样粗
      // （选中那一格的颜色全走配色套的 --calib-sel-*，没有渐变这回事了 —— 见文件头第四轮）
      style={{ ...vars, '--plate-mm': w } as CSSProperties}
    >
      <svg
        className={s.plateSvg}
        viewBox={`${x} ${y} ${w} ${h}`}
        role={onPick ? 'group' : 'img'}
        aria-label={label}
        data-model={plate.MODEL}
        data-unit={plate.UNIT}
      >
        {plate.PATHS.map((path) => {
          // 用命中层时可见层一律不吃指针；退回老路时才让 path 自己可点
          const clickable = Boolean(onPick) && path.hit && !useRects
          return (
            <path
              key={path.id}
              className={s[path.role]}
              d={path.d}
              fillRule="evenodd"
              data-id={path.id}
              data-hit={path.hit}
              data-value={path.value}
              data-on={Boolean(onPick) && path.hit ? on(path.id) : undefined}
              role={clickable ? 'button' : undefined}
              tabIndex={clickable ? 0 : undefined}
              aria-pressed={clickable ? on(path.id) : undefined}
              aria-label={clickable ? (hitLabel?.(path.id) ?? path.label ?? path.id) : undefined}
              onClick={clickable ? () => onPick?.(path.id) : undefined}
              onKeyDown={
                clickable
                  ? (e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        onPick?.(path.id)
                      }
                    }
                  : undefined
              }
            />
          )
        })}
        {plate.TEXTS.map((t) => (
          <text
            key={t.id}
            className={s.glyph}
            x={t.x}
            y={t.y}
            fontFamily={plate.FONT_FAMILY}
            fontWeight={plate.FONT_WEIGHT}
            fontSize={t.fontSize}
            textAnchor={t.anchor}
            textLength={t.textLength}
            transform={t.rotate ? `rotate(${t.rotate} ${t.x} ${t.y})` : undefined}
            data-id={t.id}
          >
            {t.text}
          </text>
        ))}
        {useRects &&
          plate.HOTSPOTS.map((spot) => (
            <rect
              key={spot.id}
              className={s.hit}
              x={spot.bbox[0] - grow}
              y={spot.bbox[1] - grow}
              width={spot.bbox[2] - spot.bbox[0] + grow * 2}
              height={spot.bbox[3] - spot.bbox[1] + grow * 2}
              data-id={spot.id}
              data-hit="true"
              role="button"
              tabIndex={0}
              aria-pressed={on(spot.id)}
              aria-label={hitLabel?.(spot.id) ?? spot.id}
              onClick={() => onPick?.(spot.id)}
              // 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页跟着挪
              onMouseDown={(e) => e.preventDefault()}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  onPick?.(spot.id)
                }
              }}
            />
          ))}
      </svg>
    </div>
  )
}
