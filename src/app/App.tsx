import { useCallback, useRef, useState, type ReactNode } from 'react'

import { FieldLayer } from '../components/field'
import { useDensity } from '../hooks/useDensity'
import { detectPlatform } from '../hooks/usePlatform'
import PageReport from './report/PageReport'
import ResizeEdges from './components/ResizeEdges'
import TopTabs from './components/TopTabs'
import { tabs } from './constants/tabs'
import PageBbs from './bbs/PageBbs'
import PageCalib from './calib/PageCalib'
import PageHome from './home/PageHome'
import PagePresets from './presets/PagePresets'
import PageSettings from './settings/PageSettings'
/* 通用文件导入入口（第十二层）：住在 App 层，不属于任何一页 ——
   拖拽进窗口 / 文件选择器都从这里走；预设页只是第一个消费者（见那一页的按钮） */
import { FileImportProvider, ImportBanner } from './import/FileImport'
/* 钩子那一趟的那一屏（切片器导出 G-code 时，本程序被带参数拉起来的那一次）：
   普通模式里它读到 `null`，自己渲染空 —— 所以这里无条件挂一个就够。
   ★ 那一趟的状态**在 App 这一层读一次**（`usePostProcessRun`），同时喂给两处：
     那一屏模态框 + 后台那条窄条（「后台运行」收起来之后总得有个入口回得去）。 */
import PostProcessModal from './postprocess/PostProcessModal'
import PostProcessStrip from './postprocess/PostProcessStrip'
import { usePostProcessRun } from './postprocess/usePostProcessRun'
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
 * 设置页里的开发后门（见 `settings/PageSettings.tsx`）。2026-10-09「参数」也整页退场
 * （参数从预设那一行右键「打开参数」进去，是同一份 `PageParams`，见 `constants/tabs.ts`）。
 * 现在 6 个。
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
 * （"回到本页对一次底账"那套 2026-10-06 随 AppState 退役 —— 页面订阅
 * `src/app/state/appState.ts`，底账一变所有页同帧换账，不再需要 active prop。）
 */
function PageSlot({
  hidden,
  page,
  children,
}: {
  hidden: boolean
  /**
   * a02：非首页槽带 `data-page` —— 共享 UI 原语（ui.module.css）的圆润化
   * 只作用在带标记的槽里，首页槽（含它的弹窗）保持原样。
   */
  page?: string
  children: ReactNode
}) {
  return (
    <div className={s.pageSlot} data-hidden={hidden || undefined} data-page={page}>
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
   * 反方向那一条（2026-10-13，工艺页 → 预设页）：工艺页「MKP 工艺」档里那几份
   * **还没下载**的，点一下就跳过来把那一份取回去 —— 去的是预设页的
   * 「切片器配置 → 云端」那一栏（下载在那儿）。
   *
   * 与 `openBbs` 同一个形状：**跨页跳转的出口由外壳给**，页面只把"我要去"交出去；
   * `nonce` 同样是"再点一次也要重新触发"。带的那点意图（切到切片器那一栏）
   * 由预设页自己认（见 `PagePresets` 的 `pending`）。
   */
  const [pendingPresets, setPendingPresets] = useState<{ nonce: number } | null>(null)

  const openPresetsForSlicer = useCallback(() => {
    setPendingPresets((prev) => ({ nonce: (prev?.nonce ?? 0) + 1 }))
    setTab('preset')
  }, [])

  /*
   * 预设页那句「此预设需要更新版 SupportEase」的「去更新」出口：切到设置页。
   *
   * ★ 它与 `openBbs` 同类 —— **跨页跳转的出口由外壳给**，页面只负责把"我要去"交出去。
   *   为什么不在预设页自己跳：页签状态住在外壳（`tab` 在这儿），页面不该伸手改它。
   */
  const openSettings = useCallback(() => setTab('settings'), [])

  /*
   * 后处理那一屏的「看报告」出口（跨页跳转的出口由外壳给，与 `openBbs` / `openSettings` 同类）。
   * 失败也照样进报告页 —— 那一条执行记录里写着原因与错误码（见 `app/postprocess/`）。
   */
  const openReport = useCallback(() => setTab('report'), [])

  /* 后处理那一趟：读一次，两处用（见 import 处那条注释） */
  const postProcess = usePostProcessRun()

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

          {/* 后处理在后台跑那条窄条：只在「后台运行」收起来之后、那一趟还没跑完时出现 */}
          <PostProcessStrip view={postProcess} />

          <main className={s.body}>
            <PageSlot hidden={tab !== 'machine'}>
              <PageHome density={density} />
            </PageSlot>
            <PageSlot hidden={tab !== 'preset'} page="presets">
              <PagePresets
                density={density}
                onOpenBbs={openBbs}
                onOpenSettings={openSettings}
                pending={pendingPresets}
              />
            </PageSlot>
            <PageSlot hidden={tab !== 'calib'} page="calib">
              <PageCalib />
            </PageSlot>
            <PageSlot hidden={tab !== 'bbs'} page="bbs">
              <PageBbs
                density={density}
                pending={pendingBbs}
                onNeedDownload={openPresetsForSlicer}
              />
            </PageSlot>
            <PageSlot hidden={tab !== 'report'} page="report">
              <PageReport />
            </PageSlot>
            <PageSlot hidden={tab !== 'settings'} page="settings">
              <PageSettings />
            </PageSlot>
          </main>
          <PostProcessModal view={postProcess} onOpenReport={openReport} />
        </FieldLayer>
      </FileImportProvider>

      {/* Windows 的 decorations: false 之后系统 resize 边框在可见窗口之外，
          补一圈内侧命中区让抓取带跨在边界上。macOS 不需要——系统管 resize */}
      {inTauri && PLATFORM === 'windows' && <ResizeEdges />}
    </div>
  )
}
