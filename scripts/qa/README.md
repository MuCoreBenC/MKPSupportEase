# 工作台质检跑道

在真浏览器里点工作台，出带截图的报告。

```bash
npm run qa                      # 全部 spec（没起服务就自己起 dev:workbench）
npm run qa -- --only=order      # 只跑文件名含 order 的
npm run qa -- --legacy          # 顺带跑 scripts/probes 里那批老探针（要先起客户端预览 4173）
npm run qa -- --url=http://localhost:5322   # 复用已在跑的服务
```

产物在 `tmp-shots/qa-<时间戳>/`（不进库）：`report.html` 给人看、`report.md` 给 AI 与留档、
`shots/*.png` 是每一步的截图。**报告里失败的那几条就是"用户看见的坏"**，不是实现细节。

## 为什么打在 dev server 上

工作台的假后端只在 dev 装（`src/workbench/main.tsx` 的 `import.meta.env.DEV && ?mock=1`），
生产构建里那句被摇掉 —— 夹具不许冒充真后端。所以跑的是 `vite --mode workbench`（5322）。

## 加一条 spec（20 行）

`scripts/qa/specs/` 下建一个 `.mjs`，下划线开头的是共用步骤、不会被当成 spec：

```js
import { openParams, pickParam } from './_lib.mjs'

export const title = '报告里这一节叫什么'

export default async function (t, page, url) {
  await openParams(page, url)          // 进参数台、选 A1 标准版
  await pickParam(page, '擦料塔位置 X') // 点一条参数
  t.check('这一步对不对', 1 + 1 === 2, '失败的详情写这儿')
  await t.shot(page, '这一刻的截图叫什么')
}
```

三条纪律：

1. **只用界面上看得见的东西**找元素：按钮文字、`role`、`aria-label`。不写 class 名 ——
   改一次样式就全红，而红的原因跟"用户能不能用"没关系。
2. **一个 check 说一句人话**，红了要能直接贴给人看。
3. **不是所有东西都要断言**：拿不准但对用户有影响的，用 `t.note('…')` 记一条观察，
   它进报告的「观察」行，不判红。

## 和 AI 巡检的关系

机械断言答的是「点下去坏没坏」；**顺不顺手**答不了。那一半走
`docs/QA-WORKBENCH.md` 的用户故事走查，结论并进同一份报告的观察行。
