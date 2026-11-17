/* 公共组件：弹框按钮、字段提示、右键菜单、改动框、排序框。这些字在每个页面都会出现。 */
;(() => {
  const P = 'kit'
  const A = window.COPY_AUDIT_DATA.items
  A.push(
    {
      page: P, sec: '右键菜单', where: '这一行没有动作时', src: 'src/workbench/components/menu/ContextMenu.tsx:225',
      cur: '这一行没有可用的操作',
      type: ['F'], advice: 'edit', sug: '没有可用的操作。',
      why: '「这一行」是多余的定位；菜单已经长在那一行上。',
    },
    {
      page: P, sec: '字段', where: '适用机型的摘要', src: 'src/workbench/components/machineSummary.ts:19',
      cur: '全部机型（5 台全勾）',
      type: ['B'], advice: 'edit', sug: '全部机型 · 共 5 台',
      why: '括号补解释；换成中点，读起来是一个读数而不是一句注解。',
    },
    {
      page: P, sec: '字段', where: '部分机型时的摘要', src: 'src/workbench/components/machineSummary.ts:21',
      cur: 'A1、P1S、P2S —— 5 台',
      type: ['B'], advice: 'edit', sug: 'A1、P1S、P2S 等 5 台',
      why: '破折号换成「等」，一眼看出这是节选。',
    },
    {
      page: P, sec: '字段', where: '数字框超范围', src: 'src/workbench/components/field/NumberField.tsx:322',
      cur: '超出范围 1 ~ 100',
      type: ['D'], advice: 'edit', sug: '超出 1–100',
      why: '「范围」两个字在数字前面是废话。',
    },
    {
      page: P, sec: '字段', where: '数字框 tooltip', src: 'src/workbench/components/field/NumberField.tsx:259',
      cur: '范围 1 ~ 100',
      type: [], advice: 'keep',
      why: 'tooltip 里带「范围」是必要的，否则两个数不知道是什么。',
    },
    {
      page: P, sec: '字段', where: '空值占位', src: 'src/workbench/components/field/TextField.tsx:85',
      cur: '空',
      type: [], advice: 'keep',
      why: '一个字，没有可删的。',
    },
    {
      page: P, sec: '字段', where: '必填标记', src: 'src/workbench/c14/FieldMark.tsx:27',
      cur: '必填，可以之后补',
      type: [], advice: 'keep',
      why: '说清了「不是错误」，作者据此才敢先跳过。',
    },
    {
      page: P, sec: '错误框', where: '底部按钮', src: 'src/workbench/c14/ErrorDialog.tsx:43',
      cur: '复制详情',
      type: [], advice: 'keep',
      why: '动作词开头，四个字。',
    },
    {
      page: P, sec: 'G-code 框', where: '底部说明', src: 'src/workbench/c14/GcodeModal.tsx:98',
      cur: '改动留在草稿里（没有保存）—— 想丢弃这次改动，用下面的「取消」',
      type: ['B', 'D'], advice: 'edit', sug: '改动在草稿里，用「取消」丢弃。',
      why: '括号与破折号各补一句；「取消」就在下面，指路一句就够。',
    },
    {
      page: P, sec: 'G-code 框', where: '没有改动时', src: 'src/workbench/c14/GcodeModal.tsx:110',
      cur: '这一层还没改过 —— 没有可取消的',
      type: ['B', 'F'], advice: 'edit', sug: '这一层没有改动。',
      why: '第二句在解释为什么按钮是灰的，按钮灰着就说明了。',
    },
    {
      page: P, sec: 'G-code 框', where: '取消按钮 tooltip', src: 'src/workbench/c14/GcodeModal.tsx:111',
      cur: '把这一层改回打开框之前的值，然后关框',
      type: ['D'], advice: 'edit', sug: '改回打开前的值并关框。',
      why: '两件事各四个字，压成一句。',
    },
    {
      page: P, sec: '改动框', where: '底部说明', src: 'src/workbench/c14/HistoryModal.tsx:312',
      cur: '保存 = 写进仓库文件（保存本身也能撤销），放弃 = 整条一起丢',
      type: ['B', 'D'], advice: 'edit', sug: '保存写进仓库，放弃整条丢掉。',
      why: '「保存也能撤销」是附加机制，去帮助里说；这里是决定时刻。',
    },
    {
      page: P, sec: '改动框', where: '文件没差异但有内存改动', src: 'src/workbench/c14/HistoryModal.tsx:306',
      cur: '文件层面没有差异 —— 还有 3 处改动只活在内存里，保存会一并写入',
      type: ['A', 'B', 'E'], advice: 'edit', sug: '文件没差异，还有 3 处改动在内存里，保存一并写入。',
      why: '「文件层面」「只活在内存里」是内部口径；拆成三个短句。',
    },
    {
      page: P, sec: '改动框', where: '完全没改动', src: 'src/workbench/c14/HistoryModal.tsx:307',
      cur: '影子与盘上一模一样 —— 没有未保存的改动',
      type: ['B', 'E'], advice: 'edit', sug: '没有未保存的改动。',
      why: '「影子」是内部实现；后半句就是答案。',
    },
    {
      page: P, sec: '排序框', where: '顶部读数', src: 'src/workbench/c14/sort/SortDialog.tsx:84',
      cur: '共 12 项 · 拖动、改数字、按 Alt+↑↓ 都行',
      type: [], advice: 'keep',
      why: '三种操作各说一次，没有重复；「都行」让它读起来是提示不是规则。',
    },
    {
      page: P, sec: '选择框', where: '自定义选项', src: 'src/workbench/c14/PickOrType.tsx:72',
      cur: '打一个新的',
      type: ['F'], advice: 'edit', sug: '新建一项',
      why: '「打」是口语；动作词统一成「新建」。',
    },
  )
})()
