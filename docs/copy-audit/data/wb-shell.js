/* 外壳：导航 / 页头 / 状态栏 / 提示条 / toast。按界面从上到下、从左到右。 */
;(() => {
  const P = 'shell'
  const A = window.COPY_AUDIT_DATA.items
  A.push(
    {
      page: P, sec: '页头 · 副标题', where: '参数台', src: 'src/workbench/App.tsx:992',
      cur: '保存值由后端按三层算好 —— 徽章只在「现在」与它不同时出现',
      type: ['A', 'B', 'G'], advice: 'edit', sug: '',
      why: '讲后端怎么算、徽章什么时候出现 —— 用户看一眼徽章就知道，这句不用留。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '套餐', src: 'src/workbench/App.tsx:994',
      cur: '套餐 = 交付的真源（bundles.toml）—— 装什么、谁在用、改指向，都在这一页',
      type: ['B', 'E'], advice: 'edit', sug: '装了哪些文件、谁在用、怎么改指向。',
      why: '「真源」「bundles.toml」是内部说法；一句说清这页能做什么。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '资产库', src: 'src/workbench/App.tsx:1001',
      cur: '资产域定义 + 引用反查 —— 交付身份改了先进草稿',
      type: ['B', 'E'], advice: 'edit', sug: '文件登记、被谁引用；改动先进草稿。',
      why: '两个内部名词叠在一起，破折号后又是机制。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '标注台', src: 'src/workbench/App.tsx:1003',
      cur: '在图上拖框 —— 导出的就是 marks（百分比坐标，浅深两张图共用一份）',
      type: ['B', 'E'], advice: 'edit', sug: '在图上拖框，浅深两张共用一份标注。',
      why: 'marks 与百分比坐标是开发口径；作者关心的是「两张共用」。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '设置', src: 'src/workbench/App.tsx:1005',
      cur: '只读 —— 数据根与工作台子目录的职责',
      type: ['B'], advice: 'edit', sug: '数据根与各子目录的职责。',
      why: '「只读」在页面上点不动就看得出来；破折号去掉。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '机型与版本', src: 'src/workbench/App.tsx:990',
      cur: '95 台机型 · 12 个版本',
      type: [], advice: 'keep',
      why: '纯读数，短，没有可删的。',
    },
    {
      page: P, sec: '页头 · 副标题', where: '生成与发布', src: 'src/workbench/App.tsx:998',
      cur: '阻断 0 · 待办 0 · 提示 7 · 都过了',
      type: [], advice: 'keep',
      why: '三档读数 + 一句结论，作者做决定就在看这三个数。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '草稿干净时的按钮', src: 'src/workbench/App.tsx:1467',
      cur: '已保存 · 查看改动',
      type: ['C'], advice: 'edit', sug: '查看改动',
      why: '状态已在状态栏；按钮只写它点下去做什么。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '按钮 tooltip（干净）', src: 'src/workbench/App.tsx:1463',
      cur: '草稿和磁盘那一份一致',
      type: ['F'], advice: 'edit', sug: '没有未保存的改动。',
      why: '「那一份」是旧白话；这句本来就是状态。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '按钮 tooltip（有改动）', src: 'src/workbench/App.tsx:1462',
      cur: '看看这 3 处改动 —— 逐行 diff，看完在框里放弃或保存',
      type: ['B', 'D'], advice: 'edit', sug: '看这 3 处改动，在框里放弃或保存。',
      why: '「逐行 diff」是机制；tooltip 只答点了会怎样。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '发布号 chip tooltip', src: 'src/workbench/App.tsx:1394',
      cur: '已发出去的号取账本与交付目录的较大值；只在正式发布时推进',
      type: ['A', 'E'], advice: 'edit', sug: '正式发布才推进这个号。',
      why: '账本与目录的取大值是内部规则，用户只需要知道它什么时候变。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '还没有发布过', src: 'src/workbench/App.tsx:1393',
      cur: '这条发布线还没有正式发布过 —— 第一次发布会是第 1 号',
      type: ['B', 'D'], advice: 'edit', sug: '还没发布过，第一次是第 1 号。',
      why: '「发布线」是内部概念；一句说完。',
    },
    {
      page: P, sec: '导航', where: '出问题时的徽标 tooltip', src: 'src/workbench/App.tsx:1213',
      cur: '有待处理的项，但不挡生成',
      type: ['D'], advice: 'edit', sug: '有待办，不挡生成。',
      why: '「项」与「待处理」都占字；「不挡生成」才是那个意思。',
    },
    {
      page: P, sec: '导航', where: '收起侧边栏 tooltip', src: 'src/workbench/App.tsx:1159',
      cur: '收起侧边栏 —— 只留图标（记着我这次的选择）',
      type: ['B', 'F'], advice: 'edit', sug: '收起侧边栏，只留图标。',
      why: '会不会记住是行为，不用承诺；括号与破折号都去掉。',
    },
    {
      page: P, sec: '导航', where: '展开侧边栏 tooltip', src: 'src/workbench/App.tsx:1158',
      cur: '展开侧边栏（窗口小也能展开）',
      type: ['B'], advice: 'edit', sug: '展开侧边栏。',
      why: '括号在解释能力，用户点一下就知道。',
    },
    {
      page: P, sec: '导航', where: '预设源行', src: 'src/workbench/App.tsx:1247',
      cur: '预设源 / MKPSupportEase/presets',
      type: [], advice: 'keep',
      why: '一行两个信息：这是什么、在哪。窄档只留灯，宽度够才显示。',
    },
    {
      page: P, sec: '提示条', where: '启动失败（presets/ 定位不到）', src: 'src/workbench/App.tsx:1486',
      cur: '确认仓库里的 presets/ 已就位，然后点 重新加载 ，或去 设置页 看数据根。',
      type: ['D'], advice: 'edit', sug: 'presets/ 放回仓库后，点「重新加载」。',
      why: '一条一步就够；设置页那颗入口一直在导航上。',
    },
    {
      page: P, sec: '提示条', where: '外部改过 presets/', src: 'src/workbench/App.tsx:1512',
      cur: '盘上的 presets/ 在这个窗口外面改过了 —— 现在显示的仍是打开时读进来的那一份（参数名、可选项、分组都可能是旧的）。点 重新加载 读进来，不用重启。有未保存的改动时会先问你一句，不会悄悄丢掉。',
      type: ['A', 'B', 'D', 'F'], advice: 'edit', sug: '盘上的 presets/ 改过了，这里还是旧数据。',
      why: '四句压成一句：发生了什么。重载按钮就在旁边，会不会丢东西点的时候才知道。',
    },
    {
      page: P, sec: '提示条', where: '未保存改动按钮 title', src: 'src/workbench/App.tsx:1613',
      cur: '看看这几处都改了什么 —— 每一格从什么值改成什么值',
      type: ['B'], advice: 'edit', sug: '看每一格从什么值改成什么值。',
      why: '前半句是重复按钮名，删。',
    },
    {
      page: P, sec: '状态栏', where: '第三段：快照/上游状态', src: 'src/workbench/App.tsx:1579',
      cur: '已同步到上游',
      type: ['E'], advice: 'edit', sug: '已入库',
      why: '「上游」是内部说法；这一格说的是本地这份有没有进仓库。',
    },
    {
      page: P, sec: '状态栏', where: '请求号 tooltip', src: 'src/workbench/App.tsx:1635',
      cur: '日志里搜 req-xxxx 就是这个动作的全部行（wb_save，失败了）',
      type: ['E', 'G'], advice: 'edit', sug: '日志里搜这个号，能找到这次操作。',
      why: '命令名与「全部行」是开发口径；这是给作者查日志用的，说用途就够。',
    },
    {
      page: P, sec: '状态栏', where: '撤销 / 重做 tooltip', src: 'src/workbench/App.tsx:1590',
      cur: '撤销：落笔位 X（3 处）',
      type: [], advice: 'keep',
      why: '点了会撤什么、撤几处，tooltip 该有的都有了。',
    },
    {
      page: P, sec: '回执', where: '保存成功 toast', src: 'src/workbench/App.tsx:812',
      cur: '已保存 —— 改动已写进仓库文件',
      type: ['B', 'C'], advice: 'edit', sug: '已保存到仓库。',
      why: '成功回执只说做成了什么；写进哪份文件是机制。',
    },
    {
      page: P, sec: '回执', where: '撤销失败 toast', src: 'src/workbench/App.tsx:718',
      cur: '没撤：那一笔是 A1/FAST261111 的改动',
      type: ['F'], advice: 'edit', sug: '没撤：这一笔属于 A1/FAST261111。',
      why: '「那一笔」是旧白话；后半句改成从句更顺。',
    },
    {
      page: P, sec: '页头 · 保存槽', where: '保存禁用时的 tooltip', src: 'src/workbench/App.tsx:942',
      cur: '这一份没有要写回去的改动（和仓库里那份一样）',
      type: ['A', 'B', 'F'], advice: 'edit', sug: '没有要保存的改动。',
      why: '括号在解释为什么没有；「写回去」是机制。',
    },
    {
      page: P, sec: '页面骨架', where: '加载占位', src: 'src/workbench/App.tsx:1026',
      cur: '参数台正在加载',
      type: [], advice: 'keep',
      why: '一句、短、说清在等什么。',
    },
  )
})()
