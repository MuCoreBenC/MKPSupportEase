/*
 * **最低客户端版本的自动判断**（工作台 · 自试验场 C15 的 `store/compat.ts` 移植）。
 *
 * # 这份「兼容性清单」是什么
 *
 * 作者的动机：minClientVersion 一直是手填的，而发布检查自己的旧文案就写着
 * 「要客户端先给一份兼容性清单，工作台不能瞎填」—— 这份清单与自动判断就是那件事的落地。
 * 清单的语义：**客户端团队维护**的一张表，说清包里的每一样「结构特性」是从哪个客户端版本
 * 起被支持的。工作台拿当前这份包去扫描，命中的特性里取**最高**的那个 since =
 * 这份包的最低客户端版本（木桶原理）。
 *
 * # 演示口径（诚实声明，与试验场同一份）
 *
 *   · 这张表按演示数据编（真实世界里由客户端发布、工作台同步）；版本轴与测试端一致
 *     （`0.9.0 / 1.0.0 / 1.1.0 / 2.0.0`）；
 *   · `1.0.0` 首发客户端就会读：基础结构（机型 / 版本 / 字段 / 值）与条件显隐；
 *   · `1.1.0` 契约后来补的四样：tabId / uiComponent / files / optionalFiles；
 *   · `2.0.0` 下一代结构（schemaVersion ≠ 1）—— 这一版还到不了。
 *
 * # 为什么检测器跟表住一起
 *
 * 每一行自带 `detect`，判据**只看包本身**（`ClientDataPackage` 的形状）。
 * 生成侧读的是「已保存」的那一份，两边同口径 —— 不许拿工作态的编辑中间态来算。
 * 表与算法同居一个文件：将来清单演进（客户端发新版本）只动这张表。
 *
 * # 这本账（产品仓的纪律）
 *
 * 「前端不算业务」是产品仓的既定纪律（14b 把 C14 `derive` 的判定整批搬进了 Rust）。
 * 这个判定同属那一类，**归宿是 Rust**（清单与 `words.rs` 词表同源）；
 * 本轮只做前端，所以先住在这儿，作为已登记的债 —— 见 `C15-A40-PORT-PLAN.md` §6「仍待定」。
 * 搬过去那天，界面上「自动判断」「结论那一句」两处不动，换的是它们后面那个人。
 *
 * 类型只从契约 `import type` 进来（编译期擦除）—— 工作台不因此把客户端的
 * api / bridge / mock 拖进自己的 bundle。
 */
import type { ClientDataPackage } from '../api/contract'

/** 清单里的一行：一样特性、它的支持门槛、以及「包里有没有用到它」的判据 */
export interface CompatFeature {
  id: string
  /** 从哪个客户端版本起支持（x.y.z） */
  since: string
  /** 给人看的名字（title / toast 的明细里用） */
  label: string
  /** 这份包用到了吗 */
  detect: (pkg: ClientDataPackage) => boolean
}

/**
 * 客户端兼容性清单（演示版）。**顺序随便写** —— 算法自己按版本取最大。
 *
 * `detect` 只看包：字段表 / 机型版本 / 可选文件 / 结构版本。四个判据分别对应
 * 生成说明书时那几段「后加的栏」。
 */
export const CLIENT_COMPAT: CompatFeature[] = [
  {
    id: 'baseline',
    since: '1.0.0',
    label: '基础包结构（机型 / 版本 / 字段 / 值）',
    detect: () => true,
  },
  {
    id: 'showWhen',
    since: '1.0.0',
    label: '条件显隐（showWhen）',
    detect: (p) => p.fields.some((f) => f.showWhen !== undefined),
  },
  {
    id: 'tabId',
    since: '1.1.0',
    label: '英文页签 id（tabId）',
    detect: (p) => p.fields.some((f) => f.tabId !== undefined),
  },
  {
    id: 'uiComponent',
    since: '1.1.0',
    label: '控件原词（uiComponent）',
    detect: (p) => p.fields.some((f) => f.uiComponent !== undefined),
  },
  {
    id: 'files',
    since: '1.1.0',
    label: '版本文件清单（files）',
    detect: (p) => p.machines.some((m) => m.versions.some((v) => v.files.length > 0)),
  },
  {
    id: 'optionalFiles',
    since: '1.1.0',
    label: '柜台上单卖的（optionalFiles）',
    detect: (p) => p.optionalFiles.length > 0,
  },
  {
    id: 'schema2',
    since: '2.0.0',
    label: '下一代结构（schemaVersion 2）',
    detect: (p) => p.meta.schemaVersion !== 1,
  },
]

/** 自动判断的结论 */
export interface MinClientVerdict {
  /** 最低客户端版本（x.y.z） */
  version: string
  /** 命中的特性，按 since 升序 —— 这就是「为什么是这个数」 */
  hits: CompatFeature[]
}

/** 三段数字比大小（与测试端同一个口径）。看不懂的值退回字符串序，排前面 */
function cmpVer(a: string, b: string): number {
  const pa = /^(\d+)\.(\d+)\.(\d+)$/.exec(a)
  const pb = /^(\d+)\.(\d+)\.(\d+)$/.exec(b)
  if (pa === null || pb === null) return a.localeCompare(b)
  for (let i = 1; i <= 3; i += 1) {
    const x = Number(pa[i])
    const y = Number(pb[i])
    if (x !== y) return x - y
  }
  return 0
}

/**
 * **扫描一份包，算出它的最低客户端版本。**
 *
 * 命中的特性里取 since 最高的那个 —— 客户端要同时会读包里用到的每一样东西，
 * 「最低能用的版本」由要求最高的那一样说了算（木桶原理）。
 */
export function minClientOf(pkg: ClientDataPackage): MinClientVerdict {
  const hits = CLIENT_COMPAT.filter((f) => f.detect(pkg)).sort((a, b) => cmpVer(a.since, b.since))
  /* baseline 恒命中，hits 至少一条；真被清空了也退回 1.0.0（首发版本），不给空串 */
  const top = hits[hits.length - 1]
  return { version: top?.since ?? '1.0.0', hits }
}

/**
 * 结论的一句人话（title / toast 的全账）：
 * `1.1.0 —— 基础包结构（1.0.0 起）；条件显隐（1.0.0 起）；英文页签 id（1.1.0 起）；…`
 */
export function verdictTextOf(v: MinClientVerdict): string {
  return `${v.version} —— ${v.hits.map((h) => `${h.label}（${h.since} 起）`).join('；')}`
}
