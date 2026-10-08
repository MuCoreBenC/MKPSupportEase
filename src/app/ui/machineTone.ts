/**
 * 机型的**识别色**（2026-10-08）。
 *
 * # 为什么要有它
 *
 * 作者原话：「A1 和 A1 mini 长得这么像，我很讨厌这种感觉……我就是说这个客户端
 * 的列表这里面容易看错，A1 和 mini 长这么像，终于我一直在犯错」——
 * 预设表里「机型」那一列原来是一串灰字（`A1` / `A1 mini` 只差四个字母），
 * 「名称」列 `A1-fast.toml` / `A1_MINI-fast.toml` 也只差一段；他在真机上把 A1 那行
 * 看成了 A1 mini，点了下载才发现。
 *
 * **只靠读字分不开的东西，得靠颜色分**：一眼扫过去，颜色先于文字被认出来。
 *
 * # 为什么按「机型」上色，而不是按「系列」
 *
 * A1 与 A1 mini 同属一条产品线 —— 按系列上色正好把它俩染成同一个颜色，
 * 而那恰恰是唯一要分开的一对。所以**一色一机型**。
 *
 * # 规矩
 *
 * - 颜色**写死在这张表里**，不按 id 哈希生成：哈希出来的色会随机型增删而变，
 *   而且不可读（一眼看过去是"随便挑的"，人记不住、下次还认不出来）；
 * - 借用现有的三档语义色手感（蓝 / 紫 / 青 / 橙 / 玫瑰 / 绿），但**只当描边 + 浅底**用 ——
 *   与"可下载"那种实心蓝徽章、与"已下载"的绿字仍然分得开，各说各的；
 * - **认不出的机型 → 中性灰**（`NEUTRAL`），绝不借用别人的颜色：
 *   上游新加一台机时借色，就等于给它发了一张别人的身份证，那比没有颜色更坏。
 */

/** 一份色：`ink` 是字色与描边，`bg` 是浅底 */
export interface MachineTone {
  ink: string
  bg: string
}

/**
 * 一色一机型。**改这里就是改全客户端的机型识别色**（表格、以及将来别处要用它的地方）。
 *
 * 挑色的两条约束：① A1 与 A1 mini 要差得最开（蓝 / 紫）；② 与"状态色"别撞手感 ——
 * 实心蓝表示"可下载"、绿表示"已下载 / 生效中"，所以这里的绿给的是最深的一档，
 * 而且一律只做描边。
 */
const TONES: Record<string, MachineTone> = {
  /* 蓝 —— 与 A1 mini 的紫差得最开 */
  A1: { ink: '#1d4ed8', bg: 'rgb(37 99 235 / 10%)' },
  /* 紫（与参数页 `--axis-z` 同一个色系：用户已经把这个紫认成"第三个东西"） */
  A1_MINI: { ink: '#7c3aed', bg: 'rgb(139 92 246 / 12%)' },
  A2L: { ink: '#0f766e', bg: 'rgb(13 148 136 / 12%)' },
  P1S: { ink: '#b45309', bg: 'rgb(217 119 6 / 12%)' },
  P2S: { ink: '#be123c', bg: 'rgb(225 29 72 / 10%)' },
  X1C: { ink: '#15803d', bg: 'rgb(22 163 74 / 12%)' },
}

/** 认不出的机型：中性灰。**不借色**（理由见模块头） */
const NEUTRAL: MachineTone = { ink: 'var(--text-2)', bg: 'var(--surface-sunken)' }

/** 这台机型该用哪一色；认不出就是中性灰 */
export function machineToneOf(machineId: string | null | undefined): MachineTone {
  if (machineId === null || machineId === undefined || machineId === '') return NEUTRAL
  return TONES[machineId] ?? NEUTRAL
}

/**
 * 一行该按哪台机型上色。
 *
 * ★ **「我的文件」那一行要按文件自己标的归属**（`ownMachineId`，来自文件头
 * `# machine:` 那两行），不是按当前筛选档（行上的 `machineId` 在"全部机型"那一档下
 * 是空的）。用筛选档上色会出现"筛 A1 时 mini 那份也染成蓝的"—— 那是更大的错。
 */
export function machineToneKeyOf(row: {
  machineId: string
  ownMachineId?: string | null
}): string {
  return row.ownMachineId ?? row.machineId
}
