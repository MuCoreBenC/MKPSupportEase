import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react'

import { api } from '../api'
import { FieldLayer } from '../components/field'
import NotifyHost from './notify/NotifyHost'
import { useDensity } from '../hooks/useDensity'
import { detectPlatform } from '../hooks/usePlatform'
import ResizeEdges from './components/ResizeEdges'
import TopTabs from './components/TopTabs'
import { tabs } from './constants/tabs'
import PageHome from './home/PageHome'
import Skeleton from './home/Skeleton'

/*
 * ★★ **首屏只装首页，其余五页在第一次点开时才取**（2026-10-19，Windows 白屏那一刀）。
 *
 * 这六个页面原来是**静态 import** —— dev 模式下意味着浏览器要先把这五页的整棵依赖树
 * 都请求并 transform 完，`App` 才挂得上第一帧。哪怕用户只看首页，PagePresets /
 * PageCalib / PageBbs 那几棵子树也得先编一遍（Windows 上单模块 transform 比 Mac
 * 慢一截，几百个模块叠起来就是用户看到的白屏）。
 *
 * 改成 `lazy` 之后：首屏 = 外壳 + 首页那一棵；点开别的页签时再取那一页的模块，
 * 取的过程中由 `PageSlot` 里那具 `PageLoading` 顶着（与工作台"先给骨架、数据后到"
 * 是同一条口径：**点了立刻有反馈**）。生产构建也跟着受益 —— 五页各自成 chunk，
 * 首包不必背它们。
 *
 * ★ 首页**保持静态**：它是落地页，拆出去只会多一次串行请求。
 * ★ 动态 import 的路径必须是字面量（Vite 要在构建期静态分析出 chunk）。
 */
const PagePresets = lazy(() => import('./presets/PagePresets'))
const PageCalib = lazy(() => import('./calib/PageCalib'))
const PageBbs = lazy(() => import('./bbs/PageBbs'))
const PageReport = lazy(() => import('./report/PageReport'))
const PageSettings = lazy(() => import('./settings/PageSettings'))
/* 通用文件导入入口（第十二层）：住在 App 层，不属于任何一页 ——
   拖拽进窗口 / 文件选择器都从这里走；预设页只是第一个消费者（见那一页的按钮） */
import { FileImportProvider, ImportBanner } from './import/FileImport'
import { DevPreviewProvider } from './dev/preview'
import { DevPreviewTabs } from './dev/DevPreviewTabs'
/* 钩子那一趟的那一屏（切片器导出 G-code 时，本程序被带参数拉起来的那一次）：
   普通模式里它读到 `null`，自己渲染空 —— 所以这里无条件挂一个就够。
   ★ 那一趟的状态**在 App 这一层读一次**（`usePostProcessRun`），同时喂给两处：
     那一屏模态框 + 标题栏那枚小胶囊（「后台运行」收起来之后总得有个入口回得去）。 */
import PostProcessModal from './postprocess/PostProcessModal'
import { usePostProcessRun } from './postprocess/usePostProcessRun'
import { usePinPref } from './state/appState'
import { usePostprocPrefs } from './state/usePostprocPrefs'
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
/**
 * 懒加载页那一瞬的占位（见文件头 `PagePresets` 那几条 `lazy`）。
 *
 * 与 `home/Skeleton` 同一条规矩：**灰条就是灰条，不预先填字**。它只在"第一次点开
 * 某一页、那一页的模块还在下载/编译"时出现；模块到了整棵换成真页面。
 */
type PageLayout = 'list' | 'wizard' | 'split' | 'form'

/**
 * 每种页型一具轮廓 —— 点开「预设」看到的是一张表的形状，点开「校准」看到的是
 * 向导的形状，而不是统一的五根条子。
 *
 * 只在**模块还没下载/编译完**的那一瞬出现（见文件头那几条 `lazy`）；数据加载
 * 各页有自己的加载态，与这一层无关。仍然是灰条就是灰条，不预填任何字。
 */
function PageLoading({ layout }: { layout: PageLayout }) {
  return (
    <div className={s.pageLoading} data-layout={layout} role="status" aria-label="页面正在加载">
      {layout === 'list' && (
        <>
          <div className={s.skRow}>
            <Skeleton width="5.4em" />
            <Skeleton width="5.4em" />
            <span className={s.skGrow} />
            <Skeleton width="3.6em" />
            <Skeleton width="3.6em" />
            <Skeleton width="13em" />
          </div>
          <div className={s.skRow}>
            <Skeleton width="12em" />
            <span className={s.skGrow} />
            <Skeleton width="9em" />
          </div>
          <div className={s.skTable}>
            <div className={s.skTableRow}>
              <Skeleton width="70%" />
              <Skeleton width="55%" />
              <Skeleton width="60%" />
              <Skeleton width="50%" />
              <Skeleton width="58%" />
            </div>
            {[0, 1, 2].map((i) => (
              <div key={i} className={s.skTableRow}>
                <Skeleton width={i === 1 ? '84%' : '66%'} />
                <Skeleton width="52%" />
                <Skeleton width="70%" />
                <Skeleton width="48%" />
                <Skeleton width="56%" />
              </div>
            ))}
          </div>
          <div className={s.skFoot}>
            <Skeleton width="55%" />
          </div>
        </>
      )}
      {layout === 'wizard' && (
        <>
          <div className={s.skRow}>
            <Skeleton width="4.2em" />
            <Skeleton width="4.2em" />
            <Skeleton width="4.2em" />
            <span className={s.skGrow} />
            <Skeleton width="7em" />
          </div>
          <div className={s.skWizard}>
            <div className={s.skFigure} />
            <div className={s.skWizardSide}>
              <Skeleton width="80%" />
              <Skeleton width="55%" />
              <Skeleton width="70%" />
              <Skeleton width="64%" />
            </div>
          </div>
          <div className={s.skRow}>
            <span className={s.skGrow} />
            <Skeleton width="5.4em" />
            <Skeleton width="5.4em" />
          </div>
        </>
      )}
      {layout === 'split' && (
        <>
          <div className={s.skRow}>
            <Skeleton width="8em" />
            <span className={s.skGrow} />
            <Skeleton width="10em" />
          </div>
          <div className={s.skSplit}>
            <div className={s.skList}>
              {[0, 1, 2, 3, 4].map((i) => (
                <div key={i} className={s.skRow}>
                  <Skeleton width={i === 1 ? '86%' : '68%'} />
                  <span className={s.skGrow} />
                  <Skeleton width="2.4em" />
                </div>
              ))}
            </div>
            <div className={s.skDetail}>
              <Skeleton width="45%" />
              <Skeleton width="72%" />
              <div className={s.skFigure} />
              <Skeleton width="90%" />
              <Skeleton width="58%" />
            </div>
          </div>
        </>
      )}
      {layout === 'form' && (
        <>
          <div className={s.skRow}>
            <Skeleton width="6em" />
          </div>
          {[0, 1, 2].map((i) => (
            <div key={i} className={s.skGroup}>
              <Skeleton width="5em" />
              <div className={s.skRow}>
                <Skeleton width={i === 1 ? '32%' : '48%'} />
                <span className={s.skGrow} />
                <Skeleton width="5em" />
              </div>
            </div>
          ))}
        </>
      )}
    </div>
  )
}

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
  layout = 'list',
  children,
}: {
  hidden: boolean
  /**
   * a02：非首页槽带 `data-page` —— 共享 UI 原语（ui.module.css）的圆润化
   * 只作用在带标记的槽里，首页槽（含它的弹窗）保持原样。
   */
  page?: string
  /** 懒加载占位用哪种轮廓（见 `PageLoading`）；不传按列表页画 */
  layout?: PageLayout
  children: ReactNode
}) {
  return (
    <div className={s.pageSlot} data-hidden={hidden || undefined} data-page={page}>
      {/* 懒加载页第一次挂载时会 suspend —— 这一层兜住那一瞬（见 `PageLoading`） */}
      <Suspense fallback={<PageLoading layout={layout} />}>{children}</Suspense>
    </div>
  )
}

export default function App() {
  const rootRef = useRef<HTMLDivElement>(null)
  const density = useDensity(rootRef)
  const [tab, setTab] = useState('machine')

  /*
   * 窗口置顶（2026-10-17）：**落地只看这一处** —— 档位来自 AppState（工艺页那枚胶囊
   * 点出来的），"现在在哪一页"只有这里知道（`tab`）。于是：
   *
   *   always → 恒置顶；page → 只有 `bbs`（工艺页）置顶；off → 不置顶。
   *
   * ★ 切页的每一个入口（顶栏页签、`openBbs`、`openPresetsForSlicer`、开发台）
   *   最终都改的是 `tab`，所以一条 effect 就够，不必逐处埋点。
   * ★ 不写盘、不发广播：档位没变，只是把它落到窗口上（见 `api.applyPin`）。
   */
  const pin = usePinPref()
  useEffect(() => {
    void api.applyPin(tab === 'bbs').catch(() => {
      /* 置顶失败不该打断用户 —— 真实原因是窗口没了之类，日志在 Rust 侧 */
    })
  }, [pin, tab])

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
  /*
   * 后处理那一趟的状态：`silent` 决定它会不会自己弹出来。
   *
   * 设置页「后处理模式」选了静默 ⇒ 这一屏**永远不自己弹**（跑着不弹、跑完也不弹），
   * 只在标题栏右上角留一枚状态（作者 2026-10-17：「选择了静默，就不用出现弹窗，
   * 就是不用这个后处理的前端出现，缩在右上角就可以，结束也不出现」）。
   *
   * ★ 读不到偏好（还没读回来 / 读失败）按 **GUI** 处理：静默是用户主动选的
   *   "这次别打扰我"，读不到时退回"看得见"这一侧 —— 免得用户以为程序没动静。
   *   与后端 `hook_ipc::wants_display` 的兜底同一个取向。
   */
  const { prefs: postprocPrefs } = usePostprocPrefs()
  const silentPostproc = postprocPrefs?.resolved.uiMode === 'silent'
  const postProcess = usePostProcessRun(silentPostproc)

  return (
    <div ref={rootRef} className={s.shell} data-density={density}>
      {/* 开发台（`dev-preview` 事件的接收口）：只在开发构建里收得到事件，真机上是恒"没有"
          （见 `app/dev/preview.tsx`）—— 放最外层，各页的模态框都能认领自己的那一格 */}
      <DevPreviewProvider>
      {/* 开发台：把那一格所在的页签切到前台（认领方在别的页签上时，光开框看不见） */}
      <DevPreviewTabs onTab={setTab} />
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
            /* 后台跑着的那一趟：喂给标题栏那枚小胶囊（**不再是页签下面一条横条**，
               那条一出现就把整页往下推 —— 作者 2026-10-17 否了它的位置） */
            postProcess={postProcess}
          />

          {/* 导入结果条（in-flow，标签栏下面一条）：有结果才出现，不是会自己消失的提示 */}
          <ImportBanner />

          <main className={s.body}>
            <PageSlot hidden={tab !== 'machine'}>
              <PageHome density={density} />
            </PageSlot>
            <PageSlot hidden={tab !== 'preset'} page="presets" layout="list">
              <PagePresets
                density={density}
                onOpenBbs={openBbs}
                onOpenSettings={openSettings}
                pending={pendingPresets}
              />
            </PageSlot>
            <PageSlot hidden={tab !== 'calib'} page="calib" layout="wizard">
              <PageCalib />
            </PageSlot>
            <PageSlot hidden={tab !== 'bbs'} page="bbs" layout="split">
              <PageBbs
                density={density}
                pending={pendingBbs}
                onNeedDownload={openPresetsForSlicer}
              />
            </PageSlot>
            <PageSlot hidden={tab !== 'report'} page="report" layout="list">
              <PageReport />
            </PageSlot>
            <PageSlot hidden={tab !== 'settings'} page="settings" layout="form">
              <PageSettings />
            </PageSlot>
          </main>
          <PostProcessModal view={postProcess} onOpenReport={openReport} />
        </FieldLayer>
      </FileImportProvider>
      </DevPreviewProvider>

      {/* Windows 的 decorations: false 之后系统 resize 边框在可见窗口之外，
          补一圈内侧命中区让抓取带跨在边界上。macOS 不需要——系统管 resize */}
      {inTauri && PLATFORM === 'windows' && <ResizeEdges />}

      {/* 通知停泊区（右上角）。**挂在根部一次**就够 —— 它自己 portal 到 body，
          与这里的 DOM 层级无关（见 `notify/NotifyHost.tsx` 头注释） */}
      <NotifyHost />
    </div>
  )
}
