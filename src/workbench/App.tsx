/**
 * 工作台外壳：左边机型树，右边工作区。
 *
 * 状态刻意全放在这里（没有 store、没有 context）：整个工作台只有"当前选中哪个版本"
 * 和"当前在哪个页"两个跨页状态，为它们引一套状态管理是负收益。
 *
 * 首屏做三件事，顺序不能换：
 * 1. `bootstrap()` —— 建目录、补全局配置（**不写任何配方**）
 * 2. `roots()` —— 把实际读的目录显示在顶栏。出问题时第一句要问的就是这个
 * 3. `tree()` —— 机型与版本
 */
import { useCallback, useEffect, useState } from 'react'

import {
  describeError,
  wb,
  type BootstrapReport,
  type MachineNode,
  type Registry,
  type Roots,
} from './api'
import { CatalogPage } from './pages/Catalog'
import { MatrixPage } from './pages/Matrix'
import { RecipesPage } from './pages/Recipes'
import { RegistryPage } from './pages/Registry'
import { ReleasePage } from './pages/Release'
import { ResourcesPage } from './pages/Resources'
import { StructurePage } from './pages/Structure'

export interface Selection {
  machineId: string
  versionId: string
}

type Tab = 'recipes' | 'matrix' | 'structure' | 'resources' | 'catalog' | 'release' | 'registry'

/* 页签顺序就是工作顺序：改配方 → 比对 → 整理结构 → 看资源 → 编菜单 → 生成发布。
   「字段定义」放最后，因为它是只读参考，不是操作台 */
const TABS: { id: Tab; label: string }[] = [
  { id: 'recipes', label: '配方' },
  { id: 'matrix', label: '参数矩阵' },
  { id: 'structure', label: '结构 / 回收站' },
  { id: 'resources', label: '预设 / 资源' },
  { id: 'catalog', label: '菜单 / 套餐' },
  { id: 'release', label: '生成 / 发布' },
  { id: 'registry', label: '字段定义' },
]

export function WorkbenchApp() {
  const [roots, setRoots] = useState<Roots | null>(null)
  const [report, setReport] = useState<BootstrapReport | null>(null)
  const [registry, setRegistry] = useState<Registry | null>(null)
  const [tree, setTree] = useState<MachineNode[]>([])
  const [sel, setSel] = useState<Selection | null>(null)
  const [tab, setTab] = useState<Tab>('recipes')
  const [error, setError] = useState<string | null>(null)

  const reloadTree = useCallback(async () => {
    const t = await wb.tree()
    setTree(t)
    return t
  }, [])

  useEffect(() => {
    void (async () => {
      try {
        setReport(await wb.bootstrap())
        setRoots(await wb.roots())
        setRegistry(await wb.registry())
        const t = await reloadTree()
        // 默认选中第一个有版本的机型，省掉一次点击
        const first = t.find((m) => m.versions.length > 0)
        if (first) {
          setSel({ machineId: first.id, versionId: first.versions[0].id })
        }
      } catch (e) {
        setError(describeError(e))
      }
    })()
  }, [reloadTree])

  return (
    <div className="wb-shell">
      <header className="wb-topbar">
        <span className="wb-brand">后厨工作台</span>
        <span className="wb-hint">开发者工具 · 不随客户端交付</span>
        {roots && (
          <span className="wb-paths" title={`发布目录：${roots.dist}`}>
            开发数据：{roots.workbench}
          </span>
        )}
      </header>

      {error && <div className="wb-banner wb-banner--bad">{error}</div>}

      {/* 空仓库不替你造配方，只说一句 —— doc §7 */}
      {report && !report.hasMachines && !error && (
        <div className="wb-banner">
          `workbench/machines/` 下还没有机型。工作台不会替你造一个：新建机型是你的决定，
          不是默认值。
        </div>
      )}

      <div className="wb-main">
        <nav className="wb-tree">
          {tree.map((m) => (
            <div key={m.id} className="wb-tree-group">
              <div className="wb-tree-machine">
                <span>{m.displayName}</span>
                {/* 基底一个字段都没写 → 这个机型下所有版本都会是"未配置" */}
                {m.baseFieldCount === 0 && <span className="wb-tag wb-tag--warn">基底空</span>}
              </div>
              {m.versions.length === 0 && <div className="wb-tree-empty">（没有版本）</div>}
              {m.versions.map((v) => {
                const on = sel?.machineId === m.id && sel?.versionId === v.id
                return (
                  <button
                    key={v.id}
                    type="button"
                    className={on ? 'wb-tree-item wb-tree-item--on' : 'wb-tree-item'}
                    onClick={() => setSel({ machineId: m.id, versionId: v.id })}
                  >
                    {v.displayName}
                  </button>
                )
              })}
            </div>
          ))}
        </nav>

        <section className="wb-work">
          <div className="wb-tabs">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                className={tab === t.id ? 'wb-tab wb-tab--on' : 'wb-tab'}
                onClick={() => setTab(t.id)}
              >
                {t.label}
              </button>
            ))}
          </div>

          <div className="wb-page">
            {tab === 'registry' && <RegistryPage registry={registry} />}
            {tab === 'matrix' && (
              <MatrixPage tree={tree} registry={registry} selection={sel} />
            )}
            {tab === 'resources' && (
              <ResourcesPage tree={tree} onChanged={() => void reloadTree()} />
            )}
            {tab === 'catalog' && <CatalogPage onChanged={() => void reloadTree()} />}
            {tab === 'release' && <ReleasePage onChanged={() => void reloadTree()} />}
            {tab === 'structure' && (
              <StructurePage
                tree={tree}
                selection={sel}
                onChanged={() => void reloadTree()}
                onSelect={setSel}
              />
            )}
            {tab === 'recipes' &&
              (sel ? (
                <RecipesPage
                  registry={registry}
                  selection={sel}
                  onTreeChanged={() => void reloadTree()}
                />
              ) : (
                <p className="wb-placeholder">左边选一个版本。</p>
              ))}
          </div>
        </section>
      </div>
    </div>
  )
}
