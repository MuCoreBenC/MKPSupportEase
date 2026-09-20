/**
 * 建 / 更新 GitHub 服务端 ruleset —— 第二层拦截。
 *
 * 为什么是脚本而不是在网页上勾：规则本身要进版本控制。
 * "我记得在设置页勾过 require PR" 和"闸门装好了"是同一类幻觉 —— 第一层的七道闸
 * 在试验场就是这么空转了几个月（文件在、可执行位不在）。规则写成代码，改动才有 diff、
 * 才能 review、才能在换机器或换仓库时原样重放。
 *
 * 幂等：按 name 找，存在就 PUT 覆盖，不存在才 POST。可以反复跑。
 *
 * 用法：node scripts/setup-ruleset.mjs
 */
import { execFileSync } from 'node:child_process'

/* 必须通过的状态检查。名字与 .github/workflows/ci.yml 里两个 job 的 `name:` 逐字对应 ——
   写错不会报错，只会永远等一个不存在的检查。
   （建 ruleset 那一轮这里是空的：CI 还不存在，勾了第一个 PR 永远合不进去。） */
const REQUIRED_CHECKS = ['web', 'rust']

const MAIN_RULESET = 'main-pr-only'
const TAG_RULESET = 'tags-v-no-delete'

const ok = (m) => console.log(`  ok    ${m}`)
const info = (m) => console.log(`        ${m}`)
const die = (m) => {
  console.error(`\n  失败：${m}\n`)
  process.exit(1)
}

/** 调 gh api。body 为 undefined 时是 GET。 */
const api = (path, method = 'GET', body) => {
  const args = ['api', path, '-H', 'Accept: application/vnd.github+json']
  if (method !== 'GET') args.push('-X', method, '--input', '-')
  try {
    const out = execFileSync('gh', args, {
      input: body === undefined ? undefined : JSON.stringify(body),
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    return out.trim() ? JSON.parse(out) : null
  } catch (e) {
    const detail = (e.stderr || e.stdout || e.message || '').toString().trim()
    die(`gh api ${method} ${path}\n  ${detail.split('\n').join('\n  ')}`)
  }
}

console.log('\nMKP 服务端 ruleset')

let repo
try {
  repo = JSON.parse(
    execFileSync('gh', ['repo', 'view', '--json', 'nameWithOwner,visibility'], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
    }),
  )
} catch {
  die('读不到当前仓库（gh repo view 失败）。确认在仓库目录里、origin 已配置、gh 已登录。')
}
const slug = repo.nameWithOwner
ok(`仓库 ${slug}（${repo.visibility}）`)

/* main：PR-only。
   - pull_request 的 required_approving_review_count: 0 —— 单人开发，要的是"必须走 PR"这条路径，
     不是"必须有人批"。批准数留 0，PR 才能自己合掉。
   - required_review_thread_resolution —— 评论必须解决完才能合。
   - required_linear_history —— 配合 squash 合并，历史永远是一条直线。
   - non_fast_forward —— 挡 force push。
   - deletion —— 挡删分支。
   - bypass_actors 为空数组 = 不允许任何人绕过，仓库 admin（也就是我自己）也不行。
     这是整套设计的关键一刀：第一层的逃生开关留给"我知道我在干什么"的本地操作，
     第二层不留任何口子，否则两层会同时失效。 */
const mainRules = [
  {
    type: 'pull_request',
    parameters: {
      required_approving_review_count: 0,
      dismiss_stale_reviews_on_push: false,
      require_code_owner_review: false,
      require_last_push_approval: false,
      required_review_thread_resolution: true,
    },
  },
  { type: 'required_linear_history' },
  { type: 'non_fast_forward' },
  { type: 'deletion' },
]

if (REQUIRED_CHECKS.length) {
  mainRules.push({
    type: 'required_status_checks',
    parameters: {
      strict_required_status_checks_policy: true,
      required_status_checks: REQUIRED_CHECKS.map((context) => ({ context })),
    },
  })
}

const mainBody = {
  name: MAIN_RULESET,
  target: 'branch',
  enforcement: 'active',
  bypass_actors: [],
  conditions: { ref_name: { include: ['~DEFAULT_BRANCH'], exclude: [] } },
  rules: mainRules,
}

/* 发布 tag：只挡删除。
   tag 名与 annotated 的判据留在本地闸⑤ —— 服务端没有"tag 必须是 annotated"这种规则，
   两层各管自己管得住的那部分，不假装覆盖。 */
const tagBody = {
  name: TAG_RULESET,
  target: 'tag',
  enforcement: 'active',
  bypass_actors: [],
  conditions: { ref_name: { include: ['refs/tags/v*'], exclude: [] } },
  rules: [{ type: 'deletion' }],
}

const existing = api(`/repos/${slug}/rulesets`) ?? []

const upsert = (body) => {
  const hit = existing.find((r) => r.name === body.name)
  const res = hit
    ? api(`/repos/${slug}/rulesets/${hit.id}`, 'PUT', body)
    : api(`/repos/${slug}/rulesets`, 'POST', body)
  ok(`${hit ? '更新' : '新建'} ruleset「${body.name}」(id ${res.id})`)
  return res.id
}

const mainId = upsert(mainBody)
const tagId = upsert(tagBody)

/* 回读确认 —— 写完就当成功是另一种"规则没落到机器上" */
console.log('\n回读确认：')
for (const id of [mainId, tagId]) {
  const r = api(`/repos/${slug}/rulesets/${id}`)
  const types = r.rules.map((x) => x.type).join('、')
  info(`${r.name}  target=${r.target}  ${r.enforcement}  bypass_actors=${r.bypass_actors?.length ?? 0}`)
  info(`  include=${r.conditions?.ref_name?.include?.join('、') ?? '-'}`)
  info(`  rules=${types}`)
}

if (!REQUIRED_CHECKS.length) {
  console.log('')
  info('注意：required status checks 还没挂上（REQUIRED_CHECKS 是空的）。')
  info('CI 存在之后把它填成 [\'web\', \'rust\'] 再跑一次本脚本。')
}

console.log(`
现在 main 只能这样推进：

  git push -u origin HEAD && gh pr create   →   CI 绿   →   gh pr merge --squash --delete-branch

细节见 docs/GIT-WORKFLOW.md
`)
