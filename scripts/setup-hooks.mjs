/**
 * 装闸门：把 core.hooksPath 指到版本化的 scripts/hooks，然后自检并报告。
 *
 * 由 package.json 的 prepare 钩子在 npm install 时自动执行，所以**任何情况下都不能让
 * npm install 失败** —— 不在 git 仓库里、git 不在 PATH 里，都只警告后正常退出。
 *
 * 为什么是 core.hooksPath 而不是复制进 .git/hooks：
 * 复制是单向快照，改了 scripts/hooks 不重跑安装脚本就等于没改，而且没有任何提示。
 * 指路径只有一份真相源，漂移问题从根上消失。
 */
import { execFileSync } from 'node:child_process'
import { chmodSync, existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const HOOKS_DIR = 'scripts/hooks'
const EXPECTED = ['pre-commit', 'pre-merge-commit', 'pre-push']

const ok = (m) => console.log(`  ok    ${m}`)
const warn = (m) => console.log(`  警告  ${m}`)
const info = (m) => console.log(`        ${m}`)

/** 跑 git，失败返回 null 而不是抛 —— prepare 阶段任何异常都不该让 npm install 挂掉 */
const git = (args) => {
  try {
    return execFileSync('git', args, { encoding: 'utf8', stdio: 'pipe' }).trim()
  } catch {
    return null
  }
}

console.log('\nMKP 闸门安装')

const root = git(['rev-parse', '--show-toplevel'])
if (!root) {
  warn('不在 git 仓库里（或 git 不可用），跳过闸门安装。')
  info('拿到仓库后跑一次：node scripts/setup-hooks.mjs')
  process.exit(0)
}

/* 1. 钩子文件齐不齐 */
const missing = EXPECTED.filter((n) => !existsSync(join(root, HOOKS_DIR, n)))
if (missing.length) {
  warn(`${HOOKS_DIR}/ 下缺少：${missing.join('、')}`)
  info('闸门会因此不完整，请确认工作区是完整 clone。')
} else {
  ok(`钩子齐全：${EXPECTED.join('、')}`)
}

/* 1b. 可执行位。
      这一步是从试验场学来的：那边四个钩子在磁盘和 git 索引里都是 644，
      于是 git 只打一行 hint（"hook was ignored because it's not set as executable"）
      就照常提交 —— core.hooksPath 指对了、文件也齐，但七道闸一道都没跑。
      **闸门失效是静默的**，所以这里既修磁盘权限，也检查 git 索引里的 mode：
      索引里存 644 的话，clone 到别的机器又会变回不可执行。 */
const notExec = []
for (const n of EXPECTED.filter((n) => existsSync(join(root, HOOKS_DIR, n)))) {
  const p = join(root, HOOKS_DIR, n)
  if (!(statSync(p).mode & 0o111)) {
    try {
      chmodSync(p, 0o755)
    } catch {
      notExec.push(n)
    }
  }
}
if (notExec.length) {
  warn(`这些钩子不可执行且修不动：${notExec.join('、')}`)
  info('git 会静默忽略不可执行的钩子 —— 闸门等于不存在。手动 chmod +x 后重跑本脚本。')
} else {
  ok('钩子可执行位正常')
}

/* 索引里的 mode 才是跟着 clone 走的那一份，644 要显眼报出来 */
const indexModes = git(['ls-files', '-s', HOOKS_DIR])
if (indexModes) {
  const flat = indexModes
    .split('\n')
    .filter((l) => l.startsWith('100644'))
    .map((l) => l.split('\t')[1])
  if (flat.length) {
    warn(`git 索引里这些钩子是 100644（不可执行）：${flat.join('、')}`)
    info(`修：git update-index --chmod=+x ${HOOKS_DIR}/* 然后提交。否则别的机器 clone 出来闸门是空的。`)
  }
}

/* 1c. 换行符。
      这一步是给 Windows 那台机器准备的：Git for Windows 默认 core.autocrlf=true，
      clone 时把 LF 换成 CRLF。钩子是 `#!/bin/sh` 脚本，一旦变成 CRLF，shebang 就成了
      `/bin/sh\r` —— 内核找不到这个解释器，钩子**静默不执行**。
      与"缺可执行位"是同一类失效，只是成因不同，所以同样交给机器查。
      仓库根的 .gitattributes 已经把这几类文件钉成 eol=lf，这里是第二道确认：
      万一有人手工改了 core.autocrlf 或绕过了 attributes，装钩子时就能看见。 */
const crlf = []
for (const n of EXPECTED.filter((n) => existsSync(join(root, HOOKS_DIR, n)))) {
  try {
    if (readFileSync(join(root, HOOKS_DIR, n), 'utf8').includes('\r\n')) crlf.push(n)
  } catch {
    /* 读不动就算了，这一步只是体检 */
  }
}
if (crlf.length) {
  warn(`这些钩子是 CRLF 换行：${crlf.join('、')}`)
  info('shebang 会变成 /bin/sh\\r，git 会静默跳过 —— 闸门等于不存在。')
  info(`修：git rm --cached -r . && git reset --hard（.gitattributes 会把它们重新取成 LF）`)
} else {
  ok('钩子换行符正常（LF）')
}

/* 2. 指路径。相对路径由 git 解析为「相对工作树根」，所以换机器也不用改 */
if (git(['config', 'core.hooksPath', HOOKS_DIR]) === null) {
  warn('写 core.hooksPath 失败，闸门未生效。')
  process.exit(0)
}
const actual = git(['config', '--get', 'core.hooksPath'])
if (actual === HOOKS_DIR) {
  ok(`core.hooksPath = ${actual}`)
} else {
  warn(`core.hooksPath 期望 ${HOOKS_DIR}，实际 ${actual ?? '（空）'}`)
}

/* 3. Windows 上钩子是 #!/bin/sh，要有 sh 才跑得起来 */
if (process.platform === 'win32') {
  if (canRun('sh', ['-c', 'exit 0'])) ok('sh 可用（Git for Windows 自带），#!/bin/sh 钩子能执行')
  else warn('找不到 sh，钩子可能不会执行。装 Git for Windows 或把它的 usr/bin 加进 PATH。')
}

/* 4. 上一代复制进 .git/hooks 的真钩子已经失效了，提示但不自动删。
      注意不能用 `git rev-parse --git-path hooks` 拿这个目录 —— core.hooksPath 一设好，
      它返回的就是 scripts/hooks 本身，于是会把我们自己的钩子报成"上一代残留"。
      要的是物理上的 .git/hooks，所以从 git 目录直接拼。 */
const gitDir = git(['rev-parse', '--absolute-git-dir'])
const legacyHooks = gitDir ? join(gitDir, 'hooks') : null
if (legacyHooks && existsSync(legacyHooks)) {
  const stale = readdirSync(legacyHooks).filter(
    (n) => !n.endsWith('.sample') && statSync(join(legacyHooks, n)).isFile(),
  )
  if (stale.length) {
    warn(`.git/hooks 下还留着上一代复制进去的钩子：${stale.join('、')}`)
    info('core.hooksPath 生效后它们不再被调用，确认无误可以手动删掉。')
  } else {
    ok('.git/hooks 下没有遗留的旧钩子')
  }
}

/* 5. 发版标记残留会让闸①/①b 一直放行 main 上的提交，必须显眼报出来 */
if (gitDir && existsSync(join(gitDir, 'RELEASE_IN_PROGRESS'))) {
  warn('检测到 .git/RELEASE_IN_PROGRESS 残留 —— 闸①/①b 现在会放行 main 上的提交！')
  info('上次 npm run release 没走完。确认没在发版中，删掉这个文件。')
}

console.log(`
现在生效的闸门（判据写在 ${HOOKS_DIR}/ 里）：

  ①  pre-commit        main 上直接提交              ALLOW_COMMIT_ON_MAIN=1
  ①b pre-merge-commit  合并进 main                  ALLOW_COMMIT_ON_MAIN=1
  ②  pre-push          对 main 的任何 push          ALLOW_PUSH_MAIN=1
  ③  pre-push          非快进（force）推送          ALLOW_FORCE_PUSH=1
  ④  pre-push          删除 main 或 tag              ALLOW_DELETE_REMOTE=1
  ⑤  pre-push          tag 名与 package.json 不符   ALLOW_TAG_MISMATCH=1
  ⑥  pre-commit        分支名没有合法前缀           ALLOW_ANY_BRANCH=1
  ⑦  pre-commit        密钥文件 / >2MB 非 public/   ALLOW_BIG_OR_SECRET=1

main 只能由 PR 推进（squash 合并），本地永远不 push main。
发版走一条命令：npm run release
逃生开关一律留痕到 .git/bypass.log，下次发版时会当着你的面复述。
细节见 docs/GIT-WORKFLOW.md
`)

function canRun(cmd, args) {
  try {
    execFileSync(cmd, args, { stdio: 'ignore' })
    return true
  } catch {
    return false
  }
}
