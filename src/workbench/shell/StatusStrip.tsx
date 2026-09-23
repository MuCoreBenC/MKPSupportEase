/**
 * 结构带 1：状态条（26px）。
 *
 * 它回答**一个问题**：配方与产物现在是什么关系。三格，各自只说一件事：
 *
 * | 格 | 说什么 |
 * |---|---|
 * | `RECIPE` | 开发源数据在哪 + 有没有未保存的改动 |
 * | `ARTIFACT` | 产物在哪 + 是不是最新的 |
 * | `LAST BUILD` | 全店最近一次生成的时间 |
 *
 * 三条带**分工不重叠**（doc §7）：这一条只说「配方↔产物」，文件头说「这是什么、能做什么」，
 * 底部条说「当前这一屏的计数与选中」。同一个数字在两条带上出现过一次之后就不再出现，
 * 否则两处对不上的时候没人知道该信哪个。
 *
 * 状态词一律从 `words` 查（doc §13）——**这个文件里不出现任何中文状态词**。
 */
import type { ArtifactState, SaveState, Words } from '../api'

interface Props {
  recipePath: string
  distPath: string
  save: SaveState
  artifact: ArtifactState
  lastBuild: string | null
  words: Words
}

export function StatusStrip({
  recipePath,
  distPath,
  save,
  artifact,
  lastBuild,
  words,
}: Props) {
  const saveWord = words.save[save]
  const artifactWord = words.artifact[artifact]

  return (
    <div className="wb-strip">
      <Cell label="RECIPE" path={recipePath}>
        <span
          className="wb-dot"
          data-state={save === 'dirty' ? 'warn' : 'ok'}
          aria-hidden
        />
        <span>{saveWord.label}</span>
      </Cell>

      <Cell label="ARTIFACT" path={distPath}>
        <span
          className="wb-dot"
          data-state={artifact === 'fresh' ? 'ok' : 'warn'}
          aria-hidden
        />
        {/* 解释句挂 title：状态词只有三个字，「现在该做什么」得说得出来 */}
        <span title={artifactWord.explain ?? undefined}>{artifactWord.label}</span>
      </Cell>

      <Cell
        label="LAST BUILD"
        /* 这个时间是**全店**的，不是某一版的。不写清楚的话，
           用户会拿它当「我刚改的那一版生成过了」的证据 */
        path={lastBuild ?? '—'}
        title="全店最近一次生成，只作辅助；每个版本各自的时间在生成视角里"
      />
    </div>
  )
}

function Cell({
  label,
  path,
  title,
  children,
}: {
  label: string
  path: string
  title?: string
  children?: React.ReactNode
}) {
  return (
    <div className="wb-strip__cell" title={title}>
      <span className="wb-strip__key">{label}</span>
      <span className="wb-strip__path" title={path}>
        {path}
      </span>
      {children && <span className="wb-strip__state">{children}</span>}
    </div>
  )
}
