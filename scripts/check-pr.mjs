#!/usr/bin/env node
/**
 * `scripts/pr.mjs` 三条判据的自检。
 *
 * 闸本身没人验 = 闸会静默失效（`docs/GIT-WORKFLOW.md` §1「闸门失效是静默的」）。
 * 这里只测**纯函数**：分支要不要 PR、标题取什么、标题对不上怎么说。
 * 真正的 git / gh 调用不在这里测 —— 那需要仓库和网络，属于人工演练。
 */

import {
  refusalFor,
  titleFor,
  prTitleFrom,
  NO_PR_PATTERNS,
  ALLOWED_PREFIX,
} from './pr.mjs'

let bad = 0
function check(name, cond, detail) {
  if (cond) {
    console.log(`  ✓ ${name}`)
  } else {
    bad += 1
    console.log(`  ✗ ${name}${detail ? ` —— ${detail}` : ''}`)
  }
}

console.log('\n分支：进号不建 PR\n')
check('main 被拒', refusalFor('main') !== null, refusalFor('main'))
check(
  'chore/next-preview-2 被拒且理由说清「不建 PR」',
  /不建 PR/.test(refusalFor('chore/next-preview-2') ?? ''),
  refusalFor('chore/next-preview-2'),
)
check('chore/version-0.0.12 被拒', refusalFor('chore/version-0.0.12') !== null)
check('chore/next-dev-version 被拒', refusalFor('chore/next-dev-version') !== null)

console.log('\n分支：该建 PR 的放行\n')
check('feat/x 放行', refusalFor('feat/x') === null, refusalFor('feat/x'))
check('fix/x 放行', refusalFor('fix/x') === null, refusalFor('fix/x'))
check('chore/gitignore 放行', refusalFor('chore/gitignore') === null, refusalFor('chore/gitignore'))
check('没前缀的被拒', refusalFor('my-branch') !== null, refusalFor('my-branch'))
check('空分支名被拒', refusalFor('') !== null)

console.log('\n标题：PR 自己的属性\n')
check('多空格归一', titleFor('  a   b  ') === 'a b', titleFor('  a   b  '))
check('原样保留中文与标点', titleFor('test: 解耦 + 0.0.11') === 'test: 解耦 + 0.0.11')
check('空串不炸', titleFor('') === '')
check(
  '显式标题优先于提交',
  prTitleFrom({ explicit: '显式标题', firstCommit: '最早提交', branch: 'feat/x' }) === '显式标题',
)
check(
  '没显式标题时取最早提交（后续提交不参与）',
  prTitleFrom({ explicit: null, firstCommit: '最早提交', branch: 'feat/x' }) === '最早提交',
)
check(
  '连最早提交都没有才退回分支名',
  prTitleFrom({ explicit: null, firstCommit: '', branch: 'feat/x' }) === 'feat/x',
)

console.log('\n反空转：判据表不许被清空\n')
check('NO_PR_PATTERNS 至少 4 条', NO_PR_PATTERNS.length >= 4, String(NO_PR_PATTERNS.length))
check('ALLOWED_PREFIX 至少 6 个', ALLOWED_PREFIX.length >= 6, String(ALLOWED_PREFIX.length))
check(
  '进号那条确实在表里',
  NO_PR_PATTERNS.some((p) => p.re.test('chore/next-preview-2')),
)

console.log('')
if (bad > 0) {
  console.error(`✗ pr.mjs 自检失败：${bad} 条\n`)
  process.exit(1)
}
console.log('✓ pr.mjs 自检通过：分支 / 标题 / 反空转 都对\n')
