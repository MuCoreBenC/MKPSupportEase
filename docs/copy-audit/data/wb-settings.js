/* 设置：外观 → 发布账户 → 云端源 → 本地测试源 → 本地预览发布 → 端口 → 数据根。 */
;(() => {
  const P = 'settings'
  const A = window.COPY_AUDIT_DATA.items
  const S = 'src/workbench/views/SettingsPage.tsx'
  A.push(
    {
      page: P, sec: '外观', where: '跟随系统', src: `${S}:66`,
      cur: '系统切深色就跟着深色 —— 没选过的时候就是这一档',
      type: ['B', 'F'], advice: 'edit', sug: '系统切深色就跟着深色。',
      why: '后半句在说默认值；默认值看选中状态就知道。',
    },
    {
      page: P, sec: '外观', where: '浅色 / 深色', src: `${S}:67`,
      cur: '一直是浅色，不管系统怎么设',
      type: ['D'], advice: 'edit', sug: '不管系统，一直浅色。',
      why: '条件前置，三个词就能说完。',
    },
    {
      page: P, sec: '外观', where: '各记各的', src: `${S}:524`,
      cur: '各记各的一档（两个应用的数据根是分开的，谁也不去读对方那棵树）。工作台这一窗；客户端那扇窗在它自己的「设置 → 外观」里。',
      type: ['A', 'B', 'D'], advice: 'edit', sug: '工作台与客户端各记各的。',
      why: '数据根、读不读对方的树是机制；「哪扇窗」那句由标题已经说明。',
    },
    {
      page: P, sec: '发布账户', where: '必填校验', src: `${S}:144`,
      cur: '仓库地址与用户名都要填',
      type: [], advice: 'keep',
      why: '两个字段、一句说清。',
    },
    {
      page: P, sec: '发布账户', where: '保存成功', src: `${S}:161`,
      cur: '已保存 GitHub 发布账户（Token 存进本机凭据文件）',
      type: ['A', 'B'], advice: 'edit', sug: '已保存 GitHub 发布账户。',
      why: 'Token 存哪是机制；用户只关心存没存上。',
    },
    {
      page: P, sec: '发布账户', where: '清除成功', src: `${S}:177`,
      cur: '已清除 GitHub 发布账户（配置 + 凭据）',
      type: ['B'], advice: 'edit', sug: '已清除 GitHub 发布账户。',
      why: '括号在列清了两样东西 —— 结果一样是「清了」。',
    },
    {
      page: P, sec: '发布账户', where: '取不到 Token', src: `${S}:209`,
      cur: 'GitHub 这会儿取不到 Token（可能已被清掉）',
      type: ['B', 'F'], advice: 'edit', sug: '取不到 GitHub 的 Token。',
      why: '「这会儿」是口语；「可能已被清掉」是猜测，用户点一下就知道。',
    },
    {
      page: P, sec: '发布账户', where: '剪贴板不可用', src: `${S}:230`,
      cur: '剪贴板用不了 —— 请手动选中明文再复制',
      type: ['B'], advice: 'edit', sug: '剪贴板不可用，请手选复制。',
      why: '失败句先说没做成什么，再给替代动作。',
    },
    {
      page: P, sec: '本地测试源', where: '启动成功', src: `${S}:287`,
      cur: '已启动 —— http://localhost:5321/',
      type: ['B'], advice: 'edit', sug: '已启动：http://localhost:5321/',
      why: '破折号换成冒号，地址作为一个值更清楚。',
    },
    {
      page: P, sec: '本地测试源', where: '停止成功', src: `${S}:305`,
      cur: '已停，服务端口也让出来了。',
      type: ['F'], advice: 'edit', sug: '已停止，端口已释放。',
      why: '「让出来了」是口语；与「已停」统一成一个词。',
    },
    {
      page: P, sec: '本地预览发布', where: '开始编译', src: `${S}:336`,
      cur: '已开始编译并交付……（一两分钟）',
      type: ['B'], advice: 'edit', sug: '开始编译并交付，约一两分钟。',
      why: '括号改逗号；「已开始」与「正在」不混用。',
    },
    {
      page: P, sec: '本地预览发布', where: '交付完成', src: `${S}:365`,
      cur: '交付完成，开发版本已滚到下一版。',
      type: ['F'], advice: 'edit', sug: '已交付，开发版本进入下一版。',
      why: '「滚到」是黑话；结果句「已交付」开头。',
    },
    {
      page: P, sec: '本地测试源', where: '占用已停', src: `${S}:412`,
      cur: '占用者已停 —— 现在可以点「启动」了。',
      type: ['B', 'F'], advice: 'edit', sug: '占用已停，可以启动。',
      why: '两句并在逗号里；「现在可以…了」是口语时间词。',
    },
    {
      page: P, sec: '本地测试源', where: '还有占用', src: `${S}:413`,
      cur: '停是停了，但还有 2 条占用没处理。',
      type: ['F'], advice: 'edit', sug: '还有 2 条占用没处理。',
      why: '前半句是复述上一步的结果。',
    },
    {
      page: P, sec: '发布账户', where: '保存需要重启', src: `${S}:466`,
      cur: '已保存（重启 dev / 重打正式包后客户端才吃得到）',
      type: ['A', 'B', 'D'], advice: 'edit', sug: '已保存，重启客户端后生效。',
      why: 'dev / 正式包两种方式是实现；用户要知道的是「要重启」。',
    },
    {
      page: P, sec: '云端源', where: '重新读取后', src: `${S}:498`,
      cur: '已从磁盘重读',
      type: ['A', 'E'], advice: 'edit', sug: '已重新读取。',
      why: '「从磁盘」是实现来源；动作是「重新读取」。',
    },
    {
      page: P, sec: '发布账户', where: '读不到 remote', src: `${S}:579`,
      cur: '读不到 git remote（这台机器上取不到 git / 不是仓库）',
      type: ['B', 'D', 'E'], advice: 'edit', sug: '读不到 git remote。',
      why: '括号里在给两种可能，用户都没法在这里修。',
    },
    {
      page: P, sec: '发布账户', where: '与 remote 一致', src: `${S}:584`,
      cur: '—— 与已配置的发布账户一致 ✓',
      type: ['B'], advice: 'edit', sug: '与发布账户一致。',
      why: '开头的破折号是在接上一行；「已配置的」可省。',
    },
    {
      page: P, sec: '发布账户', where: '与 remote 不一致', src: `${S}:585`,
      cur: '—— 与已配置的发布账户不一致，发布前请确认',
      type: ['B'], advice: 'edit', sug: '与发布账户不一致，发布前请确认。',
      why: '同上；「请确认」这句要留 —— 它会改变人的动作。',
    },
    {
      page: P, sec: '发布账户', where: 'Token 输入 tooltip', src: `${S}:660`,
      cur: '已配置。点击可填入新的 Token（不改则保持原值）',
      type: ['B', 'D'], advice: 'edit', sug: '点击换新的 Token。',
      why: '「不改则保持原值」是行为承诺；tooltip 只说点击会发生什么。',
    },
    {
      page: P, sec: '云端源', where: '区块说明', src: `${S}:767`,
      cur: '发布产物发到哪 —— 工作台唯一一处；客户端构建时编进去',
      type: ['A', 'B', 'E', 'H'], advice: 'layout', sug: '两个地址框与「哪一段用于什么」该由分组和标签说清；这段说明可以整块删。',
      why: '读者之所以需要这句，是因为主源 / 镜像两格长得一样、只有小字能区分 —— 那是布局欠账。',
    },
    {
      page: P, sec: '云端源', where: '地址说明', src: `${S}:774`,
      cur: '填仓库地址就够（仓库地址或克隆地址）——',
      type: ['B', 'D'], advice: 'edit', sug: '填仓库地址即可。',
      why: '括号在解释两种地址都行，破折号后面没说完。',
    },
    {
      page: P, sec: '云端源', where: 'GitHub 没配', src: `${S}:838`,
      cur: 'GitHub 还没配 —— 客户端构建时不会注入默认源（下载会如实说「没配」）',
      type: ['A', 'B', 'D', 'G'], advice: 'edit', sug: '还没配 GitHub，客户端构建时不会带默认源。',
      why: '「如实说」是元话术；后半句已经说清后果。',
    },
    {
      page: P, sec: '云端源', where: 'Gitee 没配', src: `${S}:844`,
      cur: '没配（客户端不出现 Gitee 档）',
      type: ['B'], advice: 'edit', sug: '没配，客户端不显示 Gitee。',
      why: '括号去掉；「档」换成「显示」。',
    },
    {
      page: P, sec: '本地测试源', where: '区块说明', src: `${S}:866`,
      cur: '只读 —— 下面每一格都是盘上与运行时的真实读数',
      type: ['A', 'B', 'G'], advice: 'edit', sug: '只读，显示盘上与运行时的实际读数。',
      why: '「每一格都是真实读数」是在自证数据可信。',
    },
    {
      page: P, sec: '本地测试源', where: '认不出占用进程', src: `${S}:1080`,
      cur: '认不出占着它的进程 —— 请在那个终端里自己停掉它',
      type: ['B', 'D'], advice: 'edit', sug: '认不出占用进程，请在终端里停掉。',
      why: '「那个终端」指向不明；「自己」是多余的强调。',
    },
    {
      page: P, sec: '本地测试源', where: '停掉占用', src: `${S}:1081`,
      cur: '把它停掉（后端会先确认这个端口还是它的，再动手）',
      type: ['A', 'B'], advice: 'edit', sug: '停掉占用进程。',
      why: '括号里是安全保证；按钮名本身已经说清动作。',
    },
    {
      page: P, sec: '本地预览发布', where: '阻断：未跟踪文件', src: `${S}:1114`,
      cur: '以下文件尚未跟踪，无法确认是否属于本次构建：',
      type: ['D'], advice: 'edit', sug: '这些文件还没提交，无法确认是否属于本次构建：',
      why: '「尚未跟踪」是 git 术语；「还没提交」谁都能懂。',
    },
    {
      page: P, sec: '本地预览发布', where: '阻断：下一步', src: `${S}:1123`,
      cur: '请先提交它们或加进 .gitignore，再重新执行。',
      type: ['D'], advice: 'keep',
      why: '给了两条具体出路，删了用户真不知道怎么办。',
    },
    {
      page: P, sec: '本地测试源', where: '只起服务', src: `${S}:1137`,
      cur: '只起服务，不碰你的客户端 —— 地址自己填一次：',
      type: ['B', 'D'], advice: 'edit', sug: '只起服务，客户端地址要手填。',
      why: '「不碰你的客户端」是安抚；冒号后面那串地址由旁边的读数给。',
    },
    {
      page: P, sec: '本地测试源', where: '一起起', src: `${S}:1143`,
      cur: '一起起（它会自动把源指过来，注入），',
      type: ['A', 'B'], advice: 'edit', sug: '会一并启动并把源指过来。',
      why: '「注入」是实现；这句嵌在长句中间，单独说更清楚。',
    },
    {
      page: P, sec: '本地测试源', where: '端口被占', src: `${S}:1151`,
      cur: '端口被占着的话，上面会先把它摆出来（谁占着、PID 多少），',
      type: ['B', 'D', 'F'], advice: 'edit', sug: '端口被占时会列出占用进程。',
      why: '「摆出来」是口语；括号里的细节由那张表自己显示。',
    },
    {
      page: P, sec: '数据根', where: '唯一真相源', src: `${S}:1162`,
      cur: '唯一真相源 —— 没有第二候选、不 fallback',
      type: ['A', 'B', 'E'], advice: 'edit', sug: '唯一数据根。',
      why: '「不 fallback」是代码词；前半句已经说清。',
    },
    {
      page: P, sec: '数据根', where: '配方本', src: `${S}:1183`,
      cur: '开发源数据、草稿与回收站住的地方。',
      type: ['F'], advice: 'edit', sug: '开发数据、草稿与回收站。',
      why: '「住的地方」是拟人；「开发源数据」两个「源/数据」重了。',
    },
    {
      page: P, sec: '本地预览发布', where: '账本缺记录', src: `${S}:991`,
      cur: '（账本缺记录，可补）',
      type: ['B', 'E'], advice: 'edit', sug: '账本缺记录，可以补。',
      why: '括号补丁；「账本」在这一页已经是常用词，留。',
    },
  )
})()
