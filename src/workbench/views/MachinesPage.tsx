/**
 * 「机型与版本」页 —— 照 mkppanel 那个「机型目录」的骨架。
 *
 * # 这一页回答的就是那个问过的问题
 *
 * 「加一个新机型在哪里加？加一个新版本怎么加？」——**在这里**。
 * 左边是机型卡，右上角是「+ 新增机型」，版本区右上角是「+ 新增版本」。
 *
 * 之前答不出来是因为机型清单被定成「上游的、不许改」，结构上就没有那个位置；
 * 现在数据在我们自己项目里（`presets/machines/*.toml`），
 * 一个机型 = 一个文件，一个版本 = 那个文件里的一个 `[[versions]]` 块。
 *
 * # 这一页刻意不做的
 *
 * - **不共享任何跨页状态**：它自己读自己写，不碰参数页那套草稿/撤销栈。
 *   「新建一个机型」和「把 X 轴偏移改成 -1」在撤销语义上不是一回事。
 * - **不做拖拽排序**：顺序由文件名决定，改顺序不是这一页的职责。
 * - **本轮只读。** 新增/改名/删除的按钮**不摆出来** —— 摆一个点不动的按钮
 *   比没有更糟，这是上一轮刚被指出来的问题。写入在下一轮接上（后端的
 *   往返保真与原子写已经就位）。
 */
import { useCallback, useEffect, useState } from 'react'

import {
  isAppError,
  wb,
  type BrandView,
  type MachineField,
  type MachineList,
  type MachineView,
  type VersionField,
} from '../api'

export function MachinesPage() {
  const [data, setData] = useState<MachineList | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [sel, setSel] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [addingMachine, setAddingMachine] = useState(false)

  const load = useCallback(() => {
    void wb
      .machines()
      .then((d) => {
        setData(d)
        // 默认选第一台：空着的右栏没有信息量
        setSel((s) => s ?? d.machines[0]?.id ?? null)
      })
      .catch((e: unknown) => setError(isAppError(e) ? e.message : String(e)))
  }, [])

  useEffect(load, [load])

  if (error) {
    return (
      <p className="wb-todo" data-tone="danger">
        {error}
      </p>
    )
  }
  if (!data) return <p className="wb-todo">正在读 presets/machines/…</p>

  const q = query.trim().toLowerCase()
  const shown = q
    ? data.machines.filter(
        (m) =>
          m.id.toLowerCase().includes(q) ||
          m.display.toLowerCase().includes(q) ||
          m.brand.toLowerCase().includes(q) ||
          m.externalAliases.some((a) => a.toLowerCase().includes(q)),
      )
    : data.machines
  const current = data.machines.find((m) => m.id === sel) ?? null

  return (
    <div className="wb-mc">
      <div className="wb-mc__left">
        <div className="wb-mc__lefthead">
          <input
            className="wb-input"
            type="search"
            placeholder="搜机型名 / ID / 品牌 / 别名"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button
            type="button"
            className="wb-btn"
            data-tone="primary"
            onClick={() => setAddingMachine((a) => !a)}
          >
            {addingMachine ? '取消' : '+ 新增机型'}
          </button>
        </div>

        {addingMachine && (
          <NewMachineForm
            brands={data.brands}
            onDone={(next) => {
              setData(next)
              setAddingMachine(false)
            }}
          />
        )}

        <div className="wb-mc__cards">
          {shown.map((m) => (
            <button
              key={m.id}
              type="button"
              className="wb-mc__card"
              data-on={m.id === sel ? 'yes' : undefined}
              onClick={() => setSel(m.id)}
            >
              <span className="wb-mc__brand">{m.brand}</span>
              <span className="wb-mc__name">{m.display}</span>
              <span className="wb-mc__meta">
                {m.id} · {m.versions.length} 版
              </span>
              {/* 缺配置要在卡片上就看得见，不用点进去才发现 */}
              {!m.hasDimensions && <i className="wb-mc__warn">未配尺寸</i>}
            </button>
          ))}
          {shown.length === 0 && <p className="wb-todo">没有匹配的机型</p>}
        </div>
        <p className="wb-mc__count">
          {data.machines.length} 机型 · {data.brands.length} 品牌
        </p>
        {/* 明写数据根。顶部状态条上的 RECIPE 指的是**参数页那一套**的目录，
            这一页读写的是另一个地方 —— 不写出来就会让人以为在改同一份东西 */}
        <p className="wb-mc__root" title={data.root}>
          读写 {data.root}
        </p>
      </div>

      <div className="wb-mc__right">
        {current ? (
          <MachineDetail m={current} onChanged={setData} />
        ) : (
          <p className="wb-todo">左边选一台机型</p>
        )}
      </div>
    </div>
  )
}

/**
 * 一格「点一下就地改」。
 *
 * 纪律与参数页那边一致：**失焦或回车才提交**，不是每敲一个字符提交一次 ——
 * 后者会把一次改名变成十几次写盘。Esc 放弃。
 *
 * 「清空」按钮只给可选的那几格：必填格清空之后卡片上就只剩一个 ID 了。
 * 而且清空**不是写空串**，是删掉文件里那一行（后端负责，前端只传 null）。
 */
function EditableKV({
  label,
  value,
  onSave,
  clearable = true,
}: {
  label: string
  value: string | null
  onSave: (next: string | null) => Promise<unknown>
  clearable?: boolean
}) {
  const [editing, setEditing] = useState(false)
  const [text, setText] = useState(value ?? '')
  const [busy, setBusy] = useState(false)

  const commit = (next: string | null) => {
    setBusy(true)
    void onSave(next).finally(() => {
      setBusy(false)
      setEditing(false)
    })
  }

  return (
    <>
      <dt>{label}</dt>
      <dd className="wb-kv__edit">
        {editing ? (
          <input
            className="wb-ctl"
            data-form="row"
            value={text}
            autoFocus
            disabled={busy}
            onChange={(e) => setText(e.target.value)}
            onBlur={() => (text === (value ?? '') ? setEditing(false) : commit(text))}
            onKeyDown={(e) => {
              if (e.key === 'Enter') commit(text)
              if (e.key === 'Escape') {
                setText(value ?? '')
                setEditing(false)
              }
            }}
          />
        ) : (
          <>
            <button
              type="button"
              className="wb-kv__btn"
              title="点一下改"
              onClick={() => {
                setText(value ?? '')
                setEditing(true)
              }}
            >
              {value ?? '—'}
            </button>
            {clearable && value !== null && (
              <button
                type="button"
                className="wb-kv__clear"
                title="清空这一格（会把文件里那一行删掉，不是写成空串）"
                disabled={busy}
                onClick={() => commit(null)}
              >
                ✕
              </button>
            )}
          </>
        )}
      </dd>
    </>
  )
}

/**
 * 删一个版本。**两步**：第一步先去问「会留下什么孤儿」，第二步才真删。
 *
 * 为什么不是一步一个 confirm：孤儿引用这件事**用户不查就不知道**。
 * 一个只写着「确定删除吗」的对话框等于什么都没告诉他 ——
 * 而这个操作不可逆（没有回收站也没有撤销）。
 */
function DeleteVersion({
  machineId,
  version,
  onChanged,
}: {
  machineId: string
  version: string
  onChanged: (next: MachineList) => void
}) {
  /** null = 还没问过；数组 = 问过了，这是结果 */
  const [orphans, setOrphans] = useState<string[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  if (orphans === null) {
    return (
      <div className="wb-ver__foot">
        <button
          type="button"
          className="wb-btn"
          data-tone="danger"
          disabled={busy}
          onClick={() => {
            setBusy(true)
            setErr(null)
            void wb
              .versionOrphans(machineId, version)
              .then(setOrphans)
              .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
              .finally(() => setBusy(false))
          }}
        >
          删除这个版本…
        </button>
        {err && (
          <span className="wb-todo" data-tone="danger">
            {err}
          </span>
        )}
      </div>
    )
  }

  return (
    <div className="wb-ver__confirm">
      <p className="wb-ver__warn">
        要删掉 <b>{version}</b>。**立刻写入文件，没有回收站也没有撤销。**
      </p>
      {orphans.length > 0 ? (
        <p className="wb-ver__orphans">
          删掉之后这 {orphans.length} 项在 param_registry 里会留下指向它的**孤儿引用**
          （不报错，但那几项在这台机器上会悄悄不生效）：
          <br />
          <code>{orphans.join('、')}</code>
        </p>
      ) : (
        <p className="wb-ver__ok">没有任何字段引用这一版，删掉不会留下孤儿。</p>
      )}
      <div className="wb-ver__buttons">
        <button type="button" className="wb-btn" onClick={() => setOrphans(null)}>
          不删了
        </button>
        <button
          type="button"
          className="wb-btn"
          data-tone="danger"
          disabled={busy}
          onClick={() => {
            setBusy(true)
            setErr(null)
            void wb
              .removeVersion(machineId, version)
              .then(onChanged)
              .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
              .finally(() => setBusy(false))
          }}
        >
          确认删除
        </button>
      </div>
      {err && (
        <p className="wb-todo" data-tone="danger">
          {err}
        </p>
      )}
    </div>
  )
}

/**
 * 新建机型。**它会建一个新文件**，所以表单上要把这件事说出来 ——
 * 「加一个版本」是往已有文件里插一段，这个是从零造一个文件，风险不同。
 *
 * 品牌用下拉（从已有品牌里选）而不是自由输入：品牌是个小的封闭集合，
 * 自由输入会攒出「Bambu Lab」「BambuLab」「bambu lab」三个同义词
 */
function NewMachineForm({
  brands,
  onDone,
}: {
  brands: BrandView[]
  onDone: (next: MachineList) => void
}) {
  const [id, setId] = useState('')
  const [display, setDisplay] = useState('')
  const [brand, setBrand] = useState(brands[0]?.name ?? '')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const submit = () => {
    setBusy(true)
    setErr(null)
    void wb
      .addMachine(id, brand, display)
      .then(onDone)
      // 校验全在后端（ID 字符集、与别名查重、文件已存在）。前端不复制一份判定
      .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
      .finally(() => setBusy(false))
  }

  return (
    <div className="wb-mc__add">
      <label className="wb-mc__field">
        <span>机型 ID</span>
        <input
          className="wb-input"
          value={id}
          placeholder="会变成文件名"
          autoFocus
          onChange={(e) => setId(e.target.value.toUpperCase())}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
      </label>
      <label className="wb-mc__field">
        <span>显示名</span>
        <input
          className="wb-input"
          value={display}
          placeholder="给人看的名字"
          onChange={(e) => setDisplay(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
      </label>
      <label className="wb-mc__field">
        <span>品牌</span>
        <select className="wb-ctl" data-form="row" value={brand} onChange={(e) => setBrand(e.target.value)}>
          {brands.map((b) => (
            <option key={b.id} value={b.name}>
              {b.name}
            </option>
          ))}
        </select>
      </label>
      <button type="button" className="wb-btn" data-tone="primary" disabled={busy} onClick={submit}>
        新建 {id || 'ID'}.toml
      </button>
      <p className="wb-mc__hint">
        会在 presets/machines/ 下**新建一个文件**并立刻写入（没有草稿、没有撤销）。
        同名文件已存在时不会被覆盖，会直接报错。
      </p>
      {err && (
        <p className="wb-todo" data-tone="danger">
          {err}
        </p>
      )}
    </div>
  )
}

function MachineDetail({
  m,
  onChanged,
}: {
  m: MachineView
  /** 写入成功后后端回的新清单 —— 界面直接用它，不再自己猜状态 */
  onChanged: (next: MachineList) => void
}) {
  const [openVersion, setOpenVersion] = useState<string | null>(m.versions[0]?.id ?? null)
  const [adding, setAdding] = useState(false)
  const [newId, setNewId] = useState('')
  const [newName, setNewName] = useState('')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  /** 改机型自己的一格。错误就近显示在元信息卡片头上 */
  const save = (field: MachineField, value: string | null) => {
    setErr(null)
    return wb
      .setMachineField(m.id, field, value)
      .then(onChanged)
      .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
  }

  /** 改某个版本的一格 */
  const saveVersion = (versionId: string, field: VersionField, value: string | null) => {
    setErr(null)
    return wb
      .setVersionField(m.id, versionId, field, value)
      .then(onChanged)
      .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
  }


  const submit = () => {
    setBusy(true)
    setErr(null)
    void wb
      .addVersion(m.id, newId, newName)
      .then((next) => {
        onChanged(next)
        setAdding(false)
        setNewId('')
        setNewName('')
        setOpenVersion(newId.trim())
      })
      /* 校验在后端（ID 字符集、重名都在那儿判）。
         前端**不复制一份判定** —— 两处判定迟早分岔，而分岔的表现是
         「界面说可以，后端说不行」 */
      .catch((e: unknown) => setErr(isAppError(e) ? e.message : String(e)))
      .finally(() => setBusy(false))
  }

  return (
    <>
      <header className="wb-mc__head">
        <span className="wb-mc__title">{m.display}</span>
        <span className="wb-mx__count">{m.file}</span>
      </header>

      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">元信息</span>
          {err && (
            <span className="wb-todo" data-tone="danger">
              {err}
            </span>
          )}
        </header>
        <dl className="wb-kv">
          <dt>机型 ID</dt>
          <dd>
            {m.id}
            <i className="wb-kv__note">（就是文件名，改不了）</i>
          </dd>
          <EditableKV
            label="显示名"
            value={m.display}
            clearable={false}
            onSave={(v) => save('display', v)}
          />
          <EditableKV label="品牌" value={m.brand} clearable={false} onSave={(v) => save('brand', v)} />
          <EditableKV label="内部名" value={m.name || null} onSave={(v) => save('name', v)} />
          <dt>外部别名</dt>
          <dd>{m.externalAliases.length > 0 ? m.externalAliases.join('、') : '—'}</dd>
          <dt>默认套餐</dt>
          <dd>{m.defaultBundle ?? '—'}</dd>
          <EditableKV label="图片" value={m.image} onSave={(v) => save('image', v)} />
          <EditableKV label="图标" value={m.icon} onSave={(v) => save('icon', v)} />
          <dt>尺寸</dt>
          <dd>{m.hasDimensions ? '已配置' : '未配置'}</dd>
          <dt>禁区</dt>
          <dd>{m.zoneCount > 0 ? `${m.zoneCount} 块` : '无'}</dd>
        </dl>
      </section>

      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">版本</span>
          <span className="wb-mc__headright">
            <span className="wb-mx__count">{m.versions.length} 个</span>
            <button
              type="button"
              className="wb-btn"
              data-tone="primary"
              onClick={() => setAdding((a) => !a)}
            >
              {adding ? '取消' : '+ 新增版本'}
            </button>
          </span>
        </header>

        {/* 内联表单，不是模态框 —— 加一个版本只要两个字段，
            为它盖一层遮罩把整页挡住不值得 */}
        {adding && (
          <div className="wb-mc__add">
            <label className="wb-mc__field">
              <span>版本 ID</span>
              <input
                className="wb-input"
                value={newId}
                placeholder="大写字母 / 数字 / 下划线"
                autoFocus
                onChange={(e) => setNewId(e.target.value.toUpperCase())}
                onKeyDown={(e) => e.key === 'Enter' && submit()}
              />
            </label>
            <label className="wb-mc__field">
              <span>版本名称</span>
              <input
                className="wb-input"
                value={newName}
                placeholder="给人看的名字，例如 标准版"
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && submit()}
              />
            </label>
            <button type="button" className="wb-btn" data-tone="primary" disabled={busy} onClick={submit}>
              写进 {m.file}
            </button>
            {/* 说清它会立刻落盘 —— **没有草稿也没有撤销**，那就要提前讲 */}
            <p className="wb-mc__hint">
              确认后**立刻写入**那个文件（没有草稿、没有撤销）。写错了就删掉这个版本。
            </p>
            {err && (
              <p className="wb-todo" data-tone="danger">
                {err}
              </p>
            )}
          </div>
        )}

        {m.versions.map((v) => {
          const open = openVersion === v.id
          return (
            <div key={v.id} className="wb-ver" data-open={open ? 'yes' : undefined}>
              <button
                type="button"
                className="wb-ver__head"
                onClick={() => setOpenVersion(open ? null : v.id)}
              >
                <span className="wb-ver__name">{v.name || v.id}</span>
                <span className="wb-ver__id">{v.id}</span>
                {v.tag && <i className="wb-ver__tag">{v.tag}</i>}
              </button>
              {open && (
                <>
                  <dl className="wb-kv">
                    <dt>版本 ID</dt>
                    <dd>
                      {v.id}
                      <i className="wb-kv__note">（改 ID 等于删一个再加一个，没做）</i>
                    </dd>
                    <EditableKV
                      label="版本名称"
                      value={v.name || null}
                      clearable={false}
                      onSave={(x) => saveVersion(v.id, 'name', x)}
                    />
                    <EditableKV
                      label="预设文件"
                      value={v.presetFile}
                      onSave={(x) => saveVersion(v.id, 'presetFile', x)}
                    />
                    <EditableKV
                      label="推荐套餐"
                      value={v.recommendedBundle}
                      onSave={(x) => saveVersion(v.id, 'recommendedBundle', x)}
                    />
                    <EditableKV
                      label="标签"
                      value={v.tag}
                      onSave={(x) => saveVersion(v.id, 'tag', x)}
                    />
                    <EditableKV
                      label="描述"
                      value={v.description}
                      onSave={(x) => saveVersion(v.id, 'description', x)}
                    />
                  </dl>
                  <DeleteVersion machineId={m.id} version={v.id} onChanged={onChanged} />
                </>
              )}
            </div>
          )
        })}
      </section>

      {/* 还没接上的**不摆按钮** —— 摆一个点不动的按钮比没有更糟 */}
      <p className="wb-todo">
        「新增版本」已经能用。新增机型 / 改名 / 删除还没接上 ——
        后端的写回与逐字节保真已经就位。
      </p>
    </>
  )
}
