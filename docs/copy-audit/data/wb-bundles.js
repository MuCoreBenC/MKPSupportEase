/* 套餐：列表 → 右栏（谁在用 / 装了什么）→ 内容框 → 指向框 → 新建/编辑/复制/删除框。 */
;(() => {
  const P = 'bundles'
  const A = window.COPY_AUDIT_DATA.items
  const S = 'src/workbench/views/BundlesPage.tsx'
  A.push(
    {
      page: P, sec: '列表', where: '加载中', src: `${S}:399`,
      cur: '正在读套餐定义……',
      type: ['E'], advice: 'edit', sug: '正在读套餐…',
      why: '「定义」是数据口径；界面上的省略号统一用三个点。',
    },
    {
      page: P, sec: '列表', where: '筛选占位', src: `${S}:412`,
      cur: '筛套餐（id / 显示名）',
      type: ['B'], advice: 'edit', sug: '筛套餐',
      why: '括号在列字段名；能按什么筛，输入一次就知道。',
    },
    {
      page: P, sec: '列表', where: '被多个版本引用', src: `${S}:454`,
      cur: '好几个版本指着它',
      type: ['F'], advice: 'edit', sug: '3 个版本在用',
      why: '「好几个」在该给数的地方含糊；「指着」是黑话，别处已用「用」。',
    },
    {
      page: P, sec: '列表', where: '只有一个版本引用', src: `${S}:458`,
      cur: '1 个版本用它',
      type: [], advice: 'keep',
      why: '数与动作都说清了。',
    },
    {
      page: P, sec: '列表', where: '没有版本引用', src: `${S}:463`,
      cur: '没有版本指着它（大家都走机型默认）',
      type: ['B', 'F'], advice: 'edit', sug: '没有版本用它，都走机型默认。',
      why: '「大家」是拟人；括号去掉，逗号更顺。',
    },
    {
      page: P, sec: '列表', where: '文件数 tooltip', src: `${S}:471`,
      cur: '这份套餐装了几份文件（MKP 预设 + 切片器）',
      type: ['B', 'F'], advice: 'edit', sug: '装了几份文件。',
      why: '哪两类文件，右边的清单已经列着；「这份」是旧白话。',
    },
    {
      page: P, sec: '列表', where: '含归档文件的标记', src: `${S}:475`,
      cur: '含归档文件',
      type: [], advice: 'keep',
      why: '四个字，标出这一条的特殊之处。',
    },
    {
      page: P, sec: '列表', where: '底部计数', src: `${S}:490`,
      cur: '39 个套餐 · 装了 120 个文件',
      type: [], advice: 'keep',
      why: '两个读数，一句。',
    },
    {
      page: P, sec: '右栏', where: '空态', src: `${S}:498`,
      cur: '选一个套餐',
      type: [], advice: 'keep',
      why: '最短的指路。',
    },
    {
      page: P, sec: '右栏', where: '含仅归档文件时的注', src: `${S}:528`,
      cur: '不会报错',
      type: ['G', 'F'], advice: 'edit', sug: '',
      why: '「不会报错」是在安抚 —— 用户没在担心报错，这一句本身制造担心。',
    },
    {
      page: P, sec: '右栏', where: 'MKP 预设的说明', src: `${S}:537`,
      cur: '资产库里的 MKP 预设类。文件在不在都能挂 —— 「有没有生成」是生成页的状态，不拦选用。',
      type: ['A', 'B', 'D'], advice: 'edit', sug: 'MKP 预设都能挂，没生成也能选。',
      why: '「资产库里的类」「不拦选用」都是系统口径；结论是「都能挂」。',
    },
    {
      page: P, sec: '右栏', where: '没生成的产物', src: `${S}:550`,
      cur: '这一版的产物还没生成',
      type: [], advice: 'keep',
      why: '说清状态，不解释为什么。',
    },
    {
      page: P, sec: '内容框', where: '顶部说明', src: `${S}:646`,
      cur: '一个框里两个页签（MKP 预设 / 切片器）—— 和机型页那颗「改套餐内容…」打开的是同一个框。改动先进草稿，保存后才写进仓库（updatedAt 会盖上今天）；存一条 BBS 都没有的套餐存不进去 —— MKP 与 BBS 必须成套配发。',
      type: ['A', 'B', 'D', 'E', 'H'], advice: 'layout', sug: '这句在替两个入口做解释 —— 两处铺同一行「MKP 与 BBS 成套」就够，其余进折叠区。',
      why: '同一件事说三遍（框、草稿、成套），说明入口与分组没排好，不是这一句太长。',
    },
    {
      page: P, sec: '右栏', where: '引用 chip tooltip', src: `${S}:653`,
      cur: '点 chip 跳到那个版本，点 × 取消它自己的指向（它会回到机型默认）。右键 chip 也是这两项。',
      type: ['B', 'D'], advice: 'edit', sug: '点 chip 去那个版本，点 × 取消它的指向。',
      why: '右键与「回到机型默认」是同一件事的补充，chip 上已经有 ×。',
    },
    {
      page: P, sec: '右栏', where: '指向关系说明', src: `${S}:707`,
      cur: '一个套餐可以被多个版本指（不限机型）；一个版本只指一个套餐 —— 一版一套。真源关系就是一版一套：改了指向之后那一版的生成与发布跟着走（后端判据，不在前端）。',
      type: ['A', 'D', 'E'], advice: 'edit', sug: '一个版本只用一个套餐；改指向后，这一版的生成与发布跟着走。',
      why: '「真源关系」「后端判据」是实现；四句压成一句后果。',
    },
    {
      page: P, sec: '指向框', where: '底部说明', src: `${S}:755`,
      cur: '「确认指向」先给你看影响预览（哪些版本会被改、原来指着谁），确认后进草稿，保存才写进仓库。',
      type: ['B', 'D'], advice: 'edit', sug: '确认前先列出会影响哪些版本。',
      why: '「进草稿 / 保存才写」是每个框都成立的机制。',
    },
    {
      page: P, sec: '新建框', where: 'id 占位', src: `${S}:821`,
      cur: '如 A1_NEW（不许空白或 /）',
      type: ['B'], advice: 'edit', sug: '如 A1_NEW，不能有空行或 /。',
      why: '括号里是校验规则，用逗号接更顺。',
    },
    {
      page: P, sec: '新建框', where: '显示名占位', src: `${S}:831`,
      cur: '留空 = 用 id',
      type: [], advice: 'keep',
      why: '一行两个信息，没有可删的。',
    },
    {
      page: P, sec: '新建框', where: '切片器要求', src: `${S}:841`,
      cur: '至少一条切片器预设 —— MKP 与 BBS 必须成套配发，缺了存不进去。内容以后也能改。',
      type: ['B', 'D'], advice: 'edit', sug: 'MKP 与 BBS 要成套，先加一条 BBS 预设。',
      why: '「缺了存不进去」与「以后也能改」是同一件事的两面。',
    },
    {
      page: P, sec: '编辑框', where: '改 id 的说明', src: `${S}:883`,
      cur: '改 id 会把机型文件里的 defaultBundle / recommendedBundle 一并重指（后端做）—— 它同时是 bundles.toml 里的键。',
      type: ['A', 'B', 'E'], advice: 'edit', sug: '改 id 会同步改掉机型和版本里的引用。',
      why: '字段名、文件表名、「后端做」都是机制；用户要知道的是「别处会跟着变」。',
    },
    {
      page: P, sec: '复制框', where: '说明', src: `${S}:933`,
      cur: '内容与归属照抄；复制出来的那份不被任何版本指着 —— 指向要人显式改。',
      type: ['B', 'D', 'F'], advice: 'edit', sug: '内容照抄；副本不改任何版本的指向。',
      why: '「要人显式改」是流程提醒；前半句已经说了结果。',
    },
    {
      page: P, sec: '指向框', where: '机型范围', src: `${S}:968`,
      cur: '不限机型：本机型的排前面只是方便，别家的也能选。',
      type: ['F'], advice: 'edit', sug: '不限机型，本机型的排在前面。',
      why: '「只是方便」「别家的也能选」在解释排序理由。',
    },
    {
      page: P, sec: '删除框', where: '没人引用时', src: `${S}:1078`,
      cur: '没人指着它 —— 删掉之后这份登记就没了（文件本体还在资产库）',
      type: ['B', 'D', 'E'], advice: 'edit', sug: '删除后这份套餐登记没了，文件还在资产库。',
      why: '「登记」「本体」是数据口径；两句说清删什么、留什么。',
    },
    {
      page: P, sec: '删除框', where: '有引用时', src: `${S}:1098`,
      cur: '还有地方指着它，不能删',
      type: ['F'], advice: 'keep',
      why: '一句挡回，够用。',
    },
    {
      page: P, sec: '指向框', where: '确认底部', src: `${S}:1059`,
      cur: '确认后进草稿，撤销能整条退回来。',
      type: ['D'], advice: 'edit', sug: '确认后进草稿，可以撤销。',
      why: '「整条退回来」换成「可以撤销」，与全站一致。',
    },
  )
})()
