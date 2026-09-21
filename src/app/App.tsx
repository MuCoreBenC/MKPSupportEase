import { useRef, useState } from 'react'

import { useDensity } from '../hooks/useDensity'
import { detectPlatform } from '../hooks/usePlatform'
import PagePlaceholder from './components/PagePlaceholder'
import ResizeEdges from './components/ResizeEdges'
import TopTabs from './components/TopTabs'
import { tabs } from './constants/tabs'
import PageCalib from './pages/PageCalib'
import PageHome from './pages/PageHome'
import { inTauri } from './window'
import s from './App.module.css'

/* 平台只影响标题栏把窗口按钮画在左边还是右边，一次探测就够，不必进 state */
const PLATFORM = detectPlatform()

/**
 * 应用根组件。
 *
 * 试验场那份（AppV023）多两样东西，都是预览器的：`VersionProps`（稿号外壳注入的 platform 与
 * reportMode）与「报告」的全屏子视图开关。产品里窗口就是窗口、只有一个界面，所以：
 * 平台自己探测，报告页按普通页签处理。
 *
 * 预设 / 参数 / 报告 / 设置四页这一轮不接（对应的 v005 页面与 ReportView 没有搬），
 * 但页签结构保持完整 —— 少一个页签会让"这一版缺什么"变得看不见。
 */
export default function App() {
  const rootRef = useRef<HTMLDivElement>(null)
  const density = useDensity(rootRef)
  const [tab, setTab] = useState('machine')

  const renderPage = () => {
    switch (tab) {
      case 'calib':
        return <PageCalib />
      case 'preset':
        return <PagePlaceholder title="预设" hint="本地与云端预设的管理" />
      case 'params':
        return <PagePlaceholder title="参数" hint="后处理参数与风险项" />
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
      <TopTabs
        tabs={tabs}
        active={tab}
        onChange={setTab}
        density={density}
        platform={PLATFORM}
        fluid
      />

      <main className={s.body}>{renderPage()}</main>

      {/* Windows 的 decorations: false 之后系统 resize 边框在可见窗口之外，
          补一圈内侧命中区让抓取带跨在边界上。macOS 不需要——系统管 resize */}
      {inTauri && PLATFORM === 'windows' && <ResizeEdges />}
    </div>
  )
}
