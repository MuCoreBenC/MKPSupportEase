/*
 * 图片引导那个模态框（`src/app/guide/GuideImage.tsx`）—— 版式与"框落得准"。
 *
 * 作者 2026-10-16 定的两件事，都由这条探针守着：
 *   ① **图在左、文字全在右**（状态那句 + 步骤一列，状态原来横在大标题下面占满整幅宽）
 *   ② 图按"宽 / 高里更紧的那条"**等比缩到完整显示** —— 任何窗高都不裁剪、框身不滚
 *      （他原话：「你不只是判断它的这个宽度、高度，反正你要把它缩放，可以显示完整，
 *        就像我的测试模型那一页那个图片的变化一样」）
 * ③ 图缩了之后**标注还套在控件上**：框是按容器百分比画的 ⇒ 容器必须与图一样大
 *    （这一条最容易崩：谁给容器写死一个宽、或给图加个 `width:100%` 就飘）
 *
 * 三个窗：大窗（宽那一档管）/ 矮窗（高那一档接管）/ 壳的下限 620×520。截图落 `tmp-shots/`。
 *
 * 跑法：`npx vite preview --port 4173 --strictPort`（dist 已 build）+ 本脚本
 *   node scripts/probes/guide-image.mjs
 */
import { mkdir } from 'node:fs/promises'

import { chromium } from 'playwright-core'

const url = 'http://localhost:4173/?mock=1'
await mkdir('tmp-shots', { recursive: true })

const problems = []
const say = (ok, text) => {
  console.log(`${ok ? '✓' : '✗'} ${text}`)
  if (!ok) problems.push(text)
}

const browser = await chromium.launch({ channel: 'msedge' })
const page = await browser.newPage({ viewport: { width: 1080, height: 900 }, deviceScaleFactor: 2 })
page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))

const openGuide = async () => {
  await page.getByRole('button', { name: '设置' }).click()
  await page.getByRole('button', { name: '去系统里改…' }).click()
  await page.getByText('改 .3mf 的默认打开方式').waitFor({ timeout: 8000 })
  await page.locator('[role="dialog"] img').first().waitFor({ timeout: 8000 })
  await page.waitForTimeout(350)
}

await page.goto(url)
await openGuide()

/** 这一屏的读数：图的盒子、模态框的盒子、有没有哪一层在滚、每个框相对图的位置 */
const readState = () =>
  page.evaluate(() => {
    const dialog = document.querySelector('[role="dialog"]')
    const img = dialog.querySelector('img')
    const r = (el) => {
      const b = el.getBoundingClientRect()
      return { l: b.left, t: b.top, w: b.width, h: b.height }
    }
    /* 从图往上走：任何一层"内容比可视高"就是有地方在滚（作者要的"完整显示"） */
    const scrollers = []
    for (let el = img.parentElement; el !== null; el = el.parentElement) {
      if (el.scrollHeight > el.clientHeight + 1) scrollers.push(el.className || el.tagName)
      if (el === dialog) break
    }
    const marks = [...dialog.querySelectorAll('span[style*="left:"][style*="top:"]')].map((m) => ({
      style: { left: m.style.left, top: m.style.top, width: m.style.width },
      box: r(m),
    }))
    return {
      img: r(img),
      imgSrc: img.getAttribute('src') ?? '',
      natural: { w: img.naturalWidth, h: img.naturalHeight },
      dialog: r(dialog),
      scrollers,
      marks,
      /* 右列那两样：状态那句（框里第一个 p）与步骤表（ol） */
      statusX: (() => {
        const p = dialog.querySelector('p')
        return p ? { l: r(p).l, text: (p.textContent ?? '').trim().slice(0, 14) } : null
      })(),
      stepsX: (() => {
        const ol = dialog.querySelector('ol')
        return ol ? r(ol).l : null
      })(),
      imgRight: r(img).l + r(img).w,
    }
  })

const check = (state, label) => {
  const ratioWant = state.natural.w / state.natural.h
  const ratioGot = state.img.w / state.img.h
  say(
    Math.abs(ratioGot - ratioWant) / ratioWant < 0.01,
    `${label}：图等比（${state.img.w.toFixed(0)}×${state.img.h.toFixed(0)}，比例差 ${((Math.abs(ratioGot - ratioWant) / ratioWant) * 100).toFixed(2)}%）`,
  )
  say(
    state.img.t >= state.dialog.t - 1 && state.img.t + state.img.h <= state.dialog.t + state.dialog.h + 1,
    `${label}：图完整落在框里（图底 ${(state.img.t + state.img.h).toFixed(0)} vs 框底 ${(state.dialog.t + state.dialog.h).toFixed(0)}）`,
  )
  say(state.scrollers.length === 0, `${label}：没有一层在滚（${state.scrollers.join('/') || '干净'}）`)
  /*
   * ★ 2026-10-16 方案 A：交付的图**已经带框**（后端烧进像素）⇒ 前端**不该再叠画**
   * `.mark` —— 叠了就是双份框，放大到 400% 时那点百分比取整差会露成重影。
   * 所以这一条判据是「**一个 mark 都不该有**」，而不是从前那条「每个 mark 都套在控件上」。
   * （图本身是不是成品图，由下一条 `rendered` 断言。）
   */
  if (state.marks.length === 0) {
    say(
      state.imgSrc.includes('.rendered.webp'),
      `${label}：没有叠画框，且图就是成品图（${state.imgSrc.split('/').pop()}）`,
    )
  }
  for (const m of state.marks) {
    const leftWant = Number.parseFloat(m.style.left)
    const topWant = Number.parseFloat(m.style.top)
    const leftGot = ((m.box.l - state.img.l) / state.img.w) * 100
    const topGot = ((m.box.t - state.img.t) / state.img.h) * 100
    say(
      Math.abs(leftGot - leftWant) < 0.4 && Math.abs(topGot - topWant) < 0.4,
      `${label}：框落在 ${leftWant}%/${topWant}% 上（量到 ${leftGot.toFixed(1)}%/${topGot.toFixed(1)}%）`,
    )
  }
  say(
    state.statusX !== null && state.statusX.l > state.imgRight && state.stepsX !== null && state.stepsX > state.imgRight,
    `${label}：文字（状态 + 步骤）全在图右侧 —— 状态「${state.statusX?.text}…」左缘 ${state.statusX?.l.toFixed(0)}、步骤左缘 ${state.stepsX?.toFixed(0)}，图右缘 ${state.imgRight.toFixed(0)}`,
  )
}

const shot = async (name) => {
  await page.locator('[role="dialog"]').screenshot({ path: `tmp-shots/${name}.png` })
}

/* 一：大窗（作者平时的窗） */
check(await readState(), '1080×900')
await shot('guide-layout-1080x900')

/* 二：矮窗 —— 高度那一档该接管（图变小、但仍完整） */
await page.setViewportSize({ width: 1000, height: 700 })
await page.waitForTimeout(350)
check(await readState(), '1000×700')
await shot('guide-layout-1000x700')

/* 三：最小窗（壳的下限 600×500） */
await page.setViewportSize({ width: 620, height: 520 })
await page.waitForTimeout(350)
check(await readState(), '620×520')
await shot('guide-layout-620x520')

/* ---------------------------------------------------------------------------
 * 四：点开看大的那一层（作者：「可以点就悬浮出来……就在这个模态框里面实现，
 *     不要另开一个窗；没那么突兀，悬浮出来、居中」）
 * ------------------------------------------------------------------------- */

const zoomProbe = async (label, shotName = null) => {
  const figBtn = page.locator('[role="dialog"] button[aria-label^="放大看："]')
  const bigFig = page.locator('button[aria-label^="收起放大图"]')
  const inline = figBtn.locator('img')
  const before = await inline.boundingBox()
  const smallBox = await figBtn.boundingBox()
  await figBtn.click()
  /* 先趁动画还在跑抓一眼（作者要的"移动出来"那一趟），再去量最终的落点 */
  await page.waitForTimeout(60)
  const mid = await page.evaluate(() => {
    const el = document.querySelector('button[aria-label^="收起放大图"] > div')
    return {
      anims: el.getAnimations().length,
      transform: getComputedStyle(el).transform,
      layerOpacity: getComputedStyle(el.closest('div')).opacity,
    }
  })
  say(mid.anims > 0, `${label}：放大那一趟有过渡（${mid.anims} 个动画在跑）`)
  say(
    /^matrix\(/.test(mid.transform) && !/^matrix\(1, 0, 0, 1, 0, 0\)$/.test(mid.transform),
    `${label}：刚开的那一刹那图是**偏着、缩着**的（${mid.transform.slice(0, 48)}…）—— 从原位置飞出来`,
  )
  await page.waitForTimeout(320)
  if (shotName !== null) await page.screenshot({ path: `tmp-shots/${shotName}.png` })

  say(await bigFig.isVisible(), `${label}：点小图 → 放大层浮出来了`)
  const bigBox = await bigFig.boundingBox()
  say(
    Math.abs(bigBox.width / smallBox.width - bigBox.height / smallBox.height) < 0.05 && bigBox.width > smallBox.width + 20,
    `${label}：动画放完落到中间（大图 ${bigBox.width.toFixed(0)}×${bigBox.height.toFixed(0)}，小图 ${smallBox.width.toFixed(0)}×${smallBox.height.toFixed(0)}）`,
  )

  const z = await page.evaluate(() => {
    const dialog = document.querySelector('[role="dialog"]')
    const scrim = dialog.parentElement
    const layer = document.querySelector('button[aria-label^="收起放大图"]').parentElement
    const img = document.querySelector('button[aria-label^="收起放大图"] img')
    const r = (el) => {
      const b = el.getBoundingClientRect()
      return { l: b.left, t: b.top, w: b.width, h: b.height, r: b.right, b: b.bottom }
    }
    const marks = [...layer.querySelectorAll('span[style*="left:"][style*="top:"]')].map((m) => ({
      want: { left: Number.parseFloat(m.style.left), top: Number.parseFloat(m.style.top) },
      box: r(m),
    }))
    return { img: r(img), natural: { w: img.naturalWidth, h: img.naturalHeight }, layer: r(layer), scrim: r(scrim), marks }
  })

  say(z.img.w > before.width + 20, `${label}：大图确实更大了（小图 ${before.width.toFixed(0)} → 大图 ${z.img.w.toFixed(0)}）`)
  const ratioWant = z.natural.w / z.natural.h
  say(Math.abs(z.img.w / z.img.h - ratioWant) / ratioWant < 0.01, `${label}：大图也等比`)
  say(
    z.img.t > z.layer.t && z.img.b < z.layer.b && z.img.l > z.layer.l && z.img.r < z.layer.r,
    `${label}：大图完整落在这一层里（四周都没出界）`,
  )
  say(
    Math.abs(z.layer.l - z.scrim.l) < 1 && Math.abs(z.layer.t - z.scrim.t) < 1 && Math.abs(z.layer.w - z.scrim.w) < 1,
    `${label}：放大层的舞台 = 模态框那层遮罩（没跑到整页上去）`,
  )
  for (const m of z.marks) {
    const leftGot = ((m.box.l - z.img.l) / z.img.w) * 100
    const topGot = ((m.box.t - z.img.t) / z.img.h) * 100
    say(
      Math.abs(leftGot - m.want.left) < 0.4 && Math.abs(topGot - m.want.top) < 0.4,
      `${label}：大图上的框仍在 ${m.want.left}%/${m.want.top}%（量到 ${leftGot.toFixed(1)}%/${topGot.toFixed(1)}%）`,
    )
  }

  /* Esc：先收这一层（带倒放），**模态框不许跟着关** */
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  say((await bigFig.count()) === 0, `${label}：Esc 收起了放大层`)
  say(await page.locator('[role="dialog"]').isVisible(), `${label}：模态框还在（Esc 没顺手把它关了）`)

  /* 点大图自己也收 */
  await figBtn.click()
  await page.waitForTimeout(320)
  await bigFig.locator('img').click()
  await page.waitForTimeout(400)
  say((await bigFig.count()) === 0, `${label}：点大图收起`)

  /* 点图上外面那一圈也收 */
  await figBtn.click()
  await page.waitForTimeout(320)
  const layerBox = (await bigFig.boundingBox()).y
  await page.mouse.click(30, Math.max(layerBox - 6, 24))
  await page.waitForTimeout(400)
  say((await bigFig.count()) === 0, `${label}：点图外面那一圈收起`)

  /* 页面里不该留下"点一下放大"那种提示文字（作者 2026-10-16 点名不要） */
  say(
    (await page.getByText('点一下放大').count()) === 0,
    `${label}：没有"点一下放大"那颗提示`,
  )

  /* ---- 收起那一趟：不许闪（作者 2026-10-16：「图片缩小的时候，页面会闪」） ----
     两个判据都是"逐帧看"出来的：
       ① **那一层底从头到尾一直都在**（它的 opacity 恒为 1）—— 动它（或动父级）就会
          在透明那一帧把背后整块界面透出来，那就是闪；
       ② **图在卸掉之前就已经几乎看不见**（末帧 opacity 趋 0）—— 否则卸层那一帧
          要让一个 626×750 的框塌掉，接缝上会被看见。 */
  await figBtn.click()
  await page.waitForTimeout(300)
  const frames = await page.evaluate(() => {
    const el = document.querySelector('button[aria-label^="收起放大图"] > div')
    /* ★ 别用 `el.closest('div')`：那个 div 是 `.frame` 的父级、可能**就是 `.frame`
       自己**（`GuideFigure` 的最外层就是 div）—— 拿错了元素就量到空背景。
       认 `.zoomLayer` 的类名片段更稳。 */
    const layer = [...el.parentElement.parentElement.classList].some((c) => c.includes('zoomLayer'))
      ? el.parentElement.parentElement
      : el.closest('div[class*="zoomLayer"]')
    const veil = getComputedStyle(layer, '::before')
    return { cls: layer.className, veilBg: veil.backgroundColor, veilOpacity: veil.opacity }
  })
  say(
    /zoomLayer/.test(String(frames.cls)),
    `${label}：量到的是那一层（${String(frames.cls).slice(0, 40)}）`,
  )
  say(
    frames.veilOpacity === '1' && frames.veilBg !== 'rgba(0, 0, 0, 0)',
    `${label}：遮罩底是独立的一层（${frames.veilBg}）—— 收起时不跟着变透明`,
  )

  await bigFig.locator('img').click()
  const tail = await page.evaluate(async () => {
    const el = document.querySelector('button[aria-label^="收起放大图"] > div')
    if (el === null) return { gone: true }
    const a = el.getAnimations()[0]
    if (a === undefined) return { gone: true }
    await a.finished.catch(() => {})
    return { gone: false, after: getComputedStyle(el).opacity, transform: getComputedStyle(el).transform }
  })
  say(
    tail.gone === true || Number(tail.after) < 0.15,
    `${label}：卸层那一刻图已经淡没了（opacity ${tail.gone ? '—（已卸）' : tail.after}）`,
  )
  await page.waitForTimeout(150)
  say((await bigFig.count()) === 0, `${label}：收起这一趟干干净净结束`)

  /* ---------------------------------------------------------------------------
   * 五：缩放（作者：「触摸板双指那种把它缩放」「没有触摸板的也应该支持滚轮放大」
   *     「下次点击放大打开又是默认的大小」）
   * ------------------------------------------------------------------------- */

  const frame = page.locator('button[aria-label^="收起放大图"] > div')
  /** 当前缩放倍率（从 computed transform 的矩阵里读，别信自己的想法） */
  const scaleOf = () =>
    page.evaluate(() => {
      const el = document.querySelector('button[aria-label^="收起放大图"] > div')
      const m = new DOMMatrixReadOnly(getComputedStyle(el).transform)
      return m.a
    })
  /** 图上第一块标注相对**图**的百分比（缩放前后都必须还是它） */
  const markPct = () =>
    page.evaluate(() => {
      const layer = document.querySelector('button[aria-label^="收起放大图"]')
      const img = layer.querySelector('img')
      const mark = layer.querySelector('span[style*="left:"][style*="top:"]')
      /* 成品图（方案 A）前端不叠画框 ⇒ 没有 mark 可量，交回 null 让调用方跳过 */
      if (mark === null) return null
      const ir = img.getBoundingClientRect()
      const mr = mark.getBoundingClientRect()
      const cs = getComputedStyle(mark)
      /* 线画在 box-shadow 上（border 会被引擎钳到 1px，见 CSS 那条注释）。
         ★ 别按空格 split：computed 值是 `rgba(32, 178, 90, 1) 0px 0px 0px 1.44px` ——
         颜色里就有空格。用一个"四个长度"的正则取第 4 个（扩散半径） */
      const four = /(-?[\d.]+)px\s+(-?[\d.]+)px\s+(-?[\d.]+)px\s+(-?[\d.]+)px/.exec(cs.boxShadow)
      return {
        left: ((mr.left - ir.left) / ir.width) * 100,
        top: ((mr.top - ir.top) / ir.height) * 100,
        stroke: four === null ? 0 : Number.parseFloat(four[4]),
        radius: Number.parseFloat(cs.borderTopLeftRadius),
      }
    })

  await figBtn.click()
  await page.waitForTimeout(320)
  const w0 = (await frame.boundingBox()).width
  /* 「默认大小」= 与**这一档本来就该有的大小**一致（`z.img.w` = 放大层第一次量到的宽度） */
  say(
    (await scaleOf()) === 1 && Math.abs(w0 - z.img.w) < 2,
    `${label}：刚打开是默认大小（${w0.toFixed(0)}px，这一档本来就该是 ${z.img.w.toFixed(0)}px）`,
  )

  /* ① 普通滚轮（鼠标） */
  const c = await frame.boundingBox()
  await page.mouse.move(c.x + c.width / 2, c.y + c.height / 2)
  await page.mouse.wheel(0, -300)
  await page.waitForTimeout(180)
  const k1 = await scaleOf()
  say(k1 > 1.1, `${label}：滚轮向上放大（缩放 ${k1.toFixed(2)}）`)

  /* ② 触摸板双指：WKWebView 里以 `wheel + ctrlKey` 到达（合成一条真事件走同一条路） */
  await page.evaluate(() => {
    const el = document.querySelector('button[aria-label^="收起放大图"]')
    for (let i = 0; i < 6; i++) {
      el.dispatchEvent(
        new WheelEvent('wheel', { deltaY: -40, ctrlKey: true, bubbles: true, cancelable: true, clientX: 400, clientY: 400 }),
      )
    }
  })
  await page.waitForTimeout(200)
  const k2 = await scaleOf()
  say(k2 > k1 * 1.05, `${label}：触控板双指（ctrl+wheel）继续放大（${k1.toFixed(2)} → ${k2.toFixed(2)}）`)

  /* ③ 放大之后：框线**视觉粗细不变**、框仍钉在同一个百分比上。
     ★ 方案 A 起：图是**成品图**、前端不叠画框 ⇒ 这里没有 `.mark` 可量，
     这两条判据自动跳过（框的粗细 / 位置已经烧进像素，缩放跟着图一起缩，不会有"框线变粗"的问题）。 */
  const pct = await markPct()
  if (pct === null) {
    say(true, `${label}：成品图不叠画框 ⇒ 框线粗细/位置两条判据不适用（框在图里）`)
  } else {
    say(
      Math.abs(pct.stroke * k2 - 2) < 0.25 && Math.abs(pct.radius * k2 - 8) < 0.5,
      `${label}：放大后框线视觉粗细不变（${(pct.stroke * k2).toFixed(2)}px 线 / ${(pct.radius * k2).toFixed(1)}px 圆角）`,
    )
    say(
      Math.abs(pct.left - 18.8) < 0.5 && Math.abs(pct.top - 58.2) < 0.5,
      `${label}：放大后框仍在 ${18.8}%/${58.2}%（量到 ${pct.left.toFixed(1)}%/${pct.top.toFixed(1)}%）`,
    )
  }

  /* ④ 收起（这一趟是"缩着收"）—— 走完不残留 */
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  say((await bigFig.count()) === 0, `${label}：放大状态下 Esc 照样收得起`)

  /* ⑤ 再打开：回到默认大小（不记忆上次缩放 —— 作者点名要的） */
  await figBtn.click()
  await page.waitForTimeout(320)
  const k3 = await scaleOf()
  const w3 = (await frame.boundingBox()).width
  say(
    k3 === 1 && Math.abs(w3 - z.img.w) < 2,
    `${label}：再打开又是默认大小（缩放 ${k3}，${w3.toFixed(0)}px）`,
  )
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
}

await page.setViewportSize({ width: 1080, height: 900 })
await page.waitForTimeout(300)
await zoomProbe('1080×900', 'guide-zoom-1080x900')

/* 小窗再走一遍（作者就是嫌小窗里图太小） */
await page.setViewportSize({ width: 620, height: 520 })
await page.waitForTimeout(300)
await zoomProbe('620×520', 'guide-zoom-620x520')

/* ---------------------------------------------------------------------------
 * 六：图**取不到**（云端失效 / 地址在但 404）—— 作者定的落法：
 *     「要不然就留空吧，不要让它显示」⇒ **整块不画**，右列文字自己占满整幅宽
 * ------------------------------------------------------------------------- */

const broken = await browser.newPage({ viewport: { width: 1080, height: 900 }, deviceScaleFactor: 2 })
/* ★ 故意把两张引导图打成 404（真机上就是"云端失效"那种）。
   浏览器会照常在 console 里记一条网络失败 —— 那不是我们代码的噪音，滤掉；
   我们自己的 `onError` **不许**再往 console 写东西 */
broken.on('console', (m) => {
  if (m.type() !== 'error') return
  const text = m.text()
  if (/favicon\.ico/.test(m.location?.()?.url ?? '') || /Failed to load resource/.test(text)) return
  /* 浏览器演示里那几条"未实现的接口"是既有且无害的（与别的探针同一份白名单） */
  if (/未实现的接口/.test(text)) return
  problems.push(`404 那一屏 console.error: ${text}`)
})
broken.on('pageerror', (e) => problems.push(`404 那一屏 pageerror: ${e.message}`))
await broken.route('**/assets/guides/**', (r) => r.fulfill({ status: 404, body: 'gone' }))

await broken.goto(url)
await broken.getByRole('button', { name: '设置' }).click()
await broken.getByRole('button', { name: '去系统里改…' }).click()
await broken.getByText('改 .3mf 的默认打开方式').waitFor({ timeout: 8000 })
await broken.waitForTimeout(600)

const empty = await broken.evaluate(() => {
  const dialog = document.querySelector('[role="dialog"]')
  const p = dialog.querySelector('p')
  const d = dialog.getBoundingClientRect()
  const pr = p.getBoundingClientRect()
  return {
    imgs: dialog.querySelectorAll('img').length,
    figs: dialog.querySelectorAll('button[aria-label^="放大看："]').length,
    statusLeft: pr.left - d.left,
    statusWidth: pr.width,
    dialogWidth: d.width,
  }
})
say(empty.imgs === 0 && empty.figs === 0, `404：那一块整块不画（框里 ${empty.imgs} 张图 / ${empty.figs} 个可点区）`)
say(
  empty.statusLeft < 60 && empty.statusWidth > empty.dialogWidth * 0.8,
  `404：文字自己占了整幅宽（左缘 ${empty.statusLeft.toFixed(0)}px、宽 ${empty.statusWidth.toFixed(0)} / 框 ${empty.dialogWidth.toFixed(0)}）`,
)
await broken.screenshot({ path: 'tmp-shots/guide-broken-404.png' })

console.log(problems.length === 0 ? '\n全绿' : `\n问题 ${problems.length} 条：\n${problems.join('\n')}`)
await browser.close()
