/*
 * 文案审查台 · 通用内核
 *
 * 一份数据 = 一张审查台。数据从 `window.COPY_AUDIT_DATA` 读（同目录 `<script src>`，
 * 不走 fetch —— file:// 直接双击打开也能用）。
 *
 * ── 数据契约 ────────────────────────────────────────────────────────────
 *   meta: {
 *     title        顶栏标题
 *     sub          顶栏副标题（一行，说这批是什么）
 *     dataset      数据集标识；换一份数据就换一个，localStorage 按它隔离
 *     storageKey   批注存的键
 *     howto        可选，说明块里的补充 HTML（口径、边界、出处）
 *   }
 *   pages: [ { id, label } ]                 顺序 = 侧栏顺序 = 列表顺序
 *   items: [ {
 *     page         对应 pages[].id
 *     sec          区块名（同一页里连续的同名区块合成一节）
 *     where        精确位置（卡片第一行）
 *     label        可选，条目的名字（参数名 / 键名）
 *     cur          现文案原文
 *     old          可选，旧文案（灰掉划掉，留作复盘）
 *     type         可选，问题类型代号数组（A–H，见 README）
 *     advice       'edit' | 'keep' | 'layout'
 *     sug          建议文案；advice=edit 且 sug 为空 = 建议删掉这句
 *     why          一句理由（只给作者看）
 *     src          出处（`文件:行` 或注册表键），点一下复制
 *   } ]
 *
 * ── 决定 ────────────────────────────────────────────────────────────────
 *   decisions[id] = { d:'use'|'drop'|'edit', e?:'自己写的文案', n?:'备注' }
 *   再点同一个按钮 = 取消。存 localStorage，导出 JSON 可原样导回。
 */
;(() => {
  const D = window.COPY_AUDIT_DATA
  if (!D || !Array.isArray(D.items)) {
    document.body.innerHTML =
      '<p style="padding:40px;font:14px -apple-system,sans-serif">没读到数据：' +
      '检查同目录的 <code>data/*.js</code> 是否在本页之前引入。</p>'
    return
  }
  const M = D.meta || {}
  const PAGES = D.pages || []
  const PAGE_LABEL = new Map(PAGES.map((p) => [p.id, p.label]))
  const ORDER = new Map(PAGES.map((p, i) => [p.id, i]))
  const ITEMS = D.items.map((it, i) => ({ ...it, id: `${it.page}#${i}`, _i: i }))
  const BY_ID = new Map(ITEMS.map((it) => [it.id, it]))
  const LS = M.storageKey || 'copy-audit.v1'

  const $ = (sel, root) => (root || document).querySelector(sel)
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel))
  const esc = (s) =>
    String(s == null ? '' : s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))

  /* ---------------- 批注 ---------------- */
  let decisions = {}
  let storageOK = true
  try {
    const raw = localStorage.getItem(LS)
    if (raw) decisions = JSON.parse(raw) || {}
  } catch {
    storageOK = false
  }
  let saveTimer = null
  function save() {
    if (saveTimer !== null) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      saveTimer = null
      try {
        localStorage.setItem(LS, JSON.stringify(decisions))
      } catch {
        if (storageOK) toast('批注存不下（浏览器存储被禁用），先导出 JSON')
        storageOK = false
      }
    }, 120)
  }

  /* ---------------- 骨架 ---------------- */
  const pageIdx = new Map()
  ITEMS.forEach((it, i) => {
    if (!pageIdx.has(it.page)) pageIdx.set(it.page, [])
    pageIdx.get(it.page).push(it)
  })
  const pageCount = (id) => (pageIdx.get(id) || []).length
  const doneCount = (id) => (pageIdx.get(id) || []).filter((it) => decisions[it.id]).length

  document.body.innerHTML = `
    <header class="top">
      <div class="top-row">
        <div class="brand">
          <div class="ttl">${esc(M.title || '文案审查台')}</div>
          <div class="sub">${esc(M.sub || '')}</div>
        </div>
        <div class="prog" title="已批 / 全部"><i id="progFill"></i></div>
        <div class="counts" id="counts"></div>
        <div class="ops">
          <button class="btn" id="btnUsePage" title="把当前页所有「建议改」且还没批的条目一次标成同意">采用本页建议</button>
          <button class="btn" id="btnExportMd">导出 Markdown</button>
          <button class="btn" id="btnExportJson">导出 JSON</button>
          <button class="btn" id="btnImport">导入 JSON</button>
          <button class="btn danger" id="btnClear">清空批注</button>
          <input type="file" id="fileIn" accept=".json,application/json" hidden>
        </div>
      </div>
      <div class="filters">
        <button class="chip" data-page="all">全部<span class="n" id="nAll"></span></button>
        <span id="pageChips"></span>
        <span class="chip sep">|</span>
        <span id="statusChips"></span>
        <input id="q" type="search" placeholder="搜现文案 / 建议 / 位置 / 键名（⌘K）" />
        <select class="sel" id="advSel" title="按建议去向筛">
          <option value="all">全部去向</option>
          <option value="edit">建议改</option>
          <option value="keep">建议留</option>
          <option value="layout">布局欠账</option>
        </select>
      </div>
    </header>
    <div class="app">
      <aside class="toc">
        <div class="toc-h">页面</div>
        <div id="tocPages"></div>
        <div class="toc-h" id="tocSecH" style="margin-top:14px">本页区块</div>
        <div id="tocSecs"></div>
      </aside>
      <main>
        <details class="legend" id="legend">
          <summary>怎么批 · 判据 · 边界</summary>
          <div class="legend-body">
            <p><b>三个按钮</b>：<b>同意</b> 按建议改；<b>不改</b> 保留现在这句；<b>自己写</b> 用你写的那句（只填中文，其余语言我按意思补）。
            再点同一个 = 取消。右侧 <b>＋备注</b> 只给我看，不落进界面。</p>
            <p><b>快捷键</b>：<kbd>a</kbd> 同意 · <kbd>d</kbd> 不改 · <kbd>e</kbd> 自己写 · <kbd>j</kbd>/<kbd>k</kbd> 下一条/上一条 · <kbd>Esc</kbd> 收起输入框。
            光标在输入框里时快捷键不生效。</p>
            <table>
              <tr><th>代号</th><th>类型</th><th>判据</th></tr>
              <tr><td>A</td><td>机制解释</td><td>在讲软件内部怎么运作，用户做决定用不上</td></tr>
              <tr><td>B</td><td>括号 / 破折号补丁</td><td>主句后面挂一段解释，句号前又开一句</td></tr>
              <tr><td>C</td><td>一处说几遍</td><td>同一事实在行、详情、提示条各来一遍</td></tr>
              <tr><td>D</td><td>长句 / 从句套从句</td><td>一句里两三个分句，读一半要回头</td></tr>
              <tr><td>E</td><td>字段名当句子</td><td>发布序号、基线、台账、真源这类内部说法上屏</td></tr>
              <tr><td>F</td><td>口语赘词 / 安抚尾巴</td><td>这一份、的话、随时、一个字都没动</td></tr>
              <tr><td>G</td><td>元话术</td><td>谈软件自己，而不是用户正在做的事</td></tr>
              <tr><td>H</td><td>说明代偿布局</td><td>字删不掉是因为布局没交代（这类另开布局轮）</td></tr>
            </table>
            <p>判据只有一句：<b>删掉它，用户会不会做错？</b> 不会就删。说明只讲后果，不讲机制；一句只留一句。
            字数上限：按钮 4 字、状态词 8 字、说明句 30 字、提示条 40 字、tooltip 50 字。</p>
            ${M.howto || ''}
          </div>
        </details>
        <div id="list"></div>
      </main>
    </div>
    <div id="toast"></div>
  `

  /* ---------------- 顶栏计数与筛选项 ---------------- */
  $('#nAll').textContent = String(ITEMS.length)
  $('#pageChips').innerHTML = PAGES.filter((p) => pageCount(p.id) > 0)
    .map(
      (p) =>
        `<button class="chip" data-page="${esc(p.id)}">${esc(p.label)}<span class="n">${pageCount(p.id)}</span></button>`,
    )
    .join('')
  $('#statusChips').innerHTML = [
    ['todo', '待批'],
    ['done', '已批'],
    ['all', '不限'],
  ]
    .map(([k, label]) => `<button class="chip" data-status="${k}">${label}<span class="n"></span></button>`)
    .join('')

  const state = { page: 'all', status: 'todo', adv: 'all', q: '' }

  function stat() {
    let use = 0
    let drop = 0
    let edit = 0
    for (const it of ITEMS) {
      const d = decisions[it.id]
      if (!d) continue
      if (d.d === 'use') use++
      else if (d.d === 'drop') drop++
      else if (d.d === 'edit') edit++
    }
    const done = use + drop + edit
    const edits = ITEMS.filter((it) => it.advice === 'edit')
    const editsDone = edits.filter((it) => decisions[it.id]).length
    return { use, drop, edit, done, total: ITEMS.length, edits: edits.length, editsDone }
  }

  function paintTop() {
    const s = stat()
    $('#progFill').style.width = `${s.total === 0 ? 0 : (s.done / s.total) * 100}%`
    $('#counts').innerHTML =
      `<span><b>${s.done}</b> / ${s.total}</span>` +
      `<span>建议改已批 <b>${s.editsDone}</b> / ${s.edits}</span>` +
      `<span>同意 <b>${s.use}</b> · 不改 <b>${s.drop}</b> · 自写 <b>${s.edit}</b></span>`
    $$('.chip[data-page]').forEach((b) => {
      const id = b.dataset.page
      b.setAttribute('aria-pressed', state.page === id ? 'true' : 'false')
    })
    $$('.chip[data-status]').forEach((b) => {
      const id = b.dataset.status
      b.setAttribute('aria-pressed', state.status === id ? 'true' : 'false')
      const nEl = $('.n', b)
      if (nEl) {
        const n =
          id === 'todo'
            ? ITEMS.filter((it) => !decisions[it.id]).length
            : id === 'done'
              ? ITEMS.filter((it) => decisions[it.id]).length
              : ITEMS.length
        nEl.textContent = String(n)
      }
    })
    paintToc()
  }

  function paintToc() {
    $('#tocPages').innerHTML = [
      `<button class="toc-a" data-toc-page="all"><span class="dot"></span>全部<span class="n">${ITEMS.length}</span></button>`,
      ...PAGES.filter((p) => pageCount(p.id) > 0).map((p) => {
        const total = pageCount(p.id)
        const done = doneCount(p.id)
        const cls = done === total ? 'dot done' : 'dot'
        return `<button class="toc-a" data-toc-page="${esc(p.id)}" aria-current="${state.page === p.id}">
          <span class="${cls}"></span>${esc(p.label)}<span class="n">${done}/${total}</span></button>`
      }),
    ].join('')

    const cur = state.page === 'all' ? [] : PAGES.filter((p) => p.id === state.page)
    const secs = []
    for (const p of cur) {
      const seen = new Set()
      for (const it of pageIdx.get(p.id) || []) {
        if (seen.has(it.sec)) continue
        seen.add(it.sec)
        secs.push({ page: p.id, sec: it.sec, id: secAnchor(it.page, it.sec) })
      }
    }
    $('#tocSecH').style.display = cur.length === 0 ? 'none' : ''
    $('#tocSecs').innerHTML =
      cur.length === 0
        ? ''
        : secs
            .filter((s) => document.getElementById(s.id))
            .map((s) => `<button class="toc-sub" data-scroll="${s.id}">${esc(s.sec)}</button>`)
            .join('')
  }

  /* ---------------- 列表 ---------------- */
  const secAnchor = (page, sec) => `sec-${page}-${encodeURIComponent(sec).replace(/%/g, '')}`

  function cardHTML(it) {
    const types = (it.type || []).map((t) => `<span class="type">${esc(t)}</span>`).join('')
    const advTag =
      it.advice === 'edit'
        ? '<span class="tag edit">建议改</span>'
        : it.advice === 'keep'
          ? '<span class="tag keep">建议留</span>'
          : '<span class="tag layout">布局</span>'
    const pageLabel = PAGE_LABEL.get(it.page) || it.page
    const sugLine =
      it.advice === 'edit'
        ? `<div class="line sug${it.sug ? '' : ' gone'}">
             <span class="lab">建议</span>
             <div class="txt">${it.sug ? esc(it.sug) : '删掉这一句'}</div>
           </div>`
        : it.advice === 'layout' && it.sug
          ? `<div class="line sug">
               <span class="lab">布局</span>
               <div class="txt">${esc(it.sug)}</div>
             </div>`
          : ''
    return `
      <article class="card" data-id="${esc(it.id)}" data-adv="${esc(it.advice || 'keep')}" tabindex="-1">
        <div class="head">
          <span class="where"><b>${esc(pageLabel)}</b><span class="sep2">›</span>${esc(it.sec)}${
            it.where ? `<span class="sep2">·</span>${esc(it.where)}` : ''
          }</span>
          ${types}
          ${advTag}
          <span class="badge"></span>
          ${it.src ? `<span class="src" title="点一下复制出处">${esc(it.src)}</span>` : '<span class="grow"></span>'}
        </div>
        ${it.label ? `<div class="why"><span class="lbl">名称</span><b>${esc(it.label)}</b>${
          it.labelSug ? ` <span class="lbl">→</span><b>${esc(it.labelSug)}</b>` : ''
        }</div>` : ''}
        <div class="line cur">
          <span class="lab">现文案</span>
          <div class="txt">${esc(it.cur)}</div>
        </div>
        ${it.old ? `<div class="note-old">旧：${esc(it.old)}</div>` : ''}
        ${sugLine}
        ${it.why ? `<p class="why">${esc(it.why)}</p>` : ''}
        <div class="decide">
          <button type="button" data-act="use">同意</button>
          <button type="button" data-act="drop">不改</button>
          <button type="button" data-act="edit">自己写</button>
          <button type="button" class="mini" data-act="note">＋ 备注</button>
        </div>
        <div class="editor">
          <textarea placeholder="用你写的那句（只填中文）"></textarea>
          <div class="hint">⌘Enter 收下 · Esc 收起</div>
        </div>
        <input class="noteInput" placeholder="给我看的备注（不落进界面）" />
      </article>`
  }

  function build() {
    const frag = document.createDocumentFragment()
    const pages = state.page === 'all' ? PAGES.filter((p) => pageCount(p.id) > 0) : PAGES.filter((p) => p.id === state.page)
    for (const p of pages) {
      const list = pageIdx.get(p.id) || []
      let curSec = null
      for (const it of list) {
        if (it.sec !== curSec) {
          curSec = it.sec
          const h = document.createElement('div')
          h.className = 'sec-h'
          h.id = secAnchor(p.id, it.sec)
          h.innerHTML = `${
            state.page === 'all' ? `${esc(PAGE_LABEL.get(p.id) || p.id)} · ` : ''
          }${esc(it.sec)}<span class="n">${list.filter((x) => x.sec === it.sec).length}</span>`
          frag.appendChild(h)
        }
        const wrap = document.createElement('div')
        wrap.innerHTML = cardHTML(it)
        frag.appendChild(wrap.firstElementChild)
      }
    }
    frag.appendChild(Object.assign(document.createElement('div'), { id: 'emptyBox', className: 'empty', hidden: true, textContent: '这一组筛选下没有条目。' }))
    const list = $('#list')
    list.innerHTML = ''
    list.appendChild(frag)
    ITEMS.forEach((it) => paint(it.id))
  }

  function paint(id) {
    const card = $(`.card[data-id="${CSS.escape(id)}"]`)
    if (!card) return
    const d = decisions[id]
    const badge = $('.badge', card)
    const noteBtn = $('.mini[data-act="note"]', card)
    if (!d) {
      card.removeAttribute('data-d')
      badge.className = 'badge'
      badge.textContent = ''
      card.classList.remove('noting')
      noteBtn.textContent = '＋ 备注'
      noteBtn.classList.remove('hasNote')
      return
    }
    card.dataset.d = d.d
    const map = { use: ['use', '已同意'], drop: ['drop', '不改'], edit: ['edit', '自己写'] }
    const [cls, text] = map[d.d] || ['', '']
    badge.className = `badge on ${cls}`
    badge.textContent = text
    const hasNote = !!(d.n && d.n.trim())
    noteBtn.textContent = hasNote ? '备注 ✓' : '＋ 备注'
    noteBtn.classList.toggle('hasNote', hasNote)
    if (hasNote) card.classList.add('noting')
    else if (!$('.noteInput', card).matches(':focus')) card.classList.remove('noting')
    const ta = $('textarea', card)
    if (document.activeElement !== ta) ta.value = d.e || ''
    const ni = $('.noteInput', card)
    if (document.activeElement !== ni) ni.value = d.n || ''
  }

  /* ---------------- 筛选 ---------------- */
  function apply() {
    const q = state.q.trim().toLowerCase()
    let shown = 0
    for (const it of ITEMS) {
      const card = $(`.card[data-id="${CSS.escape(it.id)}"]`)
      if (!card) continue
      const d = decisions[it.id]
      let ok = true
      if (state.page !== 'all' && it.page !== state.page) ok = false
      if (ok && state.status === 'todo') ok = !d
      if (ok && state.status === 'done') ok = !!d
      if (ok && state.adv !== 'all') ok = it.advice === state.adv
      if (ok && q) {
        const hay = [it.cur, it.sug, it.where, it.sec, it.label, it.old, it.src].join(' ').toLowerCase()
        ok = hay.includes(q)
      }
      card.hidden = !ok
      if (ok) shown++
    }
    // 区块标题：本节一条都没显示就一起藏
    $$('.sec-h').forEach((h) => {
      let n = 0
      let el = h.nextElementSibling
      while (el && !el.classList.contains('sec-h')) {
        if (el.classList.contains('card') && !el.hidden) n++
        el = el.nextElementSibling
      }
      h.hidden = n === 0
      const toc = $(`.toc-sub[data-scroll="${h.id}"]`)
      if (toc) toc.dataset.hidden = n === 0 ? 'true' : 'false'
    })
    $('#emptyBox').hidden = shown > 0
    if (shown === 0) {
      // 空提示摆到列表末尾——上面全是隐藏的卡
      const box = $('#emptyBox')
      if (box.parentElement !== $('#list')) $('#list').appendChild(box)
    }
  }

  function refresh() {
    paintTop()
    apply()
  }

  /* ---------------- 决定 ---------------- */
  function setDecision(id, d, extra) {
    const prev = decisions[id] || {}
    const next = { ...prev, ...(extra || {}) }
    if (d === null) {
      delete decisions[id]
    } else if (prev.d === d) {
      // 再点一次 = 取消这一种决定，但备注留着
      if (next.e) delete next.e
      if (next.n) decisions[id] = { n: next.n }
      else delete decisions[id]
      if (decisions[id]) delete decisions[id].d
      if (decisions[id] && !decisions[id].d && !decisions[id].n) delete decisions[id]
    } else {
      next.d = d
      decisions[id] = next
    }
    save()
    paint(id)
    refresh()
  }

  function makeEditable(id) {
    const card = $(`.card[data-id="${CSS.escape(id)}"]`)
    if (!card) return
    card.classList.add('editing')
    const ta = $('textarea', card)
    ta.focus()
    ta.setSelectionRange(ta.value.length, ta.value.length)
  }

  /* ---------------- 事件 ---------------- */
  document.addEventListener('click', (e) => {
    const t = e.target
    const chip = t.closest('.chip[data-page]')
    if (chip) {
      state.page = chip.dataset.page
      build()
      refresh()
      window.scrollTo({ top: 0, behavior: 'smooth' })
      return
    }
    const st = t.closest('.chip[data-status]')
    if (st) {
      state.status = st.dataset.status
      apply()
      paintTop()
      return
    }
    const tp = t.closest('.toc-a[data-toc-page]')
    if (tp) {
      state.page = tp.dataset.tocPage
      build()
      refresh()
      window.scrollTo({ top: 0, behavior: 'smooth' })
      return
    }
    const ts = t.closest('.toc-sub[data-scroll]')
    if (ts) {
      const el = document.getElementById(ts.dataset.scroll)
      if (el) {
        if (state.status !== 'all') {
          state.status = 'all'
          apply()
          paintTop()
        }
        el.scrollIntoView({ behavior: 'smooth', block: 'start' })
      }
      return
    }
    const act = t.closest('[data-act]')
    if (act) {
      const card = act.closest('.card')
      if (!card) return
      const id = card.dataset.id
      const a = act.dataset.act
      if (a === 'use' || a === 'drop') setDecision(id, a)
      else if (a === 'edit') makeEditable(id)
      else if (a === 'note') {
        card.classList.add('noting')
        $('.noteInput', card).focus()
      }
      return
    }
    const src = t.closest('.src')
    if (src) {
      copy(src.textContent.trim(), '已复制出处')
    }
  })

  document.addEventListener('input', (e) => {
    const t = e.target
    if (t.id === 'q') {
      state.q = t.value
      apply()
      return
    }
    const card = t.closest('.card')
    if (!card) return
    const id = card.dataset.id
    if (t.tagName === 'TEXTAREA') {
      const v = t.value
      const prev = decisions[id] || {}
      decisions[id] = { ...prev, d: 'edit', e: v }
      save()
      paint(id)
      refresh()
    } else if (t.classList.contains('noteInput')) {
      const prev = decisions[id] || {}
      const n = t.value
      if (prev.d) decisions[id] = { ...prev, n }
      else if (n.trim()) decisions[id] = { ...prev, n }
      else delete decisions[id]
      save()
      paint(id)
      refresh()
    }
  })

  document.addEventListener('change', (e) => {
    if (e.target.id === 'advSel') {
      state.adv = e.target.value
      apply()
    }
  })

  document.addEventListener('keydown', (e) => {
    const t = e.target
    const inField = t && (t.tagName === 'TEXTAREA' || t.tagName === 'INPUT' || t.tagName === 'SELECT')
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault()
      $('#q').focus()
      $('#q').select()
      return
    }
    if (e.key === 'Escape') {
      $$('.card.editing').forEach((c) => c.classList.remove('editing'))
      if (inField) t.blur()
      return
    }
    if (inField) {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        const card = t.closest('.card')
        if (card) {
          card.classList.remove('editing')
          card.focus()
        }
      }
      return
    }
    const cards = $$('.card').filter((c) => !c.hidden)
    if (cards.length === 0) return
    const idx = cards.findIndex((c) => c === document.activeElement)
    if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault()
      const n = cards[Math.min(cards.length - 1, idx + 1)] || cards[0]
      n.focus()
      n.scrollIntoView({ block: 'center', behavior: 'smooth' })
      return
    }
    if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault()
      const n = cards[Math.max(0, idx - 1)] || cards[0]
      n.focus()
      n.scrollIntoView({ block: 'center', behavior: 'smooth' })
      return
    }
    const card = t.closest && t.closest('.card')
    if (!card) return
    const id = card.dataset.id
    if (!BY_ID.has(id)) return
    if (e.key === 'a') {
      e.preventDefault()
      setDecision(id, 'use')
    } else if (e.key === 'd') {
      e.preventDefault()
      setDecision(id, 'drop')
    } else if (e.key === 'e') {
      e.preventDefault()
      makeEditable(id)
    }
  })

  /* ---------------- 导出 / 导入 / 清空 ---------------- */
  $('#btnExportMd').addEventListener('click', () => {
    const s = stat()
    const now = new Date()
    const stamp = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
    const lines = []
    lines.push(`# ${M.title || '文案审查台'} · 决定清单`)
    lines.push('')
    lines.push(`> 数据集 \`${M.dataset || '-'}\` · 导出 ${stamp} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`)
    lines.push(`> 已批 ${s.done} / ${s.total} · 同意 ${s.use} · 不改 ${s.drop} · 自己写 ${s.edit}`)
    lines.push('')
    lines.push('只列已批的条目，按界面顺序。`[同意]` = 按建议改；`[不改]` = 保留现文案；`[自写]` = 用「自己写」那一句。')
    for (const p of PAGES) {
      const decided = (pageIdx.get(p.id) || []).filter((it) => decisions[it.id])
      if (decided.length === 0) continue
      lines.push('')
      lines.push(`## ${p.label}`)
      let curSec = null
      for (const it of decided) {
        if (it.sec !== curSec) {
          curSec = it.sec
          lines.push('')
          lines.push(`### ${it.sec}`)
        }
        const d = decisions[it.id]
        const mark = d.d === 'use' ? '同意' : d.d === 'drop' ? '不改' : '自写'
        lines.push('')
        lines.push(`- **[${mark}]** ${it.where || it.cur}`)
        if (it.label) lines.push(`  - 名称：${it.label}${it.labelSug ? ` → ${it.labelSug}` : ''}`)
        if (it.src) lines.push(`  - 出处：\`${it.src}\``)
        lines.push(`  - 现：${it.cur}`)
        if (it.advice === 'edit') lines.push(`  - 建议：${it.sug || '（删掉这一句）'}`)
        if (it.advice === 'layout' && it.sug) lines.push(`  - 布局：${it.sug}`)
        if (d.d === 'edit' && d.e) lines.push(`  - 自己写：${d.e}`)
        if (d.n) lines.push(`  - 备注：${d.n}`)
      }
    }
    download(`copy-audit-${M.dataset || 'export'}-${stamp}.md`, lines.join('\n'), 'text/markdown')
    toast('已导出 Markdown')
  })

  $('#btnExportJson').addEventListener('click', () => {
    const payload = {
      dataset: M.dataset || null,
      title: M.title || null,
      exportedAt: new Date().toISOString(),
      decisions,
    }
    download(`copy-audit-${M.dataset || 'export'}-${Date.now()}.json`, JSON.stringify(payload, null, 2), 'application/json')
    toast('已导出 JSON')
  })

  $('#btnImport').addEventListener('click', () => $('#fileIn').click())
  $('#fileIn').addEventListener('change', (e) => {
    const f = e.target.files && e.target.files[0]
    if (!f) return
    const fr = new FileReader()
    fr.onload = () => {
      try {
        const raw = JSON.parse(String(fr.result))
        const incoming = raw && typeof raw === 'object' && raw.decisions ? raw.decisions : raw
        if (!incoming || typeof incoming !== 'object') throw new Error('bad shape')
        if (raw && raw.dataset && M.dataset && raw.dataset !== M.dataset) {
          if (!confirm(`这份批注属于「${raw.dataset}」，当前是「${M.dataset}」。仍然导入？`)) return
        }
        let n = 0
        for (const [k, v] of Object.entries(incoming)) {
          if (!BY_ID.has(k)) continue
          if (!v || !v.d) continue
          decisions[k] = v
          n++
        }
        save()
        build()
        refresh()
        toast(`导入 ${n} 条批注`)
      } catch {
        toast('这个文件读不出来（要导出 JSON 那一份）')
      }
    }
    fr.readAsText(f)
    e.target.value = ''
  })

  $('#btnClear').addEventListener('click', () => {
    if (Object.keys(decisions).length === 0) return
    if (!confirm('清空全部批注？只清批注，不动条目。')) return
    decisions = {}
    save()
    build()
    refresh()
    toast('已清空批注')
  })

  $('#btnUsePage').addEventListener('click', () => {
    const pages = state.page === 'all' ? PAGES.map((p) => p.id) : [state.page]
    const targets = ITEMS.filter(
      (it) => pages.includes(it.page) && it.advice === 'edit' && !decisions[it.id] && visibleNow(it),
    )
    if (targets.length === 0) {
      toast('当前筛选下没有可采用的建议')
      return
    }
    if (!confirm(`把当前筛选下的 ${targets.length} 条「建议改」标成同意？之后还能逐条改。`)) return
    for (const it of targets) decisions[it.id] = { d: 'use' }
    save()
    build()
    refresh()
    toast(`已采用 ${targets.length} 条`)
  })

  function visibleNow(it) {
    const q = state.q.trim().toLowerCase()
    if (state.page !== 'all' && it.page !== state.page) return false
    if (state.adv !== 'all' && it.advice !== state.adv) return false
    if (q) {
      const hay = [it.cur, it.sug, it.where, it.sec, it.label, it.old, it.src].join(' ').toLowerCase()
      if (!hay.includes(q)) return false
    }
    return true
  }

  /* ---------------- 小工具 ---------------- */
  function download(name, text, type) {
    const blob = new Blob([text], { type: `${type};charset=utf-8` })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = name
    a.click()
    setTimeout(() => URL.revokeObjectURL(a.href), 4000)
  }

  function toast(msg) {
    const t = $('#toast')
    t.textContent = msg
    t.classList.add('on')
    clearTimeout(toast._t)
    toast._t = setTimeout(() => t.classList.remove('on'), 1800)
  }

  function copy(text, okMsg) {
    const done = () => toast(okMsg)
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done, fallback)
    } else fallback()
    function fallback() {
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      try {
        document.execCommand('copy')
        done()
      } catch {
        toast('复制不了，手选一下')
      }
      document.body.removeChild(ta)
    }
  }

  /* ---------------- 起 ---------------- */
  build()
  refresh()
  if (!storageOK) toast('浏览器禁了本地存储：批完请立刻导出 JSON')
  if (D.meta && D.meta.openLegend) $('#legend').open = true
})()
