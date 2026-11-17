/*
 * 参数台 · 名称与说明（一）：偏移 + 擦料。
 * 顺序 = 页签 → 分组 → 组内顺序（param_registry.toml 的 layout.order）。
 * 「名称」那一行只在建议改名时才写 labelSug。
 */
;(() => {
  const P = 'registry'
  const A = window.COPY_AUDIT_DATA.items
  const SRC = 'param_registry.toml'
  A.push(
    {
      page: P, sec: '偏移 › 空间偏移', where: 'X 轴偏移', label: 'X 轴偏移', src: `${SRC} · toolhead.offset.x`,
      cur: '涂胶笔尖相对喷嘴的 X 方向偏移量 (mm)，负值向左正值向右',
      type: ['B', 'D'], advice: 'edit', sug: '笔尖相对喷嘴的横向偏差，正值向右。',
      why: '单位在单位栏；「负值向左」与「正值向右」是一句话的两半，留一半就够懂。',
    },
    {
      page: P, sec: '偏移 › 空间偏移', where: 'Y 轴偏移', label: 'Y 轴偏移', src: `${SRC} · toolhead.offset.y`,
      cur: '涂胶笔尖相对喷嘴的 Y 方向偏移量 (mm)，负值向前正值向后',
      type: ['B', 'D'], advice: 'edit', sug: '笔尖相对喷嘴的前后偏差，正值向后。',
      why: '与 X 同一条写法，两条一起改才像一套。',
    },
    {
      page: P, sec: '偏移 › 空间偏移', where: 'Z 轴偏移', label: 'Z 轴偏移', src: `${SRC} · toolhead.offset.z`,
      cur: '涂胶笔尖相对喷嘴的 Z 方向偏移量 (mm)，校准的时候左侧为正（向上抬）右侧为负（向下压），一般不允许小于0。',
      type: ['B', 'D', 'F'], advice: 'edit', sug: '笔尖相对喷嘴的高度差，正值抬起。',
      why: '哪侧为正、允不允许负数由范围与校准页管；说明只答「正负是什么意思」。',
    },
    {
      page: P, sec: '偏移 › 运动与挤出', where: '涂胶速度限制', label: '涂胶速度限制', src: `${SRC} · toolhead.speed_limit`,
      cur: '涂胶笔的移动速度 (mm/s)，过高可能导致涂胶不均匀或丢步',
      type: ['D'], advice: 'edit', sug: '涂胶笔的移动速度，太快会涂不匀。',
      why: '「丢步」是运动控制术语，用户能感知到的后果只有「涂不匀」。',
    },
    {
      page: P, sec: '擦料 › 擦料方式', where: '擦料方式', label: '擦料方式', src: `${SRC} · wiping.have_wiping_components`,
      cur: '擦料塔在指定位置生成塔体进行擦拭；圆盘擦拭为测试方案，禁用。',
      type: ['A', 'D'], advice: 'edit', sug: '用擦料塔擦拭；圆盘擦拭是测试方案，已禁用。',
      why: '「在指定位置生成塔体」是机制；这条要说的是「只有一条路可走」。',
    },
    {
      page: P, sec: '擦料 › 擦料塔位置与打印', where: '擦料塔X坐标', label: '擦料塔X坐标', src: `${SRC} · wiping.wiper_x`,
      cur: '擦料塔在热床上的X坐标位置 (mm)',
      type: ['B'], advice: 'edit', sug: '擦料塔在热床上的 X 位置。',
      why: '「坐标位置」重了一个词；单位在单位栏。',
    },
    {
      page: P, sec: '擦料 › 擦料塔位置与打印', where: '擦料塔Y坐标', label: '擦料塔Y坐标', src: `${SRC} · wiping.wiper_y`,
      cur: '擦料塔在热床上的Y坐标位置 (mm)',
      type: ['B'], advice: 'edit', sug: '擦料塔在热床上的 Y 位置。',
      why: '同上。',
    },
    {
      page: P, sec: '擦料 › 擦料塔位置与打印', where: '擦料塔挤出倍率', label: '擦料塔挤出倍率', src: `${SRC} · wiping.tower_extrude_ratio`,
      cur: '擦料塔出胶量比例，值越大出胶越多。会被处理器动态计算覆盖',
      type: ['A', 'F'], advice: 'edit', sug: '塔的出胶倍数，越大越多；生成时会重算。',
      why: '「处理器动态计算覆盖」是内部实现；用户要知道的是「改它未必原样生效」。',
    },
    {
      page: P, sec: '擦料 › 擦料塔位置与打印', where: '塔刮擦速度', label: '塔刮擦速度', src: `${SRC} · wiping.tower_travel_speed`,
      cur: '擦料塔刮擦移动速度，范围50~500mm/s',
      type: ['B'], advice: 'edit', sug: '塔的擦嘴移动速度，50–500 mm/s。',
      why: '「范围」两字由范围本身说明；数字贴着单位更好读。',
    },
    {
      page: P, sec: '擦料 › 支撑', where: '支撑体挤出倍率', label: '支撑体挤出倍率', src: `${SRC} · wiping.support_extrusion_multiplier`,
      cur: '支撑体的挤出倍率，影响支撑硬度和拆除难度',
      type: ['D', 'E'], advice: 'edit', sug: '支撑的出胶倍数，越高越硬、越难拆。',
      why: '「挤出倍率」与「支撑体」都是工程词；把两个后果放进一句。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '擦料塔首层流量', label: '擦料塔首层流量', src: `${SRC} · wiping.tower_first_layer_flow`,
      cur: '擦料塔首层护套和斜肋的挤出流量比例，1.0为标准流量',
      type: ['B', 'E'], advice: 'edit', sug: '首层护套与斜肋的出胶比例，1.0 为标准。',
      why: '「挤出流量比例」在同族参数里叫「出胶比例」，统一。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '擦料塔速度与质量', label: '擦料塔速度与质量', labelSug: '塔层高与擦嘴',
      src: `${SRC} · wiping.fast_tower_mode`,
      cur: '快速：最大层高打印，仅涂胶面层慢速擦嘴 | 跟随模型：层高与模型一致 | 自定义：自定义层高和擦嘴策略',
      type: ['D', 'E'], advice: 'edit', sug: '快速、跟随模型，或自己定层高与擦嘴。',
      why: '名称说的是「速度与质量」，实际选的是层高与擦嘴策略；说明里三个选项名控件上已经列着。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '塔体打印速度', label: '塔体打印速度', src: `${SRC} · wiping.wipetower_speed`,
      cur: '非涂胶面层的塔体打印速度',
      type: ['F'], advice: 'edit', sug: '塔非涂胶面的打印速度。',
      why: '「层」与「塔体」叠在一起，读起来多一道拆解。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '涂胶面层速度', label: '涂胶面层速度', src: `${SRC} · wiping.tower_glue_layer_speed`,
      cur: '涂胶面层的塔体打印速度',
      type: ['F'], advice: 'edit', sug: '塔涂胶面的打印速度。',
      why: '同上，与上一条成对。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '塔层高策略', label: '塔层高策略', src: `${SRC} · wiping.tower_custom_layer_height`,
      cur: '自定义模式下的擦料塔层高策略',
      type: ['E', 'F'], advice: 'edit', sug: '自定义模式下的塔层高。',
      why: '「策略」是给一组规则用的词，这一格就是一个层高。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '擦嘴策略', label: '擦嘴策略', src: `${SRC} · wiping.tower_wipe_mode`,
      cur: '开启：每层都擦嘴 | 关闭：包括涂胶面都不擦 | 自动：仅涂胶面层擦嘴',
      type: ['D', 'E'], advice: 'edit', sug: '开启每层擦；自动只擦涂胶面；关闭都不擦。',
      why: '「选项名：解释」读起来像配置文件；按「什么时候擦」排更好扫。',
    },
    {
      page: P, sec: '擦料 › 安全与高级参数', where: '塔安全Z偏移', label: '塔安全Z偏移', src: `${SRC} · wiping.tower_safe_z_offset`,
      cur: '擦料塔安全Z偏移距离，范围0.3~5.0mm',
      type: ['B'], advice: 'edit', sug: '塔安全抬起的高度，0.3–5 mm。',
      why: '「偏移距离」与「范围」都是能省的词。',
    },
    {
      page: P, sec: '擦料 › 圆盘动作控制', where: '错开模式', label: '错开模式', src: `${SRC} · wiping.disk_stagger_mode`,
      cur: '关闭：不旋转 | 开启：始终错开 | 自动：小层高时自动错开',
      type: ['D', 'E'], advice: 'edit', sug: '关闭不旋转；开启始终错开；自动在小层高时错开。',
      why: '去掉「选项名：」的句式，整条读成一句话。',
    },
    {
      page: P, sec: '擦料 › 圆盘动作控制', where: '错开角度', label: '错开角度', src: `${SRC} · wiping.disk_stagger_angle`,
      cur: '每层圆盘旋转偏移的角度（0-30°）',
      type: ['B', 'F'], advice: 'edit', sug: '每层圆盘多转的角度，0–30°。',
      why: '括号里的范围拆出来；「旋转偏移」压成「多转」。',
    },
    {
      page: P, sec: '擦料 › 圆盘动作控制', where: '最大偏移角度', label: '最大偏移角度', src: `${SRC} · wiping.disk_stagger_max_angle`,
      cur: '摆动的最大偏移角度（10-90°）',
      type: ['B'], advice: 'edit', sug: '摆动能到的最多角度，10–90°。',
      why: '「最大偏移角度」与名称重复；说明说清它管的是摆动幅度。',
    },
    {
      page: P, sec: '擦料 › 圆盘动作控制', where: '摆动模式', label: '摆动模式', src: `${SRC} · wiping.disk_stagger_swing_mode`,
      cur: '旋转楼梯：单向累加到达最大值后回卷 | 锯齿摆动：到最大值后反向折返',
      type: ['D'], advice: 'edit', sug: '旋转楼梯单向累加；锯齿摆动到顶折返。',
      why: '两句都在讲「到最大值怎么办」，压成四个字。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '外围结构', label: '外围结构', src: `${SRC} · wiping.outer_structure`,
      cur: '选择擦料塔外围结构类型：无=不生成外围；斜肋外墙=在塔外侧生成斜肋支撑；护套=包裹塔体外围防止倒塌',
      type: ['A', 'D'], advice: 'edit', sug: '无、斜肋外墙或护套。',
      why: '三种结构怎么长是机制；选项名本身已经说清，说明只列三条路。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '斜肋打印速度', label: '斜肋打印速度', src: `${SRC} · wiping.tower_rib_speed`,
      cur: '跟随塔：使用塔体打印速度 | 自定义：指定斜肋打印速度',
      type: ['D', 'E'], advice: 'edit', sug: '跟随塔速，或自己定斜肋速度。',
      why: '去掉「选项名：解释」。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '自定义速度', label: '自定义速度', src: `${SRC} · wiping.tower_rib_speed_value`,
      cur: '斜肋外墙的自定义打印速度',
      type: ['F'], advice: 'edit', sug: '斜肋外墙的打印速度。',
      why: '「自定义」由上一格的模式决定，这一格只管数字。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '斜肋额外长度', label: '斜肋额外长度', src: `${SRC} · wiping.rib_extra_length`,
      cur: '斜肋外墙的额外延伸长度，0为不延伸',
      type: ['D'], advice: 'edit', sug: '斜肋比塔多伸出的长度，0 不延伸。',
      why: '「额外延伸长度」五个字说一个「多伸出」。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '斜肋宽度', label: '斜肋宽度', src: `${SRC} · wiping.rib_width`,
      cur: '斜肋外墙的宽度',
      type: ['F'], advice: 'edit', sug: '斜肋的宽度。',
      why: '名称已经写着「斜肋」，不必再解释。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '外墙倒圆角', label: '外墙倒圆角', src: `${SRC} · wiping.rib_fillet_wall`,
      cur: '斜肋外墙是否倒圆角',
      type: ['E'], advice: 'edit', sug: '开启后斜肋外墙转角倒圆。',
      why: '开关的参数说明不用是非问句，写开启后发生什么。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '护套打印速度', label: '护套打印速度', src: `${SRC} · wiping.tower_sheath_speed`,
      cur: '护套的打印速度',
      type: [], advice: 'keep',
      why: '名称与说明同义，但这一句就是最短的准确说法。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '护套自动启用高度', label: '护套自动启用高度', src: `${SRC} · wiping.sheath_enable_height`,
      cur: '自动模式下，模型高度超过此值时启用护套',
      type: ['D'], advice: 'edit', sug: '模型高过这个值时加护套（自动模式）。',
      why: '「启用」换「加」；条件先说，模式放括号里。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '护套底层膨胀量', label: '护套底层膨胀量', src: `${SRC} · wiping.sheath_base_expand`,
      cur: '护套底层相对塔体的单边膨胀量',
      type: ['E'], advice: 'edit', sug: '护套底层比塔每边多出的宽度。',
      why: '「相对…的单边膨胀量」是几何术语。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '护套壁宽', label: '护套壁宽', src: `${SRC} · wiping.sheath_wall_width`,
      cur: '护套壁的宽度',
      type: ['F'], advice: 'edit', sug: '护套壁的厚度。',
      why: '「宽度」与名称重复；壁用「厚度」更自然。',
    },
    {
      page: P, sec: '擦料 › 塔结构加强', where: '护套收敛层数', label: '护套收敛层数', src: `${SRC} · wiping.sheath_converge_layers`,
      cur: '护套从底层膨胀逐渐收敛到塔体外轮廓的层数',
      type: ['D', 'E'], advice: 'edit', sug: '护套收回到塔外轮廓用几层。',
      why: '「膨胀逐渐收敛」压成一个「收回」。',
    },
  )
})()
