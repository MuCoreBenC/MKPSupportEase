/*
 * BBS 的条件显隐规则 —— 译自 machine-motion/src/bbs/toggle-rules.js（那份又是从
 * `ConfigManipulation.cpp:861` 的 `toggle_print_fff_options()` 逐条翻的，BBS v02.08.02.61，
 * 共 86 条规则、覆盖 178 个 key）。纯数据 + 纯函数，不碰 DOM。
 *
 * BBS 用两种调用：
 *   toggle_line(key, false)  → 整行不显示
 *   toggle_field(key, false) → 行还在，控件置灰（值照显示）
 * 这个区别必须照抄，抹平任何一边都会和 BBS 差出行数来。
 *
 * 顺序有意义：同一个 key 可能被 toggle 两次（`detect_thin_wall` 就有
 * `wall_loops > 0` 和 `wall_generator != arachne` 两条），**后面的覆盖前面的**，
 * 所以这张表要保持源码顺序，evalToggles 也按顺序执行。
 *
 * 有 5 类条件读的是**工艺预设里没有的东西**（打印机预设 / 程序运行态）：
 *   is_BBL_printer / printer_model / gcode_flavor / printer_type / is_global_config
 * 它们由 env 注入，取值与「在 BBS 里看一台拓竹机的全局工艺参数」一致（见 DEFAULT_ENV）。
 * 判定不了的一律**当作显示** —— 宁可多一行，也不要凭空隐藏让人以为 BBS 里没这个参数。
 */

import type { BbsEnv, BbsToggle, BbsValue, BbsValues } from './bbsTypes'

const norm = (v: BbsValue | undefined) => (Array.isArray(v) ? v[0] : v)

export const str = (v: BbsValue | undefined): string => String(norm(v) ?? '').trim()

export const num = (v: BbsValue | undefined): number => {
  const s = str(v).replace('%', '')
  const n = Number(s)
  return Number.isFinite(n) ? n : 0
}

export const bool = (v: BbsValue | undefined): boolean => {
  const s = str(v).toLowerCase()
  if (s === '' || s === '0' || s === 'false' || s === 'off' || s === 'no' || s === 'nil') return false
  if (s === '1' || s === 'true' || s === 'on' || s === 'yes') return true
  const n = Number(s)
  return Number.isFinite(n) ? n !== 0 : true
}

export const isIn = (v: BbsValue | undefined, list: string[]): boolean => list.includes(str(v))

const isTree = (v: BbsValues) => isIn(v.support_type, ['tree(auto)', 'tree(manual)'])
const hasSupport = (v: BbsValues) => bool(v.enable_support) || num(v.raft_layers) > 0

/* 与「在 BBS 里看一台拓竹机的全局工艺参数」对齐：
 *   is_BBL_printer   我们的数据源就是 BBL 的系统预设，恒真（BBL 机器上那 7 个 jerk 行是隐藏的）
 *   gcode_flavor     拓竹机不是 klipper，所以 accel_to_decel / exclude_object 隐藏
 *   is_global_config 我们复刻的是「全局」面板，对象级的 flush_into_objects 那几行隐藏
 *   printer_model    由调用方从当前预设的 compatible_printers 推断（H2C/H2D/X2D 才显示擦料塔界面特性）
 *   wrapping         机型能力表在程序里，查不到 → 当作显示
 */
export const DEFAULT_ENV: BbsEnv = {
  is_BBL_printer: true,
  gcode_flavor: 'marlin',
  is_global_config: true,
  printer_model: '',
  wrapping: true,
}

/* 填充图案的枚举字符串有两个坑（PrintConfig.cpp 里 enum_values 就这么写的）：
   ipRectilinear 写进 json 是 `zig-zag`，ipZigZag 才是 `zigzag`；ipStars 是 `tri-hexagon`。 */
const FILL_MULTILINE_OK = ['cubic', 'grid', 'zig-zag', 'tri-hexagon', 'alignedrectilinear',
  'gyroid', 'honeycomb', 'lightning', '3dhoneycomb', 'adaptivecubic', 'supportcubic']

interface ToggleRule {
  kind: 'line' | 'field'
  keys: string[]
  when: (v: BbsValues, e: BbsEnv) => boolean
}

export const TOGGLE_RULES: ToggleRule[] = [
  { kind: 'field', keys: ['ensure_vertical_shell_thickness', 'detect_thin_wall', 'detect_overhang_wall', 'seam_position', 'seam_placement_away_from_overhangs', 'seam_gap', 'wipe_speed', 'wall_sequence', 'outer_wall_line_width', 'inner_wall_speed', 'outer_wall_speed', 'small_perimeter_speed', 'small_perimeter_threshold'], when: (v) => num(v.wall_loops) > 0 },
  { kind: 'line', keys: ['seam_placement_away_from_overhangs'], when: (v) => isIn(v.seam_position, ['aligned', 'back']) },

  { kind: 'line', keys: ['sparse_infill_pattern', 'sparse_infill_anchor_max', 'infill_combination', 'minimum_sparse_infill_area', 'sparse_infill_filament', 'infill_shift_step', 'infill_rotate_step', 'symmetric_infill_y_axis', 'sparse_infill_lattice_angle_1', 'sparse_infill_lattice_angle_2'], when: (v) => num(v.sparse_infill_density) > 0 },
  { kind: 'line', keys: ['fill_multiline'], when: (v) => num(v.sparse_infill_density) > 0 && isIn(v.sparse_infill_pattern, FILL_MULTILINE_OK) },
  { kind: 'line', keys: ['sparse_infill_anchor'], when: (v) => num(v.sparse_infill_density) > 0 && num(v.sparse_infill_anchor_max) > 0 },
  { kind: 'line', keys: ['infill_instead_top_bottom_surfaces', 'skeleton_infill_density', 'skin_infill_density', 'infill_lock_depth', 'skin_infill_depth', 'skin_infill_line_width', 'skeleton_infill_line_width', 'locked_skin_infill_pattern', 'locked_skeleton_infill_pattern'], when: (v) => num(v.sparse_infill_density) > 0 && str(v.sparse_infill_pattern) === 'lockedzag' },
  { kind: 'line', keys: ['infill_rotate_step'], when: (v) => num(v.sparse_infill_density) > 0 && str(v.sparse_infill_pattern) === 'zigzag' },
  { kind: 'line', keys: ['infill_shift_step'], when: (v) => num(v.sparse_infill_density) > 0 && isIn(v.sparse_infill_pattern, ['crosszag', 'lockedzag']) },
  { kind: 'line', keys: ['symmetric_infill_y_axis'], when: (v) => num(v.sparse_infill_density) > 0 && isIn(v.sparse_infill_pattern, ['zigzag', 'crosszag', 'lockedzag']) },
  { kind: 'line', keys: ['sparse_infill_lattice_angle_1', 'sparse_infill_lattice_angle_2'], when: (v) => num(v.sparse_infill_density) > 0 && str(v.sparse_infill_pattern) === '2dlattice' },

  { kind: 'line', keys: ['spiral_mode_smooth'], when: (v) => bool(v.spiral_mode) },
  { kind: 'line', keys: ['spiral_mode_max_xy_smoothing'], when: (v) => bool(v.spiral_mode_smooth) },
  { kind: 'field', keys: ['z_direction_outwall_speed_continuous'], when: (v) => !bool(v.spiral_mode) },

  { kind: 'field', keys: ['top_surface_pattern', 'bottom_surface_pattern', 'top_surface_density', 'bottom_surface_density', 'internal_solid_infill_pattern', 'solid_infill_filament'], when: (v) => num(v.top_shell_layers) > 0 || num(v.bottom_shell_layers) > 0 },
  { kind: 'field', keys: ['infill_direction', 'sparse_infill_line_width', 'bridge_angle', 'sparse_infill_speed', 'bridge_speed'], when: (v) => num(v.sparse_infill_density) > 0 || num(v.top_shell_layers) > 0 || num(v.bottom_shell_layers) > 0 },
  { kind: 'field', keys: ['top_shell_thickness'], when: (v) => !bool(v.spiral_mode) && num(v.top_shell_layers) > 0 },
  { kind: 'field', keys: ['bottom_shell_thickness'], when: (v) => !bool(v.spiral_mode) && num(v.bottom_shell_layers) > 0 },
  { kind: 'field', keys: ['gap_infill_speed'], when: (v) => num(v.wall_loops) > 0 },
  { kind: 'field', keys: ['top_surface_line_width', 'top_surface_speed'], when: (v) => num(v.top_shell_layers) > 0 || (bool(v.spiral_mode) && num(v.bottom_shell_layers) > 0) },

  { kind: 'field', keys: ['initial_layer_acceleration', 'outer_wall_acceleration', 'top_surface_acceleration', 'inner_wall_acceleration', 'sparse_infill_acceleration'], when: (v) => num(v.default_acceleration) > 0 },
  { kind: 'line', keys: ['default_jerk', 'outer_wall_jerk', 'inner_wall_jerk', 'infill_jerk', 'top_surface_jerk', 'initial_layer_jerk', 'travel_jerk'], when: (_v, e) => !e.is_BBL_printer },
  { kind: 'field', keys: ['outer_wall_jerk', 'inner_wall_jerk', 'infill_jerk', 'top_surface_jerk', 'initial_layer_jerk', 'travel_jerk'], when: (v, e) => !e.is_BBL_printer && num(v.default_jerk) > 0 },

  { kind: 'field', keys: ['skirt_height'], when: (v) => num(v.skirt_loops) > 0 && str(v.draft_shield) !== 'enabled' },
  { kind: 'field', keys: ['skirt_distance', 'draft_shield'], when: (v) => num(v.skirt_loops) > 0 },
  { kind: 'field', keys: ['brim_object_gap'], when: (v) => str(v.brim_type) !== 'no_brim' },
  { kind: 'field', keys: ['brim_width'], when: (v) => !isIn(v.brim_type, ['no_brim', 'auto_brim', 'brim_ears']) },
  { kind: 'field', keys: ['wall_filament'], when: (v) => num(v.wall_loops) > 0 || str(v.brim_type) !== 'no_brim' },

  { kind: 'field', keys: ['support_style', 'support_base_pattern', 'support_base_pattern_spacing', 'support_expansion', 'support_angle', 'support_interface_pattern', 'support_interface_top_layers', 'bridge_no_support', 'max_bridge_length', 'support_top_z_distance', 'support_bottom_z_distance', 'support_type', 'support_on_build_plate_only', 'support_remove_small_overhang', 'support_interface_not_for_body', 'support_object_xy_distance', 'support_object_first_layer_gap'], when: (v) => hasSupport(v) },
  { kind: 'field', keys: ['support_threshold_angle'], when: (v) => hasSupport(v) && isIn(v.support_type, ['normal(auto)', 'tree(auto)']) },
  { kind: 'field', keys: ['tree_support_branch_angle', 'tree_support_branch_distance', 'tree_support_branch_diameter', 'tree_support_branch_diameter_angle'], when: (v) => bool(v.enable_support) && isTree(v) },
  { kind: 'line', keys: ['tree_support_branch_angle', 'tree_support_branch_distance', 'tree_support_branch_diameter', 'tree_support_branch_diameter_angle', 'max_bridge_length'], when: (v) => bool(v.enable_support) && isTree(v) },
  { kind: 'line', keys: ['support_critical_regions_only'], when: (v) => bool(v.enable_support) && str(v.support_type) === 'tree(auto)' },
  { kind: 'line', keys: ['detect_floating_vertical_shell'], when: (v) => bool(v.detect_narrow_internal_solid_infill) },
  { kind: 'line', keys: ['vertical_shell_speed'], when: (v) => bool(v.detect_narrow_internal_solid_infill) },
  { kind: 'line', keys: ['bridge_no_support'], when: (v) => !(bool(v.enable_support) && isTree(v)) },
  { kind: 'line', keys: ['support_bottom_interface_spacing'], when: (v) => !(bool(v.enable_support) && isTree(v)) },
  { kind: 'line', keys: ['support_interface_bottom_layers'], when: (v) => !(bool(v.enable_support) && isTree(v)) },
  { kind: 'field', keys: ['support_interface_spacing', 'support_interface_filament', 'support_interface_loop_pattern'], when: (v) => hasSupport(v) && (num(v.support_interface_top_layers) > 0 || num(v.support_interface_bottom_layers) > 0) },
  { kind: 'line', keys: ['support_speed'], when: (v) => hasSupport(v) || (num(v.skirt_loops) > 0 && (num(v.skirt_height) > 1 || str(v.draft_shield) !== 'enabled')) },
  { kind: 'line', keys: ['support_interface_speed'], when: (v) => hasSupport(v) && (num(v.support_interface_top_layers) > 0 || num(v.support_interface_bottom_layers) > 0) },
  { kind: 'field', keys: ['inner_wall_line_width'], when: (v) => num(v.wall_loops) > 0 || num(v.skirt_loops) > 0 || str(v.brim_type) !== 'no_brim' },
  { kind: 'field', keys: ['support_filament'], when: (v) => hasSupport(v) || num(v.skirt_loops) > 0 },
  { kind: 'line', keys: ['raft_contact_distance'], when: (v) => num(v.raft_layers) > 0 && !(hasSupport(v) && num(v.support_top_z_distance) === 0) },

  { kind: 'line', keys: ['ironing_pattern', 'ironing_speed', 'ironing_flow', 'ironing_spacing', 'ironing_direction', 'ironing_inset'], when: (v) => str(v.ironing_type) !== 'no ironing' },
  { kind: 'field', keys: ['enable_support_ironing'], when: (v) => num(v.raft_layers) > 1 || (bool(v.enable_support) && num(v.support_interface_top_layers) > 0) },
  { kind: 'line', keys: ['support_ironing_pattern', 'support_ironing_speed', 'support_ironing_flow', 'support_ironing_spacing', 'support_ironing_direction', 'support_ironing_inset'], when: (v) => bool(v.enable_support_ironing) && (num(v.raft_layers) > 1 || (bool(v.enable_support) && num(v.support_interface_top_layers) > 0)) },

  { kind: 'field', keys: ['standby_temperature_delta'], when: (v) => bool(v.ooze_prevention) },
  { kind: 'line', keys: ['prime_tower_width', 'prime_tower_brim_width', 'prime_tower_skip_points', 'prime_tower_rib_wall', 'prime_tower_infill_gap', 'prime_tower_enable_framework', 'prime_tower_max_speed'], when: (v) => bool(v.enable_prime_tower) },
  { kind: 'line', keys: ['enable_tower_interface_features'], when: (v, e) => bool(v.enable_prime_tower) && /H2C|H2D|X2D/i.test(e.printer_model || '') },
  { kind: 'line', keys: ['prime_tower_extra_rib_length', 'prime_tower_rib_width', 'prime_tower_fillet_wall'], when: (v) => bool(v.enable_prime_tower) && bool(v.prime_tower_rib_wall) },
  { kind: 'field', keys: ['prime_tower_width'], when: (v) => !(bool(v.enable_prime_tower) && bool(v.prime_tower_rib_wall)) },
  { kind: 'field', keys: ['flush_into_infill', 'flush_into_support', 'flush_into_objects'], when: (v) => bool(v.enable_prime_tower) },

  { kind: 'line', keys: ['max_travel_detour_distance'], when: (v) => bool(v.reduce_crossing_wall) },
  { kind: 'line', keys: ['avoid_crossing_wall_includes_support'], when: (v) => bool(v.reduce_crossing_wall) },
  { kind: 'line', keys: ['overhang_1_4_speed', 'overhang_2_4_speed', 'overhang_3_4_speed', 'overhang_4_4_speed'], when: (v) => bool(v.enable_overhang_speed) },
  { kind: 'line', keys: ['slowdown_start_height', 'slowdown_start_speed', 'slowdown_start_acc', 'slowdown_end_height', 'slowdown_end_speed', 'slowdown_end_acc'], when: (v) => bool(v.enable_height_slowdown) },
  /* 这几个是对象级参数，全局面板里 BBS 不显示 */
  { kind: 'line', keys: ['flush_into_objects', 'print_flow_ratio', 'wall_filament', 'solid_infill_filament', 'sparse_infill_filament'], when: (_v, e) => !e.is_global_config },
  { kind: 'line', keys: ['support_interface_not_for_body'], when: (v) => num(v.support_interface_filament) !== 0 && num(v.support_filament) === 0 },

  { kind: 'line', keys: ['fuzzy_skin_thickness', 'fuzzy_skin_point_distance', 'fuzzy_skin_first_layer', 'fuzzy_skin_noise_type', 'fuzzy_skin_mode'], when: (v) => str(v.fuzzy_skin) !== 'disabled_fuzzy' },
  { kind: 'line', keys: ['fuzzy_skin_scale'], when: (v) => str(v.fuzzy_skin) !== 'disabled_fuzzy' && str(v.fuzzy_skin_noise_type) !== 'classic' },
  { kind: 'line', keys: ['fuzzy_skin_octaves'], when: (v) => str(v.fuzzy_skin) !== 'disabled_fuzzy' && !isIn(v.fuzzy_skin_noise_type, ['classic', 'voronoi']) },
  { kind: 'line', keys: ['fuzzy_skin_persistence'], when: (v) => str(v.fuzzy_skin) !== 'disabled_fuzzy' && isIn(v.fuzzy_skin_noise_type, ['perlin', 'billow']) },

  { kind: 'line', keys: ['wall_transition_length', 'wall_transition_filter_deviation', 'wall_transition_angle', 'min_feature_size', 'min_bead_width', 'wall_distribution_count'], when: (v) => str(v.wall_generator) === 'arachne' },
  /* 注意：detect_thin_wall 被 toggle 两次，这条在后，覆盖前面那条 wall_loops>0 */
  { kind: 'field', keys: ['detect_thin_wall'], when: (v) => str(v.wall_generator) !== 'arachne' },

  { kind: 'line', keys: ['accel_to_decel_enable', 'accel_to_decel_factor'], when: (_v, e) => str(e.gcode_flavor) === 'klipper' },
  { kind: 'field', keys: ['accel_to_decel_factor'], when: (v, e) => str(e.gcode_flavor) === 'klipper' && bool(v.accel_to_decel_enable) },
  { kind: 'line', keys: ['exclude_object'], when: (_v, e) => str(e.gcode_flavor) === 'klipper' },

  { kind: 'line', keys: ['mmu_segmented_region_interlocking_depth'], when: (v) => !bool(v.interlocking_beam) },
  { kind: 'line', keys: ['interlocking_beam_width', 'interlocking_orientation', 'interlocking_beam_layer_count', 'interlocking_depth', 'interlocking_boundary_avoidance'], when: (v) => bool(v.interlocking_beam) },

  { kind: 'field', keys: ['xy_hole_compensation'], when: (v) => !bool(v.enable_circle_compensation) },
  { kind: 'field', keys: ['xy_contour_compensation'], when: (v) => !bool(v.enable_circle_compensation) },
  { kind: 'line', keys: ['circle_compensation_manual_offset'], when: (v) => bool(v.enable_circle_compensation) },

  { kind: 'line', keys: ['seam_slope_type', 'seam_slope_start_height', 'seam_slope_gap', 'seam_slope_min_length'], when: (v) => bool(v.override_filament_scarf_seam_setting) },
  { kind: 'line', keys: ['enable_wrapping_detection'], when: (_v, e) => !!e.wrapping },
]

/* 跑一遍规则 → Map(key → {line, field})。没被任何规则碰到的 key 不出现在 Map 里（= 显示且不置灰）。
 * 规则里读不到的值按「空」处理（num→0 / bool→false / str→''），与 BBS 读默认值的行为接近；
 * 真要报警的话看 coverage() 的未覆盖清单。 */
export function evalToggles(values: BbsValues, env: Partial<BbsEnv> = {}): Map<string, BbsToggle> {
  const e: BbsEnv = { ...DEFAULT_ENV, ...env }
  const out = new Map<string, BbsToggle>()
  for (const r of TOGGLE_RULES) {
    let ok = true
    try { ok = !!r.when(values, e) } catch { continue }   // 条件算不出来就跳过这条，不敢隐藏
    for (const k of r.keys) {
      const cur = out.get(k) || { line: true, field: true }
      cur[r.kind] = ok                                   // 后覆盖前，与源码顺序一致
      out.set(k, cur)
    }
  }
  return out
}

/* 这张表覆盖了多少 key —— 写进状态条的 title，方便判断「某行没隐藏」是规则漏了还是本来就该显示 */
export function coverage(): { rules: number; keys: number; keySet: Set<string> } {
  const keys = new Set<string>()
  for (const r of TOGGLE_RULES) for (const k of r.keys) keys.add(k)
  return { rules: TOGGLE_RULES.length, keys: keys.size, keySet: keys }
}

/**
 * 从预设的 compatible_printers 推 printer_model（只有擦料塔那一条规则用得上）。
 * 认不出来返回 ''，那条规则就当不满足 —— 与 DEFAULT_ENV 一致。
 */
export function printerModelOf(compatiblePrinters: BbsValue | null | undefined): string {
  const s = str(compatiblePrinters ?? undefined)
  const m = s.match(/H2C|H2DP|H2D|X2D/i)
  return m ? m[0] : ''
}
