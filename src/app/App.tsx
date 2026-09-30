import { useCallback, useRef, useState } from 'react'

import { FieldLayer } from '../components/field'
import { useDensity } from '../hooks/useDensity'
import { detectPlatform } from '../hooks/usePlatform'
import PagePlaceholder from './components/PagePlaceholder'
import ResizeEdges from './components/ResizeEdges'
import TopTabs from './components/TopTabs'
import { tabs } from './constants/tabs'
import PageCalib from './calib/PageCalib'
import PageHome from './home/PageHome'
import PagePresets from './presets/PagePresets'
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
 * # 「客户端接发布包」这一轮（P1）改了什么
 *
 * - 页签从 6 个变 8 个（加「同步」「BBS 预设」，见 `constants/tabs.ts`）；
 * - 外壳套上 `FieldLayer` —— 预设页 / 参数页的下拉、浮层、右键菜单全挂在它上面
 *   （`src/components/field/` 那一套）。它不产生包裹元素，只在最后多一个绝对定位的层，
 *   所以 `.shell` 的 flex 列布局一个字不用改。
 *
 * 六个页签（预设 / 参数 / 同步 / BBS 预设 / 报告 / 设置）这一轮先是空态：
 * 按阶段计划 P2–P5 一页一页换成真页面（报告与设置两页作者已裁决**本轮就做空态**）。
 * 空态写的是"这一页本版未接入"，不是白屏 —— 少一个页签会让"这一版缺什么"变得看不见。
 */
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
   * BBS 页本轮还是空态，所以这一格暂时只被写入、没有读者 —— 留着是因为
   * **它就是那条入口的状态**，等 BBS 页接上来时不该再去重造一遍。
   *
   * 下面那一句 `void` 只是为了把「暂时没有读者」这件事写成代码：
   * 状态本身是好的（下一次点击会换 nonce），只是读它的那一页还没搬过来。
   * BBS 页接上之后，把 `pending` 传给它、这句就删掉。
   */
  const [pendingBbs, setPendingBbs] = useState<{ name: string; nonce: number } | null>(null)
  void pendingBbs

  const openBbs = useCallback((name: string) => {
    setPendingBbs((prev) => ({ name, nonce: (prev?.nonce ?? 0) + 1 }))
    setTab('bbs')
  }, [])

  const renderPage = () => {
    switch (tab) {
      case 'calib':
        return <PageCalib />
      case 'preset':
        return <PagePresets density={density} onOpenBbs={openBbs} />
      case 'params':
        return <PagePlaceholder title="参数" hint="后处理参数与风险项" />
      case 'sync':
        return <PagePlaceholder title="同步" hint="客户端说明书与本机预设的同步" />
      case 'bbs':
        return <PagePlaceholder title="BBS 预设" hint="Bambu Studio 工艺预设查看器" />
      case 'report':
        return <PagePlaceholder title="报告" hint="后处理执行报告与历史" />
      case 'settings':
        return <PagePlaceholder title="设置" hint="应用设置、诊断与版本" />
      default:
        return <PageHome density={density} />
    }
  }

  return (
    <div ref={rootRef} className={s.shell} data-density={density}>
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

        <main className={s.body}>{renderPage()}</main>
      </FieldLayer>

      {/* Windows 的 decorations: false 之后系统 resize 边框在可见窗口之外，
          补一圈内侧命中区让抓取带跨在边界上。macOS 不需要——系统管 resize */}
      {inTauri && PLATFORM === 'windows' && <ResizeEdges />}
    </div>
  )
}
