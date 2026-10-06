import { useCallback, useRef, useState, type ReactNode } from 'react'

import { FieldLayer } from '../components/field'
import { useDensity } from '../hooks/useDensity'
import { detectPlatform } from '../hooks/usePlatform'
import PagePlaceholder from './components/PagePlaceholder'
import ResizeEdges from './components/ResizeEdges'
import TopTabs from './components/TopTabs'
import { tabs } from './constants/tabs'
import PageBbs from './bbs/PageBbs'
import PageCalib from './calib/PageCalib'
import PageHome from './home/PageHome'
import PageParams from './params/PageParams'
import PagePresets from './presets/PagePresets'
import PageSettings from './settings/PageSettings'
/* 通用文件导入入口（第十二层）：住在 App 层，不属于任何一页 ——
   拖拽进窗口 / 文件选择器都从这里走；预设页只是第一个消费者（见那一页的按钮） */
import { FileImportProvider, ImportBanner } from './import/FileImport'
import { inTauri } from './window'
import s from './App.module.css'

/* 平台只影响标题栏把窗口按钮画在左边还是右边，一次探测就够，不必进 state */
const PLATFORM = detectPlatform()

/**
 * 应用根组件。
 *
 * 试验场那份（AppV023 / AppA40）多两样东西，都是预览器的：`VersionProps`（稿号外壳注入的
 * platform 与 reportMode）与「报告」的全屏子视图开关。产品里窗口就是窗口、只有一个界面，所以：
 * 平台自己探测，报告页按普通页签处理。
 *
 * # 页签的变动
 *
 * P1 起是 8 个（加了「同步」「BBS 预设」）；2026-10-02 作者裁决「同步」**整页退役** ——
 * 普通用户不需要"同步"这个概念（catalog 随包走、更新是内部机制），数据源配置降级成
 * 设置页里的开发后门（见 `settings/PageSettings.tsx`）。现在 7 个。
 *
 * 外壳套上 `FieldLayer` —— 预设页 / 参数页的下拉、浮层、右键菜单全挂在它上面
 * （`src/components/field/` 那一套）。它不产生包裹元素，只在最后多一个绝对定位的层，
 * 所以 `.shell` 的 flex 列布局一个字不用改。
 *
 * 一开始六个页签全是空态（"这一页本版未接入"，不是白屏 —— 少一个页签会让"这一版缺什么"
 * 变得看不见）；P2–P5 把预设 / 参数 / BBS 预设接成真页面，2026-10-02 设置接上最小版
 * （高级设置 → 预设数据源）；报告仍是空态。
 */
/*
 * 页签槽（2026-10-05）：页签从卸载重挂改成**常驻 + 切显示**（作者：「所有页面我都
 * 希望保持我之前点击的状态」）。每页常驻挂一次，槽只切 data-hidden —— 滚动位置、
 * 页内步骤（校准的 Z/XY/测试模型）、输入草稿、抽屉开合全部留住，切回也不重新拉数据。
 * 走 visibility 不走 display：display:none 会把滚动位置一起毁掉。
 * 需要「回到本页时对一次底账」的页面（首页 / 校准）另收 `active` 自己处理。
 */
function PageSlot({ hidden, children }: { hidden: boolean; children: ReactNode }) {
  return (
    <div className={s.pageSlot} data-hidden={hidden || undefined}>
      {children}
    </div>
  )
}

export default function App() {
  const rootRef = useRef<HTMLDivElement>(null)
  const density = useDensity(rootRef)
  const [tab, setTab] = useState('machine')

  /*
   * 预设页右键「在 BBS 预设查看器中打开」要跨页带一个目标过去。
   *
   * 存文件名而不是存整行：BBS 页自己有一份清单，按名字查得到就选中，查不到就说一句 ——
   * 两页的数据源不是同一份（预设页走 `src/api`，BBS 页走本机 BBS 目录），不能互相塞对象。
   * `nonce` 是「同一个文件再点一次也要重新触发」用的，不然第二次点没反应。
   *
   * BBS 页（P5）接上来之后这一格有了读者 —— 它整份传下去，由那一页按文件名在
   * 自己的清单里找（见 `src/app/bbs/PageBbs.tsx` 里那个 effect）。
   */
  const [pendingBbs, setPendingBbs] = useState<{ name: string; nonce: number } | null>(null)

  const openBbs = useCallback((name: string) => {
    setPendingBbs((prev) => ({ name, nonce: (prev?.nonce ?? 0) + 1 }))
    setTab('bbs')
  }, [])

  /*
   * 预设页那句「此预设需要更新版 SupportEase」的「去更新」出口：切到设置页。
   *
   * ★ 它与 `openBbs` 同类 —— **跨页跳转的出口由外壳给**，页面只负责把"我要去"交出去。
   *   为什么不在预设页自己跳：页签状态住在外壳（`tab` 在这儿），页面不该伸手改它。
   */
  const openSettings = useCallback(() => setTab('settings'), [])

  return (
    <div ref={rootRef} className={s.shell} data-density={density}>
      {/* 通用导入入口（第十二层）包在最外层：拖拽事件要落在外壳上、重名那一格要盖全窗 */}
      <FileImportProvider>
        {/* 浮层层要在最外面：它给所有弹出物提供挂载点与坐标基准 */}
        <FieldLayer>
          <TopTabs
            tabs={tabs}
            active={tab}
            onChange={setTab}
            density={density}
            platform={PLATFORM}
            fluid
          />

          {/* 导入结果条（in-flow，标签栏下面一条）：有结果才出现，不是会自己消失的提示 */}
          <ImportBanner />

          <main className={s.body}>
            <PageSlot hidden={tab !== 'machine'}>
              <PageHome density={density} active={tab === 'machine'} />
            </PageSlot>
            <PageSlot hidden={tab !== 'preset'}>
              <PagePresets density={density} onOpenBbs={openBbs} onOpenSettings={openSettings} />
            </PageSlot>
            <PageSlot hidden={tab !== 'calib'}>
              <PageCalib active={tab === 'calib'} />
            </PageSlot>
            <PageSlot hidden={tab !== 'params'}>
              <PageParams density={density} active={tab === 'params'} />
            </PageSlot>
            <PageSlot hidden={tab !== 'bbs'}>
              <PageBbs density={density} pending={pendingBbs} />
            </PageSlot>
            <PageSlot hidden={tab !== 'report'}>
              <PagePlaceholder title="报告" hint="后处理执行报告与历史" />
            </PageSlot>
            <PageSlot hidden={tab !== 'settings'}>
              <PageSettings />
            </PageSlot>
          </main>
        </FieldLayer>
      </FileImportProvider>

      {/* Windows 的 decorations: false 之后系统 resize 边框在可见窗口之外，
          补一圈内侧命中区让抓取带跨在边界上。macOS 不需要——系统管 resize */}
      {inTauri && PLATFORM === 'windows' && <ResizeEdges />}
    </div>
  )
}
