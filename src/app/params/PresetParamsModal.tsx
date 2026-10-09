/*
 * **右键某一份预设 → 打开它的参数**（2026-10-09）。
 *
 * # 它与「参数」页签是同一个编辑器
 *
 * 里面就是 `PageParams`（`embedded` 那一档）—— 分类胶囊、参数卡、G-code 块、
 * 擦料塔画布、修改历史、撤销重做、草稿链**一件不少、一个字节不改**。
 * 这一层只做两件事：
 *
 *   · 把它装进一个**撑满内容区**的框（`size="full"` + `bodyFill`：从工具条一直到页脚，
 *     滚动与底栏归它自己管）—— 挂载点在 `PagePresets` 的 `.page` 那一层（定位祖先），
 *     `inset: 0` 正好是"顶栏页签以下整块"；
 *   · **钉死编辑对象**——用户右键的是哪一份，改的就是哪一份（`target` 传下去）。
 *
 * 「钉死」很要紧：整页那一版按「机型 · 版本」挑编辑目标（同一组合用户线优先），
 * 而这里用户明确点的是**这一行**：两份都挂在同一组合下时，只有这一条路分得清他要改谁。
 *
 * # 为什么是模态框而不是又一个页签
 *
 * 参数是**某一份预设的属性**，不是一块常驻的工作台 —— 没有"当前正在编辑哪一份"这个
 * 悬着的问题，也就不会出现"我到底在改谁"。关掉就回到他刚才看的那张表。
 */

import { Modal } from '../../components/modal'
import type { Density } from '../../hooks/useDensity'
import PageParams from './PageParams'

interface Props {
  open: boolean
  density: Density
  /** 改哪一份（`path` 是相对用户根的 `presets-mine/…`） */
  target: { path: string; fileName: string; machineId: string; versionId: string }
  /** 一进来就把「选择要跟随的更新」那一屏打开（右键菜单那条直路用） */
  openSyncOnStart?: boolean
  host?: HTMLElement | null
  onClose: () => void
}

export default function PresetParamsModal({
  open,
  density,
  target,
  openSyncOnStart = false,
  host = null,
  onClose,
}: Props) {
  return (
    <Modal
      open={open}
      title={target.fileName}
      subtitle="这一份就是你的预设 —— 改的是它自己；官方那一版只用来标明默认值与有没有更新"
      size="full"
      bodyFill
      host={host}
      /* 框里有没保存的改动（草稿在盘上）—— 手滑点一下遮罩不该把关掉变成丢东西 */
      closeOnScrim={false}
      closeTitle="关掉（没保存的改动还在草稿里，回来点「改这份」接着改）"
      onClose={onClose}
    >
      {/*
       * `key` 钉住这一份：换一份（或关掉再打开另一份）就整块重挂 ——
       * 草稿链、官方更新账、分类胶囊与搜索全部从这一份重新开始，不留上一条的残留。
       */}
      <PageParams
        key={target.path}
        density={density}
        embedded
        target={target}
        openSyncOnStart={openSyncOnStart}
      />
    </Modal>
  )
}
