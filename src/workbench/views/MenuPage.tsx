/**
 * 「套餐与菜单」视角 —— doc §4.2 的「套餐管理」行。
 *
 * # 这一版接通的是**读**（b05 Task 14.1）
 *
 * 套餐定义（①层）的写命令还没建（Task 10.4 的纪律：写入口等界面一起落），
 * 摆一个点不动的「新建套餐」比没有更糟。所以这一页展示三件事：
 *
 * 1. 套餐清单（`presets/bundles.toml`，`wb_bundles`）；
 * 2. 每条 `assetRef` 的解析状态（能不能解析到真实资产、是不是 BBS ——
 *    每条套餐至少一条 BBS，MKP 与 BBS 成套配发）；
 * 3. 谁在引用它（机型 `defaultBundle` / 版本 `recommendedBundle` 的反查，
 *    纯展示 join，不产生任何状态词 —— 那是后端的职责）。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb, type BundleList, type MachineList } from '../api'

export function MenuPage() {
  const [bundles, setBundles] = useState<BundleList | null>(null)
  const [machines, setMachines] = useState<MachineList | null>(null)
  const [err, setErr] = useState<string | null>(null)

  const load = useCallback(() => {
    void Promise.all([wb.bundles(), wb.machines()])
      .then(([b, m]) => {
        setBundles(b)
        setMachines(m)
      })
      .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
  }, [])

  useEffect(load, [load])

  if (err) {
    return (
      <p className="wb-todo" data-tone="danger">
        {err}
      </p>
    )
  }
  if (!bundles || !machines) return <p className="wb-todo">正在读 presets/bundles.toml…</p>

  return (
    <div className="wb-menu">
      {bundles.bundles.map((b) => {
        const usedByMachines = machines.machines.filter((m) => m.defaultBundle === b.id)
        const usedByVersions = machines.machines.flatMap((m) =>
          m.versions
            .filter((v) => v.recommendedBundle === b.id)
            .map((v) => `${m.display} / ${v.name}`),
        )
        return (
          <section key={b.id} className="wb-card">
            <header className="wb-card__head">
              <span className="wb-card__title">{b.display}</span>
              <span className="wb-mx__count">
                <span className="wb-mono">{b.id}</span> · {b.machineId}
                {b.updatedAt && ` · ${b.updatedAt}`}
              </span>
            </header>
            <dl className="wb-kv">
              <dt>资产引用（{b.assetRefs.length}）</dt>
              <dd>
                {b.assetRefs.length === 0 ? (
                  <span className="wb-todo" data-tone="warn">没有引用 —— 套餐不能是空的</span>
                ) : (
                  <ul className="wb-menu__refs">
                    {b.assetRefs.map((r) => (
                      <li key={r.id} data-broken={r.resolvable ? undefined : 'yes'}>
                        <span className="wb-mono">{r.id}</span>
                        {r.isBbs && <i className="wb-menu__bbs">BBS</i>}
                        {!r.resolvable && <i className="wb-menu__broken">解析不到资产</i>}
                      </li>
                    ))}
                  </ul>
                )}
              </dd>
              <dt>被谁引用</dt>
              <dd>
                {usedByMachines.length === 0 && usedByVersions.length === 0 ? (
                  '—'
                ) : (
                  <ul className="wb-menu__refs">
                    {usedByMachines.map((m) => (
                      <li key={`m-${m.id}`}>
                        {m.display} <i className="wb-menu__role">默认套餐</i>
                      </li>
                    ))}
                    {usedByVersions.map((s) => (
                      <li key={`v-${s}`}>
                        {s} <i className="wb-menu__role">推荐套餐</i>
                      </li>
                    ))}
                  </ul>
                )}
              </dd>
            </dl>
          </section>
        )
      })}
      <p className="wb-todo">
        套餐定义目前只读（{bundles.bundles.length} 条）。新增 / 改套餐的写入口还没建 ——
        先有文件才能选择（doc §4.4）。
      </p>
    </div>
  )
}
