/*
 * 参数台 · 名称与说明（二）：风扇 + 涂胶 + 切换 + 更多。
 * 顺序 = 页签 → 分组 → 组内顺序。
 */
;(() => {
  const P = 'registry'
  const A = window.COPY_AUDIT_DATA.items
  const SRC = 'param_registry.toml'
  A.push(
    /* ---- 风扇 ---- */
    {
      page: P, sec: '风扇 › 温控与风扇', where: '涂胶期间降温', label: '涂胶期间降温', src: `${SRC} · wiping.nozzle_cooling_flag`,
      cur: '涂胶期间降低喷嘴温度以防止胶水因过热而滴漏',
      type: ['D'], advice: 'edit', sug: '涂胶时降喷嘴温度，防止胶水过热滴漏。',
      why: '「以防止…而」四个虚词，压成一句因果。',
    },
    {
      page: P, sec: '风扇 › 温控与风扇', where: '风扇速度', label: '风扇速度', src: `${SRC} · wiping.fan_speed`,
      cur: '风扇 PWM 速度 (0-255)，控制冷却风量',
      type: ['E'], advice: 'edit', sug: '冷却风量，0–255。',
      why: 'PWM 是实现；「风扇速度」与「冷却风量」在用户眼里是一件事，留一个。',
    },
    {
      page: P, sec: '风扇 › 温控与风扇', where: '额外干燥时间', label: '额外干燥时间', src: `${SRC} · wiping.user_dry_time`,
      cur: '涂胶后额外等待干燥的时间 (秒)，让胶水充分固化',
      type: ['D'], advice: 'edit', sug: '涂胶后多等几秒，让胶水干。',
      why: '单位在单位栏；「充分固化」是书面语。',
    },

    /* ---- 涂胶 · 支撑面熨烫 ---- */
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '支撑面熨烫', label: '支撑面熨烫', src: `${SRC} · wiping.ironing_mode`,
      cur: '控制支撑面上层熨烫的保留/移除：关闭=始终移除熨烫挤出；开启=始终保留；自动=根据挤出量阈值判断',
      type: ['D', 'E'], advice: 'edit', sug: '关闭移除；开启保留；自动按挤出阈值判。',
      why: '「控制…的保留/移除」是给三个选项做的目录；直接说三个选项各做什么。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '熨烫挤出阈值', label: '熨烫挤出阈值', src: `${SRC} · wiping.ironing_e_sum_threshold`,
      cur: '当熨烫段挤出总量超过此阈值时自动移除熨烫（仅在自动模式下生效）',
      type: ['B', 'D'], advice: 'edit', sug: '熨烫挤出超过这个量就移除（自动模式）。',
      why: '括号里的前提提上来；「当…时」压成「就」。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '涂胶面使用熨烫路径', label: '涂胶面使用熨烫路径', src: `${SRC} · wiping.use_ironing_path`,
      cur: '开启后，涂胶面优先使用熨烫段的 XY 路径；熨烫覆盖不足的区域自动回退到支撑面路径',
      type: ['D', 'B'], advice: 'edit', sug: '涂胶面优先走熨烫路径，不够的地方回退支撑面。',
      why: '「XY 路径」「覆盖不足」都能压；一句说清走哪、什么时候退。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '熨烫覆盖阈值', label: '熨烫覆盖阈值', src: `${SRC} · wiping.ironing_coverage_threshold`,
      cur: '熨烫面占支撑面的最低路径长度比例，低于此值时回退到支撑面',
      type: ['D'], advice: 'edit', sug: '熨烫面不够这个比例就回退支撑面。',
      why: '「最低路径长度比例」是定义式说法，用户要的是触发条件。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '熨烫抑制扩展范围', label: '熨烫抑制扩展范围', src: `${SRC} · wiping.ironing_suppress_expand`,
      cur: '已激活熨烫段包围盒向外扩展的距离，0为关闭扩展',
      type: ['B', 'E'], advice: 'edit', sug: '熨烫区向外多扩多少不涂胶，0 关闭。',
      why: '「包围盒」是几何实现；「0 为」换成「0 关闭」更好读。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '熨烫抑制扩展模式', label: '熨烫抑制扩展模式', src: `${SRC} · wiping.ironing_suppress_expand_mode`,
      cur: '熨烫抑制区域的扩展方式：包围盒外扩=沿XY轴均匀外扩；质心半径外扩=从质心向外径向扩展',
      type: ['D', 'E'], advice: 'edit', sug: '包围盒：沿 XY 均匀外扩；质心：从中心向外扩。',
      why: '「径向扩展」「质心半径」都是几何词；选项名后面直接说效果。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '最小支撑涂胶面过滤', label: '最小支撑涂胶面过滤', src: `${SRC} · wiping.min_support_interface_enabled`,
      cur: '启用后过滤掉过小的支撑涂胶面，避免无效涂胶',
      type: ['D', 'F'], advice: 'edit', sug: '过滤过小的支撑涂胶面，避免白涂。',
      why: '「启用后」是开关说明的默认句式；「无效涂胶」换个更直的说法。',
    },
    {
      page: P, sec: '涂胶 › 支撑面熨烫', where: '涂胶稀疏比', label: '涂胶稀疏比', src: `${SRC} · wiping.glue_sparse_ratio`,
      cur: '小喷嘴(≤0.3mm)涂胶路径稀疏比例，0=不稀疏，50=隔行删除(默认)，100=仅保留首尾',
      type: ['B', 'D'], advice: 'edit', sug: '小喷嘴涂胶的稀疏程度：0 不稀、50 隔行、100 只留首尾。',
      why: '三个等号连成一行没有停顿；用冒号 + 逗号分开，扫一眼能找到默认值。',
    },

    /* ---- 涂胶 · 方向补偿 ---- */
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '涂胶方向补偿', label: '涂胶方向补偿', src: `${SRC} · wiping.glue_z_compensation_enabled`,
      cur: '开启后涂胶 Z 会根据 X/Y 位置按 4 角补偿值进行双线性插值，解决特定方向涂不上胶的问题。补偿范围自动外扩 2mm 避免边缘突变',
      type: ['A', 'D'], advice: 'edit', sug: '按四角补偿值修正涂胶 Z，解决某些方向涂不上胶。',
      why: '双线性插值、外扩 2mm 是算法细节；这条要说的是「为什么需要它」。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '补偿范围模式', label: '补偿范围模式', src: `${SRC} · wiping.glue_z_compensation_mode`,
      cur: '热床范围：针对整个热床物理面积补偿，适合床面本身倾斜（范围 ±0.2mm，外扩 2mm）；涂胶面范围：针对每层每个连续涂胶区域的包围盒补偿，适合模型局部不平（范围 ±0.1mm，外扩 2mm）',
      type: ['A', 'B', 'D'], advice: 'edit', sug: '热床：按整张床补偿；涂胶面：按每块涂胶区补偿。',
      why: '两条各带三个括号数字，说明位放不下；范围留给输入框的 min/max。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '热床-前左补偿', label: '热床-前左补偿', src: `${SRC} · wiping.glue_z_compensation_bed_fl`,
      cur: '热床模式下前左角的 Z 补偿值。负值=下压（涂胶笔更近床面，胶更多）；正值=上抬。补偿范围自动外扩 2mm 避免边缘突变',
      type: ['B', 'D'], advice: 'edit', sug: '前左角的 Z 补偿，负值下压、正值抬起。',
      why: '与同组另外三个角一字不差地重复机制；一条说清，其余同。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '热床-前右补偿', label: '热床-前右补偿', src: `${SRC} · wiping.glue_z_compensation_bed_fr`,
      cur: '热床模式下前右角的 Z 补偿值。负值=下压（涂胶笔更近床面，胶更多）；正值=上抬。补偿范围自动外扩 2mm 避免边缘突变',
      type: ['B', 'D'], advice: 'edit', sug: '前右角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '热床-后左补偿', label: '热床-后左补偿', src: `${SRC} · wiping.glue_z_compensation_bed_bl`,
      cur: '热床模式下后左角的 Z 补偿值。负值=下压（涂胶笔更近床面，胶更多）；正值=上抬。补偿范围自动外扩 2mm 避免边缘突变',
      type: ['B', 'D'], advice: 'edit', sug: '后左角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '热床-后右补偿', label: '热床-后右补偿', src: `${SRC} · wiping.glue_z_compensation_bed_br`,
      cur: '热床模式下后右角的 Z 补偿值。负值=下压（涂胶笔更近床面，胶更多）；正值=上抬。补偿范围自动外扩 2mm 避免边缘突变',
      type: ['B', 'D'], advice: 'edit', sug: '后右角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '涂胶面-前左补偿', label: '涂胶面-前左补偿', src: `${SRC} · wiping.glue_z_compensation_patch_fl`,
      cur: '涂胶面模式下前左角的 Z 补偿值。每块涂胶区域的包围盒自动外扩 2mm 作为补偿范围。负值=下压；正值=上抬',
      type: ['B', 'D'], advice: 'edit', sug: '前左角的 Z 补偿，负值下压、正值抬起。',
      why: '与热床那四条同一个意思，四条一起压成一句。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '涂胶面-前右补偿', label: '涂胶面-前右补偿', src: `${SRC} · wiping.glue_z_compensation_patch_fr`,
      cur: '涂胶面模式下前右角的 Z 补偿值。每块涂胶区域的包围盒自动外扩 2mm 作为补偿范围。负值=下压；正值=上抬',
      type: ['B', 'D'], advice: 'edit', sug: '前右角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '涂胶面-后左补偿', label: '涂胶面-后左补偿', src: `${SRC} · wiping.glue_z_compensation_patch_bl`,
      cur: '涂胶面模式下后左角的 Z 补偿值。每块涂胶区域的包围盒自动外扩 2mm 作为补偿范围。负值=下压；正值=上抬',
      type: ['B', 'D'], advice: 'edit', sug: '后左角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },
    {
      page: P, sec: '涂胶 › 涂胶方向补偿', where: '涂胶面-后右补偿', label: '涂胶面-后右补偿', src: `${SRC} · wiping.glue_z_compensation_patch_br`,
      cur: '涂胶面模式下后右角的 Z 补偿值。每块涂胶区域的包围盒自动外扩 2mm 作为补偿范围。负值=下压；正值=上抬',
      type: ['B', 'D'], advice: 'edit', sug: '后右角的 Z 补偿，负值下压、正值抬起。',
      why: '同上。',
    },

    /* ---- 切换 · 自定义 G-code ---- */
    {
      page: P, sec: '切换 › 自定义 G-code', where: '回抽位置', label: '回抽位置', src: `${SRC} · toolhead.MKP_retract_at`,
      cur: '回抽发生在换笔的哪一刻',
      type: ['F'], advice: 'edit', sug: '回抽放在换笔的哪一步。',
      why: '「发生在…哪一刻」是书面说法；「放在哪一步」更贴动作。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '落笔 G-code', label: '落笔 G-code', src: `${SRC} · toolhead.custom_mount_gcode`,
      cur: '让笔尖落下的自定义指令（碰一下右端机构完成切换），支持 [AUTO] 变量',
      type: ['B', 'A'], advice: 'edit', sug: '让笔尖落下的指令，可用 [AUTO] 变量。',
      why: '括号里的机构动作是机制；「[AUTO] 变量」是要保留的能力。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '落笔位 X', label: '落笔位 X', src: `${SRC} · toolhead.pen_mount_x`,
      cur: '推到底、笔尖真正挂上工具头的那个 X，对应落笔骨架里的 {target}',
      type: ['B', 'E'], advice: 'edit', sug: '推到底、笔尖挂上工具头的 X。',
      why: '「真正」是强调；{target} 是骨架变量名，改骨架的人自己知道，说明位不必带。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '落笔速度', label: '落笔速度', src: `${SRC} · toolhead.pen_mount_speed`,
      cur: '落笔时推到动作位那一段的进给速度，对应骨架里的 {speed}',
      type: ['E'], advice: 'edit', sug: '推到落笔位那一段的进给速度。',
      why: '「落笔时」与「推到动作位」重复；{speed} 是骨架变量。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '回抽长度', label: '回抽长度', src: `${SRC} · toolhead.MKP_retract`,
      cur: '涂胶笔回抽长度(mm)，负值=回抽。落笔/收笔的 G-code 里不再写 E，改由这里统一管',
      type: ['A', 'D'], advice: 'edit', sug: '收笔回抽量，负值回抽；G-code 里不再写 E。',
      why: '「不再写 E」是用户必须知道的后果，留；「改由这里统一管」是同一件事的重复。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '回抽速度', label: '回抽速度', src: `${SRC} · toolhead.MKP_retract_speed`,
      cur: '回抽那一笔的进给速度。太快咬料，太慢挂丝',
      type: ['F'], advice: 'edit', sug: '回抽那一下的速度，太快咬料、太慢挂丝。',
      why: '「那一笔」是旧白话；两句用逗号连起来，读着是一个提醒。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '落笔等待位 X', label: '落笔等待位 X', src: `${SRC} · toolhead.pen_mount_wait_x`,
      cur: '落笔前停住的那个 X 坐标，对应落笔骨架里的 {wait}',
      type: ['E', 'F'], advice: 'edit', sug: '落笔前停住的 X。',
      why: '「那个」「坐标」与变量名都可省。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '落笔等待位 Y', label: '落笔等待位 Y', src: `${SRC} · toolhead.pen_mount_wait_y`,
      cur: '落笔等待位的 Y 坐标，对应骨架里的 {wait_y}。A 系列不动这个轴，留 0',
      type: ['E'], advice: 'edit', sug: '落笔前停住的 Y；A 系列不用，留 0。',
      why: '「不动这个轴」换成「不用」；前者是机械口径。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '收笔 G-code', label: '收笔 G-code', src: `${SRC} · toolhead.custom_unmount_gcode`,
      cur: '把笔尖收起的自定义指令（再碰一次右端机构完成切换），支持 [AUTO] 变量',
      type: ['B', 'A'], advice: 'edit', sug: '收起笔尖的指令，可用 [AUTO] 变量。',
      why: '与落笔那条对称；括号里的机构动作删。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '收笔位 X', label: '收笔位 X', src: `${SRC} · toolhead.pen_unmount_x`,
      cur: '碰一下把笔尖收回去的那个 X，对应收笔骨架里的 {target}',
      type: ['E', 'F'], advice: 'edit', sug: '碰一下收回笔尖的 X。',
      why: '「把…回去的那个」绕；变量名删。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '收笔速度', label: '收笔速度', src: `${SRC} · toolhead.pen_unmount_speed`,
      cur: '收笔时推到动作位那一段的进给速度，对应骨架里的 {speed}',
      type: ['E'], advice: 'edit', sug: '推到收笔位那一段的进给速度。',
      why: '与落笔那条对称。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '挤出长度', label: '挤出长度', src: `${SRC} · toolhead.MKP_extrude`,
      cur: '收笔时回填的挤出量(mm)。填 0 = 自动按「回抽长度绝对值 + 0.5」算；填了别的数就按这个数，不再联动',
      type: ['B', 'D'], advice: 'edit', sug: '收笔回填量；0 = 按回抽长度 + 0.5 自动算。',
      why: '「绝对值」在这一格恒为回抽量，可省；「填了别的数就按这个数」是废话。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '挤出速度', label: '挤出速度', src: `${SRC} · toolhead.MKP_extrude_speed`,
      cur: '回填那一笔的进给速度，与挤出长度各管一件事',
      type: ['F'], advice: 'edit', sug: '回填那一下的速度。',
      why: '后半句在提醒别跟上一格混淆，但两格名字已经不同。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '挤出分摊', label: '挤出分摊', src: `${SRC} · toolhead.MKP_extrude_spread`,
      cur: '开启后回填量分摊到塔上的擦料移动里，不在原地挤成一团',
      type: ['D'], advice: 'edit', sug: '回填量分摊到塔上的擦料移动，不在原地挤成一团。',
      why: '「开启后」是开关说明的默认句式，删掉仍是同一个意思。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '收笔等待位 X', label: '收笔等待位 X', src: `${SRC} · toolhead.pen_unmount_wait_x`,
      cur: '收笔前停住的那个 X 坐标，对应收笔骨架里的 {wait}',
      type: ['E', 'F'], advice: 'edit', sug: '收笔前停住的 X。',
      why: '与落笔等待位对称。',
    },
    {
      page: P, sec: '切换 › 自定义 G-code', where: '收笔等待位 Y', label: '收笔等待位 Y', src: `${SRC} · toolhead.pen_unmount_wait_y`,
      cur: '收笔等待位的 Y 坐标，对应骨架里的 {wait_y}。A 系列不动这个轴，留 0',
      type: ['E'], advice: 'edit', sug: '收笔前停住的 Y；A 系列不用，留 0。',
      why: '与落笔等待位对称。',
    },

    /* ---- 更多 · 润笔 ---- */
    {
      page: P, sec: '更多 › 润笔', where: '首笔活化', label: '首笔活化', src: `${SRC} · toolhead.first_pen_revitalization_flag`,
      cur: '在擦料塔的首层涂胶激活胶笔出胶，仅在开启擦料塔时有效',
      type: ['D'], advice: 'edit', sug: '首层先出胶，只在开擦料塔时生效。',
      why: '「激活胶笔出胶」压成「先出胶」；前提放后半句。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '启用润笔', label: '启用润笔', src: `${SRC} · wiping.prime_enabled`,
      cur: '润笔总开关，关闭后所有润笔相关参数不显示且润笔逻辑不执行',
      type: ['D', 'A'], advice: 'edit', sug: '润笔总开关，关掉后相关参数一起隐藏。',
      why: '「逻辑不执行」是实现；用户能看见的是参数消失。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '润笔路径长度', label: '润笔路径长度', src: `${SRC} · toolhead.prime_length`,
      cur: '长时间未涂胶后重新润笔的路径长度，确保胶水充分流出',
      type: ['D'], advice: 'edit', sug: '重新出胶前先空走这么长。',
      why: '「确保充分流出」是目的，不是做法；一句说清这一格是什么。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '润笔最小路径长度', label: '润笔最小路径长度', src: `${SRC} · wiping.prime_min_path_length`,
      cur: '触发润笔的最小路径长度阈值，路径长度小于此值时不触发润笔',
      type: ['D', 'E'], advice: 'edit', sug: '路径短于这个值就不润笔。',
      why: '「阈值」与后半句是一件事说两遍。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '润笔触发层数', label: '润笔触发层数', src: `${SRC} · wiping.prime_trigger_layers`,
      cur: '超过此层数未涂胶时自动触发润笔',
      type: ['F'], advice: 'edit', sug: '超过这么多层没涂胶就润笔。',
      why: '「此层数」「自动触发」都是书面词。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '每个模型都润笔', label: '每个模型都润笔', src: `${SRC} · wiping.prime_per_model`,
      cur: '开启后每个模型的涂胶前都执行润笔，关闭则每层只润笔一次',
      type: ['D'], advice: 'edit', sug: '每个模型都先润笔；关闭则每层一次。',
      why: '两句合成一句，前半说开、后半说关。',
    },
    {
      page: P, sec: '更多 › 润笔', where: '每层润笔', label: '每层润笔', src: `${SRC} · wiping.prime_every_layer`,
      cur: '开启后每层涂胶前都触发润笔，与润笔触发层数互斥',
      type: ['D'], advice: 'edit', sug: '每层涂胶前都润笔，与触发层数互斥。',
      why: '「互斥」是必要的，留下；「开启后」删。',
    },

    /* ---- 更多 · 涂胶参数 ---- */
    {
      page: P, sec: '更多 › 涂胶参数', where: '小特征因子', label: '小特征因子', src: `${SRC} · wiping.small_feature_factor`,
      cur: '小特征速度系数 = MaxSpeed × 此值。值越小(如0.5)小特征越精细但更慢',
      type: ['B', 'D', 'E'], advice: 'edit', sug: '小特征速度 = 最高速 × 这个值，越小越精细越慢。',
      why: 'MaxSpeed 是代码里的名字；括号里的例子可以让位给「越小越…」这句结论。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '涂胶Z偏移', label: '涂胶Z偏移', src: `${SRC} · toolhead.glue_z_offset`,
      cur: '涂胶时Z轴额外偏移，负值使笔尖更靠近模型表面',
      type: ['D'], advice: 'edit', sug: '涂胶时笔再往下压多少，负值更贴面。',
      why: '「额外偏移」是参数腔；说清正负。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '涂胶次数', label: '涂胶次数', src: `${SRC} · wiping.glue_pass_count`,
      cur: '同一区域的涂胶遍数，多次涂胶可增加粘合力但延长打印时间',
      type: ['D'], advice: 'edit', sug: '同一区域涂几遍，多了更粘、更慢。',
      why: '「增加粘合力但延长打印时间」是翻译腔的两面；用两个短词。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '预涂胶速度', label: '预涂胶速度', src: `${SRC} · wiping.bed_glue_speed_mode`,
      cur: '预涂胶时胶笔的移动速度：跟随机型=用机型预设的移动速度；涂胶速度=用涂胶速度限制那一档；自定义=下面填数',
      type: ['D', 'E'], advice: 'edit', sug: '跟随机型、用涂胶速度，或自己填。',
      why: '三段「选项名=解释」在控件上就是三个按钮；说明只列三条路。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '预涂胶速度·自定义', label: '预涂胶速度·自定义', src: `${SRC} · wiping.bed_glue_speed`,
      cur: '仅在预涂胶速度选「自定义」时生效，单位 mm/s',
      type: ['B'], advice: 'edit', sug: '只在选「自定义」时生效。',
      why: '单位在单位栏；「仅在…时生效」换成「只在…时生效」更顺。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '段间横移速度', label: '段间横移速度', src: `${SRC} · wiping.glue_travel_speed_mode`,
      cur: '涂胶时从一个涂胶段移到下一段的速度（笔还在胶面附近，不是抬起来飞的那段）',
      type: ['B', 'D'], advice: 'edit', sug: '涂胶段之间移动的速度，笔不抬起。',
      why: '括号在排除另一种抬笔移动 —— 那个区别说成「笔不抬起」四个字就够。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '段间横移速度·自定义', label: '段间横移速度·自定义', src: `${SRC} · wiping.glue_travel_speed`,
      cur: '仅在段间横移速度选「自定义」时生效，单位 mm/s',
      type: ['B'], advice: 'edit', sug: '只在选「自定义」时生效。',
      why: '同上。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '涂胶前热床预热温度', label: '涂胶前热床预热温度', src: `${SRC} · wiping.bed_glue_preheat_temp`,
      cur: '涂胶前把热床降到这个温度再涂，避免胶水在高温下干得太快',
      type: ['D'], advice: 'edit', sug: '涂胶前把热床降到这个温度，免得胶干太快。',
      why: '「在高温下」可由上下文知道；一句一行。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '预涂胶等待时间', label: '预涂胶等待时间', src: `${SRC} · wiping.bed_glue_wait_seconds`,
      cur: '涂完胶、笔抬起来之后先停多久再开始打印第一层，0 表示不停',
      type: ['D'], advice: 'edit', sug: '涂完抬起后等多久再打第一层，0 不等。',
      why: '「笔抬起来之后」「开始打印」都能压；「表示」删。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '擦笔模式', label: '擦笔模式', src: `${SRC} · wiping.tower_pen_wipe_mode`,
      cur: '在擦料塔上擦净胶笔（预留参数，当前版本不产生动作）。注意与「润笔」不同：润笔是多重复涂一段距离，擦笔是清洁笔头',
      type: ['B', 'D'], advice: 'edit', sug: '在塔上擦净笔头；预留参数，当前不产生动作。',
      why: '「与润笔不同」那半句是给实现者看的区分；保留「预留参数」这条最重要的。',
    },
    {
      page: P, sec: '更多 › 涂胶参数', where: '擦笔间隔层数', label: '擦笔间隔层数', src: `${SRC} · wiping.tower_pen_wipe_interval`,
      cur: '选择「自定义」模式时，每多少层擦一次笔（预留参数，当前版本不产生动作）',
      type: ['B', 'D'], advice: 'edit', sug: '自定义模式下每几层擦一次；预留参数。',
      why: '「当前版本不产生动作」与上一条重复；这一格只需要「还没生效」。',
    },

    /* ---- 更多 · Z 轴抬升 ---- */
    {
      page: P, sec: '更多 › Z轴抬升', where: '涂胶Z轴抬升高度', label: '涂胶Z轴抬升高度', src: `${SRC} · wiping.glue_z_lift_height`,
      cur: "涂胶前/后抬笔高度。换笔芯后涂胶前抬笔 + 涂胶结束抬笔使用此高度。前3层使用子参数'前3层抬升高度'",
      type: ['A', 'D'], advice: 'edit', sug: '涂胶前后抬笔的高度；前 3 层叠加下面的额外高度。',
      why: '两句在列举什么时候用它；一句说清用途与例外。',
    },
    {
      page: P, sec: '更多 › Z轴抬升', where: '前3层抬升高度', label: '前3层抬升高度', src: `${SRC} · wiping.glue_z_lift_height_first_layers`,
      cur: '前3层涂胶前/后抬笔的额外高度，用于首层附着稳定。与涂胶Z轴抬升高度叠加使用',
      type: ['D'], advice: 'edit', sug: '前 3 层在此高度上再抬一点，帮首层贴牢。',
      why: '「叠加使用」是加法机制，换成「再抬一点」；「用于…稳定」说人话。',
    },
    {
      page: P, sec: '更多 › Z轴抬升', where: '涂胶区域间Z轴抬升高度', label: '涂胶区域间Z轴抬升高度', src: `${SRC} · wiping.glue_inter_model_lift_height`,
      cur: '模型间跳转 + 段间跳转微抬合并控制。涂胶区域间移动和段间跳转均使用此高度',
      type: ['D', 'E'], advice: 'edit', sug: '换涂胶区域和段间跳转时的抬笔高度。',
      why: '「合并控制」是代码里的写法；两种跳转说一遍就够。',
    },
    {
      page: P, sec: '更多 › Z轴抬升', where: '换笔脱层高度', label: '换笔脱层高度', src: `${SRC} · wiping.swap_hop_height`,
      cur: '换笔时喷嘴离开刚打完表面的高度（z-hop）。只影响那一次小螺旋脱层，不是抬笔高度',
      type: ['B', 'D'], advice: 'edit', sug: '换笔时喷嘴离开已打表面的高度，只管那一次脱层。',
      why: '「不是抬笔高度」是在纠错，改成正面说「只管那一次」。',
    },

    /* ---- 更多 · 校准 ---- */
    {
      page: P, sec: '更多 › 校准', where: 'XY校准模式', label: 'XY校准模式', src: `${SRC} · wiping.xy_calibration_mode`,
      cur: 'XY校准模式：原版按机型固定坐标走；新版按模型在板上的实际位置生成（默认）',
      type: ['D', 'E'], advice: 'edit', sug: '原版按机型固定坐标，新版按模型实际位置（默认）。',
      why: '「XY校准模式：」是在复述名称；两个版本各一句。',
    },
    {
      page: P, sec: '更多 › 校准', where: 'Z校准模式', label: 'Z校准模式', src: `${SRC} · wiping.z_calibration_mode`,
      cur: 'Z校准模式：原版按机型固定坐标走；新版按模型在板上的实际位置生成（默认）',
      type: ['D', 'E'], advice: 'edit', sug: '原版按机型固定坐标，新版按模型实际位置（默认）。',
      why: '同上。',
    },

    /* ---- 更多 · 机型专属 ---- */
    {
      page: P, sec: '更多 › 机型专属', where: '强制关闭前盖脱落报警', label: '强制关闭前盖脱落报警（有效性存疑）', labelSug: '关闭前盖脱落报警',
      src: `${SRC} · wiping.disable_front_cover_alarm`,
      cur: '开启后将强制关闭前盖脱落报警。仅对X1、P1系列机型有效，民间流传方法，有效性存疑。',
      type: ['B', 'E', 'F'], advice: 'edit', sug: '开启后不再报前盖脱落；只对 X1/P1，民间做法，有效性存疑。',
      why: '名称里带「（有效性存疑）」太长，判断挪进说明；「强制」与开关重复。',
    },
    {
      page: P, sec: '更多 › 机型专属', where: '关闭第三层裹头检测', label: '关闭第三层裹头检测', src: `${SRC} · wiping.disable_3rd_layer_clog_detect`,
      cur: '开启后移除第三层裹头检测段。仅对A1/A1 mini机型有效',
      type: ['A', 'B'], advice: 'edit', sug: '移除第三层裹头检测；只对 A1/A1 mini。',
      why: '「检测段」是实现单位；「机型」可由前半句的机型名知道。',
    },
    {
      page: P, sec: '更多 › 机型专属', where: '关闭延时摄影', label: '关闭延时摄影', src: `${SRC} · wiping.disable_timelapse`,
      cur: '开启后移除延时摄影段(SKIPTYPE: timelapse)。仅对i3结构的A系列机型有效',
      type: ['A', 'B', 'E'], advice: 'edit', sug: '移除延时摄影；只对 i3 结构 A 系列。',
      why: 'SKIPTYPE 是 G-code 标记，给改代码的人看；说明位只留适用范围。',
    },
  )
})()
