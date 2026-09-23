; HEADER_BLOCK_START
; BambuStudio 02.07.01.62
; model printing time: 10m 45s; total estimated time: 16m 48s
; total layer number: 43
; total filament length [mm] : 597.51
; total filament volume [cm^3] : 1437.18
; total filament weight [g] : 1.80
; filament_density: 1.25
; filament_diameter: 1.75
; max_z_height: 8.50
; filament: 1
; HEADER_BLOCK_END

; CONFIG_BLOCK_START
; accel_to_decel_enable = 0
; accel_to_decel_factor = 50%
; activate_air_filtration = 0
; additional_cooling_fan_speed = 0
; additional_fan_full_speed_layer = 0
; alternate_extra_wall = 0
; apply_scarf_seam_on_circles = 1
; auxiliary_fan = 0
; avoid_crossing_wall_includes_support = 0
; bed_custom_model = 
; bed_custom_texture = 
; bed_exclude_area = 
; bed_temperature_formula = by_first_filament
; before_layer_change_gcode = 
; best_object_pos = 0.7,0.5
; bottom_color_penetration_layers = 3
; bottom_shell_layers = 3
; bottom_shell_thickness = 0.6
; bottom_surface_density = 100%
; bottom_surface_pattern = monotonic
; bridge_angle = 90
; bridge_flow = 1
; bridge_no_support = 0
; bridge_speed = 50
; brim_object_gap = 0.1
; brim_type = auto_brim
; brim_width = 3
; chamber_temperatures = 0
; change_filament_gcode = ;===== A1mini 20251031 =====\nG392 S0\nM1007 S0\nM620 S[next_extruder]A\nM204 S9000\nG1 Z{max_layer_z + 3.0} F1200\n\nM400\nM106 P1 S0\nM106 P2 S0\n{if old_filament_temp > 142 && next_extruder < 255}\nM104 S[old_filament_temp]\n{endif}\n\nG1 X180 F18000\n\n{if long_retractions_when_cut[previous_extruder]}\nM620.11 S1 I[previous_extruder] E-{retraction_distances_when_cut[previous_extruder]} F1200\n{else}\nM620.11 S0\n{endif}\nM400\n\nM620.1 E F{flush_volumetric_speeds[previous_extruder]/2.4053*60} T{flush_temperatures[previous_extruder]}\nM620.10 A0 F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nT[next_extruder]\nM620.1 E F{flush_volumetric_speeds[next_extruder]/2.4053*60} T{flush_temperatures[next_extruder]}\nM620.10 A1 F{flush_volumetric_speeds[next_extruder]/2.4053*60} L[flush_length] H[nozzle_diameter] T{flush_temperatures[next_extruder]}\n\nG1 Y90 F9000\n\n{if next_extruder < 255}\n\n{if long_retractions_when_cut[previous_extruder]}\nM620.11 S1 I[previous_extruder] E{retraction_distances_when_cut[previous_extruder]} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nM628 S1\nG92 E0\nG1 E{retraction_distances_when_cut[previous_extruder]} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nM400\nM629 S1\n{else}\nM620.11 S0\n{endif}\n\nM400\nG92 E0\nM628 S0\n\n{if flush_length_1 > 1}\n; FLUSH_START\n; always use highest temperature to flush\nM400\nM1002 set_filament_type:UNKNOWN\nM109 S[flush_temperatures[next_extruder]]\nM106 P1 S60\n{if flush_length_1 > 23.7}\nG1 E23.7 F{flush_volumetric_speeds[previous_extruder]/2.4053*60} ; do not need pulsatile flushing for start part\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\n{else}\nG1 E{flush_length_1} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\n{endif}\n; FLUSH_END\nG1 E-[old_retract_length_toolchange] F1800\nG1 E[old_retract_length_toolchange] F300\nM400\nM1002 set_filament_type:{filament_type[next_extruder]}\n{endif}\n\n{if flush_length_1 > 45 && flush_length_2 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_2 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\n; FLUSH_END\nG1 E-[new_retract_length_toolchange] F1800\nG1 E[new_retract_length_toolchange] F300\n{endif}\n\n{if flush_length_2 > 45 && flush_length_3 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_3 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\n; FLUSH_END\nG1 E-[new_retract_length_toolchange] F1800\nG1 E[new_retract_length_toolchange] F300\n{endif}\n\n{if flush_length_3 > 45 && flush_length_4 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_4 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\n; FLUSH_END\n{endif}\n\nM629\n\nM400\nM106 P1 S60\nM109 S[new_filament_temp]\nG1 E5 F{flush_volumetric_speeds[next_extruder]/2.4053*60} ;Compensate for filament spillage during waiting temperature\nM400\nG92 E0\nG1 E-[new_retract_length_toolchange] F1800\nM400\nM106 P1 S178\nM400 S3\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nG1 X-3.5 F18000\nG1 X-13.5 F3000\nM400\nG1 Z{max_layer_z + 3.0} F3000\nM106 P1 S0\n{if layer_z <= (initial_layer_print_height + 0.001)}\nM204 S[initial_layer_acceleration]\n{else}\nM204 S[default_acceleration]\n{endif}\n{else}\nG1 X[x_after_toolchange] Y[y_after_toolchange] Z[z_after_toolchange] F12000\n{endif}\n\nM622.1 S0\nM9833 F{outer_wall_volumetric_speed/2.4} A0.3 ; cali dynamic extrusion compensation\nM1002 judge_flag filament_need_cali_flag\nM622 J1\n  G92 E0\n  G1 E-[new_retract_length_toolchange] F1800\n  M400\n  \n  M106 P1 S178\n  M400 S7\n  G1 X0 F18000\n  G1 X-13.5 F3000\n  G1 X0 F18000 ;wipe and shake\n  G1 X-13.5 F3000\n  G1 X0 F12000 ;wipe and shake\n  G1 X-13.5 F3000\n  G1 X0 F12000 ;wipe and shake\n  M400\n  M106 P1 S0 \nM623\n\nM621 S[next_extruder]A\nG392 S0\n\nM1007 S1\n
; circle_compensation_manual_offset = 0
; circle_compensation_speed = 200
; close_additional_fan_first_x_layers = 3
; close_fan_the_first_x_layers = 3
; complete_print_exhaust_fan_speed = 70
; cool_plate_temp = 0
; cool_plate_temp_initial_layer = 0
; cooling_filter_enabled = 0
; cooling_perimeter_transition_distance = 10
; cooling_slowdown_logic = uniform_cooling
; counter_coef_1 = 0
; counter_coef_2 = 0.008
; counter_coef_3 = -0.041
; counter_limit_max = 0.033
; counter_limit_min = -0.035
; curr_bed_type = Textured PEI Plate
; default_acceleration = 6000
; default_filament_colour = ""
; default_filament_profile = "Bambu PLA Basic @BBL A1M"
; default_jerk = 0
; default_nozzle_volume_type = Standard
; default_print_profile = 0.20mm Standard @BBL A1M
; deretraction_speed = 30
; detect_floating_vertical_shell = 1
; detect_narrow_internal_solid_infill = 1
; detect_overhang_wall = 1
; detect_thin_wall = 0
; diameter_limit = 50
; different_settings_to_system = bottom_shell_thickness;bridge_angle;brim_width;enable_arc_fitting;enable_prime_tower;enable_support;enable_support_ironing;post_process;raft_first_layer_density;support_interface_bottom_layers;support_interface_filament;support_interface_pattern;support_interface_spacing;support_interface_top_layers;support_ironing_flow;support_ironing_pattern;support_ironing_spacing;support_ironing_speed;support_object_first_layer_gap;support_object_xy_distance;support_remove_small_overhang;support_style;support_top_z_distance;support_type;tree_support_wall_count;z_direction_outwall_speed_continuous;;
; draft_shield = disabled
; during_print_exhaust_fan_speed = 70
; elefant_foot_compensation = 0
; embedding_wall_into_infill = 0
; enable_arc_fitting = 0
; enable_circle_compensation = 0
; enable_filament_dynamic_map = 0
; enable_height_slowdown = 0
; enable_long_retraction_when_cut = 2
; enable_mixed_color_sublayer = 0
; enable_order_independent_overlap_carving = 0
; enable_overhang_bridge_fan = 1
; enable_overhang_speed = 1
; enable_pre_heating = 0
; enable_pressure_advance = 0
; enable_prime_tower = 0
; enable_support = 1
; enable_support_ironing = 1
; enable_tower_interface_features = 0
; enable_wrapping_detection = 0
; enforce_support_layers = 0
; eng_plate_temp = 70
; eng_plate_temp_initial_layer = 70
; ensure_vertical_shell_thickness = enabled
; exclude_object = 1
; extruder_ams_count = 1#0|4#0;1#0|4#0
; extruder_clearance_dist_to_rod = 56.5
; extruder_clearance_height_to_lid = 180
; extruder_clearance_height_to_rod = 25
; extruder_clearance_max_radius = 73
; extruder_colour = #018001
; extruder_max_nozzle_count = 1
; extruder_nozzle_stats = Standard#1
; extruder_offset = 0x0
; extruder_printable_area = 
; extruder_type = Direct Drive
; extruder_variant_list = "Direct Drive Standard"
; fan_cooling_layer_time = 30
; fan_direction = undefine
; fan_max_speed = 50
; fan_min_speed = 30
; filament_adaptive_volumetric_speed = 0
; filament_adhesiveness_category = 300
; filament_bridge_speed = 25
; filament_change_length = 10
; filament_change_length_nc = 10
; filament_colour = #FFFFFF
; filament_colour_type = 0
; filament_cooling_before_tower = 0
; filament_cost = 24.99
; filament_density = 1.25
; filament_dev_ams_drying_ams_limitations = 1;0
; filament_dev_ams_drying_heat_distortion_temperature = 75
; filament_dev_ams_drying_temperature = 65,65,55,55
; filament_dev_ams_drying_time = 12,12,12,12
; filament_dev_chamber_drying_bed_temperature = 80
; filament_dev_chamber_drying_time = 12
; filament_dev_drying_cooling_temperature = 55
; filament_dev_drying_softening_temperature = 60
; filament_diameter = 1.75
; filament_enable_overhang_speed = 1
; filament_end_gcode = "; filament end gcode \n\n"
; filament_extruder_compatibility = 0
; filament_extruder_variant = "Direct Drive Standard"
; filament_flow_ratio = 0.94
; filament_flush_temp = 0
; filament_flush_temp_fast = 0
; filament_flush_volumetric_speed = 0
; filament_ids = GFG00
; filament_is_mixed = 0
; filament_is_support = 0
; filament_long_retractions_when_cut = 1
; filament_map = 1
; filament_map_2 = 0
; filament_map_mode = Auto For Flush
; filament_max_volumetric_speed = 13
; filament_metal_stickiness = High
; filament_minimal_purge_on_wipe_tower = 15
; filament_mixed_components = ""
; filament_mixed_gradient = 0
; filament_mixed_gradient_curve = ""
; filament_mixed_gradient_per_part = 0
; filament_mixed_gradient_range = ""
; filament_mixed_sublayer_ratios = ""
; filament_multi_colour = #FFFFFF
; filament_notes = 
; filament_nozzle_map = 0
; filament_overhang_1_4_speed = 0
; filament_overhang_2_4_speed = 50
; filament_overhang_3_4_speed = 30
; filament_overhang_4_4_speed = 10
; filament_overhang_totally_speed = 10
; filament_pre_cooling_temperature = 0
; filament_pre_cooling_temperature_nc = 0
; filament_preheat_temperature_delta = 0
; filament_prime_volume = 45
; filament_prime_volume_nc = 60
; filament_printable = 3
; filament_ramming_travel_time = 0
; filament_ramming_travel_time_nc = 0
; filament_ramming_volumetric_speed = -1
; filament_ramming_volumetric_speed_nc = -1
; filament_retract_length_nc = 14
; filament_retraction_distances_when_cut = 18
; filament_retraction_length = 0.4
; filament_scarf_gap = 0%
; filament_scarf_height = 10%
; filament_scarf_length = 10
; filament_scarf_seam_type = none
; filament_self_index = 1
; filament_settings_id = "Bambu PETG Basic @BBL A1M 0.4 nozzle"
; filament_shrink = 100%
; filament_soluble = 0
; filament_start_gcode = "; filament start gcode\n{if (bed_temperature[current_extruder] >80)||(bed_temperature_initial_layer[current_extruder] >80)}M106 P3 S255\n{elsif (bed_temperature[current_extruder] >60)||(bed_temperature_initial_layer[current_extruder] >60)}M106 P3 S180\n{endif}\n\n{if activate_air_filtration[current_extruder] && support_air_filtration}\nM106 P3 S{during_print_exhaust_fan_speed_num[current_extruder]} \n{endif}"
; filament_tower_interface_pre_extrusion_dist = 10
; filament_tower_interface_pre_extrusion_length = 0
; filament_tower_interface_print_temp = -1
; filament_tower_interface_purge_volume = 20
; filament_tower_ironing_area = 4
; filament_type = PETG
; filament_velocity_adaptation_factor = 1
; filament_vendor = "Bambu Lab"
; filament_volume_map = 0
; filament_wipe = 1
; filament_wipe_distance = 1
; filament_z_hop_types = Spiral Lift
; filename_format = {input_filename_base}_{filament_type[0]}_{print_time}.gcode
; fill_multiline = 1
; filter_out_gap_fill = 0
; first_layer_print_sequence = 0
; first_x_layer_fan_speed = 0
; first_x_layer_part_fan_speed = 0
; flush_into_infill = 0
; flush_into_objects = 0
; flush_into_support = 1
; flush_multiplier = 1
; flush_multiplier_fast = 1.2
; flush_volumes_matrix = 0
; flush_volumes_vector = 140,140
; full_fan_speed_layer = 0
; fuzzy_skin = none
; fuzzy_skin_first_layer = 0
; fuzzy_skin_mode = displacement
; fuzzy_skin_noise_type = classic
; fuzzy_skin_octaves = 4
; fuzzy_skin_persistence = 0.5
; fuzzy_skin_point_distance = 0.8
; fuzzy_skin_scale = 1
; fuzzy_skin_thickness = 0.3
; gap_infill_speed = 250
; gcode_add_line_number = 0
; gcode_flavor = marlin
; grab_length = 17.4
; group_algo_with_time = 0
; has_filament_switcher = 0
; has_scarf_joint_seam = 0
; head_wrap_detect_zone = 156x152,180x152,180x180,156x180
; hole_coef_1 = 0
; hole_coef_2 = -0.008
; hole_coef_3 = 0.23415
; hole_limit_max = 0.22
; hole_limit_min = 0.088
; host_type = octoprint
; hot_plate_temp = 70
; hot_plate_temp_initial_layer = 70
; hotend_cooling_rate = 2
; hotend_heating_rate = 2
; impact_strength_z = 13.6
; independent_support_layer_height = 1
; infill_combination = 0
; infill_direction = 45
; infill_instead_top_bottom_surfaces = 0
; infill_jerk = 9
; infill_lock_depth = 1
; infill_rotate_step = 0
; infill_shift_step = 0.4
; infill_wall_overlap = 15%
; inherits_group = "0.20mm Standard @BBL A1M";;
; initial_layer_acceleration = 500
; initial_layer_flow_ratio = 1
; initial_layer_infill_speed = 105
; initial_layer_jerk = 9
; initial_layer_line_width = 0.5
; initial_layer_print_height = 0.2
; initial_layer_speed = 50
; initial_layer_travel_acceleration = 6000
; inner_wall_acceleration = 0
; inner_wall_jerk = 9
; inner_wall_line_width = 0.45
; inner_wall_speed = 300
; interface_shells = 0
; interlocking_beam = 0
; interlocking_beam_layer_count = 2
; interlocking_beam_width = 0.8
; interlocking_boundary_avoidance = 2
; interlocking_depth = 2
; interlocking_orientation = 22.5
; internal_bridge_support_thickness = 0.8
; internal_solid_infill_line_width = 0.42
; internal_solid_infill_pattern = zig-zag
; internal_solid_infill_speed = 250
; ironing_direction = 45
; ironing_fan_speed = -1
; ironing_flow = 10%
; ironing_inset = 0.21
; ironing_pattern = zig-zag
; ironing_spacing = 0.15
; ironing_speed = 30
; ironing_type = no ironing
; is_infill_first = 0
; layer_change_gcode = ; layer num/total_layer_count: {layer_num+1}/[total_layer_count]\n; update layer progress\nM73 L{layer_num+1}\nM991 S0 P{layer_num} ;notify layer change
; layer_height = 0.2
; line_width = 0.42
; locked_skeleton_infill_pattern = zigzag
; locked_skin_infill_pattern = crosszag
; long_retractions_when_cut = 0
; long_retractions_when_ec = 0
; machine_bed_mass_Y = 0
; machine_end_gcode = ;===== date: 20260513 =====================\n;turn off nozzle clog detect\nG392 S0\n\nM400 ; wait for buffer to clear\nG92 E0 ; zero the extruder\nG90\nG1 Z{max_layer_z + 0.4} F900 ; lower z a little\nG1 X0 Y{first_layer_center_no_wipe_tower[1]} F18000 ; move to safe pos\nG1 X-13.0 F3000 ; move to safe pos\n{if !spiral_mode && print_sequence != \"by object\"}\nM1002 judge_flag timelapse_record_flag\nM622 J1\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM991 S0 P-1 ;end timelapse at safe pos\nM623\n{endif}\n\nM140 S0 ; turn off bed\nM106 S0 ; turn off fan\nM106 P2 S0 ; turn off remote part cooling fan\nM106 P3 S0 ; turn off chamber cooling fan\n\n;G1 X27 F15000 ; wipe\n\n; pull back filament to AMS\nM620 S255\nG1 X181 F12000\nT255\nG1 X0 F18000\nG1 X-13.0 F3000\nG1 X0 F18000 ; wipe\nM621 S255\n\nM104 S0 ; turn off hotend\n\nM400 ; wait all motion done\nM17 S\nM17 Z0.4 ; lower z motor current to reduce impact if there is something in the bottom\n{if (max_layer_z + 100.0) < 180}\n    G1 Z{max_layer_z + 100.0} F600\n    G1 Z{max_layer_z +98.0}\n{else}\n    G1 Z180 F600\n    G1 Z180\n{endif}\nM400 P100\nM17 R ; restore z current\n\nG90\nG1 X-13 Y180 F3600\n\nG91\nG1 Z-1 F600\nG90\nM83\n\nM220 S100  ; Reset feedrate magnitude\nM201.2 K1.0 ; Reset acc magnitude\nM73.2   R1.0 ;Reset left time magnitude\nM1002 set_gcode_claim_speed_level : 0\n\n;=====printer finish  sound=========\nM17\nM400 S1\nM1006 S1\nM1006 A0 B20 L100 C37 D20 M100 E42 F20 N100\nM1006 A0 B10 L100 C44 D10 M100 E44 F10 N100\nM1006 A0 B10 L100 C46 D10 M100 E46 F10 N100\nM1006 A44 B20 L100 C39 D20 M100 E48 F20 N100\nM1006 A0 B10 L100 C44 D10 M100 E44 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B10 L100 C39 D10 M100 E39 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B10 L100 C44 D10 M100 E44 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B10 L100 C39 D10 M100 E39 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A44 B10 L100 C0 D10 M100 E48 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A44 B20 L100 C41 D20 M100 E49 F20 N100\nM1006 A0 B20 L100 C0 D20 M100 E0 F20 N100\nM1006 A0 B20 L100 C37 D20 M100 E37 F20 N100\nM1006 W\n;=====printer finish  sound=========\nM400 S1\nM18 X Y Z\n
; machine_hotend_change_time = 0
; machine_load_filament_time = 28
; machine_max_acceleration_e = 5000,5000
; machine_max_acceleration_extruding = 20000,20000
; machine_max_acceleration_retracting = 5000,5000
; machine_max_acceleration_travel = 9000,9000
; machine_max_acceleration_x = 20000,20000
; machine_max_acceleration_y = 20000,20000
; machine_max_acceleration_z = 1500,1500
; machine_max_force_Y = 0
; machine_max_jerk_e = 3,3
; machine_max_jerk_x = 9,9
; machine_max_jerk_y = 9,9
; machine_max_jerk_z = 5,5
; machine_max_printed_mass = 0
; machine_max_speed_e = 30,30
; machine_max_speed_x = 500,200
; machine_max_speed_y = 500,200
; machine_max_speed_z = 30,30
; machine_min_extruding_rate = 0,0
; machine_min_travel_rate = 0,0
; machine_pause_gcode = M400 U1
; machine_prepare_compensation_time = 260
; machine_start_gcode = ;===== machine: A1 mini =========================\n;===== date: 20260513 ==================\n\n;===== start to heat heatbead&hotend==========\nM1002 gcode_claim_action : 2\nM1002 set_filament_type:{filament_type[initial_no_support_extruder]}\nM104 S170\nM140 S[bed_temperature_initial_layer_single]\nG392 S0 ;turn off clog detect\nM9833.2\n;=====start printer sound ===================\nM17\nM400 S1\nM1006 S1\nM1006 A0 B0 L100 C37 D10 M100 E37 F10 N100\nM1006 A0 B0 L100 C41 D10 M100 E41 F10 N100\nM1006 A0 B0 L100 C44 D10 M100 E44 F10 N100\nM1006 A0 B10 L100 C0 D10 M100 E0 F10 N100\nM1006 A43 B10 L100 C39 D10 M100 E46 F10 N100\nM1006 A0 B0 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B0 L100 C39 D10 M100 E43 F10 N100\nM1006 A0 B0 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B0 L100 C41 D10 M100 E41 F10 N100\nM1006 A0 B0 L100 C44 D10 M100 E44 F10 N100\nM1006 A0 B0 L100 C49 D10 M100 E49 F10 N100\nM1006 A0 B0 L100 C0 D10 M100 E0 F10 N100\nM1006 A44 B10 L100 C39 D10 M100 E48 F10 N100\nM1006 A0 B0 L100 C0 D10 M100 E0 F10 N100\nM1006 A0 B0 L100 C39 D10 M100 E44 F10 N100\nM1006 A0 B0 L100 C0 D10 M100 E0 F10 N100\nM1006 A43 B10 L100 C39 D10 M100 E46 F10 N100\nM1006 W\nM18\n;=====avoid end stop =================\nG91\nG380 S2 Z30 F1200\nG380 S3 Z-20 F1200\nG1 Z5 F1200\nG90\n\n;===== reset machine status =================\nM204 S6000\n\nM630 S0 P0\nG91\nM17 Z0.3 ; lower the z-motor current\n\nG90\nM17 X0.7 Y0.9 Z0.5 ; reset motor current to default\nM960 S5 P1 ; turn on logo lamp\nG90\nM83\nM220 S100 ;Reset Feedrate\nM221 S100 ;Reset Flowrate\nM73.2   R1.0 ;Reset left time magnitude\n;====== cog noise reduction=================\nM982.2 S1 ; turn on cog noise reduction\n\n;===== prepare print temperature and material ==========\nM400\nM18\nM109 S100 H170\nM104 S170\nM400\nM17\nM400\nG28 X\n\nM211 X0 Y0 Z0 ;turn off soft endstop ; turn off soft endstop to prevent protential logic problem\n\nM975 S1 ; turn on\n\nG1 X0.0 F30000\nG1 X-13.5 F3000\n\nM620 M ;enable remap\nM620 S[initial_no_support_extruder]A   ; switch material if AMS exist\n    G392 S0 ;turn on clog detect\n    M1002 gcode_claim_action : 4\n    M400\n    M1002 set_filament_type:UNKNOWN\n    M109 S[nozzle_temperature_initial_layer]\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M104 S220\n{else}\n    M104 S250\n{endif}\n    M400\n    T[initial_no_support_extruder]\n    G1 X-13.5 F3000\n    M400\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M620.1 E F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60} T220\n    M109 S220 ;set nozzle to common flush temp\n{else}\n    M620.1 E F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60} T{flush_temperatures[initial_no_support_extruder]}\n    M109 S250 ;set nozzle to common flush temp\n{endif}\n    M106 P1 S0\n    G92 E0\n    G1 E50 F200\n    M400\n    M1002 set_filament_type:{filament_type[initial_no_support_extruder]}\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M104 S220\n{else}\n    M104 S{flush_temperatures[initial_no_support_extruder]}\n{endif}\n    G92 E0\n    G1 E50 F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60}\n    M400\n    M106 P1 S178\n    G92 E0\n    G1 E5 F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60}\n    M109 S{nozzle_temperature_initial_layer[initial_no_support_extruder]-20} ; drop nozzle temp, make filament shink a bit\n    M104 S{nozzle_temperature_initial_layer[initial_no_support_extruder]-40}\n    G92 E0\n    G1 E-0.5 F300\n\n    G1 X0 F30000\n    G1 X-13.5 F3000\n    G1 X0 F30000 ;wipe and shake\n    G1 X-13.5 F3000\n    G1 X0 F12000 ;wipe and shake\n    G1 X0 F30000\n    G1 X-13.5 F3000\n    M109 S{nozzle_temperature_initial_layer[initial_no_support_extruder]-40}\n    G392 S0 ;turn off clog detect\nM621 S[initial_no_support_extruder]A\n\nM400\nM106 P1 S0\n;===== prepare print temperature and material end =====\n\n\n;===== mech mode fast check============================\nM1002 gcode_claim_action : 3\nG0 X25 Y175 F20000 ; find a soft place to home\n;M104 S0\nG28 Z P0 T300; home z with low precision,permit 300deg temperature\nG29.2 S0 ; turn off ABL\nM104 S170\n\n; build plate detect\nM1002 judge_flag build_plate_detect_flag\nM622 S1\n  G39.4\n  M400\nM623\n\nG1 Z5 F3000\nG1 X90 Y-1 F30000\nM400 P200\nM970.3 Q1 A7 K0 O2\nM974 Q1 S2 P0\n\nG1 X90 Y0 Z5 F30000\nM400 P200\nM970 Q0 A10 B50 C90 H15 K0 M20 O3\nM974 Q0 S2 P0\n\nM975 S1\nG1 F30000\nG1 X-1 Y10\nG28 X ; re-home XY\n\n;===== wipe nozzle ===============================\nM1002 gcode_claim_action : 14\nM975 S1\n\nM104 S170 ; set temp down to heatbed acceptable\nM106 S255 ; turn on fan (G28 has turn off fan)\nM211 S; push soft endstop status\nM211 X0 Y0 Z0 ;turn off Z axis endstop\n\nM83\nG1 E-1 F500\nG90\nM83\n\nM109 S170\nM104 S140\nG0 X90 Y-4 F30000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X91 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X92 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X93 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X94 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X95 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X96 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X97 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X98 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X99 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X99 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X99 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X99 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X99 F10000\nG380 S3 Z-5 F1200\n\nG1 Z5 F30000\n;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;\nG1 X25 Y175 F30000.1 ;Brush material\nG1 Z0.2 F30000.1\nG1 Y185\nG91\nG1 X-30 F30000\nG1 Y-2\nG1 X27\nG1 Y1.5\nG1 X-28\nG1 Y-2\nG1 X30\nG1 Y1.5\nG1 X-30\nG90\nM83\n\nG1 Z5 F3000\nG0 X50 Y175 F20000 ; find a soft place to home\nG28 Z P0 T300; home z with low precision, permit 300deg temperature\nG29.2 S0 ; turn off ABL\n\nG0 X85 Y185 F10000 ;move to exposed steel surface and stop the nozzle\nG0 Z-1.01 F10000\nG91\n\nG2 I1 J0 X2 Y0 F2000.1\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\n\nG90\nG1 Z5 F30000\nG1 X25 Y175 F30000.1 ;Brush material\nG1 Z0.2 F30000.1\nG1 Y185\nG91\nG1 X-30 F30000\nG1 Y-2\nG1 X27\nG1 Y1.5\nG1 X-28\nG1 Y-2\nG1 X30\nG1 Y1.5\nG1 X-30\nG90\nM83\n\nG1 Z5\nG0 X55 Y175 F20000 ; find a soft place to home\nG28 Z P0 T300; home z with low precision, permit 300deg temperature\nG29.2 S0 ; turn off ABL\n\nG1 Z10\nG1 X85 Y185\nG1 Z-1.01\nG1 X95\nG1 X90\n\nM211 R; pop softend status\n\nM106 S0 ; turn off fan , too noisy\n;===== wipe nozzle end ================================\n\n\n;===== wait heatbed  ====================\nM1002 gcode_claim_action:54\nM104 S0\nM190 S[bed_temperature_initial_layer_single];set bed temp\nM109 S140\n\nG1 Z5 F3000\nG29.2 S1\nG1 X10 Y10 F20000\n\n;===== bed leveling ==================================\n;M1002 set_flag g29_before_print_flag=1\nM1002 judge_flag g29_before_print_flag\nM622 J1\n    M1002 gcode_claim_action : 1\n    G29 A1 X{first_layer_print_min[0]} Y{first_layer_print_min[1]} I{first_layer_print_size[0]} J{first_layer_print_size[1]}\n    M400\n    M500 ; save cali data\nM623\n;===== bed leveling end ================================\n\n;===== home after wipe mouth============================\nM1002 judge_flag g29_before_print_flag\nM622 J0\n\n    M1002 gcode_claim_action : 13\n    G28 T145\n\nM623\n\n;===== home after wipe mouth end =======================\n\nM975 S1 ; turn on vibration supression\n;===== nozzle load line ===============================\nM975 S1\nG90\nM83\nT1000\n\nG1 X-13.5 Y0 Z10 F10000\nG1 E1.2 F500\nM400\nM1002 set_filament_type:UNKNOWN\nM109 S{nozzle_temperature[initial_extruder]}\nM400\n\nM412 S1 ;    ===turn on  filament runout detection===\nM400 P10\n\nG392 S0 ;turn on clog detect\n\nM620.3 W1; === turn on filament tangle detection===\nM400 S2\n\nM1002 set_filament_type:{filament_type[initial_no_support_extruder]}\n;M1002 set_flag extrude_cali_flag=1\nM1002 judge_flag extrude_cali_flag\nM622 J1\n    M1002 gcode_claim_action : 8\n    \n    M400\n    M900 K0.0 L1000.0 M1.0\n    G90\n    M83\n    G0 X68 Y-4 F30000\n    G0 Z0.3 F18000 ;Move to start position\n    M400\n    G0 X88 E10  F{outer_wall_volumetric_speed/(24/20)    * 60}\n    G0 X93 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X98 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X103 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X108 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X113 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 Y0 Z0 F20000\n    M400\n    \n    G1 X-13.5 Y0 Z10 F10000\n    M400\n    \n    G1 E10 F{outer_wall_volumetric_speed/2.4*60}\n    M983 F{outer_wall_volumetric_speed/2.4} A0.3 H[nozzle_diameter]; cali dynamic extrusion compensation\n    M106 P1 S178\n    M400 S7\n    G1 X0 F18000\n    G1 X-13.5 F3000\n    G1 X0 F18000 ;wipe and shake\n    G1 X-13.5 F3000\n    G1 X0 F12000 ;wipe and shake\n    G1 X-13.5 F3000\n    M400\n    M106 P1 S0\n\n    M1002 judge_last_extrude_cali_success\n    M622 J0\n        M983 F{outer_wall_volumetric_speed/2.4} A0.3 H[nozzle_diameter]; cali dynamic extrusion compensation\n        M106 P1 S178\n        M400 S7\n        G1 X0 F18000\n        G1 X-13.5 F3000\n        G1 X0 F18000 ;wipe and shake\n        G1 X-13.5 F3000\n        G1 X0 F12000 ;wipe and shake\n        M400\n        M106 P1 S0\n    M623\n    \n    G1 X-13.5 F3000\n    M400\n    M984 A0.1 E1 S1 F{outer_wall_volumetric_speed/2.4} H[nozzle_diameter]\n    M106 P1 S178\n    M400 S7\n    G1 X0 F18000\n    G1 X-13.5 F3000\n    G1 X0 F18000 ;wipe and shake\n    G1 X-13.5 F3000\n    G1 X0 F12000 ;wipe and shake\n    G1 X-13.5 F3000\n    M400\n    M106 P1 S0\n\nM623 ; end of \"draw extrinsic para cali paint\"\n\n;===== extrude cali test ===============================\nM104 S{nozzle_temperature_initial_layer[initial_extruder]}\nG90\nM83\nG0 X68 Y-2.5 F30000\nG0 Z0.3 F18000 ;Move to start position\nG0 X88 E10  F{outer_wall_volumetric_speed/(24/20)    * 60}\nG0 X93 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\nG0 X98 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\nG0 X103 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\nG0 X108 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\nG0 X113 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\nG0 X115 Z0 F20000\nG0 Z5\nM400\n\n;========turn off light and wait extrude temperature =============\nM1002 gcode_claim_action : 0\n\nM400 ; wait all motion done before implement the emprical L parameters\n\n;===== for Textured PEI Plate , lower the nozzle as the nozzle was touching topmost of the texture when homing ==\n;curr_bed_type={curr_bed_type}\n{if curr_bed_type==\"Textured PEI Plate\"}\nG29.1 Z{-0.02} ; for Textured PEI Plate\n{endif}\n\nM960 S1 P0 ; turn off laser\nM960 S2 P0 ; turn off laser\nM106 S0 ; turn off fan\nM106 P2 S0 ; turn off big fan\nM106 P3 S0 ; turn off chamber fan\n\nM975 S1 ; turn on mech mode supression\nG90\nM83\nT1000\n\nM211 X0 Y0 Z0 ;turn off soft endstop\nM1007 S1\n\n\n\n
; machine_switch_extruder_time = 0
; machine_unload_filament_time = 34
; master_extruder_id = 1
; max_bridge_length = 0
; max_layer_height = 0.28
; max_travel_detour_distance = 0
; min_bead_width = 85%
; min_feature_size = 25%
; min_layer_height = 0.08
; minimum_sparse_infill_area = 15
; mmu_segmented_region_interlocking_depth = 0
; mmu_segmented_region_max_width = 0
; monotonic_travel_into_wall = 0%
; no_slow_down_for_cooling_on_outwalls = 0
; nozzle_diameter = 0.4
; nozzle_flush_dataset = 0
; nozzle_height = 4.76
; nozzle_temperature = 245
; nozzle_temperature_initial_layer = 245
; nozzle_temperature_range_high = 270
; nozzle_temperature_range_low = 230
; nozzle_type = stainless_steel
; nozzle_volume = 92
; nozzle_volume_type = Standard
; only_one_wall_first_layer = 0
; ooze_prevention = 0
; other_layers_print_sequence = 0
; other_layers_print_sequence_nums = 0
; outer_wall_acceleration = 5000
; outer_wall_jerk = 9
; outer_wall_line_width = 0.42
; outer_wall_speed = 200
; overhang_1_4_speed = 0
; overhang_2_4_speed = 50
; overhang_3_4_speed = 30
; overhang_4_4_speed = 10
; overhang_fan_speed = 50
; overhang_fan_threshold = 10%
; overhang_threshold_participating_cooling = 95%
; overhang_totally_speed = 10
; override_filament_scarf_seam_setting = 0
; override_process_overhang_speed = 0
; physical_extruder_map = 0
; post_process = "\"/Users/wzy/projects/mkpse-next_v3/mkpsupporte/build/bin/mkp-supporte.app/Contents/MacOS/MKPSupporte\" --Toml \"/Users/wzy/Documents/MKPSupportE/presets/mkp/A1MF_260628.toml\" --Gcode"
; pre_start_fan_time = 2
; precise_outer_wall = 0
; precise_z_height = 0
; pressure_advance = 0.02
; prime_tower_brim_width = 3
; prime_tower_enable_framework = 0
; prime_tower_extra_rib_length = 0
; prime_tower_fillet_wall = 1
; prime_tower_flat_ironing = 0
; prime_tower_infill_gap = 150%
; prime_tower_lift_height = -1
; prime_tower_lift_speed = 90
; prime_tower_max_speed = 90
; prime_tower_rib_wall = 1
; prime_tower_rib_width = 8
; prime_tower_skip_points = 1
; prime_tower_width = 35
; prime_volume_mode = Default
; print_compatible_printers = "Bambu Lab A1 mini 0.4 nozzle"
; print_extruder_id = 1
; print_extruder_variant = "Direct Drive Standard"
; print_flow_ratio = 1
; print_in_clockwise = 0
; print_sequence = by layer
; print_settings_id = 0.20mm Standard @BBL A1M
; printable_area = 0x0,180x0,180x180,0x180
; printable_height = 180
; printer_extruder_id = 1
; printer_extruder_variant = "Direct Drive Standard"
; printer_model = Bambu Lab A1 mini
; printer_notes = 
; printer_settings_id = Bambu Lab A1 mini 0.4 nozzle
; printer_structure = i3
; printer_technology = FFF
; printer_variant = 0.4
; printhost_authorization_type = key
; printhost_ssl_ignore_revoke = 0
; printing_by_object_gcode = 
; process_notes = 
; raft_contact_distance = 0.1
; raft_expansion = 1.5
; raft_first_layer_density = 100%
; raft_first_layer_expansion = -1
; raft_layers = 0
; reduce_crossing_wall = 0
; reduce_fan_stop_start_freq = 1
; reduce_infill_retraction_mode = Auto
; required_nozzle_HRC = 3
; resolution = 0.012
; retract_before_wipe = 0%
; retract_length_toolchange = 2
; retract_lift_above = 0
; retract_lift_below = 179
; retract_restart_extra = 0
; retract_restart_extra_toolchange = 0
; retract_when_changing_layer = 1
; retraction_distances_when_cut = 18
; retraction_distances_when_ec = 0
; retraction_length = 0.8
; retraction_minimum_travel = 1
; retraction_speed = 30
; role_base_wipe_speed = 1
; scan_first_layer = 0
; scarf_angle_threshold = 155
; seam_gap = 15%
; seam_placement_away_from_overhangs = 0
; seam_position = aligned
; seam_slope_conditional = 1
; seam_slope_entire_loop = 0
; seam_slope_gap = 0
; seam_slope_inner_walls = 1
; seam_slope_min_length = 10
; seam_slope_start_height = 10%
; seam_slope_steps = 10
; seam_slope_type = none
; silent_mode = 0
; single_extruder_multi_material = 1
; skeleton_infill_density = 15%
; skeleton_infill_line_width = 0.45
; skin_infill_density = 15%
; skin_infill_depth = 2
; skin_infill_line_width = 0.45
; skirt_distance = 2
; skirt_height = 1
; skirt_loops = 0
; skirt_per_object = 1
; slice_closing_radius = 0.049
; slicing_mode = regular
; slow_down_for_layer_cooling = 1
; slow_down_layer_time = 12
; slow_down_min_speed = 10
; slowdown_end_acc = 100000
; slowdown_end_height = 400
; slowdown_end_speed = 1000
; slowdown_start_acc = 100000
; slowdown_start_height = 0
; slowdown_start_speed = 1000
; small_perimeter_speed = 50%
; small_perimeter_threshold = 0
; smooth_coefficient = 80
; smooth_speed_discontinuity_area = 1
; solid_infill_filament = 0
; sparse_infill_acceleration = 100%
; sparse_infill_anchor = 400%
; sparse_infill_anchor_max = 20
; sparse_infill_density = 15%
; sparse_infill_filament = 0
; sparse_infill_lattice_angle_1 = -45
; sparse_infill_lattice_angle_2 = 45
; sparse_infill_line_width = 0.45
; sparse_infill_pattern = grid
; sparse_infill_speed = 270
; spiral_mode = 0
; spiral_mode_max_xy_smoothing = 200%
; spiral_mode_smooth = 0
; standby_temperature_delta = -5
; start_end_points = 30x-3,54x245
; supertack_plate_temp = 70
; supertack_plate_temp_initial_layer = 70
; support_air_filtration = 0
; support_angle = 0
; support_base_pattern = default
; support_base_pattern_spacing = 2.5
; support_bottom_interface_spacing = 0.5
; support_bottom_z_distance = 0.2
; support_chamber_temp_control = 0
; support_cooling_filter = 0
; support_critical_regions_only = 0
; support_expansion = 0
; support_fast_purge_mode = 0
; support_filament = 0
; support_interface_bottom_layers = 0
; support_interface_filament = 1
; support_interface_loop_pattern = 0
; support_interface_not_for_body = 1
; support_interface_pattern = rectilinear_interlaced
; support_interface_spacing = 0
; support_interface_speed = 80
; support_interface_top_layers = 1
; support_ironing_direction = 0
; support_ironing_flow = 15%
; support_ironing_inset = 0
; support_ironing_pattern = concentric
; support_ironing_spacing = 0.3
; support_ironing_speed = 70
; support_line_width = 0.42
; support_object_first_layer_gap = 0.45
; support_object_skip_flush = 0
; support_object_xy_distance = 0.3
; support_on_build_plate_only = 0
; support_remove_small_overhang = 0
; support_speed = 150
; support_style = grid
; support_threshold_angle = 30
; support_top_z_distance = 0
; support_type = normal(auto)
; symmetric_infill_y_axis = 0
; temperature_vitrification = 60
; template_custom_gcode = 
; textured_plate_temp = 70
; textured_plate_temp_initial_layer = 70
; thick_bridges = 0
; thumbnail_size = 50x50
; time_lapse_gcode = ;===================== date: 20250206 =====================\n{if !spiral_mode && print_sequence != \"by object\"}\n; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer\n; SKIPPABLE_START\n; SKIPTYPE: timelapse\nM622.1 S1 ; for prev firmware, default turned on\nM1002 judge_flag timelapse_record_flag\nM622 J1\nG92 E0\nG1 Z{max_layer_z + 0.4}\nG1 X0 Y{first_layer_center_no_wipe_tower[1]} F18000 ; move to safe pos\nG1 X-13.0 F3000 ; move to safe pos\nM400\nM1004 S5 P1  ; external shutter\nM400 P300\nM971 S11 C11 O0\nG92 E0\nG1 X0 F18000\nM623\n\n; SKIPTYPE: head_wrap_detect\nM622.1 S1\nM1002 judge_flag g39_3rd_layer_detect_flag\nM622 J1\n    ; enable nozzle clog detect at 3rd layer\n    {if layer_num == 2}\n      M400\n      G90\n      M83\n      M204 S5000\n      G0 Z2 F4000\n      G0 X187 Y178 F20000\n      G39 S1 X187 Y178\n      G0 Z2 F4000\n    {endif}\n\n\n    M622.1 S1\n    M1002 judge_flag g39_detection_flag\n    M622 J1\n      {if !in_head_wrap_detect_zone}\n        M622.1 S0\n        M1002 judge_flag g39_mass_exceed_flag\n        M622 J1\n        {if layer_num > 2}\n            G392 S0\n            M400\n            G90\n            M83\n            M204 S5000\n            G0 Z{max_layer_z + 0.4} F4000\n            G39.3 S1\n            G0 Z{max_layer_z + 0.4} F4000\n            G392 S0\n          {endif}\n        M623\n    {endif}\n    M623\nM623\n; SKIPPABLE_END\n{endif}\n\n\n
; timelapse_type = 0
; top_area_threshold = 200%
; top_color_penetration_layers = 5
; top_one_wall_type = all top
; top_shell_layers = 5
; top_shell_thickness = 1
; top_solid_infill_flow_ratio = 1
; top_surface_acceleration = 2000
; top_surface_density = 100%
; top_surface_jerk = 9
; top_surface_line_width = 0.42
; top_surface_pattern = monotonicline
; top_surface_speed = 200
; top_z_overrides_xy_distance = 0
; travel_acceleration = 10000
; travel_jerk = 9
; travel_short_distance_acceleration = 250
; travel_speed = 700
; travel_speed_z = 0
; tree_support_branch_angle = 45
; tree_support_branch_diameter = 2
; tree_support_branch_diameter_angle = 5
; tree_support_branch_distance = 5
; tree_support_wall_count = 1
; upward_compatible_machine = "Bambu Lab P1S 0.4 nozzle";"Bambu Lab P1P 0.4 nozzle";"Bambu Lab X1 0.4 nozzle";"Bambu Lab X1 Carbon 0.4 nozzle";"Bambu Lab X1E 0.4 nozzle";"Bambu Lab A1 0.4 nozzle";"Bambu Lab H2D 0.4 nozzle";"Bambu Lab H2D Pro 0.4 nozzle";"Bambu Lab H2S 0.4 nozzle";"Bambu Lab P2S 0.4 nozzle";"Bambu Lab H2C 0.4 nozzle";"Bambu Lab X2D 0.4 nozzle";"Bambu Lab A2L 0.4 nozzle"
; use_firmware_retraction = 0
; use_relative_e_distances = 1
; vertical_shell_speed = 80%
; volumetric_speed_coefficients = "0 0 0 0 0 0"
; wall_distribution_count = 1
; wall_filament = 0
; wall_generator = classic
; wall_loops = 2
; wall_sequence = inner wall/outer wall
; wall_transition_angle = 10
; wall_transition_filter_deviation = 25%
; wall_transition_length = 100%
; wipe = 1
; wipe_distance = 2
; wipe_speed = 80%
; wipe_tower_no_sparse_layers = 0
; wipe_tower_rotation_angle = 0
; wipe_tower_x = 15,15,15,15,15,15,15,15
; wipe_tower_y = 140.972,140.972,140.972,140.972,140.972,140.972,140.972,130.196
; wrapping_detection_gcode = 
; wrapping_detection_layers = 20
; wrapping_exclude_area = 
; xy_contour_compensation = 0
; xy_hole_compensation = 0
; z_direction_outwall_speed_continuous = 1
; z_hop = 0.4
; z_hop_types = Auto Lift
; CONFIG_BLOCK_END

; EXECUTABLE_BLOCK_START
M73 P0 R16
M201 X20000 Y20000 Z1500 E5000
M203 X500 Y500 Z30 E30
M204 P20000 R5000 T20000
M205 X9.00 Y9.00 Z5.00 E3.00
M106 S0
; FEATURE: Custom
;===== machine: A1 mini =========================
;===== date: 20260513 ==================

;===== start to heat heatbead&hotend==========
M1002 gcode_claim_action : 2
M1002 set_filament_type:PETG
M104 S170
M140 S70
G392 S0 ;turn off clog detect
M9833.2
;=====start printer sound ===================
M17
M400 S1
M1006 S1
M1006 A0 B0 L100 C37 D10 M100 E37 F10 N100
M1006 A0 B0 L100 C41 D10 M100 E41 F10 N100
M1006 A0 B0 L100 C44 D10 M100 E44 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A43 B10 L100 C39 D10 M100 E46 F10 N100
M1006 A0 B0 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B0 L100 C39 D10 M100 E43 F10 N100
M1006 A0 B0 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B0 L100 C41 D10 M100 E41 F10 N100
M1006 A0 B0 L100 C44 D10 M100 E44 F10 N100
M1006 A0 B0 L100 C49 D10 M100 E49 F10 N100
M1006 A0 B0 L100 C0 D10 M100 E0 F10 N100
M1006 A44 B10 L100 C39 D10 M100 E48 F10 N100
M1006 A0 B0 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B0 L100 C39 D10 M100 E44 F10 N100
M1006 A0 B0 L100 C0 D10 M100 E0 F10 N100
M1006 A43 B10 L100 C39 D10 M100 E46 F10 N100
M1006 W
M18
;=====avoid end stop =================
G91
G380 S2 Z30 F1200
G380 S3 Z-20 F1200
G1 Z5 F1200
G90

;===== reset machine status =================
M204 S6000

M630 S0 P0
G91
M17 Z0.3 ; lower the z-motor current

G90
M17 X0.7 Y0.9 Z0.5 ; reset motor current to default
M960 S5 P1 ; turn on logo lamp
G90
M83
M220 S100 ;Reset Feedrate
M221 S100 ;Reset Flowrate
M73.2   R1.0 ;Reset left time magnitude
;====== cog noise reduction=================
M982.2 S1 ; turn on cog noise reduction

;===== prepare print temperature and material ==========
M400
M18
M109 S100 H170
M104 S170
M400
M17
M400
G28 X

M211 X0 Y0 Z0 ;turn off soft endstop ; turn off soft endstop to prevent protential logic problem

M975 S1 ; turn on

G1 X0.0 F30000
G1 X-13.5 F3000

M620 M ;enable remap
M620 S0A   ; switch material if AMS exist
    G392 S0 ;turn on clog detect
    M1002 gcode_claim_action : 4
    M400
    M1002 set_filament_type:UNKNOWN
    M109 S245

    M104 S250

    M400
    T0
    G1 X-13.5 F3000
    M400

    M620.1 E F324.284 T270
    M109 S250 ;set nozzle to common flush temp

    M106 P1 S0
    G92 E0
M73 P2 R16
    G1 E50 F200
    M400
    M1002 set_filament_type:PETG

    M104 S270

    G92 E0
    G1 E50 F324.284
    M400
    M106 P1 S178
    G92 E0
M73 P4 R16
    G1 E5 F324.284
    M109 S225 ; drop nozzle temp, make filament shink a bit
    M104 S205
    G92 E0
M73 P5 R15
    G1 E-0.5 F300

    G1 X0 F30000
    G1 X-13.5 F3000
    G1 X0 F30000 ;wipe and shake
    G1 X-13.5 F3000
    G1 X0 F12000 ;wipe and shake
    G1 X0 F30000
    G1 X-13.5 F3000
    M109 S205
    G392 S0 ;turn off clog detect
M621 S0A

M400
M106 P1 S0
;===== prepare print temperature and material end =====


;===== mech mode fast check============================
M1002 gcode_claim_action : 3
G0 X25 Y175 F20000 ; find a soft place to home
;M104 S0
G28 Z P0 T300; home z with low precision,permit 300deg temperature
G29.2 S0 ; turn off ABL
M104 S170

; build plate detect
M1002 judge_flag build_plate_detect_flag
M622 S1
  G39.4
  M400
M623

G1 Z5 F3000
G1 X90 Y-1 F30000
M400 P200
M970.3 Q1 A7 K0 O2
M974 Q1 S2 P0

G1 X90 Y0 Z5 F30000
M400 P200
M970 Q0 A10 B50 C90 H15 K0 M20 O3
M974 Q0 S2 P0

M975 S1
G1 F30000
G1 X-1 Y10
G28 X ; re-home XY

;===== wipe nozzle ===============================
M1002 gcode_claim_action : 14
M975 S1

M104 S170 ; set temp down to heatbed acceptable
M106 S255 ; turn on fan (G28 has turn off fan)
M211 S; push soft endstop status
M211 X0 Y0 Z0 ;turn off Z axis endstop

M83
G1 E-1 F500
G90
M83

M109 S170
M104 S140
G0 X90 Y-4 F30000
G380 S3 Z-5 F1200
M73 P31 R11
G1 Z2 F1200
G1 X91 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X92 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X93 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X94 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X95 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X96 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X97 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X98 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X99 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X99 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X99 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X99 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X99 F10000
G380 S3 Z-5 F1200

G1 Z5 F30000
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
G1 X25 Y175 F30000.1 ;Brush material
G1 Z0.2 F30000.1
G1 Y185
G91
G1 X-30 F30000
G1 Y-2
G1 X27
G1 Y1.5
G1 X-28
G1 Y-2
G1 X30
G1 Y1.5
G1 X-30
G90
M83

G1 Z5 F3000
G0 X50 Y175 F20000 ; find a soft place to home
G28 Z P0 T300; home z with low precision, permit 300deg temperature
G29.2 S0 ; turn off ABL

G0 X85 Y185 F10000 ;move to exposed steel surface and stop the nozzle
G0 Z-1.01 F10000
G91

G2 I1 J0 X2 Y0 F2000.1
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5
G2 I1 J0 X2
G2 I-0.75 J0 X-1.5

G90
G1 Z5 F30000
G1 X25 Y175 F30000.1 ;Brush material
G1 Z0.2 F30000.1
G1 Y185
G91
G1 X-30 F30000
G1 Y-2
G1 X27
G1 Y1.5
G1 X-28
M73 P32 R11
G1 Y-2
G1 X30
G1 Y1.5
G1 X-30
G90
M83

G1 Z5
G0 X55 Y175 F20000 ; find a soft place to home
G28 Z P0 T300; home z with low precision, permit 300deg temperature
G29.2 S0 ; turn off ABL

G1 Z10
G1 X85 Y185
G1 Z-1.01
G1 X95
G1 X90

M211 R; pop softend status

M106 S0 ; turn off fan , too noisy
;===== wipe nozzle end ================================


;===== wait heatbed  ====================
M1002 gcode_claim_action:54
M104 S0
M190 S70;set bed temp
M109 S140

G1 Z5 F3000
G29.2 S1
G1 X10 Y10 F20000

;===== bed leveling ==================================
;M1002 set_flag g29_before_print_flag=1
M1002 judge_flag g29_before_print_flag
M622 J1
    M1002 gcode_claim_action : 1
    G29 A1 X78.2273 Y78.2274 I23.5453 J23.5453
    M400
    M500 ; save cali data
M623
;===== bed leveling end ================================

;===== home after wipe mouth============================
M1002 judge_flag g29_before_print_flag
M622 J0

    M1002 gcode_claim_action : 13
    G28 T145

M623

;===== home after wipe mouth end =======================

M975 S1 ; turn on vibration supression
;===== nozzle load line ===============================
M975 S1
G90
M83
T1000

G1 X-13.5 Y0 Z10 F10000
G1 E1.2 F500
M400
M1002 set_filament_type:UNKNOWN
M109 S245
M400

M412 S1 ;    ===turn on  filament runout detection===
M400 P10

G392 S0 ;turn on clog detect

M620.3 W1; === turn on filament tangle detection===
M400 S2

M1002 set_filament_type:PETG
;M1002 set_flag extrude_cali_flag=1
M1002 judge_flag extrude_cali_flag
M622 J1
    M1002 gcode_claim_action : 8
    
    M400
    M900 K0.0 L1000.0 M1.0
    G90
    M83
    G0 X68 Y-4 F30000
    G0 Z0.3 F18000 ;Move to start position
    M400
    G0 X88 E10  F780
    G0 X93 E.3742  F1300
    G0 X98 E.3742  F5200
    G0 X103 E.3742  F1300
    G0 X108 E.3742  F5200
    G0 X113 E.3742  F1300
    G0 Y0 Z0 F20000
    M400
    
    G1 X-13.5 Y0 Z10 F10000
    M400
    
    G1 E10 F325
    M983 F5.41667 A0.3 H0.4; cali dynamic extrusion compensation
    M106 P1 S178
    M400 S7
    G1 X0 F18000
    G1 X-13.5 F3000
    G1 X0 F18000 ;wipe and shake
    G1 X-13.5 F3000
    G1 X0 F12000 ;wipe and shake
    G1 X-13.5 F3000
    M400
    M106 P1 S0

    M1002 judge_last_extrude_cali_success
    M622 J0
        M983 F5.41667 A0.3 H0.4; cali dynamic extrusion compensation
        M106 P1 S178
        M400 S7
        G1 X0 F18000
        G1 X-13.5 F3000
        G1 X0 F18000 ;wipe and shake
        G1 X-13.5 F3000
        G1 X0 F12000 ;wipe and shake
        M400
        M106 P1 S0
    M623
    
M73 P33 R11
    G1 X-13.5 F3000
    M400
    M984 A0.1 E1 S1 F5.41667 H0.4
    M106 P1 S178
    M400 S7
    G1 X0 F18000
    G1 X-13.5 F3000
    G1 X0 F18000 ;wipe and shake
    G1 X-13.5 F3000
    G1 X0 F12000 ;wipe and shake
    G1 X-13.5 F3000
    M400
    M106 P1 S0

M623 ; end of "draw extrinsic para cali paint"

;===== extrude cali test ===============================
M104 S245
G90
M83
G0 X68 Y-2.5 F30000
G0 Z0.3 F18000 ;Move to start position
G0 X88 E10  F780
G0 X93 E.3742  F1300
G0 X98 E.3742  F5200
G0 X103 E.3742  F1300
G0 X108 E.3742  F5200
G0 X113 E.3742  F1300
G0 X115 Z0 F20000
G0 Z5
M400

;========turn off light and wait extrude temperature =============
M1002 gcode_claim_action : 0

M400 ; wait all motion done before implement the emprical L parameters

;===== for Textured PEI Plate , lower the nozzle as the nozzle was touching topmost of the texture when homing ==
;curr_bed_type=Textured PEI Plate

G29.1 Z-0.02 ; for Textured PEI Plate


M960 S1 P0 ; turn off laser
M960 S2 P0 ; turn off laser
M106 S0 ; turn off fan
M106 P2 S0 ; turn off big fan
M106 P3 S0 ; turn off chamber fan

M975 S1 ; turn on mech mode supression
G90
M83
T1000

M211 X0 Y0 Z0 ;turn off soft endstop
M1007 S1



; MACHINE_START_GCODE_END
; filament start gcode
M106 P3 S180


;VT0 H-1
G90
G21
M83 ; use relative distances for extrusion
M981 S1 P20000 ;open spaghetti detector
; CHANGE_LAYER
; Z_HEIGHT: 0.2
; LAYER_HEIGHT: 0.2
G1 E-.4 F1800
; layer num/total_layer_count: 1/43
; update layer progress
M73 L1
M991 S0 P0 ;notify layer change
M106 S0
; OBJECT_ID: 346
G1 X96.987 Y80.63 F42000
M204 S6000
G1 Z.4
G1 Z.2
M73 P34 R10
G1 E.4 F1800
; FEATURE: Support
; LINE_WIDTH: 0.5
G1 F3000
M204 S500
G1 X83.219 Y80.63 E.49189
G1 X83.219 Y81.087 E.01633
G1 X96.781 Y81.087 E.48454
G1 X96.781 Y81.544 E.01633
M73 P35 R10
G1 X83.219 Y81.544 E.48454
G1 X83.219 Y82.001 E.01633
G1 X96.781 Y82.001 E.48454
G1 X96.781 Y82.458 E.01633
G1 X83.219 Y82.458 E.48454
G1 X83.219 Y82.915 E.01633
G1 X96.781 Y82.915 E.48454
G1 X96.781 Y83.219 E.01083
G1 X99.658 Y83.219 E.10279
G1 X99.658 Y83.373 E.0055
G1 X80.342 Y83.373 E.69012
G1 X80.342 Y83.83 E.01633
G1 X99.658 Y83.83 E.69012
G1 X99.658 Y84.287 E.01633
G1 X80.342 Y84.287 E.69012
G1 X80.342 Y84.744 E.01633
G1 X99.658 Y84.744 E.69012
G1 X99.658 Y85.201 E.01633
G1 X80.342 Y85.201 E.69012
G1 X80.342 Y85.658 E.01633
G1 X99.658 Y85.658 E.69012
G1 X99.658 Y86.115 E.01633
G1 X80.342 Y86.115 E.69012
M73 P36 R10
G1 X80.342 Y86.572 E.01633
G1 X99.658 Y86.572 E.69012
G1 X99.658 Y87.029 E.01633
G1 X80.342 Y87.029 E.69012
G1 X80.342 Y87.486 E.01633
G1 X99.658 Y87.486 E.69012
G1 X99.658 Y87.943 E.01633
G1 X80.342 Y87.943 E.69012
G1 X80.342 Y88.4 E.01633
G1 X99.658 Y88.4 E.69012
G1 X99.658 Y88.857 E.01633
G1 X80.342 Y88.857 E.69012
G1 X80.342 Y89.314 E.01633
G1 X99.658 Y89.314 E.69012
G1 X99.658 Y89.772 E.01633
G1 X80.342 Y89.772 E.69012
G1 X80.342 Y90.229 E.01633
G1 X99.658 Y90.229 E.69012
G1 X99.658 Y90.686 E.01633
G1 X80.342 Y90.686 E.69012
G1 X80.342 Y91.143 E.01633
G1 X99.658 Y91.143 E.69012
G1 X99.658 Y91.6 E.01633
G1 X80.342 Y91.6 E.69012
G1 X80.342 Y92.057 E.01633
G1 X99.658 Y92.057 E.69012
G1 X99.658 Y92.514 E.01633
G1 X80.342 Y92.514 E.69012
G1 X80.342 Y92.971 E.01633
G1 X99.658 Y92.971 E.69012
G1 X99.658 Y93.428 E.01633
G1 X80.342 Y93.428 E.69012
G1 X80.342 Y93.885 E.01633
G1 X99.658 Y93.885 E.69012
G1 X99.658 Y94.342 E.01633
G1 X80.342 Y94.342 E.69012
G1 X80.342 Y94.799 E.01633
G1 X99.658 Y94.799 E.69012
G1 X99.658 Y95.256 E.01633
G1 X80.342 Y95.256 E.69012
G1 X80.342 Y95.713 E.01633
G1 X99.658 Y95.713 E.69012
G1 X99.658 Y96.17 E.01633
G1 X80.342 Y96.17 E.69012
G1 X80.342 Y96.628 E.01633
G1 X99.658 Y96.628 E.69012
G1 X99.658 Y96.781 E.0055
G1 X96.781 Y96.781 E.10279
G1 X96.781 Y97.085 E.01083
M73 P37 R10
G1 X83.219 Y97.085 E.48454
G1 X83.219 Y97.542 E.01633
G1 X96.781 Y97.542 E.48454
G1 X96.781 Y97.999 E.01633
G1 X83.219 Y97.999 E.48454
G1 X83.219 Y98.456 E.01633
G1 X96.781 Y98.456 E.48454
G1 X96.781 Y98.913 E.01633
G1 X83.219 Y98.913 E.48454
G1 X83.219 Y99.37 E.01633
G1 X96.987 Y99.37 E.49189
; WIPE_START
G1 X95.987 Y99.37 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.214 J.088 P1  F42000
G1 X97.193 Y82.807 Z.6
G1 Z.2
G1 E.4 F1800
G1 F3000
M204 S500
G1 X100.07 Y82.807 E.10279
G1 X100.07 Y97.193 E.51394
G1 X97.193 Y97.193 E.10279
G1 X97.193 Y100.07 E.10279
G1 X82.807 Y100.07 E.51394
G1 X82.807 Y97.193 E.10279
G1 X79.93 Y97.193 E.10279
G1 X79.93 Y82.807 E.51394
G1 X82.807 Y82.807 E.10279
G1 X82.807 Y79.93 E.10279
G1 X97.193 Y79.93 E.51394
G1 X97.193 Y82.739 E.10034
; WIPE_START
G1 X97.193 Y81.739 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z0.6
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
        M623
    
    M623
M623
; SKIPPABLE_END




; CHANGE_LAYER
; Z_HEIGHT: 0.35
; LAYER_HEIGHT: 0.15
; layer num/total_layer_count: 2/43
; update layer progress
M73 L2
M991 S0 P1 ;notify layer change
; open powerlost recovery
M1003 S1
; OBJECT_ID: 346
M204 S10000
G1 X95.733 Y84.209 F42000
G1 Z.35
G1 E.4 F1800
; LINE_WIDTH: 0.42
G1 F919
M204 S6000
G1 X95.733 Y81.39 E.06408
G1 X84.267 Y81.39 E.26065
G1 X84.267 Y84.267 E.06541
G1 X81.39 Y84.267 E.06541
G1 X81.39 Y95.733 E.26065
G1 X84.267 Y95.733 E.06541
G1 X84.267 Y98.61 E.06541
G1 X95.733 Y98.61 E.26065
G1 X95.733 Y95.733 E.06541
G1 X98.61 Y95.733 E.06541
M73 P38 R10
G1 X98.61 Y84.267 E.26065
G1 X95.733 Y84.267 E.06541
M204 S10000
G1 X95.559 Y84.246 F42000
G1 F919
M204 S6000
G1 X84.611 Y84.246 E.24889
G1 X84.611 Y84.611 E.0083
G1 X81.734 Y84.611 E.06541
G1 X81.734 Y87.123 E.05711
G1 X86.381 Y87.123 E.10564
G1 X86.381 Y90 E.06541
G1 X81.734 Y90 E.10564
G1 X81.734 Y92.877 E.06541
G1 X86.381 Y92.877 E.10564
G1 X86.381 Y93.619 E.01688
G1 X93.619 Y93.619 E.16457
G1 X93.619 Y92.877 E.01688
G1 X98.266 Y92.877 E.10564
G1 X98.266 Y90 E.06541
G1 X93.619 Y90 E.10564
G1 X93.619 Y87.123 E.06541
G1 X98.436 Y87.123 E.1095
M204 S10000
G1 X95.559 Y95.754 F42000
G1 F919
M204 S6000
G1 X84.441 Y95.754 E.25274
M204 S10000
G1 X86.724 Y86.782 F42000
G1 F919
M204 S6000
G1 X86.724 Y93.276 E.14762
G1 X93.276 Y93.276 E.14894
G1 X93.276 Y86.724 E.14894
G1 X86.724 Y86.724 E.14894
; WIPE_START
G1 F9000
G1 X87.724 Y86.724 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.75 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z0.75
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
        M623
    
    M623
M623
; SKIPPABLE_END




; CHANGE_LAYER
; Z_HEIGHT: 0.5
; LAYER_HEIGHT: 0.15
; layer num/total_layer_count: 3/43
; update layer progress
M73 L3
M991 S0 P2 ;notify layer change
; OBJECT_ID: 346
G1 X87.172 Y86.916 F42000
G1 Z.5
G1 E.4 F1800
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X87.172 Y84.246 E.0607
G1 X87.549 Y84.246 E.00857
M73 P39 R10
G1 X87.549 Y86.746 E.05684
G1 X87.926 Y86.746 E.00857
G1 X87.926 Y84.246 E.05684
G1 X88.303 Y84.246 E.00857
G1 X88.303 Y86.746 E.05684
G1 X88.68 Y86.746 E.00857
G1 X88.68 Y84.246 E.05684
G1 X89.057 Y84.246 E.00857
G1 X89.057 Y86.746 E.05684
G1 X89.434 Y86.746 E.00857
G1 X89.434 Y84.246 E.05684
G1 X89.811 Y84.246 E.00857
G1 X89.811 Y86.746 E.05684
G1 X90.188 Y86.746 E.00857
G1 X90.188 Y84.246 E.05684
G1 X90.566 Y84.246 E.00857
G1 X90.566 Y86.746 E.05684
G1 X90.943 Y86.746 E.00857
G1 X90.943 Y84.246 E.05684
G1 X91.32 Y84.246 E.00857
G1 X91.32 Y86.746 E.05684
G1 X91.697 Y86.746 E.00857
G1 X91.697 Y84.246 E.05684
G1 X92.074 Y84.246 E.00857
G1 X92.074 Y86.746 E.05684
G1 X92.451 Y86.746 E.00857
G1 X92.451 Y84.246 E.05684
G1 X92.828 Y84.246 E.00857
G1 X92.828 Y86.916 E.0607
M204 S10000
G1 X95.733 Y84.209 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X95.733 Y81.39 E.06408
G1 X84.267 Y81.39 E.26065
G1 X84.267 Y84.267 E.06541
G1 X81.39 Y84.267 E.06541
G1 X81.39 Y95.733 E.26065
G1 X84.267 Y95.733 E.06541
G1 X84.267 Y98.61 E.06541
G1 X95.733 Y98.61 E.26065
G1 X95.733 Y95.733 E.06541
G1 X98.61 Y95.733 E.06541
G1 X98.61 Y84.267 E.26065
G1 X95.733 Y84.267 E.06541
M204 S10000
G1 X95.559 Y84.246 F42000
G1 F9000
M204 S6000
G1 X93.619 Y84.246 E.04409
G1 X93.619 Y86.381 E.04853
G1 X96.496 Y86.381 E.06541
G1 X96.496 Y87.123 E.01688
G1 X98.266 Y87.123 E.04023
G1 X98.266 Y90 E.06541
G1 X96.496 Y90 E.04023
G1 X96.496 Y92.877 E.06541
G1 X98.266 Y92.877 E.04023
G1 X98.266 Y95.389 E.05711
G1 X95.389 Y95.389 E.06541
G1 X95.389 Y95.754 E.0083
G1 X93.619 Y95.754 E.04023
G1 X93.619 Y96.497 E.01688
G1 X86.381 Y96.497 E.16457
G1 X86.381 Y95.754 E.01688
G1 X84.611 Y95.754 E.04023
G1 X84.611 Y95.389 E.0083
G1 X81.734 Y95.389 E.06541
G1 X81.734 Y92.877 E.05711
G1 X83.504 Y92.877 E.04023
G1 X83.504 Y90 E.06541
G1 X81.734 Y90 E.04023
G1 X81.734 Y87.123 E.06541
G1 X83.504 Y87.123 E.04023
G1 X83.504 Y86.381 E.01688
G1 X86.381 Y86.381 E.06541
G1 X86.381 Y84.246 E.04853
G1 X84.441 Y84.246 E.04409
M204 S10000
G1 X86.724 Y83.905 F42000
G1 F9000
M204 S6000
G1 X86.724 Y86.724 E.06408
G1 X83.847 Y86.724 E.06541
G1 X83.847 Y93.276 E.14894
G1 X86.724 Y93.276 E.06541
G1 X86.724 Y96.153 E.06541
G1 X93.276 Y96.153 E.14894
G1 X93.276 Y93.276 E.06541
G1 X96.153 Y93.276 E.06541
G1 X96.153 Y86.724 E.14894
G1 X93.276 Y86.724 E.06541
G1 X93.276 Y83.847 E.06541
G1 X86.724 Y83.847 E.14894
; WIPE_START
G1 X87.724 Y83.847 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.9 I-1.19 J-.255 P1  F42000
G1 X86.953 Y87.448 Z.9
G1 Z.5
G1 E.4 F1800
; FEATURE: Support interface
; LAYER_HEIGHT: 0.3
G1 F4800
M204 S6000
G1 X87.278 Y87.123 E.01915
G1 X87.811 Y87.123 E.02223
G1 X87.123 Y87.811 E.04059
G1 X87.123 Y88.344 E.02223
G1 X88.344 Y87.123 E.07204
G1 X88.878 Y87.123 E.02223
G1 X87.123 Y88.878 E.10348
G1 X87.123 Y89.411 E.02223
G1 X89.411 Y87.123 E.13492
G1 X89.944 Y87.123 E.02223
G1 X87.123 Y89.944 E.16636
G1 X87.123 Y90.478 E.02223
G1 X90.477 Y87.123 E.19781
G1 X91.011 Y87.123 E.02223
G1 X87.123 Y91.011 E.22925
G1 X87.123 Y91.544 E.02223
G1 X91.544 Y87.123 E.26069
G1 X92.077 Y87.123 E.02223
G1 X87.123 Y92.077 E.29213
G1 X87.123 Y92.611 E.02223
G1 X92.611 Y87.123 E.32358
G1 X92.877 Y87.123 E.01112
G1 X92.877 Y87.389 E.01111
G1 X87.389 Y92.877 E.32359
G1 X87.923 Y92.877 E.02223
G1 X92.877 Y87.923 E.29214
G1 X92.877 Y88.456 E.02223
G1 X88.456 Y92.877 E.2607
G1 X88.989 Y92.877 E.02223
G1 X92.877 Y88.989 E.22926
G1 X92.877 Y89.522 E.02223
G1 X89.522 Y92.877 E.19782
G1 X90.056 Y92.877 E.02223
G1 X92.877 Y90.056 E.16637
G1 X92.877 Y90.589 E.02223
G1 X90.589 Y92.877 E.13493
G1 X91.122 Y92.877 E.02223
G1 X92.877 Y91.122 E.10349
G1 X92.877 Y91.655 E.02223
G1 X91.655 Y92.877 E.07204
G1 X92.189 Y92.877 E.02223
G1 X92.877 Y92.189 E.0406
G1 X92.877 Y92.722 E.02223
G1 X92.552 Y93.047 E.01916
M204 S10000
G1 X93.582 Y93.047 F42000
; FEATURE: Support transition
; LAYER_HEIGHT: 0.15
G1 F3000
M204 S6000
G1 X93.582 Y87.123 E.13468
G1 X93.959 Y87.123 E.00857
G1 X93.959 Y92.877 E.13083
G1 X94.336 Y92.877 E.00857
G1 X94.336 Y87.123 E.13083
G1 X94.713 Y87.123 E.00857
G1 X94.713 Y92.877 E.13083
M73 P40 R10
G1 X95.09 Y92.877 E.00857
G1 X95.09 Y87.123 E.13083
G1 X95.467 Y87.123 E.00857
G1 X95.467 Y93.047 E.13468
M204 S10000
G1 X92.828 Y95.924 F42000
G1 F3000
M204 S6000
G1 X92.828 Y93.254 E.0607
G1 X92.451 Y93.254 E.00857
G1 X92.451 Y95.754 E.05684
G1 X92.074 Y95.754 E.00857
G1 X92.074 Y93.254 E.05684
G1 X91.697 Y93.254 E.00857
G1 X91.697 Y95.754 E.05684
G1 X91.32 Y95.754 E.00857
G1 X91.32 Y93.254 E.05684
G1 X90.943 Y93.254 E.00857
G1 X90.943 Y95.754 E.05684
G1 X90.566 Y95.754 E.00857
G1 X90.566 Y93.254 E.05684
G1 X90.188 Y93.254 E.00857
G1 X90.188 Y95.754 E.05684
G1 X89.811 Y95.754 E.00857
G1 X89.811 Y93.254 E.05684
G1 X89.434 Y93.254 E.00857
G1 X89.434 Y95.754 E.05684
G1 X89.057 Y95.754 E.00857
G1 X89.057 Y93.254 E.05684
G1 X88.68 Y93.254 E.00857
G1 X88.68 Y95.754 E.05684
G1 X88.303 Y95.754 E.00857
G1 X88.303 Y93.254 E.05684
G1 X87.926 Y93.254 E.00857
G1 X87.926 Y95.754 E.05684
G1 X87.549 Y95.754 E.00857
G1 X87.549 Y93.254 E.05684
G1 X87.172 Y93.254 E.00857
G1 X87.172 Y95.924 E.0607
M204 S10000
G1 X86.418 Y93.047 F42000
G1 F3000
M204 S6000
G1 X86.418 Y87.123 E.13468
G1 X86.041 Y87.123 E.00857
G1 X86.041 Y92.877 E.13083
G1 X85.664 Y92.877 E.00857
G1 X85.664 Y87.123 E.13083
G1 X85.287 Y87.123 E.00857
G1 X85.287 Y92.877 E.13083
G1 X84.91 Y92.877 E.00857
G1 X84.91 Y87.123 E.13083
G1 X84.532 Y87.123 E.00857
G1 X84.532 Y93.047 E.13468
; WIPE_START
G1 X84.532 Y92.047 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.9 I.577 J1.071 P1  F42000
G1 X88.738 Y89.782 Z.9
G1 Z.5
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X88.702 Y89.86 E.00042
G1 X88.719 Y90.032 E.00084
G1 X88.699 Y90.116 E.00042
G1 X88.75 Y90.282 E.00084
G1 X88.747 Y90.368 E.00042
G1 X88.829 Y90.52 E.00084
G1 X88.843 Y90.605 E.00042
G1 X88.953 Y90.738 E.00084
G1 X88.983 Y90.819 E.00042
G1 X89.117 Y90.928 E.00084
G1 X89.163 Y91.002 E.00042
G1 X89.315 Y91.083 E.00084
G1 X89.374 Y91.146 E.00042
G1 X89.54 Y91.196 E.00084
G1 X89.61 Y91.246 E.00042
G1 X89.782 Y91.262 E.00084
G1 X89.86 Y91.298 E.00042
G1 X90.032 Y91.281 E.00084
G1 X90.116 Y91.301 E.00042
G1 X90.281 Y91.25 E.00084
G1 X90.368 Y91.253 E.00042
G1 X90.52 Y91.171 E.00084
G1 X90.605 Y91.157 E.00042
G1 X90.738 Y91.047 E.00084
G1 X90.819 Y91.017 E.00042
G1 X90.928 Y90.883 E.00084
G1 X91.002 Y90.837 E.00042
G1 X91.083 Y90.685 E.00084
G1 X91.146 Y90.626 E.00042
G1 X91.196 Y90.46 E.00084
G1 X91.246 Y90.39 E.00042
G1 X91.262 Y90.218 E.00084
G1 X91.298 Y90.14 E.00042
G1 X91.281 Y89.968 E.00084
G1 X91.301 Y89.884 E.00042
G1 X91.25 Y89.719 E.00084
G1 X91.253 Y89.632 E.00042
G1 X91.171 Y89.48 E.00084
G1 X91.157 Y89.395 E.00042
G1 X91.047 Y89.262 E.00084
G1 X91.017 Y89.181 E.00042
G1 X90.883 Y89.072 E.00084
G1 X90.837 Y88.998 E.00042
G1 X90.685 Y88.917 E.00084
G1 X90.626 Y88.854 E.00042
G1 X90.46 Y88.804 E.00084
G1 X90.39 Y88.754 E.00042
G1 X90.218 Y88.738 E.00084
G1 X90.139 Y88.702 E.00042
G1 X89.968 Y88.719 E.00084
G1 X89.884 Y88.699 E.00042
G1 X89.719 Y88.75 E.00084
G1 X89.632 Y88.747 E.00042
G1 X89.48 Y88.829 E.00084
G1 X89.395 Y88.843 E.00042
G1 X89.262 Y88.953 E.00084
G1 X89.181 Y88.983 E.00042
G1 X89.072 Y89.117 E.00084
G1 X88.998 Y89.163 E.00042
G1 X88.917 Y89.315 E.00084
G1 X88.854 Y89.374 E.00042
G1 X88.804 Y89.54 E.00084
G1 X88.747 Y89.632 E.00053
G1 X88.738 Y89.782 E.00073
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X89.019 Y89.904 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X89.019 Y90.097 E.00094
G1 X89.057 Y90.286 E.00094
G1 X89.131 Y90.465 E.00094
G1 X89.238 Y90.625 E.00094
G1 X89.375 Y90.762 E.00094
G1 X89.536 Y90.869 E.00094
G1 X89.714 Y90.943 E.00094
G1 X89.904 Y90.981 E.00094
G1 X90.097 Y90.981 E.00094
G1 X90.286 Y90.943 E.00093
G1 X90.464 Y90.869 E.00094
G1 X90.625 Y90.762 E.00094
G1 X90.762 Y90.625 E.00094
G1 X90.869 Y90.465 E.00093
G1 X90.943 Y90.286 E.00094
G1 X90.981 Y90.096 E.00094
G1 X90.981 Y89.903 E.00094
G1 X90.943 Y89.714 E.00094
G1 X90.869 Y89.536 E.00094
G1 X90.762 Y89.375 E.00094
G1 X90.625 Y89.238 E.00094
G1 X90.464 Y89.131 E.00094
G1 X90.286 Y89.057 E.00094
G1 X90.096 Y89.019 E.00094
G1 X89.904 Y89.019 E.00093
G1 X89.714 Y89.057 E.00094
G1 X89.535 Y89.131 E.00094
G1 X89.375 Y89.238 E.00094
G1 X89.238 Y89.375 E.00094
G1 X89.131 Y89.536 E.00094
G1 X89.057 Y89.714 E.00094
G1 X89.019 Y89.904 E.00094
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X89.319 Y89.933 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X89.345 Y89.802 E.00065
G1 X89.397 Y89.678 E.00065
G1 X89.471 Y89.566 E.00065
G1 X89.566 Y89.471 E.00065
G1 X89.677 Y89.397 E.00065
G1 X89.801 Y89.345 E.00065
G1 X89.933 Y89.319 E.00065
G1 X90.067 Y89.319 E.00065
G1 X90.198 Y89.345 E.00065
G1 X90.322 Y89.397 E.00065
G1 X90.434 Y89.471 E.00065
G1 X90.529 Y89.566 E.00065
G1 X90.603 Y89.678 E.00065
G1 X90.655 Y89.801 E.00065
G1 X90.681 Y89.933 E.00065
G1 X90.681 Y90.067 E.00065
G1 X90.655 Y90.199 E.00065
G1 X90.603 Y90.323 E.00065
G1 X90.529 Y90.434 E.00065
G1 X90.434 Y90.529 E.00065
G1 X90.322 Y90.603 E.00065
G1 X90.198 Y90.655 E.00065
G1 X90.067 Y90.681 E.00065
G1 X89.933 Y90.681 E.00065
G1 X89.802 Y90.655 E.00065
G1 X89.678 Y90.603 E.00065
G1 X89.566 Y90.529 E.00065
G1 X89.471 Y90.434 E.00065
G1 X89.397 Y90.323 E.00065
G1 X89.346 Y90.199 E.00065
G1 X89.319 Y90.067 E.00065
G1 X89.319 Y89.933 E.00065
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X89.619 Y89.963 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X89.634 Y90.111 E.00072
G1 X89.704 Y90.243 E.00072
G1 X89.82 Y90.337 E.00072
G1 X89.963 Y90.381 E.00072
G1 X90.111 Y90.366 E.00072
G1 X90.243 Y90.296 E.00072
G1 X90.337 Y90.18 E.00072
G1 X90.381 Y90.037 E.00073
G1 X90.366 Y89.889 E.00072
G1 X90.296 Y89.757 E.00072
G1 X90.18 Y89.663 E.00072
G1 X90.037 Y89.619 E.00072
G1 X89.889 Y89.634 E.00072
G1 X89.757 Y89.704 E.00072
G1 X89.663 Y89.82 E.00072
G1 X89.619 Y89.963 E.00072
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X89.663 Y89.82 E-.05678
G1 X89.757 Y89.704 E-.05665
G1 X89.889 Y89.634 E-.05673
G1 X90.037 Y89.619 E-.05672
G1 X90.18 Y89.663 E-.05673
G1 X90.296 Y89.757 E-.05674
G1 X90.345 Y89.849 E-.03964
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z0.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    
      M400
      G90
      M83
      M204 S5000
      G0 Z2 F4000
      G0 X187 Y178 F20000
      G39 S1 X187 Y178
      G0 Z2 F4000
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
        M623
    
    M623
M623
; SKIPPABLE_END




G1 Z0.900
; CHANGE_LAYER
; Z_HEIGHT: 0.7
; LAYER_HEIGHT: 0.2
; layer num/total_layer_count: 4/43
; update layer progress
M73 L4
M991 S0 P3 ;notify layer change
; OBJECT_ID: 346
G1 X91.32 Y86.953 F42000
G1 Z.7
G1 E.4 F1800
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X91.32 Y89.029 E.06117
G1 X91.245 Y88.925 E.00377
G1 X91.147 Y88.853 E.00358
G1 X91.075 Y88.755 E.00358
G1 X90.943 Y88.666 E.00471
G1 X90.943 Y87.123 E.04549
G1 X90.566 Y87.123 E.01111
G1 X90.566 Y88.463 E.03951
G1 X90.438 Y88.415 E.00403
G1 X90.316 Y88.409 E.00358
G1 X90.188 Y88.366 E.00398
G1 X90.188 Y87.123 E.03665
G1 X89.811 Y87.123 E.01111
G1 X89.811 Y88.366 E.03665
G1 X89.684 Y88.409 E.00397
G1 X89.562 Y88.414 E.00358
G1 X89.434 Y88.463 E.00403
G1 X89.434 Y87.123 E.03951
G1 X89.057 Y87.123 E.01111
G1 X89.057 Y88.666 E.04549
G1 X88.925 Y88.755 E.0047
G1 X88.853 Y88.853 E.00358
G1 X88.755 Y88.925 E.00358
G1 X88.68 Y89.029 E.00378
G1 X88.68 Y87.123 E.05618
G1 X88.303 Y87.123 E.01111
G1 X88.303 Y92.877 E.1696
G1 X88.68 Y92.877 E.01111
M73 P40 R9
G1 X88.68 Y90.971 E.05618
G1 X88.755 Y91.075 E.00377
G1 X88.853 Y91.147 E.00359
G1 X88.925 Y91.245 E.00358
G1 X89.057 Y91.334 E.00471
G1 X89.057 Y92.877 E.04549
G1 X89.434 Y92.877 E.01111
G1 X89.434 Y91.537 E.03951
G1 X89.562 Y91.586 E.00403
G1 X89.684 Y91.591 E.00358
G1 X89.811 Y91.634 E.00397
G1 X89.811 Y92.877 E.03665
G1 X90.188 Y92.877 E.01111
G1 X90.188 Y91.634 E.03665
G1 X90.316 Y91.591 E.00398
G1 X90.438 Y91.586 E.00357
G1 X90.566 Y91.537 E.00404
G1 X90.566 Y92.877 E.03951
G1 X90.943 Y92.877 E.01111
G1 X90.943 Y91.334 E.04549
G1 X91.075 Y91.245 E.0047
G1 X91.147 Y91.147 E.00358
G1 X91.245 Y91.075 E.00358
G1 X91.32 Y90.971 E.00377
G1 X91.32 Y92.877 E.05617
G1 X91.697 Y92.877 E.01111
G1 X91.697 Y87.123 E.1696
G1 X92.074 Y87.123 E.01111
G1 X92.074 Y92.877 E.1696
G1 X92.451 Y92.877 E.01111
G1 X92.451 Y87.123 E.1696
G1 X92.828 Y87.123 E.01111
G1 X92.828 Y93.047 E.1746
M204 S10000
G1 X93.582 Y93.047 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X93.582 Y87.123 E.17461
G1 X93.959 Y87.123 E.01111
G1 X93.959 Y92.877 E.16961
G1 X94.336 Y92.877 E.01111
G1 X94.336 Y87.123 E.16961
G1 X94.713 Y87.123 E.01111
G1 X94.713 Y92.877 E.16961
G1 X95.09 Y92.877 E.01111
G1 X95.09 Y87.123 E.16961
G1 X95.467 Y87.123 E.01111
G1 X95.467 Y93.047 E.17461
M204 S10000
G1 X92.828 Y95.924 F42000
G1 F3000
M204 S6000
G1 X92.828 Y93.254 E.07869
G1 X92.451 Y93.254 E.01111
G1 X92.451 Y95.754 E.07369
G1 X92.074 Y95.754 E.01111
G1 X92.074 Y93.254 E.07369
G1 X91.697 Y93.254 E.01111
G1 X91.697 Y95.754 E.07369
G1 X91.32 Y95.754 E.01111
G1 X91.32 Y93.254 E.07369
G1 X90.943 Y93.254 E.01111
G1 X90.943 Y95.754 E.07369
G1 X90.566 Y95.754 E.01111
G1 X90.566 Y93.254 E.07369
G1 X90.188 Y93.254 E.01111
G1 X90.188 Y95.754 E.07369
G1 X89.811 Y95.754 E.01111
G1 X89.811 Y93.254 E.07369
G1 X89.434 Y93.254 E.01111
G1 X89.434 Y95.754 E.07369
G1 X89.057 Y95.754 E.01111
G1 X89.057 Y93.254 E.07369
G1 X88.68 Y93.254 E.01111
G1 X88.68 Y95.754 E.07369
G1 X88.303 Y95.754 E.01111
G1 X88.303 Y93.254 E.07369
G1 X87.926 Y93.254 E.01111
G1 X87.926 Y95.754 E.07369
G1 X87.549 Y95.754 E.01111
G1 X87.549 Y93.254 E.07369
G1 X87.172 Y93.254 E.01111
G1 X87.172 Y95.924 E.07869
M204 S10000
G1 X84.532 Y93.047 F42000
G1 F3000
M204 S6000
G1 X84.532 Y87.123 E.17461
G1 X84.91 Y87.123 E.01111
G1 X84.91 Y92.877 E.16961
G1 X85.287 Y92.877 E.01111
G1 X85.287 Y87.123 E.16961
G1 X85.664 Y87.123 E.01111
G1 X85.664 Y92.877 E.16961
G1 X86.041 Y92.877 E.01111
G1 X86.041 Y87.123 E.16961
G1 X86.418 Y87.123 E.01111
G1 X86.418 Y93.047 E.17461
; WIPE_START
G1 X86.418 Y92.047 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.1 I-.672 J1.014 P1  F42000
G1 X87.926 Y93.047 Z1.1
G1 Z.7
G1 E.4 F1800
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X87.926 Y87.123 E.1746
G1 X87.549 Y87.123 E.01111
G1 X87.549 Y92.877 E.1696
G1 X87.172 Y92.877 E.01111
G1 X87.172 Y86.953 E.1746
M204 S10000
G1 X87.172 Y86.916 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X87.172 Y84.246 E.07869
G1 X87.549 Y84.246 E.01111
G1 X87.549 Y86.746 E.07369
G1 X87.926 Y86.746 E.01111
G1 X87.926 Y84.246 E.07369
G1 X88.303 Y84.246 E.01111
G1 X88.303 Y86.746 E.07369
G1 X88.68 Y86.746 E.01111
G1 X88.68 Y84.246 E.07369
G1 X89.057 Y84.246 E.01111
G1 X89.057 Y86.746 E.07369
G1 X89.434 Y86.746 E.01111
G1 X89.434 Y84.246 E.07369
G1 X89.811 Y84.246 E.01111
M73 P41 R9
G1 X89.811 Y86.746 E.07369
G1 X90.188 Y86.746 E.01111
G1 X90.188 Y84.246 E.07369
G1 X90.566 Y84.246 E.01111
G1 X90.566 Y86.746 E.07369
G1 X90.943 Y86.746 E.01111
G1 X90.943 Y84.246 E.07369
G1 X91.32 Y84.246 E.01111
G1 X91.32 Y86.746 E.07369
G1 X91.697 Y86.746 E.01111
G1 X91.697 Y84.246 E.07369
G1 X92.074 Y84.246 E.01111
G1 X92.074 Y86.746 E.07369
G1 X92.451 Y86.746 E.01111
G1 X92.451 Y84.246 E.07369
G1 X92.828 Y84.246 E.01111
G1 X92.828 Y86.916 E.07869
M204 S10000
G1 X95.733 Y84.211 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X95.733 Y81.39 E.08313
G1 X84.267 Y81.39 E.33792
G1 X84.267 Y84.267 E.0848
G1 X81.39 Y84.267 E.0848
G1 X81.39 Y95.733 E.33792
G1 X84.267 Y95.733 E.0848
G1 X84.267 Y98.61 E.0848
G1 X95.733 Y98.61 E.33792
G1 X95.733 Y95.733 E.0848
G1 X98.61 Y95.733 E.0848
G1 X98.61 Y84.267 E.33792
G1 X95.733 Y84.267 E.0848
M204 S10000
G1 X95.563 Y84.246 F42000
G1 F9000
M204 S6000
G1 X93.615 Y84.246 E.05741
G1 X93.615 Y86.385 E.06304
G1 X96.492 Y86.385 E.0848
G1 X96.492 Y87.123 E.02175
G1 X98.27 Y87.123 E.05241
G1 X98.27 Y90 E.0848
G1 X96.492 Y90 E.05241
G1 X96.492 Y92.877 E.0848
G1 X98.27 Y92.877 E.05241
G1 X98.27 Y95.393 E.07416
G1 X95.393 Y95.393 E.0848
G1 X95.393 Y95.754 E.01063
G1 X93.615 Y95.754 E.05241
G1 X93.615 Y96.492 E.02175
G1 X86.385 Y96.492 E.2131
G1 X86.385 Y95.754 E.02175
G1 X84.607 Y95.754 E.05241
G1 X84.607 Y95.393 E.01063
G1 X81.73 Y95.393 E.0848
G1 X81.73 Y92.877 E.07416
G1 X83.508 Y92.877 E.05241
G1 X83.508 Y90 E.0848
G1 X81.73 Y90 E.05241
G1 X81.73 Y87.123 E.0848
G1 X83.508 Y87.123 E.05241
G1 X83.508 Y86.385 E.02175
G1 X86.385 Y86.385 E.0848
G1 X86.385 Y84.246 E.06304
G1 X84.437 Y84.246 E.05741
M204 S10000
G1 X86.724 Y83.904 F42000
G1 F9000
M204 S6000
G1 X86.724 Y86.724 E.08313
G1 X83.847 Y86.724 E.0848
G1 X83.847 Y93.276 E.19309
G1 X86.724 Y93.276 E.0848
G1 X86.724 Y96.153 E.0848
G1 X93.276 Y96.153 E.19309
G1 X93.276 Y93.276 E.0848
G1 X96.153 Y93.276 E.0848
G1 X96.153 Y86.724 E.19309
G1 X93.276 Y86.724 E.0848
G1 X93.276 Y83.847 E.0848
G1 X86.724 Y83.847 E.19309
; WIPE_START
G1 X87.724 Y83.847 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.1 I-1.205 J.168 P1  F42000
G1 X88.502 Y89.443 Z1.1
G1 Z.7
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X88.452 Y89.57 E.00066
G1 X88.448 Y89.691 E.00059
G1 X88.406 Y89.805 E.00059
G1 X88.398 Y89.88 E.00037
G1 X88.418 Y90 E.00059
G1 X88.398 Y90.12 E.00059
G1 X88.406 Y90.195 E.00037
G1 X88.448 Y90.309 E.00059
G1 X88.452 Y90.43 E.00059
G1 X88.474 Y90.502 E.00037
G1 X88.538 Y90.605 E.00059
G1 X88.566 Y90.723 E.00059
G1 X88.602 Y90.79 E.00037
G1 X88.685 Y90.879 E.00059
G1 X88.735 Y90.989 E.00059
G1 X88.783 Y91.048 E.00037
G1 X88.881 Y91.119 E.00059
G1 X88.952 Y91.217 E.00059
G1 X89.01 Y91.265 E.00037
G1 X89.121 Y91.315 E.00059
G1 X89.21 Y91.398 E.00059
G1 X89.276 Y91.434 E.00037
G1 X89.395 Y91.462 E.00059
G1 X89.498 Y91.526 E.00059
G1 X89.57 Y91.548 E.00037
G1 X89.691 Y91.552 E.00059
G1 X89.805 Y91.594 E.00059
G1 X89.88 Y91.602 E.00037
G1 X90 Y91.582 E.00059
G1 X90.12 Y91.602 E.00059
G1 X90.195 Y91.594 E.00037
G1 X90.309 Y91.552 E.00059
G1 X90.43 Y91.548 E.00059
G1 X90.502 Y91.526 E.00037
G1 X90.605 Y91.462 E.00059
G1 X90.724 Y91.434 E.00059
G1 X90.791 Y91.398 E.00037
G1 X90.879 Y91.315 E.00059
G1 X90.989 Y91.265 E.00059
G1 X91.048 Y91.217 E.00037
G1 X91.119 Y91.119 E.00059
G1 X91.217 Y91.048 E.00059
G1 X91.265 Y90.99 E.00037
G1 X91.315 Y90.879 E.00059
G1 X91.398 Y90.79 E.00059
G1 X91.434 Y90.724 E.00037
G1 X91.462 Y90.605 E.00059
G1 X91.526 Y90.502 E.00059
G1 X91.548 Y90.43 E.00037
G1 X91.552 Y90.309 E.00059
G1 X91.594 Y90.195 E.00059
G1 X91.602 Y90.12 E.00037
G1 X91.582 Y90 E.00059
G1 X91.602 Y89.88 E.00059
G1 X91.594 Y89.805 E.00037
G1 X91.552 Y89.691 E.00059
G1 X91.548 Y89.57 E.00059
G1 X91.526 Y89.498 E.00037
G1 X91.462 Y89.395 E.00059
G1 X91.434 Y89.276 E.00059
G1 X91.398 Y89.21 E.00037
G1 X91.315 Y89.121 E.00059
G1 X91.265 Y89.011 E.00059
G1 X91.217 Y88.952 E.00037
G1 X91.119 Y88.881 E.00059
G1 X91.048 Y88.783 E.00059
G1 X90.99 Y88.735 E.00037
G1 X90.879 Y88.685 E.00059
G1 X90.79 Y88.602 E.00059
G1 X90.724 Y88.566 E.00037
G1 X90.605 Y88.538 E.00059
G1 X90.502 Y88.474 E.00059
G1 X90.43 Y88.452 E.00037
G1 X90.309 Y88.448 E.00059
G1 X90.195 Y88.406 E.00059
G1 X90.12 Y88.398 E.00037
G1 X90 Y88.418 E.00059
G1 X89.88 Y88.398 E.00059
G1 X89.805 Y88.406 E.00037
G1 X89.691 Y88.448 E.00059
G1 X89.57 Y88.452 E.00059
G1 X89.498 Y88.474 E.00037
G1 X89.395 Y88.538 E.00059
G1 X89.276 Y88.566 E.00059
G1 X89.21 Y88.602 E.00037
G1 X89.121 Y88.685 E.00059
G1 X89.011 Y88.735 E.00059
G1 X88.952 Y88.783 E.00037
G1 X88.881 Y88.881 E.00059
G1 X88.783 Y88.952 E.00059
G1 X88.735 Y89.011 E.00037
G1 X88.685 Y89.121 E.00059
G1 X88.602 Y89.209 E.00059
G1 X88.566 Y89.276 E.00037
G1 X88.538 Y89.395 E.00059
G1 X88.502 Y89.443 E.00029
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X88.251 Y89.276 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X88.158 Y89.508 E.00121
G1 X88.11 Y89.749 E.00119
G1 X88.097 Y90.123 E.00181
G1 X88.158 Y90.492 E.00181
G1 X88.289 Y90.841 E.00181
G1 X88.486 Y91.159 E.00181
G1 X88.741 Y91.432 E.00181
G1 X89.045 Y91.65 E.00181
G1 X89.385 Y91.805 E.00181
G1 X89.749 Y91.89 E.00181
G1 X90.123 Y91.903 E.00181
G1 X90.491 Y91.842 E.00181
G1 X90.842 Y91.711 E.00182
G1 X91.16 Y91.514 E.00181
G1 X91.432 Y91.259 E.00181
G1 X91.65 Y90.955 E.00181
G1 X91.805 Y90.614 E.00181
G1 X91.89 Y90.251 E.00181
G1 X91.903 Y89.877 E.00181
G1 X91.842 Y89.509 E.00181
G1 X91.711 Y89.158 E.00182
G1 X91.514 Y88.841 E.00181
G1 X91.259 Y88.568 E.00181
G1 X90.955 Y88.35 E.00181
G1 X90.615 Y88.195 E.00181
G1 X90.251 Y88.11 E.00181
G1 X89.877 Y88.097 E.00181
G1 X89.508 Y88.158 E.00181
G1 X89.158 Y88.289 E.00181
G1 X88.84 Y88.486 E.00181
G1 X88.568 Y88.741 E.00181
G1 X88.35 Y89.045 E.00181
G1 X88.251 Y89.276 E.00122
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X87.877 Y89.121 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X87.789 Y89.329 E.0011
G1 X87.701 Y89.774 E.0022
G1 X87.701 Y90.226 E.0022
G1 X87.789 Y90.671 E.0022
G1 X87.962 Y91.089 E.0022
G1 X88.214 Y91.466 E.0022
G1 X88.534 Y91.786 E.0022
G1 X88.911 Y92.038 E.0022
G1 X89.329 Y92.211 E.0022
G1 X89.774 Y92.299 E.0022
G1 X90.226 Y92.299 E.0022
G1 X90.671 Y92.211 E.0022
G1 X91.089 Y92.038 E.0022
G1 X91.466 Y91.786 E.0022
G1 X91.786 Y91.466 E.0022
G1 X92.038 Y91.089 E.0022
G1 X92.211 Y90.671 E.0022
G1 X92.299 Y90.226 E.0022
G1 X92.299 Y89.774 E.0022
G1 X92.211 Y89.329 E.0022
G1 X92.038 Y88.911 E.0022
G1 X91.786 Y88.534 E.0022
G1 X91.466 Y88.214 E.0022
G1 X91.089 Y87.962 E.0022
G1 X90.671 Y87.789 E.0022
G1 X90.226 Y87.701 E.0022
G1 X89.774 Y87.701 E.0022
G1 X89.329 Y87.789 E.0022
G1 X88.911 Y87.962 E.0022
G1 X88.534 Y88.214 E.0022
G1 X88.214 Y88.534 E.0022
G1 X87.962 Y88.911 E.0022
G1 X87.877 Y89.121 E.0011
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X87.6 Y89.006 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X87.698 Y88.768 E.00125
G1 X87.98 Y88.345 E.00247
G1 X88.342 Y87.983 E.00248
G1 X88.771 Y87.696 E.0025
G1 X89.24 Y87.502 E.00247
G1 X89.742 Y87.402 E.00248
G1 X90.258 Y87.402 E.0025
G1 X90.76 Y87.502 E.00248
G1 X91.229 Y87.696 E.00247
G1 X91.655 Y87.98 E.00248
G1 X92.017 Y88.342 E.00248
G1 X92.302 Y88.768 E.00248
G1 X92.498 Y89.24 E.00248
G1 X92.598 Y89.742 E.00248
G1 X92.599 Y90.254 E.00248
G1 X92.499 Y90.756 E.00248
G1 X92.304 Y91.229 E.00248
G1 X92.017 Y91.658 E.0025
G1 X91.658 Y92.017 E.00247
G1 X91.232 Y92.302 E.00248
G1 X90.76 Y92.498 E.00248
G1 X90.254 Y92.599 E.0025
G1 X89.746 Y92.599 E.00246
G1 X89.244 Y92.499 E.00248
G1 X88.771 Y92.304 E.00248
G1 X88.345 Y92.02 E.00248
G1 X87.983 Y91.658 E.00248
G1 X87.696 Y91.229 E.0025
G1 X87.502 Y90.76 E.00246
G1 X87.402 Y90.258 E.00248
G1 X87.401 Y89.746 E.00248
G1 X87.501 Y89.244 E.00248
G1 X87.6 Y89.006 E.00125
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X87.501 Y89.244 E-.09795
G1 X87.401 Y89.746 E-.19456
G1 X87.401 Y89.976 E-.08749
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.1 I.256 J1.19 P1  F42000
G1 X90.148 Y89.385 Z1.1
G1 Z.7
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F9580.435
M204 S6000
G1 X90.265 Y89.426 E.00394
G1 X90.372 Y89.488 E.00394
G1 X90.464 Y89.571 E.00394
G1 X90.539 Y89.669 E.00395
G1 X90.593 Y89.781 E.00395
G1 X90.625 Y89.901 E.00394
G1 X90.632 Y90.025 E.00394
G1 X90.615 Y90.148 E.00395
G1 X90.574 Y90.265 E.00395
G1 X90.512 Y90.372 E.00394
G1 X90.429 Y90.464 E.00394
G1 X90.331 Y90.539 E.00395
G1 X90.219 Y90.593 E.00395
G1 X90.099 Y90.625 E.00394
G1 X89.975 Y90.632 E.00394
G1 X89.853 Y90.615 E.00394
G1 X89.735 Y90.574 E.00394
G1 X89.628 Y90.512 E.00395
G1 X89.536 Y90.429 E.00394
G1 X89.461 Y90.331 E.00394
G1 X89.407 Y90.219 E.00395
G1 X89.375 Y90.099 E.00394
G1 X89.368 Y89.975 E.00394
G1 X89.385 Y89.853 E.00394
G1 X89.426 Y89.735 E.00394
G1 X89.488 Y89.628 E.00395
G1 X89.571 Y89.536 E.00394
G1 X89.669 Y89.461 E.00395
G1 X89.781 Y89.407 E.00395
G1 X89.901 Y89.375 E.00394
G1 X90.057 Y89.377 E.00496
G1 X90.088 Y89.38 E.00099
M204 S250
G1 X90.293 Y89.004 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F10342.643
M204 S5000
G1 X90.34 Y89.039 E.00171
G1 X90.482 Y89.081 E.00436
G1 X90.521 Y89.124 E.00171
G1 X90.652 Y89.192 E.00437
G1 X90.682 Y89.242 E.00171
G1 X90.797 Y89.335 E.00437
G1 X90.816 Y89.39 E.00171
G1 X90.912 Y89.503 E.00436
G1 X90.92 Y89.561 E.00171
G1 X90.991 Y89.691 E.00437
G1 X90.988 Y89.749 E.00171
G1 X91.032 Y89.89 E.00436
G1 X91.018 Y89.946 E.00171
G1 X91.034 Y90.093 E.00436
G1 X91.009 Y90.146 E.00171
G1 X90.996 Y90.293 E.00436
G1 X90.961 Y90.34 E.00171
G1 X90.919 Y90.482 E.00437
G1 X90.876 Y90.521 E.00171
G1 X90.808 Y90.652 E.00437
G1 X90.758 Y90.682 E.00171
G1 X90.665 Y90.797 E.00437
G1 X90.61 Y90.816 E.00171
G1 X90.497 Y90.912 E.00437
G1 X90.439 Y90.92 E.00171
G1 X90.309 Y90.991 E.00437
G1 X90.251 Y90.988 E.00171
G1 X90.11 Y91.032 E.00436
G1 X90.054 Y91.018 E.00171
G1 X89.907 Y91.034 E.00437
G1 X89.854 Y91.009 E.00171
G1 X89.707 Y90.996 E.00436
G1 X89.66 Y90.961 E.00171
G1 X89.518 Y90.919 E.00437
G1 X89.479 Y90.876 E.00171
G1 X89.348 Y90.808 E.00437
G1 X89.318 Y90.758 E.00171
G1 X89.203 Y90.665 E.00436
G1 X89.184 Y90.61 E.00171
G1 X89.088 Y90.497 E.00437
G1 X89.08 Y90.439 E.00171
G1 X89.009 Y90.309 E.00436
G1 X89.012 Y90.251 E.00171
G1 X88.968 Y90.11 E.00436
G1 X88.982 Y90.054 E.00171
G1 X88.966 Y89.907 E.00436
G1 X88.991 Y89.854 E.00171
G1 X89.004 Y89.707 E.00437
G1 X89.039 Y89.66 E.00171
G1 X89.081 Y89.518 E.00437
G1 X89.124 Y89.479 E.00171
G1 X89.192 Y89.348 E.00437
G1 X89.242 Y89.318 E.00171
G1 X89.335 Y89.203 E.00437
G1 X89.39 Y89.184 E.00171
G1 X89.503 Y89.088 E.00436
G1 X89.561 Y89.08 E.00171
G1 X89.691 Y89.009 E.00437
G1 X89.749 Y89.012 E.00171
G1 X89.89 Y88.968 E.00437
G1 X89.948 Y88.983 E.00177
G1 X90.087 Y88.965 E.00412
G1 X90.152 Y88.992 E.00206
G1 X90.234 Y88.999 E.00243
; WIPE_START
M204 S6000
G1 X90.34 Y89.039 E-.04315
G1 X90.482 Y89.081 E-.05627
G1 X90.521 Y89.124 E-.02208
G1 X90.652 Y89.192 E-.05629
G1 X90.682 Y89.242 E-.02207
G1 X90.797 Y89.335 E-.05629
G1 X90.816 Y89.39 E-.02208
G1 X90.912 Y89.503 E-.05628
G1 X90.92 Y89.561 E-.02207
G1 X90.949 Y89.615 E-.02343
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z1.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z1.1 F4000
            G39.3 S1
            G0 Z1.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.458 Y90.09 F42000
G1 Z.7
G1 E.4 F1800
; FEATURE: Bottom surface
; LINE_WIDTH: 0.4954
G1 F6300
M204 S6000
G1 X90.084 Y89.716 E.01871
G1 X89.897 Y89.721 E.00661
G1 X89.759 Y89.825 E.00611
G1 X89.704 Y89.976 E.00568
G1 X90.166 Y90.437 E.02307
; CHANGE_LAYER
; Z_HEIGHT: 0.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F6300
G1 X89.704 Y89.976 E-.2479
G1 X89.759 Y89.825 E-.06104
G1 X89.897 Y89.721 E-.06567
G1 X89.911 Y89.72 E-.0054
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 5/43
; update layer progress
M73 L5
M991 S0 P4 ;notify layer change
M106 S124.95
; OBJECT_ID: 346
M204 S10000
G17
G3 Z1.1 I.762 J-.949 P1  F42000
G1 X86.418 Y86.916 Z1.1
G1 Z.9
G1 E.4 F1800
; FEATURE: Support transition
; LINE_WIDTH: 0.42
G1 F3000
M204 S6000
G1 X86.418 Y84.246 E.07869
G1 X86.041 Y84.246 E.01111
G1 X86.041 Y86.746 E.07369
G1 X85.664 Y86.746 E.01111
G1 X85.664 Y84.246 E.07369
G1 X85.287 Y84.246 E.01111
G1 X85.287 Y86.746 E.07369
G1 X84.91 Y86.746 E.01111
G1 X84.91 Y84.246 E.07369
G1 X84.532 Y84.246 E.01111
G1 X84.532 Y86.916 E.07869
; WIPE_START
G1 X84.532 Y85.916 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I-.619 J1.048 P1  F42000
G1 X96.323 Y92.877 Z1.3
G1 Z.9
G1 E.4 F1800
; FEATURE: Support
G1 F3806
M204 S6000
G1 X98.27 Y92.877 E.05741
G1 X98.27 Y90 E.0848
G1 X96.492 Y90 E.05241
G1 X96.492 Y87.123 E.0848
G1 X98.44 Y87.123 E.05741
M204 S10000
G1 X98.61 Y84.267 F42000
G1 F3806
M204 S6000
G1 X98.61 Y95.733 E.33792
G1 X96.153 Y95.733 E.07242
G1 X96.153 Y84.267 E.33792
G1 X98.553 Y84.267 E.07075
; WIPE_START
G1 F9000
G1 X97.553 Y84.267 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I.274 J-1.186 P1  F42000
G1 X95.733 Y83.847 Z1.3
G1 Z.9
G1 E.4 F1800
G1 F3806
M204 S6000
G1 X84.267 Y83.847 E.33792
G1 X84.267 Y81.39 E.07241
G1 X95.733 Y81.39 E.33792
G1 X95.733 Y83.791 E.07075
M204 S10000
G1 X95.467 Y86.916 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X95.467 Y84.246 E.07869
G1 X95.09 Y84.246 E.01111
G1 X95.09 Y86.746 E.07369
G1 X94.713 Y86.746 E.01111
G1 X94.713 Y84.246 E.07369
G1 X94.336 Y84.246 E.01111
G1 X94.336 Y86.746 E.07369
G1 X93.959 Y86.746 E.01111
G1 X93.959 Y84.246 E.07369
G1 X93.582 Y84.246 E.01111
G1 X93.582 Y86.916 E.07869
; WIPE_START
G1 X93.582 Y85.916 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I1.135 J-.439 P1  F42000
G1 X93.047 Y84.533 Z1.3
G1 Z.9
G1 E.4 F1800
; FEATURE: Support interface
G1 F3806
M204 S6000
G1 X87.123 Y84.533 E.1746
G1 X87.123 Y84.91 E.01111
G1 X92.877 Y84.91 E.1696
G1 X92.877 Y85.287 E.01111
G1 X87.123 Y85.287 E.1696
G1 X87.123 Y85.664 E.01111
G1 X92.877 Y85.664 E.1696
G1 X92.877 Y86.041 E.01111
G1 X87.123 Y86.041 E.1696
G1 X87.123 Y86.418 E.01111
G1 X92.877 Y86.418 E.1696
G1 X92.877 Y86.795 E.01111
G1 X87.123 Y86.795 E.1696
G1 X87.123 Y87.123 E.00966
G1 X84.246 Y87.123 E.0848
G1 X84.246 Y87.172 E.00145
G1 X95.754 Y87.172 E.33919
G1 X95.754 Y87.549 E.01111
G1 X91.619 Y87.549 E.12189
G1 X91.861 Y87.711 E.00859
G1 X92.079 Y87.926 E.00904
G1 X95.754 Y87.926 E.10831
G1 X95.754 Y88.303 E.01111
G1 X92.401 Y88.303 E.09884
G1 X92.634 Y88.68 E.01307
G1 X95.754 Y88.68 E.09196
G1 X95.754 Y89.057 E.01111
G1 X92.791 Y89.057 E.08735
G1 X92.882 Y89.434 E.01143
G1 X95.754 Y89.434 E.08466
G1 X95.754 Y89.812 E.01111
G1 X92.939 Y89.812 E.08297
G1 X92.937 Y90 E.00555
G1 X92.939 Y90.189 E.00556
M73 P42 R9
G1 X95.754 Y90.189 E.08297
G1 X95.754 Y90.566 E.01111
G1 X92.882 Y90.566 E.08466
G1 X92.791 Y90.943 E.01143
G1 X95.754 Y90.943 E.08735
G1 X95.754 Y91.32 E.01111
G1 X92.634 Y91.32 E.09197
G1 X92.401 Y91.697 E.01307
G1 X95.754 Y91.697 E.09884
G1 X95.754 Y92.074 E.01111
G1 X92.079 Y92.074 E.10831
G1 X91.861 Y92.289 E.00903
G1 X91.618 Y92.451 E.00859
G1 X95.754 Y92.451 E.1219
G1 X95.754 Y92.828 E.01111
G1 X84.246 Y92.828 E.33919
G1 X84.246 Y92.451 E.01111
G1 X88.382 Y92.451 E.12189
G1 X88.139 Y92.289 E.00859
G1 X87.921 Y92.074 E.00903
G1 X84.246 Y92.074 E.10831
G1 X84.246 Y91.697 E.01111
G1 X87.599 Y91.697 E.09884
G1 X87.366 Y91.32 E.01307
G1 X84.246 Y91.32 E.09197
G1 X84.246 Y90.943 E.01111
G1 X87.209 Y90.943 E.08735
G1 X87.118 Y90.566 E.01143
G1 X84.246 Y90.566 E.08466
G1 X84.246 Y90.189 E.01111
G1 X87.061 Y90.189 E.08297
G1 X87.063 Y90 E.00556
G1 X87.061 Y89.812 E.00556
G1 X84.246 Y89.812 E.08297
G1 X84.246 Y89.434 E.01111
G1 X87.118 Y89.434 E.08466
G1 X87.209 Y89.057 E.01143
G1 X84.246 Y89.057 E.08735
G1 X84.246 Y88.68 E.01111
G1 X87.366 Y88.68 E.09196
G1 X87.599 Y88.303 E.01307
G1 X84.246 Y88.303 E.09884
G1 X84.246 Y87.926 E.01111
G1 X87.921 Y87.926 E.10831
G1 X88.139 Y87.711 E.00904
G1 X88.381 Y87.549 E.00859
G1 X84.076 Y87.549 E.12689
M204 S10000
G1 X83.677 Y87.123 F42000
; FEATURE: Support
G1 F3806
M204 S6000
G1 X81.73 Y87.123 E.05741
G1 X81.73 Y90 E.0848
G1 X83.508 Y90 E.05241
G1 X83.508 Y92.877 E.0848
G1 X81.56 Y92.877 E.05741
M204 S10000
G1 X81.447 Y95.733 F42000
G1 F3806
M204 S6000
G1 X83.847 Y95.733 E.07075
G1 X83.847 Y84.267 E.33792
G1 X81.39 Y84.267 E.07241
G1 X81.39 Y95.733 E.33792
M204 S10000
G1 X84.532 Y95.924 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X84.532 Y93.254 E.07869
G1 X84.91 Y93.254 E.01111
G1 X84.91 Y95.754 E.07369
G1 X85.287 Y95.754 E.01111
G1 X85.287 Y93.254 E.07369
G1 X85.664 Y93.254 E.01111
G1 X85.664 Y95.754 E.07369
G1 X86.041 Y95.754 E.01111
G1 X86.041 Y93.254 E.07369
G1 X86.418 Y93.254 E.01111
G1 X86.418 Y95.924 E.07869
M204 S10000
G1 X86.953 Y95.468 F42000
M106 S127.5
; FEATURE: Support interface
G1 F3806
M204 S6000
G1 X92.877 Y95.468 E.1746
G1 X92.877 Y95.09 E.01111
G1 X87.123 Y95.09 E.1696
G1 X87.123 Y94.713 E.01111
G1 X92.877 Y94.713 E.1696
G1 X92.877 Y94.336 E.01111
G1 X87.123 Y94.336 E.1696
G1 X87.123 Y93.959 E.01111
G1 X92.877 Y93.959 E.1696
G1 X92.877 Y93.582 E.01111
G1 X87.123 Y93.582 E.1696
G1 X87.123 Y93.205 E.01111
G1 X93.047 Y93.205 E.1746
M204 S10000
G1 X93.582 Y95.924 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X93.582 Y93.254 E.07869
G1 X93.959 Y93.254 E.01111
G1 X93.959 Y95.754 E.07369
G1 X94.336 Y95.754 E.01111
G1 X94.336 Y93.254 E.07369
G1 X94.713 Y93.254 E.01111
G1 X94.713 Y95.754 E.07369
G1 X95.09 Y95.754 E.01111
G1 X95.09 Y93.254 E.07369
G1 X95.467 Y93.254 E.01111
G1 X95.467 Y95.924 E.07869
M204 S10000
G1 X95.733 Y98.553 F42000
; FEATURE: Support
G1 F3806
M204 S6000
G1 X95.733 Y96.153 E.07075
G1 X84.267 Y96.153 E.33792
G1 X84.267 Y98.61 E.07242
G1 X95.733 Y98.61 E.33792
; WIPE_START
G1 F9000
G1 X94.733 Y98.61 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I1.13 J-.452 P1  F42000
G1 X90.133 Y87.098 Z1.3
G1 Z.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F3806
M204 S6000
G1 X90.298 Y87.104 E.0008
G1 X90.833 Y87.211 E.00265
G1 X91.361 Y87.427 E.00277
G1 X91.837 Y87.742 E.00277
G1 X92.242 Y88.144 E.00277
G1 X92.562 Y88.617 E.00277
G1 X92.782 Y89.143 E.00277
G1 X92.896 Y89.702 E.00277
G1 X92.898 Y90.273 E.00277
G1 X92.789 Y90.833 E.00277
G1 X92.573 Y91.361 E.00277
G1 X92.258 Y91.837 E.00277
G1 X91.856 Y92.243 E.00277
G1 X91.383 Y92.562 E.00277
G1 X90.857 Y92.782 E.00277
G1 X90.298 Y92.896 E.00277
G1 X89.727 Y92.898 E.00277
G1 X89.167 Y92.789 E.00277
G1 X88.639 Y92.573 E.00277
G1 X88.163 Y92.258 E.00277
G1 X87.757 Y91.856 E.00277
G1 X87.438 Y91.383 E.00277
G1 X87.218 Y90.857 E.00277
G1 X87.104 Y90.298 E.00277
G1 X87.102 Y89.727 E.00277
G1 X87.211 Y89.167 E.00277
G1 X87.427 Y88.639 E.00277
G1 X87.742 Y88.163 E.00277
G1 X88.144 Y87.757 E.00277
G1 X88.617 Y87.438 E.00277
G1 X89.143 Y87.218 E.00277
G1 X89.702 Y87.104 E.00277
G1 X90.133 Y87.098 E.00209
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.103 Y86.593 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F3806
M204 S6000
G1 X89.832 Y86.595 E.00131
G1 X89.5 Y86.627 E.00162
G1 X89.171 Y86.693 E.00162
G1 X88.852 Y86.79 E.00162
G1 X88.542 Y86.918 E.00162
G1 X88.247 Y87.075 E.00162
G1 X87.969 Y87.262 E.00163
G1 X87.71 Y87.474 E.00162
G1 X87.474 Y87.71 E.00162
G1 X87.262 Y87.969 E.00162
G1 X87.076 Y88.247 E.00162
G1 X86.918 Y88.542 E.00162
G1 X86.79 Y88.852 E.00163
G1 X86.693 Y89.171 E.00162
G1 X86.627 Y89.5 E.00162
G1 X86.595 Y89.832 E.00162
G1 X86.595 Y90.168 E.00163
G1 X86.627 Y90.5 E.00162
G1 X86.693 Y90.829 E.00162
G1 X86.79 Y91.148 E.00162
G1 X86.918 Y91.458 E.00162
G1 X87.075 Y91.753 E.00162
G1 X87.262 Y92.031 E.00163
G1 X87.474 Y92.289 E.00162
G1 X87.711 Y92.526 E.00163
G1 X87.969 Y92.738 E.00162
G1 X88.247 Y92.924 E.00162
G1 X88.542 Y93.082 E.00162
G1 X88.852 Y93.21 E.00162
G1 X89.171 Y93.307 E.00162
G1 X89.5 Y93.373 E.00162
G1 X89.832 Y93.405 E.00162
G1 X90.168 Y93.405 E.00163
G1 X90.5 Y93.373 E.00162
G1 X90.829 Y93.307 E.00162
G1 X91.149 Y93.21 E.00162
G1 X91.458 Y93.082 E.00162
G1 X91.752 Y92.925 E.00162
G1 X92.031 Y92.738 E.00163
G1 X92.289 Y92.526 E.00162
G1 X92.526 Y92.29 E.00162
G1 X92.738 Y92.031 E.00162
G1 X92.924 Y91.753 E.00162
G1 X93.082 Y91.458 E.00162
G1 X93.21 Y91.148 E.00163
G1 X93.307 Y90.829 E.00162
G1 X93.373 Y90.5 E.00162
G1 X93.405 Y90.168 E.00162
G1 X93.405 Y89.832 E.00163
G1 X93.373 Y89.5 E.00162
G1 X93.307 Y89.171 E.00162
G1 X93.21 Y88.852 E.00162
G1 X93.082 Y88.542 E.00162
G1 X92.924 Y88.247 E.00162
G1 X92.738 Y87.969 E.00162
G1 X92.527 Y87.711 E.00162
G1 X92.29 Y87.474 E.00163
G1 X92.031 Y87.262 E.00162
G1 X91.753 Y87.076 E.00162
G1 X91.458 Y86.918 E.00162
G1 X91.148 Y86.79 E.00163
G1 X90.829 Y86.693 E.00162
G1 X90.335 Y86.605 E.00243
G1 X90.103 Y86.593 E.00113
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 F4200
G1 X90.335 Y86.605 E-.08841
G1 X90.829 Y86.693 E-.19043
G1 X91.083 Y86.77 E-.10116
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I-1.138 J-.432 P1  F42000
G1 X90.568 Y88.128 Z1.3
G1 Z.9
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F600
M204 S6000
G1 X90.922 Y88.275 E.0122
G1 X91.241 Y88.488 E.0122
G1 X91.512 Y88.759 E.0122
G1 X91.725 Y89.078 E.0122
G1 X91.872 Y89.432 E.0122
G1 X91.947 Y89.808 E.0122
G1 X91.947 Y90.192 E.0122
G1 X91.872 Y90.568 E.0122
G1 X91.725 Y90.922 E.01221
G1 X91.512 Y91.241 E.0122
G1 X91.241 Y91.512 E.0122
G1 X90.922 Y91.725 E.0122
G1 X90.568 Y91.872 E.0122
G1 X90.192 Y91.947 E.0122
G1 X89.808 Y91.947 E.0122
G1 X89.432 Y91.872 E.0122
G1 X89.078 Y91.725 E.0122
G1 X88.759 Y91.512 E.0122
G1 X88.488 Y91.241 E.0122
G1 X88.275 Y90.922 E.0122
G1 X88.128 Y90.568 E.0122
G1 X88.053 Y90.192 E.0122
G1 X88.053 Y89.808 E.0122
G1 X88.128 Y89.432 E.0122
G1 X88.275 Y89.078 E.0122
G1 X88.488 Y88.759 E.0122
G1 X88.759 Y88.488 E.0122
G1 X89.078 Y88.275 E.0122
G1 X89.432 Y88.128 E.0122
G1 X89.808 Y88.053 E.0122
G1 X90.192 Y88.053 E.0122
G1 X90.509 Y88.116 E.0103
M106 S124.95
M106 S127.5
M204 S250
G1 X90.682 Y87.751 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.108 Y87.927 E.01358
G1 X91.491 Y88.183 E.01358
G1 X91.817 Y88.509 E.01358
G1 X92.073 Y88.892 E.01358
G1 X92.249 Y89.318 E.01358
G1 X92.339 Y89.77 E.01358
G1 X92.339 Y90.23 E.01358
G1 X92.249 Y90.682 E.01358
G1 X92.073 Y91.108 E.01358
G1 X91.817 Y91.491 E.01358
G1 X91.491 Y91.817 E.01358
G1 X91.108 Y92.073 E.01358
G1 X90.682 Y92.249 E.01358
G1 X90.23 Y92.339 E.01358
G1 X89.77 Y92.339 E.01358
G1 X89.318 Y92.249 E.01358
G1 X88.892 Y92.073 E.01358
G1 X88.509 Y91.817 E.01358
G1 X88.183 Y91.491 E.01358
G1 X87.927 Y91.108 E.01358
G1 X87.751 Y90.682 E.01358
G1 X87.661 Y90.23 E.01358
G1 X87.661 Y89.77 E.01358
G1 X87.751 Y89.318 E.01358
G1 X87.927 Y88.892 E.01358
G1 X88.183 Y88.509 E.01358
G1 X88.509 Y88.183 E.01358
G1 X88.892 Y87.927 E.01358
G1 X89.318 Y87.751 E.01358
G1 X89.77 Y87.661 E.01358
G1 X90.23 Y87.661 E.01358
G1 X90.623 Y87.739 E.01181
M106 S124.95
M106 S127.5
; WIPE_START
M204 S6000
G1 X91.108 Y87.927 E-.19749
G1 X91.491 Y88.183 E-.17508
G1 X91.505 Y88.197 E-.00743
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z1.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z1.3 F4000
            G39.3 S1
            G0 Z1.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X91.602 Y90.793 F42000
G1 Z.9
G1 E.4 F1800
; FEATURE: Bridge
; LINE_WIDTH: 0.4434
G1 F3000
M204 S6000
G1 X91.602 Y89.782 E.03164
G1 X91.552 Y89.529 E.00806
G1 X91.43 Y89.236 E.00995
G1 X91.201 Y88.919 E.01222
G1 X91.201 Y91.081 E.06765
G1 X91.029 Y91.253 E.00765
G1 X90.801 Y91.406 E.00857
G1 X90.801 Y88.594 E.08799
G1 X90.764 Y88.57 E.00138
G1 X90.4 Y88.434 E.01215
G1 X90.4 Y91.566 E.09801
G1 X90.159 Y91.614 E.00771
G1 X90 Y91.614 E.00498
G1 X90 Y88.386 E.10102
G1 X89.841 Y88.386 E.00497
G1 X89.599 Y88.434 E.00771
G1 X89.599 Y91.566 E.09801
G1 X89.529 Y91.552 E.00224
G1 X89.199 Y91.405 E.01131
G1 X89.199 Y88.595 E.08799
M73 P43 R9
G1 X88.971 Y88.747 E.00857
G1 X88.799 Y88.919 E.00765
G1 X88.799 Y91.081 E.06764
G1 X88.57 Y90.764 E.01221
G1 X88.448 Y90.471 E.00995
G1 X88.398 Y90.218 E.00808
G1 X88.398 Y89.208 E.03162
M106 S124.95
; CHANGE_LAYER
; Z_HEIGHT: 1.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F3000
G1 X88.398 Y90.208 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 6/43
; update layer progress
M73 L6
M991 S0 P5 ;notify layer change
; OBJECT_ID: 346
M204 S10000
G17
G3 Z1.3 I-.936 J-.777 P1  F42000
G1 X86.041 Y93.047 Z1.3
G1 Z1.1
G1 E.4 F1800
; FEATURE: Support interface
; LINE_WIDTH: 0.42
G1 F4650
M204 S6000
G1 X86.041 Y87.123 E.1746
G1 X85.664 Y87.123 E.01111
G1 X85.664 Y92.877 E.1696
G1 X85.287 Y92.877 E.01111
G1 X85.287 Y87.123 E.1696
G1 X84.91 Y87.123 E.01111
G1 X84.91 Y92.877 E.1696
G1 X84.532 Y92.877 E.01111
G1 X84.532 Y86.953 E.1746
M204 S10000
G1 X84.532 Y86.916 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X84.532 Y84.246 E.07869
G1 X84.91 Y84.246 E.01111
G1 X84.91 Y86.746 E.07369
G1 X85.287 Y86.746 E.01111
G1 X85.287 Y84.246 E.07369
G1 X85.664 Y84.246 E.01111
G1 X85.664 Y86.746 E.07369
G1 X86.041 Y86.746 E.01111
G1 X86.041 Y84.246 E.07369
G1 X86.418 Y84.246 E.01111
G1 X86.418 Y86.916 E.07869
M204 S10000
G1 X83.677 Y87.123 F42000
; FEATURE: Support
G1 F4650
M204 S6000
G1 X81.73 Y87.123 E.05741
G1 X81.73 Y90 E.0848
G1 X83.508 Y90 E.05241
G1 X83.508 Y92.877 E.0848
G1 X81.56 Y92.877 E.05741
M204 S10000
G1 X81.447 Y95.733 F42000
G1 F4650
M204 S6000
G1 X83.847 Y95.733 E.07075
G1 X83.847 Y84.267 E.33792
G1 X81.39 Y84.267 E.07241
G1 X81.39 Y95.733 E.33792
M204 S10000
G1 X84.532 Y95.924 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X84.532 Y93.254 E.07869
G1 X84.91 Y93.254 E.01111
G1 X84.91 Y95.754 E.07369
G1 X85.287 Y95.754 E.01111
G1 X85.287 Y93.254 E.07369
G1 X85.664 Y93.254 E.01111
G1 X85.664 Y95.754 E.07369
G1 X86.041 Y95.754 E.01111
G1 X86.041 Y93.254 E.07369
G1 X86.418 Y93.254 E.01111
G1 X86.418 Y95.924 E.07869
M204 S10000
G1 X93.582 Y95.924 F42000
G1 F3000
M204 S6000
G1 X93.582 Y93.254 E.07869
G1 X93.959 Y93.254 E.01111
G1 X93.959 Y95.754 E.07369
G1 X94.336 Y95.754 E.01111
G1 X94.336 Y93.254 E.07369
G1 X94.713 Y93.254 E.01111
G1 X94.713 Y95.754 E.07369
G1 X95.09 Y95.754 E.01111
G1 X95.09 Y93.254 E.07369
G1 X95.467 Y93.254 E.01111
G1 X95.467 Y95.924 E.07869
M204 S10000
G1 X95.733 Y98.553 F42000
; FEATURE: Support
G1 F4650
M204 S6000
G1 X95.733 Y96.153 E.07075
G1 X84.267 Y96.153 E.33792
G1 X84.267 Y98.61 E.07242
G1 X95.733 Y98.61 E.33792
M204 S10000
G1 X95.467 Y86.916 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X95.467 Y84.246 E.07869
G1 X95.09 Y84.246 E.01111
G1 X95.09 Y86.746 E.07369
G1 X94.713 Y86.746 E.01111
G1 X94.713 Y84.246 E.07369
G1 X94.336 Y84.246 E.01111
G1 X94.336 Y86.746 E.07369
G1 X93.959 Y86.746 E.01111
G1 X93.959 Y84.246 E.07369
G1 X93.582 Y84.246 E.01111
G1 X93.582 Y86.916 E.07869
M204 S10000
G1 X93.205 Y86.953 F42000
M106 S127.5
; FEATURE: Support interface
G1 F4650
M204 S6000
G1 X93.205 Y88.058 E.03256
G1 X93.011 Y87.769 E.01027
G1 X92.828 Y87.54 E.00863
G1 X92.828 Y84.246 E.0971
G1 X92.451 Y84.246 E.01111
G1 X92.451 Y87.164 E.08602
G1 X92.074 Y86.883 E.01386
G1 X92.074 Y84.246 E.07773
G1 X91.697 Y84.246 E.01111
G1 X91.697 Y86.658 E.07109
G1 X91.32 Y86.494 E.01211
G1 X91.32 Y84.246 E.06627
G1 X90.943 Y84.246 E.01111
G1 X90.943 Y86.373 E.06269
G1 X90.566 Y86.295 E.01135
G1 X90.566 Y84.246 E.06041
G1 X90.188 Y84.246 E.01111
G1 X90.188 Y86.259 E.05933
G1 X89.811 Y86.257 E.01111
G1 X89.811 Y84.246 E.05928
G1 X89.434 Y84.246 E.01111
G1 X89.434 Y86.295 E.06041
G1 X89.057 Y86.373 E.01135
G1 X89.057 Y84.246 E.06269
G1 X88.68 Y84.246 E.01111
G1 X88.68 Y86.494 E.06628
G1 X88.303 Y86.658 E.01211
G1 X88.303 Y84.246 E.07109
G1 X87.926 Y84.246 E.01111
G1 X87.926 Y86.883 E.07773
G1 X87.549 Y87.164 E.01386
G1 X87.549 Y84.246 E.08602
G1 X87.172 Y84.246 E.01111
G1 X87.172 Y87.54 E.09711
G1 X86.989 Y87.768 E.00862
G1 X86.795 Y88.058 E.01027
G1 X86.795 Y87.123 E.02757
G1 X86.418 Y87.123 E.01111
G1 X86.418 Y92.877 E.1696
G1 X86.795 Y92.877 E.01111
G1 X86.795 Y91.942 E.02757
G1 X86.989 Y92.231 E.01027
G1 X87.172 Y92.46 E.00863
G1 X87.172 Y95.754 E.09711
G1 X87.549 Y95.754 E.01111
G1 X87.549 Y92.836 E.08602
G1 X87.926 Y93.117 E.01386
G1 X87.926 Y95.754 E.07773
G1 X88.303 Y95.754 E.01111
G1 X88.303 Y93.342 E.07109
G1 X88.68 Y93.506 E.01211
G1 X88.68 Y95.754 E.06628
G1 X89.057 Y95.754 E.01111
G1 X89.057 Y93.627 E.06269
G1 X89.434 Y93.705 E.01135
G1 X89.434 Y95.754 E.06041
G1 X89.811 Y95.754 E.01111
G1 X89.811 Y93.743 E.05928
G1 X90.188 Y93.743 E.01111
G1 X90.188 Y95.754 E.05928
G1 X90.566 Y95.754 E.01111
G1 X90.566 Y93.705 E.06041
G1 X90.943 Y93.627 E.01135
G1 X90.943 Y95.754 E.06269
G1 X91.32 Y95.754 E.01111
G1 X91.32 Y93.506 E.06627
G1 X91.697 Y93.342 E.01211
G1 X91.697 Y95.754 E.07109
G1 X92.074 Y95.754 E.01111
G1 X92.074 Y93.117 E.07773
G1 X92.451 Y92.836 E.01386
G1 X92.451 Y95.754 E.08602
G1 X92.828 Y95.754 E.01111
G1 X92.828 Y92.46 E.0971
G1 X93.011 Y92.232 E.00862
G1 X93.205 Y91.942 E.01027
G1 X93.205 Y92.877 E.02756
G1 X93.582 Y92.877 E.01111
G1 X93.582 Y87.123 E.1696
G1 X93.959 Y87.123 E.01111
G1 X93.959 Y92.877 E.1696
G1 X94.336 Y92.877 E.01111
G1 X94.336 Y87.123 E.1696
G1 X94.713 Y87.123 E.01111
G1 X94.713 Y92.877 E.1696
G1 X95.09 Y92.877 E.01111
G1 X95.09 Y87.123 E.1696
G1 X95.467 Y87.123 E.01111
G1 X95.467 Y93.047 E.1746
M204 S10000
G1 X96.323 Y92.877 F42000
; FEATURE: Support
G1 F4650
M204 S6000
G1 X98.27 Y92.877 E.05741
G1 X98.27 Y90 E.0848
G1 X96.492 Y90 E.05241
G1 X96.492 Y87.123 E.0848
G1 X98.44 Y87.123 E.05741
M204 S10000
G1 X98.61 Y84.267 F42000
G1 F4650
M204 S6000
G1 X98.61 Y95.733 E.33792
G1 X96.153 Y95.733 E.07242
G1 X96.153 Y84.267 E.33792
G1 X98.553 Y84.267 E.07075
; WIPE_START
G1 F9000
G1 X97.553 Y84.267 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I.274 J-1.186 P1  F42000
G1 X95.733 Y83.847 Z1.5
G1 Z1.1
G1 E.4 F1800
G1 F4650
M204 S6000
G1 X84.267 Y83.847 E.33792
G1 X84.267 Y81.39 E.07241
G1 X95.733 Y81.39 E.33792
G1 X95.733 Y83.791 E.07075
; WIPE_START
G1 F9000
G1 X95.733 Y82.791 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I-.643 J-1.033 P1  F42000
G1 X90.106 Y86.293 Z1.5
G1 Z1.1
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X90.37 Y86.307 E.00128
G1 X90.9 Y86.401 E.00261
G1 X91.251 Y86.508 E.00178
G1 X91.585 Y86.646 E.00175
G1 X91.908 Y86.819 E.00178
G1 X92.209 Y87.02 E.00175
G1 X92.492 Y87.252 E.00178
G1 X92.748 Y87.508 E.00175
G1 X92.98 Y87.791 E.00178
G1 X93.181 Y88.092 E.00175
G1 X93.354 Y88.415 E.00178
G1 X93.492 Y88.749 E.00175
G1 X93.599 Y89.1 E.00178
G1 X93.669 Y89.454 E.00175
G1 X93.705 Y89.819 E.00178
G1 X93.705 Y90.181 E.00175
G1 X93.669 Y90.546 E.00178
G1 X93.599 Y90.9 E.00175
G1 X93.492 Y91.251 E.00178
G1 X93.354 Y91.585 E.00175
G1 X93.181 Y91.908 E.00178
G1 X92.98 Y92.209 E.00175
G1 X92.748 Y92.492 E.00178
G1 X92.492 Y92.748 E.00175
G1 X92.209 Y92.98 E.00178
G1 X91.908 Y93.181 E.00175
G1 X91.585 Y93.354 E.00178
G1 X91.251 Y93.492 E.00175
G1 X90.9 Y93.599 E.00178
G1 X90.546 Y93.669 E.00175
G1 X90.181 Y93.705 E.00178
G1 X89.819 Y93.705 E.00175
G1 X89.454 Y93.669 E.00178
G1 X89.1 Y93.599 E.00175
G1 X88.749 Y93.492 E.00178
G1 X88.415 Y93.354 E.00175
G1 X88.092 Y93.181 E.00178
G1 X87.791 Y92.98 E.00175
G1 X87.508 Y92.748 E.00178
G1 X87.252 Y92.492 E.00175
G1 X87.02 Y92.209 E.00178
G1 X86.819 Y91.908 E.00175
G1 X86.646 Y91.585 E.00178
G1 X86.508 Y91.251 E.00175
G1 X86.401 Y90.9 E.00178
G1 X86.331 Y90.546 E.00175
G1 X86.295 Y90.181 E.00178
G1 X86.295 Y89.819 E.00175
G1 X86.331 Y89.454 E.00178
G1 X86.401 Y89.1 E.00175
G1 X86.508 Y88.749 E.00178
G1 X86.646 Y88.415 E.00175
G1 X86.819 Y88.092 E.00178
G1 X87.02 Y87.791 E.00175
G1 X87.252 Y87.508 E.00178
G1 X87.508 Y87.252 E.00175
G1 X87.791 Y87.02 E.00178
G1 X88.092 Y86.819 E.00175
G1 X88.415 Y86.646 E.00178
G1 X88.749 Y86.508 E.00175
G1 X89.1 Y86.401 E.00178
G1 X89.454 Y86.331 E.00175
G1 X89.819 Y86.295 E.00178
G1 X90.106 Y86.293 E.00139
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.102 Y85.845 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X90.608 Y85.887 E.00246
G1 X91.009 Y85.966 E.00198
G1 X91.399 Y86.085 E.00198
G1 X91.776 Y86.24 E.00197
G1 X92.137 Y86.433 E.00198
G1 X92.478 Y86.661 E.00199
G1 X92.791 Y86.918 E.00197
G1 X93.079 Y87.206 E.00197
G1 X93.339 Y87.522 E.00198
G1 X93.567 Y87.864 E.00199
G1 X93.758 Y88.221 E.00196
G1 X93.915 Y88.601 E.00199
G1 X94.033 Y88.988 E.00196
G1 X94.113 Y89.391 E.00199
G1 X94.153 Y89.795 E.00197
G1 X94.153 Y90.202 E.00197
G1 X94.113 Y90.608 E.00198
G1 X94.034 Y91.009 E.00198
G1 X93.915 Y91.399 E.00198
G1 X93.758 Y91.779 E.00199
G1 X93.567 Y92.136 E.00196
G1 X93.339 Y92.478 E.00199
G1 X93.082 Y92.791 E.00196
G1 X92.791 Y93.082 E.00199
G1 X92.478 Y93.339 E.00197
G1 X92.136 Y93.567 E.00199
G1 X91.779 Y93.758 E.00196
G1 X91.399 Y93.915 E.00199
G1 X91.011 Y94.033 E.00196
G1 X90.608 Y94.113 E.00199
G1 X90.205 Y94.153 E.00196
G1 X89.798 Y94.153 E.00197
G1 X89.392 Y94.113 E.00198
G1 X88.991 Y94.034 E.00198
G1 X88.601 Y93.915 E.00198
G1 X88.221 Y93.758 E.00199
G1 X87.864 Y93.567 E.00196
G1 X87.522 Y93.339 E.00199
G1 X87.209 Y93.082 E.00196
G1 X86.918 Y92.791 E.00199
G1 X86.661 Y92.478 E.00197
G1 X86.433 Y92.136 E.00199
G1 X86.242 Y91.779 E.00196
G1 X86.085 Y91.4 E.00199
G1 X85.967 Y91.012 E.00196
G1 X85.887 Y90.608 E.002
G1 X85.847 Y90.205 E.00196
G1 X85.847 Y89.798 E.00197
G1 X85.887 Y89.391 E.00198
G1 X85.966 Y88.991 E.00198
G1 X86.085 Y88.601 E.00198
G1 X86.242 Y88.221 E.00199
G1 X86.433 Y87.864 E.00196
G1 X86.661 Y87.522 E.00199
G1 X86.918 Y87.209 E.00196
G1 X87.206 Y86.921 E.00198
G1 X87.522 Y86.661 E.00198
G1 X87.864 Y86.433 E.00199
G1 X88.221 Y86.242 E.00196
G1 X88.597 Y86.086 E.00198
G1 X88.989 Y85.967 E.00198
G1 X89.387 Y85.888 E.00197
M73 P44 R9
G1 X89.795 Y85.847 E.00198
G1 X90.102 Y85.845 E.00149
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 1
; WIPE_START
G1 X89.795 Y85.847 E-.11665
G1 X89.387 Y85.888 E-.15553
G1 X89.109 Y85.943 E-.10782
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I-.782 J.932 P1  F42000
G1 X90.8 Y87.362 Z1.5
G1 Z1.1
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F600
M204 S6000
G1 X91.3 Y87.569 E.01719
G1 X91.749 Y87.869 E.0172
G1 X92.131 Y88.251 E.01719
G1 X92.431 Y88.7 E.0172
G1 X92.638 Y89.2 E.0172
G1 X92.744 Y89.73 E.0172
G1 X92.744 Y90.27 E.0172
G1 X92.638 Y90.8 E.01719
G1 X92.431 Y91.3 E.0172
G1 X92.131 Y91.749 E.0172
G1 X91.749 Y92.131 E.0172
G1 X91.3 Y92.431 E.01719
G1 X90.8 Y92.638 E.0172
G1 X90.27 Y92.744 E.0172
G1 X89.73 Y92.744 E.0172
G1 X89.2 Y92.638 E.0172
G1 X88.7 Y92.431 E.0172
G1 X88.251 Y92.131 E.0172
G1 X87.869 Y91.749 E.0172
G1 X87.569 Y91.3 E.01719
G1 X87.362 Y90.8 E.0172
G1 X87.256 Y90.27 E.0172
G1 X87.256 Y89.73 E.0172
G1 X87.362 Y89.2 E.01719
G1 X87.569 Y88.7 E.0172
G1 X87.869 Y88.251 E.0172
G1 X88.251 Y87.869 E.0172
G1 X88.7 Y87.569 E.0172
G1 X89.2 Y87.362 E.01719
G1 X89.73 Y87.256 E.0172
G1 X90.27 Y87.256 E.0172
G1 X90.741 Y87.35 E.01529
M106 S124.95
M106 S127.5
; COOLING_NODE: 1
M204 S250
G1 X90.88 Y86.978 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X90.915 Y86.985 E.00104
G1 X91.485 Y87.221 E.0182
G1 X91.999 Y87.564 E.01821
G1 X92.436 Y88.001 E.0182
G1 X92.779 Y88.515 E.01821
G1 X93.015 Y89.085 E.01821
G1 X93.136 Y89.691 E.01821
G1 X93.136 Y90.309 E.01821
G1 X93.015 Y90.915 E.0182
G1 X92.779 Y91.485 E.01821
G1 X92.436 Y91.999 E.0182
G1 X91.999 Y92.436 E.01821
G1 X91.485 Y92.779 E.0182
G1 X90.915 Y93.015 E.01821
G1 X90.309 Y93.136 E.01821
G1 X89.691 Y93.136 E.0182
G1 X89.085 Y93.015 E.0182
G1 X88.515 Y92.779 E.01821
G1 X88.001 Y92.436 E.0182
G1 X87.564 Y91.999 E.01821
G1 X87.221 Y91.485 E.0182
G1 X86.985 Y90.915 E.01821
G1 X86.864 Y90.309 E.01821
G1 X86.864 Y89.691 E.01821
G1 X86.985 Y89.085 E.0182
G1 X87.221 Y88.515 E.01821
G1 X87.564 Y88.001 E.01821
G1 X88.001 Y87.564 E.01821
G1 X88.515 Y87.221 E.01821
G1 X89.085 Y86.985 E.0182
G1 X89.691 Y86.864 E.01821
G1 X90.309 Y86.864 E.0182
G1 X90.821 Y86.966 E.0154
M106 S124.95
M106 S127.5
; WIPE_START
M204 S6000
G1 X90.915 Y86.985 E-.03618
G1 X91.485 Y87.221 E-.2347
G1 X91.724 Y87.381 E-.10912
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z1.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z1.5 F4000
            G39.3 S1
            G0 Z1.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.8 Y89.157 F42000
G1 Z1.1
G1 E.4 F1800
; FEATURE: Bridge
; LINE_WIDTH: 0.44277
G1 F3000
M204 S6000
G1 X90.8 Y87.722 E.04485
G1 X90.703 Y87.682 E.00326
G1 X90.4 Y87.622 E.00967
G1 X90.4 Y88.722 E.03439
G1 X90.131 Y88.666 E.00858
G1 X90 Y88.669 E.0041
G1 X90 Y87.589 E.03375
G1 X89.763 Y87.589 E.00742
G1 X89.6 Y87.622 E.00518
G1 X89.6 Y88.722 E.03439
G1 X89.368 Y88.818 E.00784
G1 X89.2 Y88.932 E.00635
G1 X89.2 Y87.722 E.03781
G1 X88.8 Y87.902 E.01371
G1 X88.8 Y92.098 E.13111
G1 X88.463 Y91.872 E.01267
G1 X88.401 Y91.81 E.00278
G1 X88.401 Y88.19 E.11311
G1 X88.128 Y88.463 E.01207
G1 X88.001 Y88.653 E.00713
G1 X88.001 Y91.347 E.08418
G1 X87.864 Y91.142 E.0077
G1 X87.682 Y90.703 E.01484
G1 X87.601 Y90.295 E.01301
G1 X87.601 Y89.049 E.03893
M106 S124.95
M106 S127.5
; WIPE_START
G1 X87.601 Y90.049 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I.052 J1.216 P1  F42000
G1 X90.935 Y89.908 Z1.5
G1 Z1.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.41999
G1 F4650
M204 S6000
G1 X90.917 Y89.722 E.0055
G1 X90.741 Y89.392 E.01102
G1 X90.452 Y89.155 E.01102
G1 X90.094 Y89.046 E.01102
G1 X89.722 Y89.083 E.01101
G1 X89.392 Y89.259 E.01102
G1 X89.155 Y89.548 E.01101
G1 X89.047 Y89.906 E.01103
G1 X89.083 Y90.278 E.01101
G1 X89.259 Y90.608 E.01102
G1 X89.548 Y90.845 E.01101
G1 X89.906 Y90.953 E.01103
G1 X90.278 Y90.917 E.01101
G1 X90.608 Y90.741 E.01102
G1 X90.845 Y90.452 E.01102
G1 X90.954 Y90.094 E.01103
G1 X90.941 Y89.968 E.00374
M204 S10000
G1 X90.119 Y89.855 F42000
; LINE_WIDTH: 0.41695
G1 F4650
M204 S6000
G1 X89.982 Y89.814 E.00419
G1 X89.855 Y89.881 E.00418
G1 X89.814 Y90.018 E.00419
G1 X89.881 Y90.145 E.00418
G1 X90.018 Y90.186 E.00419
G1 X90.145 Y90.119 E.00418
G1 X90.186 Y89.982 E.00419
G1 X90.147 Y89.908 E.00243
M204 S10000
G1 X89.462 Y89.837 F42000
; LINE_WIDTH: 0.41999
G1 F4650
M204 S6000
G1 X89.44 Y90.055 E.00647
G1 X89.504 Y90.265 E.00647
G1 X89.643 Y90.435 E.00647
G1 X89.837 Y90.538 E.00648
G1 X90.055 Y90.56 E.00647
G1 X90.265 Y90.496 E.00647
G1 X90.435 Y90.357 E.00647
G1 X90.538 Y90.163 E.00648
G1 X90.56 Y89.945 E.00646
G1 X90.496 Y89.735 E.00646
G1 X90.357 Y89.565 E.00647
G1 X90.163 Y89.462 E.00648
G1 X89.945 Y89.44 E.00646
G1 X89.735 Y89.504 E.00647
G1 X89.565 Y89.643 E.00647
G1 X89.49 Y89.784 E.00471
; WIPE_START
G1 F10342.907
G1 X89.565 Y89.643 E-.06073
G1 X89.735 Y89.504 E-.08339
G1 X89.945 Y89.44 E-.08341
G1 X90.163 Y89.462 E-.08332
G1 X90.324 Y89.547 E-.06915
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.5 I-.919 J-.797 P1  F42000
G1 X89.2 Y90.843 Z1.5
G1 Z1.1
G1 E.4 F1800
; FEATURE: Bridge
; LINE_WIDTH: 0.44277
G1 F3000
M204 S6000
G1 X89.2 Y92.278 E.04485
G1 X89.297 Y92.318 E.00327
G1 X89.6 Y92.378 E.00966
G1 X89.6 Y91.278 E.03439
G1 X89.869 Y91.334 E.00857
G1 X90 Y91.331 E.00411
G1 X90 Y92.411 E.03375
G1 X90.237 Y92.411 E.00742
G1 X90.4 Y92.378 E.00518
G1 X90.4 Y91.278 E.03439
G1 X90.632 Y91.182 E.00785
G1 X90.8 Y91.068 E.00634
G1 X90.8 Y92.278 E.03782
G1 X91.2 Y92.098 E.01371
G1 X91.2 Y87.902 E.13111
G1 X91.537 Y88.128 E.01267
G1 X91.599 Y88.19 E.00277
G1 X91.599 Y91.81 E.11311
G1 X91.872 Y91.537 E.01207
G1 X91.999 Y91.347 E.00713
G1 X91.999 Y88.653 E.08418
G1 X92.136 Y88.858 E.0077
G1 X92.318 Y89.297 E.01484
G1 X92.399 Y89.705 E.01301
G1 X92.399 Y90.951 E.03892
M106 S124.95
; CHANGE_LAYER
; Z_HEIGHT: 1.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F3000
G1 X92.399 Y89.951 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 7/43
; update layer progress
M73 L7
M991 S0 P6 ;notify layer change
; OBJECT_ID: 346
M204 S10000
G17
G3 Z1.5 I-.728 J.975 P1  F42000
G1 X96.323 Y92.877 Z1.5
G1 Z1.3
G1 E.4 F1800
; FEATURE: Support
; LINE_WIDTH: 0.42
G1 F3861
M204 S6000
G1 X98.27 Y92.877 E.05741
G1 X98.27 Y90 E.0848
G1 X96.492 Y90 E.05241
G1 X96.492 Y87.123 E.0848
G1 X98.44 Y87.123 E.05741
M204 S10000
G1 X98.553 Y84.267 F42000
G1 F3861
M204 S6000
G1 X96.153 Y84.267 E.07075
G1 X96.153 Y95.733 E.33792
G1 X98.61 Y95.733 E.07242
G1 X98.61 Y84.267 E.33792
; WIPE_START
G1 F9000
G1 X98.61 Y85.267 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I.556 J-1.083 P1  F42000
G1 X95.733 Y83.791 Z1.7
G1 Z1.3
G1 E.4 F1800
G1 F3861
M204 S6000
G1 X95.733 Y81.39 E.07075
G1 X84.267 Y81.39 E.33792
G1 X84.267 Y83.847 E.07241
G1 X95.733 Y83.847 E.33792
M204 S10000
G1 X95.924 Y84.533 F42000
; FEATURE: Support interface
G1 F3861
M204 S6000
G1 X84.246 Y84.533 E.34419
G1 X84.246 Y84.91 E.01111
G1 X95.754 Y84.91 E.33919
G1 X95.754 Y85.287 E.01111
G1 X84.246 Y85.287 E.33919
G1 X84.246 Y85.664 E.01111
G1 X95.754 Y85.664 E.33919
G1 X95.754 Y86.041 E.01111
G1 X92.132 Y86.041 E.10676
G1 X92.493 Y86.259 E.01244
G1 X92.716 Y86.418 E.00807
G1 X95.754 Y86.418 E.08954
G1 X95.754 Y86.795 E.01111
G1 X93.152 Y86.795 E.07671
G1 X93.496 Y87.172 E.01504
G1 X95.754 Y87.172 E.06657
G1 X95.754 Y87.549 E.01111
G1 X93.768 Y87.549 E.05855
G1 X93.989 Y87.926 E.01288
G1 X95.754 Y87.926 E.05203
G1 X95.754 Y88.303 E.01111
G1 X94.163 Y88.303 E.04691
G1 X94.299 Y88.68 E.01181
G1 X95.754 Y88.68 E.0429
G1 X95.754 Y89.057 E.01111
G1 X94.395 Y89.057 E.04007
G1 X94.46 Y89.434 E.01128
G1 X95.754 Y89.434 E.03816
G1 X95.754 Y89.812 E.01111
G1 X94.492 Y89.812 E.03721
G1 X94.492 Y90.189 E.01111
G1 X95.754 Y90.189 E.03721
G1 X95.754 Y90.566 E.01111
G1 X94.46 Y90.566 E.03816
G1 X94.395 Y90.943 E.01128
G1 X95.754 Y90.943 E.04007
G1 X95.754 Y91.32 E.01111
G1 X94.299 Y91.32 E.04291
G1 X94.163 Y91.697 E.01181
G1 X95.754 Y91.697 E.04691
G1 X95.754 Y92.074 E.01111
G1 X93.989 Y92.074 E.05203
G1 X93.768 Y92.451 E.01288
G1 X95.754 Y92.451 E.05855
G1 X95.754 Y92.828 E.01111
G1 X93.495 Y92.828 E.06657
G1 X93.152 Y93.205 E.01504
G1 X95.754 Y93.205 E.07671
G1 X95.754 Y93.582 E.01111
G1 X92.716 Y93.582 E.08954
G1 X92.493 Y93.741 E.00806
G1 X92.132 Y93.959 E.01245
G1 X95.754 Y93.959 E.10677
G1 X95.754 Y94.336 E.01111
G1 X84.246 Y94.336 E.33919
G1 X84.246 Y93.959 E.01111
G1 X87.868 Y93.959 E.10677
G1 X87.507 Y93.741 E.01245
G1 X87.284 Y93.582 E.00807
G1 X84.246 Y93.582 E.08954
G1 X84.246 Y93.205 E.01111
G1 X86.848 Y93.205 E.07671
G1 X86.505 Y92.828 E.01504
G1 X84.246 Y92.828 E.06657
G1 X84.246 Y92.451 E.01111
G1 X86.232 Y92.451 E.05855
G1 X86.011 Y92.074 E.01288
G1 X84.246 Y92.074 E.05203
G1 X84.246 Y91.697 E.01111
G1 X85.837 Y91.697 E.04691
G1 X85.701 Y91.32 E.01181
G1 X84.246 Y91.32 E.0429
G1 X84.246 Y90.943 E.01111
G1 X85.605 Y90.943 E.04007
G1 X85.54 Y90.566 E.01128
G1 X84.246 Y90.566 E.03816
G1 X84.246 Y90.189 E.01111
G1 X85.508 Y90.189 E.03721
G1 X85.508 Y89.812 E.01111
G1 X84.246 Y89.812 E.03721
G1 X84.246 Y89.434 E.01111
G1 X85.54 Y89.434 E.03816
M73 P45 R9
G1 X85.605 Y89.057 E.01128
G1 X84.246 Y89.057 E.04007
G1 X84.246 Y88.68 E.01111
G1 X85.701 Y88.68 E.0429
G1 X85.837 Y88.303 E.01181
G1 X84.246 Y88.303 E.04691
G1 X84.246 Y87.926 E.01111
G1 X86.011 Y87.926 E.05203
G1 X86.232 Y87.549 E.01288
G1 X84.246 Y87.549 E.05855
G1 X84.246 Y87.172 E.01111
G1 X86.504 Y87.172 E.06657
G1 X86.848 Y86.795 E.01504
G1 X84.246 Y86.795 E.07671
G1 X84.246 Y86.418 E.01111
G1 X87.284 Y86.418 E.08954
G1 X87.507 Y86.259 E.00807
G1 X87.868 Y86.041 E.01245
G1 X84.076 Y86.041 E.11177
; WIPE_START
G1 F4800
G1 X85.076 Y86.041 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I-.745 J-.962 P1  F42000
G1 X83.677 Y87.123 Z1.7
G1 Z1.3
G1 E.4 F1800
; FEATURE: Support
G1 F3861
M204 S6000
G1 X81.73 Y87.123 E.05741
G1 X81.73 Y90 E.0848
G1 X83.508 Y90 E.05241
G1 X83.508 Y92.877 E.0848
G1 X81.56 Y92.877 E.05741
M204 S10000
G1 X81.39 Y95.733 F42000
G1 F3861
M204 S6000
G1 X81.39 Y84.267 E.33792
G1 X83.847 Y84.267 E.07241
G1 X83.847 Y95.733 E.33792
G1 X81.447 Y95.733 E.07075
; WIPE_START
G1 F9000
G1 X82.447 Y95.733 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I.196 J1.201 P1  F42000
G1 X84.076 Y95.468 Z1.7
G1 Z1.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Support interface
G1 F3861
M204 S6000
G1 X95.754 Y95.468 E.34419
G1 X95.754 Y95.09 E.01111
G1 X84.246 Y95.09 E.33919
G1 X84.246 Y94.713 E.01111
G1 X95.924 Y94.713 E.34419
; WIPE_START
G1 F4800
G1 X94.924 Y94.713 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I-1.191 J.251 P1  F42000
G1 X95.733 Y98.553 Z1.7
G1 Z1.3
G1 E.4 F1800
; FEATURE: Support
G1 F3861
M204 S6000
G1 X95.733 Y96.153 E.07075
G1 X84.267 Y96.153 E.33792
G1 X84.267 Y98.61 E.07242
G1 X95.733 Y98.61 E.33792
; WIPE_START
G1 F9000
G1 X94.733 Y98.61 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I1.147 J-.406 P1  F42000
G1 X90.109 Y85.545 Z1.7
G1 Z1.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F3861
M204 S6000
G1 X90.65 Y85.59 E.00263
G1 X91.077 Y85.674 E.00211
G1 X91.498 Y85.801 E.00213
G1 X91.9 Y85.967 E.00211
G1 X92.288 Y86.174 E.00213
G1 X92.659 Y86.422 E.00216
G1 X92.99 Y86.694 E.00208
G1 X93.298 Y87.001 E.00211
G1 X93.578 Y87.341 E.00213
G1 X93.826 Y87.712 E.00216
G1 X94.028 Y88.09 E.00208
G1 X94.199 Y88.502 E.00216
G1 X94.323 Y88.913 E.00208
G1 X94.41 Y89.35 E.00216
G1 X94.452 Y89.777 E.00208
G1 X94.453 Y90.212 E.00211
G1 X94.41 Y90.65 E.00213
G1 X94.323 Y91.087 E.00216
G1 X94.199 Y91.498 E.00208
G1 X94.028 Y91.91 E.00216
G1 X93.826 Y92.288 E.00208
G1 X93.578 Y92.659 E.00216
G1 X93.306 Y92.991 E.00208
G1 X92.99 Y93.306 E.00216
G1 X92.659 Y93.578 E.00208
G1 X92.288 Y93.826 E.00216
G1 X91.91 Y94.028 E.00208
G1 X91.498 Y94.199 E.00216
G1 X91.087 Y94.323 E.00208
G1 X90.65 Y94.41 E.00216
G1 X90.223 Y94.452 E.00208
G1 X89.788 Y94.453 E.00211
G1 X89.35 Y94.41 E.00213
G1 X88.913 Y94.323 E.00216
G1 X88.502 Y94.199 E.00208
G1 X88.09 Y94.028 E.00216
G1 X87.712 Y93.826 E.00208
G1 X87.341 Y93.578 E.00216
G1 X87.01 Y93.306 E.00208
G1 X86.694 Y92.99 E.00216
G1 X86.422 Y92.659 E.00208
G1 X86.174 Y92.288 E.00216
G1 X85.972 Y91.91 E.00208
G1 X85.801 Y91.498 E.00216
G1 X85.677 Y91.087 E.00208
G1 X85.59 Y90.65 E.00216
G1 X85.548 Y90.223 E.00208
G1 X85.547 Y89.788 E.00211
G1 X85.59 Y89.35 E.00213
G1 X85.677 Y88.913 E.00216
G1 X85.802 Y88.502 E.00208
G1 X85.972 Y88.09 E.00216
G1 X86.174 Y87.712 E.00208
G1 X86.422 Y87.341 E.00216
G1 X86.694 Y87.01 E.00208
G1 X87.001 Y86.702 E.00211
G1 X87.341 Y86.422 E.00213
G1 X87.712 Y86.174 E.00216
G1 X88.09 Y85.972 E.00208
G1 X88.492 Y85.805 E.00211
G1 X88.913 Y85.677 E.00213
G1 X89.339 Y85.592 E.00211
G1 X89.777 Y85.548 E.00213
G1 X90.109 Y85.545 E.00161
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.129 Y85.289 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F3861
M204 S6000
G1 X89.767 Y85.291 E.00176
G1 X89.306 Y85.337 E.00225
G1 X88.853 Y85.427 E.00224
G1 X88.41 Y85.562 E.00224
G1 X87.983 Y85.739 E.00224
G1 X87.578 Y85.956 E.00223
G1 X87.19 Y86.214 E.00226
G1 X86.832 Y86.508 E.00224
G1 X86.508 Y86.833 E.00222
G1 X86.212 Y87.193 E.00226
G1 X85.956 Y87.578 E.00224
G1 X85.739 Y87.982 E.00223
G1 X85.562 Y88.41 E.00224
G1 X85.427 Y88.853 E.00224
G1 X85.337 Y89.306 E.00224
G1 X85.291 Y89.77 E.00226
G1 X85.291 Y90.23 E.00223
G1 X85.337 Y90.694 E.00226
G1 X85.427 Y91.147 E.00224
G1 X85.562 Y91.59 E.00225
G1 X85.739 Y92.017 E.00224
G1 X85.957 Y92.425 E.00224
G1 X86.214 Y92.81 E.00224
G1 X86.508 Y93.167 E.00224
G1 X86.833 Y93.492 E.00222
G1 X87.19 Y93.786 E.00225
G1 X87.575 Y94.043 E.00224
G1 X87.986 Y94.263 E.00226
G1 X88.41 Y94.438 E.00222
G1 X88.853 Y94.573 E.00224
G1 X89.31 Y94.664 E.00226
G1 X89.77 Y94.709 E.00224
G1 X90.233 Y94.709 E.00224
G1 X90.69 Y94.664 E.00223
G1 X91.144 Y94.573 E.00224
G1 X91.59 Y94.438 E.00226
G1 X92.014 Y94.263 E.00222
G1 X92.426 Y94.043 E.00226
G1 X92.807 Y93.788 E.00222
G1 X93.165 Y93.494 E.00224
G1 X93.494 Y93.165 E.00226
G1 X93.788 Y92.807 E.00224
G1 X94.045 Y92.422 E.00224
G1 X94.263 Y92.014 E.00224
G1 X94.439 Y91.586 E.00224
G1 X94.574 Y91.144 E.00224
G1 X94.664 Y90.69 E.00224
G1 X94.709 Y90.23 E.00224
G1 X94.709 Y89.767 E.00224
G1 X94.663 Y89.306 E.00224
G1 X94.574 Y88.856 E.00223
G1 X94.439 Y88.414 E.00224
G1 X94.263 Y87.986 E.00224
G1 X94.043 Y87.575 E.00226
G1 X93.785 Y87.19 E.00224
G1 X93.492 Y86.833 E.00224
G1 X93.165 Y86.506 E.00224
G1 X92.81 Y86.214 E.00223
G1 X92.425 Y85.957 E.00224
G1 X92.014 Y85.737 E.00226
G1 X91.586 Y85.561 E.00224
G1 X91.144 Y85.427 E.00224
G1 X90.69 Y85.336 E.00224
G1 X90.129 Y85.289 E.00273
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 2
; WIPE_START
G1 F4200
G1 X90.69 Y85.336 E-.21397
G1 X91.119 Y85.422 E-.16603
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I-1.188 J-.264 P1  F42000
G1 X90.835 Y86.699 Z1.7
G1 Z1.3
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F1440
M204 S6000
G1 X91.147 Y86.794 E.01036
G1 X91.456 Y86.922 E.01064
G1 X91.751 Y87.079 E.01063
G1 X92.028 Y87.265 E.01063
G1 X92.287 Y87.477 E.01062
G1 X92.523 Y87.713 E.01064
G1 X92.735 Y87.972 E.01062
G1 X92.921 Y88.25 E.01063
G1 X93.078 Y88.544 E.01063
G1 X93.206 Y88.853 E.01064
G1 X93.303 Y89.173 E.01063
G1 X93.368 Y89.5 E.01064
G1 X93.401 Y89.833 E.01063
G1 X93.401 Y90.167 E.01064
G1 X93.368 Y90.499 E.01063
G1 X93.303 Y90.828 E.01064
G1 X93.206 Y91.147 E.01062
G1 X93.078 Y91.456 E.01064
G1 X92.921 Y91.75 E.01062
G1 X92.735 Y92.028 E.01064
G1 X92.523 Y92.287 E.01064
G1 X92.287 Y92.523 E.01063
G1 X92.028 Y92.735 E.01063
G1 X91.751 Y92.921 E.01063
G1 X91.456 Y93.078 E.01063
G1 X91.147 Y93.206 E.01064
G1 X90.828 Y93.303 E.01062
G1 X90.5 Y93.368 E.01064
G1 X90.167 Y93.401 E.01063
G1 X89.833 Y93.401 E.01064
G1 X89.501 Y93.368 E.01062
G1 X89.173 Y93.303 E.01064
G1 X88.853 Y93.206 E.01063
G1 X88.544 Y93.078 E.01064
G1 X88.249 Y92.921 E.01063
G1 X87.972 Y92.735 E.01063
G1 X87.713 Y92.523 E.01063
G1 X87.477 Y92.286 E.01065
G1 X87.265 Y92.029 E.01061
G1 X87.079 Y91.75 E.01064
G1 X86.922 Y91.456 E.01063
G1 X86.794 Y91.147 E.01064
G1 X86.697 Y90.828 E.01062
G1 X86.632 Y90.5 E.01064
G1 X86.599 Y90.167 E.01063
G1 X86.599 Y89.833 E.01064
G1 X86.632 Y89.5 E.01063
G1 X86.697 Y89.173 E.01064
G1 X86.794 Y88.853 E.01063
G1 X86.922 Y88.544 E.01064
G1 X87.079 Y88.25 E.01063
G1 X87.265 Y87.972 E.01064
G1 X87.477 Y87.713 E.01062
G1 X87.714 Y87.477 E.01065
G1 X87.971 Y87.265 E.01061
G1 X88.25 Y87.079 E.01064
G1 X88.544 Y86.922 E.01062
G1 X88.853 Y86.794 E.01065
G1 X89.172 Y86.697 E.01062
G1 X89.5 Y86.632 E.01064
G1 X89.835 Y86.599 E.0107
G1 X90.077 Y86.597 E.00768
G1 X90.504 Y86.633 E.01365
G1 X90.777 Y86.687 E.00885
M106 S124.95
M106 S127.5
; COOLING_NODE: 2
M204 S250
G1 X90.949 Y86.324 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.279 Y86.424 E.01016
G1 X91.624 Y86.567 E.011
G1 X91.952 Y86.743 E.01098
G1 X92.262 Y86.95 E.01098
G1 X92.55 Y87.186 E.01098
G1 X92.814 Y87.45 E.01099
G1 X93.05 Y87.738 E.01098
G1 X93.257 Y88.048 E.01099
G1 X93.433 Y88.376 E.01098
G1 X93.576 Y88.721 E.01099
G1 X93.684 Y89.077 E.01097
G1 X93.756 Y89.443 E.01099
G1 X93.793 Y89.814 E.01098
G1 X93.793 Y90.186 E.01099
G1 X93.756 Y90.557 E.01098
G1 X93.684 Y90.923 E.01099
G1 X93.576 Y91.279 E.01097
G1 X93.433 Y91.624 E.01099
G1 X93.257 Y91.952 E.01097
G1 X93.05 Y92.262 E.01099
G1 X92.814 Y92.55 E.01099
G1 X92.55 Y92.814 E.01099
G1 X92.262 Y93.05 E.01098
G1 X91.952 Y93.257 E.01099
G1 X91.624 Y93.433 E.01098
G1 X91.279 Y93.576 E.01099
G1 X90.923 Y93.684 E.01097
G1 X90.557 Y93.756 E.01099
G1 X90.186 Y93.793 E.01098
G1 X89.814 Y93.793 E.01099
G1 X89.443 Y93.756 E.01098
G1 X89.077 Y93.684 E.01099
G1 X88.721 Y93.576 E.01098
G1 X88.376 Y93.433 E.01099
G1 X88.048 Y93.257 E.01098
G1 X87.738 Y93.05 E.01098
G1 X87.45 Y92.814 E.01098
G1 X87.186 Y92.55 E.011
G1 X86.95 Y92.262 E.01097
G1 X86.743 Y91.952 E.01099
G1 X86.567 Y91.624 E.01098
G1 X86.424 Y91.279 E.01099
G1 X86.316 Y90.923 E.01097
G1 X86.244 Y90.557 E.01099
G1 X86.207 Y90.186 E.01098
G1 X86.207 Y89.814 E.01099
G1 X86.244 Y89.443 E.01098
G1 X86.316 Y89.077 E.01099
G1 X86.424 Y88.721 E.01098
G1 X86.567 Y88.376 E.01099
G1 X86.743 Y88.048 E.01098
G1 X86.95 Y87.738 E.01099
G1 X87.186 Y87.45 E.01097
G1 X87.45 Y87.186 E.011
G1 X87.738 Y86.95 E.01097
G1 X88.048 Y86.743 E.01099
G1 X88.376 Y86.567 E.01097
G1 X88.721 Y86.424 E.011
G1 X89.077 Y86.316 E.01097
G1 X89.443 Y86.244 E.01099
G1 X89.814 Y86.207 E.011
G1 X90.091 Y86.205 E.00816
G1 X90.559 Y86.244 E.01382
G1 X90.891 Y86.31 E.00999
M106 S124.95
; WIPE_START
M204 S6000
G1 X91.279 Y86.424 E-.15379
G1 X91.624 Y86.567 E-.14177
G1 X91.82 Y86.672 E-.08445
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z1.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z1.7 F4000
            G39.3 S1
            G0 Z1.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X88.575 Y87.091 F42000
G1 Z1.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.42599
G1 F3861
M204 S6000
G1 X87.704 Y87.962 E.03689
G1 X87.533 Y88.17 E.00807
G1 X87.366 Y88.421 E.00903
G1 X87.224 Y88.687 E.00902
G1 X87.05 Y89.158 E.01504
G1 X89.158 Y87.05 E.08928
G1 X89.549 Y86.962 E.012
G1 X89.814 Y86.936 E.00795
G1 X86.936 Y89.814 E.12186
G1 X86.932 Y90.151 E.01009
G1 X86.951 Y90.34 E.0057
G1 X90.338 Y86.953 E.14342
G1 X90.797 Y87.036 E.01396
G1 X87.036 Y90.797 E.15925
G1 X87.176 Y91.199 E.01274
G1 X91.199 Y87.176 E.17033
G1 X91.561 Y87.356 E.0121
G1 X87.356 Y91.561 E.17805
G1 X87.576 Y91.882 E.01167
G1 X91.882 Y87.576 E.18233
G1 X92.169 Y87.831 E.01149
G1 X87.831 Y92.169 E.1837
G1 X88.118 Y92.424 E.01149
G1 X92.424 Y88.118 E.18233
G1 X92.644 Y88.439 E.01167
G1 X88.439 Y92.644 E.17805
G1 X88.801 Y92.824 E.0121
G1 X92.824 Y88.801 E.17033
G1 X92.964 Y89.203 E.01274
G1 X89.203 Y92.964 E.15925
G1 X89.66 Y93.049 E.01391
G1 X93.049 Y89.66 E.14351
G1 X93.068 Y89.849 E.0057
G1 X93.064 Y90.186 E.0101
G1 X90.186 Y93.064 E.12185
G1 X90.451 Y93.038 E.00795
G1 X90.842 Y92.95 E.012
G1 X92.95 Y90.842 E.08927
G1 X92.776 Y91.313 E.01504
G1 X92.634 Y91.579 E.00902
G1 X92.467 Y91.83 E.00903
G1 X92.295 Y92.039 E.00809
G1 X91.425 Y92.909 E.03686
; CHANGE_LAYER
; Z_HEIGHT: 1.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F10180.907
G1 X92.132 Y92.202 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 8/43
; update layer progress
M73 L8
M991 S0 P7 ;notify layer change
; OBJECT_ID: 346
M204 S10000
G17
G3 Z1.7 I-.906 J.812 P1  F42000
G1 X95.467 Y95.924 Z1.7
G1 Z1.5
G1 E.4 F1800
; FEATURE: Support interface
; LINE_WIDTH: 0.42
G1 F4800
M204 S6000
M73 P46 R9
G1 X95.467 Y84.246 E.34419
G1 X95.09 Y84.246 E.01111
G1 X95.09 Y95.754 E.33919
G1 X94.713 Y95.754 E.01111
G1 X94.713 Y91.817 E.11605
G1 X94.565 Y92.166 E.01117
G1 X94.336 Y92.594 E.01431
G1 X94.336 Y95.754 E.09315
G1 X93.959 Y95.754 E.01111
G1 X93.959 Y93.137 E.07713
G1 X93.582 Y93.563 E.01676
G1 X93.582 Y95.754 E.06458
G1 X93.205 Y95.754 E.01111
G1 X93.205 Y93.906 E.05447
G1 X92.828 Y94.187 E.01385
G1 X92.828 Y95.754 E.0462
G1 X92.451 Y95.754 E.01111
G1 X92.451 Y94.417 E.03941
G1 X92.074 Y94.606 E.01243
G1 X92.074 Y95.754 E.03384
G1 X91.697 Y95.754 E.01111
G1 X91.697 Y94.759 E.02932
G1 X91.32 Y94.876 E.01163
G1 X91.32 Y95.754 E.02588
G1 X90.943 Y95.754 E.01111
G1 X90.943 Y94.963 E.02331
G1 X90.566 Y95.02 E.01124
G1 X90.566 Y95.754 E.02164
G1 X90.188 Y95.754 E.01111
G1 X90.188 Y95.048 E.02081
G1 X89.811 Y95.048 E.01111
G1 X89.811 Y95.754 E.02081
G1 X89.434 Y95.754 E.01111
G1 X89.434 Y95.02 E.02164
G1 X89.057 Y94.963 E.01124
G1 X89.057 Y95.754 E.02331
G1 X88.68 Y95.754 E.01111
G1 X88.68 Y94.876 E.02588
G1 X88.303 Y94.759 E.01163
G1 X88.303 Y95.754 E.02933
G1 X87.926 Y95.754 E.01111
G1 X87.926 Y94.606 E.03384
G1 X87.549 Y94.417 E.01243
G1 X87.549 Y95.754 E.03941
G1 X87.172 Y95.754 E.01111
G1 X87.172 Y94.187 E.0462
G1 X86.795 Y93.906 E.01385
G1 X86.795 Y95.754 E.05447
G1 X86.418 Y95.754 E.01111
G1 X86.418 Y93.563 E.06458
G1 X86.041 Y93.137 E.01676
G1 X86.041 Y95.754 E.07713
G1 X85.664 Y95.754 E.01111
G1 X85.664 Y92.594 E.09316
G1 X85.435 Y92.166 E.01431
G1 X85.287 Y91.817 E.01117
G1 X85.287 Y95.754 E.11606
G1 X84.91 Y95.754 E.01111
G1 X84.91 Y84.246 E.33919
G1 X85.287 Y84.246 E.01111
G1 X85.287 Y88.183 E.11606
G1 X85.435 Y87.834 E.01117
G1 X85.664 Y87.406 E.01431
G1 X85.664 Y84.246 E.09315
G1 X86.041 Y84.246 E.01111
G1 X86.041 Y86.863 E.07713
G1 X86.418 Y86.437 E.01676
G1 X86.418 Y84.246 E.06458
G1 X86.795 Y84.246 E.01111
G1 X86.795 Y86.094 E.05447
G1 X87.172 Y85.813 E.01385
G1 X87.172 Y84.246 E.0462
G1 X87.549 Y84.246 E.01111
G1 X87.549 Y85.583 E.03941
G1 X87.926 Y85.394 E.01243
G1 X87.926 Y84.246 E.03384
G1 X88.303 Y84.246 E.01111
G1 X88.303 Y85.241 E.02932
G1 X88.68 Y85.124 E.01163
G1 X88.68 Y84.246 E.02588
G1 X89.057 Y84.246 E.01111
G1 X89.057 Y85.037 E.02331
G1 X89.434 Y84.98 E.01124
G1 X89.434 Y84.246 E.02164
G1 X89.811 Y84.246 E.01111
G1 X89.811 Y84.952 E.02081
G1 X90.188 Y84.952 E.01111
G1 X90.188 Y84.246 E.02081
G1 X90.566 Y84.246 E.01111
G1 X90.566 Y84.98 E.02164
G1 X90.943 Y85.037 E.01124
G1 X90.943 Y84.246 E.02331
G1 X91.32 Y84.246 E.01111
G1 X91.32 Y85.124 E.02588
G1 X91.697 Y85.241 E.01163
G1 X91.697 Y84.246 E.02932
G1 X92.074 Y84.246 E.01111
G1 X92.074 Y85.394 E.03384
G1 X92.451 Y85.583 E.01243
G1 X92.451 Y84.246 E.03941
G1 X92.828 Y84.246 E.01111
G1 X92.828 Y85.813 E.0462
G1 X93.205 Y86.094 E.01385
G1 X93.205 Y84.246 E.05447
G1 X93.582 Y84.246 E.01111
G1 X93.582 Y86.437 E.06458
G1 X93.959 Y86.863 E.01676
G1 X93.959 Y84.246 E.07713
G1 X94.336 Y84.246 E.01111
G1 X94.336 Y87.406 E.09315
G1 X94.565 Y87.834 E.01431
G1 X94.713 Y88.183 E.01116
G1 X94.713 Y84.076 E.12105
M204 S10000
G1 X95.733 Y83.847 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X93.276 Y83.847 E.07242
G1 X93.276 Y81.39 E.07241
G1 X95.733 Y81.39 E.07242
G1 X95.733 Y83.791 E.07075
M204 S10000
G1 X92.828 Y84.038 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X92.828 Y81.369 E.07869
G1 X92.451 Y81.369 E.01111
G1 X92.451 Y83.869 E.07369
G1 X92.074 Y83.869 E.01111
G1 X92.074 Y81.369 E.07369
G1 X91.697 Y81.369 E.01111
G1 X91.697 Y83.869 E.07369
G1 X91.32 Y83.869 E.01111
G1 X91.32 Y81.369 E.07369
G1 X90.943 Y81.369 E.01111
G1 X90.943 Y83.869 E.07369
G1 X90.566 Y83.869 E.01111
G1 X90.566 Y81.369 E.07369
G1 X90.188 Y81.369 E.01111
G1 X90.188 Y83.869 E.07369
G1 X89.811 Y83.869 E.01111
G1 X89.811 Y81.369 E.07369
G1 X89.434 Y81.369 E.01111
G1 X89.434 Y83.869 E.07369
G1 X89.057 Y83.869 E.01111
G1 X89.057 Y81.369 E.07369
G1 X88.68 Y81.369 E.01111
G1 X88.68 Y83.869 E.07369
G1 X88.303 Y83.869 E.01111
G1 X88.303 Y81.369 E.07369
G1 X87.926 Y81.369 E.01111
G1 X87.926 Y83.869 E.07369
G1 X87.549 Y83.869 E.01111
G1 X87.549 Y81.369 E.07369
G1 X87.172 Y81.369 E.01111
G1 X87.172 Y84.038 E.07869
M204 S10000
G1 X86.724 Y83.847 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X84.267 Y83.847 E.07241
G1 X84.267 Y81.39 E.07241
G1 X86.724 Y81.39 E.07241
G1 X86.724 Y83.791 E.07075
; WIPE_START
G1 X86.724 Y82.791 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I-.616 J-1.05 P1  F42000
G1 X84.532 Y84.076 Z1.9
G1 Z1.5
G1 E.4 F1800
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X84.532 Y95.924 E.3492
M204 S10000
G1 X83.847 Y95.733 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X81.39 Y95.733 E.07241
G1 X81.39 Y93.276 E.07242
G1 X83.847 Y93.276 E.07241
G1 X83.847 Y95.676 E.07075
M204 S10000
G1 X83.778 Y93.047 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X83.778 Y87.123 E.17461
G1 X83.401 Y87.123 E.01111
M73 P46 R8
G1 X83.401 Y92.877 E.16961
G1 X83.024 Y92.877 E.01111
G1 X83.024 Y87.123 E.16961
G1 X82.647 Y87.123 E.01111
G1 X82.647 Y92.877 E.16961
G1 X82.27 Y92.877 E.01111
G1 X82.27 Y87.123 E.16961
G1 X81.893 Y87.123 E.01111
G1 X81.893 Y92.877 E.16961
G1 X81.516 Y92.877 E.01111
G1 X81.516 Y86.953 E.17461
M204 S10000
G1 X83.847 Y86.724 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X81.39 Y86.724 E.07241
G1 X81.39 Y84.267 E.07241
G1 X83.847 Y84.267 E.07241
G1 X83.847 Y86.668 E.07075
M204 S10000
G1 X86.724 Y98.553 F42000
G1 F9000
M204 S6000
G1 X86.724 Y96.153 E.07075
G1 X84.267 Y96.153 E.07241
G1 X84.267 Y98.61 E.07242
G1 X86.724 Y98.61 E.07241
M204 S10000
G1 X87.172 Y98.801 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X87.172 Y96.131 E.07869
G1 X87.549 Y96.131 E.01111
G1 X87.549 Y98.631 E.07369
G1 X87.926 Y98.631 E.01111
G1 X87.926 Y96.131 E.07369
G1 X88.303 Y96.131 E.01111
G1 X88.303 Y98.631 E.07369
G1 X88.68 Y98.631 E.01111
G1 X88.68 Y96.131 E.07369
G1 X89.057 Y96.131 E.01111
G1 X89.057 Y98.631 E.07369
G1 X89.434 Y98.631 E.01111
G1 X89.434 Y96.131 E.07369
G1 X89.811 Y96.131 E.01111
G1 X89.811 Y98.631 E.07369
G1 X90.188 Y98.631 E.01111
G1 X90.188 Y96.131 E.07369
G1 X90.566 Y96.131 E.01111
G1 X90.566 Y98.631 E.07369
G1 X90.943 Y98.631 E.01111
G1 X90.943 Y96.131 E.07369
G1 X91.32 Y96.131 E.01111
G1 X91.32 Y98.631 E.07369
G1 X91.697 Y98.631 E.01111
G1 X91.697 Y96.131 E.07369
G1 X92.074 Y96.131 E.01111
G1 X92.074 Y98.631 E.07369
G1 X92.451 Y98.631 E.01111
G1 X92.451 Y96.131 E.07369
G1 X92.828 Y96.131 E.01111
G1 X92.828 Y98.801 E.07869
M204 S10000
G1 X95.733 Y98.61 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X93.276 Y98.61 E.07242
G1 X93.276 Y96.153 E.07242
G1 X95.733 Y96.153 E.07242
G1 X95.733 Y98.553 E.07075
; WIPE_START
G1 X95.733 Y97.553 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I.651 J1.028 P1  F42000
G1 X98.61 Y95.733 Z1.9
G1 Z1.5
G1 E.4 F1800
M106 S127.5
G1 F9000
M204 S6000
G1 X96.153 Y95.733 E.07242
G1 X96.153 Y93.276 E.07242
G1 X98.61 Y93.276 E.07242
G1 X98.61 Y95.676 E.07075
M204 S10000
G1 X98.484 Y93.047 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X98.484 Y87.123 E.17461
G1 X98.107 Y87.123 E.01111
G1 X98.107 Y92.877 E.16961
G1 X97.73 Y92.877 E.01111
G1 X97.73 Y87.123 E.16961
G1 X97.353 Y87.123 E.01111
G1 X97.353 Y92.877 E.16961
G1 X96.976 Y92.877 E.01111
G1 X96.976 Y87.123 E.16961
G1 X96.599 Y87.123 E.01111
G1 X96.599 Y92.877 E.16961
G1 X96.222 Y92.877 E.01111
G1 X96.222 Y86.953 E.17461
M204 S10000
G1 X98.61 Y86.724 F42000
; FEATURE: Support
G1 F9000
M204 S6000
G1 X96.153 Y86.724 E.07242
G1 X96.153 Y84.267 E.07241
G1 X98.61 Y84.267 E.07242
G1 X98.61 Y86.668 E.07075
; WIPE_START
G1 X98.61 Y85.668 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I.097 J-1.213 P1  F42000
G1 X90.137 Y84.989 Z1.9
G1 Z1.5
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X90.73 Y85.039 E.00289
G1 X91.213 Y85.135 E.00238
G1 X91.684 Y85.277 E.00239
G1 X92.139 Y85.465 E.00238
G1 X92.573 Y85.696 E.00239
G1 X92.982 Y85.969 E.00238
G1 X93.363 Y86.281 E.00239
G1 X93.712 Y86.628 E.00239
G1 X94.024 Y87.009 E.00239
G1 X94.298 Y87.417 E.00238
G1 X94.531 Y87.851 E.00239
G1 X94.719 Y88.305 E.00238
G1 X94.863 Y88.776 E.00239
G1 X94.959 Y89.259 E.00238
G1 X95.008 Y89.749 E.00239
G1 X95.008 Y90.24 E.00239
G1 X94.961 Y90.73 E.00239
G1 X94.865 Y91.213 E.00239
G1 X94.723 Y91.684 E.00239
G1 X94.535 Y92.139 E.00239
G1 X94.304 Y92.573 E.00239
G1 X94.031 Y92.983 E.00238
G1 X93.719 Y93.363 E.00239
G1 X93.372 Y93.711 E.00238
G1 X92.991 Y94.024 E.00239
G1 X92.583 Y94.298 E.00239
G1 X92.149 Y94.531 E.00239
M73 P47 R8
G1 X91.695 Y94.719 E.00238
G1 X91.224 Y94.863 E.00239
G1 X90.741 Y94.959 E.00239
G1 X90.251 Y95.008 E.00239
G1 X89.76 Y95.009 E.00239
G1 X89.269 Y94.961 E.00239
G1 X88.787 Y94.865 E.00238
G1 X88.316 Y94.723 E.00239
G1 X87.861 Y94.535 E.00238
G1 X87.427 Y94.304 E.00239
G1 X87.018 Y94.031 E.00238
G1 X86.637 Y93.719 E.00239
G1 X86.289 Y93.372 E.00238
G1 X85.976 Y92.991 E.00239
G1 X85.702 Y92.583 E.00238
G1 X85.469 Y92.149 E.00239
G1 X85.281 Y91.695 E.00238
G1 X85.137 Y91.223 E.00239
G1 X85.041 Y90.741 E.00238
G1 X84.992 Y90.251 E.00239
G1 X84.992 Y89.76 E.00239
G1 X85.039 Y89.27 E.00239
G1 X85.137 Y88.776 E.00244
G1 X85.281 Y88.305 E.00239
G1 X85.469 Y87.851 E.00238
G1 X85.696 Y87.427 E.00233
G1 X85.969 Y87.018 E.00238
G1 X86.281 Y86.637 E.00239
G1 X86.628 Y86.288 E.00238
G1 X87.009 Y85.976 E.00239
G1 X87.417 Y85.702 E.00238
G1 X87.851 Y85.469 E.00239
G1 X88.305 Y85.281 E.00238
G1 X88.776 Y85.137 E.00239
G1 X89.259 Y85.041 E.00239
G1 X89.749 Y84.992 E.00239
G1 X90.137 Y84.989 E.00188
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.184 Y84.797 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X90.762 Y84.849 E.00282
G1 X91.264 Y84.949 E.00248
G1 X91.753 Y85.097 E.00248
G1 X92.225 Y85.292 E.00248
G1 X92.678 Y85.535 E.00249
G1 X93.103 Y85.819 E.00248
G1 X93.498 Y86.143 E.00248
G1 X93.857 Y86.502 E.00246
G1 X94.184 Y86.9 E.0025
G1 X94.467 Y87.324 E.00248
G1 X94.708 Y87.775 E.00248
G1 X94.903 Y88.248 E.00248
G1 X95.052 Y88.737 E.00248
G1 X95.15 Y89.234 E.00246
G1 X95.201 Y89.743 E.00248
G1 X95.201 Y90.254 E.00248
G1 X95.151 Y90.762 E.00248
G1 X95.051 Y91.264 E.00248
G1 X94.902 Y91.756 E.00249
G1 X94.706 Y92.228 E.00248
G1 X94.467 Y92.676 E.00246
G1 X94.181 Y93.103 E.00249
G1 X93.857 Y93.498 E.00248
G1 X93.495 Y93.859 E.00248
G1 X93.1 Y94.184 E.00248
G1 X92.675 Y94.467 E.00248
G1 X92.225 Y94.708 E.00248
G1 X91.756 Y94.902 E.00246
G1 X91.263 Y95.052 E.0025
G1 X90.762 Y95.151 E.00248
G1 X90.258 Y95.201 E.00246
G1 X89.746 Y95.201 E.00248
G1 X89.234 Y95.15 E.0025
G1 X88.736 Y95.051 E.00246
G1 X88.248 Y94.903 E.00248
G1 X87.775 Y94.708 E.00248
G1 X87.324 Y94.467 E.00248
G1 X86.897 Y94.181 E.00249
G1 X86.502 Y93.857 E.00248
G1 X86.143 Y93.498 E.00246
G1 X85.817 Y93.101 E.00249
G1 X85.533 Y92.675 E.00248
G1 X85.292 Y92.225 E.00248
G1 X85.097 Y91.752 E.00248
G1 X84.948 Y91.263 E.00248
G1 X84.85 Y90.766 E.00246
G1 X84.799 Y90.258 E.00248
G1 X84.799 Y89.746 E.00248
G1 X84.849 Y89.238 E.00248
G1 X84.949 Y88.733 E.0025
G1 X85.098 Y88.244 E.00248
G1 X85.292 Y87.775 E.00246
G1 X85.533 Y87.324 E.00248
G1 X85.819 Y86.897 E.00249
G1 X86.141 Y86.504 E.00246
G1 X86.502 Y86.143 E.00248
G1 X86.897 Y85.819 E.00248
G1 X87.325 Y85.533 E.0025
G1 X87.772 Y85.294 E.00246
G1 X88.244 Y85.098 E.00248
G1 X88.737 Y84.949 E.0025
G1 X89.234 Y84.85 E.00246
G1 X89.743 Y84.799 E.00248
G1 X90.184 Y84.797 E.00214
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 3
; WIPE_START
G1 X89.743 Y84.799 E-.16764
G1 X89.234 Y84.85 E-.19424
G1 X89.187 Y84.859 E-.01812
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I-.715 J.985 P1  F42000
G1 X90.986 Y86.164 Z1.9
G1 Z1.5
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F2640
M204 S6000
G1 X91.335 Y86.27 E.0116
G1 X91.694 Y86.419 E.01237
G1 X92.037 Y86.602 E.01237
G1 X92.36 Y86.818 E.01237
G1 X92.66 Y87.065 E.01237
G1 X92.935 Y87.34 E.01237
G1 X93.182 Y87.64 E.01237
G1 X93.398 Y87.963 E.01237
G1 X93.581 Y88.306 E.01237
G1 X93.73 Y88.665 E.01237
G1 X93.843 Y89.037 E.01236
G1 X93.919 Y89.419 E.01238
G1 X93.957 Y89.806 E.01237
G1 X93.957 Y90.194 E.01237
G1 X93.919 Y90.581 E.01237
G1 X93.843 Y90.963 E.01237
G1 X93.73 Y91.335 E.01237
G1 X93.581 Y91.694 E.01238
G1 X93.398 Y92.037 E.01237
G1 X93.182 Y92.36 E.01237
G1 X92.935 Y92.66 E.01237
G1 X92.66 Y92.935 E.01237
G1 X92.36 Y93.182 E.01237
G1 X92.037 Y93.398 E.01237
G1 X91.694 Y93.581 E.01237
G1 X91.335 Y93.73 E.01237
G1 X90.963 Y93.843 E.01237
G1 X90.581 Y93.919 E.01238
G1 X90.194 Y93.957 E.01237
G1 X89.806 Y93.957 E.01237
G1 X89.419 Y93.919 E.01237
G1 X89.037 Y93.843 E.01237
G1 X88.665 Y93.73 E.01237
G1 X88.306 Y93.581 E.01237
G1 X87.963 Y93.398 E.01238
G1 X87.64 Y93.182 E.01236
G1 X87.34 Y92.935 E.01237
G1 X87.065 Y92.66 E.01237
G1 X86.818 Y92.36 E.01237
G1 X86.602 Y92.037 E.01237
G1 X86.419 Y91.694 E.01237
G1 X86.27 Y91.335 E.01237
G1 X86.157 Y90.962 E.01237
G1 X86.081 Y90.581 E.01236
G1 X86.043 Y90.194 E.01237
G1 X86.043 Y89.806 E.01237
G1 X86.081 Y89.419 E.01237
G1 X86.157 Y89.037 E.01238
G1 X86.27 Y88.665 E.01236
G1 X86.419 Y88.306 E.01237
G1 X86.602 Y87.963 E.01238
G1 X86.818 Y87.64 E.01236
G1 X87.065 Y87.34 E.01237
G1 X87.34 Y87.065 E.01237
G1 X87.64 Y86.818 E.01237
G1 X87.963 Y86.602 E.01237
G1 X88.306 Y86.419 E.01237
G1 X88.665 Y86.27 E.01237
G1 X89.037 Y86.157 E.01236
G1 X89.419 Y86.081 E.01238
G1 X89.808 Y86.043 E.01243
M106 S124.95
M106 S127.5
G1 F2520
G1 X90.102 Y86.041 E.00937
M106 S124.95
M106 S127.5
G1 F2640
G1 X90.585 Y86.082 E.01543
M106 S124.95
M106 S127.5
G1 X90.928 Y86.15 E.0111
M106 S124.95
M106 S127.5
; COOLING_NODE: 3
M204 S250
G1 X91.1 Y85.789 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.467 Y85.9 E.01131
G1 X91.862 Y86.064 E.01259
G1 X92.238 Y86.265 E.01259
G1 X92.594 Y86.503 E.01259
G1 X92.924 Y86.774 E.0126
G1 X93.226 Y87.076 E.01259
G1 X93.497 Y87.406 E.0126
G1 X93.735 Y87.761 E.01259
G1 X93.936 Y88.138 E.0126
G1 X94.1 Y88.533 E.01259
G1 X94.224 Y88.942 E.01259
G1 X94.307 Y89.361 E.0126
G1 X94.349 Y89.786 E.01259
G1 X94.349 Y90.214 E.0126
G1 X94.307 Y90.639 E.01259
G1 X94.224 Y91.058 E.0126
G1 X94.1 Y91.467 E.01259
G1 X93.936 Y91.862 E.0126
G1 X93.735 Y92.239 E.01259
G1 X93.497 Y92.594 E.01259
G1 X93.226 Y92.924 E.01259
G1 X92.924 Y93.226 E.01259
G1 X92.594 Y93.497 E.0126
G1 X92.238 Y93.735 E.01259
G1 X91.862 Y93.936 E.01259
G1 X91.467 Y94.1 E.01259
G1 X91.058 Y94.224 E.01259
G1 X90.639 Y94.307 E.0126
G1 X90.214 Y94.349 E.01259
G1 X89.786 Y94.349 E.0126
G1 X89.361 Y94.307 E.01259
G1 X88.942 Y94.224 E.0126
G1 X88.533 Y94.1 E.01259
G1 X88.138 Y93.936 E.01259
G1 X87.761 Y93.735 E.0126
G1 X87.406 Y93.497 E.01259
G1 X87.076 Y93.226 E.01259
G1 X86.774 Y92.924 E.01259
G1 X86.503 Y92.594 E.0126
G1 X86.265 Y92.238 E.01259
G1 X86.064 Y91.862 E.01259
G1 X85.9 Y91.467 E.01259
G1 X85.776 Y91.058 E.0126
G1 X85.693 Y90.639 E.01259
G1 X85.651 Y90.214 E.01259
G1 X85.651 Y89.786 E.0126
G1 X85.693 Y89.361 E.01259
G1 X85.776 Y88.942 E.0126
G1 X85.9 Y88.533 E.01259
G1 X86.064 Y88.138 E.01259
G1 X86.265 Y87.761 E.0126
G1 X86.503 Y87.406 E.01259
G1 X86.774 Y87.076 E.0126
G1 X87.076 Y86.774 E.01259
G1 X87.406 Y86.503 E.0126
G1 X87.761 Y86.265 E.01259
G1 X88.138 Y86.064 E.01259
G1 X88.533 Y85.9 E.01259
G1 X88.942 Y85.776 E.01259
G1 X89.361 Y85.693 E.0126
G1 X89.787 Y85.651 E.01261
G1 X90.117 Y85.649 E.00974
G1 X90.64 Y85.693 E.01547
G1 X91.042 Y85.773 E.01207
M106 S124.95
; WIPE_START
M204 S6000
G1 X91.467 Y85.9 E-.16856
G1 X91.862 Y86.064 E-.16237
G1 X91.975 Y86.125 E-.04906
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z1.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z1.9 F4000
            G39.3 S1
            G0 Z1.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.144 Y87.75 F42000
G1 Z1.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F9253
M204 S6000
G1 X90.656 Y87.838 E.01654
G1 X91.065 Y88.008 E.01409
G1 X91.433 Y88.254 E.01409
G1 X91.589 Y88.411 E.00704
G1 X88.411 Y91.589 E.14296
G1 X88.254 Y91.433 E.00704
G1 X88.008 Y91.065 E.01409
G1 X87.838 Y90.656 E.01409
G1 X87.752 Y90.221 E.01409
G1 X87.752 Y89.779 E.01409
G1 X87.838 Y89.344 E.01409
G1 X88.008 Y88.935 E.01409
G1 X88.254 Y88.567 E.01409
G1 X88.411 Y88.411 E.00705
G1 X91.589 Y91.589 E.14296
G1 X91.746 Y91.433 E.00704
G1 X91.992 Y91.065 E.01409
G1 X92.162 Y90.656 E.01409
G1 X92.25 Y90.144 E.01654
; WIPE_START
G1 F9580.435
G1 X92.162 Y90.656 E-.19748
G1 X91.992 Y91.065 E-.16832
G1 X91.971 Y91.096 E-.01419
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I.956 J-.752 P1  F42000
G1 X89.143 Y87.501 Z1.9
G1 Z1.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.41999
G1 F9253
M204 S6000
G1 X89.833 Y87.36 E.02075
G1 X90.165 Y87.36 E.00977
G1 X90.675 Y87.442 E.01524
G1 X91.161 Y87.623 E.01528
G1 X91.603 Y87.895 E.01528
G1 X91.982 Y88.248 E.01528
G1 X92.286 Y88.669 E.0153
G1 X92.502 Y89.14 E.01526
G1 X92.622 Y89.645 E.0153
G1 X92.64 Y90.163 E.01528
G1 X92.558 Y90.675 E.01528
G1 X92.377 Y91.161 E.01528
G1 X92.105 Y91.603 E.01528
G1 X91.752 Y91.982 E.01528
G1 X91.331 Y92.286 E.01529
G1 X90.86 Y92.502 E.01528
G1 X90.355 Y92.622 E.0153
G1 X89.836 Y92.64 E.01528
G1 X89.325 Y92.558 E.01528
G1 X88.839 Y92.377 E.01529
G1 X88.397 Y92.105 E.01527
G1 X88.017 Y91.751 E.01529
G1 X87.714 Y91.331 E.01528
G1 X87.498 Y90.859 E.01529
G1 X87.379 Y90.355 E.01527
G1 X87.36 Y89.836 E.01529
G1 X87.442 Y89.324 E.01529
G1 X87.623 Y88.839 E.01528
G1 X87.881 Y88.415 E.01461
G1 X88.222 Y88.041 E.01491
G1 X88.642 Y87.73 E.01541
G1 X89.089 Y87.526 E.01448
; WIPE_START
G1 F10342.907
G1 X88.642 Y87.73 E-.18675
G1 X88.233 Y88.033 E-.19325
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z1.9 I.595 J1.062 P1  F42000
G1 X90.234 Y86.911 Z1.9
G1 Z1.5
G1 E.4 F1800
; LINE_WIDTH: 0.580662
G1 F7252.56
M204 S6000
G1 X89.758 Y86.91 E.02001
G1 X89.185 Y87.01 E.02444
G1 X88.49 Y87.292 E.03155
G1 X87.979 Y87.647 E.02614
G1 X87.559 Y88.087 E.02557
G1 X87.233 Y88.6 E.02554
G1 X87.013 Y89.166 E.02554
G1 X86.907 Y89.765 E.02557
G1 X86.921 Y90.373 E.02557
G1 X87.053 Y90.967 E.02554
G1 X87.298 Y91.523 E.02556
G1 X87.647 Y92.021 E.02556
G1 X88.087 Y92.441 E.02557
G1 X88.6 Y92.767 E.02554
G1 X89.167 Y92.987 E.02556
G1 X89.765 Y93.093 E.02555
G1 X90.373 Y93.079 E.02555
G1 X90.967 Y92.947 E.02557
G1 X91.523 Y92.702 E.02555
G1 X92.021 Y92.352 E.02557
G1 X92.441 Y91.913 E.02554
G1 X92.767 Y91.4 E.02555
G1 X92.987 Y90.833 E.02555
G1 X93.093 Y90.235 E.02555
G1 X93.072 Y89.544 E.02902
G1 X92.947 Y89.033 E.02214
G1 X92.702 Y88.477 E.02552
G1 X92.352 Y87.979 E.02558
G1 X91.913 Y87.559 E.02556
G1 X91.4 Y87.233 E.02555
G1 X90.833 Y87.013 E.02555
G1 X90.293 Y86.921 E.02302
M204 S10000
G1 X90.295 Y86.462 F42000
; LINE_WIDTH: 0.41999
G1 F9253
M204 S6000
G1 X89.799 Y86.451 E.01461
G1 X89.435 Y86.481 E.01076
G1 X88.798 Y86.64 E.01936
G1 X88.165 Y86.939 E.02062
G1 X87.604 Y87.356 E.02061
G1 X87.134 Y87.874 E.02062
G1 X86.774 Y88.475 E.02063
G1 X86.539 Y89.133 E.02061
G1 X86.436 Y89.825 E.02062
G1 X86.47 Y90.524 E.02062
G1 X86.64 Y91.202 E.02061
G1 X86.939 Y91.835 E.02062
G1 X87.356 Y92.396 E.02062
G1 X87.874 Y92.866 E.02061
G1 X88.474 Y93.226 E.02062
G1 X89.133 Y93.461 E.02061
G1 X89.825 Y93.564 E.02062
G1 X90.524 Y93.53 E.02062
G1 X91.202 Y93.36 E.02062
G1 X91.835 Y93.061 E.02062
G1 X92.396 Y92.644 E.02061
G1 X92.866 Y92.126 E.02062
G1 X93.226 Y91.526 E.02061
G1 X93.461 Y90.867 E.02062
G1 X93.564 Y90.175 E.02062
G1 X93.532 Y89.522 E.01928
G1 X93.391 Y88.921 E.01817
G1 X93.115 Y88.281 E.02053
G1 X92.72 Y87.706 E.02057
G1 X92.22 Y87.22 E.02056
G1 X91.635 Y86.84 E.02055
G1 X90.987 Y86.582 E.02055
G1 X90.354 Y86.472 E.01896
; CHANGE_LAYER
; Z_HEIGHT: 1.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F10342.907
G1 X90.987 Y86.582 E-.24441
G1 X91.319 Y86.714 E-.13559
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 9/43
; update layer progress
M73 L9
M991 S0 P8 ;notify layer change
; OBJECT_ID: 346
M204 S10000
G17
G3 Z1.9 I-.119 J1.211 P1  F42000
G1 X95.962 Y87.172 Z1.9
G1 Z1.7
G1 E.4 F1800
; FEATURE: Support transition
; LINE_WIDTH: 0.42
G1 F3000
M204 S6000
G1 X98.631 Y87.172 E.07869
G1 X98.631 Y87.549 E.01111
G1 X96.131 Y87.549 E.07369
G1 X96.131 Y87.926 E.01111
G1 X98.631 Y87.926 E.07369
G1 X98.631 Y88.303 E.01111
G1 X96.131 Y88.303 E.07369
G1 X96.131 Y88.68 E.01111
G1 X98.631 Y88.68 E.07369
G1 X98.631 Y89.057 E.01111
G1 X96.131 Y89.057 E.07369
G1 X96.131 Y89.434 E.01111
G1 X98.631 Y89.434 E.07369
G1 X98.631 Y89.811 E.01111
G1 X96.131 Y89.811 E.07369
G1 X96.131 Y90.188 E.01111
G1 X98.631 Y90.188 E.07369
G1 X98.631 Y90.566 E.01111
G1 X96.131 Y90.566 E.07369
G1 X96.131 Y90.943 E.01111
G1 X98.631 Y90.943 E.07369
G1 X98.631 Y91.32 E.01111
G1 X96.131 Y91.32 E.07369
G1 X96.131 Y91.697 E.01111
G1 X98.631 Y91.697 E.07369
G1 X98.631 Y92.074 E.01111
G1 X96.131 Y92.074 E.07369
G1 X96.131 Y92.451 E.01111
G1 X98.631 Y92.451 E.07369
G1 X98.631 Y92.828 E.01111
G1 X95.962 Y92.828 E.07869
M204 S10000
G1 X98.61 Y95.676 F42000
; FEATURE: Support
G1 F6662
M204 S6000
G1 X98.61 Y93.276 E.07075
G1 X96.153 Y93.276 E.07242
G1 X96.153 Y95.733 E.07242
G1 X98.61 Y95.733 E.07242
; WIPE_START
G1 F9000
G1 X97.61 Y95.733 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I-1.019 J-.665 P1  F42000
G1 X95.733 Y98.61 Z2.1
G1 Z1.7
G1 E.4 F1800
G1 F6662
M204 S6000
G1 X93.276 Y98.61 E.07242
G1 X93.276 Y96.153 E.07242
G1 X95.733 Y96.153 E.07242
G1 X95.733 Y98.553 E.07075
M204 S10000
G1 X93.047 Y96.222 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X87.123 Y96.222 E.17461
G1 X87.123 Y96.599 E.01111
G1 X92.877 Y96.599 E.16961
G1 X92.877 Y96.976 E.01111
G1 X87.123 Y96.976 E.16961
G1 X87.123 Y97.353 E.01111
G1 X92.877 Y97.353 E.16961
G1 X92.877 Y97.73 E.01111
G1 X87.123 Y97.73 E.16961
G1 X87.123 Y98.107 E.01111
G1 X92.877 Y98.107 E.16961
G1 X92.877 Y98.484 E.01111
G1 X86.953 Y98.484 E.17461
M204 S10000
G1 X86.724 Y98.553 F42000
; FEATURE: Support
G1 F6662
M204 S6000
G1 X86.724 Y96.153 E.07075
G1 X84.267 Y96.153 E.07241
G1 X84.267 Y98.61 E.07242
G1 X86.724 Y98.61 E.07241
M204 S10000
G1 X83.847 Y95.733 F42000
G1 F6662
M204 S6000
G1 X81.39 Y95.733 E.07241
G1 X81.39 Y93.276 E.07242
G1 X83.847 Y93.276 E.07241
G1 X83.847 Y95.676 E.07075
M204 S10000
G1 X81.199 Y92.828 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X83.869 Y92.828 E.07869
G1 X83.869 Y92.451 E.01111
G1 X81.369 Y92.451 E.07369
G1 X81.369 Y92.074 E.01111
G1 X83.869 Y92.074 E.07369
G1 X83.869 Y91.697 E.01111
G1 X81.369 Y91.697 E.07369
G1 X81.369 Y91.32 E.01111
G1 X83.869 Y91.32 E.07369
G1 X83.869 Y90.943 E.01111
G1 X81.369 Y90.943 E.07369
G1 X81.369 Y90.566 E.01111
G1 X83.869 Y90.566 E.07369
G1 X83.869 Y90.188 E.01111
G1 X81.369 Y90.188 E.07369
G1 X81.369 Y89.811 E.01111
G1 X83.869 Y89.811 E.07369
G1 X83.869 Y89.434 E.01111
G1 X81.369 Y89.434 E.07369
G1 X81.369 Y89.057 E.01111
G1 X83.869 Y89.057 E.07369
G1 X83.869 Y88.68 E.01111
M73 P48 R8
G1 X81.369 Y88.68 E.07369
G1 X81.369 Y88.303 E.01111
G1 X83.869 Y88.303 E.07369
G1 X83.869 Y87.926 E.01111
G1 X81.369 Y87.926 E.07369
G1 X81.369 Y87.549 E.01111
G1 X83.869 Y87.549 E.07369
G1 X83.869 Y87.172 E.01111
G1 X81.199 Y87.172 E.07869
M204 S10000
G1 X83.847 Y86.724 F42000
; FEATURE: Support
G1 F6662
M204 S6000
G1 X81.39 Y86.724 E.07241
G1 X81.39 Y84.267 E.07241
G1 X83.847 Y84.267 E.07241
G1 X83.847 Y86.668 E.07075
M204 S10000
G1 X86.724 Y83.847 F42000
G1 F6662
M204 S6000
G1 X84.267 Y83.847 E.07241
G1 X84.267 Y81.39 E.07241
G1 X86.724 Y81.39 E.07241
G1 X86.724 Y83.791 E.07075
M204 S10000
G1 X86.953 Y83.778 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X92.877 Y83.778 E.17461
G1 X92.877 Y83.401 E.01111
G1 X87.123 Y83.401 E.16961
G1 X87.123 Y83.024 E.01111
G1 X92.877 Y83.024 E.16961
G1 X92.877 Y82.647 E.01111
G1 X87.123 Y82.647 E.16961
G1 X87.123 Y82.27 E.01111
G1 X92.877 Y82.27 E.16961
G1 X92.877 Y81.893 E.01111
G1 X87.123 Y81.893 E.16961
G1 X87.123 Y81.516 E.01111
G1 X93.047 Y81.516 E.17461
M204 S10000
G1 X95.733 Y83.791 F42000
; FEATURE: Support
G1 F6662
M204 S6000
G1 X95.733 Y81.39 E.07075
G1 X93.276 Y81.39 E.07242
G1 X93.276 Y83.847 E.07241
G1 X95.733 Y83.847 E.07242
M204 S10000
G1 X95.924 Y84.533 F42000
M106 S127.5
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X84.246 Y84.533 E.34419
G1 X84.246 Y84.91 E.01111
G1 X87.803 Y84.91 E.10485
G1 X87.391 Y85.106 E.01345
G1 X87.081 Y85.287 E.01059
G1 X84.246 Y85.287 E.08355
G1 X84.246 Y85.664 E.01111
G1 X86.545 Y85.664 E.06777
G1 X86.118 Y86.041 E.01679
G1 X84.246 Y86.041 E.05519
G1 X84.246 Y86.418 E.01111
G1 X85.768 Y86.418 E.04488
G1 X85.476 Y86.795 E.01406
G1 X84.246 Y86.795 E.03627
G1 X84.246 Y87.172 E.01111
G1 X85.23 Y87.172 E.02902
G1 X85.027 Y87.549 E.01263
G1 X84.246 Y87.549 E.02303
G1 X84.246 Y87.926 E.01111
G1 X84.858 Y87.926 E.01804
G1 X84.722 Y88.303 E.01181
G1 X84.246 Y88.303 E.01403
G1 X84.246 Y88.68 E.01111
G1 X84.614 Y88.68 E.01086
G1 X84.537 Y89.057 E.01135
G1 X84.246 Y89.057 E.00858
G1 X84.246 Y89.434 E.01111
G1 X84.484 Y89.434 E.00702
G1 X84.459 Y89.812 E.01114
G1 X84.246 Y89.812 E.00629
G1 X84.246 Y90.189 E.01111
G1 X84.459 Y90.189 E.00629
G1 X84.484 Y90.566 E.01114
G1 X84.246 Y90.566 E.00702
G1 X84.246 Y90.943 E.01111
G1 X84.537 Y90.943 E.00858
G1 X84.614 Y91.32 E.01135
G1 X84.246 Y91.32 E.01086
G1 X84.246 Y91.697 E.01111
G1 X84.722 Y91.697 E.01403
G1 X84.858 Y92.074 E.01181
G1 X84.246 Y92.074 E.01804
G1 X84.246 Y92.451 E.01111
G1 X85.027 Y92.451 E.02303
G1 X85.23 Y92.828 E.01263
G1 X84.246 Y92.828 E.02902
G1 X84.246 Y93.205 E.01111
G1 X85.476 Y93.205 E.03627
G1 X85.768 Y93.582 E.01406
G1 X84.246 Y93.582 E.04488
G1 X84.246 Y93.959 E.01111
G1 X86.118 Y93.959 E.05519
G1 X86.545 Y94.336 E.01679
G1 X84.246 Y94.336 E.06778
G1 X84.246 Y94.713 E.01111
G1 X87.081 Y94.713 E.08355
G1 X87.391 Y94.893 E.01058
G1 X87.803 Y95.09 E.01346
G1 X84.246 Y95.09 E.10485
G1 X84.246 Y95.468 E.01111
G1 X95.754 Y95.468 E.33919
G1 X95.754 Y95.09 E.01111
G1 X92.197 Y95.09 E.10485
G1 X92.609 Y94.894 E.01346
G1 X92.919 Y94.713 E.01058
G1 X95.754 Y94.713 E.08355
G1 X95.754 Y94.336 E.01111
G1 X93.455 Y94.336 E.06778
G1 X93.882 Y93.959 E.01679
G1 X95.754 Y93.959 E.0552
G1 X95.754 Y93.582 E.01111
G1 X94.232 Y93.582 E.04488
G1 X94.524 Y93.205 E.01406
G1 X95.754 Y93.205 E.03627
G1 X95.754 Y92.828 E.01111
G1 X94.77 Y92.828 E.02902
G1 X94.973 Y92.451 E.01263
G1 X95.754 Y92.451 E.02303
G1 X95.754 Y92.074 E.01111
G1 X95.142 Y92.074 E.01804
G1 X95.278 Y91.697 E.01181
G1 X95.754 Y91.697 E.01404
G1 X95.754 Y91.32 E.01111
G1 X95.386 Y91.32 E.01086
G1 X95.463 Y90.943 E.01135
G1 X95.754 Y90.943 E.00858
G1 X95.754 Y90.566 E.01111
G1 X95.516 Y90.566 E.00702
G1 X95.541 Y90.189 E.01114
G1 X95.754 Y90.189 E.00629
G1 X95.754 Y89.812 E.01111
G1 X95.541 Y89.812 E.00629
G1 X95.516 Y89.434 E.01114
G1 X95.754 Y89.434 E.00702
G1 X95.754 Y89.057 E.01111
G1 X95.463 Y89.057 E.00858
G1 X95.386 Y88.68 E.01135
G1 X95.754 Y88.68 E.01086
G1 X95.754 Y88.303 E.01111
G1 X95.278 Y88.303 E.01403
G1 X95.142 Y87.926 E.01181
G1 X95.754 Y87.926 E.01804
G1 X95.754 Y87.549 E.01111
G1 X94.973 Y87.549 E.02303
G1 X94.77 Y87.172 E.01263
G1 X95.754 Y87.172 E.02902
G1 X95.754 Y86.795 E.01111
G1 X94.524 Y86.795 E.03627
G1 X94.232 Y86.418 E.01406
G1 X95.754 Y86.418 E.04488
G1 X95.754 Y86.041 E.01111
G1 X93.882 Y86.041 E.05519
G1 X93.455 Y85.664 E.01679
G1 X95.754 Y85.664 E.06777
G1 X95.754 Y85.287 E.01111
G1 X92.919 Y85.287 E.08355
G1 X92.609 Y85.107 E.01058
G1 X92.197 Y84.91 E.01346
G1 X95.924 Y84.91 E.10985
M204 S10000
G1 X98.61 Y86.668 F42000
; FEATURE: Support
G1 F6662
M204 S6000
G1 X98.61 Y84.267 E.07075
G1 X96.153 Y84.267 E.07242
G1 X96.153 Y86.724 E.07241
G1 X98.61 Y86.724 E.07242
; WIPE_START
G1 F9000
G1 X97.61 Y86.724 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I.353 J-1.165 P1  F42000
G1 X90.265 Y84.499 Z2.1
G1 Z1.7
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X89.724 Y84.5 E.00262
G1 X89.187 Y84.553 E.00262
G1 X88.656 Y84.659 E.00262
G1 X88.14 Y84.817 E.00262
G1 X87.65 Y85.019 E.00257
G1 X87.173 Y85.274 E.00262
G1 X86.715 Y85.58 E.00267
G1 X86.306 Y85.916 E.00257
G1 X85.916 Y86.306 E.00267
G1 X85.573 Y86.724 E.00262
G1 X85.274 Y87.174 E.00262
G1 X85.019 Y87.65 E.00262
G1 X84.813 Y88.15 E.00262
G1 X84.657 Y88.667 E.00262
G1 X84.552 Y89.197 E.00262
G1 X84.499 Y89.735 E.00262
G1 X84.5 Y90.276 E.00262
G1 X84.553 Y90.813 E.00262
G1 X84.659 Y91.344 E.00262
G1 X84.817 Y91.86 E.00262
G1 X85.024 Y92.36 E.00262
G1 X85.279 Y92.836 E.00262
G1 X85.58 Y93.285 E.00262
G1 X85.923 Y93.702 E.00262
G1 X86.306 Y94.084 E.00262
G1 X86.724 Y94.427 E.00262
G1 X87.173 Y94.726 E.00262
G1 X87.65 Y94.981 E.00262
G1 X88.15 Y95.187 E.00262
G1 X88.667 Y95.343 E.00262
G1 X89.198 Y95.448 E.00262
G1 X89.735 Y95.501 E.00262
G1 X90.276 Y95.5 E.00262
G1 X90.813 Y95.447 E.00262
G1 X91.344 Y95.341 E.00262
G1 X91.86 Y95.183 E.00262
G1 X92.36 Y94.976 E.00262
G1 X92.836 Y94.721 E.00262
G1 X93.285 Y94.42 E.00262
G1 X93.702 Y94.077 E.00262
G1 X94.084 Y93.694 E.00262
G1 X94.427 Y93.276 E.00262
G1 X94.726 Y92.827 E.00262
G1 X94.981 Y92.35 E.00262
G1 X95.187 Y91.85 E.00262
G1 X95.343 Y91.333 E.00262
G1 X95.448 Y90.803 E.00262
G1 X95.501 Y90.265 E.00262
G1 X95.5 Y89.724 E.00262
G1 X95.447 Y89.187 E.00262
G1 X95.341 Y88.657 E.00262
G1 X95.183 Y88.14 E.00262
G1 X94.976 Y87.64 E.00262
G1 X94.721 Y87.164 E.00262
G1 X94.42 Y86.715 E.00262
G1 X94.077 Y86.298 E.00262
G1 X93.694 Y85.916 E.00262
G1 X93.276 Y85.573 E.00262
G1 X92.826 Y85.274 E.00262
G1 X92.35 Y85.019 E.00262
G1 X91.85 Y84.813 E.00262
G1 X91.333 Y84.657 E.00262
G1 X90.802 Y84.552 E.00262
G1 X90.265 Y84.499 E.00262
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.271 Y84.354 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X89.721 Y84.354 E.00267
G1 X89.168 Y84.408 E.00269
G1 X88.625 Y84.517 E.00269
G1 X88.097 Y84.677 E.00267
G1 X87.585 Y84.889 E.00269
G1 X87.095 Y85.15 E.00269
G1 X86.634 Y85.458 E.00269
G1 X86.202 Y85.812 E.00271
G1 X85.81 Y86.205 E.00269
G1 X85.458 Y86.634 E.00269
G1 X85.15 Y87.095 E.00269
G1 X84.889 Y87.585 E.00269
G1 X84.677 Y88.098 E.00269
G1 X84.516 Y88.628 E.00269
G1 X84.408 Y89.168 E.00267
G1 X84.354 Y89.721 E.00269
G1 X84.354 Y90.275 E.00269
G1 X84.408 Y90.827 E.00269
G1 X84.517 Y91.375 E.00271
G1 X84.677 Y91.903 E.00267
G1 X84.89 Y92.419 E.00271
G1 X85.15 Y92.905 E.00267
G1 X85.458 Y93.366 E.00269
G1 X85.812 Y93.798 E.00271
G1 X86.202 Y94.188 E.00267
G1 X86.631 Y94.54 E.00269
G1 X87.095 Y94.85 E.00271
G1 X87.585 Y95.111 E.00269
G1 X88.097 Y95.323 E.00269
G1 X88.624 Y95.483 E.00267
G1 X89.168 Y95.592 E.00269
G1 X89.721 Y95.646 E.00269
G1 X90.279 Y95.646 E.00271
G1 X90.832 Y95.592 E.00269
G1 X91.372 Y95.484 E.00267
G1 X91.903 Y95.323 E.00269
G1 X92.415 Y95.111 E.00269
G1 X92.905 Y94.85 E.00269
G1 X93.366 Y94.542 E.00269
G1 X93.795 Y94.19 E.00269
G1 X94.187 Y93.798 E.00269
G1 X94.542 Y93.366 E.00271
G1 X94.85 Y92.905 E.00269
G1 X95.11 Y92.419 E.00267
G1 X95.323 Y91.903 E.00271
G1 X95.484 Y91.371 E.00269
G1 X95.592 Y90.828 E.00269
G1 X95.646 Y90.276 E.00269
G1 X95.646 Y89.721 E.00269
G1 X95.592 Y89.172 E.00267
G1 X95.484 Y88.628 E.00269
G1 X95.323 Y88.097 E.00269
G1 X95.11 Y87.581 E.00271
G1 X94.85 Y87.095 E.00267
G1 X94.539 Y86.631 E.00271
G1 X94.188 Y86.202 E.00269
G1 X93.795 Y85.81 E.00269
G1 X93.366 Y85.458 E.00269
G1 X92.905 Y85.15 E.00269
G1 X92.415 Y84.889 E.00269
G1 X91.906 Y84.678 E.00267
G1 X91.376 Y84.517 E.00269
G1 X90.827 Y84.408 E.00271
G1 X90.271 Y84.354 E.00271
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 4
; WIPE_START
G1 X90.827 Y84.408 E-.21255
G1 X91.26 Y84.494 E-.16745
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I-1.208 J-.144 P1  F42000
G1 X91.117 Y85.69 Z2.1
G1 Z1.7
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F6662
M204 S6000
G1 X91.501 Y85.806 E.01274
G1 X91.905 Y85.973 E.01391
G1 X92.29 Y86.179 E.0139
G1 X92.653 Y86.422 E.01391
G1 X92.991 Y86.7 E.01391
G1 X93.301 Y87.009 E.01391
G1 X93.578 Y87.347 E.0139
G1 X93.821 Y87.71 E.01391
G1 X94.027 Y88.095 E.0139
G1 X94.194 Y88.499 E.01391
G1 X94.321 Y88.918 E.01391
G1 X94.406 Y89.346 E.01391
G1 X94.449 Y89.781 E.01391
G1 X94.449 Y90.219 E.01391
G1 X94.406 Y90.654 E.01391
G1 X94.321 Y91.082 E.01391
G1 X94.194 Y91.501 E.01391
G1 X94.027 Y91.904 E.01391
G1 X93.821 Y92.29 E.01391
G1 X93.578 Y92.653 E.0139
G1 X93.3 Y92.991 E.01391
G1 X92.991 Y93.3 E.01391
G1 X92.653 Y93.578 E.01391
G1 X92.29 Y93.821 E.0139
G1 X91.905 Y94.027 E.01391
G1 X91.501 Y94.194 E.01391
G1 X91.082 Y94.321 E.0139
G1 X90.653 Y94.406 E.01392
G1 X90.219 Y94.449 E.0139
G1 X89.781 Y94.449 E.01391
G1 X89.346 Y94.406 E.01391
G1 X88.918 Y94.321 E.01391
G1 X88.499 Y94.194 E.01391
G1 X88.095 Y94.027 E.01391
G1 X87.71 Y93.821 E.01391
G1 X87.347 Y93.578 E.01391
G1 X87.009 Y93.3 E.01391
G1 X86.7 Y92.991 E.01391
G1 X86.422 Y92.653 E.01391
G1 X86.179 Y92.29 E.0139
G1 X85.973 Y91.904 E.01391
G1 X85.806 Y91.501 E.01391
G1 X85.679 Y91.082 E.0139
G1 X85.594 Y90.653 E.01392
G1 X85.551 Y90.219 E.0139
G1 X85.551 Y89.781 E.01391
G1 X85.594 Y89.346 E.0139
G1 X85.679 Y88.918 E.01391
G1 X85.806 Y88.499 E.01391
G1 X85.973 Y88.095 E.01391
G1 X86.179 Y87.71 E.01391
G1 X86.422 Y87.347 E.0139
G1 X86.7 Y87.009 E.01391
G1 X87.009 Y86.7 E.01391
G1 X87.347 Y86.422 E.01391
G1 X87.71 Y86.18 E.0139
G1 X88.096 Y85.973 E.01391
G1 X88.499 Y85.806 E.01391
G1 X88.918 Y85.679 E.01391
G1 X89.346 Y85.594 E.01391
G1 X89.783 Y85.551 E.01394
G1 X90.154 Y85.549 E.01183
M106 S124.95
M106 S127.5
G1 X90.656 Y85.594 E.01604
M106 S124.95
M106 S127.5
G1 X91.059 Y85.675 E.01308
M106 S124.95
M106 S127.5
; COOLING_NODE: 4
M204 S250
G1 X91.231 Y85.315 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.633 Y85.437 E.01237
G1 X92.072 Y85.619 E.01402
G1 X92.492 Y85.843 E.01401
G1 X92.887 Y86.107 E.01402
G1 X93.255 Y86.409 E.01402
G1 X93.591 Y86.745 E.01402
G1 X93.893 Y87.113 E.01402
G1 X94.157 Y87.508 E.01402
G1 X94.381 Y87.928 E.01401
G1 X94.564 Y88.367 E.01402
G1 X94.702 Y88.822 E.01402
G1 X94.794 Y89.289 E.01402
G1 X94.841 Y89.762 E.01402
G1 X94.841 Y90.238 E.01402
G1 X94.794 Y90.711 E.01402
G1 X94.702 Y91.178 E.01402
G1 X94.564 Y91.633 E.01402
G1 X94.381 Y92.072 E.01402
G1 X94.157 Y92.492 E.01402
G1 X93.893 Y92.887 E.01401
G1 X93.591 Y93.255 E.01402
G1 X93.255 Y93.591 E.01402
G1 X92.887 Y93.893 E.01402
G1 X92.492 Y94.157 E.01401
G1 X92.072 Y94.381 E.01402
G1 X91.633 Y94.564 E.01402
G1 X91.178 Y94.702 E.01401
G1 X90.711 Y94.794 E.01402
G1 X90.238 Y94.841 E.01402
G1 X89.762 Y94.841 E.01402
G1 X89.289 Y94.794 E.01402
G1 X88.822 Y94.702 E.01402
G1 X88.367 Y94.564 E.01402
G1 X87.928 Y94.381 E.01402
G1 X87.508 Y94.157 E.01402
G1 X87.113 Y93.893 E.01402
G1 X86.745 Y93.591 E.01402
G1 X86.409 Y93.255 E.01402
G1 X86.107 Y92.887 E.01402
G1 X85.843 Y92.492 E.01401
G1 X85.619 Y92.072 E.01402
G1 X85.436 Y91.633 E.01402
G1 X85.298 Y91.178 E.01401
G1 X85.206 Y90.711 E.01403
G1 X85.159 Y90.238 E.01402
G1 X85.159 Y89.762 E.01402
G1 X85.206 Y89.289 E.01402
G1 X85.298 Y88.822 E.01402
G1 X85.436 Y88.367 E.01402
G1 X85.619 Y87.928 E.01402
G1 X85.843 Y87.508 E.01402
G1 X86.107 Y87.113 E.01401
G1 X86.409 Y86.745 E.01403
G1 X86.745 Y86.409 E.01402
G1 X87.113 Y86.107 E.01402
G1 X87.508 Y85.843 E.01401
G1 X87.928 Y85.619 E.01402
G1 X88.367 Y85.436 E.01402
G1 X88.822 Y85.298 E.01402
G1 X89.289 Y85.206 E.01402
G1 X89.763 Y85.159 E.01403
G1 X90.171 Y85.157 E.01204
G1 X90.712 Y85.206 E.01601
G1 X91.174 Y85.298 E.01387
M106 S124.95
; WIPE_START
M204 S6000
G1 X91.633 Y85.437 E-.18231
G1 X92.072 Y85.619 E-.18075
G1 X92.112 Y85.64 E-.01694
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z2.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z2.1 F4000
            G39.3 S1
            G0 Z2.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.061 Y86.633 F42000
G1 Z1.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.41999
G1 F6662
M204 S6000
G1 X90.802 Y86.729 E.02203
G1 X91.427 Y86.949 E.01954
G1 X91.995 Y87.286 E.01945
G1 X92.486 Y87.728 E.01946
G1 X92.881 Y88.256 E.01945
G1 X93.166 Y88.852 E.01947
M73 P49 R8
G1 X93.33 Y89.492 E.01947
G1 X93.365 Y90.151 E.01946
G1 X93.271 Y90.805 E.01947
G1 X93.051 Y91.427 E.01946
G1 X92.714 Y91.995 E.01945
G1 X92.272 Y92.486 E.01947
G1 X91.744 Y92.882 E.01945
G1 X91.148 Y93.167 E.01947
G1 X90.508 Y93.33 E.01945
G1 X89.849 Y93.365 E.01945
G1 X89.195 Y93.271 E.01947
G1 X88.573 Y93.051 E.01946
G1 X88.005 Y92.714 E.01946
G1 X87.514 Y92.272 E.01947
G1 X87.118 Y91.744 E.01946
G1 X86.834 Y91.148 E.01945
G1 X86.67 Y90.508 E.01946
G1 X86.635 Y89.832 E.01996
G1 X86.729 Y89.195 E.01897
G1 X86.949 Y88.573 E.01946
G1 X87.286 Y88.005 E.01946
G1 X87.728 Y87.514 E.01947
G1 X88.256 Y87.118 E.01945
G1 X88.852 Y86.834 E.01947
G1 X89.492 Y86.67 E.01946
G1 X90.001 Y86.637 E.01503
; WIPE_START
G1 F10342.907
G1 X89.492 Y86.67 E-.19381
G1 X89.017 Y86.791 E-.18619
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I-.568 J-1.076 P1  F42000
G1 X86.376 Y88.186 Z2.1
G1 Z1.7
G1 E.4 F1800
G1 F6662
M204 S6000
G1 X86.091 Y88.927 E.02341
G1 X85.957 Y89.711 E.02343
G1 X85.978 Y90.505 E.02342
G1 X86.154 Y91.28 E.02341
G1 X86.478 Y92.005 E.02341
G1 X86.937 Y92.654 E.02341
G1 X87.513 Y93.201 E.02343
G1 X88.186 Y93.624 E.02342
G1 X88.928 Y93.909 E.02342
G1 X89.711 Y94.043 E.02342
G1 X90.505 Y94.022 E.02341
G1 X91.28 Y93.846 E.02341
G1 X92.006 Y93.522 E.02343
G1 X92.654 Y93.063 E.02341
G1 X93.201 Y92.486 E.02343
G1 X93.624 Y91.814 E.02341
G1 X93.909 Y91.073 E.02341
G1 X94.043 Y90.289 E.02342
G1 X94.019 Y89.438 E.0251
G1 X93.846 Y88.72 E.02177
G1 X93.522 Y87.994 E.02343
G1 X93.063 Y87.346 E.02341
G1 X92.487 Y86.799 E.02342
G1 X91.814 Y86.376 E.02341
G1 X91.073 Y86.091 E.02341
G1 X90.19 Y85.95 E.02633
G1 X89.487 Y85.978 E.02076
G1 X88.72 Y86.154 E.02318
G1 X87.994 Y86.478 E.02342
G1 X87.346 Y86.937 E.02341
G1 X86.799 Y87.513 E.02342
G1 X86.408 Y88.135 E.02165
M204 S10000
G1 X86.415 Y89.043 F42000
; LINE_WIDTH: 0.353424
G1 F6662
M204 S6000
G1 X86.297 Y89.761 E.01766
G1 X86.322 Y90.488 E.01766
G1 X86.488 Y91.196 E.01765
G1 X86.788 Y91.859 E.01765
G1 X87.213 Y92.449 E.01765
G1 X87.744 Y92.946 E.01766
G1 X88.362 Y93.33 E.01765
G1 X89.043 Y93.585 E.01765
G1 X89.761 Y93.703 E.01766
G1 X90.488 Y93.678 E.01765
G1 X91.196 Y93.512 E.01765
G1 X91.859 Y93.211 E.01766
G1 X92.449 Y92.787 E.01765
G1 X92.946 Y92.256 E.01766
G1 X93.33 Y91.638 E.01765
G1 X93.585 Y90.957 E.01765
G1 X93.703 Y90.239 E.01766
G1 X93.674 Y89.455 E.01904
G1 X93.512 Y88.803 E.01629
G1 X93.211 Y88.141 E.01766
G1 X92.787 Y87.551 E.01764
G1 X92.256 Y87.054 E.01765
G1 X91.638 Y86.67 E.01765
G1 X90.957 Y86.415 E.01765
G1 X90.143 Y86.292 E.01998
G1 X89.506 Y86.322 E.01548
G1 X88.804 Y86.488 E.0175
G1 X88.141 Y86.789 E.01766
G1 X87.551 Y87.213 E.01765
G1 X87.054 Y87.744 E.01766
G1 X86.67 Y88.362 E.01765
G1 X86.436 Y88.987 E.01619
; WIPE_START
G1 F12560.236
G1 X86.67 Y88.362 E-.25357
G1 X86.846 Y88.08 E-.12643
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.1 I-1.084 J.552 P1  F42000
G1 X89.298 Y92.892 Z2.1
G1 Z1.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F6662
M204 S6000
G1 X88.997 Y92.803 E.00999
G1 X88.727 Y92.691 E.00929
G1 X88.47 Y92.553 E.00929
G1 X88.227 Y92.391 E.0093
G1 X87.898 Y92.102 E.01392
G1 X92.102 Y87.898 E.18921
G1 X91.773 Y87.609 E.01393
G1 X91.531 Y87.447 E.00929
G1 X91.273 Y87.309 E.00929
G1 X91.003 Y87.197 E.00931
G1 X90.724 Y87.112 E.00928
G1 X90.444 Y87.057 E.00907
G1 X90.06 Y87.025 E.01225
G1 X89.563 Y87.055 E.01585
G1 X89.277 Y87.112 E.0093
G1 X88.997 Y87.197 E.00929
G1 X88.727 Y87.309 E.00931
G1 X88.47 Y87.447 E.00928
G1 X88.226 Y87.609 E.00931
G1 X87.898 Y87.898 E.01392
G1 X92.102 Y92.102 E.18921
G1 X92.391 Y91.773 E.01393
G1 X92.554 Y91.53 E.0093
G1 X92.691 Y91.273 E.00928
G1 X92.803 Y91.003 E.0093
G1 X92.892 Y90.702 E.00998
; CHANGE_LAYER
; Z_HEIGHT: 1.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X92.803 Y91.003 E-.11918
G1 X92.691 Y91.273 E-.11112
G1 X92.554 Y91.53 E-.11082
G1 X92.497 Y91.615 E-.03889
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 10/43
; update layer progress
M73 L10
M991 S0 P9 ;notify layer change
M106 S117.3
; OBJECT_ID: 346
M204 S10000
G17
G3 Z2.1 I.94 J.773 P1  F42000
G1 X95.467 Y88.002 Z2.1
G1 Z1.9
G1 E.4 F1800
; FEATURE: Support interface
; LINE_WIDTH: 0.42
G1 F4800
M204 S6000
G1 X95.467 Y84.246 E.1107
G1 X95.09 Y84.246 E.01111
G1 X95.09 Y86.842 E.07653
G1 X94.713 Y86.304 E.01938
G1 X94.713 Y84.246 E.06065
G1 X94.336 Y84.246 E.01111
G1 X94.336 Y85.868 E.0478
G1 X93.959 Y85.505 E.01543
G1 X93.959 Y84.246 E.03711
G1 X93.582 Y84.246 E.01111
G1 X93.582 Y85.197 E.02805
G1 X93.205 Y84.94 E.01346
G1 X93.205 Y84.246 E.02045
G1 X92.877 Y84.246 E.00966
G1 X92.877 Y81.369 E.0848
G1 X92.828 Y81.369 E.00145
G1 X92.828 Y84.718 E.09871
G1 X92.451 Y84.534 E.01236
G1 X92.451 Y81.369 E.0933
G1 X92.074 Y81.369 E.01111
G1 X92.074 Y84.38 E.08875
G1 X91.697 Y84.254 E.01171
G1 X91.697 Y81.369 E.08505
G1 X91.32 Y81.369 E.01111
G1 X91.32 Y84.157 E.08219
G1 X90.943 Y84.084 E.01132
G1 X90.943 Y81.369 E.08003
G1 X90.566 Y81.369 E.01111
G1 X90.566 Y84.036 E.0786
G1 X90.188 Y84.013 E.01113
G1 X90.188 Y81.369 E.07793
G1 X89.811 Y81.369 E.01111
G1 X89.811 Y84.013 E.07793
G1 X89.434 Y84.036 E.01113
G1 X89.434 Y81.369 E.0786
G1 X89.057 Y81.369 E.01111
G1 X89.057 Y84.084 E.08003
G1 X88.68 Y84.157 E.01132
G1 X88.68 Y81.369 E.08219
G1 X88.303 Y81.369 E.01111
G1 X88.303 Y84.255 E.08506
G1 X87.926 Y84.38 E.01171
G1 X87.926 Y81.369 E.08875
G1 X87.549 Y81.369 E.01111
G1 X87.549 Y84.534 E.0933
G1 X87.172 Y84.718 E.01236
G1 X87.172 Y81.369 E.09871
G1 X87.123 Y81.369 E.00145
G1 X87.123 Y84.246 E.0848
G1 X86.795 Y84.246 E.00967
G1 X86.795 Y84.94 E.02045
G1 X86.418 Y85.197 E.01346
G1 X86.418 Y84.246 E.02805
G1 X86.041 Y84.246 E.01111
G1 X86.041 Y85.505 E.03711
G1 X85.664 Y85.868 E.01543
G1 X85.664 Y84.246 E.04781
G1 X85.287 Y84.246 E.01111
G1 X85.287 Y86.304 E.06065
G1 X84.91 Y86.843 E.01939
G1 X84.91 Y84.246 E.07654
G1 X84.532 Y84.246 E.01111
G1 X84.532 Y88.002 E.11071
; WIPE_START
G1 X84.532 Y87.002 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.457 J-1.128 P1  F42000
G1 X83.847 Y86.724 Z2.3
G1 Z1.9
G1 E.4 F1800
; FEATURE: Support
G1 F9000
M204 S6000
G1 X81.39 Y86.724 E.07241
G1 X81.39 Y84.267 E.07241
G1 X83.847 Y84.267 E.07241
G1 X83.847 Y86.668 E.07075
; WIPE_START
G1 X83.847 Y85.668 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.651 J1.028 P1  F42000
G1 X86.724 Y83.847 Z2.3
G1 Z1.9
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X84.267 Y83.847 E.07241
G1 X84.267 Y81.39 E.07241
G1 X86.724 Y81.39 E.07241
G1 X86.724 Y83.791 E.07075
; WIPE_START
G1 X86.724 Y82.791 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-.134 J1.21 P1  F42000
G1 X95.733 Y83.791 Z2.3
G1 Z1.9
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X95.733 Y81.39 E.07075
G1 X93.276 Y81.39 E.07242
G1 X93.276 Y83.847 E.07241
G1 X95.733 Y83.847 E.07242
; WIPE_START
G1 X94.733 Y83.847 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-.716 J.984 P1  F42000
G1 X98.61 Y86.668 Z2.3
G1 Z1.9
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X98.61 Y84.267 E.07075
G1 X96.153 Y84.267 E.07242
G1 X96.153 Y86.724 E.07241
G1 X98.61 Y86.724 E.07242
; WIPE_START
G1 X97.61 Y86.724 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-1.202 J-.19 P1  F42000
G1 X95.733 Y98.61 Z2.3
G1 Z1.9
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X93.276 Y98.61 E.07242
G1 X93.276 Y96.153 E.07242
G1 X95.733 Y96.153 E.07242
G1 X95.733 Y98.553 E.07075
; WIPE_START
G1 X95.733 Y97.553 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.651 J1.028 P1  F42000
G1 X98.61 Y95.733 Z2.3
G1 Z1.9
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X96.153 Y95.733 E.07242
G1 X96.153 Y93.276 E.07242
G1 X98.61 Y93.276 E.07242
G1 X98.61 Y95.676 E.07075
; WIPE_START
G1 X98.61 Y94.676 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I1.213 J-.094 P1  F42000
G1 X98.484 Y93.047 Z2.3
G1 Z1.9
G1 E.4 F1800
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X98.484 Y87.123 E.1746
G1 X98.107 Y87.123 E.01111
G1 X98.107 Y92.877 E.1696
G1 X97.73 Y92.877 E.01111
G1 X97.73 Y87.123 E.1696
G1 X97.353 Y87.123 E.01111
G1 X97.353 Y92.877 E.1696
G1 X96.976 Y92.877 E.01111
G1 X96.976 Y87.123 E.1696
G1 X96.599 Y87.123 E.01111
G1 X96.599 Y92.877 E.1696
G1 X96.222 Y92.877 E.01111
G1 X96.222 Y87.123 E.1696
G1 X95.845 Y87.123 E.01111
G1 X95.845 Y92.877 E.1696
G1 X95.754 Y92.877 E.00266
G1 X95.754 Y95.754 E.0848
G1 X95.467 Y95.754 E.00845
G1 X95.467 Y92.447 E.09748
G1 X95.287 Y92.819 E.0122
G1 X95.09 Y93.158 E.01153
G1 X95.09 Y95.754 E.07653
G1 X94.713 Y95.754 E.01111
G1 X94.713 Y93.697 E.06065
G1 X94.336 Y94.132 E.01699
G1 X94.336 Y95.754 E.0478
G1 X93.959 Y95.754 E.01111
G1 X93.959 Y94.495 E.03711
G1 X93.582 Y94.803 E.01434
G1 X93.582 Y95.754 E.02805
G1 X93.205 Y95.754 E.01111
G1 X93.205 Y95.06 E.02045
G1 X92.828 Y95.282 E.01289
G1 X92.828 Y98.631 E.09871
G1 X92.451 Y98.631 E.01111
G1 X92.451 Y95.466 E.0933
G1 X92.074 Y95.62 E.01201
G1 X92.074 Y98.631 E.08875
G1 X91.697 Y98.631 E.01111
G1 X91.697 Y95.746 E.08506
G1 X91.32 Y95.843 E.01148
G1 X91.32 Y98.631 E.08219
G1 X90.943 Y98.631 E.01111
G1 X90.943 Y95.916 E.08003
G1 X90.566 Y95.965 E.01121
G1 X90.566 Y98.631 E.0786
G1 X90.188 Y98.631 E.01111
G1 X90.188 Y95.987 E.07793
G1 X89.811 Y95.987 E.01111
G1 X89.811 Y98.631 E.07793
G1 X89.434 Y98.631 E.01111
G1 X89.434 Y95.964 E.0786
G1 X89.057 Y95.916 E.01121
G1 X89.057 Y98.631 E.08004
G1 X88.68 Y98.631 E.01111
G1 X88.68 Y95.843 E.08219
G1 X88.303 Y95.745 E.01148
G1 X88.303 Y98.631 E.08506
G1 X87.926 Y98.631 E.01111
G1 X87.926 Y95.62 E.08875
G1 X87.549 Y95.466 E.01201
G1 X87.549 Y98.631 E.09331
G1 X87.172 Y98.631 E.01111
G1 X87.172 Y95.282 E.09871
G1 X86.795 Y95.06 E.01289
G1 X86.795 Y95.754 E.02046
G1 X86.418 Y95.754 E.01111
G1 X86.418 Y94.803 E.02805
G1 X86.041 Y94.495 E.01434
G1 X86.041 Y95.754 E.03711
G1 X85.664 Y95.754 E.01111
G1 X85.664 Y94.132 E.04781
G1 X85.287 Y93.696 E.01699
G1 X85.287 Y95.754 E.06065
G1 X84.91 Y95.754 E.01111
G1 X84.91 Y93.157 E.07654
G1 X84.713 Y92.819 E.01153
G1 X84.532 Y92.447 E.01221
G1 X84.532 Y95.754 E.09749
G1 X84.246 Y95.754 E.00845
G1 X84.246 Y92.877 E.0848
G1 X84.155 Y92.877 E.00266
G1 X84.155 Y87.123 E.1696
G1 X83.778 Y87.123 E.01111
G1 X83.778 Y92.877 E.1696
G1 X83.401 Y92.877 E.01111
G1 X83.401 Y87.123 E.1696
G1 X83.024 Y87.123 E.01111
G1 X83.024 Y92.877 E.1696
G1 X82.647 Y92.877 E.01111
G1 X82.647 Y87.123 E.1696
G1 X82.27 Y87.123 E.01111
G1 X82.27 Y92.877 E.1696
G1 X81.893 Y92.877 E.01111
G1 X81.893 Y87.123 E.1696
G1 X81.516 Y87.123 E.01111
G1 X81.516 Y93.047 E.1746
; WIPE_START
G1 X81.516 Y92.047 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-1.024 J.658 P1  F42000
G1 X83.847 Y95.676 Z2.3
G1 Z1.9
G1 E.4 F1800
; FEATURE: Support
G1 F9000
M204 S6000
G1 X83.847 Y93.276 E.07075
G1 X81.39 Y93.276 E.07241
G1 X81.39 Y95.733 E.07242
G1 X83.847 Y95.733 E.07241
; WIPE_START
G1 X82.847 Y95.733 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-.716 J.984 P1  F42000
G1 X86.724 Y98.553 Z2.3
G1 Z1.9
M73 P50 R8
G1 E.4 F1800
G1 F9000
M204 S6000
G1 X86.724 Y96.153 E.07075
G1 X84.267 Y96.153 E.07241
G1 X84.267 Y98.61 E.07242
G1 X86.724 Y98.61 E.07241
; WIPE_START
G1 X85.724 Y98.61 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.571 J1.074 P1  F42000
G1 X95.493 Y93.415 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X95.493 Y95.493 E.01007
G1 X93.415 Y95.493 E.01007
G1 X93.353 Y95.276 E.00109
G1 X93.96 Y94.839 E.00363
G1 X94.415 Y94.428 E.00298
G1 X94.839 Y93.96 E.00306
G1 X95.276 Y93.353 E.00363
G1 X95.493 Y93.415 E.00109
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X95.193 Y94.01 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X94.625 Y94.643 E.00412
G1 X94.01 Y95.193 E.004
G1 X95.193 Y95.193 E.00574
G1 X95.193 Y94.01 E.00574
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X95.193 Y95.01 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-.306 J-1.178 P1  F42000
G1 X92.616 Y95.678 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X92.616 Y98.37 E.01305
G1 X87.384 Y98.37 E.02537
G1 X87.384 Y95.679 E.01305
G1 X87.885 Y95.884 E.00262
G1 X88.176 Y95.981 E.00149
G1 X88.771 Y96.131 E.00298
G1 X89.396 Y96.224 E.00307
G1 X90.01 Y96.253 E.00298
G1 X90.623 Y96.222 E.00298
G1 X91.23 Y96.131 E.00298
G1 X91.824 Y95.981 E.00297
G1 X92.115 Y95.884 E.00149
G1 X92.616 Y95.678 E.00262
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X92.316 Y96.135 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X92.316 Y98.07 E.00938
G1 X87.684 Y98.07 E.02246
G1 X87.684 Y96.135 E.00938
G1 X88.11 Y96.275 E.00218
G1 X88.735 Y96.43 E.00312
G1 X89.371 Y96.523 E.00312
G1 X89.987 Y96.553 E.00299
G1 X90.629 Y96.523 E.00312
G1 X91.266 Y96.43 E.00312
G1 X91.89 Y96.275 E.00312
G1 X92.316 Y96.135 E.00217
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X92.016 Y96.551 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X92.016 Y97.77 E.00591
G1 X87.984 Y97.77 E.01955
G1 X87.984 Y96.551 E.00591
G1 X88.351 Y96.652 E.00184
G1 X88.978 Y96.776 E.0031
G1 X89.345 Y96.822 E.00179
G1 X89.983 Y96.853 E.0031
G1 X90.353 Y96.844 E.00179
G1 X91.022 Y96.777 E.00326
G1 X91.649 Y96.652 E.0031
G1 X92.016 Y96.551 E.00184
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X91.716 Y96.944 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X91.716 Y97.47 E.00255
G1 X88.284 Y97.47 E.01664
G1 X88.284 Y96.944 E.00255
G1 X88.625 Y97.02 E.00169
G1 X89.278 Y97.116 E.0032
G1 X89.979 Y97.153 E.00341
G1 X90.723 Y97.116 E.00361
G1 X91.376 Y97.02 E.0032
G1 X91.716 Y96.944 E.00169
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X91.376 Y97.02 E-.13244
G1 X90.731 Y97.115 E-.24756
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.565 J-1.078 P1  F42000
G1 X84.807 Y94.01 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.357 Y94.625 E.004
G1 X85.99 Y95.193 E.00412
G1 X84.807 Y95.193 E.00573
G1 X84.807 Y94.01 E.00573
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.724 Y93.353 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.16 Y93.96 E.00363
G1 X85.572 Y94.415 E.00298
G1 X86.04 Y94.839 E.00306
G1 X86.647 Y95.276 E.00363
G1 X86.585 Y95.493 E.00109
G1 X84.507 Y95.493 E.01007
G1 X84.507 Y93.415 E.01007
G1 X84.724 Y93.353 E.00109
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.321 Y92.616 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X84.116 Y92.115 E.00262
G1 X84.019 Y91.824 E.00149
G1 X83.869 Y91.229 E.00297
G1 X83.778 Y90.622 E.00298
G1 X83.747 Y90.009 E.00297
G1 X83.776 Y89.396 E.00298
G1 X83.869 Y88.771 E.00307
G1 X84.019 Y88.176 E.00297
G1 X84.116 Y87.885 E.00149
G1 X84.321 Y87.384 E.00262
G1 X81.63 Y87.384 E.01305
G1 X81.63 Y92.616 E.02537
G1 X84.321 Y92.616 E.01305
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X83.865 Y92.316 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X83.725 Y91.89 E.00217
G1 X83.57 Y91.265 E.00312
G1 X83.477 Y90.629 E.00312
G1 X83.447 Y89.987 E.00312
G1 X83.48 Y89.345 E.00312
G1 X83.57 Y88.734 E.00299
G1 X83.725 Y88.11 E.00312
G1 X83.865 Y87.684 E.00218
G1 X81.93 Y87.684 E.00938
G1 X81.93 Y92.316 E.02246
G1 X83.865 Y92.316 E.00938
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X83.449 Y92.016 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X83.282 Y91.353 E.00331
G1 X83.219 Y90.989 E.00179
G1 X83.156 Y90.353 E.0031
G1 X83.147 Y89.983 E.00179
G1 X83.178 Y89.345 E.0031
G1 X83.224 Y88.977 E.0018
G1 X83.348 Y88.351 E.00309
G1 X83.449 Y87.984 E.00185
G1 X82.23 Y87.984 E.00591
G1 X82.23 Y92.016 E.01955
G1 X83.449 Y92.016 E.00591
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X83.056 Y91.716 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X82.928 Y91.071 E.00319
G1 X82.883 Y90.721 E.00171
G1 X82.847 Y90.021 E.0034
G1 X82.883 Y89.279 E.0036
G1 X82.928 Y88.929 E.00171
G1 X83.056 Y88.284 E.00319
G1 X82.53 Y88.284 E.00255
G1 X82.53 Y91.716 E.01664
G1 X83.056 Y91.716 E.00255
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X82.53 Y91.716 E-.1998
G1 X82.53 Y91.242 E-.18021
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I1.12 J.476 P1  F42000
G1 X84.507 Y86.585 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X84.507 Y84.507 E.01007
G1 X86.585 Y84.507 E.01007
G1 X86.647 Y84.724 E.00109
G1 X86.04 Y85.161 E.00363
G1 X85.585 Y85.572 E.00298
G1 X85.16 Y86.04 E.00306
G1 X84.724 Y86.647 E.00363
G1 X84.507 Y86.585 E.00109
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.807 Y85.99 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X84.807 Y84.807 E.00573
G1 X85.99 Y84.807 E.00573
G1 X85.375 Y85.357 E.004
G1 X84.807 Y85.99 E.00412
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X85.375 Y85.357 E-.32302
G1 X85.487 Y85.257 E-.05698
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.669 J1.017 P1  F42000
G1 X87.084 Y84.207 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X84.207 Y84.207 E.01395
G1 X84.207 Y87.084 E.01395
G1 X81.33 Y87.084 E.01395
G1 X81.33 Y92.916 E.02828
G1 X84.207 Y92.916 E.01395
G1 X84.207 Y95.793 E.01395
G1 X87.084 Y95.793 E.01395
G1 X87.084 Y98.67 E.01395
G1 X92.916 Y98.67 E.02828
G1 X92.916 Y95.793 E.01395
G1 X95.793 Y95.793 E.01395
G1 X95.793 Y92.916 E.01395
G1 X98.67 Y92.916 E.01395
G1 X98.67 Y87.084 E.02828
G1 X95.793 Y87.084 E.01395
G1 X95.793 Y84.207 E.01395
G1 X92.916 Y84.207 E.01395
G1 X92.916 Y81.33 E.01395
G1 X87.084 Y81.33 E.02828
G1 X87.084 Y84.207 E.01395
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X87.384 Y84.321 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X87.885 Y84.116 E.00262
G1 X88.176 Y84.019 E.00149
G1 X88.771 Y83.869 E.00297
G1 X89.378 Y83.778 E.00297
G1 X89.99 Y83.747 E.00298
G1 X90.622 Y83.778 E.00307
G1 X91.229 Y83.869 E.00298
G1 X91.824 Y84.019 E.00297
G1 X92.115 Y84.116 E.00149
G1 X92.616 Y84.321 E.00262
G1 X92.616 Y81.63 E.01305
G1 X87.384 Y81.63 E.02537
G1 X87.384 Y84.321 E.01305
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X87.684 Y83.865 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X87.684 Y81.93 E.00938
G1 X92.316 Y81.93 E.02246
G1 X92.316 Y83.865 E.00938
G1 X91.889 Y83.725 E.00218
G1 X91.266 Y83.57 E.00311
G1 X90.629 Y83.477 E.00312
G1 X89.986 Y83.447 E.00312
G1 X89.371 Y83.477 E.00299
G1 X88.735 Y83.57 E.00312
G1 X88.11 Y83.725 E.00312
G1 X87.684 Y83.865 E.00217
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X87.984 Y83.449 F42000
M106 S127.5
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X88.646 Y83.282 E.00331
G1 X89.011 Y83.219 E.00179
G1 X89.646 Y83.156 E.0031
G1 X90.017 Y83.147 E.0018
G1 X90.654 Y83.178 E.00309
G1 X91.022 Y83.224 E.0018
G1 X91.649 Y83.348 E.0031
G1 X92.016 Y83.449 E.00184
G1 X92.016 Y82.23 E.00591
G1 X87.984 Y82.23 E.01955
G1 X87.984 Y83.449 E.00591
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X88.284 Y83.056 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X88.625 Y82.98 E.00169
G1 X89.278 Y82.883 E.0032
G1 X89.979 Y82.847 E.0034
G1 X90.721 Y82.883 E.0036
G1 X91.07 Y82.927 E.00171
G1 X91.716 Y83.056 E.00319
G1 X91.716 Y82.53 E.00255
G1 X88.284 Y82.53 E.01664
G1 X88.284 Y83.056 E.00255
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X88.284 Y82.53 E-.19979
G1 X88.758 Y82.53 E-.18022
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-1.209 J-.142 P1  F42000
G1 X88.559 Y84.224 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X88 Y84.393 E.00283
G1 X87.46 Y84.616 E.00283
G1 X86.944 Y84.891 E.00283
G1 X86.458 Y85.215 E.00283
G1 X86.006 Y85.585 E.00283
G1 X85.593 Y85.998 E.00283
G1 X85.215 Y86.458 E.00289
G1 X84.897 Y86.935 E.00278
G1 X84.621 Y87.45 E.00283
G1 X84.397 Y87.989 E.00283
G1 X84.227 Y88.548 E.00283
G1 X84.112 Y89.121 E.00283
G1 X84.054 Y89.703 E.00283
G1 X84.054 Y90.287 E.00283
G1 X84.111 Y90.868 E.00283
G1 X84.224 Y91.441 E.00283
G1 X84.393 Y92 E.00283
G1 X84.616 Y92.54 E.00283
G1 X84.891 Y93.056 E.00283
G1 X85.215 Y93.542 E.00283
G1 X85.585 Y93.994 E.00283
G1 X85.998 Y94.407 E.00283
G1 X86.449 Y94.778 E.00283
G1 X86.935 Y95.103 E.00283
G1 X87.45 Y95.379 E.00283
G1 X87.989 Y95.603 E.00283
G1 X88.548 Y95.773 E.00283
G1 X89.121 Y95.888 E.00283
G1 X89.702 Y95.946 E.00283
G1 X90.287 Y95.946 E.00283
G1 X90.868 Y95.889 E.00283
G1 X91.441 Y95.776 E.00283
G1 X92 Y95.607 E.00283
G1 X92.54 Y95.384 E.00283
G1 X93.056 Y95.109 E.00283
G1 X93.542 Y94.785 E.00283
G1 X93.994 Y94.415 E.00283
G1 X94.407 Y94.002 E.00283
G1 X94.778 Y93.551 E.00283
G1 X95.103 Y93.065 E.00283
G1 X95.379 Y92.55 E.00283
G1 X95.603 Y92.011 E.00283
G1 X95.776 Y91.441 E.00289
G1 X95.888 Y90.879 E.00278
G1 X95.946 Y90.297 E.00283
G1 X95.946 Y89.702 E.00289
G1 X95.888 Y89.121 E.00283
G1 X95.773 Y88.548 E.00283
G1 X95.603 Y87.989 E.00283
G1 X95.379 Y87.45 E.00283
G1 X95.103 Y86.935 E.00283
G1 X94.778 Y86.449 E.00283
G1 X94.407 Y85.998 E.00283
G1 X93.994 Y85.585 E.00283
G1 X93.542 Y85.215 E.00283
G1 X93.056 Y84.891 E.00283
G1 X92.54 Y84.616 E.00283
G1 X92 Y84.393 E.00283
G1 X91.441 Y84.224 E.00283
G1 X90.868 Y84.111 E.00283
G1 X90.289 Y84.054 E.00282
G1 X89.702 Y84.054 E.00284
G1 X89.132 Y84.111 E.00278
G1 X88.559 Y84.224 E.00283
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X89.132 Y84.111 E-.22197
G1 X89.546 Y84.07 E-.15803
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-.206 J1.199 P1  F42000
G1 X93.353 Y84.724 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X93.415 Y84.507 E.00109
G1 X95.493 Y84.507 E.01007
G1 X95.493 Y86.585 E.01007
G1 X95.276 Y86.647 E.00109
G1 X94.839 Y86.04 E.00363
G1 X94.428 Y85.585 E.00297
G1 X93.96 Y85.161 E.00306
G1 X93.353 Y84.724 E.00363
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X94.01 Y84.807 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X94.643 Y85.375 E.00412
G1 X95.193 Y85.99 E.004
G1 X95.193 Y84.807 E.00573
G1 X94.01 Y84.807 E.00573
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X95.01 Y84.807 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I-1.178 J.306 P1  F42000
G1 X95.679 Y87.384 Z2.3
G1 Z1.9
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X95.884 Y87.885 E.00262
G1 X95.981 Y88.176 E.00149
G1 X96.131 Y88.771 E.00297
G1 X96.224 Y89.396 E.00306
G1 X96.253 Y90.009 E.00297
G1 X96.222 Y90.622 E.00298
G1 X96.131 Y91.229 E.00298
G1 X95.981 Y91.824 E.00297
G1 X95.884 Y92.115 E.00149
G1 X95.678 Y92.616 E.00262
G1 X98.37 Y92.616 E.01305
G1 X98.37 Y87.384 E.02537
G1 X95.679 Y87.384 E.01305
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X96.135 Y87.684 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X98.07 Y87.684 E.00938
G1 X98.07 Y92.316 E.02246
G1 X96.135 Y92.316 E.00938
G1 X96.275 Y91.89 E.00217
G1 X96.43 Y91.266 E.00312
G1 X96.52 Y90.656 E.00299
G1 X96.553 Y90.012 E.00312
G1 X96.523 Y89.371 E.00311
G1 X96.43 Y88.735 E.00312
G1 X96.275 Y88.11 E.00312
G1 X96.135 Y87.684 E.00217
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X96.551 Y87.984 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X96.652 Y88.352 E.00185
G1 X96.776 Y88.978 E.00309
G1 X96.822 Y89.345 E.00179
G1 X96.853 Y89.983 E.0031
G1 X96.844 Y90.353 E.00179
G1 X96.776 Y91.022 E.00326
G1 X96.652 Y91.649 E.0031
G1 X96.551 Y92.016 E.00184
G1 X97.77 Y92.016 E.00591
G1 X97.77 Y87.984 E.01955
G1 X96.551 Y87.984 E.00591
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X96.944 Y88.284 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X97.47 Y88.284 E.00255
G1 X97.47 Y91.716 E.01664
G1 X96.944 Y91.716 E.00255
G1 X97.073 Y91.07 E.00319
G1 X97.117 Y90.722 E.0017
G1 X97.153 Y90.019 E.00341
G1 X97.116 Y89.278 E.0036
G1 X97.073 Y88.93 E.0017
G1 X96.944 Y88.284 E.00319
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 5
; WIPE_START
G1 X97.073 Y88.93 E-.25013
G1 X97.115 Y89.269 E-.12987
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.686 J-1.006 P1  F42000
G1 X91.235 Y85.26 Z2.3
G1 Z1.9
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F9580.435
M204 S6000
G1 X91.651 Y85.386 E.01383
G1 X92.095 Y85.57 E.0153
G1 X92.519 Y85.797 E.0153
G1 X92.919 Y86.064 E.0153
G1 X93.291 Y86.369 E.01531
G1 X93.631 Y86.709 E.0153
G1 X93.936 Y87.081 E.01529
G1 X94.203 Y87.481 E.01531
G1 X94.43 Y87.905 E.0153
G1 X94.614 Y88.349 E.0153
G1 X94.753 Y88.809 E.0153
G1 X94.847 Y89.281 E.01531
G1 X94.894 Y89.76 E.0153
G1 X94.894 Y90.24 E.0153
G1 X94.847 Y90.719 E.0153
G1 X94.753 Y91.191 E.01531
G1 X94.614 Y91.651 E.0153
G1 X94.43 Y92.095 E.0153
G1 X94.203 Y92.519 E.01531
G1 X93.936 Y92.919 E.0153
G1 X93.631 Y93.291 E.01531
G1 X93.291 Y93.631 E.0153
G1 X92.919 Y93.936 E.0153
G1 X92.519 Y94.203 E.0153
G1 X92.095 Y94.43 E.0153
G1 X91.651 Y94.614 E.0153
G1 X91.191 Y94.754 E.0153
G1 X90.719 Y94.847 E.0153
G1 X90.24 Y94.894 E.0153
G1 X89.76 Y94.894 E.0153
G1 X89.281 Y94.847 E.0153
G1 X88.809 Y94.754 E.01531
G1 X88.349 Y94.614 E.0153
G1 X87.905 Y94.43 E.0153
G1 X87.481 Y94.203 E.0153
G1 X87.081 Y93.936 E.0153
G1 X86.709 Y93.631 E.01529
G1 X86.369 Y93.291 E.01532
G1 X86.064 Y92.919 E.0153
G1 X85.797 Y92.519 E.0153
G1 X85.57 Y92.095 E.01531
G1 X85.386 Y91.651 E.0153
G1 X85.246 Y91.191 E.0153
G1 X85.153 Y90.719 E.0153
G1 X85.106 Y90.24 E.0153
G1 X85.106 Y89.76 E.0153
G1 X85.153 Y89.281 E.01531
G1 X85.246 Y88.809 E.01531
G1 X85.386 Y88.349 E.0153
G1 X85.57 Y87.905 E.0153
G1 X85.797 Y87.481 E.01531
G1 X86.064 Y87.081 E.0153
G1 X86.369 Y86.709 E.0153
G1 X86.709 Y86.369 E.0153
G1 X87.081 Y86.064 E.0153
G1 X87.481 Y85.797 E.0153
G1 X87.905 Y85.57 E.01531
G1 X88.349 Y85.386 E.01529
G1 X88.809 Y85.246 E.0153
G1 X89.281 Y85.153 E.0153
G1 X89.76 Y85.106 E.0153
G1 X90.232 Y85.105 E.01504
G1 X90.719 Y85.153 E.01557
G1 X91.177 Y85.244 E.01485
; COOLING_NODE: 5
M204 S250
G1 X91.349 Y84.885 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.783 Y85.016 E.01337
G1 X92.263 Y85.215 E.01531
G1 X92.721 Y85.46 E.01531
G1 X93.153 Y85.749 E.0153
G1 X93.555 Y86.078 E.01532
G1 X93.922 Y86.446 E.01531
G1 X94.251 Y86.847 E.01531
G1 X94.54 Y87.279 E.01531
G1 X94.785 Y87.737 E.01531
G1 X94.983 Y88.217 E.01531
G1 X95.134 Y88.714 E.01531
G1 X95.236 Y89.223 E.01531
G1 X95.287 Y89.74 E.01531
G1 X95.287 Y90.26 E.0153
G1 X95.236 Y90.777 E.01531
G1 X95.134 Y91.286 E.01531
G1 X94.984 Y91.783 E.01531
G1 X94.785 Y92.263 E.01531
G1 X94.54 Y92.721 E.01531
G1 X94.251 Y93.153 E.0153
G1 X93.922 Y93.555 E.01531
G1 X93.555 Y93.922 E.01531
G1 X93.153 Y94.251 E.01531
G1 X92.721 Y94.54 E.01531
G1 X92.263 Y94.785 E.01531
G1 X91.783 Y94.984 E.0153
G1 X91.286 Y95.134 E.01531
G1 X90.777 Y95.236 E.01531
G1 X90.26 Y95.287 E.01531
G1 X89.74 Y95.287 E.01531
G1 X89.223 Y95.236 E.01531
G1 X88.714 Y95.134 E.01532
G1 X88.217 Y94.984 E.0153
G1 X87.737 Y94.785 E.01531
G1 X87.279 Y94.54 E.01531
G1 X86.847 Y94.251 E.01531
G1 X86.446 Y93.922 E.0153
G1 X86.078 Y93.554 E.01532
G1 X85.749 Y93.153 E.01531
G1 X85.46 Y92.721 E.0153
G1 X85.215 Y92.263 E.01531
G1 X85.016 Y91.783 E.01531
G1 X84.866 Y91.286 E.01531
G1 X84.764 Y90.777 E.01531
G1 X84.713 Y90.26 E.01531
G1 X84.713 Y89.74 E.0153
G1 X84.764 Y89.223 E.01531
M73 P51 R8
G1 X84.866 Y88.714 E.01531
G1 X85.016 Y88.217 E.01531
G1 X85.215 Y87.737 E.01531
G1 X85.46 Y87.279 E.01532
G1 X85.749 Y86.847 E.0153
G1 X86.078 Y86.445 E.01531
G1 X86.446 Y86.078 E.01531
G1 X86.847 Y85.749 E.01531
G1 X87.279 Y85.46 E.01531
G1 X87.737 Y85.215 E.01531
G1 X88.217 Y85.017 E.0153
G1 X88.714 Y84.866 E.01531
G1 X89.223 Y84.764 E.01531
G1 X89.74 Y84.713 E.01531
G1 X90.251 Y84.713 E.01506
G1 X90.777 Y84.764 E.01556
G1 X91.286 Y84.866 E.01531
G1 X91.291 Y84.867 E.00016
M106 S117.3
; WIPE_START
M204 S6000
G1 X91.783 Y85.016 E-.19523
G1 X92.232 Y85.203 E-.18477
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z2.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z2.3 F4000
            G39.3 S1
            G0 Z2.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X93.347 Y91.125 F42000
G1 Z1.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F9580.435
M204 S6000
G1 X93.194 Y91.511 E.01319
G1 X93.031 Y91.817 E.01103
G1 X92.838 Y92.105 E.01103
G1 X92.496 Y92.496 E.01653
G1 X87.504 Y87.504 E.22459
G1 X87.895 Y87.162 E.01653
G1 X88.183 Y86.969 E.01103
G1 X88.489 Y86.806 E.01104
G1 X88.81 Y86.673 E.01104
G1 X89.141 Y86.572 E.01102
G1 X89.482 Y86.505 E.01104
G1 X89.83 Y86.47 E.01115
G1 X90.085 Y86.469 E.00812
G1 X90.525 Y86.506 E.01405
G1 X90.859 Y86.572 E.01081
G1 X91.19 Y86.673 E.01103
G1 X91.511 Y86.806 E.01103
G1 X91.816 Y86.969 E.01103
G1 X92.105 Y87.162 E.01104
G1 X92.496 Y87.504 E.01653
G1 X87.504 Y92.496 E.22459
G1 X87.895 Y92.838 E.01653
G1 X88.183 Y93.031 E.01103
G1 X88.489 Y93.194 E.01104
G1 X88.875 Y93.347 E.01319
; WIPE_START
G1 X88.489 Y93.194 E-.15747
G1 X88.183 Y93.031 E-.13182
G1 X87.985 Y92.898 E-.09071
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.676 J-1.012 P1  F42000
G1 X85.785 Y91.428 Z2.3
G1 Z1.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.520346
G1 F8168.816
M204 S6000
G1 X86.145 Y92.224 E.03257
G1 X86.653 Y92.933 E.03255
G1 X87.29 Y93.53 E.03256
G1 X88.03 Y93.99 E.03255
G1 X88.846 Y94.298 E.03255
G1 X89.707 Y94.44 E.03256
G1 X90.579 Y94.412 E.03255
G1 X91.429 Y94.215 E.03255
G1 X92.223 Y93.855 E.03255
G1 X92.933 Y93.347 E.03256
G1 X93.529 Y92.711 E.03254
G1 X93.99 Y91.97 E.03256
G1 X94.298 Y91.154 E.03255
G1 X94.44 Y90.293 E.03256
G1 X94.409 Y89.37 E.03447
G1 X94.215 Y88.571 E.03066
G1 X93.855 Y87.776 E.03257
G1 X93.347 Y87.067 E.03255
G1 X92.71 Y86.471 E.03255
G1 X91.97 Y86.01 E.03256
G1 X91.154 Y85.702 E.03254
G1 X90.234 Y85.554 E.03474
G1 X89.416 Y85.588 E.03057
G1 X88.571 Y85.785 E.03237
G1 X87.776 Y86.145 E.03257
G1 X87.067 Y86.653 E.03255
G1 X86.47 Y87.29 E.03257
G1 X86.01 Y88.03 E.03255
G1 X85.702 Y88.846 E.03254
G1 X85.56 Y89.707 E.03256
G1 X85.588 Y90.579 E.03255
G1 X85.772 Y91.37 E.0303
; WIPE_START
G1 X85.588 Y90.579 E-.30859
G1 X85.582 Y90.391 E-.07141
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.3 I.847 J.874 P1  F42000
G1 X90.083 Y86.028 Z2.3
G1 Z1.9
G1 E.4 F1800
; LINE_WIDTH: 0.520461
G1 F8166.851
M204 S6000
G1 X90.944 Y86.139 E.0324
G1 X91.682 Y86.399 E.02919
G1 X92.352 Y86.796 E.02909
G1 X92.932 Y87.316 E.02908
G1 X93.399 Y87.94 E.02908
G1 X93.736 Y88.643 E.0291
G1 X93.929 Y89.398 E.02908
G1 X93.971 Y90.176 E.02908
G1 X93.86 Y90.947 E.02909
G1 X93.601 Y91.682 E.02908
G1 X93.204 Y92.352 E.02909
G1 X92.684 Y92.932 E.02908
G1 X92.06 Y93.399 E.02909
G1 X91.357 Y93.736 E.02908
G1 X90.602 Y93.929 E.02908
G1 X89.824 Y93.971 E.02908
G1 X89.053 Y93.86 E.02909
G1 X88.318 Y93.601 E.02908
G1 X87.648 Y93.204 E.02907
G1 X87.068 Y92.684 E.02909
G1 X86.601 Y92.06 E.02908
G1 X86.264 Y91.357 E.0291
G1 X86.071 Y90.602 E.02908
G1 X86.029 Y89.824 E.02908
G1 X86.14 Y89.053 E.02909
G1 X86.399 Y88.318 E.02907
G1 X86.796 Y87.648 E.02908
G1 X87.316 Y87.068 E.0291
G1 X87.94 Y86.601 E.02908
G1 X88.643 Y86.264 E.02909
G1 X89.398 Y86.071 E.02908
G1 X90.023 Y86.032 E.02339
; CHANGE_LAYER
; Z_HEIGHT: 2.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F8166.851
G1 X89.398 Y86.071 E-.23813
G1 X89.036 Y86.164 E-.14187
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 11/43
; update layer progress
M73 L11
M991 S0 P10 ;notify layer change
M106 S122.4
; OBJECT_ID: 346
M204 S10000
G17
G3 Z2.3 I.618 J1.049 P1  F42000
G1 X93.085 Y83.778 Z2.3
G1 Z2.1
G1 E.4 F1800
; FEATURE: Support transition
; LINE_WIDTH: 0.42
G1 F3000
M204 S6000
G1 X95.754 Y83.778 E.07869
G1 X95.754 Y83.401 E.01111
G1 X93.254 Y83.401 E.07369
G1 X93.254 Y83.024 E.01111
G1 X95.754 Y83.024 E.07369
G1 X95.754 Y82.647 E.01111
G1 X93.254 Y82.647 E.07369
G1 X93.254 Y82.27 E.01111
G1 X95.754 Y82.27 E.07369
G1 X95.754 Y81.893 E.01111
G1 X93.254 Y81.893 E.07369
G1 X93.254 Y81.516 E.01111
G1 X95.924 Y81.516 E.07869
; WIPE_START
G1 X94.924 Y81.516 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I0 J-1.217 P1  F42000
G1 X93.047 Y81.516 Z2.5
G1 Z2.1
G1 E.4 F1800
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X87.123 Y81.516 E.1746
G1 X87.123 Y81.893 E.01111
G1 X92.877 Y81.893 E.1696
G1 X92.877 Y82.27 E.01111
G1 X87.123 Y82.27 E.1696
G1 X87.123 Y82.647 E.01111
G1 X92.877 Y82.647 E.1696
G1 X92.877 Y83.024 E.01111
G1 X87.123 Y83.024 E.1696
G1 X87.123 Y83.401 E.01111
G1 X92.877 Y83.401 E.1696
G1 X92.877 Y83.778 E.01111
G1 X87.123 Y83.778 E.1696
G1 X87.123 Y84.155 E.01111
G1 X87.43 Y84.155 E.00906
G1 X86.995 Y84.365 E.01424
G1 X86.7 Y84.533 E.00998
G1 X84.246 Y84.533 E.07234
G1 X84.246 Y84.91 E.01111
G1 X86.145 Y84.91 E.05598
G1 X85.691 Y85.287 E.01739
G1 X84.246 Y85.287 E.04261
G1 X84.246 Y85.664 E.01111
G1 X85.313 Y85.664 E.03146
G1 X84.991 Y86.041 E.01461
G1 X84.246 Y86.041 E.02197
G1 X84.246 Y86.418 E.01111
G1 X84.714 Y86.418 E.01379
G1 X84.478 Y86.795 E.01311
G1 X84.246 Y86.795 E.00684
G1 X84.246 Y87.123 E.00966
G1 X81.369 Y87.123 E.0848
G1 X81.369 Y87.172 E.00145
G1 X84.276 Y87.172 E.08568
G1 X84.103 Y87.549 E.01223
G1 X81.369 Y87.549 E.08058
G1 X81.369 Y87.926 E.01111
G1 X83.961 Y87.926 E.07641
G1 X83.845 Y88.303 E.01163
G1 X81.369 Y88.303 E.07299
G1 X81.369 Y88.68 E.01111
G1 X83.753 Y88.68 E.07027
G1 X83.684 Y89.057 E.0113
G1 X81.369 Y89.057 E.06823
G1 X81.369 Y89.434 E.01111
G1 X83.64 Y89.434 E.06694
G1 X83.618 Y89.812 E.01113
G1 X81.369 Y89.812 E.0663
G1 X81.369 Y90.189 E.01111
G1 X83.618 Y90.189 E.0663
G1 X83.64 Y90.566 E.01113
G1 X81.369 Y90.566 E.06694
G1 X81.369 Y90.943 E.01111
G1 X83.684 Y90.943 E.06823
G1 X83.753 Y91.32 E.0113
G1 X81.369 Y91.32 E.07027
G1 X81.369 Y91.697 E.01111
G1 X83.845 Y91.697 E.07299
G1 X83.961 Y92.074 E.01163
G1 X81.369 Y92.074 E.07641
G1 X81.369 Y92.451 E.01111
G1 X84.103 Y92.451 E.08058
G1 X84.276 Y92.828 E.01223
G1 X81.369 Y92.828 E.08568
G1 X81.369 Y92.877 E.00145
G1 X84.246 Y92.877 E.0848
G1 X84.246 Y93.205 E.00966
G1 X84.478 Y93.205 E.00684
G1 X84.714 Y93.582 E.01311
G1 X84.246 Y93.582 E.0138
G1 X84.246 Y93.959 E.01111
G1 X84.991 Y93.959 E.02197
G1 X85.313 Y94.336 E.01461
G1 X84.246 Y94.336 E.03146
G1 X84.246 Y94.713 E.01111
G1 X85.691 Y94.713 E.04261
G1 X86.145 Y95.09 E.01739
G1 X84.246 Y95.09 E.05599
G1 X84.246 Y95.468 E.01111
G1 X86.7 Y95.468 E.07235
G1 X86.995 Y95.635 E.00998
G1 X87.43 Y95.845 E.01424
G1 X87.123 Y95.845 E.00906
G1 X87.123 Y96.222 E.01111
G1 X92.877 Y96.222 E.1696
G1 X92.877 Y95.845 E.01111
G1 X92.57 Y95.845 E.00906
G1 X93.005 Y95.635 E.01425
G1 X93.3 Y95.468 E.00998
G1 X95.754 Y95.468 E.07235
G1 X95.754 Y95.09 E.01111
G1 X93.855 Y95.09 E.05599
G1 X94.309 Y94.713 E.01739
G1 X95.754 Y94.713 E.04261
G1 X95.754 Y94.336 E.01111
G1 X94.687 Y94.336 E.03146
G1 X95.009 Y93.959 E.01461
G1 X95.754 Y93.959 E.02197
G1 X95.754 Y93.582 E.01111
G1 X95.286 Y93.582 E.0138
G1 X95.522 Y93.205 E.01311
G1 X95.754 Y93.205 E.00684
G1 X95.754 Y92.877 E.00966
G1 X98.631 Y92.877 E.0848
G1 X98.631 Y92.828 E.00145
G1 X95.724 Y92.828 E.08568
G1 X95.897 Y92.451 E.01223
G1 X98.631 Y92.451 E.08058
G1 X98.631 Y92.074 E.01111
G1 X96.039 Y92.074 E.07641
G1 X96.155 Y91.697 E.01163
G1 X98.631 Y91.697 E.07299
G1 X98.631 Y91.32 E.01111
G1 X96.247 Y91.32 E.07027
G1 X96.316 Y90.943 E.0113
G1 X98.631 Y90.943 E.06823
G1 X98.631 Y90.566 E.01111
G1 X96.36 Y90.566 E.06694
G1 X96.382 Y90.189 E.01113
G1 X98.631 Y90.189 E.0663
G1 X98.631 Y89.812 E.01111
G1 X96.382 Y89.812 E.0663
G1 X96.36 Y89.434 E.01113
G1 X98.631 Y89.434 E.06694
G1 X98.631 Y89.057 E.01111
G1 X96.316 Y89.057 E.06823
G1 X96.247 Y88.68 E.0113
G1 X98.631 Y88.68 E.07027
G1 X98.631 Y88.303 E.01111
G1 X96.155 Y88.303 E.07299
G1 X96.039 Y87.926 E.01163
G1 X98.631 Y87.926 E.07641
G1 X98.631 Y87.549 E.01111
G1 X95.897 Y87.549 E.08058
G1 X95.724 Y87.172 E.01223
G1 X98.631 Y87.172 E.08568
G1 X98.631 Y87.123 E.00145
G1 X95.754 Y87.123 E.0848
G1 X95.754 Y86.795 E.00966
G1 X95.522 Y86.795 E.00684
G1 X95.286 Y86.418 E.01311
G1 X95.754 Y86.418 E.0138
G1 X95.754 Y86.041 E.01111
G1 X95.009 Y86.041 E.02197
G1 X94.687 Y85.664 E.01461
G1 X95.754 Y85.664 E.03146
G1 X95.754 Y85.287 E.01111
G1 X94.309 Y85.287 E.04261
G1 X93.855 Y84.91 E.01739
G1 X95.754 Y84.91 E.05599
G1 X95.754 Y84.533 E.01111
G1 X93.3 Y84.533 E.07234
G1 X93.005 Y84.365 E.00999
G1 X92.57 Y84.155 E.01424
G1 X95.924 Y84.155 E.09885
M204 S10000
G1 X95.962 Y84.532 F42000
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X98.631 Y84.532 E.07869
G1 X98.631 Y84.91 E.01111
G1 X96.131 Y84.91 E.07369
G1 X96.131 Y85.287 E.01111
G1 X98.631 Y85.287 E.07369
G1 X98.631 Y85.664 E.01111
G1 X96.131 Y85.664 E.07369
G1 X96.131 Y86.041 E.01111
G1 X98.631 Y86.041 E.07369
G1 X98.631 Y86.418 E.01111
G1 X95.962 Y86.418 E.07869
; WIPE_START
G1 X96.962 Y86.418 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I-1.205 J-.168 P1  F42000
G1 X95.962 Y93.582 Z2.5
G1 Z2.1
G1 E.4 F1800
G1 F3000
M204 S6000
G1 X98.631 Y93.582 E.07869
G1 X98.631 Y93.959 E.01111
G1 X96.131 Y93.959 E.07369
G1 X96.131 Y94.336 E.01111
G1 X98.631 Y94.336 E.07369
G1 X98.631 Y94.713 E.01111
G1 X96.131 Y94.713 E.07369
M73 P52 R8
G1 X96.131 Y95.09 E.01111
G1 X98.631 Y95.09 E.07369
G1 X98.631 Y95.467 E.01111
G1 X95.962 Y95.467 E.07869
M204 S10000
G1 X95.924 Y96.222 F42000
G1 F3000
M204 S6000
G1 X93.254 Y96.222 E.07869
G1 X93.254 Y96.599 E.01111
G1 X95.754 Y96.599 E.07369
G1 X95.754 Y96.976 E.01111
G1 X93.254 Y96.976 E.07369
G1 X93.254 Y97.353 E.01111
G1 X95.754 Y97.353 E.07369
G1 X95.754 Y97.73 E.01111
G1 X93.254 Y97.73 E.07369
G1 X93.254 Y98.107 E.01111
G1 X95.754 Y98.107 E.07369
G1 X95.754 Y98.484 E.01111
G1 X93.085 Y98.484 E.07869
M204 S10000
G1 X93.047 Y98.484 F42000
; FEATURE: Support interface
G1 F4800
M204 S6000
G1 X87.123 Y98.484 E.1746
G1 X87.123 Y98.107 E.01111
G1 X92.877 Y98.107 E.1696
G1 X92.877 Y97.73 E.01111
G1 X87.123 Y97.73 E.1696
G1 X87.123 Y97.353 E.01111
G1 X92.877 Y97.353 E.1696
G1 X92.877 Y96.976 E.01111
G1 X87.123 Y96.976 E.1696
G1 X87.123 Y96.599 E.01111
G1 X93.047 Y96.599 E.1746
; WIPE_START
G1 X92.047 Y96.599 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I.089 J-1.214 P1  F42000
G1 X86.916 Y96.222 Z2.5
G1 Z2.1
G1 E.4 F1800
; FEATURE: Support transition
G1 F3000
M204 S6000
G1 X84.246 Y96.222 E.07869
G1 X84.246 Y96.599 E.01111
G1 X86.746 Y96.599 E.07369
G1 X86.746 Y96.976 E.01111
G1 X84.246 Y96.976 E.07369
G1 X84.246 Y97.353 E.01111
G1 X86.746 Y97.353 E.07369
G1 X86.746 Y97.73 E.01111
G1 X84.246 Y97.73 E.07369
G1 X84.246 Y98.107 E.01111
G1 X86.746 Y98.107 E.07369
G1 X86.746 Y98.484 E.01111
G1 X84.076 Y98.484 E.07869
; WIPE_START
G1 X85.076 Y98.484 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I.747 J-.96 P1  F42000
G1 X81.199 Y95.467 Z2.5
G1 Z2.1
G1 E.4 F1800
G1 F3000
M204 S6000
G1 X83.869 Y95.467 E.07869
G1 X83.869 Y95.09 E.01111
G1 X81.369 Y95.09 E.07369
G1 X81.369 Y94.713 E.01111
G1 X83.869 Y94.713 E.07369
G1 X83.869 Y94.336 E.01111
G1 X81.369 Y94.336 E.07369
G1 X81.369 Y93.959 E.01111
G1 X83.869 Y93.959 E.07369
G1 X83.869 Y93.582 E.01111
G1 X81.199 Y93.582 E.07869
; WIPE_START
G1 X82.199 Y93.582 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I1.205 J-.168 P1  F42000
G1 X81.199 Y86.418 Z2.5
G1 Z2.1
G1 E.4 F1800
M106 S127.5
G1 F3000
M204 S6000
G1 X83.869 Y86.418 E.07869
G1 X83.869 Y86.041 E.01111
G1 X81.369 Y86.041 E.07369
G1 X81.369 Y85.664 E.01111
G1 X83.869 Y85.664 E.07369
G1 X83.869 Y85.287 E.01111
G1 X81.369 Y85.287 E.07369
G1 X81.369 Y84.91 E.01111
G1 X83.869 Y84.91 E.07369
G1 X83.869 Y84.532 E.01111
G1 X81.199 Y84.532 E.07869
; WIPE_START
G1 X82.199 Y84.532 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I.454 J1.129 P1  F42000
G1 X84.076 Y83.778 Z2.5
G1 Z2.1
G1 E.4 F1800
G1 F3000
M204 S6000
G1 X86.746 Y83.778 E.07869
G1 X86.746 Y83.401 E.01111
G1 X84.246 Y83.401 E.07369
G1 X84.246 Y83.024 E.01111
G1 X86.746 Y83.024 E.07369
G1 X86.746 Y82.647 E.01111
G1 X84.246 Y82.647 E.07369
G1 X84.246 Y82.27 E.01111
G1 X86.746 Y82.27 E.07369
G1 X86.746 Y81.893 E.01111
G1 X84.246 Y81.893 E.07369
G1 X84.246 Y81.516 E.01111
G1 X86.916 Y81.516 E.07869
; WIPE_START
G1 X85.916 Y81.516 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I-.555 J1.083 P1  F42000
G1 X90.084 Y83.654 Z2.5
G1 Z2.1
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X90.628 Y83.683 E.00264
G1 X91.244 Y83.775 E.00302
G1 X91.848 Y83.927 E.00302
G1 X92.434 Y84.138 E.00302
G1 X92.988 Y84.399 E.00297
G1 X93.531 Y84.725 E.00307
G1 X94.031 Y85.097 E.00302
G1 X94.492 Y85.515 E.00302
G1 X94.91 Y85.977 E.00302
G1 X95.281 Y86.478 E.00302
G1 X95.596 Y87.003 E.00297
G1 X95.867 Y87.576 E.00307
G1 X96.076 Y88.163 E.00302
G1 X96.227 Y88.767 E.00302
G1 X96.318 Y89.383 E.00302
G1 X96.348 Y90.006 E.00302
G1 X96.318 Y90.617 E.00297
G1 X96.225 Y91.244 E.00307
G1 X96.076 Y91.837 E.00297
G1 X95.867 Y92.424 E.00302
G1 X95.601 Y92.988 E.00302
G1 X95.275 Y93.531 E.00307
G1 X94.903 Y94.031 E.00302
G1 X94.485 Y94.492 E.00302
G1 X94.023 Y94.91 E.00302
G1 X93.522 Y95.281 E.00302
G1 X92.988 Y95.601 E.00302
G1 X92.424 Y95.867 E.00302
G1 X91.838 Y96.076 E.00302
G1 X91.233 Y96.227 E.00302
G1 X90.617 Y96.318 E.00302
M73 P52 R7
G1 X89.994 Y96.348 E.00302
G1 X89.372 Y96.317 E.00302
G1 X88.756 Y96.225 E.00302
G1 X88.152 Y96.073 E.00302
G1 X87.566 Y95.862 E.00302
G1 X87.003 Y95.596 E.00302
G1 X86.469 Y95.275 E.00302
G1 X85.969 Y94.903 E.00302
G1 X85.508 Y94.485 E.00302
G1 X85.089 Y94.023 E.00302
G1 X84.725 Y93.531 E.00297
G1 X84.399 Y92.988 E.00307
G1 X84.133 Y92.424 E.00302
G1 X83.924 Y91.838 E.00302
G1 X83.773 Y91.233 E.00302
G1 X83.682 Y90.617 E.00302
G1 X83.652 Y89.994 E.00302
G1 X83.683 Y89.372 E.00302
G1 X83.775 Y88.756 E.00302
G1 X83.927 Y88.152 E.00302
G1 X84.138 Y87.566 E.00302
G1 X84.404 Y87.003 E.00302
G1 X84.725 Y86.469 E.00302
G1 X85.097 Y85.969 E.00302
G1 X85.515 Y85.507 E.00302
G1 X85.969 Y85.097 E.00297
G1 X86.469 Y84.725 E.00302
G1 X87.003 Y84.404 E.00302
G1 X87.576 Y84.133 E.00308
G1 X88.152 Y83.927 E.00297
G1 X88.767 Y83.773 E.00307
G1 X89.383 Y83.682 E.00302
G1 X90.084 Y83.654 E.0034
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X90.184 Y83.589 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X89.683 Y83.592 E.00243
G1 X89.057 Y83.654 E.00305
G1 X88.439 Y83.777 E.00305
G1 X87.837 Y83.96 E.00305
G1 X87.259 Y84.2 E.00304
G1 X86.703 Y84.496 E.00305
G1 X86.18 Y84.846 E.00305
G1 X85.69 Y85.247 E.00307
G1 X85.247 Y85.69 E.00304
G1 X84.846 Y86.18 E.00307
G1 X84.496 Y86.703 E.00305
G1 X84.2 Y87.259 E.00305
G1 X83.959 Y87.84 E.00305
G1 X83.776 Y88.443 E.00305
G1 X83.653 Y89.061 E.00305
G1 X83.592 Y89.687 E.00305
G1 X83.592 Y90.317 E.00305
G1 X83.653 Y90.939 E.00303
G1 X83.776 Y91.557 E.00305
G1 X83.959 Y92.159 E.00305
G1 X84.2 Y92.741 E.00305
G1 X84.496 Y93.297 E.00305
G1 X84.848 Y93.823 E.00307
G1 X85.248 Y94.31 E.00305
G1 X85.693 Y94.755 E.00305
G1 X86.177 Y95.152 E.00304
G1 X86.703 Y95.504 E.00307
G1 X87.255 Y95.799 E.00303
G1 X87.84 Y96.041 E.00307
G1 X88.443 Y96.224 E.00305
G1 X89.057 Y96.346 E.00303
G1 X89.687 Y96.408 E.00307
G1 X90.317 Y96.408 E.00305
G1 X90.943 Y96.346 E.00305
G1 X91.557 Y96.224 E.00304
G1 X92.16 Y96.041 E.00305
G1 X92.741 Y95.8 E.00305
G1 X93.297 Y95.504 E.00305
G1 X93.824 Y95.152 E.00307
G1 X94.307 Y94.755 E.00303
G1 X94.755 Y94.307 E.00307
G1 X95.154 Y93.821 E.00305
G1 X95.502 Y93.3 E.00304
G1 X95.8 Y92.741 E.00307
G1 X96.04 Y92.163 E.00304
G1 X96.223 Y91.561 E.00305
G1 X96.346 Y90.943 E.00305
G1 X96.408 Y90.313 E.00307
G1 X96.408 Y89.683 E.00305
G1 X96.346 Y89.057 E.00305
G1 X96.223 Y88.439 E.00305
G1 X96.04 Y87.837 E.00305
G1 X95.8 Y87.259 E.00303
G1 X95.504 Y86.703 E.00305
G1 X95.152 Y86.177 E.00307
G1 X94.752 Y85.69 E.00305
G1 X94.307 Y85.245 E.00305
G1 X93.823 Y84.848 E.00303
G1 X93.3 Y84.498 E.00305
G1 X92.741 Y84.2 E.00307
G1 X92.16 Y83.959 E.00305
G1 X91.557 Y83.776 E.00305
G1 X90.939 Y83.653 E.00306
G1 X90.184 Y83.589 E.00368
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 5
; WIPE_START
G1 X90.939 Y83.653 E-.28818
G1 X91.176 Y83.701 E-.09182
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I-1.206 J.164 P1  F42000
G1 X91.337 Y84.883 Z2.5
G1 Z2.1
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F9580.435
M204 S6000
G1 X91.537 Y84.933 E.00657
G1 X92.026 Y85.108 E.01653
G1 X92.496 Y85.33 E.01654
G1 X92.942 Y85.597 E.01653
G1 X93.359 Y85.907 E.01653
G1 X93.744 Y86.256 E.01653
G1 X94.093 Y86.641 E.01654
G1 X94.403 Y87.058 E.01654
G1 X94.67 Y87.504 E.01653
G1 X94.892 Y87.974 E.01654
G1 X95.067 Y88.463 E.01654
G1 X95.193 Y88.967 E.01653
G1 X95.27 Y89.481 E.01653
G1 X95.295 Y90 E.01653
G1 X95.27 Y90.519 E.01654
G1 X95.193 Y91.033 E.01653
G1 X95.067 Y91.537 E.01653
G1 X94.892 Y92.026 E.01654
G1 X94.67 Y92.496 E.01653
G1 X94.403 Y92.942 E.01653
G1 X94.093 Y93.359 E.01653
G1 X93.744 Y93.744 E.01653
G1 X93.359 Y94.093 E.01653
G1 X92.942 Y94.403 E.01654
G1 X92.496 Y94.67 E.01653
G1 X92.026 Y94.892 E.01654
G1 X91.537 Y95.067 E.01653
G1 X91.033 Y95.193 E.01653
G1 X90.519 Y95.27 E.01653
G1 X90 Y95.295 E.01654
G1 X89.481 Y95.27 E.01654
G1 X88.967 Y95.193 E.01653
G1 X88.463 Y95.067 E.01653
G1 X87.973 Y94.892 E.01654
G1 X87.504 Y94.67 E.01653
G1 X87.058 Y94.403 E.01653
G1 X86.641 Y94.093 E.01654
G1 X86.256 Y93.744 E.01653
G1 X85.907 Y93.359 E.01654
G1 X85.597 Y92.942 E.01653
G1 X85.33 Y92.496 E.01653
G1 X85.108 Y92.026 E.01654
G1 X84.933 Y91.537 E.01653
G1 X84.807 Y91.033 E.01653
G1 X84.73 Y90.519 E.01653
G1 X84.705 Y90 E.01654
G1 X84.73 Y89.481 E.01654
G1 X84.807 Y88.967 E.01653
G1 X84.933 Y88.463 E.01653
G1 X85.108 Y87.974 E.01653
G1 X85.33 Y87.504 E.01654
G1 X85.597 Y87.058 E.01653
G1 X85.907 Y86.641 E.01654
G1 X86.256 Y86.256 E.01653
G1 X86.641 Y85.907 E.01654
G1 X87.058 Y85.597 E.01654
G1 X87.504 Y85.33 E.01653
G1 X87.974 Y85.108 E.01654
G1 X88.463 Y84.933 E.01653
G1 X88.967 Y84.807 E.01654
G1 X89.478 Y84.731 E.01645
G1 X90.069 Y84.707 E.01881
G1 X90.518 Y84.73 E.0143
G1 X91.033 Y84.807 E.01657
G1 X91.278 Y84.868 E.00805
; COOLING_NODE: 5
M204 S250
G1 X91.432 Y84.502 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X91.651 Y84.557 E.00666
G1 X92.177 Y84.745 E.01645
G1 X92.681 Y84.984 E.01645
G1 X93.16 Y85.271 E.01645
G1 X93.608 Y85.603 E.01645
G1 X94.022 Y85.978 E.01644
G1 X94.397 Y86.392 E.01646
G1 X94.729 Y86.84 E.01645
G1 X95.016 Y87.319 E.01645
G1 X95.255 Y87.824 E.01646
G1 X95.443 Y88.349 E.01645
G1 X95.578 Y88.89 E.01645
G1 X95.66 Y89.442 E.01645
G1 X95.688 Y90 E.01645
G1 X95.66 Y90.558 E.01645
G1 X95.578 Y91.11 E.01645
G1 X95.443 Y91.651 E.01645
G1 X95.255 Y92.177 E.01645
G1 X95.016 Y92.681 E.01645
G1 X94.729 Y93.16 E.01645
G1 X94.397 Y93.608 E.01645
G1 X94.022 Y94.022 E.01644
G1 X93.608 Y94.397 E.01645
G1 X93.16 Y94.729 E.01645
G1 X92.681 Y95.016 E.01645
G1 X92.177 Y95.255 E.01646
G1 X91.651 Y95.443 E.01645
G1 X91.11 Y95.578 E.01645
G1 X90.558 Y95.66 E.01645
G1 X90 Y95.688 E.01645
G1 X89.442 Y95.66 E.01645
G1 X88.89 Y95.578 E.01645
G1 X88.349 Y95.443 E.01645
G1 X87.823 Y95.255 E.01645
G1 X87.319 Y95.016 E.01645
G1 X86.84 Y94.729 E.01645
G1 X86.392 Y94.397 E.01645
G1 X85.978 Y94.022 E.01645
G1 X85.603 Y93.608 E.01646
G1 X85.271 Y93.16 E.01644
G1 X84.984 Y92.681 E.01645
G1 X84.745 Y92.176 E.01646
G1 X84.557 Y91.651 E.01644
G1 X84.422 Y91.11 E.01645
G1 X84.34 Y90.558 E.01645
G1 X84.312 Y90 E.01645
G1 X84.34 Y89.442 E.01645
G1 X84.422 Y88.89 E.01645
G1 X84.557 Y88.349 E.01645
G1 X84.745 Y87.823 E.01645
G1 X84.984 Y87.319 E.01645
G1 X85.271 Y86.84 E.01645
G1 X85.603 Y86.392 E.01645
G1 X85.978 Y85.978 E.01645
G1 X86.392 Y85.603 E.01645
G1 X86.84 Y85.271 E.01646
G1 X87.319 Y84.984 E.01645
G1 X87.823 Y84.745 E.01646
G1 X88.349 Y84.557 E.01645
G1 X88.89 Y84.422 E.01645
G1 X89.442 Y84.34 E.01642
G1 X90.072 Y84.314 E.01858
G1 X90.557 Y84.34 E.01433
G1 X91.11 Y84.422 E.01646
G1 X91.374 Y84.488 E.00803
M106 S122.4
; WIPE_START
M204 S6000
G1 X91.651 Y84.557 E-.10861
G1 X92.177 Y84.745 E-.21213
G1 X92.318 Y84.812 E-.05926
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z2.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z2.5 F4000
            G39.3 S1
            G0 Z2.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X93.638 Y86.778 F42000
G1 Z2.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.477442
G1 F8975.397
M204 S6000
G1 X93.428 Y86.542 E.01075
G1 X92.688 Y85.939 E.03242
G1 X91.844 Y85.493 E.03242
G1 X90.929 Y85.22 E.03243
G1 X90.23 Y85.141 E.02389
G1 X89.53 Y85.151 E.02377
G1 X89.039 Y85.224 E.01684
G1 X88.395 Y85.41 E.02276
G1 X87.723 Y85.695 E.0248
G1 X86.927 Y86.222 E.03242
G1 X86.249 Y86.894 E.03242
G1 X85.715 Y87.685 E.03242
G1 X85.346 Y88.566 E.03242
G1 X85.156 Y89.501 E.03242
G1 X85.152 Y90.456 E.03243
G1 X85.334 Y91.393 E.03242
G1 X85.695 Y92.277 E.03242
G1 X86.222 Y93.073 E.03241
G1 X86.894 Y93.751 E.03243
G1 X87.686 Y94.285 E.03242
G1 X88.566 Y94.654 E.03242
G1 X89.501 Y94.844 E.03242
G1 X90.456 Y94.848 E.03243
G1 X91.393 Y94.666 E.03242
M73 P53 R7
G1 X92.277 Y94.305 E.03242
G1 X93.073 Y93.778 E.03242
G1 X93.751 Y93.106 E.03242
G1 X94.285 Y92.315 E.03241
G1 X94.658 Y91.424 E.03279
G1 X94.779 Y90.94 E.01697
G1 X94.851 Y90.294 E.02205
G1 X94.87 Y90.021 E.00929
G1 X94.815 Y89.342 E.02314
G1 X94.772 Y89.029 E.01074
G1 X94.594 Y88.416 E.02169
G1 X94.491 Y88.117 E.01074
G1 X94.197 Y87.55 E.02169
G1 X94.037 Y87.277 E.01073
G1 X93.676 Y86.825 E.01965
; WIPE_START
G1 X94.037 Y87.277 E-.21981
G1 X94.197 Y87.55 E-.12011
G1 X94.245 Y87.643 E-.04008
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I.619 J-1.048 P1  F42000
G1 X90.876 Y85.655 Z2.5
G1 Z2.1
G1 E.4 F1800
; LINE_WIDTH: 0.469793
G1 F9136.223
M204 S6000
G1 X91.706 Y85.908 E.02898
G1 X92.472 Y86.319 E.029
G1 X93.143 Y86.872 E.029
G1 X93.692 Y87.546 E.029
G1 X94.1 Y88.313 E.029
G1 X94.351 Y89.146 E.029
G1 X94.434 Y90.011 E.02899
G1 X94.345 Y90.886 E.02935
G1 X94.25 Y91.267 E.0131
G1 X94.02 Y91.883 E.02195
G1 X93.576 Y92.631 E.02903
G1 X92.994 Y93.278 E.02904
G1 X92.297 Y93.799 E.02904
G1 X91.511 Y94.175 E.02905
G1 X90.668 Y94.389 E.02903
G1 X89.799 Y94.435 E.02903
G1 X88.937 Y94.31 E.02904
G1 X88.117 Y94.02 E.02903
G1 X87.369 Y93.576 E.02904
G1 X86.714 Y92.987 E.02937
G1 X86.201 Y92.297 E.0287
G1 X85.826 Y91.511 E.02905
G1 X85.611 Y90.668 E.02904
G1 X85.565 Y89.799 E.02903
G1 X85.689 Y88.937 E.02904
G1 X85.98 Y88.117 E.02904
G1 X86.424 Y87.369 E.02902
G1 X87.006 Y86.722 E.02904
G1 X87.703 Y86.201 E.02903
G1 X88.489 Y85.826 E.02905
G1 X89.146 Y85.649 E.02269
G1 X89.55 Y85.588 E.01367
G1 X90.066 Y85.561 E.01723
G1 X90.816 Y85.648 E.02518
; WIPE_START
G1 X90.066 Y85.561 E-.28682
G1 X89.821 Y85.574 E-.09318
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.5 I-1.043 J-.626 P1  F42000
G1 X86.266 Y91.495 Z2.5
G1 Z2.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F9580.435
M204 S6000
G1 X86.36 Y91.721 E.00781
G1 X86.547 Y92.07 E.01257
G1 X86.766 Y92.398 E.01257
G1 X87.017 Y92.704 E.01257
G1 X87.156 Y92.844 E.00628
G1 X92.844 Y87.156 E.2559
G1 X92.704 Y87.017 E.00628
G1 X92.399 Y86.766 E.01257
G1 X92.07 Y86.546 E.01259
G1 X91.722 Y86.36 E.01256
G1 X91.357 Y86.209 E.01257
G1 X90.978 Y86.094 E.01258
G1 X90.595 Y86.018 E.01242
G1 X90.136 Y85.977 E.01467
G1 X89.804 Y85.978 E.01056
G1 X89.409 Y86.017 E.01262
G1 X89.022 Y86.094 E.01258
G1 X88.644 Y86.209 E.01257
G1 X88.279 Y86.36 E.01257
G1 X87.93 Y86.547 E.01258
G1 X87.602 Y86.766 E.01256
G1 X87.296 Y87.017 E.01258
G1 X87.156 Y87.156 E.00628
G1 X92.844 Y92.844 E.2559
G1 X92.704 Y92.983 E.00629
G1 X92.398 Y93.234 E.01257
G1 X92.07 Y93.453 E.01257
G1 X91.722 Y93.64 E.01257
G1 X91.495 Y93.734 E.00781
; CHANGE_LAYER
; Z_HEIGHT: 2.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X91.722 Y93.64 E-.09329
G1 X92.07 Y93.453 E-.15013
G1 X92.369 Y93.254 E-.13658
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 12/43
; update layer progress
M73 L12
M991 S0 P11 ;notify layer change
M106 S112.2
; OBJECT_ID: 346
M204 S10000
G17
G3 Z2.5 I-.487 J1.115 P1  F42000
G1 X98.484 Y95.924 Z2.5
G1 Z2.3
G1 E.4 F1800
; FEATURE: Support interface
; LINE_WIDTH: 0.42
G1 F4800
M204 S6000
G1 X98.484 Y84.246 E.34419
G1 X98.107 Y84.246 E.01111
G1 X98.107 Y95.754 E.33919
G1 X97.73 Y95.754 E.01111
G1 X97.73 Y84.246 E.33919
G1 X97.353 Y84.246 E.01111
G1 X97.353 Y95.754 E.33919
G1 X96.976 Y95.754 E.01111
G1 X96.976 Y84.246 E.33919
G1 X96.599 Y84.246 E.01111
G1 X96.599 Y95.754 E.33919
G1 X96.222 Y95.754 E.01111
G1 X96.222 Y92.626 E.09219
G1 X96.103 Y92.893 E.00861
G1 X95.845 Y93.382 E.01629
G1 X95.845 Y95.754 E.06992
G1 X95.754 Y95.754 E.00266
G1 X95.754 Y98.631 E.0848
G1 X95.467 Y98.631 E.00845
G1 X95.467 Y93.963 E.13758
G1 X95.09 Y94.436 E.01783
G1 X95.09 Y98.631 E.12364
G1 X94.713 Y98.631 E.01111
G1 X94.713 Y94.836 E.11187
G1 X94.336 Y95.177 E.01499
G1 X94.336 Y98.631 E.10182
G1 X93.959 Y98.631 E.01111
G1 X93.959 Y95.47 E.09317
G1 X93.582 Y95.724 E.01339
G1 X93.582 Y98.631 E.0857
G1 X93.205 Y98.631 E.01111
G1 X93.205 Y95.945 E.07918
G1 X92.828 Y96.132 E.01241
G1 X92.828 Y98.631 E.07366
G1 X92.451 Y98.631 E.01111
G1 X92.451 Y96.292 E.06896
G1 X92.074 Y96.426 E.0118
G1 X92.074 Y98.631 E.065
G1 X91.697 Y98.631 E.01111
G1 X91.697 Y96.536 E.06174
G1 X91.32 Y96.624 E.01141
G1 X91.32 Y98.631 E.05917
G1 X90.943 Y98.631 E.01111
G1 X90.943 Y96.687 E.05731
G1 X90.566 Y96.729 E.01118
G1 X90.566 Y98.631 E.05608
G1 X90.188 Y98.631 E.01111
G1 X90.188 Y96.75 E.05546
G1 X89.811 Y96.75 E.01111
G1 X89.811 Y98.631 E.05546
G1 X89.434 Y98.631 E.01111
G1 X89.434 Y96.729 E.05608
G1 X89.057 Y96.687 E.01118
G1 X89.057 Y98.631 E.05731
G1 X88.68 Y98.631 E.01111
G1 X88.68 Y96.624 E.05917
G1 X88.303 Y96.536 E.01141
G1 X88.303 Y98.631 E.06175
G1 X87.926 Y98.631 E.01111
G1 X87.926 Y96.426 E.065
G1 X87.549 Y96.292 E.0118
G1 X87.549 Y98.631 E.06896
G1 X87.172 Y98.631 E.01111
G1 X87.172 Y96.132 E.07366
G1 X86.795 Y95.945 E.01241
G1 X86.795 Y98.631 E.07919
G1 X86.418 Y98.631 E.01111
G1 X86.418 Y95.724 E.0857
G1 X86.041 Y95.47 E.01339
G1 X86.041 Y98.631 E.09317
G1 X85.664 Y98.631 E.01111
G1 X85.664 Y95.177 E.10182
G1 X85.287 Y94.836 E.01499
G1 X85.287 Y98.631 E.11188
G1 X84.91 Y98.631 E.01111
G1 X84.91 Y94.436 E.12364
G1 X84.532 Y93.963 E.01783
G1 X84.532 Y98.631 E.13759
G1 X84.246 Y98.631 E.00845
G1 X84.246 Y95.754 E.0848
G1 X84.155 Y95.754 E.00266
G1 X84.155 Y93.382 E.06993
G1 X83.897 Y92.893 E.01629
G1 X83.778 Y92.626 E.00861
G1 X83.778 Y95.754 E.09219
G1 X83.401 Y95.754 E.01111
G1 X83.401 Y84.246 E.33919
G1 X83.778 Y84.246 E.01111
G1 X83.778 Y87.374 E.09219
G1 X83.897 Y87.107 E.00861
G1 X84.155 Y86.618 E.01629
G1 X84.155 Y84.246 E.06993
G1 X84.246 Y84.246 E.00266
G1 X84.246 Y81.369 E.0848
G1 X84.532 Y81.369 E.00845
G1 X84.532 Y86.037 E.13759
G1 X84.91 Y85.564 E.01783
G1 X84.91 Y81.369 E.12364
G1 X85.287 Y81.369 E.01111
G1 X85.287 Y85.164 E.11187
G1 X85.664 Y84.823 E.01499
G1 X85.664 Y81.369 E.10182
G1 X86.041 Y81.369 E.01111
G1 X86.041 Y84.53 E.09317
G1 X86.418 Y84.276 E.01339
G1 X86.418 Y81.369 E.0857
G1 X86.795 Y81.369 E.01111
G1 X86.795 Y84.055 E.07918
G1 X87.172 Y83.868 E.01241
G1 X87.172 Y81.369 E.07366
G1 X87.549 Y81.369 E.01111
G1 X87.549 Y83.708 E.06896
G1 X87.926 Y83.574 E.0118
G1 X87.926 Y81.369 E.065
G1 X88.303 Y81.369 E.01111
G1 X88.303 Y83.464 E.06174
G1 X88.68 Y83.376 E.01141
G1 X88.68 Y81.369 E.05917
G1 X89.057 Y81.369 E.01111
G1 X89.057 Y83.313 E.0573
G1 X89.434 Y83.271 E.01118
G1 X89.434 Y81.369 E.05607
G1 X89.811 Y81.369 E.01111
G1 X89.811 Y83.25 E.05546
G1 X90.188 Y83.25 E.01111
G1 X90.188 Y81.369 E.05546
G1 X90.566 Y81.369 E.01111
G1 X90.566 Y83.271 E.05607
G1 X90.943 Y83.313 E.01118
G1 X90.943 Y81.369 E.0573
G1 X91.32 Y81.369 E.01111
G1 X91.32 Y83.376 E.05917
G1 X91.697 Y83.464 E.01141
G1 X91.697 Y81.369 E.06174
G1 X92.074 Y81.369 E.01111
G1 X92.074 Y83.574 E.065
G1 X92.451 Y83.708 E.0118
G1 X92.451 Y81.369 E.06896
G1 X92.828 Y81.369 E.01111
G1 X92.828 Y83.868 E.07366
G1 X93.205 Y84.055 E.01241
G1 X93.205 Y81.369 E.07918
G1 X93.582 Y81.369 E.01111
G1 X93.582 Y84.276 E.08569
G1 X93.959 Y84.53 E.01339
G1 X93.959 Y81.369 E.09316
G1 X94.336 Y81.369 E.01111
G1 X94.336 Y84.823 E.10182
G1 X94.713 Y85.164 E.01499
G1 X94.713 Y81.369 E.11187
G1 X95.09 Y81.369 E.01111
G1 X95.09 Y85.564 E.12364
G1 X95.467 Y86.037 E.01783
G1 X95.467 Y81.369 E.13758
G1 X95.754 Y81.369 E.00845
G1 X95.754 Y84.246 E.0848
G1 X95.845 Y84.246 E.00266
G1 X95.845 Y86.618 E.06992
G1 X96.103 Y87.107 E.0163
G1 X96.222 Y87.374 E.0086
G1 X96.222 Y84.076 E.09719
; WIPE_START
G1 X96.222 Y85.076 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I.083 J-1.214 P1  F42000
G1 X81.516 Y84.076 Z2.7
G1 Z2.3
G1 E.4 F1800
G1 F4800
M204 S6000
G1 X81.516 Y95.754 E.34419
G1 X81.893 Y95.754 E.01111
G1 X81.893 Y84.246 E.33919
G1 X82.27 Y84.246 E.01111
G1 X82.27 Y95.754 E.33919
G1 X82.647 Y95.754 E.01111
G1 X82.647 Y84.246 E.33919
G1 X83.024 Y84.246 E.01111
G1 X83.024 Y95.924 E.34419
; WIPE_START
G1 X83.024 Y94.924 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-.094 J1.213 P1  F42000
G1 X94.893 Y95.839 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
; FEATURE: Support ironing
; LAYER_HEIGHT: 0.03
G1 F4200
M204 S6000
G1 X94.893 Y97.77 E.00936
G1 X91.36 Y97.77 E.01713
G1 X91.334 Y97.499 E.00132
G1 X91.834 Y97.391 E.00248
G1 X92.227 Y97.283 E.00198
G1 X92.9 Y97.042 E.00346
G1 X93.241 Y96.891 E.00181
G1 X93.605 Y96.708 E.00197
G1 X94.217 Y96.341 E.00346
G1 X94.893 Y95.839 E.00408
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X94.593 Y96.458 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X93.75 Y96.971 E.00479
G1 X93.213 Y97.232 E.00289
G1 X93.263 Y97.47 E.00118
G1 X94.593 Y97.47 E.00645
G1 X94.593 Y96.458 E.0049
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X94.593 Y97.458 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I1.034 J.642 P1  F42000
G1 X97.232 Y93.213 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X96.972 Y93.749 E.00289
G1 X96.458 Y94.593 E.00479
G1 X97.47 Y94.593 E.0049
G1 X97.47 Y93.263 E.00645
G1 X97.232 Y93.213 E.00118
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X97.47 Y93.263 E-.09238
G1 X97.47 Y94.02 E-.28762
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I1.217 J.013 P1  F42000
G1 X97.499 Y91.334 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X97.77 Y91.361 E.00132
G1 X97.77 Y94.893 E.01713
G1 X95.839 Y94.893 E.00936
G1 X96.341 Y94.217 E.00408
G1 X96.709 Y93.604 E.00347
G1 X96.891 Y93.242 E.00197
G1 X97.165 Y92.581 E.00347
G1 X97.283 Y92.226 E.00181
G1 X97.391 Y91.836 E.00197
G1 X97.499 Y91.334 E.00249
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X97.391 Y91.836 E-.19488
G1 X97.283 Y92.226 E-.154
G1 X97.257 Y92.304 E-.03111
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I1.209 J-.14 P1  F42000
G1 X96.459 Y85.407 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X96.971 Y86.25 E.00478
G1 X97.232 Y86.787 E.00289
G1 X97.47 Y86.737 E.00118
M73 P54 R7
G1 X97.47 Y85.407 E.00645
G1 X96.459 Y85.407 E.0049
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X95.839 Y85.107 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X96.341 Y85.783 E.00408
G1 X96.708 Y86.395 E.00346
G1 X96.892 Y86.759 E.00198
G1 X97.165 Y87.419 E.00346
G1 X97.283 Y87.774 E.00181
G1 X97.391 Y88.166 E.00198
G1 X97.499 Y88.666 E.00248
G1 X97.77 Y88.639 E.00132
G1 X97.77 Y85.107 E.01713
G1 X95.839 Y85.107 E.00936
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X96.839 Y85.107 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I.66 J-1.023 P1  F42000
G1 X93.213 Y82.768 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X93.263 Y82.53 E.00118
G1 X94.593 Y82.53 E.00645
G1 X94.593 Y83.541 E.0049
G1 X93.75 Y83.029 E.00478
G1 X93.213 Y82.768 E.00289
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X93.75 Y83.029 E-.22683
G1 X94.094 Y83.238 E-.15317
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I.314 J-1.176 P1  F42000
G1 X91.334 Y82.501 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X91.835 Y82.609 E.00248
G1 X92.226 Y82.717 E.00197
G1 X92.581 Y82.835 E.00181
G1 X93.241 Y83.109 E.00347
G1 X93.605 Y83.292 E.00197
G1 X94.217 Y83.659 E.00346
G1 X94.893 Y84.161 E.00408
G1 X94.893 Y82.23 E.00936
G1 X91.361 Y82.23 E.01713
G1 X91.334 Y82.501 E.00132
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X91.361 Y82.23 E-.10331
G1 X92.089 Y82.23 E-.27669
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-.234 J-1.194 P1  F42000
G1 X85.407 Y83.542 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.407 Y82.53 E.0049
G1 X86.737 Y82.53 E.00645
G1 X86.787 Y82.768 E.00118
G1 X86.25 Y83.029 E.00289
G1 X85.407 Y83.542 E.00479
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X85.107 Y84.161 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.783 Y83.659 E.00408
G1 X86.395 Y83.292 E.00346
G1 X86.759 Y83.109 E.00197
G1 X87.419 Y82.835 E.00347
G1 X87.774 Y82.717 E.00181
G1 X88.165 Y82.609 E.00197
G1 X88.666 Y82.501 E.00248
G1 X88.639 Y82.23 E.00132
G1 X85.107 Y82.23 E.01712
G1 X85.107 Y84.161 E.00936
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X85.652 Y84.117 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.097 Y84.571 E.00348
G1 X84.588 Y85.078 E.00348
G1 X84.132 Y85.632 E.00348
G1 X83.732 Y86.228 E.00348
G1 X83.393 Y86.86 E.00348
G1 X83.117 Y87.523 E.00348
G1 X82.907 Y88.209 E.00348
G1 X82.766 Y88.914 E.00348
G1 X82.694 Y89.628 E.00348
G1 X82.693 Y90.346 E.00348
G1 X82.762 Y91.061 E.00348
G1 X82.901 Y91.765 E.00348
G1 X83.108 Y92.452 E.00348
G1 X83.381 Y93.116 E.00348
G1 X83.719 Y93.75 E.00348
G1 X84.116 Y94.347 E.00347
G1 X84.571 Y94.903 E.00349
G1 X85.078 Y95.412 E.00348
G1 X85.632 Y95.868 E.00348
G1 X86.228 Y96.268 E.00348
G1 X86.861 Y96.608 E.00348
G1 X87.523 Y96.883 E.00348
G1 X88.21 Y97.093 E.00348
G1 X88.914 Y97.235 E.00348
G1 X89.628 Y97.306 E.00348
G1 X90.346 Y97.307 E.00348
G1 X91.061 Y97.238 E.00348
G1 X91.765 Y97.099 E.00348
G1 X92.452 Y96.892 E.00348
G1 X93.116 Y96.619 E.00348
G1 X93.749 Y96.282 E.00348
G1 X94.348 Y95.883 E.00349
G1 X94.903 Y95.429 E.00348
G1 X95.411 Y94.923 E.00348
G1 X95.869 Y94.368 E.00349
G1 X96.268 Y93.772 E.00348
G1 X96.607 Y93.14 E.00347
G1 X96.884 Y92.476 E.00349
G1 X97.093 Y91.79 E.00348
G1 X97.234 Y91.086 E.00348
G1 X97.306 Y90.372 E.00348
G1 X97.307 Y89.654 E.00348
G1 X97.238 Y88.94 E.00348
G1 X97.099 Y88.235 E.00348
G1 X96.892 Y87.548 E.00348
G1 X96.619 Y86.884 E.00348
G1 X96.281 Y86.25 E.00348
G1 X95.884 Y85.653 E.00348
G1 X95.429 Y85.097 E.00348
G1 X94.922 Y84.588 E.00348
G1 X94.367 Y84.131 E.00349
G1 X93.772 Y83.732 E.00348
G1 X93.14 Y83.393 E.00348
G1 X92.476 Y83.116 E.00349
G1 X91.79 Y82.907 E.00348
G1 X91.086 Y82.766 E.00348
G1 X90.372 Y82.694 E.00348
G1 X89.654 Y82.693 E.00348
G1 X88.94 Y82.762 E.00348
G1 X88.235 Y82.901 E.00348
G1 X87.548 Y83.108 E.00348
G1 X86.884 Y83.381 E.00348
G1 X86.25 Y83.719 E.00348
G1 X85.652 Y84.117 E.00348
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X85.828 Y84.36 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X86.401 Y83.978 E.00334
G1 X87.009 Y83.654 E.00334
G1 X87.628 Y83.398 E.00325
G1 X88.286 Y83.197 E.00334
G1 X88.961 Y83.062 E.00334
G1 X89.646 Y82.993 E.00334
G1 X90.335 Y82.992 E.00334
G1 X91.02 Y83.059 E.00334
G1 X91.696 Y83.193 E.00334
G1 X92.355 Y83.391 E.00334
G1 X92.991 Y83.654 E.00334
G1 X93.598 Y83.978 E.00334
G1 X94.186 Y84.37 E.00343
G1 X94.705 Y84.796 E.00325
G1 X95.192 Y85.282 E.00334
G1 X95.63 Y85.814 E.00334
G1 X96.013 Y86.385 E.00334
G1 X96.338 Y86.992 E.00334
G1 X96.602 Y87.628 E.00334
G1 X96.803 Y88.287 E.00334
G1 X96.938 Y88.961 E.00334
G1 X97.007 Y89.647 E.00334
G1 X97.008 Y90.335 E.00334
G1 X96.941 Y91.02 E.00334
G1 X96.807 Y91.696 E.00334
G1 X96.602 Y92.372 E.00342
G1 X96.338 Y93.008 E.00334
G1 X96.013 Y93.614 E.00334
G1 X95.63 Y94.186 E.00334
G1 X95.192 Y94.718 E.00334
G1 X94.705 Y95.204 E.00334
G1 X94.172 Y95.64 E.00334
G1 X93.598 Y96.022 E.00334
G1 X92.991 Y96.346 E.00334
G1 X92.355 Y96.609 E.00334
G1 X91.696 Y96.808 E.00334
G1 X91.02 Y96.941 E.00334
G1 X90.335 Y97.008 E.00334
G1 X89.646 Y97.007 E.00334
G1 X88.962 Y96.938 E.00334
G1 X88.286 Y96.803 E.00334
G1 X87.628 Y96.602 E.00334
G1 X86.992 Y96.338 E.00334
G1 X86.386 Y96.013 E.00334
G1 X85.828 Y95.64 E.00325
G1 X85.295 Y95.204 E.00334
G1 X84.808 Y94.718 E.00334
G1 X84.37 Y94.186 E.00334
G1 X83.987 Y93.615 E.00334
G1 X83.662 Y93.008 E.00334
G1 X83.398 Y92.372 E.00334
G1 X83.197 Y91.714 E.00333
G1 X83.062 Y91.039 E.00334
G1 X82.993 Y90.354 E.00334
G1 X82.993 Y89.646 E.00343
G1 X83.062 Y88.961 E.00334
G1 X83.197 Y88.286 E.00334
G1 X83.398 Y87.628 E.00334
G1 X83.654 Y87.009 E.00325
G1 X83.987 Y86.385 E.00343
G1 X84.371 Y85.813 E.00334
G1 X84.808 Y85.282 E.00334
G1 X85.295 Y84.796 E.00334
G1 X85.828 Y84.36 E.00334
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X86.004 Y84.603 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X86.552 Y84.237 E.0032
G1 X87.134 Y83.927 E.0032
G1 X87.743 Y83.675 E.0032
G1 X88.374 Y83.484 E.00319
G1 X89.02 Y83.356 E.0032
G1 X89.676 Y83.292 E.00319
G1 X90.335 Y83.293 E.0032
G1 X90.991 Y83.358 E.0032
G1 X91.637 Y83.487 E.0032
G1 X92.268 Y83.679 E.00319
G1 X92.876 Y83.932 E.0032
G1 X93.457 Y84.243 E.00319
G1 X94.005 Y84.609 E.0032
G1 X94.506 Y85.02 E.00314
G1 X94.972 Y85.486 E.0032
G1 X95.391 Y85.995 E.0032
G1 X95.757 Y86.543 E.00319
G1 X96.068 Y87.124 E.0032
G1 X96.321 Y87.732 E.00319
G1 X96.513 Y88.363 E.0032
G1 X96.642 Y89.009 E.0032
G1 X96.707 Y89.665 E.00319
G1 X96.707 Y90.335 E.00325
G1 X96.642 Y90.991 E.0032
G1 X96.513 Y91.637 E.0032
G1 X96.321 Y92.268 E.00319
G1 X96.068 Y92.876 E.0032
G1 X95.757 Y93.457 E.00319
G1 X95.391 Y94.005 E.00319
G1 X94.972 Y94.514 E.0032
G1 X94.506 Y94.98 E.0032
G1 X93.996 Y95.397 E.0032
G1 X93.448 Y95.763 E.0032
G1 X92.866 Y96.073 E.00319
G1 X92.257 Y96.325 E.0032
G1 X91.627 Y96.516 E.0032
G1 X90.98 Y96.644 E.0032
G1 X90.324 Y96.708 E.0032
G1 X89.665 Y96.707 E.0032
G1 X89.009 Y96.642 E.00319
G1 X88.363 Y96.513 E.0032
G1 X87.732 Y96.321 E.00319
G1 X87.124 Y96.068 E.0032
G1 X86.543 Y95.757 E.0032
G1 X86.004 Y95.397 E.00314
G1 X85.494 Y94.98 E.0032
G1 X85.028 Y94.514 E.00319
G1 X84.609 Y94.005 E.0032
G1 X84.243 Y93.457 E.00319
G1 X83.932 Y92.876 E.00319
G1 X83.679 Y92.268 E.0032
G1 X83.487 Y91.637 E.00319
G1 X83.358 Y90.991 E.0032
G1 X83.293 Y90.335 E.00319
G1 X83.292 Y89.676 E.0032
G1 X83.356 Y89.02 E.0032
G1 X83.487 Y88.363 E.00325
G1 X83.679 Y87.732 E.00319
G1 X83.932 Y87.124 E.0032
G1 X84.243 Y86.543 E.0032
G1 X84.609 Y85.995 E.00319
G1 X85.028 Y85.486 E.00319
G1 X85.494 Y85.02 E.0032
G1 X86.004 Y84.603 E.0032
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X85.494 Y85.02 E-.25042
G1 X85.253 Y85.261 E-.12958
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I.868 J-.853 P1  F42000
G1 X84.807 Y84.807 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X81.93 Y84.807 E.01395
G1 X81.93 Y95.193 E.05036
G1 X84.807 Y95.193 E.01395
G1 X84.807 Y98.07 E.01395
G1 X95.193 Y98.07 E.05036
G1 X95.193 Y95.193 E.01395
G1 X98.07 Y95.193 E.01395
G1 X98.07 Y84.807 E.05036
G1 X95.193 Y84.807 E.01395
G1 X95.193 Y81.93 E.01395
G1 X84.807 Y81.93 E.05036
G1 X84.807 Y84.807 E.01395
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.507 Y84.507 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X84.507 Y81.63 E.01395
G1 X95.493 Y81.63 E.05327
G1 X95.493 Y84.507 E.01395
G1 X98.37 Y84.507 E.01395
G1 X98.37 Y95.493 E.05327
G1 X95.493 Y95.493 E.01395
G1 X95.493 Y98.37 E.01395
G1 X84.507 Y98.37 E.05327
G1 X84.507 Y95.493 E.01395
G1 X81.63 Y95.493 E.01395
G1 X81.63 Y84.507 E.05327
G1 X84.507 Y84.507 E.01395
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.207 Y84.207 F42000
M106 S127.5
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X81.33 Y84.207 E.01395
G1 X81.33 Y95.793 E.05618
G1 X84.207 Y95.793 E.01395
G1 X84.207 Y98.67 E.01395
G1 X95.793 Y98.67 E.05618
G1 X95.793 Y95.793 E.01395
G1 X98.67 Y95.793 E.01395
G1 X98.67 Y84.207 E.05618
G1 X95.793 Y84.207 E.01395
G1 X95.793 Y81.33 E.01395
G1 X84.207 Y81.33 E.05618
G1 X84.207 Y84.207 E.01395
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X84.207 Y83.207 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-1.129 J-.454 P1  F42000
G1 X82.768 Y86.787 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X82.53 Y86.737 E.00118
G1 X82.53 Y85.407 E.00645
G1 X83.541 Y85.407 E.0049
G1 X83.029 Y86.251 E.00479
G1 X82.768 Y86.787 E.00289
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X83.029 Y86.251 E-.22666
G1 X83.238 Y85.906 E-.15334
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-1.176 J-.314 P1  F42000
G1 X82.501 Y88.666 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X82.23 Y88.639 E.00132
G1 X82.23 Y85.107 E.01713
G1 X84.161 Y85.107 E.00936
G1 X83.659 Y85.783 E.00408
G1 X83.292 Y86.396 E.00346
G1 X83.109 Y86.758 E.00197
G1 X82.958 Y87.101 E.00182
G1 X82.717 Y87.773 E.00346
G1 X82.608 Y88.167 E.00198
G1 X82.501 Y88.666 E.00248
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X82.608 Y88.167 E-.19411
G1 X82.717 Y87.773 E-.15514
G1 X82.745 Y87.697 E-.03075
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-1.209 J.14 P1  F42000
G1 X83.542 Y94.593 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X82.53 Y94.593 E.0049
G1 X82.53 Y93.263 E.00645
G1 X82.768 Y93.213 E.00118
G1 X83.029 Y93.749 E.00289
G1 X83.542 Y94.593 E.00479
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X84.161 Y94.893 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X83.659 Y94.218 E.00408
G1 X83.292 Y93.605 E.00347
G1 X83.109 Y93.241 E.00197
G1 X82.835 Y92.581 E.00346
G1 X82.717 Y92.227 E.00181
G1 X82.609 Y91.834 E.00198
G1 X82.501 Y91.334 E.00248
G1 X82.23 Y91.361 E.00132
G1 X82.23 Y94.893 E.01713
G1 X84.161 Y94.893 E.00936
M1031 S0 ;IRONING_EXTRUSIONS_END
; WIPE_START
G1 X83.161 Y94.893 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-.532 J1.094 P1  F42000
G1 X85.107 Y95.839 Z2.7
G1 Z2.3
G1 E.4 F1800
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X85.107 Y97.77 E.00936
G1 X88.639 Y97.77 E.01713
G1 X88.666 Y97.499 E.00132
G1 X88.166 Y97.392 E.00248
G1 X87.773 Y97.283 E.00198
G1 X87.101 Y97.042 E.00346
G1 X86.759 Y96.891 E.00181
G1 X86.395 Y96.708 E.00198
G1 X85.783 Y96.341 E.00346
G1 X85.107 Y95.839 E.00408
M1031 S0 ;IRONING_EXTRUSIONS_END
M204 S10000
G1 X85.407 Y96.459 F42000
M1031 S1 ;IRONING_EXTRUSIONS_START
G1 F4200
M204 S6000
G1 X86.25 Y96.971 E.00478
G1 X86.787 Y97.232 E.00289
G1 X86.737 Y97.47 E.00118
G1 X85.407 Y97.47 E.00645
G1 X85.407 Y96.459 E.0049
M1031 S0 ;IRONING_EXTRUSIONS_END
; COOLING_NODE: 5
; WIPE_START
G1 X85.407 Y97.459 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I1.103 J.514 P1  F42000
G1 X91.432 Y84.524 Z2.7
G1 Z2.3
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
; LAYER_HEIGHT: 0.2
G1 F9580.435
M204 S6000
G1 X91.908 Y84.668 E.01583
G1 X92.421 Y84.881 E.01768
G1 X92.911 Y85.143 E.01768
G1 X93.373 Y85.451 E.01768
G1 X93.803 Y85.804 E.01769
G1 X94.196 Y86.197 E.01769
G1 X94.549 Y86.627 E.01769
G1 X94.857 Y87.089 E.01768
G1 X95.119 Y87.579 E.01768
G1 X95.332 Y88.092 E.01768
G1 X95.493 Y88.624 E.01768
G1 X95.602 Y89.169 E.01769
G1 X95.656 Y89.722 E.01768
G1 X95.656 Y90.278 E.01768
G1 X95.602 Y90.831 E.01768
G1 X95.493 Y91.376 E.01769
G1 X95.332 Y91.908 E.01768
G1 X95.119 Y92.421 E.01769
G1 X94.857 Y92.911 E.01768
G1 X94.549 Y93.373 E.01767
G1 X94.196 Y93.803 E.01769
G1 X93.803 Y94.196 E.01768
G1 X93.373 Y94.549 E.01768
G1 X92.911 Y94.857 E.01768
G1 X92.421 Y95.119 E.01768
G1 X91.908 Y95.332 E.01768
G1 X91.376 Y95.493 E.01768
G1 X90.831 Y95.602 E.01769
G1 X90.278 Y95.656 E.01768
G1 X89.722 Y95.656 E.01769
G1 X89.169 Y95.602 E.01768
G1 X88.624 Y95.493 E.01769
G1 X88.092 Y95.332 E.01768
G1 X87.579 Y95.119 E.01768
G1 X87.089 Y94.857 E.01768
G1 X86.627 Y94.549 E.01768
G1 X86.197 Y94.196 E.01769
G1 X85.804 Y93.803 E.01768
G1 X85.451 Y93.373 E.01769
G1 X85.143 Y92.911 E.01768
G1 X84.881 Y92.421 E.01768
G1 X84.668 Y91.908 E.01769
G1 X84.507 Y91.376 E.01767
G1 X84.398 Y90.831 E.01769
G1 X84.344 Y90.278 E.01768
G1 X84.344 Y89.722 E.01769
G1 X84.398 Y89.169 E.01768
G1 X84.507 Y88.624 E.01769
G1 X84.668 Y88.092 E.01768
G1 X84.881 Y87.579 E.01768
G1 X85.143 Y87.089 E.01769
G1 X85.452 Y86.627 E.01768
G1 X85.804 Y86.197 E.01767
G1 X86.197 Y85.804 E.0177
G1 X86.627 Y85.451 E.01768
G1 X87.089 Y85.143 E.01768
G1 X87.579 Y84.881 E.01768
G1 X88.092 Y84.668 E.01768
G1 X88.624 Y84.507 E.01768
G1 X89.169 Y84.398 E.01769
G1 X89.724 Y84.344 E.01774
G1 X90.156 Y84.341 E.01375
G1 X90.835 Y84.399 E.02167
G1 X91.374 Y84.507 E.0175
; COOLING_NODE: 5
M204 S250
G1 X91.546 Y84.149 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X92.04 Y84.299 E.01523
G1 X92.589 Y84.526 E.01751
G1 X93.113 Y84.806 E.01751
G1 X93.607 Y85.136 E.01751
G1 X94.067 Y85.513 E.01752
G1 X94.487 Y85.933 E.01752
G1 X94.864 Y86.393 E.01752
G1 X95.194 Y86.887 E.01751
G1 X95.474 Y87.411 E.01751
G1 X95.701 Y87.96 E.01751
G1 X95.874 Y88.529 E.01751
G1 X95.99 Y89.112 E.01752
G1 X96.048 Y89.703 E.01751
G1 X96.048 Y90.297 E.01751
G1 X95.99 Y90.888 E.01751
G1 X95.874 Y91.471 E.01752
G1 X95.701 Y92.04 E.01751
G1 X95.474 Y92.589 E.01752
G1 X95.194 Y93.113 E.01752
G1 X94.864 Y93.607 E.01751
G1 X94.487 Y94.067 E.01752
G1 X94.067 Y94.487 E.01751
G1 X93.607 Y94.864 E.01751
G1 X93.113 Y95.194 E.01752
G1 X92.589 Y95.474 E.01751
G1 X92.04 Y95.701 E.01751
G1 X91.471 Y95.874 E.01751
G1 X90.888 Y95.99 E.01752
G1 X90.297 Y96.048 E.01751
G1 X89.703 Y96.048 E.01752
G1 X89.112 Y95.99 E.01751
G1 X88.529 Y95.874 E.01752
G1 X87.96 Y95.701 E.01751
G1 X87.411 Y95.474 E.01751
G1 X86.887 Y95.194 E.01751
G1 X86.393 Y94.864 E.01751
G1 X85.933 Y94.487 E.01752
G1 X85.513 Y94.067 E.01751
G1 X85.136 Y93.607 E.01752
G1 X84.806 Y93.113 E.01751
G1 X84.526 Y92.589 E.01751
G1 X84.299 Y92.04 E.01752
G1 X84.126 Y91.471 E.01751
G1 X84.01 Y90.888 E.01752
G1 X83.952 Y90.297 E.01751
G1 X83.952 Y89.703 E.01752
G1 X84.01 Y89.112 E.01751
G1 X84.126 Y88.529 E.01752
G1 X84.299 Y87.96 E.01751
G1 X84.526 Y87.411 E.01751
G1 X84.806 Y86.887 E.01752
G1 X85.136 Y86.393 E.01751
G1 X85.513 Y85.934 E.01751
M73 P55 R7
G1 X85.934 Y85.513 E.01753
G1 X86.393 Y85.136 E.01751
G1 X86.887 Y84.806 E.01751
G1 X87.411 Y84.526 E.01752
G1 X87.96 Y84.299 E.01751
G1 X88.529 Y84.126 E.01751
G1 X89.112 Y84.01 E.01752
G1 X89.703 Y83.952 E.01753
G1 X90.172 Y83.949 E.0138
G1 X90.89 Y84.01 E.02124
G1 X91.471 Y84.126 E.01748
G1 X91.488 Y84.131 E.00051
M106 S112.2
; WIPE_START
M204 S6000
G1 X92.04 Y84.299 E-.21916
G1 X92.431 Y84.461 E-.16084
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z2.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z2.7 F4000
            G39.3 S1
            G0 Z2.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.29 Y84.765 F42000
G1 Z2.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.439712
G1 F9828.827
M204 S6000
G1 X89.754 Y84.75 E.01663
G1 X89.237 Y84.793 E.01609
G1 X88.732 Y84.898 E.01601
G1 X88.236 Y85.042 E.01601
G1 X87.762 Y85.243 E.01599
G1 X87.303 Y85.481 E.01603
G1 X86.876 Y85.772 E.016
G1 X86.473 Y86.094 E.01601
G1 X86.111 Y86.462 E.01601
G1 X85.779 Y86.857 E.01601
G1 X85.496 Y87.289 E.016
G1 X85.247 Y87.741 E.01603
G1 X85.054 Y88.22 E.01599
G1 X84.897 Y88.712 E.01602
G1 X84.801 Y89.219 E.01601
G1 X84.744 Y89.732 E.01601
G1 X84.749 Y90.248 E.016
G1 X84.793 Y90.763 E.01601
G1 X84.898 Y91.268 E.01601
G1 X85.042 Y91.764 E.016
G1 X85.243 Y92.239 E.016
G1 X85.481 Y92.697 E.01602
G1 X85.772 Y93.124 E.016
G1 X86.094 Y93.527 E.01602
G1 X86.462 Y93.889 E.016
G1 X86.857 Y94.221 E.01602
G1 X87.289 Y94.504 E.016
G1 X87.741 Y94.753 E.01602
G1 X88.22 Y94.946 E.01599
G1 X88.712 Y95.103 E.01602
G1 X89.219 Y95.199 E.01601
G1 X89.732 Y95.256 E.01601
G1 X90.248 Y95.251 E.016
G1 X90.763 Y95.207 E.01602
G1 X91.268 Y95.102 E.016
G1 X91.764 Y94.958 E.01602
G1 X92.239 Y94.757 E.016
G1 X92.697 Y94.519 E.01602
G1 X93.124 Y94.228 E.01601
G1 X93.527 Y93.906 E.01601
G1 X93.889 Y93.538 E.01601
G1 X94.221 Y93.143 E.01602
G1 X94.504 Y92.711 E.01598
G1 X94.753 Y92.259 E.01603
G1 X94.946 Y91.78 E.016
G1 X95.103 Y91.288 E.01601
G1 X95.199 Y90.781 E.01601
G1 X95.256 Y90.268 E.01602
G1 X95.251 Y89.752 E.016
G1 X95.207 Y89.237 E.01601
G1 X95.102 Y88.732 E.01601
G1 X94.958 Y88.236 E.01602
G1 X94.757 Y87.761 E.016
G1 X94.519 Y87.303 E.01602
G1 X94.228 Y86.876 E.016
G1 X93.906 Y86.473 E.01601
G1 X93.538 Y86.111 E.01601
G1 X93.143 Y85.779 E.01601
G1 X92.711 Y85.496 E.016
G1 X92.259 Y85.247 E.01602
G1 X91.78 Y85.054 E.01599
G1 X91.288 Y84.897 E.01602
G1 X90.796 Y84.804 E.01555
G1 X90.35 Y84.77 E.01385
M204 S10000
G1 X90.727 Y85.201 F42000
; LINE_WIDTH: 0.434241
G1 F9966.26
M204 S6000
G1 X91.206 Y85.281 E.01487
G1 X92.069 Y85.591 E.02803
G1 X92.889 Y86.079 E.0292
G1 X93.599 Y86.718 E.02921
G1 X94.17 Y87.483 E.0292
G1 X94.581 Y88.345 E.02921
G1 X94.816 Y89.27 E.0292
G1 X94.866 Y90.224 E.0292
G1 X94.728 Y91.169 E.02921
G1 X94.41 Y92.069 E.0292
G1 X93.921 Y92.889 E.0292
G1 X93.282 Y93.599 E.02921
G1 X92.517 Y94.17 E.0292
G1 X91.655 Y94.581 E.02921
G1 X90.729 Y94.816 E.02921
G1 X89.776 Y94.866 E.02919
G1 X88.831 Y94.728 E.02921
G1 X87.931 Y94.41 E.0292
G1 X87.111 Y93.921 E.0292
G1 X86.401 Y93.282 E.02921
G1 X85.83 Y92.517 E.0292
G1 X85.419 Y91.655 E.02921
G1 X85.184 Y90.73 E.0292
G1 X85.134 Y89.776 E.0292
G1 X85.272 Y88.831 E.02922
G1 X85.602 Y87.905 E.03007
G1 X86.079 Y87.111 E.02834
G1 X86.718 Y86.401 E.02921
G1 X87.483 Y85.83 E.0292
G1 X88.345 Y85.419 E.02921
G1 X89.27 Y85.184 E.02921
G1 X90.216 Y85.14 E.02895
G1 X90.667 Y85.194 E.0139
; WIPE_START
G1 X90.216 Y85.14 E-.17264
G1 X89.671 Y85.166 E-.20736
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.7 I-1.015 J.671 P1  F42000
G1 X94.078 Y91.827 Z2.7
G1 Z2.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F9580.435
M204 S6000
G1 X93.836 Y92.299 E.01689
G1 X93.592 Y92.664 E.01396
G1 X93.314 Y93.003 E.01397
G1 X93.159 Y93.159 E.00698
G1 X86.841 Y86.841 E.28426
G1 X86.997 Y86.686 E.00699
G1 X87.336 Y86.408 E.01396
G1 X87.701 Y86.164 E.01396
G1 X88.088 Y85.957 E.01397
G1 X88.493 Y85.789 E.01396
G1 X88.913 Y85.662 E.01397
G1 X89.344 Y85.576 E.01396
G1 X89.781 Y85.533 E.01397
G1 X90.212 Y85.533 E.01371
G1 X90.657 Y85.576 E.01423
G1 X91.087 Y85.662 E.01394
G1 X91.507 Y85.789 E.01396
G1 X91.912 Y85.957 E.01397
G1 X92.299 Y86.164 E.01396
G1 X92.664 Y86.408 E.01397
G1 X93.004 Y86.686 E.01397
G1 X93.159 Y86.841 E.00698
G1 X86.841 Y93.159 E.28426
G1 X86.686 Y93.003 E.00698
G1 X86.408 Y92.664 E.01396
G1 X86.164 Y92.299 E.01397
G1 X85.922 Y91.827 E.01689
M106 S127.5
; CHANGE_LAYER
; Z_HEIGHT: 2.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X86.164 Y92.299 E-.20167
G1 X86.408 Y92.664 E-.1668
G1 X86.427 Y92.688 E-.01153
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 13/43
; update layer progress
M73 L13
M991 S0 P12 ;notify layer change
M106 S127.5
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z2.7 I1.044 J.625 P1  F42000
G1 X91.52 Y84.186 Z2.7
G1 Z2.5
G1 E.4 F1800
; FEATURE: Inner wall
G1 F1172
M204 S6000
G1 X92.025 Y84.34 E.01679
G1 X92.57 Y84.565 E.01877
G1 X93.091 Y84.844 E.01877
G1 X93.581 Y85.171 E.01877
G1 X94.037 Y85.546 E.01877
G1 X94.454 Y85.963 E.01877
G1 X94.829 Y86.419 E.01877
G1 X95.156 Y86.909 E.01877
G1 X95.435 Y87.43 E.01878
G1 X95.66 Y87.975 E.01877
G1 X95.832 Y88.539 E.01877
G1 X95.947 Y89.118 E.01877
G1 X96.004 Y89.705 E.01877
G1 X96.005 Y90.295 E.01877
G1 X95.947 Y90.882 E.01877
G1 X95.832 Y91.461 E.01878
G1 X95.66 Y92.025 E.01877
G1 X95.435 Y92.57 E.01877
G1 X95.156 Y93.091 E.01877
G1 X94.829 Y93.581 E.01877
G1 X94.454 Y94.037 E.01878
G1 X94.037 Y94.455 E.01878
G1 X93.581 Y94.829 E.01876
G1 X93.091 Y95.156 E.01877
G1 X92.57 Y95.435 E.01878
G1 X92.025 Y95.66 E.01877
G1 X91.461 Y95.832 E.01877
G1 X90.882 Y95.947 E.01877
G1 X90.295 Y96.004 E.01877
G1 X89.705 Y96.005 E.01877
G1 X89.118 Y95.947 E.01878
G1 X88.539 Y95.832 E.01877
G1 X87.975 Y95.66 E.01877
G1 X87.43 Y95.435 E.01877
G1 X86.909 Y95.156 E.01877
G1 X86.419 Y94.829 E.01877
G1 X85.963 Y94.454 E.01877
G1 X85.545 Y94.037 E.01878
G1 X85.171 Y93.581 E.01877
G1 X84.844 Y93.091 E.01877
G1 X84.565 Y92.57 E.01878
G1 X84.34 Y92.025 E.01876
G1 X84.168 Y91.461 E.01877
G1 X84.053 Y90.882 E.01878
G1 X83.996 Y90.295 E.01877
G1 X83.995 Y89.705 E.01877
G1 X84.053 Y89.118 E.01877
G1 X84.168 Y88.539 E.01877
G1 X84.34 Y87.975 E.01877
G1 X84.565 Y87.43 E.01877
G1 X84.844 Y86.909 E.01877
G1 X85.171 Y86.419 E.01876
G1 X85.546 Y85.963 E.01878
G1 X85.963 Y85.545 E.01878
G1 X86.419 Y85.171 E.01876
G1 X86.909 Y84.844 E.01878
G1 X87.43 Y84.565 E.01877
G1 X87.975 Y84.34 E.01877
G1 X88.539 Y84.168 E.01877
G1 X89.118 Y84.053 E.01877
G1 X89.705 Y83.996 E.01877
G1 X90.285 Y83.995 E.01844
G1 X90.882 Y84.053 E.01911
G1 X91.461 Y84.168 E.01876
G1 X91.463 Y84.169 E.00007
; COOLING_NODE: 5
M204 S250
G1 X91.634 Y83.811 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F600
M204 S5000
G1 X92.158 Y83.97 E.01612
G1 X92.738 Y84.211 E.01853
G1 X93.293 Y84.507 E.01853
G1 X93.815 Y84.856 E.01852
G1 X94.301 Y85.255 E.01852
G1 X94.745 Y85.699 E.01852
G1 X95.144 Y86.185 E.01852
G1 X95.493 Y86.707 E.01852
G1 X95.789 Y87.262 E.01853
G1 X96.03 Y87.842 E.01852
G1 X96.212 Y88.444 E.01852
G1 X96.335 Y89.06 E.01853
G1 X96.397 Y89.686 E.01852
G1 X96.397 Y90.314 E.01852
G1 X96.335 Y90.94 E.01853
G1 X96.212 Y91.556 E.01852
G1 X96.03 Y92.158 E.01852
G1 X95.789 Y92.738 E.01852
G1 X95.493 Y93.292 E.01852
G1 X95.144 Y93.815 E.01852
G1 X94.745 Y94.301 E.01853
G1 X94.301 Y94.745 E.01853
G1 X93.815 Y95.144 E.01851
G1 X93.293 Y95.493 E.01852
G1 X92.738 Y95.789 E.01853
G1 X92.158 Y96.03 E.01852
G1 X91.556 Y96.212 E.01852
G1 X90.94 Y96.335 E.01853
G1 X90.314 Y96.397 E.01852
G1 X89.686 Y96.397 E.01852
G1 X89.06 Y96.335 E.01853
G1 X88.444 Y96.212 E.01853
G1 X87.842 Y96.03 E.01852
G1 X87.262 Y95.789 E.01852
G1 X86.708 Y95.493 E.01852
G1 X86.185 Y95.144 E.01852
G1 X85.699 Y94.745 E.01852
G1 X85.255 Y94.301 E.01853
G1 X84.856 Y93.815 E.01852
G1 X84.507 Y93.292 E.01852
G1 X84.211 Y92.738 E.01853
G1 X83.97 Y92.158 E.01852
G1 X83.788 Y91.556 E.01852
G1 X83.665 Y90.94 E.01853
G1 X83.603 Y90.314 E.01852
G1 X83.603 Y89.686 E.01852
G1 X83.665 Y89.06 E.01853
G1 X83.788 Y88.444 E.01853
G1 X83.97 Y87.843 E.01852
G1 X84.211 Y87.262 E.01853
G1 X84.507 Y86.707 E.01853
G1 X84.856 Y86.185 E.01851
G1 X85.255 Y85.699 E.01853
G1 X85.699 Y85.255 E.01853
G1 X86.185 Y84.856 E.01852
G1 X86.708 Y84.507 E.01853
G1 X87.262 Y84.211 E.01852
G1 X87.842 Y83.97 E.01852
G1 X88.444 Y83.788 E.01852
G1 X89.06 Y83.665 E.01853
G1 X89.686 Y83.603 E.01852
G1 X90.303 Y83.603 E.01821
G1 X90.94 Y83.665 E.01884
G1 X91.556 Y83.788 E.01852
G1 X91.577 Y83.794 E.00063
M106 S124.95
; WIPE_START
M204 S6000
G1 X92.158 Y83.97 E-.23068
G1 X92.521 Y84.12 E-.14932
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z2.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z2.9 F4000
            G39.3 S1
            G0 Z2.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.275 Y84.389 F42000
G1 Z2.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.418345
G1 F1172
M204 S6000
G1 X89.734 Y84.388 E.01588
M73 P56 R7
G1 X89.185 Y84.441 E.01618
G1 X88.644 Y84.547 E.01618
G1 X88.115 Y84.704 E.01619
G1 X87.606 Y84.917 E.01617
G1 X87.12 Y85.176 E.01618
G1 X86.658 Y85.48 E.0162
G1 X86.234 Y85.83 E.01616
G1 X85.843 Y86.22 E.01619
G1 X85.49 Y86.644 E.01618
G1 X85.186 Y87.103 E.01618
G1 X84.925 Y87.589 E.01618
G1 X84.71 Y88.098 E.01619
G1 X84.552 Y88.626 E.01618
G1 X84.443 Y89.167 E.01618
G1 X84.386 Y89.715 E.01619
G1 X84.388 Y90.267 E.01618
G1 X84.441 Y90.815 E.01618
G1 X84.545 Y91.357 E.01619
G1 X84.707 Y91.884 E.01618
G1 X84.917 Y92.394 E.01617
G1 X85.174 Y92.882 E.01619
G1 X85.482 Y93.34 E.01619
G1 X85.83 Y93.766 E.01617
G1 X86.218 Y94.159 E.0162
G1 X86.645 Y94.507 E.01617
G1 X87.104 Y94.815 E.01619
G1 X87.588 Y95.078 E.01619
G1 X88.098 Y95.287 E.01618
G1 X88.626 Y95.448 E.01618
G1 X89.166 Y95.559 E.01619
G1 X89.715 Y95.611 E.01618
G1 X90.266 Y95.612 E.01617
G1 X90.816 Y95.562 E.01619
G1 X91.356 Y95.453 E.01618
G1 X91.884 Y95.293 E.01618
G1 X92.395 Y95.086 E.01618
G1 X92.881 Y94.824 E.0162
G1 X93.34 Y94.518 E.01617
G1 X93.768 Y94.171 E.01618
G1 X94.157 Y93.78 E.01618
G1 X94.507 Y93.355 E.01618
G1 X94.817 Y92.898 E.01619
G1 X95.075 Y92.411 E.01618
G1 X95.287 Y91.902 E.01618
G1 X95.451 Y91.375 E.01618
G1 X95.557 Y90.833 E.01619
G1 X95.611 Y90.285 E.01618
G1 X95.615 Y89.733 E.01618
G1 X95.559 Y89.185 E.01618
G1 X95.453 Y88.644 E.01618
G1 X95.296 Y88.115 E.01618
G1 X95.083 Y87.606 E.01618
G1 X94.824 Y87.119 E.01619
G1 X94.521 Y86.659 E.01618
G1 X94.169 Y86.234 E.01618
G1 X93.78 Y85.843 E.01619
G1 X93.356 Y85.49 E.01618
G1 X92.897 Y85.186 E.01617
G1 X92.411 Y84.925 E.01619
G1 X91.902 Y84.71 E.01619
G1 X91.374 Y84.552 E.01618
G1 X90.834 Y84.444 E.01617
G1 X90.335 Y84.394 E.01473
M204 S10000
G1 X90.257 Y84.765 F42000
; LINE_WIDTH: 0.418096
G1 F1172
M204 S6000
G1 X90.761 Y84.814 E.01484
G1 X91.283 Y84.917 E.01562
G1 X91.769 Y85.057 E.01482
G1 X92.233 Y85.257 E.01483
G1 X92.703 Y85.509 E.01564
G1 X93.127 Y85.783 E.01482
G1 X93.513 Y86.11 E.01482
G1 X93.89 Y86.487 E.01564
G1 X94.217 Y86.873 E.01483
G1 X94.491 Y87.297 E.01482
G1 X94.743 Y87.767 E.01564
G1 X94.943 Y88.231 E.01482
G1 X95.083 Y88.717 E.01482
G1 X95.187 Y89.24 E.01563
G1 X95.244 Y89.742 E.01483
G1 X95.236 Y90.248 E.01482
G1 X95.184 Y90.778 E.01563
G1 X95.093 Y91.276 E.01483
G1 X94.939 Y91.757 E.01482
G1 X94.735 Y92.25 E.01563
G1 X94.503 Y92.699 E.01483
G1 X94.216 Y93.115 E.01483
G1 X93.878 Y93.527 E.01563
G1 X93.526 Y93.89 E.01483
G1 X93.13 Y94.205 E.01482
G1 X92.687 Y94.501 E.01562
G1 X92.244 Y94.746 E.01484
G1 X91.775 Y94.932 E.01482
G1 X91.265 Y95.087 E.01562
G1 X90.77 Y95.193 E.01483
G1 X90.266 Y95.235 E.01483
G1 X89.734 Y95.235 E.01562
G1 X89.23 Y95.193 E.01483
G1 X88.735 Y95.087 E.01483
G1 X88.225 Y94.932 E.01562
G1 X87.755 Y94.746 E.01483
G1 X87.313 Y94.501 E.01483
G1 X86.87 Y94.205 E.01563
G1 X86.474 Y93.89 E.01482
G1 X86.122 Y93.527 E.01484
G1 X85.784 Y93.115 E.01562
G1 X85.497 Y92.699 E.01483
G1 X85.265 Y92.249 E.01483
G1 X85.061 Y91.757 E.01562
G1 X84.907 Y91.276 E.01483
G1 X84.816 Y90.778 E.01483
G1 X84.764 Y90.248 E.01563
G1 X84.756 Y89.743 E.01482
G1 X84.813 Y89.24 E.01483
G1 X84.917 Y88.717 E.01563
G1 X85.057 Y88.231 E.01483
G1 X85.257 Y87.767 E.01483
G1 X85.509 Y87.297 E.01563
G1 X85.783 Y86.872 E.01483
G1 X86.11 Y86.487 E.01482
G1 X86.487 Y86.11 E.01564
G1 X86.872 Y85.783 E.01481
G1 X87.297 Y85.508 E.01484
G1 X87.97 Y85.155 E.02227
G1 X88.47 Y84.975 E.0156
G1 X89.24 Y84.813 E.02307
G1 X89.734 Y84.766 E.01455
G1 X90.197 Y84.765 E.01357
; WIPE_START
G1 F10395.132
G1 X89.734 Y84.766 E-.17584
G1 X89.24 Y84.813 E-.18854
G1 X89.2 Y84.822 E-.01562
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z2.9 I-1.206 J-.166 P1  F42000
G1 X87.882 Y94.376 Z2.9
G1 Z2.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1172
M204 S6000
G1 X87.706 Y94.292 E.00621
G1 X87.296 Y94.047 E.0152
G1 X86.912 Y93.762 E.0152
G1 X86.558 Y93.442 E.0152
G1 X93.442 Y86.558 E.30972
G1 X93.088 Y86.238 E.0152
G1 X92.704 Y85.953 E.0152
G1 X92.294 Y85.708 E.0152
G1 X91.863 Y85.503 E.01519
G1 X91.413 Y85.343 E.0152
G1 X90.95 Y85.226 E.0152
G1 X90.475 Y85.156 E.01525
G1 X90.067 Y85.135 E.01302
G1 X89.519 Y85.157 E.01744
G1 X89.05 Y85.226 E.01507
G1 X88.587 Y85.343 E.0152
G1 X88.137 Y85.503 E.01519
G1 X87.706 Y85.708 E.0152
G1 X87.296 Y85.953 E.0152
G1 X86.913 Y86.238 E.01519
G1 X86.558 Y86.558 E.0152
G1 X93.442 Y93.442 E.30972
G1 X93.762 Y93.088 E.01519
G1 X94.047 Y92.704 E.0152
G1 X94.292 Y92.294 E.01519
G1 X94.376 Y92.118 E.00622
; CHANGE_LAYER
; Z_HEIGHT: 2.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X94.292 Y92.294 E-.07427
G1 X94.047 Y92.704 E-.18145
G1 X93.852 Y92.967 E-.12428
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 14/43
; update layer progress
M73 L14
M991 S0 P13 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z2.9 I1.181 J-.295 P1  F42000
G1 X91.58 Y83.883 Z2.9
G1 Z2.7
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1086
M204 S6000
G1 X91.836 Y83.947 E.00839
G1 X92.421 Y84.156 E.01975
G1 X92.982 Y84.421 E.01976
G1 X93.514 Y84.74 E.01975
G1 X94.013 Y85.11 E.01976
G1 X94.473 Y85.527 E.01975
G1 X94.89 Y85.987 E.01975
G1 X95.259 Y86.486 E.01975
G1 X95.579 Y87.018 E.01975
G1 X95.844 Y87.579 E.01975
G1 X96.053 Y88.164 E.01975
G1 X96.204 Y88.766 E.01975
G1 X96.295 Y89.38 E.01975
G1 X96.326 Y90 E.01975
G1 X96.295 Y90.62 E.01976
G1 X96.204 Y91.234 E.01975
G1 X96.053 Y91.836 E.01975
G1 X95.844 Y92.421 E.01975
G1 X95.579 Y92.982 E.01975
G1 X95.26 Y93.514 E.01974
G1 X94.89 Y94.013 E.01976
G1 X94.473 Y94.473 E.01975
G1 X94.013 Y94.89 E.01974
G1 X93.514 Y95.26 E.01976
G1 X92.982 Y95.579 E.01975
G1 X92.421 Y95.844 E.01976
G1 X91.836 Y96.053 E.01975
G1 X91.234 Y96.204 E.01975
G1 X90.62 Y96.295 E.01975
G1 X90 Y96.326 E.01975
G1 X89.38 Y96.295 E.01975
G1 X88.766 Y96.204 E.01975
G1 X88.164 Y96.053 E.01975
G1 X87.579 Y95.844 E.01975
G1 X87.018 Y95.579 E.01976
G1 X86.486 Y95.26 E.01975
G1 X85.987 Y94.89 E.01976
G1 X85.527 Y94.473 E.01974
G1 X85.11 Y94.013 E.01976
G1 X84.74 Y93.514 E.01975
G1 X84.421 Y92.982 E.01975
G1 X84.156 Y92.421 E.01975
G1 X83.947 Y91.836 E.01975
G1 X83.796 Y91.234 E.01975
G1 X83.705 Y90.62 E.01975
G1 X83.674 Y90 E.01975
G1 X83.705 Y89.38 E.01975
G1 X83.796 Y88.766 E.01975
G1 X83.947 Y88.164 E.01975
G1 X84.156 Y87.579 E.01975
G1 X84.421 Y87.018 E.01975
G1 X84.74 Y86.486 E.01975
G1 X85.11 Y85.987 E.01975
G1 X85.527 Y85.527 E.01975
G1 X85.987 Y85.11 E.01975
G1 X86.486 Y84.74 E.01975
G1 X87.018 Y84.421 E.01975
G1 X87.579 Y84.156 E.01975
G1 X88.164 Y83.947 E.01976
G1 X88.766 Y83.796 E.01975
G1 X89.387 Y83.704 E.01998
G1 X90.11 Y83.677 E.02301
G1 X90.618 Y83.705 E.01621
G1 X91.234 Y83.796 E.0198
G1 X91.522 Y83.868 E.00945
; COOLING_NODE: 5
M204 S250
G1 X91.676 Y83.502 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F720
M204 S5000
G1 X91.95 Y83.571 E.00834
G1 X92.571 Y83.793 E.01943
G1 X93.167 Y84.075 E.01943
G1 X93.732 Y84.414 E.01943
G1 X94.262 Y84.807 E.01944
G1 X94.75 Y85.25 E.01943
G1 X95.193 Y85.738 E.01943
G1 X95.586 Y86.268 E.01943
G1 X95.925 Y86.833 E.01943
G1 X96.207 Y87.429 E.01943
G1 X96.429 Y88.05 E.01944
G1 X96.589 Y88.689 E.01943
M73 P57 R7
G1 X96.686 Y89.341 E.01943
G1 X96.718 Y90 E.01943
G1 X96.686 Y90.659 E.01943
G1 X96.589 Y91.311 E.01943
G1 X96.429 Y91.95 E.01943
G1 X96.207 Y92.571 E.01944
G1 X95.925 Y93.167 E.01943
G1 X95.586 Y93.732 E.01943
G1 X95.193 Y94.262 E.01944
G1 X94.75 Y94.75 E.01943
G1 X94.262 Y95.193 E.01943
G1 X93.732 Y95.586 E.01944
G1 X93.167 Y95.925 E.01943
G1 X92.571 Y96.207 E.01943
G1 X91.95 Y96.429 E.01943
G1 X91.311 Y96.589 E.01943
G1 X90.659 Y96.686 E.01943
G1 X90 Y96.718 E.01944
G1 X89.341 Y96.686 E.01943
G1 X88.689 Y96.589 E.01943
G1 X88.05 Y96.429 E.01943
G1 X87.429 Y96.207 E.01943
G1 X86.833 Y95.925 E.01943
G1 X86.268 Y95.586 E.01943
G1 X85.738 Y95.193 E.01943
G1 X85.25 Y94.751 E.01943
G1 X84.807 Y94.262 E.01943
G1 X84.414 Y93.732 E.01944
G1 X84.075 Y93.167 E.01943
G1 X83.793 Y92.571 E.01943
G1 X83.571 Y91.95 E.01943
G1 X83.411 Y91.311 E.01943
G1 X83.314 Y90.659 E.01943
G1 X83.282 Y90 E.01943
G1 X83.314 Y89.341 E.01943
G1 X83.411 Y88.689 E.01943
G1 X83.571 Y88.05 E.01943
G1 X83.793 Y87.429 E.01943
G1 X84.075 Y86.833 E.01943
G1 X84.414 Y86.268 E.01943
G1 X84.807 Y85.738 E.01943
G1 X85.249 Y85.25 E.01943
G1 X85.738 Y84.807 E.01943
G1 X86.268 Y84.414 E.01943
G1 X86.833 Y84.075 E.01943
G1 X87.429 Y83.793 E.01943
G1 X88.05 Y83.571 E.01944
G1 X88.689 Y83.411 E.01943
G1 X89.344 Y83.314 E.0195
M106 S124.95
M106 S127.5
G1 X90.118 Y83.285 E.02284
G1 X90.658 Y83.314 E.01593
M106 S124.95
M106 S127.5
G1 X91.311 Y83.411 E.01944
G1 X91.617 Y83.488 E.00932
M106 S124.95
; WIPE_START
G1 F840
M204 S6000
G1 X91.95 Y83.571 E-.13033
G1 X92.569 Y83.792 E-.24967
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z3.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z3.1 F4000
            G39.3 S1
            G0 Z3.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.432 Y84.081 F42000
G1 Z2.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.387772
G1 F1086
M204 S6000
G1 X88.848 Y84.164 E.01589
G1 X88.281 Y84.305 E.01574
G1 X87.731 Y84.501 E.01574
G1 X87.203 Y84.75 E.01574
G1 X86.702 Y85.049 E.01573
G1 X86.233 Y85.396 E.01574
G1 X85.799 Y85.788 E.01574
G1 X85.407 Y86.22 E.01573
G1 X85.058 Y86.688 E.01574
G1 X84.758 Y87.188 E.01573
G1 X84.507 Y87.716 E.01574
G1 X84.31 Y88.265 E.01574
G1 X84.167 Y88.831 E.01574
G1 X84.081 Y89.408 E.01573
G1 X84.051 Y89.992 E.01574
G1 X84.079 Y90.575 E.01574
G1 X84.164 Y91.152 E.01573
G1 X84.305 Y91.719 E.01575
G1 X84.501 Y92.269 E.01573
G1 X84.75 Y92.797 E.01573
G1 X85.049 Y93.298 E.01574
G1 X85.396 Y93.767 E.01574
G1 X85.788 Y94.201 E.01574
G1 X86.22 Y94.593 E.01573
G1 X86.688 Y94.942 E.01574
G1 X87.188 Y95.242 E.01573
G1 X87.716 Y95.493 E.01574
G1 X88.265 Y95.69 E.01573
G1 X88.831 Y95.833 E.01574
G1 X89.408 Y95.919 E.01573
G1 X89.992 Y95.949 E.01574
G1 X90.575 Y95.921 E.01574
G1 X91.152 Y95.836 E.01572
G1 X91.719 Y95.695 E.01575
G1 X92.269 Y95.499 E.01573
G1 X92.797 Y95.25 E.01574
G1 X93.298 Y94.951 E.01573
G1 X93.767 Y94.604 E.01574
G1 X94.2 Y94.212 E.01573
G1 X94.593 Y93.78 E.01573
G1 X94.942 Y93.312 E.01574
G1 X95.242 Y92.812 E.01573
G1 X95.493 Y92.284 E.01574
G1 X95.69 Y91.735 E.01574
G1 X95.833 Y91.169 E.01574
G1 X95.919 Y90.592 E.01572
G1 X95.949 Y90.008 E.01574
G1 X95.921 Y89.425 E.01574
G1 X95.836 Y88.848 E.01573
G1 X95.695 Y88.281 E.01574
G1 X95.499 Y87.731 E.01573
G1 X95.25 Y87.203 E.01574
G1 X94.951 Y86.702 E.01573
G1 X94.604 Y86.233 E.01573
G1 X94.212 Y85.8 E.01574
G1 X93.781 Y85.407 E.01573
G1 X93.312 Y85.058 E.01575
G1 X92.812 Y84.758 E.01573
G1 X92.284 Y84.507 E.01574
G1 X91.735 Y84.31 E.01573
G1 X91.169 Y84.167 E.01575
G1 X90.589 Y84.081 E.0158
G1 X90.099 Y84.053 E.01323
G1 X89.492 Y84.078 E.01638
M204 S10000
G1 X90.115 Y84.395 F42000
; LINE_WIDTH: 0.381254
G1 F1086
M204 S6000
G1 X90.813 Y84.452 E.01852
G1 X91.351 Y84.558 E.01448
G1 X91.877 Y84.716 E.01455
G1 X92.386 Y84.926 E.01455
G1 X92.872 Y85.184 E.01456
G1 X93.33 Y85.489 E.01454
G1 X93.757 Y85.837 E.01456
G1 X94.146 Y86.225 E.01455
G1 X94.497 Y86.65 E.01456
G1 X94.803 Y87.107 E.01455
G1 X95.064 Y87.591 E.01455
G1 X95.275 Y88.099 E.01455
G1 X95.436 Y88.626 E.01455
G1 X95.545 Y89.165 E.01456
G1 X95.6 Y89.713 E.01455
G1 X95.601 Y90.263 E.01455
G1 X95.548 Y90.811 E.01455
G1 X95.442 Y91.35 E.01455
G1 X95.284 Y91.877 E.01455
G1 X95.074 Y92.386 E.01456
G1 X94.816 Y92.872 E.01455
G1 X94.511 Y93.33 E.01455
G1 X94.163 Y93.757 E.01456
G1 X93.775 Y94.146 E.01455
G1 X93.35 Y94.497 E.01456
G1 X92.893 Y94.803 E.01455
G1 X92.409 Y95.064 E.01455
G1 X91.901 Y95.275 E.01455
G1 X91.374 Y95.436 E.01455
G1 X90.835 Y95.545 E.01455
G1 X90.287 Y95.6 E.01456
G1 X89.737 Y95.601 E.01455
G1 X89.189 Y95.548 E.01455
G1 X88.65 Y95.442 E.01455
G1 X88.123 Y95.284 E.01455
G1 X87.614 Y95.074 E.01455
G1 X87.128 Y94.816 E.01455
G1 X86.67 Y94.511 E.01454
G1 X86.243 Y94.163 E.01457
G1 X85.854 Y93.775 E.01453
G1 X85.503 Y93.35 E.01457
G1 X85.197 Y92.893 E.01455
G1 X84.936 Y92.409 E.01455
G1 X84.725 Y91.901 E.01456
G1 X84.564 Y91.375 E.01454
G1 X84.455 Y90.835 E.01456
G1 X84.4 Y90.287 E.01455
G1 X84.399 Y89.737 E.01455
G1 X84.452 Y89.189 E.01455
G1 X84.558 Y88.65 E.01455
G1 X84.716 Y88.122 E.01456
G1 X84.926 Y87.614 E.01455
G1 X85.184 Y87.128 E.01456
G1 X85.489 Y86.67 E.01454
G1 X85.837 Y86.244 E.01455
G1 X86.225 Y85.853 E.01456
G1 X86.65 Y85.503 E.01455
G1 X87.107 Y85.197 E.01454
G1 X87.592 Y84.936 E.01456
G1 X88.099 Y84.725 E.01455
G1 X88.625 Y84.564 E.01455
G1 X89.173 Y84.454 E.01477
G1 X90.055 Y84.399 E.02338
; WIPE_START
G1 F11527.081
G1 X89.173 Y84.454 E-.33591
G1 X89.059 Y84.477 E-.04409
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.1 I-.994 J.703 P1  F42000
G1 X94.652 Y92.388 Z3.1
G1 Z2.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1086
M204 S6000
G1 X94.49 Y92.691 E.01095
G1 X94.205 Y93.118 E.01634
G1 X93.879 Y93.516 E.01635
G1 X93.697 Y93.697 E.00817
G1 X86.303 Y86.303 E.33272
G1 X86.485 Y86.121 E.00818
G1 X86.882 Y85.795 E.01634
G1 X87.309 Y85.51 E.01633
G1 X87.762 Y85.268 E.01636
G1 X88.236 Y85.071 E.01633
G1 X88.728 Y84.922 E.01635
G1 X89.232 Y84.822 E.01635
G1 X89.746 Y84.771 E.01645
G1 X90.139 Y84.769 E.01249
G1 X90.774 Y84.823 E.02029
G1 X91.272 Y84.922 E.01614
G1 X91.764 Y85.071 E.01635
G1 X92.238 Y85.268 E.01634
G1 X92.691 Y85.51 E.01635
G1 X93.118 Y85.795 E.01633
G1 X93.515 Y86.121 E.01635
G1 X93.697 Y86.303 E.00817
G1 X86.303 Y93.697 E.33272
G1 X86.484 Y93.878 E.00816
G1 X86.882 Y94.205 E.01636
G1 X87.309 Y94.49 E.01634
G1 X87.612 Y94.652 E.01095
; CHANGE_LAYER
; Z_HEIGHT: 2.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X87.309 Y94.49 E-.13073
G1 X86.882 Y94.205 E-.19516
G1 X86.772 Y94.114 E-.05411
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 15/43
; update layer progress
M73 L15
M991 S0 P14 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z3.1 I1.103 J.514 P1  F42000
G1 X91.673 Y83.589 Z3.1
G1 Z2.9
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1106
M204 S6000
G1 X92.233 Y83.759 E.01862
G1 X92.834 Y84.008 E.0207
G1 X93.408 Y84.315 E.0207
G1 X93.948 Y84.676 E.02069
G1 X94.451 Y85.089 E.0207
G1 X94.911 Y85.549 E.02069
G1 X95.324 Y86.052 E.0207
G1 X95.685 Y86.593 E.0207
G1 X95.992 Y87.166 E.02069
G1 X96.241 Y87.767 E.0207
G1 X96.429 Y88.389 E.0207
G1 X96.556 Y89.028 E.0207
G1 X96.62 Y89.675 E.02069
G1 X96.62 Y90.325 E.0207
G1 X96.556 Y90.972 E.02069
M73 P58 R7
G1 X96.429 Y91.611 E.0207
G1 X96.241 Y92.233 E.02069
G1 X95.992 Y92.834 E.0207
G1 X95.685 Y93.408 E.0207
G1 X95.324 Y93.948 E.02069
G1 X94.911 Y94.451 E.0207
G1 X94.451 Y94.911 E.0207
G1 X93.948 Y95.324 E.02069
G1 X93.408 Y95.685 E.0207
G1 X92.834 Y95.992 E.02069
G1 X92.233 Y96.241 E.0207
G1 X91.611 Y96.429 E.0207
G1 X90.972 Y96.556 E.0207
G1 X90.325 Y96.62 E.02069
G1 X89.675 Y96.62 E.0207
G1 X89.028 Y96.556 E.02069
G1 X88.389 Y96.429 E.0207
G1 X87.767 Y96.241 E.02069
G1 X87.166 Y95.992 E.0207
G1 X86.592 Y95.685 E.0207
G1 X86.052 Y95.324 E.02069
G1 X85.549 Y94.911 E.02069
G1 X85.089 Y94.451 E.0207
G1 X84.676 Y93.948 E.0207
G1 X84.315 Y93.408 E.02069
G1 X84.008 Y92.834 E.0207
G1 X83.759 Y92.233 E.0207
G1 X83.571 Y91.611 E.02069
G1 X83.444 Y90.973 E.0207
G1 X83.38 Y90.325 E.02069
G1 X83.38 Y89.675 E.0207
G1 X83.444 Y89.028 E.02069
G1 X83.571 Y88.389 E.0207
G1 X83.759 Y87.767 E.02069
G1 X84.008 Y87.166 E.0207
G1 X84.315 Y86.592 E.0207
G1 X84.676 Y86.052 E.02068
G1 X85.089 Y85.549 E.0207
G1 X85.549 Y85.089 E.0207
G1 X86.052 Y84.676 E.0207
G1 X86.592 Y84.315 E.02068
G1 X87.166 Y84.008 E.0207
G1 X87.767 Y83.759 E.0207
G1 X88.389 Y83.571 E.02069
G1 X89.028 Y83.444 E.0207
G1 X89.676 Y83.38 E.02072
G1 X90.255 Y83.378 E.01843
G1 X90.975 Y83.444 E.023
G1 X91.611 Y83.571 E.02063
G1 X91.615 Y83.572 E.00016
; COOLING_NODE: 5
M204 S250
G1 X91.787 Y83.214 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F777
M204 S5000
G1 X92.365 Y83.39 E.01781
G1 X93.002 Y83.653 E.02031
G1 X93.609 Y83.978 E.02031
G1 X94.182 Y84.361 E.0203
G1 X94.715 Y84.798 E.02031
G1 X95.202 Y85.285 E.0203
G1 X95.639 Y85.818 E.02031
G1 X96.022 Y86.391 E.02031
G1 X96.347 Y86.998 E.0203
G1 X96.61 Y87.635 E.02031
G1 X96.81 Y88.294 E.0203
G1 X96.945 Y88.97 E.02031
G1 X97.012 Y89.656 E.0203
G1 X97.012 Y90.345 E.02031
G1 X96.945 Y91.03 E.0203
G1 X96.81 Y91.706 E.02031
G1 X96.61 Y92.365 E.0203
G1 X96.347 Y93.002 E.02031
G1 X96.022 Y93.609 E.02031
G1 X95.639 Y94.182 E.0203
G1 X95.202 Y94.715 E.02031
G1 X94.715 Y95.202 E.02031
G1 X94.182 Y95.639 E.02031
G1 X93.609 Y96.022 E.02031
G1 X93.002 Y96.347 E.02031
G1 X92.365 Y96.61 E.02031
G1 X91.706 Y96.81 E.0203
G1 X91.03 Y96.945 E.02031
G1 X90.344 Y97.012 E.0203
G1 X89.655 Y97.012 E.02031
G1 X88.97 Y96.945 E.0203
G1 X88.294 Y96.81 E.02031
G1 X87.635 Y96.61 E.0203
G1 X86.998 Y96.347 E.02031
G1 X86.391 Y96.022 E.02031
G1 X85.818 Y95.639 E.0203
G1 X85.285 Y95.202 E.0203
M73 P58 R6
G1 X84.798 Y94.715 E.02031
G1 X84.361 Y94.182 E.02031
G1 X83.978 Y93.609 E.0203
G1 X83.653 Y93.002 E.02031
G1 X83.39 Y92.365 E.02031
G1 X83.19 Y91.706 E.0203
G1 X83.055 Y91.03 E.02031
G1 X82.988 Y90.344 E.02031
G1 X82.988 Y89.655 E.02031
G1 X83.055 Y88.97 E.0203
G1 X83.19 Y88.294 E.02031
G1 X83.39 Y87.635 E.0203
G1 X83.653 Y86.998 E.02031
G1 X83.978 Y86.391 E.02031
G1 X84.361 Y85.818 E.0203
G1 X84.798 Y85.285 E.0203
G1 X85.285 Y84.798 E.02031
G1 X85.818 Y84.361 E.02031
G1 X86.391 Y83.978 E.0203
G1 X86.998 Y83.653 E.02031
G1 X87.635 Y83.39 E.02031
G1 X88.294 Y83.19 E.0203
G1 X88.97 Y83.055 E.02031
G1 X89.656 Y82.988 E.02031
G1 X90.272 Y82.986 E.01817
G1 X91.031 Y83.055 E.02245
G1 X91.706 Y83.19 E.02029
G1 X91.729 Y83.197 E.00072
M106 S124.95
; WIPE_START
G1 F960
M204 S6000
G1 X92.365 Y83.39 E-.25249
G1 X92.675 Y83.518 E-.12751
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z3.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z3.3 F4000
            G39.3 S1
            G0 Z3.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.315 Y83.747 F42000
G1 Z2.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.361135
G1 F1106
M204 S6000
G1 X89.712 Y83.742 E.015
G1 X89.1 Y83.801 E.01528
G1 X88.497 Y83.919 E.01529
G1 X87.908 Y84.096 E.01529
G1 X87.339 Y84.329 E.01529
G1 X86.796 Y84.617 E.01529
G1 X86.284 Y84.957 E.01529
G1 X85.808 Y85.346 E.01529
G1 X85.372 Y85.779 E.01529
G1 X84.98 Y86.253 E.0153
G1 X84.637 Y86.763 E.01529
G1 X84.346 Y87.304 E.01529
G1 X84.109 Y87.872 E.0153
G1 X83.928 Y88.459 E.01529
G1 X83.807 Y89.062 E.01529
G1 X83.744 Y89.674 E.01529
G1 X83.743 Y90.288 E.01528
G1 X83.801 Y90.9 E.01529
G1 X83.919 Y91.503 E.01529
G1 X84.096 Y92.092 E.01529
G1 X84.329 Y92.661 E.01529
G1 X84.617 Y93.204 E.01529
G1 X84.957 Y93.716 E.01529
G1 X85.346 Y94.192 E.01529
G1 X85.779 Y94.628 E.01529
G1 X86.253 Y95.02 E.01529
G1 X86.763 Y95.363 E.01529
G1 X87.304 Y95.654 E.01529
G1 X87.872 Y95.891 E.01529
G1 X88.459 Y96.072 E.01529
G1 X89.062 Y96.193 E.01529
G1 X89.674 Y96.256 E.01529
G1 X90.288 Y96.257 E.01528
G1 X90.9 Y96.199 E.01529
G1 X91.503 Y96.081 E.01529
G1 X92.092 Y95.904 E.01529
G1 X92.661 Y95.671 E.01528
G1 X93.204 Y95.383 E.01529
G1 X93.716 Y95.043 E.0153
G1 X94.192 Y94.654 E.01528
G1 X94.628 Y94.221 E.01529
G1 X95.02 Y93.747 E.01529
G1 X95.363 Y93.237 E.01529
G1 X95.654 Y92.696 E.01529
G1 X95.891 Y92.128 E.0153
G1 X96.072 Y91.541 E.01528
G1 X96.193 Y90.938 E.0153
G1 X96.256 Y90.327 E.01529
G1 X96.257 Y89.7 E.01557
G1 X96.199 Y89.1 E.01501
G1 X96.081 Y88.497 E.01529
G1 X95.904 Y87.908 E.01529
G1 X95.671 Y87.339 E.01528
G1 X95.383 Y86.796 E.01529
G1 X95.043 Y86.284 E.01529
G1 X94.654 Y85.807 E.01529
G1 X94.221 Y85.372 E.01528
G1 X93.747 Y84.98 E.01529
G1 X93.237 Y84.637 E.01529
G1 X92.696 Y84.346 E.01529
G1 X92.128 Y84.109 E.0153
G1 X91.541 Y83.928 E.01529
G1 X90.939 Y83.807 E.01528
G1 X90.374 Y83.753 E.0141
M204 S10000
G1 X90.232 Y84.06 F42000
; LINE_WIDTH: 0.360978
G1 F1106
M204 S6000
G1 X90.861 Y84.117 E.01571
G1 X91.433 Y84.229 E.0145
G1 X91.992 Y84.397 E.0145
G1 X92.532 Y84.62 E.01451
G1 X93.047 Y84.894 E.0145
G1 X93.533 Y85.217 E.0145
G1 X93.984 Y85.586 E.01451
G1 X94.398 Y85.998 E.0145
G1 X94.769 Y86.448 E.01451
G1 X95.094 Y86.933 E.01451
G1 X95.37 Y87.447 E.01451
G1 X95.595 Y87.986 E.0145
G1 X95.765 Y88.544 E.0145
G1 X95.88 Y89.116 E.0145
G1 X95.938 Y89.697 E.01451
G1 X95.94 Y90.28 E.0145
G1 X95.883 Y90.861 E.0145
G1 X95.771 Y91.433 E.01451
G1 X95.603 Y91.992 E.0145
M73 P59 R6
G1 X95.38 Y92.532 E.01452
G1 X95.106 Y93.047 E.0145
G1 X94.783 Y93.532 E.0145
G1 X94.414 Y93.984 E.01451
G1 X94.002 Y94.398 E.01451
G1 X93.552 Y94.769 E.0145
G1 X93.067 Y95.094 E.01451
G1 X92.553 Y95.37 E.01451
G1 X92.014 Y95.595 E.0145
G1 X91.456 Y95.765 E.0145
G1 X90.884 Y95.88 E.01451
G1 X90.303 Y95.938 E.01451
G1 X89.72 Y95.94 E.0145
G1 X89.139 Y95.883 E.01451
G1 X88.567 Y95.771 E.01451
G1 X88.008 Y95.603 E.0145
G1 X87.468 Y95.38 E.01451
G1 X86.953 Y95.106 E.01451
G1 X86.467 Y94.783 E.01451
G1 X86.016 Y94.414 E.01451
G1 X85.602 Y94.002 E.01451
G1 X85.231 Y93.552 E.01451
G1 X84.906 Y93.067 E.01451
G1 X84.63 Y92.553 E.01451
G1 X84.402 Y92.005 E.01475
G1 X84.235 Y91.456 E.01426
G1 X84.12 Y90.884 E.01451
G1 X84.062 Y90.304 E.0145
G1 X84.06 Y89.72 E.0145
G1 X84.117 Y89.139 E.01451
G1 X84.229 Y88.567 E.0145
G1 X84.397 Y88.008 E.0145
G1 X84.62 Y87.468 E.01451
G1 X84.894 Y86.953 E.01451
G1 X85.217 Y86.467 E.0145
G1 X85.586 Y86.015 E.01451
G1 X85.998 Y85.602 E.0145
G1 X86.448 Y85.231 E.01451
G1 X86.933 Y84.906 E.01451
G1 X87.447 Y84.63 E.01451
G1 X87.986 Y84.405 E.0145
G1 X88.544 Y84.235 E.01451
G1 X89.116 Y84.12 E.01451
G1 X89.697 Y84.062 E.01451
G1 X90.172 Y84.06 E.01181
; WIPE_START
G1 F12261.917
G1 X89.697 Y84.062 E-.18054
G1 X89.174 Y84.114 E-.19946
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.3 I-1.2 J-.202 P1  F42000
G1 X87.357 Y94.911 Z3.3
G1 Z2.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1106
M204 S6000
G1 X87.129 Y94.789 E.00822
G1 X86.674 Y94.485 E.01744
G1 X86.25 Y94.137 E.01743
G1 X86.057 Y93.943 E.00872
G1 X93.943 Y86.057 E.35489
G1 X93.75 Y85.863 E.00872
G1 X93.326 Y85.515 E.01744
G1 X92.871 Y85.211 E.01743
G1 X92.387 Y84.952 E.01743
G1 X91.881 Y84.743 E.01744
G1 X91.357 Y84.584 E.01744
G1 X90.82 Y84.477 E.01742
G1 X90.264 Y84.423 E.01777
G1 X89.726 Y84.423 E.01711
G1 X89.181 Y84.477 E.01744
G1 X88.643 Y84.584 E.01744
G1 X88.119 Y84.743 E.01744
G1 X87.613 Y84.952 E.01743
G1 X87.129 Y85.211 E.01744
G1 X86.674 Y85.515 E.01744
G1 X86.25 Y85.863 E.01742
G1 X86.057 Y86.057 E.00872
G1 X93.943 Y93.944 E.35489
G1 X94.137 Y93.75 E.00871
G1 X94.485 Y93.326 E.01744
G1 X94.789 Y92.87 E.01743
G1 X94.911 Y92.643 E.00822
; CHANGE_LAYER
; Z_HEIGHT: 3.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X94.789 Y92.87 E-.09816
G1 X94.485 Y93.326 E-.20822
G1 X94.362 Y93.476 E-.07362
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 16/43
; update layer progress
M73 L16
M991 S0 P15 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z3.3 I1.178 J-.307 P1  F42000
G1 X91.717 Y83.315 Z3.3
G1 Z3.1
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1063
M204 S6000
G1 X92.006 Y83.387 E.00947
G1 X92.645 Y83.616 E.02158
G1 X93.257 Y83.906 E.02157
G1 X93.839 Y84.254 E.02158
G1 X94.384 Y84.658 E.02157
G1 X94.886 Y85.114 E.02158
G1 X95.342 Y85.616 E.02158
G1 X95.746 Y86.161 E.02157
G1 X96.094 Y86.742 E.02157
G1 X96.384 Y87.356 E.02158
G1 X96.613 Y87.994 E.02158
G1 X96.777 Y88.652 E.02158
G1 X96.877 Y89.323 E.02157
G1 X96.91 Y90 E.02158
G1 X96.877 Y90.677 E.02158
G1 X96.777 Y91.348 E.02157
G1 X96.613 Y92.006 E.02158
G1 X96.384 Y92.645 E.02158
G1 X96.094 Y93.257 E.02157
G1 X95.746 Y93.839 E.02159
G1 X95.342 Y94.384 E.02157
G1 X94.886 Y94.886 E.02158
G1 X94.384 Y95.342 E.02158
G1 X93.839 Y95.746 E.02158
G1 X93.258 Y96.094 E.02157
G1 X92.644 Y96.384 E.02158
G1 X92.006 Y96.613 E.02158
G1 X91.348 Y96.777 E.02157
G1 X90.677 Y96.877 E.02157
G1 X90 Y96.91 E.02158
G1 X89.323 Y96.877 E.02158
G1 X88.652 Y96.777 E.02157
G1 X87.994 Y96.613 E.02158
G1 X87.355 Y96.384 E.02158
G1 X86.743 Y96.094 E.02157
G1 X86.161 Y95.746 E.02158
G1 X85.616 Y95.342 E.02157
G1 X85.114 Y94.886 E.02158
G1 X84.658 Y94.384 E.02158
G1 X84.254 Y93.839 E.02158
G1 X83.906 Y93.258 E.02157
G1 X83.616 Y92.644 E.02158
G1 X83.387 Y92.006 E.02158
G1 X83.223 Y91.348 E.02157
G1 X83.123 Y90.677 E.02158
G1 X83.09 Y90 E.02158
G1 X83.123 Y89.323 E.02158
G1 X83.223 Y88.652 E.02157
G1 X83.387 Y87.994 E.02158
G1 X83.616 Y87.355 E.02158
G1 X83.906 Y86.743 E.02157
G1 X84.254 Y86.161 E.02158
G1 X84.658 Y85.616 E.02158
G1 X85.114 Y85.114 E.02157
G1 X85.616 Y84.658 E.02158
G1 X86.161 Y84.254 E.02158
G1 X86.742 Y83.906 E.02157
G1 X87.356 Y83.616 E.02159
G1 X87.994 Y83.387 E.02157
G1 X88.652 Y83.223 E.02158
G1 X89.33 Y83.122 E.02181
G1 X90.078 Y83.091 E.02384
G1 X90.676 Y83.123 E.01906
G1 X91.348 Y83.223 E.0216
G1 X91.659 Y83.3 E.01021
; COOLING_NODE: 5
M204 S250
G1 X91.813 Y82.935 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F838
M204 S5000
G1 X92.12 Y83.012 E.00934
G1 X92.795 Y83.253 E.02112
G1 X93.442 Y83.56 E.02112
G1 X94.057 Y83.928 E.02113
G1 X94.633 Y84.355 E.02112
G1 X95.164 Y84.836 E.02112
G1 X95.645 Y85.367 E.02113
G1 X96.072 Y85.943 E.02112
G1 X96.44 Y86.557 E.02112
G1 X96.747 Y87.205 E.02113
G1 X96.988 Y87.88 E.02113
G1 X97.162 Y88.575 E.02112
G1 X97.268 Y89.284 E.02112
G1 X97.303 Y90 E.02112
G1 X97.268 Y90.716 E.02112
G1 X97.162 Y91.425 E.02112
G1 X96.988 Y92.12 E.02113
G1 X96.747 Y92.795 E.02112
G1 X96.44 Y93.442 E.02112
G1 X96.072 Y94.057 E.02113
G1 X95.645 Y94.633 E.02112
G1 X95.164 Y95.164 E.02113
G1 X94.633 Y95.645 E.02112
G1 X94.057 Y96.072 E.02113
G1 X93.443 Y96.44 E.02111
G1 X92.795 Y96.747 E.02112
G1 X92.12 Y96.988 E.02113
G1 X91.425 Y97.162 E.02112
G1 X90.716 Y97.268 E.02112
G1 X90 Y97.303 E.02112
G1 X89.284 Y97.268 E.02112
G1 X88.575 Y97.162 E.02112
G1 X87.88 Y96.988 E.02113
G1 X87.205 Y96.747 E.02112
G1 X86.558 Y96.44 E.02112
G1 X85.943 Y96.072 E.02113
G1 X85.367 Y95.645 E.02112
G1 X84.836 Y95.164 E.02112
G1 X84.355 Y94.633 E.02112
G1 X83.928 Y94.057 E.02113
G1 X83.56 Y93.443 E.02112
G1 X83.253 Y92.795 E.02113
G1 X83.012 Y92.12 E.02112
G1 X82.838 Y91.425 E.02112
G1 X82.732 Y90.716 E.02112
G1 X82.697 Y90 E.02113
G1 X82.732 Y89.284 E.02112
G1 X82.838 Y88.575 E.02112
G1 X83.012 Y87.88 E.02113
G1 X83.253 Y87.205 E.02112
G1 X83.56 Y86.558 E.02112
G1 X83.928 Y85.943 E.02112
G1 X84.355 Y85.367 E.02113
G1 X84.836 Y84.836 E.02111
G1 X85.367 Y84.355 E.02113
G1 X85.943 Y83.928 E.02112
G1 X86.557 Y83.56 E.02112
G1 X87.205 Y83.253 E.02113
G1 X87.88 Y83.012 E.02112
G1 X88.575 Y82.838 E.02112
G1 X89.287 Y82.732 E.02119
M106 S124.95
M106 S127.5
G1 X89.641 Y82.706 E.01047
M106 S124.95
M106 S127.5
G1 X90.086 Y82.699 E.01312
G1 X90.716 Y82.732 E.01859
M106 S124.95
M106 S127.5
G1 X91.425 Y82.838 E.02113
G1 X91.754 Y82.92 E.01002
M106 S124.95
; WIPE_START
G1 F1200
M204 S6000
G1 X92.12 Y83.012 E-.14318
G1 X92.707 Y83.222 E-.23682
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z3.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z3.5 F4000
            G39.3 S1
            G0 Z3.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.12 Y83.6 F42000
G1 Z3.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.647716
G1 F1063
M204 S6000
G1 X89.392 Y83.626 E.03444
G1 X88.761 Y83.718 E.03014
G1 X88.151 Y83.869 E.0297
G1 X87.56 Y84.08 E.0297
G1 X86.991 Y84.348 E.02971
G1 X86.452 Y84.67 E.0297
G1 X85.946 Y85.043 E.0297
G1 X85.48 Y85.465 E.02971
G1 X85.057 Y85.93 E.0297
G1 X84.682 Y86.434 E.02971
G1 X84.358 Y86.972 E.0297
G1 X84.088 Y87.54 E.0297
G1 X83.876 Y88.131 E.02971
G1 X83.722 Y88.74 E.0297
G1 X83.629 Y89.361 E.0297
M73 P60 R6
G1 X83.597 Y89.989 E.02971
G1 X83.627 Y90.617 E.02971
G1 X83.718 Y91.239 E.0297
G1 X83.87 Y91.849 E.02971
G1 X84.08 Y92.44 E.02969
G1 X84.348 Y93.009 E.0297
G1 X84.67 Y93.548 E.02971
G1 X85.043 Y94.054 E.0297
G1 X85.465 Y94.52 E.02972
G1 X85.93 Y94.943 E.0297
G1 X86.434 Y95.318 E.02971
G1 X86.972 Y95.642 E.0297
G1 X87.54 Y95.912 E.02971
G1 X88.131 Y96.124 E.0297
G1 X88.74 Y96.278 E.02971
G1 X89.362 Y96.371 E.0297
G1 X89.989 Y96.403 E.02971
G1 X90.617 Y96.373 E.02972
G1 X91.239 Y96.282 E.0297
G1 X91.848 Y96.131 E.0297
G1 X92.44 Y95.92 E.0297
G1 X93.009 Y95.652 E.02971
G1 X93.548 Y95.33 E.0297
G1 X94.054 Y94.956 E.02973
G1 X94.52 Y94.535 E.02967
G1 X94.943 Y94.07 E.02972
G1 X95.318 Y93.566 E.02972
G1 X95.642 Y93.028 E.02969
G1 X95.912 Y92.46 E.0297
G1 X96.124 Y91.869 E.02971
G1 X96.278 Y91.26 E.0297
G1 X96.371 Y90.639 E.02969
G1 X96.404 Y90 E.03022
G1 X96.373 Y89.383 E.0292
G1 X96.282 Y88.761 E.0297
G1 X96.13 Y88.151 E.02971
G1 X95.92 Y87.56 E.0297
G1 X95.652 Y86.991 E.0297
G1 X95.33 Y86.451 E.02972
G1 X94.957 Y85.946 E.02968
G1 X94.535 Y85.48 E.02971
G1 X94.07 Y85.057 E.02971
G1 X93.566 Y84.682 E.02972
G1 X93.028 Y84.358 E.0297
G1 X92.46 Y84.088 E.02971
G1 X91.869 Y83.876 E.02971
G1 X91.26 Y83.722 E.0297
G1 X90.637 Y83.628 E.02979
G1 X90.18 Y83.603 E.02161
; WIPE_START
G1 F6448.464
G1 X90.637 Y83.628 E-.17368
G1 X91.173 Y83.709 E-.20632
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.5 I-.762 J-.948 P1  F42000
G1 X84.14 Y89.363 Z3.5
G1 Z3.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1063
M204 S6000
G1 X84.216 Y88.849 E.01653
G1 X84.356 Y88.288 E.01842
G1 X84.535 Y87.789 E.01686
G1 X87.789 Y84.535 E.14643
G1 X88.288 Y84.356 E.01686
G1 X88.849 Y84.216 E.01841
G1 X89.427 Y84.13 E.01857
G1 X90.106 Y84.105 E.02163
G1 X90.575 Y84.131 E.01496
G1 X91.151 Y84.216 E.0185
G1 X91.712 Y84.356 E.01841
G1 X92.211 Y84.535 E.01686
G1 X95.465 Y87.789 E.14643
G1 X95.644 Y88.288 E.01686
G1 X95.784 Y88.849 E.01842
G1 X95.869 Y89.422 E.01841
G1 X95.898 Y90 E.01842
G1 X95.869 Y90.578 E.01842
G1 X95.784 Y91.151 E.01841
G1 X95.644 Y91.712 E.01841
G1 X95.465 Y92.211 E.01686
G1 X92.211 Y95.465 E.14643
G1 X91.712 Y95.644 E.01686
G1 X91.151 Y95.784 E.01841
G1 X90.578 Y95.869 E.01841
G1 X90 Y95.898 E.01842
G1 X89.422 Y95.869 E.01842
G1 X88.849 Y95.784 E.01841
G1 X88.288 Y95.644 E.01841
G1 X87.789 Y95.465 E.01686
G1 X84.535 Y92.211 E.14643
G1 X84.356 Y91.712 E.01686
G1 X84.216 Y91.151 E.01842
G1 X84.14 Y90.637 E.01654
; WIPE_START
G1 F9580.435
G1 X84.216 Y91.151 E-.19752
G1 X84.333 Y91.617 E-.18248
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.5 I-1.11 J.5 P1  F42000
G1 X85.031 Y93.168 Z3.5
G1 Z3.1
G1 E.4 F1800
G1 F1063
M204 S6000
G1 X85.441 Y93.741 E.02243
G1 X85.83 Y94.17 E.01842
G1 X94.17 Y85.83 E.37529
G1 X94.559 Y86.259 E.01842
G1 X94.969 Y86.832 E.02243
; WIPE_START
G1 F9580.435
G1 X94.559 Y86.259 E-.26785
G1 X94.361 Y86.04 E-.11215
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.5 I-1.213 J.103 P1  F42000
G1 X94.969 Y93.168 Z3.5
G1 Z3.1
G1 E.4 F1800
G1 F1063
M204 S6000
G1 X94.559 Y93.741 E.02244
G1 X94.17 Y94.17 E.01841
G1 X85.83 Y85.83 E.37529
G1 X85.441 Y86.259 E.01842
G1 X85.031 Y86.832 E.02243
; CHANGE_LAYER
; Z_HEIGHT: 3.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X85.441 Y86.259 E-.26793
G1 X85.639 Y86.04 E-.11207
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 17/43
; update layer progress
M73 L17
M991 S0 P16 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z3.5 I.53 J1.095 P1  F42000
G1 X91.805 Y83.056 Z3.5
G1 Z3.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1117
M204 S6000
G1 X92.418 Y83.242 E.02037
G1 X93.069 Y83.511 E.02241
G1 X93.69 Y83.843 E.02241
G1 X94.276 Y84.235 E.02242
G1 X94.82 Y84.682 E.02241
G1 X95.319 Y85.18 E.02243
G1 X95.765 Y85.724 E.02241
G1 X96.156 Y86.31 E.02241
G1 X96.489 Y86.931 E.02241
G1 X96.758 Y87.582 E.02241
G1 X96.963 Y88.256 E.02241
G1 X97.1 Y88.947 E.02242
G1 X97.169 Y89.648 E.02241
G1 X97.169 Y90.352 E.02241
G1 X97.1 Y91.053 E.02241
G1 X96.963 Y91.744 E.02241
G1 X96.758 Y92.418 E.02242
G1 X96.489 Y93.069 E.02241
G1 X96.156 Y93.69 E.02241
G1 X95.765 Y94.276 E.02241
G1 X95.318 Y94.82 E.02241
G1 X94.82 Y95.318 E.02242
G1 X94.276 Y95.765 E.02241
G1 X93.69 Y96.157 E.02241
G1 X93.069 Y96.489 E.02241
G1 X92.418 Y96.758 E.02241
G1 X91.744 Y96.963 E.02241
G1 X91.053 Y97.1 E.02242
G1 X90.352 Y97.169 E.02241
G1 X89.648 Y97.169 E.02242
G1 X88.947 Y97.1 E.02241
G1 X88.256 Y96.963 E.02242
G1 X87.582 Y96.758 E.02241
G1 X86.931 Y96.489 E.02241
G1 X86.31 Y96.157 E.02241
G1 X85.724 Y95.765 E.02242
G1 X85.18 Y95.319 E.0224
G1 X84.682 Y94.82 E.02242
G1 X84.235 Y94.276 E.02241
G1 X83.843 Y93.69 E.02241
G1 X83.511 Y93.069 E.02241
G1 X83.242 Y92.418 E.02241
G1 X83.037 Y91.744 E.02241
G1 X82.9 Y91.053 E.02242
G1 X82.831 Y90.352 E.02241
G1 X82.831 Y89.648 E.02241
G1 X82.9 Y88.947 E.02241
G1 X83.037 Y88.256 E.02242
G1 X83.242 Y87.582 E.02241
G1 X83.511 Y86.931 E.02241
G1 X83.843 Y86.31 E.02241
G1 X84.235 Y85.724 E.02242
G1 X84.681 Y85.18 E.0224
G1 X85.18 Y84.682 E.02242
G1 X85.724 Y84.235 E.02241
G1 X86.31 Y83.843 E.02242
G1 X86.931 Y83.511 E.02241
G1 X87.582 Y83.242 E.02241
G1 X88.256 Y83.037 E.02241
G1 X88.947 Y82.9 E.02242
G1 X89.649 Y82.831 E.02244
G1 X90.255 Y82.828 E.01929
G1 X90.707 Y82.857 E.0144
G1 X91.046 Y82.899 E.01087
G1 X91.744 Y83.037 E.02265
G1 X91.748 Y83.039 E.00013
; COOLING_NODE: 5
M204 S250
G1 X91.919 Y82.681 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F899
M204 S5000
G1 X92.55 Y82.872 E.01944
G1 X93.237 Y83.157 E.0219
G1 X93.892 Y83.507 E.02189
G1 X94.51 Y83.919 E.0219
G1 X95.084 Y84.391 E.02189
G1 X95.609 Y84.916 E.02191
G1 X96.08 Y85.49 E.02189
G1 X96.493 Y86.108 E.02189
G1 X96.843 Y86.763 E.0219
G1 X97.128 Y87.45 E.0219
G1 X97.343 Y88.161 E.02189
G1 X97.488 Y88.889 E.0219
G1 X97.561 Y89.629 E.02189
G1 X97.561 Y90.371 E.0219
G1 X97.488 Y91.111 E.02189
G1 X97.343 Y91.839 E.02189
G1 X97.128 Y92.55 E.0219
M73 P61 R6
G1 X96.843 Y93.237 E.0219
G1 X96.493 Y93.892 E.0219
G1 X96.081 Y94.51 E.02189
G1 X95.609 Y95.084 E.0219
G1 X95.084 Y95.609 E.0219
G1 X94.51 Y96.081 E.0219
G1 X93.892 Y96.493 E.0219
G1 X93.237 Y96.843 E.02189
G1 X92.55 Y97.128 E.0219
G1 X91.84 Y97.343 E.02189
G1 X91.111 Y97.488 E.0219
G1 X90.371 Y97.561 E.02189
G1 X89.629 Y97.561 E.0219
G1 X88.889 Y97.488 E.02189
G1 X88.16 Y97.343 E.0219
G1 X87.45 Y97.128 E.02189
G1 X86.763 Y96.843 E.0219
G1 X86.108 Y96.493 E.02189
G1 X85.49 Y96.08 E.0219
G1 X84.916 Y95.609 E.02189
G1 X84.391 Y95.084 E.02191
G1 X83.92 Y94.51 E.02189
G1 X83.507 Y93.892 E.02189
G1 X83.157 Y93.237 E.0219
G1 X82.872 Y92.55 E.0219
G1 X82.657 Y91.84 E.02189
G1 X82.512 Y91.111 E.0219
G1 X82.439 Y90.371 E.02189
G1 X82.439 Y89.629 E.0219
G1 X82.512 Y88.889 E.02189
G1 X82.657 Y88.16 E.0219
G1 X82.872 Y87.45 E.02189
G1 X83.157 Y86.763 E.0219
G1 X83.507 Y86.108 E.02189
G1 X83.92 Y85.49 E.0219
G1 X84.391 Y84.916 E.02189
G1 X84.916 Y84.391 E.02191
G1 X85.49 Y83.92 E.02189
G1 X86.108 Y83.507 E.0219
G1 X86.763 Y83.157 E.02189
G1 X87.45 Y82.872 E.0219
G1 X88.16 Y82.657 E.02189
G1 X88.889 Y82.512 E.0219
G1 X89.629 Y82.439 E.02191
G1 X90.267 Y82.436 E.0188
M106 S124.95
M106 S127.5
G1 X90.743 Y82.466 E.01407
M106 S124.95
M106 S127.5
G1 X91.108 Y82.511 E.01084
M106 S124.95
M106 S127.5
G1 X91.84 Y82.657 E.02198
G1 X91.862 Y82.663 E.00069
M106 S124.95
; WIPE_START
G1 F1440
M204 S6000
G1 X92.55 Y82.872 E-.27342
G1 X92.809 Y82.98 E-.10658
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z3.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z3.7 F4000
            G39.3 S1
            G0 Z3.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.267 Y83.319 F42000
G1 Z3.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.612745
G1 F1117
M204 S6000
G1 X89.682 Y83.319 E.02606
G1 X89.028 Y83.383 E.02925
G1 X88.384 Y83.51 E.02924
G1 X87.756 Y83.699 E.02924
G1 X87.149 Y83.95 E.02923
G1 X86.57 Y84.258 E.02924
G1 X86.024 Y84.622 E.02921
G1 X85.516 Y85.038 E.02924
G1 X85.051 Y85.501 E.02925
G1 X84.634 Y86.008 E.02923
G1 X84.269 Y86.553 E.02921
G1 X83.958 Y87.131 E.02924
G1 X83.706 Y87.738 E.02924
G1 X83.515 Y88.365 E.02922
G1 X83.386 Y89.009 E.02925
G1 X83.32 Y89.662 E.02923
G1 X83.319 Y90.318 E.02923
G1 X83.383 Y90.972 E.02924
G1 X83.51 Y91.616 E.02923
G1 X83.699 Y92.244 E.02923
G1 X83.95 Y92.851 E.02924
G1 X84.258 Y93.43 E.02923
G1 X84.622 Y93.976 E.02922
G1 X85.038 Y94.484 E.02924
G1 X85.501 Y94.949 E.02924
G1 X86.008 Y95.366 E.02923
G1 X86.553 Y95.731 E.02921
G1 X87.131 Y96.042 E.02925
G1 X87.738 Y96.294 E.02924
G1 X88.365 Y96.485 E.02923
G1 X89.009 Y96.614 E.02924
G1 X89.662 Y96.68 E.02923
G1 X90.318 Y96.681 E.02924
G1 X90.971 Y96.617 E.02923
G1 X91.616 Y96.49 E.02924
G1 X92.244 Y96.301 E.02923
G1 X92.851 Y96.05 E.02924
G1 X93.43 Y95.742 E.02922
G1 X93.976 Y95.378 E.02923
G1 X94.484 Y94.962 E.02924
G1 X94.949 Y94.499 E.02923
G1 X95.366 Y93.992 E.02924
G1 X95.732 Y93.447 E.02922
G1 X96.042 Y92.869 E.02923
G1 X96.294 Y92.262 E.02925
G1 X96.485 Y91.635 E.02922
G1 X96.614 Y90.991 E.02924
G1 X96.68 Y90.338 E.02923
G1 X96.681 Y89.672 E.02968
G1 X96.617 Y89.028 E.02879
G1 X96.49 Y88.384 E.02924
G1 X96.301 Y87.756 E.02923
G1 X96.05 Y87.149 E.02923
G1 X95.742 Y86.57 E.02922
G1 X95.378 Y86.024 E.02924
G1 X94.962 Y85.516 E.02924
G1 X94.499 Y85.051 E.02923
G1 X93.992 Y84.634 E.02923
G1 X93.447 Y84.269 E.02922
G1 X92.869 Y83.958 E.02924
G1 X92.262 Y83.706 E.02923
G1 X91.635 Y83.515 E.02923
G1 X90.994 Y83.386 E.02912
G1 X90.327 Y83.325 E.02983
; WIPE_START
G1 F6844.205
G1 X90.994 Y83.386 E-.25447
G1 X91.318 Y83.451 E-.12553
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.7 I-.315 J-1.176 P1  F42000
G1 X86.907 Y84.632 Z3.7
G1 Z3.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1117
M204 S6000
G1 X86.307 Y85.02 E.02275
G1 X85.836 Y85.406 E.01936
G1 X85.621 Y85.621 E.00968
G1 X94.379 Y94.379 E.39407
G1 X94.164 Y94.594 E.00968
G1 X93.693 Y94.98 E.01936
G1 X93.093 Y95.368 E.02276
; WIPE_START
G1 F9580.435
G1 X93.693 Y94.98 E-.27178
G1 X93.913 Y94.799 E-.10822
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.7 I-.42 J-1.142 P1  F42000
G1 X90.126 Y96.193 Z3.7
G1 Z3.3
G1 E.4 F1800
G1 F1117
M204 S6000
G1 X90.304 Y96.193 E.00568
G1 X90.91 Y96.133 E.01935
G1 X91.506 Y96.014 E.01936
G1 X91.729 Y95.947 E.00741
G1 X95.947 Y91.729 E.18977
G1 X96.014 Y91.506 E.00741
G1 X96.133 Y90.91 E.01936
G1 X96.193 Y90.304 E.01936
G1 X96.193 Y89.696 E.01936
G1 X96.133 Y89.09 E.01936
G1 X96.014 Y88.494 E.01936
G1 X95.947 Y88.271 E.00741
G1 X91.729 Y84.053 E.18977
G1 X91.507 Y83.986 E.00741
G1 X90.913 Y83.868 E.01925
G1 X90.236 Y83.806 E.02164
G1 X89.697 Y83.807 E.01714
G1 X89.09 Y83.867 E.0194
G1 X88.494 Y83.986 E.01936
G1 X88.271 Y84.053 E.00741
G1 X84.053 Y88.271 E.18977
G1 X83.986 Y88.493 E.0074
G1 X83.867 Y89.09 E.01937
G1 X83.807 Y89.696 E.01936
G1 X83.807 Y90.304 E.01936
G1 X83.867 Y90.91 E.01936
G1 X83.986 Y91.506 E.01935
G1 X84.053 Y91.729 E.00741
G1 X88.271 Y95.947 E.18977
G1 X88.493 Y96.014 E.00741
G1 X89.09 Y96.133 E.01937
G1 X89.696 Y96.193 E.01936
G1 X89.874 Y96.193 E.00568
; WIPE_START
G1 F9580.435
G1 X89.696 Y96.193 E-.06788
G1 X89.09 Y96.133 E-.23116
G1 X88.881 Y96.091 E-.08096
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.7 I.994 J.702 P1  F42000
G1 X95.368 Y86.907 Z3.7
G1 Z3.3
G1 E.4 F1800
G1 F1117
M204 S6000
G1 X94.98 Y86.307 E.02276
G1 X94.594 Y85.836 E.01936
G1 X94.379 Y85.621 E.00968
G1 X85.621 Y94.379 E.39407
G1 X85.406 Y94.164 E.00968
G1 X85.02 Y93.693 E.01937
G1 X84.632 Y93.093 E.02275
; CHANGE_LAYER
; Z_HEIGHT: 3.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X85.02 Y93.693 E-.27176
G1 X85.201 Y93.913 E-.10825
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 18/43
; update layer progress
M73 L18
M991 S0 P17 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z3.7 I1.045 J.624 P1  F42000
G1 X91.839 Y82.808 Z3.7
G1 Z3.5
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1154
M204 S6000
G1 X92.157 Y82.888 E.01043
G1 X92.844 Y83.133 E.02321
G1 X93.504 Y83.445 E.02321
G1 X94.129 Y83.82 E.02321
G1 X94.715 Y84.255 E.0232
G1 X95.256 Y84.745 E.02321
G1 X95.745 Y85.285 E.0232
G1 X96.18 Y85.871 E.02321
G1 X96.555 Y86.496 E.02321
G1 X96.867 Y87.156 E.02321
G1 X97.112 Y87.843 E.02321
G1 X97.29 Y88.55 E.0232
G1 X97.397 Y89.271 E.02321
G1 X97.432 Y90 E.02321
M73 P62 R6
G1 X97.397 Y90.729 E.02321
G1 X97.29 Y91.45 E.02321
G1 X97.112 Y92.157 E.0232
G1 X96.867 Y92.844 E.02321
G1 X96.555 Y93.504 E.02321
G1 X96.18 Y94.129 E.02321
G1 X95.745 Y94.715 E.0232
G1 X95.256 Y95.255 E.02321
G1 X94.715 Y95.745 E.02321
G1 X94.129 Y96.18 E.02321
G1 X93.504 Y96.555 E.02321
G1 X92.844 Y96.867 E.02321
G1 X92.157 Y97.112 E.02321
G1 X91.45 Y97.29 E.02321
G1 X90.729 Y97.397 E.0232
G1 X90 Y97.432 E.02321
G1 X89.271 Y97.397 E.02321
G1 X88.55 Y97.29 E.02321
G1 X87.843 Y97.112 E.02321
G1 X87.156 Y96.867 E.02321
G1 X86.496 Y96.555 E.02321
G1 X85.871 Y96.18 E.0232
G1 X85.285 Y95.745 E.02321
G1 X84.745 Y95.256 E.0232
G1 X84.255 Y94.715 E.02321
G1 X83.82 Y94.129 E.0232
G1 X83.445 Y93.503 E.02322
G1 X83.133 Y92.844 E.0232
G1 X82.888 Y92.157 E.02321
G1 X82.71 Y91.45 E.0232
G1 X82.603 Y90.729 E.02321
G1 X82.568 Y90 E.02321
G1 X82.603 Y89.271 E.02321
G1 X82.71 Y88.55 E.02321
G1 X82.888 Y87.843 E.0232
G1 X83.133 Y87.156 E.02321
G1 X83.445 Y86.496 E.02321
G1 X83.82 Y85.871 E.0232
G1 X84.255 Y85.285 E.02321
G1 X84.745 Y84.744 E.02321
G1 X85.285 Y84.255 E.0232
G1 X85.871 Y83.82 E.02321
G1 X86.496 Y83.445 E.02321
G1 X87.156 Y83.133 E.02321
G1 X87.843 Y82.888 E.0232
G1 X88.55 Y82.71 E.0232
G1 X89.279 Y82.602 E.02344
G1 X90.07 Y82.569 E.02519
G1 X90.728 Y82.603 E.02096
G1 X91.45 Y82.71 E.02323
G1 X91.781 Y82.793 E.01086
; COOLING_NODE: 5
M204 S250
G1 X91.935 Y82.428 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F958
M204 S5000
G1 X92.271 Y82.512 E.01023
G1 X92.994 Y82.771 E.02263
G1 X93.689 Y83.099 E.02263
G1 X94.347 Y83.494 E.02264
G1 X94.964 Y83.951 E.02263
G1 X95.533 Y84.467 E.02264
G1 X96.049 Y85.036 E.02262
G1 X96.506 Y85.653 E.02263
G1 X96.901 Y86.311 E.02263
G1 X97.229 Y87.006 E.02263
G1 X97.488 Y87.729 E.02263
G1 X97.675 Y88.473 E.02263
G1 X97.787 Y89.233 E.02263
G1 X97.825 Y90 E.02264
G1 X97.787 Y90.767 E.02263
G1 X97.675 Y91.527 E.02263
G1 X97.488 Y92.271 E.02263
G1 X97.229 Y92.994 E.02264
G1 X96.901 Y93.689 E.02263
G1 X96.506 Y94.347 E.02263
G1 X96.049 Y94.964 E.02263
G1 X95.533 Y95.533 E.02263
G1 X94.964 Y96.049 E.02263
G1 X94.347 Y96.506 E.02264
G1 X93.689 Y96.901 E.02263
G1 X92.994 Y97.229 E.02264
G1 X92.271 Y97.488 E.02263
G1 X91.527 Y97.675 E.02263
G1 X90.767 Y97.787 E.02263
G1 X90 Y97.825 E.02263
G1 X89.233 Y97.787 E.02263
G1 X88.473 Y97.675 E.02263
G1 X87.729 Y97.488 E.02263
G1 X87.006 Y97.229 E.02263
G1 X86.311 Y96.901 E.02264
G1 X85.653 Y96.506 E.02263
G1 X85.036 Y96.049 E.02263
G1 X84.467 Y95.533 E.02263
G1 X83.951 Y94.964 E.02263
G1 X83.494 Y94.347 E.02263
G1 X83.099 Y93.688 E.02264
G1 X82.771 Y92.994 E.02263
G1 X82.512 Y92.271 E.02263
G1 X82.325 Y91.527 E.02263
G1 X82.213 Y90.767 E.02263
G1 X82.175 Y90 E.02264
G1 X82.213 Y89.233 E.02263
G1 X82.325 Y88.473 E.02263
G1 X82.512 Y87.729 E.02263
G1 X82.771 Y87.006 E.02264
G1 X83.099 Y86.311 E.02263
G1 X83.494 Y85.653 E.02263
G1 X83.951 Y85.036 E.02264
G1 X84.467 Y84.467 E.02263
G1 X85.036 Y83.951 E.02262
G1 X85.653 Y83.494 E.02264
G1 X86.311 Y83.099 E.02263
G1 X87.006 Y82.771 E.02263
G1 X87.729 Y82.512 E.02263
G1 X88.473 Y82.325 E.02263
G1 X89.235 Y82.212 E.0227
M106 S124.95
M106 S127.5
G1 X89.615 Y82.184 E.01122
M106 S124.95
M106 S127.5
G1 X90.077 Y82.177 E.01361
G1 X90.767 Y82.213 E.02036
M106 S124.95
M106 S127.5
G1 X91.527 Y82.325 E.02264
G1 X91.876 Y82.413 E.01063
M106 S124.95
; WIPE_START
G1 F1560
M204 S6000
G1 X92.271 Y82.512 E-.1547
G1 X92.83 Y82.712 E-.2253
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z3.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z3.9 F4000
            G39.3 S1
            G0 Z3.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.088 Y83.045 F42000
G1 Z3.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.585265
G1 F1154
M204 S6000
G1 X89.335 Y83.075 E.03192
G1 X88.652 Y83.175 E.02928
G1 X87.989 Y83.34 E.02894
G1 X87.347 Y83.569 E.02893
G1 X86.729 Y83.86 E.02895
G1 X86.143 Y84.211 E.02894
G1 X85.594 Y84.616 E.02893
G1 X85.088 Y85.074 E.02895
G1 X84.628 Y85.58 E.02894
G1 X84.221 Y86.128 E.02894
G1 X83.869 Y86.713 E.02893
G1 X83.576 Y87.329 E.02894
G1 X83.346 Y87.972 E.02894
G1 X83.179 Y88.634 E.02895
G1 X83.078 Y89.309 E.02893
G1 X83.043 Y89.991 E.02895
G1 X83.076 Y90.673 E.02895
G1 X83.175 Y91.348 E.02893
G1 X83.34 Y92.011 E.02894
G1 X83.569 Y92.654 E.02893
G1 X83.86 Y93.271 E.02894
G1 X84.211 Y93.857 E.02894
G1 X84.617 Y94.406 E.02893
G1 X85.074 Y94.912 E.02894
G1 X85.58 Y95.372 E.02895
G1 X86.127 Y95.779 E.02893
G1 X86.713 Y96.131 E.02895
G1 X87.329 Y96.423 E.02892
G1 X87.972 Y96.654 E.02895
G1 X88.634 Y96.821 E.02894
G1 X89.309 Y96.922 E.02893
G1 X89.991 Y96.957 E.02894
G1 X90.673 Y96.924 E.02895
G1 X91.348 Y96.825 E.02894
G1 X92.011 Y96.66 E.02894
G1 X92.654 Y96.431 E.02894
G1 X93.271 Y96.14 E.02894
G1 X93.857 Y95.789 E.02893
G1 X94.406 Y95.383 E.02894
G1 X94.912 Y94.926 E.02893
G1 X95.372 Y94.42 E.02896
G1 X95.779 Y93.873 E.02893
G1 X96.131 Y93.287 E.02895
G1 X96.424 Y92.671 E.02893
G1 X96.654 Y92.028 E.02895
G1 X96.821 Y91.366 E.02894
G1 X96.922 Y90.691 E.02893
G1 X96.957 Y90 E.02934
G1 X96.924 Y89.327 E.02856
G1 X96.825 Y88.652 E.02893
G1 X96.66 Y87.989 E.02894
G1 X96.431 Y87.346 E.02894
G1 X96.139 Y86.729 E.02895
G1 X95.789 Y86.143 E.02893
G1 X95.383 Y85.594 E.02894
G1 X94.926 Y85.087 E.02894
G1 X94.42 Y84.628 E.02895
G1 X93.873 Y84.221 E.02893
G1 X93.287 Y83.869 E.02894
G1 X92.671 Y83.577 E.02893
G1 X92.028 Y83.346 E.02894
G1 X91.366 Y83.179 E.02894
G1 X90.69 Y83.078 E.02899
G1 X90.148 Y83.048 E.02302
; WIPE_START
G1 F7191.006
G1 X90.69 Y83.078 E-.20635
G1 X91.142 Y83.145 E-.17365
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z3.9 I-.762 J.949 P1  F42000
G1 X95.573 Y86.704 Z3.9
G1 Z3.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1154
M204 S6000
G1 X95.39 Y86.399 E.01133
G1 X95.011 Y85.888 E.02024
G1 X94.584 Y85.416 E.02024
G1 X85.416 Y94.584 E.4125
G1 X84.989 Y94.112 E.02024
M73 P63 R6
G1 X84.61 Y93.601 E.02024
G1 X84.283 Y93.056 E.02024
G1 X84.011 Y92.481 E.02024
G1 X83.797 Y91.882 E.02024
G1 X83.66 Y91.336 E.01789
G1 X88.664 Y96.34 E.22514
G1 X89.365 Y96.451 E.02258
G1 X90 Y96.482 E.02024
G1 X90.635 Y96.451 E.02024
G1 X91.336 Y96.34 E.02258
G1 X96.34 Y91.336 E.22514
G1 X96.451 Y90.635 E.02258
G1 X96.482 Y90 E.02024
G1 X96.451 Y89.365 E.02024
G1 X96.34 Y88.664 E.02258
G1 X91.336 Y83.66 E.22514
G1 X91.265 Y83.642 E.00235
G1 X90.634 Y83.549 E.02029
G1 X90.076 Y83.519 E.01777
G1 X89.37 Y83.548 E.02248
G1 X88.664 Y83.66 E.02276
G1 X83.66 Y88.664 E.22514
G1 X83.797 Y88.118 E.01789
G1 X84.011 Y87.519 E.02024
G1 X84.283 Y86.944 E.02024
G1 X84.61 Y86.399 E.02024
G1 X84.989 Y85.888 E.02024
G1 X85.416 Y85.416 E.02024
G1 X94.584 Y94.584 E.4125
G1 X95.011 Y94.112 E.02025
G1 X95.39 Y93.601 E.02023
G1 X95.573 Y93.296 E.01133
; CHANGE_LAYER
; Z_HEIGHT: 3.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X95.39 Y93.601 E-.13535
G1 X95.011 Y94.112 E-.24159
G1 X95.006 Y94.118 E-.00306
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 19/43
; update layer progress
M73 L19
M991 S0 P18 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z3.9 I1.176 J-.314 P1  F42000
G1 X91.922 Y82.574 Z3.9
G1 Z3.7
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1200
M204 S6000
G1 X92.585 Y82.776 E.02206
G1 X93.281 Y83.064 E.02396
G1 X93.945 Y83.419 E.02395
G1 X94.571 Y83.837 E.02396
G1 X95.153 Y84.315 E.02396
G1 X95.685 Y84.847 E.02396
G1 X96.163 Y85.429 E.02396
G1 X96.581 Y86.055 E.02395
G1 X96.936 Y86.719 E.02396
G1 X97.224 Y87.415 E.02396
G1 X97.443 Y88.136 E.02396
G1 X97.59 Y88.874 E.02396
G1 X97.664 Y89.624 E.02396
G1 X97.664 Y90.377 E.02396
G1 X97.59 Y91.126 E.02396
G1 X97.443 Y91.864 E.02396
G1 X97.224 Y92.585 E.02396
G1 X96.936 Y93.281 E.02396
G1 X96.581 Y93.945 E.02395
G1 X96.163 Y94.571 E.02396
G1 X95.685 Y95.153 E.02395
G1 X95.153 Y95.685 E.02396
G1 X94.571 Y96.163 E.02396
G1 X93.945 Y96.581 E.02395
G1 X93.281 Y96.936 E.02396
G1 X92.585 Y97.224 E.02396
G1 X91.864 Y97.443 E.02396
G1 X91.126 Y97.59 E.02396
G1 X90.377 Y97.664 E.02396
G1 X89.623 Y97.664 E.02396
G1 X88.874 Y97.59 E.02395
G1 X88.136 Y97.443 E.02396
G1 X87.415 Y97.224 E.02396
G1 X86.719 Y96.936 E.02396
G1 X86.055 Y96.581 E.02396
G1 X85.429 Y96.163 E.02395
G1 X84.847 Y95.685 E.02396
G1 X84.315 Y95.153 E.02396
G1 X83.837 Y94.571 E.02396
G1 X83.419 Y93.945 E.02395
G1 X83.064 Y93.281 E.02396
G1 X82.776 Y92.585 E.02396
G1 X82.557 Y91.864 E.02396
G1 X82.41 Y91.126 E.02396
G1 X82.336 Y90.377 E.02396
G1 X82.336 Y89.623 E.02396
G1 X82.41 Y88.874 E.02395
G1 X82.557 Y88.136 E.02396
G1 X82.776 Y87.415 E.02396
G1 X83.064 Y86.719 E.02396
G1 X83.419 Y86.055 E.02396
G1 X83.837 Y85.429 E.02395
G1 X84.315 Y84.847 E.02396
G1 X84.847 Y84.315 E.02396
G1 X85.429 Y83.837 E.02396
G1 X86.055 Y83.419 E.02396
G1 X86.719 Y83.064 E.02395
G1 X87.415 Y82.776 E.02396
G1 X88.136 Y82.557 E.02396
G1 X88.874 Y82.41 E.02396
G1 X89.625 Y82.336 E.02399
G1 X90.27 Y82.334 E.02053
G1 X90.755 Y82.364 E.01548
G1 X91.119 Y82.409 E.01164
G1 X91.864 Y82.557 E.02418
; COOLING_NODE: 5
M204 S250
G1 X92.035 Y82.199 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1014
M204 S5000
G1 X92.717 Y82.406 E.021
G1 X93.448 Y82.709 E.02333
G1 X94.146 Y83.082 E.02332
G1 X94.805 Y83.522 E.02333
G1 X95.416 Y84.024 E.02333
G1 X95.976 Y84.584 E.02333
G1 X96.478 Y85.195 E.02333
G1 X96.918 Y85.853 E.02332
G1 X97.291 Y86.552 E.02333
G1 X97.594 Y87.283 E.02333
G1 X97.824 Y88.04 E.02333
G1 X97.978 Y88.817 E.02333
G1 X98.056 Y89.604 E.02333
G1 X98.056 Y90.396 E.02333
G1 X97.978 Y91.183 E.02333
G1 X97.824 Y91.96 E.02333
G1 X97.594 Y92.717 E.02332
G1 X97.291 Y93.448 E.02333
G1 X96.918 Y94.146 E.02332
G1 X96.478 Y94.805 E.02333
G1 X95.976 Y95.416 E.02332
G1 X95.416 Y95.976 E.02333
G1 X94.805 Y96.478 E.02333
G1 X94.147 Y96.918 E.02332
G1 X93.448 Y97.291 E.02333
G1 X92.717 Y97.594 E.02333
G1 X91.96 Y97.824 E.02333
G1 X91.183 Y97.978 E.02333
G1 X90.396 Y98.056 E.02333
G1 X89.604 Y98.056 E.02333
G1 X88.817 Y97.978 E.02333
G1 X88.04 Y97.824 E.02333
G1 X87.283 Y97.594 E.02332
G1 X86.552 Y97.291 E.02333
G1 X85.853 Y96.918 E.02333
G1 X85.195 Y96.478 E.02332
G1 X84.584 Y95.976 E.02333
G1 X84.024 Y95.416 E.02333
G1 X83.522 Y94.804 E.02333
G1 X83.082 Y94.146 E.02333
G1 X82.709 Y93.448 E.02332
G1 X82.406 Y92.717 E.02333
G1 X82.176 Y91.96 E.02333
G1 X82.022 Y91.183 E.02333
G1 X81.944 Y90.396 E.02333
G1 X81.944 Y89.604 E.02333
G1 X82.022 Y88.817 E.02332
G1 X82.176 Y88.04 E.02333
G1 X82.406 Y87.283 E.02332
G1 X82.709 Y86.552 E.02333
G1 X83.082 Y85.853 E.02333
G1 X83.522 Y85.195 E.02332
G1 X84.024 Y84.584 E.02333
G1 X84.584 Y84.024 E.02333
G1 X85.195 Y83.522 E.02333
G1 X85.854 Y83.082 E.02333
G1 X86.552 Y82.709 E.02332
G1 X87.283 Y82.406 E.02333
G1 X88.04 Y82.176 E.02333
G1 X88.817 Y82.022 E.02333
G1 X89.605 Y81.944 E.02334
G1 X90.281 Y81.941 E.01994
M106 S124.95
M106 S127.5
G1 X90.792 Y81.973 E.01507
M106 S124.95
M106 S127.5
G1 X91.181 Y82.021 E.01156
M106 S124.95
M106 S127.5
G1 X91.96 Y82.176 E.02341
G1 X91.978 Y82.182 E.00056
M106 S124.95
; WIPE_START
G1 F1800
M204 S6000
G1 X92.717 Y82.406 E-.29356
G1 X92.927 Y82.493 E-.08644
; WIPE_END
G1 E-.02
M204 S10000
G17
G3 Z4.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z4.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z4.1 F4000
            G39.3 S1
            G0 Z4.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.274 Y82.796 F42000
G1 Z3.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.558316
G1 F1200
M204 S6000
G1 X89.656 Y82.797 E.02492
G1 X88.951 Y82.866 E.02853
G1 X88.256 Y83.003 E.02852
G1 X87.579 Y83.208 E.0285
G1 X86.925 Y83.478 E.02851
G1 X86.301 Y83.811 E.02849
G1 X85.711 Y84.203 E.02852
G1 X85.164 Y84.651 E.0285
G1 X84.663 Y85.151 E.02852
G1 X84.214 Y85.698 E.02849
G1 X83.82 Y86.286 E.02851
G1 X83.485 Y86.909 E.02851
G1 X83.214 Y87.563 E.02851
G1 X83.007 Y88.24 E.0285
G1 X82.869 Y88.934 E.02851
G1 X82.798 Y89.638 E.02851
M73 P64 R6
G1 X82.798 Y90.345 E.0285
G1 X82.866 Y91.049 E.0285
G1 X83.003 Y91.744 E.02851
G1 X83.208 Y92.421 E.0285
G1 X83.478 Y93.075 E.02851
G1 X83.811 Y93.7 E.0285
G1 X84.203 Y94.288 E.0285
G1 X84.651 Y94.836 E.0285
G1 X85.151 Y95.337 E.02852
G1 X85.698 Y95.786 E.02849
G1 X86.286 Y96.18 E.02851
G1 X86.909 Y96.515 E.02851
G1 X87.563 Y96.786 E.0285
G1 X88.24 Y96.993 E.02851
G1 X88.934 Y97.131 E.02851
G1 X89.638 Y97.202 E.0285
G1 X90.345 Y97.202 E.0285
G1 X91.05 Y97.134 E.02851
G1 X91.744 Y96.997 E.02851
G1 X92.421 Y96.792 E.0285
G1 X93.075 Y96.522 E.02851
G1 X93.7 Y96.189 E.0285
G1 X94.288 Y95.797 E.02851
G1 X94.836 Y95.348 E.02851
G1 X95.337 Y94.849 E.0285
G1 X95.787 Y94.302 E.02851
G1 X96.18 Y93.715 E.0285
G1 X96.515 Y93.091 E.02851
G1 X96.786 Y92.437 E.02852
G1 X96.993 Y91.76 E.02851
G1 X97.131 Y91.066 E.0285
G1 X97.202 Y90.362 E.02851
G1 X97.202 Y89.646 E.02885
G1 X97.134 Y88.951 E.02816
G1 X96.997 Y88.256 E.02851
G1 X96.792 Y87.579 E.02851
G1 X96.522 Y86.925 E.02851
G1 X96.189 Y86.3 E.0285
G1 X95.797 Y85.712 E.0285
G1 X95.349 Y85.164 E.0285
G1 X94.849 Y84.663 E.02853
G1 X94.302 Y84.213 E.0285
G1 X93.714 Y83.82 E.0285
G1 X93.091 Y83.485 E.0285
G1 X92.437 Y83.214 E.02852
G1 X91.76 Y83.008 E.0285
G1 X91.058 Y82.867 E.02887
G1 X90.334 Y82.802 E.02927
; WIPE_START
G1 F7566.996
G1 X91.058 Y82.867 E-.2761
G1 X91.326 Y82.92 E-.1039
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.1 I-1.14 J-.427 P1  F42000
G1 X86.507 Y95.774 Z4.1
G1 Z3.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1200
M204 S6000
G1 X85.979 Y95.421 E.0202
G1 X85.467 Y95.001 E.02106
G1 X85.233 Y94.767 E.01055
G1 X94.767 Y85.233 E.429
G1 X94.533 Y84.999 E.01055
G1 X94.021 Y84.579 E.02107
G1 X93.47 Y84.211 E.02107
G1 X92.886 Y83.898 E.02107
G1 X92.274 Y83.645 E.02108
G1 X91.64 Y83.453 E.02108
G1 X91.002 Y83.326 E.02071
M73 P64 R5
G1 X96.674 Y88.998 E.25526
G1 X96.742 Y89.669 E.02144
G1 X96.742 Y90.331 E.02107
G1 X96.674 Y91.002 E.02144
G1 X91.002 Y96.674 E.25526
G1 X90.331 Y96.742 E.02144
G1 X89.669 Y96.742 E.02107
G1 X88.998 Y96.674 E.02144
G1 X83.326 Y91.002 E.25526
G1 X83.258 Y90.331 E.02144
G1 X83.258 Y89.669 E.02107
G1 X83.326 Y88.998 E.02144
G1 X88.998 Y83.326 E.25526
G1 X88.36 Y83.453 E.02071
G1 X87.726 Y83.645 E.02108
G1 X87.114 Y83.898 E.02108
G1 X86.53 Y84.211 E.02107
G1 X85.979 Y84.579 E.02108
G1 X85.467 Y84.999 E.02106
G1 X85.233 Y85.233 E.01055
G1 X94.767 Y94.767 E.429
G1 X94.533 Y95.001 E.01054
G1 X94.021 Y95.421 E.02107
G1 X93.493 Y95.774 E.0202
; CHANGE_LAYER
; Z_HEIGHT: 3.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X94.021 Y95.421 E-.24122
G1 X94.303 Y95.19 E-.13878
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 20/43
; update layer progress
M73 L20
M991 S0 P19 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z4.1 I1.197 J-.219 P1  F42000
G1 X91.949 Y82.351 Z4.1
G1 Z3.9
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1242
M204 S6000
G1 X92.294 Y82.437 E.01133
G1 X93.024 Y82.698 E.02467
G1 X93.726 Y83.03 E.02468
G1 X94.391 Y83.429 E.02468
G1 X95.014 Y83.891 E.02468
G1 X95.588 Y84.412 E.02468
G1 X96.109 Y84.986 E.02468
G1 X96.571 Y85.609 E.02468
G1 X96.97 Y86.274 E.02466
G1 X97.302 Y86.976 E.02469
G1 X97.563 Y87.706 E.02468
G1 X97.751 Y88.458 E.02468
G1 X97.865 Y89.225 E.02468
G1 X97.903 Y90 E.02468
G1 X97.865 Y90.775 E.02468
G1 X97.751 Y91.542 E.02468
G1 X97.563 Y92.294 E.02468
G1 X97.302 Y93.024 E.02467
G1 X96.97 Y93.726 E.02468
G1 X96.571 Y94.391 E.02468
G1 X96.109 Y95.014 E.02467
G1 X95.589 Y95.588 E.02467
G1 X95.014 Y96.109 E.02468
G1 X94.391 Y96.571 E.02469
G1 X93.726 Y96.97 E.02467
G1 X93.024 Y97.302 E.02468
G1 X92.294 Y97.563 E.02468
G1 X91.542 Y97.751 E.02467
G1 X90.775 Y97.865 E.02468
G1 X90 Y97.903 E.02468
G1 X89.225 Y97.865 E.02468
G1 X88.458 Y97.751 E.02468
G1 X87.706 Y97.563 E.02468
G1 X86.976 Y97.302 E.02468
G1 X86.274 Y96.97 E.02468
G1 X85.609 Y96.571 E.02468
G1 X84.986 Y96.109 E.02467
G1 X84.412 Y95.589 E.02468
G1 X83.891 Y95.014 E.02468
G1 X83.429 Y94.391 E.02468
G1 X83.03 Y93.726 E.02467
G1 X82.698 Y93.024 E.02469
G1 X82.437 Y92.294 E.02468
G1 X82.249 Y91.542 E.02468
G1 X82.135 Y90.775 E.02467
G1 X82.097 Y90 E.02468
G1 X82.135 Y89.225 E.02468
G1 X82.249 Y88.458 E.02468
G1 X82.437 Y87.706 E.02468
G1 X82.698 Y86.976 E.02468
G1 X83.03 Y86.274 E.02467
G1 X83.429 Y85.609 E.02468
G1 X83.891 Y84.986 E.02468
G1 X84.411 Y84.412 E.02468
G1 X84.986 Y83.891 E.02468
G1 X85.609 Y83.429 E.02468
G1 X86.274 Y83.03 E.02467
G1 X86.976 Y82.698 E.02469
G1 X87.706 Y82.437 E.02467
G1 X88.458 Y82.249 E.02468
G1 X89.233 Y82.134 E.02491
G1 X89.609 Y82.106 E.01202
G1 X90.083 Y82.099 E.01508
G1 X90.774 Y82.135 E.022
G1 X91.542 Y82.249 E.0247
G1 X91.891 Y82.336 E.01145
; COOLING_NODE: 5
M204 S250
G1 X92.044 Y81.97 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1067
M204 S5000
G1 X92.408 Y82.061 E.01106
G1 X93.175 Y82.336 E.02399
G1 X93.911 Y82.684 E.02399
G1 X94.609 Y83.102 E.02399
G1 X95.263 Y83.587 E.024
G1 X95.866 Y84.134 E.02399
G1 X96.413 Y84.737 E.024
G1 X96.898 Y85.391 E.024
G1 X97.316 Y86.089 E.02399
G1 X97.664 Y86.825 E.024
G1 X97.939 Y87.592 E.024
G1 X98.136 Y88.382 E.02399
G1 X98.256 Y89.187 E.02399
G1 X98.296 Y90 E.024
G1 X98.256 Y90.813 E.024
G1 X98.136 Y91.618 E.02399
M73 P65 R5
G1 X97.939 Y92.408 E.024
G1 X97.664 Y93.175 E.02399
G1 X97.316 Y93.911 E.024
G1 X96.898 Y94.609 E.024
G1 X96.413 Y95.263 E.02399
G1 X95.866 Y95.866 E.02399
G1 X95.263 Y96.413 E.02399
G1 X94.609 Y96.898 E.024
G1 X93.911 Y97.316 E.02399
G1 X93.175 Y97.664 E.024
G1 X92.408 Y97.939 E.024
G1 X91.618 Y98.136 E.02399
G1 X90.813 Y98.256 E.02399
G1 X90 Y98.296 E.024
G1 X89.187 Y98.256 E.024
G1 X88.382 Y98.136 E.02399
G1 X87.592 Y97.939 E.024
G1 X86.825 Y97.664 E.02399
G1 X86.089 Y97.316 E.02399
G1 X85.391 Y96.898 E.024
G1 X84.737 Y96.413 E.02399
G1 X84.134 Y95.866 E.02399
G1 X83.587 Y95.263 E.024
G1 X83.102 Y94.609 E.024
G1 X82.684 Y93.911 E.02399
G1 X82.336 Y93.175 E.024
G1 X82.061 Y92.408 E.024
G1 X81.864 Y91.618 E.02399
G1 X81.744 Y90.813 E.02399
G1 X81.704 Y90 E.024
G1 X81.744 Y89.187 E.024
G1 X81.864 Y88.382 E.02399
G1 X82.061 Y87.592 E.024
G1 X82.336 Y86.825 E.02399
G1 X82.684 Y86.089 E.02399
G1 X83.102 Y85.391 E.02399
G1 X83.587 Y84.737 E.024
G1 X84.134 Y84.134 E.02399
G1 X84.737 Y83.587 E.02399
G1 X85.391 Y83.102 E.024
G1 X86.089 Y82.684 E.02399
G1 X86.825 Y82.336 E.024
G1 X87.592 Y82.061 E.02399
G1 X88.382 Y81.864 E.02399
G1 X89.189 Y81.744 E.02406
M106 S124.95
M106 S127.5
G1 X89.592 Y81.714 E.0119
G1 X90.091 Y81.706 E.01469
M106 S124.95
M106 S127.5
G1 X90.813 Y81.744 E.02132
M106 S124.95
M106 S127.5
G1 X91.618 Y81.864 E.024
G1 X91.986 Y81.956 E.01117
M106 S124.95
; WIPE_START
G1 F1920
M204 S6000
G1 X92.408 Y82.061 E-.16539
G1 X92.94 Y82.252 E-.21461
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z4.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z4.3 F4000
            G39.3 S1
            G0 Z4.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.078 Y82.548 F42000
G1 Z3.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.534138
G1 F1242
M204 S6000
G1 X89.285 Y82.581 E.03048
G1 X88.554 Y82.688 E.02839
G1 X87.844 Y82.866 E.02809
G1 X87.155 Y83.111 E.02806
G1 X86.494 Y83.423 E.02809
G1 X85.866 Y83.798 E.02809
G1 X85.278 Y84.233 E.02808
G1 X84.736 Y84.724 E.02807
G1 X84.244 Y85.266 E.0281
G1 X83.807 Y85.853 E.02809
G1 X83.431 Y86.479 E.02807
G1 X83.117 Y87.14 E.02809
G1 X82.87 Y87.829 E.02809
G1 X82.692 Y88.538 E.02808
G1 X82.583 Y89.261 E.02808
G1 X82.547 Y89.992 E.02809
G1 X82.582 Y90.723 E.02809
G1 X82.688 Y91.446 E.02808
G1 X82.865 Y92.156 E.02808
G1 X83.111 Y92.845 E.02808
G1 X83.423 Y93.506 E.02808
G1 X83.799 Y94.134 E.0281
G1 X84.234 Y94.722 E.02808
G1 X84.724 Y95.265 E.02809
G1 X85.266 Y95.756 E.02808
G1 X85.853 Y96.193 E.02809
G1 X86.479 Y96.569 E.02807
G1 X87.14 Y96.883 E.02809
G1 X87.829 Y97.13 E.02808
G1 X88.538 Y97.308 E.02808
G1 X89.261 Y97.416 E.02808
G1 X89.992 Y97.453 E.02809
G1 X90.723 Y97.418 E.02809
G1 X91.446 Y97.312 E.02808
G1 X92.156 Y97.135 E.02808
G1 X92.845 Y96.889 E.02808
G1 X93.506 Y96.577 E.02808
G1 X94.134 Y96.202 E.02809
G1 X94.722 Y95.767 E.02808
G1 X95.265 Y95.276 E.02809
G1 X95.756 Y94.734 E.02808
G1 X96.193 Y94.147 E.02808
G1 X96.569 Y93.52 E.02808
G1 X96.883 Y92.86 E.02808
G1 X97.13 Y92.171 E.02808
G1 X97.308 Y91.462 E.02808
G1 X97.416 Y90.739 E.02808
G1 X97.454 Y90 E.0284
G1 X97.418 Y89.277 E.02778
G1 X97.312 Y88.554 E.02808
G1 X97.135 Y87.844 E.02808
G1 X96.889 Y87.155 E.02807
G1 X96.577 Y86.494 E.02808
G1 X96.202 Y85.866 E.02809
G1 X95.766 Y85.278 E.02809
G1 X95.276 Y84.736 E.02806
G1 X94.734 Y84.244 E.02811
G1 X94.147 Y83.807 E.02807
G1 X93.52 Y83.431 E.02808
G1 X92.86 Y83.117 E.02808
G1 X92.171 Y82.87 E.02809
G1 X91.462 Y82.692 E.02808
G1 X90.738 Y82.583 E.02812
G1 X90.138 Y82.551 E.02304
; WIPE_START
G1 F7939.462
G1 X90.738 Y82.583 E-.22807
G1 X91.133 Y82.642 E-.15193
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.3 I-.341 J-1.168 P1  F42000
G1 X86.326 Y84.046 Z4.3
G1 Z3.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1242
M204 S6000
G1 X86.109 Y84.176 E.00807
G1 X85.557 Y84.586 E.02187
G1 X85.047 Y85.047 E.02187
G1 X94.953 Y94.953 E.44572
G1 X94.443 Y95.414 E.02187
G1 X93.891 Y95.824 E.02187
G1 X93.302 Y96.177 E.02188
G1 X92.68 Y96.471 E.02187
G1 X92.033 Y96.703 E.02187
G1 X91.367 Y96.87 E.02187
G1 X90.709 Y96.967 E.02116
G1 X96.967 Y90.709 E.28162
G1 X97.004 Y90 E.02258
G1 X96.967 Y89.291 E.02258
G1 X90.709 Y83.033 E.28162
G1 X90.068 Y82.997 E.02043
G1 X89.291 Y83.033 E.02472
G1 X83.033 Y89.291 E.28162
G1 X82.996 Y90 E.02258
G1 X83.033 Y90.709 E.02258
G1 X89.291 Y96.967 E.28162
G1 X88.634 Y96.87 E.02116
G1 X87.967 Y96.703 E.02187
G1 X87.32 Y96.471 E.02187
G1 X86.698 Y96.177 E.02187
G1 X86.109 Y95.824 E.02187
G1 X85.557 Y95.414 E.02187
G1 X85.047 Y94.953 E.02187
G1 X94.953 Y85.047 E.44572
G1 X95.414 Y85.557 E.02187
G1 X95.824 Y86.109 E.02187
G1 X95.954 Y86.326 E.00807
; CHANGE_LAYER
; Z_HEIGHT: 4.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X95.824 Y86.109 E-.09633
G1 X95.414 Y85.557 E-.26124
G1 X95.375 Y85.513 E-.02243
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 21/43
; update layer progress
M73 L21
M991 S0 P20 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z4.3 I.864 J-.857 P1  F42000
G1 X92.024 Y82.136 Z4.3
G1 Z4.1
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1281
M204 S6000
G1 X92.736 Y82.352 E.02369
G1 X93.473 Y82.657 E.02537
G1 X94.176 Y83.033 E.02536
G1 X94.839 Y83.476 E.02536
G1 X95.455 Y83.981 E.02536
G1 X96.019 Y84.545 E.02537
G1 X96.524 Y85.161 E.02536
G1 X96.967 Y85.824 E.02537
G1 X97.343 Y86.527 E.02535
G1 X97.648 Y87.264 E.02536
G1 X97.879 Y88.026 E.02536
G1 X98.035 Y88.808 E.02537
G1 X98.113 Y89.601 E.02536
G1 X98.113 Y90.399 E.02537
G1 X98.035 Y91.192 E.02536
G1 X97.879 Y91.974 E.02537
G1 X97.648 Y92.736 E.02536
G1 X97.343 Y93.473 E.02536
G1 X96.967 Y94.176 E.02536
G1 X96.524 Y94.838 E.02535
G1 X96.018 Y95.455 E.02537
G1 X95.455 Y96.019 E.02537
G1 X94.839 Y96.524 E.02536
G1 X94.176 Y96.967 E.02535
G1 X93.473 Y97.343 E.02537
G1 X92.736 Y97.648 E.02536
G1 X91.974 Y97.879 E.02536
G1 X91.192 Y98.035 E.02537
G1 X90.399 Y98.113 E.02536
G1 X89.601 Y98.113 E.02537
G1 X88.808 Y98.035 E.02536
G1 X88.026 Y97.879 E.02537
G1 X87.264 Y97.648 E.02536
G1 X86.527 Y97.343 E.02536
G1 X85.824 Y96.967 E.02536
G1 X85.161 Y96.524 E.02536
G1 X84.545 Y96.018 E.02536
G1 X83.981 Y95.455 E.02537
G1 X83.476 Y94.839 E.02536
G1 X83.033 Y94.176 E.02536
G1 X82.657 Y93.473 E.02536
M73 P66 R5
G1 X82.352 Y92.736 E.02536
G1 X82.121 Y91.974 E.02536
G1 X81.965 Y91.192 E.02537
G1 X81.887 Y90.399 E.02536
G1 X81.887 Y89.601 E.02536
G1 X81.965 Y88.808 E.02536
G1 X82.121 Y88.026 E.02537
G1 X82.352 Y87.264 E.02536
G1 X82.657 Y86.527 E.02536
G1 X83.033 Y85.824 E.02536
G1 X83.476 Y85.161 E.02536
G1 X83.981 Y84.545 E.02536
G1 X84.545 Y83.982 E.02537
G1 X85.161 Y83.476 E.02536
G1 X85.824 Y83.033 E.02536
G1 X86.527 Y82.657 E.02537
G1 X87.264 Y82.352 E.02536
G1 X88.026 Y82.121 E.02536
G1 X88.808 Y81.965 E.02537
G1 X89.602 Y81.887 E.02539
G1 X90.306 Y81.885 E.02239
G1 X90.799 Y81.916 E.01572
G1 X91.185 Y81.964 E.01236
G1 X91.966 Y82.119 E.02536
; COOLING_NODE: 5
M204 S250
G1 X92.138 Y81.761 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1116
M204 S5000
G1 X92.869 Y81.983 E.02251
G1 X93.641 Y82.302 E.02463
G1 X94.378 Y82.696 E.02463
G1 X95.072 Y83.161 E.02462
G1 X95.718 Y83.691 E.02463
G1 X96.309 Y84.282 E.02464
G1 X96.839 Y84.928 E.02462
G1 X97.304 Y85.622 E.02463
G1 X97.698 Y86.359 E.02462
G1 X98.017 Y87.131 E.02463
G1 X98.26 Y87.931 E.02463
G1 X98.423 Y88.751 E.02464
G1 X98.505 Y89.582 E.02463
G1 X98.505 Y90.418 E.02463
G1 X98.423 Y91.249 E.02463
G1 X98.26 Y92.069 E.02464
G1 X98.017 Y92.869 E.02462
G1 X97.698 Y93.641 E.02463
G1 X97.304 Y94.378 E.02463
G1 X96.84 Y95.072 E.02462
G1 X96.309 Y95.718 E.02463
G1 X95.718 Y96.309 E.02464
G1 X95.072 Y96.839 E.02463
G1 X94.378 Y97.304 E.02462
G1 X93.641 Y97.698 E.02464
G1 X92.869 Y98.017 E.02462
G1 X92.069 Y98.26 E.02463
G1 X91.249 Y98.423 E.02463
G1 X90.418 Y98.505 E.02463
G1 X89.582 Y98.505 E.02463
G1 X88.751 Y98.423 E.02463
G1 X87.931 Y98.26 E.02464
G1 X87.131 Y98.017 E.02462
G1 X86.359 Y97.698 E.02463
G1 X85.622 Y97.304 E.02462
G1 X84.928 Y96.839 E.02463
G1 X84.282 Y96.309 E.02463
G1 X83.691 Y95.718 E.02464
G1 X83.161 Y95.072 E.02463
G1 X82.696 Y94.378 E.02463
G1 X82.302 Y93.641 E.02463
G1 X81.983 Y92.869 E.02463
G1 X81.74 Y92.069 E.02462
G1 X81.577 Y91.249 E.02464
G1 X81.495 Y90.418 E.02463
G1 X81.495 Y89.582 E.02463
G1 X81.577 Y88.751 E.02463
G1 X81.74 Y87.931 E.02463
G1 X81.983 Y87.131 E.02463
G1 X82.302 Y86.359 E.02463
G1 X82.696 Y85.622 E.02463
G1 X83.161 Y84.928 E.02462
G1 X83.691 Y84.282 E.02463
G1 X84.282 Y83.691 E.02463
G1 X84.928 Y83.161 E.02463
G1 X85.622 Y82.696 E.02462
G1 X86.359 Y82.302 E.02464
G1 X87.131 Y81.983 E.02462
G1 X87.931 Y81.74 E.02463
G1 X88.751 Y81.577 E.02463
G1 X89.582 Y81.495 E.02464
G1 X90.318 Y81.493 E.02167
M106 S124.95
M106 S127.5
G1 X90.836 Y81.526 E.0153
M106 S124.95
M106 S127.5
G1 X91.247 Y81.577 E.01221
M106 S124.95
M106 S127.5
G1 X92.069 Y81.74 E.02471
G1 X92.08 Y81.743 E.00035
M106 S124.95
; WIPE_START
G1 F2040
M204 S6000
G1 X92.869 Y81.983 E-.31304
G1 X93.031 Y82.05 E-.06696
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z4.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z4.5 F4000
            G39.3 S1
            G0 Z4.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.29 Y82.324 F42000
G1 Z4.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.512964
G1 F1281
M204 S6000
G1 X89.631 Y82.326 E.02419
G1 X88.88 Y82.399 E.02773
G1 X88.14 Y82.545 E.02771
G1 X87.419 Y82.763 E.0277
G1 X86.722 Y83.051 E.0277
G1 X86.057 Y83.406 E.02769
G1 X85.429 Y83.824 E.0277
G1 X84.846 Y84.302 E.02771
G1 X84.312 Y84.835 E.0277
G1 X83.833 Y85.417 E.0277
G1 X83.414 Y86.043 E.02769
G1 X83.058 Y86.708 E.02771
G1 X82.768 Y87.405 E.02771
G1 X82.549 Y88.126 E.02769
G1 X82.401 Y88.865 E.02771
G1 X82.326 Y89.615 E.0277
G1 X82.326 Y90.369 E.0277
G1 X82.399 Y91.12 E.02771
G1 X82.545 Y91.86 E.0277
G1 X82.763 Y92.581 E.02769
G1 X83.051 Y93.278 E.02771
G1 X83.406 Y93.943 E.02769
G1 X83.824 Y94.571 E.0277
G1 X84.302 Y95.154 E.0277
G1 X84.835 Y95.688 E.02769
G1 X85.417 Y96.167 E.02771
G1 X86.044 Y96.586 E.02769
G1 X86.708 Y96.942 E.0277
G1 X87.405 Y97.232 E.02771
G1 X88.126 Y97.451 E.02769
G1 X88.865 Y97.599 E.02771
G1 X89.615 Y97.674 E.0277
G1 X90.369 Y97.674 E.0277
G1 X91.12 Y97.601 E.0277
G1 X91.86 Y97.455 E.02771
G1 X92.581 Y97.237 E.0277
G1 X93.278 Y96.949 E.0277
G1 X93.944 Y96.594 E.0277
G1 X94.571 Y96.176 E.0277
G1 X95.154 Y95.698 E.0277
G1 X95.688 Y95.165 E.0277
G1 X96.167 Y94.583 E.0277
G1 X96.586 Y93.956 E.0277
G1 X96.942 Y93.292 E.0277
G1 X97.232 Y92.595 E.02771
G1 X97.451 Y91.874 E.0277
G1 X97.599 Y91.135 E.0277
G1 X97.674 Y90.384 E.02771
G1 X97.674 Y89.623 E.02798
G1 X97.601 Y88.88 E.02742
G1 X97.455 Y88.14 E.02771
G1 X97.237 Y87.419 E.0277
G1 X96.949 Y86.722 E.0277
G1 X96.594 Y86.057 E.02769
G1 X96.176 Y85.429 E.0277
G1 X95.698 Y84.846 E.02771
G1 X95.165 Y84.312 E.0277
G1 X94.583 Y83.833 E.0277
G1 X93.956 Y83.414 E.02771
G1 X93.292 Y83.058 E.02769
G1 X92.595 Y82.768 E.02771
G1 X91.874 Y82.549 E.02769
G1 X91.126 Y82.399 E.02804
G1 X90.349 Y82.329 E.02864
; WIPE_START
G1 F8297.108
G1 X91.126 Y82.399 E-.29622
G1 X91.342 Y82.443 E-.08378
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.5 I-.541 J-1.09 P1  F42000
G1 X83.869 Y86.151 Z4.5
G1 Z4.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1281
M204 S6000
G1 X84.181 Y85.684 E.01788
G1 X84.632 Y85.135 E.02262
G1 X84.883 Y84.883 E.01131
G1 X95.117 Y95.117 E.46047
G1 X95.368 Y94.865 E.01131
G1 X95.819 Y94.316 E.02262
G1 X96.214 Y93.725 E.02262
G1 X96.549 Y93.098 E.02262
G1 X96.821 Y92.441 E.02262
G1 X97.028 Y91.76 E.02263
G1 X97.166 Y91.063 E.02262
G1 X97.227 Y90.449 E.01963
G1 X90.449 Y97.227 E.30498
G1 X89.551 Y97.227 E.02858
G1 X82.773 Y90.449 E.30498
G1 X82.773 Y89.551 E.02858
G1 X89.551 Y82.773 E.30498
G1 X89.647 Y82.764 E.00306
G1 X90.449 Y82.773 E.02554
G1 X97.227 Y89.551 E.30498
G1 X97.166 Y88.937 E.01963
G1 X97.028 Y88.24 E.02262
G1 X96.821 Y87.559 E.02262
M73 P67 R5
G1 X96.549 Y86.902 E.02262
G1 X96.214 Y86.275 E.02262
G1 X95.819 Y85.684 E.02262
G1 X95.368 Y85.135 E.02262
G1 X95.117 Y84.883 E.01131
G1 X84.883 Y95.117 E.46047
G1 X85.134 Y95.368 E.0113
G1 X85.684 Y95.819 E.02263
G1 X86.151 Y96.131 E.01787
; CHANGE_LAYER
; Z_HEIGHT: 4.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X85.684 Y95.819 E-.21345
G1 X85.346 Y95.541 E-.16655
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 22/43
; update layer progress
M73 L22
M991 S0 P21 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z4.5 I1.092 J.538 P1  F42000
G1 X92.048 Y81.935 Z4.5
G1 Z4.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1322
M204 S6000
G1 X92.418 Y82.027 E.01215
G1 X93.188 Y82.303 E.02602
G1 X93.927 Y82.652 E.02601
G1 X94.629 Y83.073 E.02601
G1 X95.285 Y83.56 E.02602
G1 X95.891 Y84.109 E.02601
G1 X96.44 Y84.715 E.02601
G1 X96.927 Y85.371 E.02602
G1 X97.348 Y86.073 E.02601
G1 X97.697 Y86.812 E.02602
G1 X97.973 Y87.582 E.02602
G1 X98.171 Y88.375 E.02601
G1 X98.291 Y89.183 E.02601
G1 X98.331 Y90 E.02601
G1 X98.291 Y90.817 E.02602
G1 X98.171 Y91.625 E.02601
G1 X97.973 Y92.419 E.02602
G1 X97.697 Y93.188 E.02601
G1 X97.348 Y93.927 E.02602
G1 X96.927 Y94.629 E.02601
G1 X96.44 Y95.285 E.02601
G1 X95.891 Y95.891 E.02601
G1 X95.286 Y96.44 E.02601
G1 X94.629 Y96.927 E.02602
G1 X93.927 Y97.348 E.02602
G1 X93.188 Y97.697 E.02602
G1 X92.418 Y97.973 E.02601
G1 X91.625 Y98.171 E.02601
G1 X90.817 Y98.291 E.02601
G1 X90 Y98.331 E.02602
G1 X89.183 Y98.291 E.02602
G1 X88.375 Y98.171 E.02601
G1 X87.582 Y97.973 E.02601
G1 X86.812 Y97.697 E.02602
G1 X86.073 Y97.348 E.02602
G1 X85.371 Y96.927 E.02602
G1 X84.715 Y96.44 E.02601
G1 X84.109 Y95.891 E.02602
G1 X83.56 Y95.286 E.02601
G1 X83.073 Y94.629 E.02603
G1 X82.652 Y93.927 E.02601
G1 X82.303 Y93.188 E.02601
G1 X82.027 Y92.418 E.02602
G1 X81.829 Y91.625 E.02601
G1 X81.709 Y90.817 E.02601
G1 X81.669 Y90 E.02602
G1 X81.709 Y89.183 E.02602
G1 X81.829 Y88.375 E.02601
G1 X82.027 Y87.581 E.02602
G1 X82.303 Y86.812 E.02601
G1 X82.652 Y86.073 E.02602
G1 X83.073 Y85.371 E.02602
G1 X83.56 Y84.715 E.02601
G1 X84.109 Y84.109 E.02601
G1 X84.715 Y83.56 E.02601
G1 X85.371 Y83.073 E.02601
G1 X86.073 Y82.652 E.02601
G1 X86.812 Y82.303 E.02601
G1 X87.582 Y82.027 E.02602
G1 X88.375 Y81.829 E.02601
G1 X89.191 Y81.708 E.02624
G1 X89.588 Y81.678 E.01268
G1 X90.118 Y81.671 E.01688
G1 X90.816 Y81.709 E.02221
G1 X91.625 Y81.829 E.02605
G1 X91.99 Y81.92 E.01195
; COOLING_NODE: 5
M204 S250
G1 X92.143 Y81.554 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1162
M204 S5000
G1 X92.532 Y81.652 E.01182
G1 X93.338 Y81.94 E.02524
G1 X94.112 Y82.306 E.02523
G1 X94.847 Y82.746 E.02523
G1 X95.534 Y83.256 E.02524
G1 X96.169 Y83.831 E.02523
G1 X96.744 Y84.466 E.02523
G1 X97.254 Y85.153 E.02524
G1 X97.694 Y85.888 E.02523
G1 X98.06 Y86.662 E.02523
G1 X98.348 Y87.468 E.02523
G1 X98.556 Y88.298 E.02523
G1 X98.682 Y89.145 E.02523
G1 X98.724 Y90 E.02523
G1 X98.682 Y90.855 E.02524
G1 X98.556 Y91.702 E.02523
G1 X98.348 Y92.532 E.02523
G1 X98.06 Y93.338 E.02523
G1 X97.694 Y94.112 E.02524
G1 X97.254 Y94.847 E.02523
G1 X96.744 Y95.534 E.02523
G1 X96.169 Y96.169 E.02523
G1 X95.535 Y96.744 E.02523
G1 X94.847 Y97.254 E.02524
G1 X94.112 Y97.694 E.02524
G1 X93.338 Y98.06 E.02523
G1 X92.532 Y98.348 E.02523
G1 X91.702 Y98.556 E.02523
G1 X90.855 Y98.682 E.02523
G1 X90 Y98.724 E.02524
G1 X89.145 Y98.682 E.02524
G1 X88.298 Y98.556 E.02523
G1 X87.468 Y98.348 E.02523
G1 X86.661 Y98.06 E.02524
G1 X85.888 Y97.694 E.02523
G1 X85.153 Y97.254 E.02524
G1 X84.466 Y96.744 E.02523
G1 X83.831 Y96.169 E.02523
G1 X83.257 Y95.535 E.02523
G1 X82.746 Y94.847 E.02524
G1 X82.306 Y94.112 E.02523
G1 X81.94 Y93.339 E.02523
G1 X81.652 Y92.532 E.02524
G1 X81.444 Y91.702 E.02523
G1 X81.318 Y90.855 E.02523
G1 X81.276 Y90 E.02524
G1 X81.318 Y89.145 E.02523
G1 X81.444 Y88.298 E.02523
G1 X81.652 Y87.468 E.02523
G1 X81.94 Y86.662 E.02523
G1 X82.306 Y85.888 E.02523
G1 X82.746 Y85.153 E.02524
G1 X83.256 Y84.466 E.02523
G1 X83.831 Y83.831 E.02523
G1 X84.466 Y83.256 E.02523
G1 X85.153 Y82.746 E.02523
G1 X85.888 Y82.306 E.02524
G1 X86.661 Y81.94 E.02523
G1 X87.468 Y81.652 E.02524
G1 X88.298 Y81.444 E.02523
G1 X89.147 Y81.318 E.0253
M106 S124.95
M106 S127.5
G1 X89.571 Y81.287 E.01251
M106 S124.95
M106 S127.5
G1 X90.126 Y81.279 E.01638
G1 X90.855 Y81.318 E.0215
M106 S124.95
M106 S127.5
G1 X91.702 Y81.444 E.02524
G1 X92.085 Y81.54 E.01164
M106 S124.95
; WIPE_START
G1 F2160
M204 S6000
G1 X92.532 Y81.652 E-.1752
G1 X93.04 Y81.833 E-.2048
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z4.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z4.7 F4000
            G39.3 S1
            G0 Z4.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X94.015 Y83.702 F42000
G1 Z4.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1322
M204 S6000
G1 X94.153 Y83.785 E.00513
G1 X94.742 Y84.221 E.02333
G1 X95.286 Y84.714 E.02335
G1 X84.714 Y95.286 E.47569
G1 X84.221 Y94.742 E.02335
G1 X83.785 Y94.153 E.02334
G1 X83.408 Y93.524 E.02333
G1 X83.094 Y92.861 E.02335
G1 X82.847 Y92.17 E.02335
G1 X82.668 Y91.458 E.02333
G1 X82.561 Y90.733 E.02334
G1 X82.535 Y90.211 E.01662
G1 X89.789 Y97.465 E.3264
G1 X90.211 Y97.465 E.01343
G1 X97.465 Y90.211 E.3264
G1 X97.465 Y89.789 E.01343
G1 X90.209 Y82.533 E.32648
G1 X89.793 Y82.531 E.01323
G1 X82.535 Y89.789 E.3266
G1 X82.561 Y89.267 E.01662
G1 X82.668 Y88.542 E.02334
G1 X82.847 Y87.83 E.02334
G1 X83.094 Y87.139 E.02334
G1 X83.407 Y86.476 E.02334
G1 X83.785 Y85.847 E.02334
G1 X84.222 Y85.258 E.02334
G1 X84.714 Y84.714 E.02334
G1 X95.286 Y95.286 E.47569
G1 X95.778 Y94.742 E.02334
G1 X96.215 Y94.153 E.02334
G1 X96.298 Y94.015 E.00513
; WIPE_START
G1 F9580.435
G1 X96.215 Y94.153 E-.06128
G1 X95.778 Y94.742 E-.27872
G1 X95.708 Y94.82 E-.04001
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.7 I1.114 J-.49 P1  F42000
G1 X90.113 Y82.1 Z4.7
M73 P68 R5
G1 Z4.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.491329
G1 F1322
M204 S6000
G1 X89.251 Y82.133 E.03023
G1 X88.465 Y82.248 E.02784
G1 X87.713 Y82.436 E.02718
G1 X86.982 Y82.696 E.02717
G1 X86.281 Y83.027 E.02719
G1 X85.615 Y83.425 E.02718
G1 X84.992 Y83.887 E.02717
G1 X84.417 Y84.407 E.02718
G1 X83.895 Y84.981 E.02719
G1 X83.433 Y85.604 E.02718
G1 X83.034 Y86.268 E.02717
G1 X82.701 Y86.969 E.02718
G1 X82.44 Y87.699 E.02718
G1 X82.251 Y88.451 E.02718
G1 X82.136 Y89.218 E.02718
G1 X82.097 Y89.993 E.02719
G1 X82.135 Y90.768 E.02719
G1 X82.248 Y91.535 E.02718
G1 X82.436 Y92.287 E.02718
G1 X82.696 Y93.018 E.02718
G1 X83.027 Y93.719 E.02719
G1 X83.425 Y94.385 E.02718
G1 X83.887 Y95.008 E.02717
G1 X84.407 Y95.583 E.02719
G1 X84.981 Y96.104 E.02718
G1 X85.604 Y96.567 E.02718
G1 X86.269 Y96.966 E.02719
G1 X86.969 Y97.299 E.02718
G1 X87.699 Y97.56 E.02718
G1 X88.451 Y97.75 E.02718
G1 X89.218 Y97.864 E.02717
G1 X89.993 Y97.903 E.02719
G1 X90.768 Y97.865 E.02719
G1 X91.535 Y97.752 E.02718
G1 X92.287 Y97.565 E.02717
G1 X93.018 Y97.304 E.02719
G1 X93.719 Y96.973 E.02719
G1 X94.385 Y96.575 E.02717
G1 X95.008 Y96.113 E.02719
G1 X95.583 Y95.593 E.0272
G1 X96.104 Y95.019 E.02717
G1 X96.567 Y94.396 E.02718
G1 X96.966 Y93.731 E.02718
G1 X97.299 Y93.031 E.02718
G1 X97.56 Y92.301 E.02718
G1 X97.75 Y91.549 E.02719
G1 X97.864 Y90.782 E.02718
G1 X97.903 Y90.007 E.02719
G1 X97.865 Y89.225 E.02743
G1 X97.752 Y88.465 E.02694
G1 X97.564 Y87.713 E.02718
G1 X97.304 Y86.982 E.02718
G1 X96.973 Y86.281 E.02719
G1 X96.575 Y85.615 E.02718
G1 X96.113 Y84.992 E.02717
G1 X95.593 Y84.417 E.02718
G1 X95.019 Y83.895 E.02719
G1 X94.396 Y83.433 E.02718
G1 X93.731 Y83.034 E.02718
G1 X93.031 Y82.701 E.02719
G1 X92.301 Y82.44 E.02718
G1 X91.549 Y82.25 E.02718
G1 X90.781 Y82.136 E.02722
G1 X90.173 Y82.103 E.02133
; CHANGE_LAYER
; Z_HEIGHT: 4.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F8697.424
G1 X90.781 Y82.136 E-.23129
G1 X91.168 Y82.194 E-.14871
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 23/43
; update layer progress
M73 L23
M991 S0 P22 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z4.7 I.531 J1.095 P1  F42000
G1 X92.115 Y81.735 Z4.7
G1 Z4.5
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
G1 F1355
M204 S6000
G1 X92.875 Y81.965 E.02527
G1 X93.649 Y82.286 E.02665
G1 X94.387 Y82.68 E.02664
G1 X95.084 Y83.146 E.02665
G1 X95.731 Y83.677 E.02664
G1 X96.323 Y84.269 E.02665
G1 X96.854 Y84.917 E.02665
G1 X97.32 Y85.613 E.02664
G1 X97.714 Y86.351 E.02665
G1 X98.035 Y87.125 E.02664
G1 X98.278 Y87.926 E.02664
G1 X98.441 Y88.748 E.02665
G1 X98.524 Y89.581 E.02664
G1 X98.524 Y90.419 E.02665
G1 X98.441 Y91.252 E.02664
G1 X98.278 Y92.074 E.02665
G1 X98.035 Y92.875 E.02665
G1 X97.714 Y93.649 E.02665
G1 X97.32 Y94.387 E.02664
G1 X96.854 Y95.084 E.02665
G1 X96.323 Y95.731 E.02665
G1 X95.731 Y96.323 E.02665
G1 X95.084 Y96.854 E.02665
G1 X94.387 Y97.32 E.02665
G1 X93.649 Y97.714 E.02664
G1 X92.875 Y98.035 E.02665
G1 X92.074 Y98.278 E.02665
G1 X91.252 Y98.441 E.02665
G1 X90.419 Y98.524 E.02664
G1 X89.581 Y98.524 E.02665
G1 X88.748 Y98.441 E.02664
G1 X87.926 Y98.278 E.02665
G1 X87.125 Y98.035 E.02665
G1 X86.351 Y97.715 E.02664
G1 X85.613 Y97.32 E.02665
G1 X84.917 Y96.855 E.02664
G1 X84.269 Y96.323 E.02665
G1 X83.677 Y95.731 E.02665
G1 X83.146 Y95.084 E.02664
G1 X82.68 Y94.387 E.02665
G1 X82.286 Y93.649 E.02664
G1 X81.965 Y92.875 E.02665
G1 X81.722 Y92.074 E.02664
G1 X81.559 Y91.252 E.02665
G1 X81.476 Y90.419 E.02664
G1 X81.476 Y89.581 E.02665
G1 X81.559 Y88.748 E.02664
G1 X81.722 Y87.926 E.02665
G1 X81.965 Y87.125 E.02665
G1 X82.286 Y86.351 E.02664
G1 X82.68 Y85.613 E.02665
G1 X83.145 Y84.917 E.02664
G1 X83.677 Y84.269 E.02665
G1 X84.269 Y83.677 E.02665
G1 X84.916 Y83.146 E.02665
G1 X85.613 Y82.68 E.02664
G1 X86.351 Y82.286 E.02665
G1 X87.125 Y81.965 E.02665
G1 X87.926 Y81.722 E.02664
G1 X88.748 Y81.559 E.02665
G1 X89.582 Y81.476 E.02666
G1 X90.362 Y81.475 E.02483
G1 X90.838 Y81.507 E.01518
G1 X91.245 Y81.557 E.01304
G1 X92.057 Y81.719 E.02635
; COOLING_NODE: 5
M204 S250
G1 X92.229 Y81.359 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1204
M204 S5000
G1 X93.007 Y81.595 E.02397
G1 X93.817 Y81.931 E.02582
G1 X94.589 Y82.344 E.02581
G1 X95.317 Y82.83 E.02582
G1 X95.994 Y83.386 E.02581
G1 X96.614 Y84.005 E.02582
G1 X97.17 Y84.683 E.02582
G1 X97.656 Y85.411 E.02582
G1 X98.069 Y86.184 E.02582
G1 X98.405 Y86.993 E.02581
G1 X98.659 Y87.831 E.02581
G1 X98.83 Y88.69 E.02583
G1 X98.916 Y89.562 E.02581
G1 X98.916 Y90.438 E.02582
G1 X98.83 Y91.31 E.02582
G1 X98.659 Y92.169 E.02582
G1 X98.405 Y93.007 E.02581
G1 X98.069 Y93.817 E.02582
G1 X97.656 Y94.589 E.02581
G1 X97.17 Y95.317 E.02582
G1 X96.614 Y95.995 E.02582
G1 X95.995 Y96.614 E.02582
G1 X95.317 Y97.17 E.02582
G1 X94.589 Y97.656 E.02582
G1 X93.817 Y98.069 E.02581
G1 X93.007 Y98.405 E.02582
G1 X92.169 Y98.659 E.02581
G1 X91.31 Y98.83 E.02582
G1 X90.438 Y98.916 E.02582
G1 X89.562 Y98.916 E.02582
G1 X88.69 Y98.83 E.02582
G1 X87.831 Y98.659 E.02582
G1 X86.993 Y98.405 E.02582
G1 X86.184 Y98.069 E.02581
G1 X85.411 Y97.656 E.02582
G1 X84.683 Y97.17 E.02582
G1 X84.006 Y96.614 E.02582
G1 X83.386 Y95.995 E.02582
G1 X82.83 Y95.317 E.02582
G1 X82.344 Y94.589 E.02582
G1 X81.931 Y93.817 E.02581
G1 X81.595 Y93.007 E.02582
M73 P69 R5
G1 X81.341 Y92.169 E.02581
G1 X81.17 Y91.31 E.02582
G1 X81.084 Y90.438 E.02582
G1 X81.084 Y89.562 E.02582
G1 X81.17 Y88.69 E.02582
G1 X81.341 Y87.831 E.02582
G1 X81.595 Y86.993 E.02581
G1 X81.931 Y86.184 E.02582
G1 X82.344 Y85.411 E.02582
G1 X82.83 Y84.683 E.02581
G1 X83.386 Y84.005 E.02582
G1 X84.005 Y83.386 E.02582
G1 X84.683 Y82.83 E.02582
G1 X85.411 Y82.344 E.02581
G1 X86.183 Y81.931 E.02582
G1 X86.993 Y81.595 E.02582
G1 X87.831 Y81.341 E.02581
G1 X88.69 Y81.17 E.02583
G1 X89.562 Y81.084 E.02582
G1 X90.375 Y81.083 E.02396
M106 S124.95
M106 S127.5
G1 X90.876 Y81.117 E.01479
M106 S124.95
M106 S127.5
G1 X91.307 Y81.17 E.01282
M106 S124.95
M106 S127.5
G1 X92.169 Y81.341 E.0259
G1 X92.171 Y81.342 E.00007
M106 S124.95
; WIPE_START
G1 F2280
M204 S6000
G1 X93.007 Y81.595 E-.3319
G1 X93.124 Y81.644 E-.0481
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z4.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z4.9 F4000
            G39.3 S1
            G0 Z4.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.346 Y81.895 F42000
G1 Z4.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.47452
G1 F1355
M204 S6000
G1 X89.609 Y81.896 E.02485
G1 X88.816 Y81.973 E.02688
G1 X88.035 Y82.128 E.02687
G1 X87.273 Y82.358 E.02686
G1 X86.537 Y82.662 E.02685
G1 X85.834 Y83.037 E.02687
G1 X85.172 Y83.479 E.02685
G1 X84.556 Y83.984 E.02687
G1 X83.992 Y84.547 E.02687
G1 X83.487 Y85.161 E.02685
G1 X83.044 Y85.823 E.02686
G1 X82.668 Y86.525 E.02686
G1 X82.363 Y87.261 E.02687
G1 X82.131 Y88.022 E.02686
G1 X81.975 Y88.803 E.02686
G1 X81.896 Y89.595 E.02686
G1 X81.896 Y90.391 E.02685
G1 X81.973 Y91.184 E.02686
G1 X82.128 Y91.965 E.02687
G1 X82.358 Y92.727 E.02686
G1 X82.663 Y93.463 E.02686
G1 X83.037 Y94.166 E.02685
G1 X83.479 Y94.828 E.02686
G1 X83.984 Y95.444 E.02686
G1 X84.546 Y96.007 E.02686
G1 X85.161 Y96.513 E.02686
G1 X85.823 Y96.956 E.02686
G1 X86.525 Y97.332 E.02685
G1 X87.26 Y97.637 E.02686
G1 X88.022 Y97.869 E.02686
G1 X88.803 Y98.025 E.02686
G1 X89.595 Y98.104 E.02686
G1 X90.392 Y98.104 E.02686
G1 X91.184 Y98.027 E.02686
G1 X91.965 Y97.872 E.02687
G1 X92.727 Y97.642 E.02686
G1 X93.463 Y97.338 E.02685
G1 X94.166 Y96.963 E.02687
G1 X94.828 Y96.521 E.02686
G1 X95.444 Y96.016 E.02686
G1 X96.007 Y95.454 E.02686
G1 X96.513 Y94.839 E.02686
G1 X96.956 Y94.177 E.02685
G1 X97.332 Y93.475 E.02686
G1 X97.637 Y92.74 E.02687
G1 X97.869 Y91.978 E.02686
G1 X98.025 Y91.197 E.02686
G1 X98.104 Y90.405 E.02686
G1 X98.104 Y89.602 E.02709
G1 X98.027 Y88.816 E.02663
G1 X97.872 Y88.035 E.02687
G1 X97.642 Y87.273 E.02686
G1 X97.337 Y86.537 E.02686
G1 X96.963 Y85.835 E.02685
G1 X96.521 Y85.172 E.02686
G1 X96.016 Y84.556 E.02686
G1 X95.453 Y83.992 E.02687
G1 X94.839 Y83.487 E.02685
G1 X94.177 Y83.044 E.02686
G1 X93.475 Y82.668 E.02686
G1 X92.739 Y82.363 E.02687
G1 X91.978 Y82.131 E.02685
G1 X91.188 Y81.973 E.02717
G1 X90.406 Y81.901 E.02651
; WIPE_START
G1 F9036.159
G1 X91.188 Y81.973 E-.29865
G1 X91.398 Y82.015 E-.08135
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z4.9 I-1.135 J-.438 P1  F42000
G1 X85.829 Y96.456 Z4.9
G1 Z4.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1355
M204 S6000
G1 X85.416 Y96.18 E.01577
G1 X84.833 Y95.701 E.02402
G1 X84.566 Y95.434 E.01201
G1 X95.434 Y84.566 E.48906
G1 X95.167 Y84.299 E.01202
G1 X94.584 Y83.82 E.02402
G1 X93.956 Y83.4 E.02402
G1 X93.29 Y83.044 E.02403
G1 X92.592 Y82.755 E.02403
G1 X91.87 Y82.536 E.02402
G1 X91.117 Y82.386 E.02443
G1 X90.759 Y82.342 E.01147
G1 X90.011 Y82.313 E.02382
G1 X82.315 Y90.009 E.3463
G1 X82.315 Y89.991 E.00059
G1 X90.009 Y97.685 E.34624
G1 X89.991 Y97.685 E.00059
G1 X97.685 Y89.991 E.34624
G1 X97.685 Y90.009 E.00059
G1 X89.989 Y82.313 E.3463
G1 X89.624 Y82.315 E.01162
G1 X88.871 Y82.389 E.02408
G1 X88.13 Y82.536 E.02403
G1 X87.408 Y82.755 E.02402
G1 X86.71 Y83.044 E.02402
G1 X86.044 Y83.4 E.02404
G1 X85.416 Y83.82 E.02401
G1 X84.833 Y84.299 E.02403
G1 X84.566 Y84.566 E.01201
G1 X95.434 Y95.434 E.48906
G1 X95.701 Y95.167 E.01201
G1 X96.18 Y94.583 E.02403
G1 X96.456 Y94.172 E.01576
; CHANGE_LAYER
; Z_HEIGHT: 4.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X96.18 Y94.583 E-.18827
G1 X95.86 Y94.973 E-.19173
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 24/43
; update layer progress
M73 L24
M991 S0 P23 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z4.9 I1.173 J-.325 P1  F42000
G1 X92.138 Y81.554 Z4.9
G1 Z4.7
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1400
M204 S6000
G1 X92.532 Y81.652 E.01292
G1 X93.338 Y81.941 E.02723
G1 X94.112 Y82.307 E.02724
G1 X94.846 Y82.747 E.02724
G1 X95.534 Y83.257 E.02724
G1 X96.168 Y83.832 E.02724
G1 X96.743 Y84.466 E.02724
G1 X97.253 Y85.154 E.02724
G1 X97.693 Y85.888 E.02723
G1 X98.059 Y86.662 E.02725
G1 X98.348 Y87.468 E.02723
G1 X98.556 Y88.298 E.02724
G1 X98.681 Y89.145 E.02724
G1 X98.723 Y90 E.02724
G1 X98.681 Y90.855 E.02724
G1 X98.556 Y91.702 E.02724
G1 X98.348 Y92.532 E.02724
G1 X98.059 Y93.338 E.02723
G1 X97.693 Y94.112 E.02725
G1 X97.253 Y94.846 E.02723
G1 X96.743 Y95.534 E.02724
G1 X96.168 Y96.168 E.02724
G1 X95.534 Y96.743 E.02723
G1 X94.846 Y97.253 E.02725
G1 X94.112 Y97.693 E.02724
G1 X93.338 Y98.059 E.02724
G1 X92.532 Y98.348 E.02724
G1 X91.702 Y98.556 E.02724
G1 X90.855 Y98.681 E.02724
G1 X90 Y98.723 E.02724
G1 X89.145 Y98.681 E.02724
G1 X88.298 Y98.556 E.02723
G1 X87.468 Y98.348 E.02724
G1 X86.662 Y98.059 E.02724
G1 X85.888 Y97.693 E.02724
G1 X85.154 Y97.253 E.02724
G1 X84.466 Y96.743 E.02723
G1 X83.832 Y96.168 E.02724
G1 X83.257 Y95.534 E.02723
G1 X82.747 Y94.846 E.02725
G1 X82.307 Y94.112 E.02723
G1 X81.941 Y93.338 E.02725
G1 X81.652 Y92.532 E.02723
G1 X81.444 Y91.702 E.02724
G1 X81.319 Y90.855 E.02723
G1 X81.277 Y90 E.02724
G1 X81.319 Y89.145 E.02724
G1 X81.444 Y88.298 E.02723
G1 X81.652 Y87.468 E.02724
G1 X81.941 Y86.662 E.02724
M73 P70 R5
G1 X82.307 Y85.888 E.02723
G1 X82.747 Y85.154 E.02724
G1 X83.257 Y84.466 E.02724
G1 X83.832 Y83.832 E.02724
G1 X84.466 Y83.257 E.02724
G1 X85.154 Y82.747 E.02724
G1 X85.888 Y82.307 E.02723
G1 X86.662 Y81.941 E.02725
G1 X87.468 Y81.652 E.02723
G1 X88.298 Y81.444 E.02724
G1 X89.152 Y81.318 E.02747
G1 X89.568 Y81.287 E.01326
G1 X90.174 Y81.281 E.01928
G1 X90.853 Y81.318 E.02165
G1 X91.702 Y81.444 E.02729
G1 X92.08 Y81.539 E.01241
; COOLING_NODE: 5
M204 S250
G1 X92.234 Y81.173 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1243
M204 S5000
G1 X92.646 Y81.277 E.01253
G1 X93.488 Y81.578 E.02637
G1 X94.297 Y81.961 E.02637
G1 X95.064 Y82.42 E.02636
G1 X95.783 Y82.953 E.02637
G1 X96.446 Y83.554 E.02636
G1 X97.047 Y84.217 E.02636
G1 X97.58 Y84.936 E.02637
G1 X98.039 Y85.703 E.02636
G1 X98.422 Y86.512 E.02637
G1 X98.723 Y87.354 E.02637
G1 X98.941 Y88.222 E.02636
G1 X99.072 Y89.106 E.02636
G1 X99.116 Y90 E.02637
G1 X99.072 Y90.894 E.02637
G1 X98.941 Y91.778 E.02636
G1 X98.723 Y92.646 E.02637
G1 X98.422 Y93.488 E.02636
G1 X98.039 Y94.297 E.02637
G1 X97.58 Y95.064 E.02636
G1 X97.047 Y95.783 E.02637
G1 X96.446 Y96.446 E.02636
G1 X95.783 Y97.046 E.02636
G1 X95.064 Y97.58 E.02638
G1 X94.297 Y98.039 E.02637
G1 X93.488 Y98.422 E.02637
G1 X92.646 Y98.723 E.02637
G1 X91.778 Y98.941 E.02636
G1 X90.894 Y99.072 E.02636
G1 X90 Y99.116 E.02637
G1 X89.106 Y99.072 E.02637
G1 X88.222 Y98.941 E.02636
G1 X87.354 Y98.723 E.02636
G1 X86.512 Y98.422 E.02637
G1 X85.703 Y98.039 E.02637
G1 X84.935 Y97.579 E.02637
G1 X84.217 Y97.047 E.02636
G1 X83.554 Y96.446 E.02637
G1 X82.953 Y95.783 E.02636
G1 X82.42 Y95.064 E.02637
G1 X81.961 Y94.297 E.02636
G1 X81.578 Y93.488 E.02637
G1 X81.277 Y92.646 E.02637
M73 P70 R4
G1 X81.059 Y91.778 E.02636
G1 X80.928 Y90.894 E.02636
G1 X80.884 Y90 E.02637
G1 X80.928 Y89.106 E.02637
G1 X81.059 Y88.222 E.02636
G1 X81.277 Y87.354 E.02637
G1 X81.578 Y86.511 E.02637
G1 X81.961 Y85.703 E.02636
G1 X82.42 Y84.935 E.02637
G1 X82.953 Y84.217 E.02637
G1 X83.554 Y83.554 E.02636
G1 X84.217 Y82.953 E.02636
G1 X84.936 Y82.42 E.02637
G1 X85.703 Y81.961 E.02636
G1 X86.512 Y81.578 E.02637
G1 X87.354 Y81.277 E.02637
G1 X88.222 Y81.059 E.02636
G1 X89.109 Y80.928 E.02643
M106 S124.95
M106 S127.5
G1 X89.551 Y80.895 E.01308
M106 S124.95
M106 S127.5
G1 X90.183 Y80.889 E.01861
G1 X90.893 Y80.928 E.02097
M106 S124.95
M106 S127.5
G1 X91.778 Y81.059 E.02638
G1 X92.175 Y81.159 E.01206
M106 S124.95
; WIPE_START
G1 F2520
M204 S6000
G1 X92.646 Y81.277 E-.18441
G1 X93.131 Y81.45 E-.19559
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z5.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z5.1 F4000
            G39.3 S1
            G0 Z5.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.592 Y81.704 F42000
G1 Z4.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.455014
G1 F1400
M204 S6000
G1 X88.397 Y81.844 E.03877
G1 X87.593 Y82.043 E.02668
G1 X86.824 Y82.318 E.02628
G1 X86.087 Y82.666 E.02627
G1 X85.387 Y83.085 E.02628
G1 X84.731 Y83.57 E.02626
G1 X84.126 Y84.118 E.02628
G1 X83.578 Y84.722 E.02628
G1 X83.092 Y85.377 E.02628
G1 X82.672 Y86.076 E.02628
G1 X82.322 Y86.813 E.02628
G1 X82.047 Y87.581 E.02627
G1 X81.848 Y88.372 E.02628
G1 X81.728 Y89.179 E.02627
G1 X81.687 Y89.994 E.02628
G1 X81.727 Y90.809 E.02628
G1 X81.846 Y91.616 E.02628
G1 X82.043 Y92.407 E.02627
G1 X82.318 Y93.176 E.02628
G1 X82.666 Y93.913 E.02627
G1 X83.085 Y94.613 E.02628
G1 X83.57 Y95.269 E.02628
G1 X84.118 Y95.874 E.02627
G1 X84.722 Y96.422 E.02628
G1 X85.377 Y96.908 E.02627
G1 X86.076 Y97.328 E.02628
G1 X86.813 Y97.678 E.02627
G1 X87.581 Y97.953 E.02628
G1 X88.372 Y98.152 E.02627
G1 X89.179 Y98.272 E.02627
G1 X89.994 Y98.313 E.02628
G1 X90.809 Y98.273 E.02628
G1 X91.616 Y98.154 E.02628
G1 X92.407 Y97.957 E.02627
G1 X93.176 Y97.682 E.02628
G1 X93.913 Y97.334 E.02627
G1 X94.613 Y96.915 E.02628
G1 X95.269 Y96.43 E.02628
G1 X95.874 Y95.882 E.02627
G1 X96.422 Y95.278 E.02628
G1 X96.908 Y94.623 E.02627
G1 X97.328 Y93.924 E.02628
G1 X97.678 Y93.187 E.02628
G1 X97.953 Y92.419 E.02627
G1 X98.152 Y91.628 E.02628
G1 X98.272 Y90.821 E.02627
G1 X98.313 Y90.006 E.02628
G1 X98.273 Y89.185 E.02648
G1 X98.154 Y88.384 E.02608
G1 X97.957 Y87.593 E.02627
G1 X97.682 Y86.825 E.02627
G1 X97.334 Y86.087 E.02627
G1 X96.915 Y85.387 E.02628
G1 X96.43 Y84.731 E.02627
G1 X95.882 Y84.126 E.02627
G1 X95.278 Y83.578 E.02629
G1 X94.623 Y83.091 E.02628
G1 X93.924 Y82.672 E.02627
G1 X93.187 Y82.322 E.02627
G1 X92.419 Y82.047 E.02629
G1 X91.628 Y81.848 E.02627
G1 X90.819 Y81.728 E.02632
G1 X90.167 Y81.691 E.02104
G1 X89.652 Y81.703 E.01659
; WIPE_START
G1 F9463.865
G1 X90.167 Y81.691 E-.19568
G1 X90.652 Y81.718 E-.18432
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.1 I-1.182 J.291 P1  F42000
G1 X94.325 Y96.611 Z5.1
G1 Z4.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1400
M204 S6000
G1 X95.014 Y96.109 E.02712
G1 X95.588 Y95.588 E.02468
G1 X84.412 Y84.412 E.50293
G1 X84.986 Y83.891 E.02468
G1 X85.609 Y83.429 E.02467
G1 X86.274 Y83.03 E.02468
G1 X86.976 Y82.698 E.02468
G1 X87.706 Y82.437 E.02467
G1 X88.458 Y82.249 E.02468
G1 X89.238 Y82.133 E.02508
G1 X89.779 Y82.103 E.01726
G1 X97.893 Y90.217 E.36507
G1 X97.893 Y89.783 E.01379
G1 X89.783 Y97.893 E.3649
G1 X90 Y97.903 E.0069
G1 X90.217 Y97.893 E.0069
G1 X82.107 Y89.783 E.3649
G1 X82.107 Y90.217 E.01379
G1 X90.219 Y82.105 E.36501
G1 X90.773 Y82.134 E.01763
G1 X91.542 Y82.249 E.02474
G1 X92.294 Y82.437 E.02467
G1 X93.025 Y82.698 E.02469
G1 X93.725 Y83.03 E.02467
G1 X94.391 Y83.429 E.02467
G1 X95.014 Y83.891 E.02468
G1 X95.588 Y84.412 E.02468
G1 X84.412 Y95.588 E.50293
G1 X83.891 Y95.014 E.02467
G1 X83.389 Y94.325 E.02713
; CHANGE_LAYER
; Z_HEIGHT: 4.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X83.891 Y95.014 E-.32398
G1 X83.99 Y95.123 E-.05602
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 25/43
; update layer progress
M73 L25
M991 S0 P24 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z5.1 I1.045 J.623 P1  F42000
G1 X92.196 Y81.364 Z5.1
G1 Z4.9
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1442
M204 S6000
G1 X93.002 Y81.609 E.0268
G1 X93.81 Y81.944 E.02783
G1 X94.582 Y82.356 E.02784
G1 X95.309 Y82.842 E.02782
G1 X95.985 Y83.397 E.02783
G1 X96.604 Y84.015 E.02783
G1 X97.158 Y84.691 E.02782
G1 X97.644 Y85.418 E.02782
G1 X98.057 Y86.19 E.02784
G1 X98.391 Y86.997 E.02782
G1 X98.645 Y87.835 E.02783
G1 X98.816 Y88.692 E.02783
G1 X98.901 Y89.563 E.02783
G1 X98.901 Y90.437 E.02783
G1 X98.816 Y91.307 E.02782
G1 X98.645 Y92.166 E.02784
M73 P71 R4
G1 X98.391 Y93.002 E.02782
G1 X98.056 Y93.81 E.02783
G1 X97.644 Y94.582 E.02783
G1 X97.158 Y95.309 E.02782
G1 X96.603 Y95.985 E.02784
G1 X95.985 Y96.603 E.02782
G1 X95.309 Y97.158 E.02783
G1 X94.582 Y97.644 E.02783
G1 X93.811 Y98.056 E.02782
G1 X93.002 Y98.391 E.02783
G1 X92.166 Y98.645 E.02782
G1 X91.308 Y98.816 E.02784
G1 X90.437 Y98.901 E.02782
G1 X89.563 Y98.901 E.02783
G1 X88.693 Y98.816 E.02782
G1 X87.834 Y98.645 E.02784
G1 X86.998 Y98.391 E.02782
G1 X86.19 Y98.056 E.02783
G1 X85.418 Y97.644 E.02783
G1 X84.691 Y97.158 E.02782
G1 X84.015 Y96.604 E.02783
G1 X83.396 Y95.985 E.02784
G1 X82.842 Y95.309 E.02782
G1 X82.356 Y94.582 E.02783
G1 X81.944 Y93.811 E.02782
G1 X81.609 Y93.002 E.02783
G1 X81.355 Y92.166 E.02783
G1 X81.184 Y91.307 E.02784
G1 X81.099 Y90.437 E.02782
G1 X81.099 Y89.563 E.02783
G1 X81.184 Y88.692 E.02783
G1 X81.355 Y87.834 E.02783
G1 X81.609 Y86.998 E.02783
G1 X81.944 Y86.19 E.02783
G1 X82.356 Y85.418 E.02783
G1 X82.842 Y84.691 E.02782
G1 X83.396 Y84.015 E.02783
G1 X84.015 Y83.397 E.02783
G1 X84.691 Y82.842 E.02783
G1 X85.418 Y82.356 E.02783
G1 X86.19 Y81.944 E.02782
G1 X86.997 Y81.609 E.02783
G1 X87.834 Y81.355 E.02783
G1 X88.692 Y81.184 E.02783
G1 X89.563 Y81.099 E.02782
G1 X90.43 Y81.098 E.02759
G1 X91.308 Y81.184 E.02807
G1 X92.138 Y81.35 E.02695
; COOLING_NODE: 5
M204 S250
G1 X92.31 Y80.989 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1277
M204 S5000
G1 X93.135 Y81.239 E.02539
G1 X93.978 Y81.589 E.02691
G1 X94.784 Y82.019 E.02692
G1 X95.543 Y82.526 E.02691
G1 X96.249 Y83.106 E.02691
G1 X96.894 Y83.752 E.02692
G1 X97.474 Y84.457 E.02691
G1 X97.981 Y85.216 E.02691
G1 X98.411 Y86.022 E.02692
G1 X98.761 Y86.865 E.02691
G1 X99.026 Y87.739 E.02691
G1 X99.204 Y88.635 E.02691
G1 X99.293 Y89.543 E.02691
G1 X99.293 Y90.457 E.02691
G1 X99.204 Y91.365 E.02691
G1 X99.026 Y92.261 E.02692
G1 X98.761 Y93.135 E.02691
G1 X98.411 Y93.978 E.02691
G1 X97.981 Y94.784 E.02692
G1 X97.474 Y95.543 E.02691
G1 X96.894 Y96.249 E.02692
G1 X96.249 Y96.894 E.0269
G1 X95.543 Y97.474 E.02692
G1 X94.783 Y97.981 E.02691
G1 X93.978 Y98.411 E.0269
G1 X93.135 Y98.761 E.02691
G1 X92.261 Y99.026 E.02691
G1 X91.365 Y99.204 E.02692
G1 X90.457 Y99.293 E.02691
G1 X89.543 Y99.293 E.02692
G1 X88.635 Y99.204 E.02691
G1 X87.739 Y99.026 E.02692
G1 X86.865 Y98.761 E.02691
G1 X86.022 Y98.411 E.02691
G1 X85.216 Y97.981 E.02692
G1 X84.457 Y97.474 E.02691
G1 X83.751 Y96.894 E.02691
G1 X83.106 Y96.248 E.02692
G1 X82.526 Y95.543 E.0269
G1 X82.019 Y94.784 E.02691
G1 X81.589 Y93.978 E.02691
G1 X81.239 Y93.135 E.02691
G1 X80.974 Y92.261 E.02691
G1 X80.796 Y91.365 E.02692
G1 X80.707 Y90.457 E.02691
G1 X80.707 Y89.543 E.02691
G1 X80.796 Y88.635 E.02691
G1 X80.974 Y87.739 E.02691
G1 X81.239 Y86.865 E.02691
G1 X81.589 Y86.022 E.02692
G1 X82.019 Y85.216 E.02691
G1 X82.526 Y84.457 E.02691
G1 X83.106 Y83.751 E.02691
G1 X83.751 Y83.106 E.02692
G1 X84.457 Y82.526 E.02691
G1 X85.217 Y82.019 E.02691
G1 X86.022 Y81.589 E.02691
G1 X86.865 Y81.239 E.02691
G1 X87.739 Y80.974 E.02691
G1 X88.635 Y80.796 E.02692
G1 X89.543 Y80.707 E.02691
G1 X90.449 Y80.706 E.02668
G1 X91.365 Y80.796 E.02714
G1 X92.253 Y80.973 E.02667
M106 S124.95
; WIPE_START
G1 F2520
M204 S6000
G1 X93.135 Y81.239 E-.35012
G1 X93.207 Y81.269 E-.02988
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z5.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z5.3 F4000
            G39.3 S1
            G0 Z5.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X94.467 Y83.248 F42000
G1 Z4.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1442
M204 S6000
G1 X94.829 Y83.489 E.01385
G1 X95.443 Y83.994 E.0253
G1 X95.725 Y84.275 E.01266
G1 X84.275 Y95.725 E.51519
G1 X84.557 Y96.006 E.01266
G1 X85.171 Y96.511 E.02531
G1 X85.833 Y96.953 E.02531
G1 X86.534 Y97.328 E.02531
G1 X87.269 Y97.632 E.0253
G1 X88.031 Y97.863 E.02532
G1 X88.811 Y98.018 E.02531
G1 X89.582 Y98.094 E.02466
G1 X98.094 Y89.582 E.38302
G1 X98.094 Y90.418 E.0266
G1 X89.582 Y81.906 E.38302
G1 X90.348 Y81.902 E.02438
G1 X90.417 Y81.907 E.00219
G1 X81.906 Y90.418 E.38297
G1 X81.906 Y89.582 E.0266
G1 X90.418 Y98.094 E.38302
G1 X91.189 Y98.018 E.02465
G1 X91.969 Y97.863 E.02532
G1 X92.731 Y97.632 E.02531
G1 X93.466 Y97.327 E.02531
G1 X94.167 Y96.953 E.02531
G1 X94.828 Y96.511 E.02531
G1 X95.444 Y96.006 E.02532
G1 X95.725 Y95.725 E.01265
G1 X84.275 Y84.275 E.51519
G1 X84.557 Y83.994 E.01265
G1 X85.171 Y83.489 E.02531
G1 X85.533 Y83.248 E.01384
; WIPE_START
G1 F9580.435
G1 X85.171 Y83.489 E-.16533
G1 X84.735 Y83.848 E-.21467
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.3 I.465 J1.125 P1  F42000
G1 X90.41 Y81.503 Z5.3
G1 Z4.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.441679
G1 F1442
M204 S6000
G1 X89.589 Y81.501 E.0256
G1 X88.757 Y81.583 E.02603
G1 X87.938 Y81.745 E.02604
G1 X87.139 Y81.987 E.02602
G1 X86.367 Y82.306 E.02603
G1 X85.631 Y82.699 E.02602
G1 X84.936 Y83.162 E.02602
G1 X84.29 Y83.692 E.02603
G1 X83.7 Y84.282 E.02601
G1 X83.169 Y84.927 E.02603
G1 X82.705 Y85.621 E.02601
G1 X82.311 Y86.357 E.02604
G1 X81.991 Y87.128 E.02602
G1 X81.748 Y87.927 E.02602
G1 X81.584 Y88.746 E.02603
G1 X81.502 Y89.577 E.02601
G1 X81.501 Y90.412 E.02603
G1 X81.583 Y91.243 E.02602
G1 X81.745 Y92.062 E.02603
G1 X81.987 Y92.861 E.02602
G1 X82.306 Y93.633 E.02603
G1 X82.699 Y94.369 E.02602
G1 X83.162 Y95.064 E.02603
G1 X83.692 Y95.71 E.02603
G1 X84.282 Y96.301 E.02602
G1 X84.927 Y96.831 E.02603
G1 X85.621 Y97.295 E.02602
G1 X86.357 Y97.689 E.02603
G1 X87.128 Y98.009 E.02602
M73 P72 R4
G1 X87.927 Y98.252 E.02603
G1 X88.746 Y98.416 E.02602
G1 X89.577 Y98.498 E.02602
G1 X90.412 Y98.499 E.02603
G1 X91.243 Y98.417 E.02602
G1 X92.062 Y98.255 E.02603
G1 X92.861 Y98.013 E.02602
G1 X93.633 Y97.694 E.02603
G1 X94.369 Y97.301 E.02602
G1 X95.064 Y96.838 E.02602
G1 X95.71 Y96.308 E.02603
G1 X96.3 Y95.718 E.02602
G1 X96.831 Y95.073 E.02603
G1 X97.295 Y94.379 E.02603
G1 X97.689 Y93.643 E.02602
G1 X98.009 Y92.872 E.02603
G1 X98.252 Y92.073 E.02602
G1 X98.416 Y91.254 E.02603
G1 X98.498 Y90.423 E.02602
G1 X98.498 Y89.583 E.02621
G1 X98.417 Y88.757 E.02584
G1 X98.255 Y87.938 E.02603
G1 X98.013 Y87.139 E.02602
G1 X97.694 Y86.367 E.02602
G1 X97.301 Y85.631 E.02603
G1 X96.838 Y84.936 E.02602
G1 X96.308 Y84.29 E.02604
G1 X95.718 Y83.699 E.02603
G1 X95.073 Y83.169 E.02602
G1 X94.379 Y82.705 E.02603
G1 X93.643 Y82.311 E.02602
G1 X92.872 Y81.991 E.02603
G1 X92.073 Y81.748 E.02602
G1 X91.249 Y81.583 E.0262
G1 X90.47 Y81.508 E.02438
; CHANGE_LAYER
; Z_HEIGHT: 5.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9780.352
G1 X91.249 Y81.583 E-.29728
G1 X91.462 Y81.626 E-.08272
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 26/43
; update layer progress
M73 L26
M991 S0 P25 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z5.3 I.594 J1.062 P1  F42000
G1 X92.233 Y81.195 Z5.3
G1 Z5.1
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
G1 F1478
M204 S6000
G1 X93.061 Y81.446 E.02754
G1 X93.884 Y81.787 E.02836
G1 X94.671 Y82.207 E.02837
G1 X95.412 Y82.703 E.02836
G1 X96.101 Y83.268 E.02837
G1 X96.732 Y83.899 E.02837
G1 X97.297 Y84.588 E.02837
G1 X97.793 Y85.329 E.02837
G1 X98.213 Y86.116 E.02837
G1 X98.554 Y86.939 E.02836
G1 X98.813 Y87.792 E.02837
G1 X98.987 Y88.667 E.02837
G1 X99.074 Y89.554 E.02836
G1 X99.074 Y90.446 E.02837
G1 X98.987 Y91.333 E.02836
G1 X98.813 Y92.208 E.02837
G1 X98.554 Y93.061 E.02837
G1 X98.213 Y93.884 E.02837
G1 X97.792 Y94.671 E.02837
G1 X97.297 Y95.412 E.02836
G1 X96.732 Y96.101 E.02837
G1 X96.101 Y96.732 E.02837
G1 X95.412 Y97.297 E.02837
G1 X94.671 Y97.792 E.02836
G1 X93.884 Y98.213 E.02838
G1 X93.061 Y98.554 E.02836
G1 X92.208 Y98.813 E.02837
G1 X91.333 Y98.987 E.02837
G1 X90.446 Y99.074 E.02836
G1 X89.554 Y99.074 E.02837
G1 X88.667 Y98.987 E.02836
G1 X87.792 Y98.813 E.02837
G1 X86.939 Y98.554 E.02837
G1 X86.116 Y98.213 E.02837
G1 X85.329 Y97.793 E.02837
G1 X84.588 Y97.297 E.02836
G1 X83.899 Y96.732 E.02837
G1 X83.268 Y96.101 E.02837
G1 X82.703 Y95.412 E.02837
G1 X82.207 Y94.671 E.02837
G1 X81.787 Y93.884 E.02837
G1 X81.446 Y93.061 E.02836
G1 X81.187 Y92.208 E.02837
G1 X81.013 Y91.333 E.02837
G1 X80.926 Y90.446 E.02836
G1 X80.926 Y89.554 E.02837
G1 X81.013 Y88.667 E.02836
G1 X81.187 Y87.792 E.02837
G1 X81.446 Y86.939 E.02837
G1 X81.787 Y86.116 E.02836
G1 X82.208 Y85.329 E.02838
G1 X82.703 Y84.588 E.02836
G1 X83.268 Y83.899 E.02837
G1 X83.899 Y83.268 E.02837
G1 X84.588 Y82.703 E.02837
G1 X85.329 Y82.207 E.02837
G1 X86.116 Y81.787 E.02836
G1 X86.939 Y81.446 E.02837
G1 X87.792 Y81.187 E.02836
G1 X88.667 Y81.013 E.02838
G1 X89.556 Y80.926 E.02843
G1 X90.245 Y80.921 E.02191
G1 X90.895 Y80.959 E.02073
G1 X91.326 Y81.012 E.01381
G1 X92.174 Y81.181 E.02753
; COOLING_NODE: 5
M204 S250
G1 X92.346 Y80.82 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1308
M204 S5000
G1 X93.193 Y81.076 E.02607
G1 X94.052 Y81.432 E.02741
G1 X94.873 Y81.871 E.02742
G1 X95.646 Y82.387 E.02741
G1 X96.365 Y82.978 E.02741
G1 X97.022 Y83.635 E.02741
G1 X97.613 Y84.354 E.02741
G1 X98.129 Y85.128 E.02741
G1 X98.568 Y85.948 E.02742
G1 X98.924 Y86.807 E.02741
G1 X99.194 Y87.697 E.02741
G1 X99.375 Y88.609 E.02742
G1 X99.466 Y89.535 E.02741
G1 X99.466 Y90.465 E.02741
G1 X99.375 Y91.391 E.02741
G1 X99.194 Y92.303 E.02742
G1 X98.924 Y93.193 E.02741
G1 X98.568 Y94.052 E.02741
G1 X98.129 Y94.873 E.02741
G1 X97.613 Y95.646 E.02741
G1 X97.022 Y96.365 E.02742
G1 X96.365 Y97.022 E.02741
G1 X95.646 Y97.613 E.02741
G1 X94.873 Y98.129 E.02741
G1 X94.052 Y98.568 E.02742
G1 X93.193 Y98.924 E.02741
G1 X92.303 Y99.194 E.02741
G1 X91.391 Y99.375 E.02742
G1 X90.465 Y99.466 E.02741
G1 X89.535 Y99.466 E.02741
G1 X88.609 Y99.375 E.02741
G1 X87.697 Y99.194 E.02742
G1 X86.807 Y98.924 E.02741
G1 X85.948 Y98.568 E.02741
G1 X85.128 Y98.129 E.02741
G1 X84.354 Y97.613 E.02741
G1 X83.635 Y97.023 E.02741
G1 X82.978 Y96.365 E.02742
G1 X82.388 Y95.646 E.02741
G1 X81.871 Y94.872 E.02742
G1 X81.432 Y94.052 E.02741
G1 X81.076 Y93.193 E.02741
G1 X80.806 Y92.303 E.02741
G1 X80.625 Y91.391 E.02742
G1 X80.534 Y90.465 E.02741
G1 X80.534 Y89.535 E.02741
G1 X80.625 Y88.609 E.02741
G1 X80.806 Y87.697 E.02742
G1 X81.076 Y86.807 E.02741
G1 X81.432 Y85.948 E.02741
G1 X81.871 Y85.127 E.02742
G1 X82.387 Y84.354 E.02741
G1 X82.978 Y83.635 E.02741
G1 X83.635 Y82.978 E.02741
G1 X84.354 Y82.387 E.02741
G1 X85.128 Y81.871 E.02741
G1 X85.948 Y81.432 E.02741
G1 X86.807 Y81.076 E.02741
G1 X87.697 Y80.806 E.02741
G1 X88.609 Y80.625 E.02742
G1 X89.536 Y80.534 E.02743
G1 X90.255 Y80.529 E.0212
G1 X90.931 Y80.568 E.01995
M106 S124.95
M106 S127.5
G1 X91.388 Y80.624 E.01359
M106 S124.95
M106 S127.5
G1 X92.289 Y80.804 E.02706
M106 S124.95
; WIPE_START
G1 F2640
M204 S6000
G1 X93.193 Y81.076 E-.35895
G1 X93.244 Y81.098 E-.02105
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z5.5
G1 X0 Y90 F18000 ; move to safe pos
M73 P73 R4
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z5.5 F4000
            G39.3 S1
            G0 Z5.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X88.727 Y81.414 F42000
G1 Z5.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.435153
G1 F1478
M204 S6000
G1 X87.89 Y81.576 E.02613
G1 X87.074 Y81.823 E.02612
G1 X86.287 Y82.149 E.02613
G1 X85.535 Y82.551 E.02613
G1 X84.826 Y83.024 E.02614
G1 X84.168 Y83.565 E.02612
G1 X83.565 Y84.168 E.02614
G1 X83.024 Y84.826 E.02612
G1 X82.551 Y85.535 E.02613
G1 X82.149 Y86.287 E.02614
G1 X81.823 Y87.074 E.02612
G1 X81.575 Y87.89 E.02613
G1 X81.409 Y88.726 E.02614
G1 X81.326 Y89.574 E.02612
G1 X81.326 Y90.426 E.02613
G1 X81.409 Y91.274 E.02613
G1 X81.575 Y92.11 E.02613
G1 X81.823 Y92.926 E.02613
G1 X82.149 Y93.713 E.02612
G1 X82.551 Y94.465 E.02613
G1 X83.024 Y95.174 E.02614
G1 X83.565 Y95.832 E.02612
G1 X84.168 Y96.435 E.02614
G1 X84.826 Y96.976 E.02612
G1 X85.535 Y97.449 E.02613
G1 X86.287 Y97.851 E.02613
G1 X87.074 Y98.177 E.02613
G1 X87.89 Y98.425 E.02612
G1 X88.726 Y98.591 E.02614
G1 X89.574 Y98.674 E.02612
G1 X90.426 Y98.674 E.02613
G1 X91.274 Y98.591 E.02613
G1 X92.11 Y98.425 E.02614
G1 X92.926 Y98.177 E.02613
G1 X93.713 Y97.851 E.02612
G1 X94.465 Y97.449 E.02615
G1 X95.173 Y96.976 E.02611
G1 X95.832 Y96.435 E.02613
G1 X96.435 Y95.832 E.02614
G1 X96.976 Y95.174 E.02612
G1 X97.449 Y94.465 E.02613
G1 X97.851 Y93.713 E.02613
G1 X98.177 Y92.926 E.02613
G1 X98.425 Y92.11 E.02612
G1 X98.591 Y91.274 E.02614
G1 X98.674 Y90.426 E.02612
G1 X98.674 Y89.574 E.02613
G1 X98.591 Y88.726 E.02612
G1 X98.425 Y87.89 E.02614
G1 X98.177 Y87.074 E.02613
G1 X97.851 Y86.287 E.02612
G1 X97.449 Y85.535 E.02614
G1 X96.976 Y84.826 E.02613
G1 X96.435 Y84.168 E.02612
G1 X95.832 Y83.565 E.02614
G1 X95.174 Y83.024 E.02612
G1 X94.465 Y82.551 E.02613
G1 X93.713 Y82.149 E.02614
G1 X92.926 Y81.823 E.02612
G1 X92.113 Y81.576 E.02606
G1 X90.855 Y81.352 E.03916
G1 X90.235 Y81.316 E.01905
G1 X89.577 Y81.32 E.02016
G1 X88.786 Y81.407 E.02439
; WIPE_START
G1 F9943.071
G1 X89.577 Y81.32 E-.30237
G1 X89.781 Y81.319 E-.07763
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.5 I-1.162 J.36 P1  F42000
G1 X94.608 Y96.897 Z5.5
G1 Z5.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1478
M204 S6000
G1 X95.262 Y96.412 E.02591
G1 X95.866 Y95.866 E.0259
G1 X84.134 Y84.134 E.52787
G1 X84.738 Y83.588 E.02591
G1 X85.391 Y83.103 E.0259
G1 X86.09 Y82.684 E.0259
G1 X86.826 Y82.336 E.0259
G1 X87.592 Y82.062 E.0259
G1 X88.382 Y81.864 E.0259
G1 X89.199 Y81.743 E.0263
G1 X89.404 Y81.728 E.00652
G1 X98.266 Y90.59 E.39879
G1 X98.295 Y90 E.0188
G1 X98.266 Y89.41 E.0188
G1 X89.41 Y98.266 E.39852
G1 X90 Y98.295 E.0188
G1 X90.59 Y98.266 E.0188
G1 X81.734 Y89.41 E.39852
G1 X81.705 Y90 E.0188
G1 X81.734 Y90.59 E.0188
G1 X90.592 Y81.732 E.39859
G1 X90.81 Y81.744 E.00695
G1 X91.618 Y81.864 E.026
G1 X92.408 Y82.062 E.0259
G1 X93.174 Y82.336 E.0259
G1 X93.91 Y82.684 E.0259
G1 X94.608 Y83.103 E.0259
G1 X95.262 Y83.588 E.02591
G1 X95.866 Y84.134 E.0259
G1 X84.134 Y95.866 E.52787
G1 X83.588 Y95.262 E.0259
G1 X83.103 Y94.608 E.02591
; CHANGE_LAYER
; Z_HEIGHT: 5.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X83.588 Y95.262 E-.30946
G1 X83.712 Y95.4 E-.07054
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 27/43
; update layer progress
M73 L27
M991 S0 P26 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z5.5 I1.046 J.622 P1  F42000
G1 X92.261 Y81.034 Z5.5
G1 Z5.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1517
M204 S6000
G1 X92.687 Y81.141 E.01399
G1 X93.543 Y81.447 E.02891
G1 X94.364 Y81.835 E.02891
G1 X95.144 Y82.302 E.02891
G1 X95.873 Y82.844 E.02891
G1 X96.547 Y83.454 E.02891
G1 X97.157 Y84.127 E.0289
G1 X97.698 Y84.857 E.02891
G1 X98.165 Y85.636 E.0289
G1 X98.553 Y86.457 E.02892
G1 X98.859 Y87.312 E.0289
G1 X99.08 Y88.194 E.02891
G1 X99.213 Y89.092 E.0289
G1 X99.258 Y90 E.02891
G1 X99.213 Y90.908 E.02891
G1 X99.08 Y91.806 E.02891
G1 X98.859 Y92.687 E.0289
G1 X98.553 Y93.543 E.02891
G1 X98.165 Y94.364 E.0289
G1 X97.698 Y95.143 E.02891
G1 X97.157 Y95.873 E.02891
G1 X96.546 Y96.547 E.02891
G1 X95.873 Y97.156 E.0289
G1 X95.143 Y97.698 E.02892
G1 X94.364 Y98.165 E.0289
G1 X93.543 Y98.553 E.02891
G1 X92.687 Y98.859 E.02891
G1 X91.806 Y99.08 E.02891
G1 X90.908 Y99.213 E.02891
G1 X90 Y99.258 E.02891
G1 X89.092 Y99.213 E.02891
G1 X88.194 Y99.08 E.0289
G1 X87.313 Y98.859 E.02891
G1 X86.457 Y98.553 E.02891
G1 X85.636 Y98.165 E.02891
G1 X84.856 Y97.698 E.02891
G1 X84.127 Y97.157 E.0289
G1 X83.453 Y96.546 E.02892
G1 X82.843 Y95.873 E.0289
G1 X82.302 Y95.143 E.02892
G1 X81.835 Y94.364 E.0289
G1 X81.447 Y93.543 E.0289
G1 X81.141 Y92.687 E.02892
G1 X80.92 Y91.806 E.02891
G1 X80.787 Y90.908 E.0289
G1 X80.742 Y90 E.02891
G1 X80.787 Y89.092 E.02891
G1 X80.92 Y88.194 E.02891
G1 X81.141 Y87.313 E.02891
G1 X81.447 Y86.457 E.02891
G1 X81.835 Y85.636 E.02891
G1 X82.302 Y84.857 E.0289
G1 X82.843 Y84.127 E.02891
G1 X83.454 Y83.454 E.02891
G1 X84.127 Y82.844 E.02891
G1 X84.857 Y82.302 E.02891
G1 X85.636 Y81.835 E.0289
G1 X86.457 Y81.447 E.02892
G1 X87.313 Y81.141 E.0289
G1 X88.194 Y80.92 E.02891
G1 X89.1 Y80.786 E.02914
G1 X89.544 Y80.753 E.01418
G1 X90.052 Y80.743 E.01615
G1 X90.907 Y80.786 E.02725
G1 X91.806 Y80.92 E.02892
G1 X92.203 Y81.019 E.013
; COOLING_NODE: 5
M204 S250
G1 X92.356 Y80.653 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1335
M204 S5000
M73 P74 R4
G1 X92.801 Y80.765 E.01353
G1 X93.693 Y81.084 E.02792
G1 X94.549 Y81.489 E.02791
G1 X95.362 Y81.976 E.02792
G1 X96.122 Y82.54 E.02791
G1 X96.824 Y83.176 E.02791
G1 X97.46 Y83.878 E.02791
G1 X98.024 Y84.638 E.02791
G1 X98.511 Y85.451 E.02791
G1 X98.916 Y86.307 E.02793
G1 X99.235 Y87.199 E.02791
G1 X99.465 Y88.117 E.02791
G1 X99.604 Y89.054 E.02791
G1 X99.651 Y90 E.02792
G1 X99.604 Y90.946 E.02792
G1 X99.465 Y91.883 E.02791
G1 X99.235 Y92.801 E.02791
G1 X98.916 Y93.693 E.02792
G1 X98.511 Y94.549 E.02791
G1 X98.024 Y95.361 E.02791
G1 X97.46 Y96.122 E.02792
G1 X96.824 Y96.824 E.02791
G1 X96.122 Y97.46 E.02791
G1 X95.361 Y98.024 E.02792
G1 X94.549 Y98.511 E.02791
G1 X93.693 Y98.916 E.02791
G1 X92.801 Y99.235 E.02792
G1 X91.883 Y99.465 E.02791
G1 X90.946 Y99.604 E.02791
G1 X90 Y99.651 E.02792
G1 X89.054 Y99.604 E.02792
G1 X88.117 Y99.465 E.02791
G1 X87.199 Y99.235 E.02791
G1 X86.307 Y98.916 E.02792
G1 X85.451 Y98.511 E.02791
G1 X84.638 Y98.024 E.02792
G1 X83.878 Y97.46 E.02791
G1 X83.176 Y96.824 E.02792
G1 X82.54 Y96.122 E.02791
G1 X81.976 Y95.361 E.02792
G1 X81.489 Y94.549 E.0279
G1 X81.084 Y93.693 E.02791
G1 X80.765 Y92.801 E.02792
G1 X80.535 Y91.883 E.02791
G1 X80.396 Y90.946 E.02791
G1 X80.349 Y90 E.02792
G1 X80.396 Y89.054 E.02792
G1 X80.535 Y88.117 E.02791
G1 X80.765 Y87.199 E.02791
G1 X81.084 Y86.307 E.02792
G1 X81.489 Y85.451 E.02791
G1 X81.976 Y84.639 E.02791
G1 X82.54 Y83.878 E.02792
G1 X83.176 Y83.176 E.02791
G1 X83.878 Y82.54 E.02791
G1 X84.639 Y81.976 E.02792
G1 X85.451 Y81.489 E.02791
G1 X86.307 Y81.084 E.02792
G1 X87.199 Y80.765 E.02791
G1 X88.117 Y80.535 E.02791
G1 X89.056 Y80.396 E.02798
M106 S124.95
M106 S127.5
G1 X89.526 Y80.361 E.01387
G1 X90.058 Y80.351 E.01568
M106 S124.95
M106 S127.5
G1 X90.946 Y80.396 E.0262
M106 S124.95
M106 S127.5
G1 X91.883 Y80.535 E.02791
G1 X92.298 Y80.639 E.01261
M106 S124.95
; WIPE_START
G1 F2640
M204 S6000
G1 X92.801 Y80.765 E-.19722
G1 X93.254 Y80.927 E-.18278
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z5.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z5.7 F4000
            G39.3 S1
            G0 Z5.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.048 Y81.135 F42000
G1 Z5.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.419749
G1 F1517
M204 S6000
G1 X89.14 Y81.176 E.02677
G1 X88.271 Y81.305 E.02588
G1 X87.426 Y81.516 E.02564
G1 X86.607 Y81.809 E.02562
G1 X85.821 Y82.181 E.02563
G1 X85.075 Y82.628 E.02562
G1 X84.376 Y83.147 E.02563
G1 X83.731 Y83.731 E.02562
G1 X83.147 Y84.376 E.02562
G1 X82.628 Y85.075 E.02563
G1 X82.181 Y85.821 E.02562
G1 X81.809 Y86.607 E.02563
G1 X81.516 Y87.426 E.02562
G1 X81.305 Y88.271 E.02564
G1 X81.177 Y89.131 E.02562
G1 X81.134 Y90 E.02563
G1 X81.177 Y90.869 E.02563
G1 X81.305 Y91.729 E.02562
G1 X81.516 Y92.574 E.02563
G1 X81.809 Y93.393 E.02563
G1 X82.181 Y94.179 E.02563
G1 X82.628 Y94.925 E.02562
G1 X83.147 Y95.624 E.02563
G1 X83.731 Y96.269 E.02562
G1 X84.376 Y96.853 E.02564
G1 X85.074 Y97.371 E.02562
G1 X85.821 Y97.819 E.02563
G1 X86.607 Y98.191 E.02563
G1 X87.426 Y98.484 E.02562
G1 X88.271 Y98.695 E.02564
G1 X89.131 Y98.823 E.02561
G1 X90 Y98.866 E.02563
G1 X90.869 Y98.823 E.02563
G1 X91.729 Y98.695 E.02562
G1 X92.574 Y98.484 E.02563
G1 X93.393 Y98.191 E.02562
G1 X94.179 Y97.819 E.02563
G1 X94.925 Y97.372 E.02561
G1 X95.624 Y96.853 E.02564
G1 X96.269 Y96.269 E.02561
G1 X96.853 Y95.624 E.02563
G1 X97.372 Y94.925 E.02563
G1 X97.819 Y94.179 E.02562
G1 X98.191 Y93.393 E.02562
G1 X98.484 Y92.574 E.02563
G1 X98.695 Y91.729 E.02563
G1 X98.823 Y90.869 E.02562
G1 X98.866 Y90 E.02563
G1 X98.823 Y89.131 E.02563
G1 X98.695 Y88.271 E.02561
G1 X98.484 Y87.426 E.02564
G1 X98.191 Y86.607 E.02561
G1 X97.819 Y85.821 E.02564
G1 X97.371 Y85.075 E.02562
G1 X96.853 Y84.376 E.02563
G1 X96.269 Y83.731 E.02562
G1 X95.624 Y83.147 E.02563
G1 X94.925 Y82.629 E.02563
G1 X94.179 Y82.181 E.02562
G1 X93.393 Y81.809 E.02563
G1 X92.574 Y81.516 E.02562
G1 X91.729 Y81.305 E.02564
G1 X90.868 Y81.177 E.02564
G1 X90.108 Y81.138 E.02243
; WIPE_START
G1 F10349.523
G1 X90.868 Y81.177 E-.28932
G1 X91.104 Y81.212 E-.09068
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.7 I-.543 J-1.089 P1  F42000
G1 X82.975 Y85.262 Z5.7
G1 Z5.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1517
M204 S6000
G1 X83.185 Y84.946 E.01207
G1 X83.714 Y84.302 E.0265
G1 X84.008 Y84.008 E.01324
G1 X95.992 Y95.992 E.53924
G1 X96.286 Y95.698 E.01324
G1 X96.814 Y95.054 E.02649
G1 X97.277 Y94.362 E.02648
G1 X97.67 Y93.627 E.02651
G1 X97.988 Y92.858 E.02649
G1 X98.23 Y92.061 E.02649
G1 X98.392 Y91.245 E.0265
G1 X98.44 Y90.764 E.01538
G1 X89.236 Y81.56 E.41413
G1 X89.584 Y81.526 E.01111
G1 X90.409 Y81.526 E.02625
G1 X90.763 Y81.561 E.01133
G1 X81.56 Y90.764 E.41411
G1 X81.526 Y90.416 E.01111
G1 X81.526 Y89.584 E.02649
G1 X81.56 Y89.236 E.01111
G1 X90.764 Y98.44 E.41413
G1 X90.416 Y98.474 E.01111
G1 X89.584 Y98.474 E.02649
G1 X89.236 Y98.44 E.01111
G1 X98.44 Y89.236 E.41413
G1 X98.392 Y88.755 E.01538
G1 X98.23 Y87.939 E.02649
G1 X97.988 Y87.142 E.0265
G1 X97.67 Y86.373 E.02648
G1 X97.277 Y85.638 E.0265
G1 X96.815 Y84.946 E.02648
G1 X96.286 Y84.303 E.02649
G1 X95.992 Y84.008 E.01325
G1 X84.008 Y95.992 E.53924
G1 X84.302 Y96.286 E.01325
G1 X84.946 Y96.815 E.02649
G1 X85.262 Y97.025 E.01207
; CHANGE_LAYER
; Z_HEIGHT: 5.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X84.946 Y96.815 E-.14416
G1 X84.466 Y96.421 E-.23584
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 28/43
; update layer progress
M73 L28
M991 S0 P27 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z5.7 I1.087 J.548 P1  F42000
G1 X92.298 Y80.875 Z5.7
G1 Z5.5
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1553
M204 S6000
G1 X92.735 Y80.984 E.01433
G1 X93.605 Y81.296 E.02942
G1 X94.441 Y81.691 E.02943
G1 X95.235 Y82.167 E.02942
G1 X95.977 Y82.717 E.02941
G1 X96.662 Y83.338 E.02942
G1 X97.283 Y84.023 E.02942
G1 X97.834 Y84.766 E.02942
G1 X98.309 Y85.558 E.0294
G1 X98.704 Y86.395 E.02943
G1 X99.016 Y87.265 E.02941
G1 X99.24 Y88.162 E.02942
G1 X99.376 Y89.077 E.02942
G1 X99.421 Y90 E.02941
G1 X99.376 Y90.924 E.02943
G1 X99.24 Y91.838 E.02942
G1 X99.016 Y92.735 E.02941
G1 X98.704 Y93.605 E.02942
G1 X98.309 Y94.441 E.02942
G1 X97.834 Y95.234 E.02942
G1 X97.283 Y95.977 E.02941
G1 X96.662 Y96.662 E.02942
G1 X95.977 Y97.283 E.02942
G1 X95.234 Y97.834 E.02942
G1 X94.441 Y98.309 E.02942
G1 X93.605 Y98.704 E.02942
G1 X92.735 Y99.016 E.02942
G1 X91.838 Y99.24 E.02941
G1 X90.924 Y99.376 E.02942
G1 X90 Y99.421 E.02943
G1 X89.076 Y99.376 E.02943
G1 X88.162 Y99.24 E.0294
G1 X87.265 Y99.016 E.02942
G1 X86.395 Y98.704 E.02942
G1 X85.559 Y98.309 E.02942
G1 X84.766 Y97.834 E.02942
G1 X84.023 Y97.283 E.02943
G1 X83.338 Y96.662 E.02941
G1 X82.717 Y95.977 E.02942
G1 X82.166 Y95.234 E.02942
M73 P75 R4
G1 X81.691 Y94.441 E.02942
G1 X81.296 Y93.605 E.02942
G1 X80.984 Y92.735 E.02941
G1 X80.76 Y91.838 E.02941
G1 X80.624 Y90.924 E.02942
G1 X80.579 Y90 E.02943
G1 X80.624 Y89.076 E.02943
G1 X80.76 Y88.162 E.02941
G1 X80.984 Y87.265 E.02941
G1 X81.296 Y86.395 E.02943
G1 X81.691 Y85.559 E.02941
G1 X82.166 Y84.766 E.02942
G1 X82.717 Y84.023 E.02942
G1 X83.338 Y83.338 E.02942
G1 X84.023 Y82.717 E.02942
G1 X84.766 Y82.166 E.02942
G1 X85.559 Y81.691 E.02943
G1 X86.395 Y81.296 E.02941
G1 X87.265 Y80.984 E.02943
G1 X88.162 Y80.76 E.02942
G1 X89.077 Y80.624 E.02942
G1 X89.531 Y80.59 E.01451
G1 X90.326 Y80.587 E.02526
G1 X90.927 Y80.624 E.01917
G1 X91.838 Y80.76 E.0293
G1 X92.24 Y80.86 E.01319
; COOLING_NODE: 5
M204 S250
G1 X92.393 Y80.495 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1359
M204 S5000
G1 X92.848 Y80.609 E.01383
G1 X93.755 Y80.933 E.02839
G1 X94.626 Y81.345 E.02839
G1 X95.452 Y81.84 E.02838
G1 X96.226 Y82.414 E.02838
G1 X96.939 Y83.061 E.02838
G1 X97.586 Y83.774 E.02838
G1 X98.16 Y84.548 E.02839
G1 X98.417 Y84.955 E.01419
G1 X98.871 Y85.804 E.02838
G1 X99.24 Y86.694 E.02839
G1 X99.519 Y87.616 E.02838
G1 X99.707 Y88.56 E.02838
G1 X99.802 Y89.518 E.02839
G1 X99.802 Y90.481 E.02838
G1 X99.766 Y90.962 E.0142
G1 X99.625 Y91.915 E.02838
G1 X99.391 Y92.849 E.02838
G1 X99.067 Y93.755 E.02839
G1 X98.655 Y94.626 E.02839
G1 X98.16 Y95.452 E.02839
G1 X97.586 Y96.226 E.02838
G1 X96.939 Y96.939 E.02838
G1 X96.226 Y97.586 E.02838
G1 X95.452 Y98.16 E.02839
G1 X94.626 Y98.655 E.02838
G1 X93.756 Y99.067 E.02838
G1 X92.848 Y99.391 E.02839
G1 X91.915 Y99.625 E.02838
G1 X90.962 Y99.766 E.02838
G1 X90 Y99.814 E.02839
G1 X89.038 Y99.766 E.02839
G1 X88.086 Y99.625 E.02838
G1 X87.152 Y99.391 E.02838
G1 X86.245 Y99.067 E.02839
G1 X85.374 Y98.655 E.02838
G1 X84.548 Y98.16 E.02838
G1 X83.774 Y97.586 E.02839
G1 X83.061 Y96.939 E.02838
G1 X82.414 Y96.226 E.02839
G1 X81.84 Y95.452 E.02838
G1 X81.345 Y94.626 E.02838
G1 X80.933 Y93.755 E.02838
G1 X80.609 Y92.849 E.02838
G1 X80.375 Y91.915 E.02838
G1 X80.234 Y90.962 E.02838
G1 X80.198 Y90.481 E.0142
G1 X80.198 Y89.519 E.02838
G1 X80.293 Y88.56 E.02839
G1 X80.481 Y87.615 E.02838
G1 X80.76 Y86.694 E.02838
G1 X81.129 Y85.804 E.02839
G1 X81.583 Y84.955 E.02839
G1 X82.118 Y84.154 E.02838
G1 X82.729 Y83.41 E.02838
G1 X83.41 Y82.729 E.02839
G1 X84.154 Y82.118 E.02838
G1 X84.955 Y81.583 E.02838
G1 X85.804 Y81.129 E.02839
G1 X86.694 Y80.76 E.02839
G1 X87.615 Y80.481 E.02838
G1 X88.56 Y80.293 E.02838
G1 X89.516 Y80.198 E.02832
M106 S124.95
M106 S127.5
G1 X90.337 Y80.195 E.02418
G1 X90.963 Y80.234 E.0185
M106 S124.95
M106 S127.5
G1 X91.914 Y80.375 E.02835
G1 X92.335 Y80.48 E.01278
M106 S124.95
; WIPE_START
G1 F2640
M204 S6000
G1 X92.848 Y80.609 E-.20115
G1 X93.292 Y80.768 E-.17885
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z5.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z5.9 F4000
            G39.3 S1
            G0 Z5.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.563 Y80.981 F42000
G1 Z5.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.393981
G1 F1553
M204 S6000
G1 X88.679 Y81.064 E.02436
G1 X87.809 Y81.236 E.02433
G1 X86.961 Y81.493 E.02432
G1 X86.142 Y81.832 E.02433
G1 X85.36 Y82.249 E.02431
G1 X84.622 Y82.742 E.02433
G1 X83.937 Y83.304 E.02433
G1 X83.31 Y83.93 E.02432
G1 X82.747 Y84.615 E.02433
G1 X82.254 Y85.352 E.02432
G1 X81.836 Y86.134 E.02433
G1 X81.496 Y86.952 E.02432
G1 X81.238 Y87.801 E.02433
G1 X81.065 Y88.67 E.02433
G1 X80.978 Y89.552 E.02432
G1 X80.977 Y90.439 E.02433
G1 X81.064 Y91.321 E.02432
G1 X81.236 Y92.191 E.02433
G1 X81.493 Y93.039 E.02432
G1 X81.832 Y93.858 E.02432
G1 X82.249 Y94.64 E.02433
G1 X82.742 Y95.378 E.02433
G1 X83.304 Y96.063 E.02433
G1 X83.93 Y96.69 E.02433
G1 X84.615 Y97.253 E.02433
G1 X85.352 Y97.746 E.02432
G1 X86.134 Y98.164 E.02432
G1 X86.952 Y98.504 E.02433
G1 X87.801 Y98.762 E.02433
G1 X88.67 Y98.935 E.02433
G1 X89.552 Y99.022 E.02432
G1 X90.439 Y99.023 E.02433
G1 X91.321 Y98.936 E.02433
G1 X92.191 Y98.764 E.02433
G1 X93.039 Y98.507 E.02432
G1 X93.858 Y98.168 E.02432
G1 X94.64 Y97.75 E.02433
G1 X95.378 Y97.258 E.02432
G1 X96.063 Y96.696 E.02433
G1 X96.69 Y96.07 E.02433
G1 X97.253 Y95.385 E.02432
G1 X97.746 Y94.648 E.02432
G1 X98.164 Y93.866 E.02433
G1 X98.504 Y93.047 E.02433
G1 X98.762 Y92.199 E.02432
G1 X98.935 Y91.33 E.02433
G1 X99.022 Y90.448 E.02432
G1 X99.023 Y89.561 E.02432
G1 X98.99 Y89.115 E.01229
G1 X98.86 Y88.238 E.02432
G1 X98.645 Y87.378 E.02434
G1 X98.346 Y86.543 E.02432
G1 X97.967 Y85.741 E.02434
G1 X97.511 Y84.981 E.02431
G1 X96.983 Y84.269 E.02433
G1 X96.388 Y83.612 E.02433
G1 X95.731 Y83.017 E.02432
G1 X95.019 Y82.489 E.02432
G1 X94.259 Y82.033 E.02433
G1 X93.457 Y81.654 E.02433
G1 X92.622 Y81.355 E.02432
G1 X91.762 Y81.14 E.02434
G1 X90.886 Y81.012 E.02428
G1 X90.317 Y80.971 E.01567
G1 X89.623 Y80.981 E.01903
; WIPE_START
G1 F11109.206
G1 X90.317 Y80.971 E-.26349
G1 X90.622 Y80.993 E-.11651
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z5.9 I-1.101 J.518 P1  F42000
G1 X97.15 Y94.863 Z5.9
G1 Z5.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1553
M204 S6000
G1 X96.953 Y95.157 E.01126
G1 X96.415 Y95.814 E.02703
G1 X96.114 Y96.114 E.01352
G1 X83.886 Y83.886 E.55023
G1 X84.186 Y83.586 E.01351
G1 X84.843 Y83.047 E.02704
G1 X85.55 Y82.575 E.02703
M73 P76 R4
G1 X86.299 Y82.174 E.02703
G1 X87.084 Y81.849 E.02703
G1 X87.896 Y81.602 E.02703
G1 X88.73 Y81.437 E.02704
G1 X89.078 Y81.402 E.01114
G1 X98.598 Y90.922 E.42835
G1 X98.647 Y90.425 E.01589
G1 X98.647 Y89.575 E.02703
G1 X98.598 Y89.078 E.01589
G1 X89.078 Y98.598 E.42835
G1 X89.575 Y98.647 E.01589
G1 X90.425 Y98.647 E.02703
G1 X90.922 Y98.598 E.01589
G1 X81.402 Y89.078 E.42835
G1 X81.353 Y89.575 E.01589
G1 X81.353 Y90.425 E.02703
G1 X81.402 Y90.922 E.01589
G1 X90.93 Y81.394 E.42873
G1 X91.258 Y81.434 E.01051
G1 X92.103 Y81.602 E.02743
G1 X92.917 Y81.849 E.02703
G1 X93.701 Y82.174 E.02702
G1 X94.451 Y82.575 E.02705
G1 X95.157 Y83.047 E.02702
G1 X95.814 Y83.586 E.02704
G1 X96.114 Y83.886 E.01352
G1 X83.886 Y96.114 E.55023
G1 X83.586 Y95.814 E.01351
G1 X83.047 Y95.157 E.02703
G1 X82.85 Y94.863 E.01126
; CHANGE_LAYER
; Z_HEIGHT: 5.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X83.047 Y95.157 E-.13452
G1 X83.456 Y95.656 E-.24548
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 29/43
; update layer progress
M73 L29
M991 S0 P28 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z5.9 I1.046 J.622 P1  F42000
G1 X92.334 Y80.721 Z5.9
G1 Z5.7
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1589
M204 S6000
G1 X92.781 Y80.833 E.01465
G1 X93.666 Y81.15 E.02991
G1 X94.516 Y81.552 E.02992
G1 X95.322 Y82.035 E.02991
G1 X96.077 Y82.595 E.02992
G1 X96.774 Y83.226 E.0299
G1 X97.405 Y83.923 E.02991
G1 X97.965 Y84.678 E.02991
G1 X98.448 Y85.484 E.02992
G1 X98.85 Y86.334 E.02992
G1 X99.167 Y87.219 E.02992
G1 X99.396 Y88.131 E.02991
G1 X99.533 Y89.061 E.02992
G1 X99.58 Y90 E.0299
G1 X99.533 Y90.939 E.02992
G1 X99.396 Y91.869 E.02991
G1 X99.167 Y92.781 E.02991
G1 X98.85 Y93.666 E.02992
G1 X98.448 Y94.516 E.02991
G1 X97.965 Y95.322 E.02992
G1 X97.405 Y96.078 E.02992
G1 X96.774 Y96.774 E.0299
G1 X96.077 Y97.405 E.02992
G1 X95.322 Y97.965 E.02991
G1 X94.516 Y98.448 E.0299
G1 X93.666 Y98.85 E.02992
G1 X92.781 Y99.167 E.02991
G1 X91.869 Y99.396 E.02991
G1 X90.939 Y99.533 E.0299
G1 X90 Y99.58 E.02992
G1 X89.061 Y99.533 E.02991
G1 X88.131 Y99.396 E.02992
G1 X87.219 Y99.167 E.02991
G1 X86.334 Y98.85 E.02992
G1 X85.484 Y98.448 E.02991
G1 X84.678 Y97.965 E.02991
G1 X83.923 Y97.405 E.02992
G1 X83.226 Y96.774 E.0299
G1 X82.595 Y96.077 E.0299
G1 X82.035 Y95.322 E.02992
G1 X81.552 Y94.516 E.0299
G1 X81.149 Y93.666 E.02993
M73 P76 R3
G1 X80.833 Y92.781 E.02991
G1 X80.604 Y91.869 E.0299
G1 X80.467 Y90.939 E.02991
G1 X80.42 Y90 E.02991
G1 X80.467 Y89.061 E.02993
G1 X80.604 Y88.131 E.0299
G1 X80.833 Y87.219 E.02991
G1 X81.15 Y86.334 E.02991
G1 X81.552 Y85.484 E.02992
G1 X82.035 Y84.678 E.0299
G1 X82.595 Y83.923 E.02992
G1 X83.226 Y83.226 E.02991
G1 X83.923 Y82.595 E.02991
G1 X84.678 Y82.035 E.02992
G1 X85.484 Y81.552 E.02991
G1 X86.334 Y81.15 E.02991
G1 X87.219 Y80.833 E.02992
G1 X88.131 Y80.604 E.0299
G1 X89.061 Y80.467 E.02991
G1 X89.527 Y80.432 E.01487
G1 X90.124 Y80.424 E.01899
G1 X90.945 Y80.467 E.02617
G1 X91.869 Y80.604 E.02972
G1 X92.276 Y80.706 E.01334
; COOLING_NODE: 5
M204 S250
G1 X92.44 Y80.332 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1380
M204 S5000
G1 X92.894 Y80.458 E.01389
G1 X93.36 Y80.611 E.01444
G1 X93.816 Y80.787 E.01442
G1 X94.263 Y80.986 E.01443
G1 X94.701 Y81.206 E.01442
G1 X95.54 Y81.709 E.02884
G1 X95.94 Y81.991 E.01443
G1 X96.326 Y82.292 E.01443
G1 X96.697 Y82.611 E.01442
G1 X97.051 Y82.949 E.01442
G1 X97.389 Y83.304 E.01444
G1 X97.708 Y83.674 E.01441
G1 X98.009 Y84.06 E.01443
G1 X98.291 Y84.46 E.01442
G1 X98.553 Y84.874 E.01444
G1 X98.794 Y85.299 E.01442
G1 X99.014 Y85.736 E.01442
G1 X99.213 Y86.184 E.01443
G1 X99.389 Y86.641 E.01443
G1 X99.542 Y87.106 E.01443
G1 X99.673 Y87.577 E.01441
G1 X99.78 Y88.055 E.01443
G1 X99.864 Y88.537 E.01442
G1 X99.924 Y89.023 E.01443
G1 X99.96 Y89.511 E.01442
G1 X99.972 Y90 E.01443
G1 X99.96 Y90.489 E.01442
G1 X99.924 Y90.978 E.01443
G1 X99.78 Y91.945 E.02884
G1 X99.673 Y92.423 E.01442
G1 X99.389 Y93.359 E.02884
G1 X99.213 Y93.816 E.01443
G1 X99.014 Y94.263 E.01442
G1 X98.553 Y95.127 E.02885
G1 X98.291 Y95.54 E.01442
G1 X98.01 Y95.94 E.01442
G1 X97.708 Y96.326 E.01444
G1 X97.389 Y96.697 E.01442
G1 X97.051 Y97.051 E.01442
G1 X96.697 Y97.389 E.01442
G1 X95.94 Y98.009 E.02884
G1 X95.54 Y98.291 E.01443
G1 X95.126 Y98.553 E.01443
G1 X94.264 Y99.014 E.02883
G1 X93.816 Y99.213 E.01443
G1 X93.359 Y99.389 E.01442
G1 X92.423 Y99.673 E.02884
G1 X91.945 Y99.78 E.01443
G1 X91.463 Y99.864 E.01442
G1 X90.977 Y99.924 E.01443
G1 X90.489 Y99.96 E.01443
G1 X90 Y99.972 E.01442
G1 X89.511 Y99.96 E.01442
G1 X88.537 Y99.864 E.02884
G1 X88.055 Y99.78 E.01443
G1 X87.577 Y99.673 E.01442
G1 X86.641 Y99.389 E.02884
G1 X86.184 Y99.213 E.01443
G1 X85.737 Y99.014 E.01442
G1 X84.873 Y98.553 E.02885
G1 X84.46 Y98.291 E.01442
G1 X84.06 Y98.009 E.01443
G1 X83.303 Y97.389 E.02884
G1 X82.949 Y97.051 E.01443
G1 X82.611 Y96.696 E.01443
G1 X81.991 Y95.94 E.02884
G1 X81.709 Y95.54 E.01443
G1 X81.447 Y95.127 E.01442
G1 X80.986 Y94.263 E.02884
G1 X80.787 Y93.816 E.01443
G1 X80.611 Y93.359 E.01442
G1 X80.327 Y92.423 E.02884
G1 X80.22 Y91.946 E.01443
G1 X80.136 Y91.463 E.01443
G1 X80.04 Y90.489 E.02884
G1 X80.028 Y90 E.01442
G1 X80.04 Y89.511 E.01442
G1 X80.076 Y89.023 E.01443
G1 X80.136 Y88.537 E.01443
G1 X80.22 Y88.055 E.01442
G1 X80.327 Y87.577 E.01443
G1 X80.611 Y86.641 E.02884
G1 X80.787 Y86.184 E.01443
G1 X80.986 Y85.737 E.01442
G1 X81.447 Y84.873 E.02885
G1 X81.709 Y84.46 E.01441
G1 X81.991 Y84.06 E.01443
G1 X82.611 Y83.303 E.02885
G1 X82.949 Y82.949 E.01442
G1 X83.304 Y82.611 E.01444
G1 X84.06 Y81.991 E.02884
G1 X84.46 Y81.709 E.01443
G1 X84.873 Y81.447 E.01442
G1 X85.737 Y80.986 E.02884
G1 X86.184 Y80.787 E.01442
G1 X86.641 Y80.611 E.01443
G1 X87.577 Y80.327 E.02883
G1 X88.055 Y80.22 E.01443
G1 X88.537 Y80.136 E.01443
G1 X89.51 Y80.04 E.02881
G1 X90.132 Y80.032 E.01833
M106 S124.95
M106 S127.5
G1 X90.98 Y80.077 E.02503
M106 S124.95
M106 S127.5
G1 X91.945 Y80.22 E.02878
G1 X92.382 Y80.318 E.01318
M106 S124.95
; WIPE_START
G1 F2760
M204 S6000
G1 X92.894 Y80.458 E-.20191
G1 X93.339 Y80.605 E-.17809
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z6.1
G1 X0 Y90 F18000 ; move to safe pos
M73 P77 R3
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z6.1 F4000
            G39.3 S1
            G0 Z6.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.119 Y80.799 F42000
G1 Z5.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.384943
G1 F1589
M204 S6000
G1 X89.12 Y80.838 E.02673
G1 X88.209 Y80.972 E.02461
G1 X87.332 Y81.191 E.02415
G1 X86.482 Y81.495 E.02414
G1 X85.665 Y81.88 E.02415
G1 X84.89 Y82.344 E.02414
G1 X84.164 Y82.882 E.02415
G1 X83.495 Y83.488 E.02414
G1 X82.888 Y84.157 E.02415
G1 X82.349 Y84.883 E.02415
G1 X81.885 Y85.657 E.02414
G1 X81.498 Y86.474 E.02415
G1 X81.193 Y87.324 E.02415
G1 X80.973 Y88.2 E.02414
G1 X80.84 Y89.093 E.02415
G1 X80.796 Y89.996 E.02415
G1 X80.84 Y90.898 E.02415
G1 X80.972 Y91.791 E.02415
G1 X81.191 Y92.668 E.02414
G1 X81.495 Y93.518 E.02415
G1 X81.88 Y94.335 E.02415
G1 X82.344 Y95.11 E.02414
G1 X82.882 Y95.836 E.02415
G1 X83.488 Y96.505 E.02414
G1 X84.158 Y97.112 E.02417
G1 X84.883 Y97.651 E.02414
G1 X85.657 Y98.115 E.02415
G1 X86.474 Y98.502 E.02415
G1 X87.324 Y98.807 E.02415
G1 X88.2 Y99.027 E.02414
G1 X89.093 Y99.16 E.02414
G1 X89.996 Y99.204 E.02415
G1 X90.898 Y99.16 E.02415
G1 X91.791 Y99.028 E.02415
G1 X92.668 Y98.809 E.02414
G1 X93.518 Y98.505 E.02415
G1 X94.335 Y98.12 E.02415
G1 X95.11 Y97.656 E.02414
G1 X95.836 Y97.118 E.02417
G1 X96.505 Y96.512 E.02413
G1 X97.112 Y95.843 E.02415
G1 X97.651 Y95.117 E.02416
G1 X98.115 Y94.343 E.02414
G1 X98.502 Y93.527 E.02414
G1 X98.807 Y92.676 E.02415
G1 X99.027 Y91.8 E.02414
G1 X99.16 Y90.907 E.02414
G1 X99.204 Y90.004 E.02415
G1 X99.16 Y89.098 E.02426
G1 X99.028 Y88.209 E.02404
G1 X98.809 Y87.332 E.02415
G1 X98.506 Y86.482 E.02414
G1 X98.12 Y85.665 E.02416
G1 X97.656 Y84.89 E.02415
G1 X97.118 Y84.164 E.02414
G1 X96.512 Y83.495 E.02414
G1 X95.843 Y82.888 E.02416
G1 X95.117 Y82.349 E.02415
G1 X94.343 Y81.885 E.02414
G1 X93.527 Y81.498 E.02415
G1 X92.676 Y81.193 E.02415
G1 X91.8 Y80.973 E.02415
G1 X90.908 Y80.84 E.02411
G1 X90.179 Y80.802 E.01952
; WIPE_START
G1 F11402.749
G1 X90.908 Y80.84 E-.27744
G1 X91.175 Y80.88 E-.10256
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.1 I-1.139 J-.428 P1  F42000
G1 X85.01 Y97.279 Z6.1
G1 Z5.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1589
M204 S6000
G1 X84.398 Y96.826 E.02423
G1 X83.756 Y96.244 E.02758
G1 X96.244 Y83.756 E.5619
G1 X95.602 Y83.174 E.02757
G1 X94.906 Y82.658 E.02757
G1 X94.162 Y82.213 E.02757
G1 X93.379 Y81.842 E.02757
G1 X92.563 Y81.55 E.02758
G1 X91.723 Y81.34 E.02757
G1 X91.08 Y81.244 E.02068
G1 X81.244 Y91.08 E.44257
G1 X81.213 Y90.866 E.00689
G1 X81.17 Y90 E.02757
G1 X81.213 Y89.134 E.02757
G1 X81.244 Y88.92 E.00689
G1 X91.08 Y98.756 E.44257
G1 X90.866 Y98.787 E.00689
G1 X90 Y98.83 E.02757
G1 X89.134 Y98.787 E.02758
G1 X88.92 Y98.756 E.00689
G1 X98.756 Y88.92 E.44257
G1 X98.787 Y89.134 E.00689
G1 X98.83 Y90 E.02758
G1 X98.787 Y90.866 E.02757
G1 X98.756 Y91.08 E.00689
G1 X88.92 Y81.244 E.44257
G1 X88.277 Y81.34 E.02068
G1 X87.437 Y81.55 E.02757
G1 X86.621 Y81.842 E.02757
G1 X85.837 Y82.213 E.02758
G1 X85.094 Y82.658 E.02756
G1 X84.398 Y83.174 E.02757
G1 X83.756 Y83.756 E.02758
G1 X96.244 Y96.244 E.5619
G1 X96.826 Y95.602 E.02757
G1 X97.279 Y94.99 E.02424
; CHANGE_LAYER
; Z_HEIGHT: 5.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X96.826 Y95.602 E-.28949
G1 X96.666 Y95.778 E-.09051
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 30/43
; update layer progress
M73 L30
M991 S0 P29 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z6.1 I1.171 J-.331 P1  F42000
G1 X92.369 Y80.572 Z6.1
G1 Z5.9
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1621
M204 S6000
G1 X92.825 Y80.686 E.01497
G1 X93.725 Y81.008 E.03039
G1 X94.588 Y81.416 E.0304
G1 X95.407 Y81.907 E.03038
G1 X96.175 Y82.476 E.03039
G1 X96.882 Y83.118 E.03039
G1 X97.524 Y83.825 E.03039
G1 X98.093 Y84.593 E.0304
G1 X98.584 Y85.412 E.03039
G1 X98.992 Y86.275 E.03039
G1 X99.314 Y87.175 E.03039
G1 X99.546 Y88.101 E.03039
G1 X99.686 Y89.046 E.0304
G1 X99.733 Y90 E.03038
G1 X99.686 Y90.954 E.03039
G1 X99.546 Y91.899 E.0304
G1 X99.314 Y92.825 E.03038
G1 X98.992 Y93.725 E.03039
G1 X98.584 Y94.588 E.03039
G1 X98.093 Y95.408 E.0304
G1 X97.524 Y96.175 E.03039
G1 X96.882 Y96.882 E.03038
G1 X96.175 Y97.524 E.03039
G1 X95.407 Y98.093 E.0304
G1 X94.588 Y98.584 E.03039
G1 X93.725 Y98.992 E.03039
G1 X92.825 Y99.314 E.03039
G1 X91.899 Y99.546 E.03038
G1 X90.954 Y99.686 E.03039
G1 X90 Y99.733 E.03039
G1 X89.046 Y99.686 E.0304
G1 X88.101 Y99.546 E.03038
G1 X87.175 Y99.314 E.03039
G1 X86.275 Y98.992 E.03039
G1 X85.412 Y98.584 E.03039
G1 X84.593 Y98.093 E.03039
G1 X83.825 Y97.524 E.0304
G1 X83.118 Y96.882 E.03039
G1 X82.476 Y96.175 E.03039
G1 X81.907 Y95.407 E.0304
G1 X81.416 Y94.588 E.03038
G1 X81.008 Y93.725 E.0304
G1 X80.686 Y92.825 E.03039
G1 X80.454 Y91.899 E.03038
G1 X80.314 Y90.954 E.0304
G1 X80.267 Y90 E.03039
G1 X80.314 Y89.046 E.0304
G1 X80.454 Y88.101 E.03039
G1 X80.686 Y87.175 E.03038
G1 X81.008 Y86.275 E.03039
G1 X81.416 Y85.412 E.03039
G1 X81.907 Y84.593 E.0304
G1 X82.477 Y83.825 E.0304
G1 X83.118 Y83.118 E.03038
G1 X83.825 Y82.476 E.03038
G1 X84.593 Y81.907 E.0304
G1 X85.412 Y81.416 E.03039
G1 X86.275 Y81.008 E.03039
G1 X87.175 Y80.686 E.03039
G1 X88.101 Y80.454 E.03038
G1 X89.046 Y80.314 E.0304
G1 X89.516 Y80.279 E.01499
G1 X90.409 Y80.277 E.02843
G1 X90.956 Y80.314 E.01743
G1 X91.899 Y80.454 E.03033
G1 X92.311 Y80.557 E.0135
; COOLING_NODE: 5
M204 S250
G1 X92.473 Y80.182 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1398
M204 S5000
G1 X92.939 Y80.311 E.01426
G1 X93.411 Y80.467 E.01465
G1 X93.875 Y80.645 E.01464
G1 X94.329 Y80.847 E.01466
G1 X94.773 Y81.07 E.01465
G1 X95.205 Y81.315 E.01464
G1 X95.625 Y81.581 E.01465
G1 X96.032 Y81.867 E.01465
G1 X96.423 Y82.173 E.01464
G1 X96.8 Y82.498 E.01464
G1 X97.16 Y82.84 E.01465
G1 X97.502 Y83.2 E.01465
G1 X97.827 Y83.577 E.01465
G1 X98.133 Y83.968 E.01464
G1 X98.419 Y84.375 E.01465
G1 X98.685 Y84.794 E.01464
G1 X98.93 Y85.227 E.01465
G1 X99.153 Y85.671 E.01464
G1 X99.355 Y86.125 E.01465
G1 X99.533 Y86.589 E.01465
M73 P78 R3
G1 X99.689 Y87.061 E.01465
G1 X99.822 Y87.54 E.01465
G1 X99.931 Y88.025 E.01465
G1 X100.016 Y88.514 E.01464
G1 X100.076 Y89.008 E.01465
G1 X100.113 Y89.503 E.01464
G1 X100.125 Y90 E.01465
G1 X100.113 Y90.497 E.01464
G1 X100.076 Y90.993 E.01465
G1 X100.016 Y91.486 E.01464
G1 X99.931 Y91.975 E.01465
G1 X99.822 Y92.46 E.01464
G1 X99.689 Y92.939 E.01465
G1 X99.533 Y93.411 E.01465
G1 X99.355 Y93.875 E.01465
G1 X99.153 Y94.329 E.01465
G1 X98.93 Y94.773 E.01464
G1 X98.685 Y95.206 E.01465
G1 X98.419 Y95.625 E.01464
G1 X98.133 Y96.032 E.01464
G1 X97.827 Y96.423 E.01465
G1 X97.502 Y96.8 E.01464
G1 X97.16 Y97.16 E.01465
G1 X96.8 Y97.502 E.01465
G1 X96.423 Y97.827 E.01464
G1 X96.032 Y98.133 E.01465
G1 X95.625 Y98.419 E.01465
G1 X95.206 Y98.685 E.01464
G1 X94.773 Y98.93 E.01465
G1 X94.329 Y99.153 E.01464
G1 X93.875 Y99.355 E.01466
G1 X93.411 Y99.533 E.01464
G1 X92.939 Y99.689 E.01465
G1 X92.46 Y99.822 E.01463
G1 X91.975 Y99.931 E.01465
G1 X91.486 Y100.016 E.01465
G1 X90.993 Y100.076 E.01464
G1 X90.497 Y100.113 E.01465
G1 X90 Y100.125 E.01464
G1 X89.503 Y100.113 E.01464
G1 X89.007 Y100.076 E.01465
G1 X88.514 Y100.016 E.01465
G1 X88.025 Y99.931 E.01464
G1 X87.54 Y99.822 E.01465
G1 X87.061 Y99.689 E.01464
G1 X86.589 Y99.533 E.01465
G1 X86.125 Y99.354 E.01465
G1 X85.671 Y99.153 E.01464
G1 X85.227 Y98.93 E.01464
G1 X84.794 Y98.685 E.01465
G1 X84.375 Y98.419 E.01464
G1 X83.968 Y98.133 E.01465
G1 X83.577 Y97.827 E.01464
G1 X83.2 Y97.502 E.01465
G1 X82.84 Y97.16 E.01465
G1 X82.498 Y96.8 E.01465
G1 X82.173 Y96.423 E.01464
G1 X81.867 Y96.032 E.01465
G1 X81.581 Y95.625 E.01465
G1 X81.315 Y95.205 E.01465
G1 X81.07 Y94.773 E.01464
G1 X80.847 Y94.329 E.01465
G1 X80.645 Y93.875 E.01465
G1 X80.467 Y93.411 E.01464
G1 X80.311 Y92.939 E.01466
G1 X80.178 Y92.46 E.01463
G1 X80.069 Y91.975 E.01465
G1 X79.984 Y91.486 E.01465
G1 X79.924 Y90.993 E.01464
G1 X79.887 Y90.497 E.01465
G1 X79.875 Y90 E.01464
G1 X79.887 Y89.503 E.01464
G1 X79.924 Y89.007 E.01465
G1 X79.984 Y88.514 E.01464
G1 X80.069 Y88.025 E.01465
G1 X80.178 Y87.54 E.01464
G1 X80.311 Y87.061 E.01465
G1 X80.467 Y86.589 E.01465
G1 X80.645 Y86.125 E.01464
G1 X80.847 Y85.671 E.01465
G1 X81.07 Y85.227 E.01465
G1 X81.315 Y84.794 E.01465
G1 X81.581 Y84.375 E.01464
G1 X81.867 Y83.969 E.01464
G1 X82.173 Y83.576 E.01466
G1 X82.498 Y83.201 E.01463
G1 X82.84 Y82.84 E.01466
G1 X83.2 Y82.498 E.01465
G1 X83.577 Y82.173 E.01464
G1 X83.968 Y81.867 E.01465
G1 X84.375 Y81.581 E.01465
G1 X84.795 Y81.315 E.01465
G1 X85.227 Y81.07 E.01464
G1 X85.671 Y80.847 E.01465
G1 X86.125 Y80.646 E.01465
G1 X86.589 Y80.467 E.01465
G1 X87.061 Y80.311 E.01465
G1 X87.54 Y80.178 E.01464
G1 X88.025 Y80.069 E.01465
G1 X88.514 Y79.984 E.01465
G1 X89.007 Y79.924 E.01464
G1 X89.501 Y79.887 E.01458
M106 S124.95
M106 S127.5
G1 X90.422 Y79.885 E.02715
G1 X90.993 Y79.924 E.01687
M106 S124.95
M106 S127.5
G1 X91.486 Y79.984 E.01463
G1 X91.975 Y80.069 E.01465
G1 X92.415 Y80.168 E.01327
M106 S124.95
; WIPE_START
G1 F2760
M204 S6000
G1 X92.939 Y80.311 E-.2066
G1 X93.372 Y80.454 E-.1734
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z6.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z6.3 F4000
            G39.3 S1
            G0 Z6.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.537 Y80.659 F42000
G1 Z5.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.38292
G1 F1621
M204 S6000
G1 X88.189 Y80.814 E.03604
G1 X87.286 Y81.039 E.02473
G1 X86.421 Y81.348 E.02442
G1 X85.59 Y81.741 E.02441
G1 X84.802 Y82.213 E.02443
G1 X84.064 Y82.76 E.02441
G1 X83.383 Y83.376 E.02442
G1 X82.765 Y84.057 E.02442
G1 X82.217 Y84.795 E.02442
G1 X81.745 Y85.583 E.02441
G1 X81.352 Y86.413 E.02441
G1 X81.042 Y87.278 E.02442
G1 X80.818 Y88.169 E.02441
G1 X80.683 Y89.078 E.02441
G1 X80.637 Y89.996 E.02443
G1 X80.682 Y90.914 E.02443
G1 X80.816 Y91.823 E.02442
G1 X81.039 Y92.714 E.02441
G1 X81.348 Y93.579 E.02441
G1 X81.741 Y94.41 E.02442
G1 X82.213 Y95.198 E.02442
G1 X82.76 Y95.936 E.02441
G1 X83.376 Y96.617 E.02442
G1 X84.057 Y97.235 E.02442
G1 X84.795 Y97.783 E.02443
G1 X85.583 Y98.255 E.02442
G1 X86.413 Y98.648 E.02441
G1 X87.278 Y98.958 E.02442
G1 X88.169 Y99.182 E.02442
G1 X89.078 Y99.317 E.0244
G1 X89.996 Y99.363 E.02443
G1 X90.914 Y99.318 E.02443
G1 X91.823 Y99.184 E.02442
G1 X92.714 Y98.961 E.02441
G1 X93.579 Y98.652 E.02442
G1 X94.41 Y98.259 E.02441
G1 X95.198 Y97.787 E.02442
G1 X95.936 Y97.24 E.02441
G1 X96.617 Y96.624 E.02442
G1 X97.235 Y95.943 E.02443
G1 X97.783 Y95.205 E.02441
G1 X98.255 Y94.417 E.02442
G1 X98.649 Y93.587 E.02442
G1 X98.958 Y92.722 E.02442
G1 X99.182 Y91.831 E.02441
G1 X99.317 Y90.922 E.02441
G1 X99.363 Y90.004 E.02443
G1 X99.318 Y89.083 E.02452
G1 X99.184 Y88.178 E.02431
G1 X98.961 Y87.286 E.02442
G1 X98.652 Y86.421 E.02441
G1 X98.259 Y85.59 E.02443
G1 X97.787 Y84.802 E.02441
G1 X97.24 Y84.064 E.02442
G1 X96.624 Y83.383 E.02441
G1 X95.943 Y82.765 E.02442
G1 X95.205 Y82.218 E.02441
G1 X94.417 Y81.745 E.02442
G1 X93.587 Y81.352 E.02442
G1 X92.722 Y81.042 E.02442
G1 X91.831 Y80.818 E.02442
G1 X90.924 Y80.683 E.02436
G1 X90.399 Y80.648 E.01398
G1 X89.597 Y80.659 E.02133
; WIPE_START
G1 F11470.588
G1 X90.399 Y80.648 E-.30504
G1 X90.596 Y80.661 E-.07496
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.3 I-1.101 J.518 P1  F42000
G1 X97.396 Y95.106 Z6.3
G1 Z5.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1621
M204 S6000
G1 X96.952 Y95.705 E.02372
G1 X96.359 Y96.359 E.02809
G1 X83.641 Y83.641 E.5723
G1 X84.295 Y83.048 E.02808
G1 X85.004 Y82.522 E.02808
G1 X85.761 Y82.069 E.02809
G1 X86.558 Y81.691 E.02807
G1 X87.389 Y81.394 E.02808
G1 X88.246 Y81.179 E.02808
G1 X88.777 Y81.101 E.01708
G1 X98.899 Y91.223 E.45549
G1 X98.95 Y90.882 E.01099
G1 X98.993 Y90 E.02809
G1 X98.95 Y89.119 E.02808
G1 X98.899 Y88.777 E.011
G1 X88.777 Y98.899 E.45549
G1 X89.118 Y98.95 E.01099
G1 X90 Y98.993 E.02809
G1 X90.882 Y98.95 E.02809
G1 X91.223 Y98.899 E.01099
G1 X81.101 Y88.777 E.45549
G1 X81.05 Y89.118 E.01099
G1 X81.007 Y90 E.02809
G1 X81.05 Y90.882 E.02809
G1 X81.101 Y91.223 E.01099
G1 X91.223 Y81.101 E.45549
G1 X91.754 Y81.179 E.01708
G1 X92.61 Y81.394 E.02808
G1 X93.442 Y81.691 E.02809
G1 X94.239 Y82.069 E.02808
G1 X94.997 Y82.523 E.02809
G1 X95.705 Y83.048 E.02807
G1 X96.359 Y83.641 E.02808
G1 X83.641 Y96.359 E.5723
G1 X83.048 Y95.705 E.02808
G1 X82.604 Y95.106 E.02373
; CHANGE_LAYER
; Z_HEIGHT: 6.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X83.048 Y95.705 E-.28335
G1 X83.219 Y95.894 E-.09665
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 31/43
; update layer progress
M73 L31
M991 S0 P30 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z6.3 I1.046 J.621 P1  F42000
G1 X92.402 Y80.431 Z6.3
G1 Z6.1
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1652
M204 S6000
G1 X92.867 Y80.548 E.01527
G1 X93.78 Y80.875 E.03084
G1 X94.656 Y81.289 E.03084
G1 X95.487 Y81.787 E.03084
G1 X96.266 Y82.365 E.03084
G1 X96.984 Y83.016 E.03084
G1 X97.635 Y83.734 E.03085
G1 X98.213 Y84.512 E.03084
G1 X98.711 Y85.344 E.03084
G1 X99.126 Y86.22 E.03084
G1 X99.452 Y87.133 E.03084
G1 X99.688 Y88.073 E.03085
G1 X99.83 Y89.032 E.03083
G1 X99.877 Y90 E.03085
G1 X99.83 Y90.968 E.03085
G1 X99.688 Y91.927 E.03084
G1 X99.452 Y92.867 E.03083
G1 X99.126 Y93.78 E.03084
G1 X98.711 Y94.656 E.03084
G1 X98.213 Y95.488 E.03085
G1 X97.635 Y96.266 E.03083
G1 X96.984 Y96.984 E.03084
G1 X96.266 Y97.635 E.03085
G1 X95.488 Y98.213 E.03083
G1 X94.656 Y98.711 E.03084
G1 X93.78 Y99.126 E.03085
G1 X92.867 Y99.452 E.03084
G1 X91.927 Y99.688 E.03083
G1 X90.968 Y99.83 E.03084
G1 X90 Y99.877 E.03085
G1 X89.032 Y99.83 E.03085
G1 X88.073 Y99.688 E.03083
G1 X87.133 Y99.452 E.03085
G1 X86.22 Y99.126 E.03084
G1 X85.344 Y98.711 E.03084
G1 X84.512 Y98.213 E.03085
G1 X83.734 Y97.635 E.03084
G1 X83.016 Y96.984 E.03084
G1 X82.365 Y96.266 E.03084
G1 X81.787 Y95.487 E.03085
G1 X81.289 Y94.656 E.03084
G1 X80.874 Y93.78 E.03084
G1 X80.548 Y92.867 E.03085
G1 X80.312 Y91.927 E.03083
G1 X80.17 Y90.968 E.03084
G1 X80.123 Y90 E.03085
G1 X80.17 Y89.032 E.03084
G1 X80.312 Y88.073 E.03084
G1 X80.548 Y87.133 E.03085
G1 X80.874 Y86.22 E.03084
G1 X81.289 Y85.344 E.03084
G1 X81.787 Y84.512 E.03084
G1 X82.365 Y83.734 E.03084
G1 X83.016 Y83.016 E.03084
M73 P79 R3
G1 X83.734 Y82.365 E.03084
G1 X84.513 Y81.787 E.03085
G1 X85.344 Y81.289 E.03085
G1 X86.22 Y80.875 E.03083
G1 X87.133 Y80.548 E.03084
G1 X88.073 Y80.312 E.03084
G1 X89.032 Y80.17 E.03083
G1 X89.511 Y80.135 E.01531
G1 X90.195 Y80.128 E.02177
G1 X90.974 Y80.171 E.02479
G1 X91.927 Y80.312 E.03067
G1 X92.343 Y80.417 E.01366
; COOLING_NODE: 5
M204 S250
G1 X92.503 Y80.041 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1414
M204 S5000
G1 X92.981 Y80.173 E.0146
G1 X93.46 Y80.331 E.01486
G1 X93.93 Y80.512 E.01485
G1 X94.391 Y80.717 E.01486
G1 X94.841 Y80.943 E.01485
G1 X95.28 Y81.192 E.01485
G1 X95.705 Y81.461 E.01486
G1 X96.118 Y81.751 E.01485
G1 X96.515 Y82.061 E.01485
G1 X96.897 Y82.391 E.01487
G1 X97.262 Y82.738 E.01485
G1 X97.609 Y83.103 E.01486
G1 X97.939 Y83.485 E.01486
G1 X98.249 Y83.882 E.01485
G1 X98.539 Y84.295 E.01486
G1 X98.808 Y84.72 E.01485
G1 X99.057 Y85.159 E.01485
G1 X99.284 Y85.609 E.01486
G1 X99.488 Y86.07 E.01486
G1 X99.669 Y86.54 E.01486
G1 X99.827 Y87.019 E.01486
G1 X99.962 Y87.505 E.01486
G1 X100.072 Y87.997 E.01486
G1 X100.158 Y88.493 E.01485
G1 X100.22 Y88.993 E.01485
G1 X100.257 Y89.496 E.01486
G1 X100.27 Y90 E.01485
G1 X100.257 Y90.504 E.01485
G1 X100.22 Y91.007 E.01486
G1 X100.158 Y91.507 E.01485
G1 X100.072 Y92.004 E.01486
G1 X99.962 Y92.495 E.01485
G1 X99.827 Y92.981 E.01485
G1 X99.669 Y93.46 E.01486
G1 X99.488 Y93.93 E.01485
G1 X99.284 Y94.391 E.01486
G1 X99.057 Y94.841 E.01485
G1 X98.808 Y95.28 E.01486
G1 X98.539 Y95.706 E.01485
G1 X98.249 Y96.118 E.01485
G1 X97.939 Y96.515 E.01485
G1 X97.609 Y96.897 E.01487
G1 X97.262 Y97.262 E.01485
G1 X96.897 Y97.609 E.01485
G1 X96.515 Y97.939 E.01487
G1 X96.118 Y98.249 E.01485
G1 X95.706 Y98.539 E.01485
G1 X95.28 Y98.808 E.01486
G1 X94.841 Y99.057 E.01485
G1 X94.391 Y99.283 E.01485
G1 X93.93 Y99.488 E.01486
G1 X93.46 Y99.669 E.01485
G1 X92.981 Y99.827 E.01486
G1 X92.495 Y99.962 E.01484
G1 X92.004 Y100.072 E.01486
G1 X91.507 Y100.158 E.01486
G1 X91.007 Y100.22 E.01485
G1 X90.504 Y100.257 E.01486
G1 X90 Y100.27 E.01485
G1 X89.496 Y100.257 E.01485
G1 X88.993 Y100.22 E.01486
G1 X88.493 Y100.158 E.01486
G1 X87.997 Y100.072 E.01485
G1 X87.505 Y99.962 E.01486
G1 X87.019 Y99.827 E.01486
G1 X86.54 Y99.669 E.01486
G1 X86.07 Y99.488 E.01485
G1 X85.609 Y99.283 E.01486
G1 X85.159 Y99.057 E.01485
G1 X84.72 Y98.808 E.01486
G1 X84.295 Y98.539 E.01485
G1 X83.883 Y98.249 E.01485
G1 X83.485 Y97.938 E.01486
G1 X83.103 Y97.609 E.01486
G1 X82.738 Y97.262 E.01485
G1 X82.391 Y96.897 E.01485
G1 X82.061 Y96.515 E.01486
G1 X81.751 Y96.118 E.01485
G1 X81.461 Y95.705 E.01486
G1 X81.192 Y95.28 E.01485
G1 X80.943 Y94.841 E.01486
G1 X80.716 Y94.391 E.01485
G1 X80.512 Y93.93 E.01486
G1 X80.331 Y93.46 E.01485
G1 X80.173 Y92.981 E.01487
G1 X80.038 Y92.495 E.01484
G1 X79.928 Y92.004 E.01486
G1 X79.842 Y91.507 E.01486
G1 X79.78 Y91.007 E.01485
G1 X79.743 Y90.504 E.01486
G1 X79.73 Y90 E.01485
G1 X79.743 Y89.496 E.01485
G1 X79.78 Y88.993 E.01486
G1 X79.842 Y88.493 E.01486
G1 X79.928 Y87.997 E.01485
G1 X80.038 Y87.505 E.01486
G1 X80.173 Y87.019 E.01486
G1 X80.331 Y86.54 E.01486
G1 X80.512 Y86.07 E.01485
G1 X80.717 Y85.609 E.01486
G1 X80.943 Y85.159 E.01485
G1 X81.192 Y84.72 E.01485
G1 X81.461 Y84.295 E.01485
G1 X81.751 Y83.882 E.01486
G1 X82.062 Y83.485 E.01486
G1 X82.391 Y83.104 E.01484
G1 X82.738 Y82.738 E.01486
G1 X83.103 Y82.391 E.01486
G1 X83.485 Y82.062 E.01485
G1 X83.882 Y81.752 E.01485
G1 X84.295 Y81.461 E.01486
G1 X84.72 Y81.192 E.01485
G1 X85.159 Y80.943 E.01486
G1 X85.609 Y80.717 E.01485
G1 X86.07 Y80.512 E.01486
G1 X86.54 Y80.331 E.01486
G1 X87.019 Y80.173 E.01486
G1 X87.505 Y80.038 E.01486
G1 X87.997 Y79.928 E.01486
G1 X88.493 Y79.842 E.01485
G1 X88.993 Y79.78 E.01486
G1 X89.495 Y79.743 E.01482
M106 S124.95
M106 S127.5
G1 X90.204 Y79.736 E.02091
G1 X91.008 Y79.78 E.02374
M106 S124.95
M106 S127.5
G1 X91.507 Y79.842 E.0148
G1 X92.003 Y79.928 E.01485
G1 X92.445 Y80.027 E.01334
M106 S124.95
; WIPE_START
G1 F2880
M204 S6000
G1 X92.981 Y80.173 E-.21108
G1 X93.403 Y80.312 E-.16892
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z6.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z6.5 F4000
            G39.3 S1
            G0 Z6.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.533 Y80.506 F42000
G1 Z6.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.38292
G1 F1652
M204 S6000
G1 X88.159 Y80.666 E.03676
G1 X87.242 Y80.894 E.0251
G1 X86.362 Y81.209 E.02483
G1 X85.519 Y81.608 E.0248
G1 X84.717 Y82.087 E.02481
G1 X83.967 Y82.643 E.02481
G1 X83.275 Y83.27 E.02482
G1 X82.648 Y83.961 E.02481
G1 X82.091 Y84.711 E.02482
G1 X81.611 Y85.512 E.0248
G1 X81.212 Y86.356 E.02482
G1 X80.897 Y87.235 E.02481
G1 X80.669 Y88.14 E.02481
G1 X80.532 Y89.063 E.0248
G1 X80.486 Y89.996 E.02483
G1 X80.531 Y90.929 E.02482
G1 X80.668 Y91.853 E.02481
G1 X80.894 Y92.758 E.0248
G1 X81.209 Y93.637 E.02481
G1 X81.608 Y94.482 E.02483
G1 X82.087 Y95.282 E.0248
G1 X82.643 Y96.033 E.02482
G1 X83.27 Y96.725 E.02481
G1 X83.961 Y97.352 E.02481
G1 X84.711 Y97.909 E.02482
G1 X85.512 Y98.389 E.02481
G1 X86.356 Y98.788 E.02481
G1 X87.235 Y99.103 E.02481
G1 X88.14 Y99.33 E.02481
G1 X89.063 Y99.468 E.02481
G1 X89.996 Y99.514 E.02482
G1 X90.929 Y99.469 E.02482
G1 X91.852 Y99.332 E.0248
G1 X92.758 Y99.106 E.02481
G1 X93.637 Y98.791 E.02481
G1 X94.482 Y98.392 E.02482
G1 X95.282 Y97.913 E.02481
G1 X96.033 Y97.357 E.02481
G1 X96.725 Y96.73 E.02482
G1 X97.352 Y96.039 E.02481
G1 X97.909 Y95.289 E.02482
G1 X98.389 Y94.488 E.02482
G1 X98.788 Y93.644 E.02481
G1 X99.103 Y92.765 E.02481
G1 X99.33 Y91.86 E.0248
G1 X99.468 Y90.937 E.02481
G1 X99.514 Y90.004 E.02482
G1 X99.468 Y89.067 E.02492
G1 X99.332 Y88.148 E.0247
G1 X99.106 Y87.242 E.0248
G1 X98.791 Y86.363 E.02482
G1 X98.392 Y85.518 E.02482
G1 X97.913 Y84.717 E.02482
G1 X97.357 Y83.967 E.02481
G1 X96.73 Y83.275 E.02481
G1 X96.039 Y82.648 E.02481
G1 X95.289 Y82.091 E.02482
G1 X94.488 Y81.611 E.02481
G1 X93.644 Y81.212 E.02481
G1 X92.765 Y80.897 E.02482
G1 X91.86 Y80.67 E.0248
G1 X90.942 Y80.533 E.02466
G1 X90.19 Y80.492 E.02004
G1 X89.593 Y80.505 E.01587
; WIPE_START
G1 F11470.588
G1 X90.19 Y80.492 E-.22687
G1 X90.592 Y80.514 E-.15313
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.5 I-1.066 J-.587 P1  F42000
G1 X82.491 Y95.219 Z6.5
G1 Z6.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1652
M204 S6000
G1 X82.926 Y95.806 E.02324
G1 X83.529 Y96.471 E.02857
G1 X96.471 Y83.529 E.58237
G1 X95.806 Y82.926 E.02857
G1 X95.084 Y82.391 E.02858
G1 X94.314 Y81.929 E.02858
G1 X93.502 Y81.545 E.02857
G1 X92.656 Y81.242 E.02858
G1 X91.785 Y81.024 E.02856
G1 X91.363 Y80.962 E.0136
G1 X80.962 Y91.362 E.46801
G1 X80.893 Y90.897 E.01496
G1 X80.848 Y90 E.02858
G1 X80.893 Y89.103 E.02859
G1 X80.962 Y88.638 E.01496
G1 X91.362 Y99.038 E.46801
G1 X90.897 Y99.107 E.01496
G1 X90 Y99.152 E.02858
G1 X89.103 Y99.107 E.02858
G1 X88.638 Y99.038 E.01497
G1 X99.038 Y88.638 E.46801
G1 X99.107 Y89.103 E.01498
G1 X99.152 Y90 E.02857
G1 X99.107 Y90.897 E.02859
G1 X99.038 Y91.362 E.01496
G1 X88.638 Y80.962 E.46801
G1 X88.214 Y81.024 E.01361
G1 X87.344 Y81.243 E.02856
G1 X86.498 Y81.545 E.02859
G1 X85.686 Y81.929 E.02856
G1 X84.916 Y82.391 E.02858
G1 X84.194 Y82.926 E.02858
G1 X83.529 Y83.529 E.02857
M73 P80 R3
G1 X96.471 Y96.471 E.58237
G1 X97.074 Y95.806 E.02857
G1 X97.509 Y95.219 E.02324
; CHANGE_LAYER
; Z_HEIGHT: 6.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X97.074 Y95.806 E-.27759
G1 X96.893 Y96.006 E-.10241
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 32/43
; update layer progress
M73 L32
M991 S0 P31 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z6.5 I1.171 J-.332 P1  F42000
G1 X92.435 Y80.279 Z6.5
G1 Z6.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1682
M204 S6000
G1 X92.909 Y80.411 E.01566
G1 X93.376 Y80.565 E.01564
G1 X93.835 Y80.742 E.01565
G1 X94.284 Y80.941 E.01564
G1 X94.724 Y81.162 E.01565
G1 X95.152 Y81.405 E.01565
G1 X95.567 Y81.668 E.01564
G1 X95.969 Y81.951 E.01565
G1 X96.357 Y82.254 E.01565
G1 X96.73 Y82.575 E.01564
G1 X97.086 Y82.914 E.01565
G1 X97.425 Y83.27 E.01565
G1 X97.746 Y83.643 E.01565
G1 X98.049 Y84.031 E.01565
G1 X98.332 Y84.433 E.01565
G1 X98.595 Y84.848 E.01564
G1 X98.838 Y85.276 E.01566
G1 X99.059 Y85.716 E.01565
G1 X99.258 Y86.165 E.01565
G1 X99.435 Y86.624 E.01565
G1 X99.589 Y87.091 E.01565
G1 X99.721 Y87.565 E.01565
G1 X99.828 Y88.045 E.01565
G1 X99.913 Y88.53 E.01564
G1 X99.973 Y89.017 E.01564
G1 X100.009 Y89.509 E.01567
G1 X100.021 Y90 E.01563
G1 X100.009 Y90.492 E.01566
G1 X99.973 Y90.982 E.01565
G1 X99.913 Y91.47 E.01565
G1 X99.828 Y91.955 E.01566
G1 X99.721 Y92.435 E.01564
G1 X99.589 Y92.909 E.01566
G1 X99.435 Y93.376 E.01565
G1 X99.258 Y93.835 E.01564
G1 X99.059 Y94.285 E.01566
G1 X98.838 Y94.724 E.01564
G1 X98.595 Y95.152 E.01566
G1 X98.332 Y95.567 E.01564
G1 X98.049 Y95.97 E.01566
G1 X97.746 Y96.357 E.01564
G1 X97.425 Y96.73 E.01566
G1 X97.086 Y97.086 E.01565
G1 X96.729 Y97.425 E.01566
G1 X96.357 Y97.746 E.01564
G1 X95.97 Y98.049 E.01565
G1 X95.567 Y98.332 E.01565
G1 X95.152 Y98.595 E.01565
G1 X94.724 Y98.838 E.01565
G1 X94.285 Y99.059 E.01565
G1 X93.835 Y99.258 E.01564
G1 X93.376 Y99.435 E.01566
G1 X92.909 Y99.59 E.01565
G1 X92.435 Y99.721 E.01564
G1 X91.955 Y99.828 E.01564
G1 X91.47 Y99.913 E.01566
G1 X90.983 Y99.973 E.01563
G1 X90.491 Y100.009 E.01567
G1 X90 Y100.021 E.01563
G1 X89.509 Y100.009 E.01564
G1 X89.017 Y99.973 E.01567
G1 X88.53 Y99.913 E.01563
G1 X88.045 Y99.828 E.01566
G1 X87.565 Y99.721 E.01565
G1 X87.091 Y99.589 E.01565
G1 X86.624 Y99.435 E.01564
G1 X86.165 Y99.258 E.01564
G1 X85.716 Y99.059 E.01566
G1 X85.276 Y98.838 E.01565
G1 X84.848 Y98.595 E.01566
G1 X84.433 Y98.332 E.01564
G1 X84.03 Y98.049 E.01565
G1 X83.643 Y97.746 E.01565
G1 X83.27 Y97.425 E.01565
G1 X82.914 Y97.086 E.01565
G1 X82.575 Y96.73 E.01565
G1 X82.254 Y96.357 E.01565
G1 X81.951 Y95.969 E.01565
G1 X81.668 Y95.567 E.01565
G1 X81.405 Y95.152 E.01564
G1 X81.162 Y94.724 E.01566
G1 X80.941 Y94.285 E.01564
G1 X80.742 Y93.835 E.01565
G1 X80.565 Y93.376 E.01565
G1 X80.41 Y92.909 E.01566
G1 X80.279 Y92.435 E.01564
G1 X80.172 Y91.955 E.01566
G1 X80.087 Y91.471 E.01564
G1 X80.027 Y90.982 E.01565
G1 X79.991 Y90.492 E.01566
G1 X79.979 Y90 E.01565
G1 X79.991 Y89.509 E.01563
G1 X80.027 Y89.017 E.01567
G1 X80.087 Y88.53 E.01564
G1 X80.172 Y88.045 E.01566
G1 X80.279 Y87.565 E.01564
G1 X80.411 Y87.091 E.01566
G1 X80.565 Y86.624 E.01564
G1 X80.742 Y86.165 E.01565
G1 X80.941 Y85.715 E.01566
G1 X81.162 Y85.276 E.01564
G1 X81.405 Y84.848 E.01565
G1 X81.668 Y84.433 E.01565
G1 X81.951 Y84.031 E.01565
G1 X82.254 Y83.643 E.01566
G1 X82.575 Y83.27 E.01565
G1 X82.914 Y82.914 E.01565
G1 X83.271 Y82.575 E.01567
G1 X83.643 Y82.254 E.01563
G1 X84.03 Y81.951 E.01565
G1 X84.433 Y81.668 E.01566
G1 X84.848 Y81.405 E.01565
G1 X85.276 Y81.162 E.01564
G1 X85.715 Y80.941 E.01565
G1 X86.165 Y80.742 E.01566
G1 X86.624 Y80.565 E.01563
G1 X87.091 Y80.411 E.01566
G1 X87.565 Y80.279 E.01565
G1 X88.045 Y80.172 E.01565
G1 X88.529 Y80.088 E.01564
G1 X89.018 Y80.027 E.01565
G1 X89.509 Y79.991 E.01567
G1 X90.48 Y79.991 E.03091
G1 X90.982 Y80.027 E.01603
G1 X91.47 Y80.087 E.01564
G1 X91.955 Y80.172 E.01565
G1 X92.376 Y80.266 E.01373
; COOLING_NODE: 5
M204 S250
G1 X92.534 Y79.9 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1427
M204 S5000
G1 X93.023 Y80.035 E.01495
G1 X93.508 Y80.196 E.01506
G1 X93.985 Y80.38 E.01507
G1 X94.452 Y80.587 E.01505
G1 X94.909 Y80.816 E.01506
G1 X95.353 Y81.068 E.01507
G1 X95.785 Y81.342 E.01506
G1 X96.203 Y81.636 E.01507
G1 X96.606 Y81.951 E.01507
G1 X96.993 Y82.284 E.01506
G1 X97.363 Y82.637 E.01507
G1 X97.716 Y83.007 E.01507
G1 X98.049 Y83.394 E.01506
G1 X98.364 Y83.797 E.01506
G1 X98.658 Y84.215 E.01506
G1 X98.932 Y84.646 E.01506
G1 X99.184 Y85.091 E.01507
G1 X99.413 Y85.548 E.01506
G1 X99.621 Y86.015 E.01507
G1 X99.804 Y86.492 E.01506
G1 X99.965 Y86.977 E.01507
G1 X100.101 Y87.47 E.01506
G1 X100.213 Y87.969 E.01507
G1 X100.3 Y88.472 E.01506
G1 X100.363 Y88.979 E.01506
G1 X100.401 Y89.489 E.01507
G1 X100.413 Y90 E.01505
G1 X100.401 Y90.511 E.01507
G1 X100.363 Y91.021 E.01506
G1 X100.3 Y91.528 E.01507
G1 X100.213 Y92.032 E.01507
G1 X100.101 Y92.53 E.01506
G1 X99.965 Y93.023 E.01506
G1 X99.804 Y93.508 E.01507
G1 X99.621 Y93.985 E.01506
G1 X99.413 Y94.452 E.01507
G1 X99.184 Y94.909 E.01505
G1 X98.932 Y95.353 E.01507
G1 X98.658 Y95.785 E.01506
G1 X98.364 Y96.203 E.01507
G1 X98.05 Y96.606 E.01506
G1 X97.716 Y96.993 E.01507
G1 X97.363 Y97.363 E.01506
G1 X96.993 Y97.716 E.01507
G1 X96.606 Y98.05 E.01506
G1 X96.203 Y98.364 E.01506
G1 X95.785 Y98.658 E.01507
G1 X95.353 Y98.932 E.01506
G1 X94.909 Y99.184 E.01507
G1 X94.452 Y99.413 E.01506
G1 X93.985 Y99.62 E.01506
G1 X93.508 Y99.804 E.01507
G1 X93.023 Y99.965 E.01507
G1 X92.53 Y100.101 E.01506
G1 X92.032 Y100.213 E.01506
G1 X91.528 Y100.3 E.01507
G1 X91.021 Y100.363 E.01506
G1 X90.511 Y100.401 E.01507
G1 X90 Y100.413 E.01505
G1 X89.489 Y100.401 E.01507
G1 X88.979 Y100.363 E.01507
G1 X88.472 Y100.3 E.01506
G1 X87.968 Y100.213 E.01507
G1 X87.47 Y100.101 E.01507
G1 X86.977 Y99.965 E.01506
G1 X86.492 Y99.804 E.01506
G1 X86.015 Y99.621 E.01505
G1 X85.548 Y99.413 E.01507
G1 X85.091 Y99.184 E.01506
G1 X84.646 Y98.932 E.01507
G1 X84.215 Y98.658 E.01506
G1 X83.797 Y98.364 E.01506
G1 X83.394 Y98.049 E.01507
G1 X83.007 Y97.716 E.01506
G1 X82.637 Y97.363 E.01507
G1 X82.284 Y96.993 E.01507
G1 X81.951 Y96.606 E.01506
G1 X81.636 Y96.203 E.01506
G1 X81.342 Y95.785 E.01507
G1 X81.068 Y95.354 E.01505
G1 X80.816 Y94.909 E.01507
G1 X80.587 Y94.452 E.01506
G1 X80.38 Y93.985 E.01506
G1 X80.196 Y93.508 E.01507
G1 X80.035 Y93.023 E.01507
G1 X79.899 Y92.53 E.01506
G1 X79.787 Y92.031 E.01506
G1 X79.7 Y91.528 E.01506
G1 X79.637 Y91.021 E.01507
G1 X79.599 Y90.511 E.01507
G1 X79.587 Y90 E.01506
G1 X79.599 Y89.489 E.01505
G1 X79.637 Y88.979 E.01508
G1 X79.7 Y88.472 E.01506
G1 X79.787 Y87.968 E.01506
G1 X79.899 Y87.47 E.01506
G1 X80.035 Y86.977 E.01507
G1 X80.196 Y86.492 E.01506
G1 X80.379 Y86.015 E.01506
G1 X80.587 Y85.548 E.01507
G1 X80.816 Y85.091 E.01506
G1 X81.068 Y84.647 E.01507
G1 X81.342 Y84.215 E.01506
G1 X81.636 Y83.797 E.01506
G1 X81.951 Y83.394 E.01507
G1 X82.284 Y83.007 E.01506
G1 X82.637 Y82.637 E.01506
G1 X83.007 Y82.284 E.01507
G1 X83.394 Y81.951 E.01506
G1 X83.797 Y81.636 E.01506
G1 X84.215 Y81.342 E.01507
G1 X84.647 Y81.068 E.01506
G1 X85.091 Y80.816 E.01506
G1 X85.548 Y80.587 E.01506
G1 X86.015 Y80.379 E.01508
G1 X86.492 Y80.196 E.01505
G1 X86.977 Y80.035 E.01507
G1 X87.47 Y79.899 E.01507
G1 X87.969 Y79.787 E.01506
G1 X88.472 Y79.7 E.01506
G1 X88.979 Y79.637 E.01507
G1 X89.489 Y79.599 E.01507
G1 X90 Y79.587 E.01506
G1 X90.499 Y79.599 E.01472
G1 X91.021 Y79.637 E.01541
G1 X91.528 Y79.7 E.01506
G1 X92.032 Y79.787 E.01506
G1 X92.475 Y79.887 E.01341
M106 S124.95
; WIPE_START
G1 F2880
M204 S6000
G1 X93.023 Y80.035 E-.21558
G1 X93.434 Y80.171 E-.16442
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z6.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z6.7 F4000
            G39.3 S1
            G0 Z6.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X88.584 Y80.453 F42000
G1 Z6.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.38292
G1 F1682
M204 S6000
G1 X87.653 Y80.632 E.02517
G1 X86.746 Y80.907 E.02519
G1 X85.871 Y81.27 E.02517
M73 P81 R3
G1 X85.035 Y81.717 E.02519
G1 X84.247 Y82.243 E.02518
G1 X83.515 Y82.844 E.02517
G1 X82.844 Y83.515 E.0252
G1 X82.243 Y84.247 E.02519
G1 X81.717 Y85.035 E.02517
G1 X81.27 Y85.871 E.02519
G1 X80.908 Y86.746 E.02518
G1 X80.632 Y87.654 E.0252
G1 X80.447 Y88.583 E.02519
G1 X80.354 Y89.526 E.02519
G1 X80.354 Y90.474 E.02517
G1 X80.447 Y91.417 E.02519
G1 X80.632 Y92.347 E.02518
G1 X80.907 Y93.254 E.02519
G1 X81.27 Y94.129 E.02518
G1 X81.717 Y94.965 E.0252
G1 X82.243 Y95.753 E.02517
G1 X82.844 Y96.485 E.02518
G1 X83.515 Y97.155 E.02519
G1 X84.247 Y97.757 E.02519
G1 X85.035 Y98.283 E.02517
G1 X85.871 Y98.73 E.02521
G1 X86.746 Y99.093 E.02517
G1 X87.654 Y99.368 E.02519
G1 X88.583 Y99.553 E.02519
G1 X89.527 Y99.646 E.02519
G1 X90.473 Y99.646 E.02516
G1 X91.417 Y99.553 E.02519
G1 X92.346 Y99.368 E.02519
G1 X93.254 Y99.093 E.02519
G1 X94.129 Y98.73 E.02517
G1 X94.965 Y98.283 E.0252
G1 X95.753 Y97.757 E.02517
G1 X96.485 Y97.156 E.02518
G1 X97.155 Y96.485 E.02519
G1 X97.757 Y95.753 E.02519
G1 X98.283 Y94.965 E.02517
G1 X98.73 Y94.129 E.0252
G1 X99.093 Y93.254 E.02518
G1 X99.368 Y92.346 E.0252
G1 X99.553 Y91.417 E.02518
G1 X99.646 Y90.474 E.02518
G1 X99.646 Y89.526 E.02518
G1 X99.553 Y88.583 E.0252
G1 X99.368 Y87.654 E.02518
G1 X99.093 Y86.746 E.02519
G1 X98.73 Y85.871 E.02517
G1 X98.283 Y85.035 E.02521
G1 X97.757 Y84.247 E.02516
G1 X97.156 Y83.515 E.02519
G1 X96.485 Y82.844 E.02519
G1 X95.753 Y82.243 E.02518
G1 X94.965 Y81.717 E.02517
G1 X94.129 Y81.27 E.0252
G1 X93.254 Y80.908 E.02516
G1 X92.346 Y80.632 E.0252
G1 X91.417 Y80.447 E.02518
G1 X90.465 Y80.35 E.02543
G1 X89.524 Y80.349 E.02501
G1 X88.643 Y80.446 E.02354
; WIPE_START
G1 F11470.588
G1 X89.524 Y80.349 E-.33665
G1 X89.638 Y80.349 E-.04335
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.7 I-1.074 J.572 P1  F42000
G1 X97.619 Y95.329 Z6.7
G1 Z6.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1682
M204 S6000
G1 X97.193 Y95.903 E.02276
G1 X96.58 Y96.58 E.02905
G1 X83.42 Y83.42 E.59213
G1 X84.097 Y82.807 E.02905
G1 X84.83 Y82.263 E.02906
G1 X85.614 Y81.794 E.02905
G1 X86.439 Y81.403 E.02905
G1 X87.299 Y81.096 E.02905
G1 X88.184 Y80.874 E.02904
G1 X88.503 Y80.827 E.01024
G1 X99.173 Y91.497 E.48015
G1 X99.26 Y90.912 E.01883
G1 X99.305 Y90 E.02906
G1 X99.26 Y89.088 E.02905
G1 X99.173 Y88.503 E.01883
G1 X88.503 Y99.173 E.48015
G1 X89.088 Y99.26 E.01882
G1 X90 Y99.305 E.02907
G1 X90.912 Y99.26 E.02906
G1 X91.497 Y99.173 E.01882
G1 X80.827 Y88.503 E.48015
G1 X80.74 Y89.088 E.01882
G1 X80.695 Y90 E.02906
G1 X80.74 Y90.912 E.02905
G1 X80.827 Y91.497 E.01883
G1 X91.497 Y80.827 E.48015
G1 X91.816 Y80.874 E.01023
G1 X92.701 Y81.096 E.02905
G1 X93.561 Y81.403 E.02905
G1 X94.386 Y81.794 E.02906
G1 X95.17 Y82.263 E.02905
G1 X95.903 Y82.807 E.02905
G1 X96.58 Y83.42 E.02905
G1 X83.42 Y96.58 E.59213
G1 X82.807 Y95.903 E.02906
G1 X82.381 Y95.329 E.02275
; CHANGE_LAYER
; Z_HEIGHT: 6.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X82.807 Y95.903 E-.27174
G1 X82.998 Y96.114 E-.10826
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 33/43
; update layer progress
M73 L33
M991 S0 P32 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z6.7 I1.047 J.621 P1  F42000
G1 X92.467 Y80.152 Z6.7
G1 Z6.5
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1711
M204 S6000
G1 X92.947 Y80.285 E.01585
G1 X93.42 Y80.442 E.01586
G1 X93.885 Y80.621 E.01585
G1 X94.341 Y80.823 E.01586
G1 X94.785 Y81.047 E.01585
G1 X95.219 Y81.293 E.01586
G1 X95.64 Y81.559 E.01586
G1 X96.047 Y81.846 E.01584
G1 X96.44 Y82.153 E.01587
G1 X96.818 Y82.478 E.01585
G1 X97.178 Y82.822 E.01585
G1 X97.522 Y83.182 E.01585
G1 X97.847 Y83.56 E.01586
G1 X98.154 Y83.953 E.01586
G1 X98.441 Y84.36 E.01585
G1 X98.707 Y84.781 E.01584
G1 X98.953 Y85.215 E.01586
G1 X99.177 Y85.659 E.01585
G1 X99.379 Y86.115 E.01587
G1 X99.558 Y86.58 E.01585
G1 X99.715 Y87.053 E.01586
G1 X99.848 Y87.533 E.01585
G1 X99.957 Y88.02 E.01586
G1 X100.042 Y88.51 E.01584
G1 X100.103 Y89.005 E.01585
G1 X100.14 Y89.502 E.01587
G1 X100.152 Y90 E.01585
G1 X100.14 Y90.498 E.01586
G1 X100.103 Y90.995 E.01584
G1 X100.042 Y91.49 E.01587
G1 X99.957 Y91.98 E.01584
G1 X99.848 Y92.466 E.01585
G1 X99.715 Y92.947 E.01587
G1 X99.558 Y93.42 E.01585
G1 X99.379 Y93.885 E.01585
G1 X99.177 Y94.341 E.01586
G1 X98.953 Y94.785 E.01584
G1 X98.707 Y95.219 E.01586
G1 X98.441 Y95.64 E.01585
G1 X98.154 Y96.047 E.01586
G1 X97.847 Y96.44 E.01585
G1 X97.522 Y96.818 E.01585
G1 X97.178 Y97.179 E.01586
G1 X96.817 Y97.522 E.01585
G1 X96.441 Y97.847 E.01584
G1 X96.047 Y98.154 E.01588
G1 X95.64 Y98.441 E.01584
G1 X95.219 Y98.707 E.01585
G1 X94.786 Y98.953 E.01585
G1 X94.34 Y99.177 E.01586
G1 X93.885 Y99.379 E.01586
G1 X93.42 Y99.558 E.01584
G1 X92.947 Y99.715 E.01587
G1 X92.467 Y99.848 E.01584
G1 X91.98 Y99.957 E.01586
G1 X91.49 Y100.042 E.01585
G1 X90.995 Y100.103 E.01585
G1 X90.498 Y100.14 E.01587
G1 X90 Y100.152 E.01584
G1 X89.502 Y100.14 E.01584
G1 X89.005 Y100.103 E.01587
G1 X88.511 Y100.042 E.01584
G1 X88.019 Y99.957 E.01586
G1 X87.533 Y99.848 E.01585
G1 X87.053 Y99.715 E.01586
G1 X86.58 Y99.558 E.01585
G1 X86.115 Y99.379 E.01585
G1 X85.659 Y99.177 E.01586
G1 X85.214 Y98.953 E.01586
G1 X84.781 Y98.708 E.01585
G1 X84.36 Y98.441 E.01585
G1 X83.952 Y98.154 E.01587
G1 X83.56 Y97.848 E.01585
G1 X83.183 Y97.522 E.01585
G1 X82.822 Y97.178 E.01586
G1 X82.478 Y96.818 E.01585
G1 X82.153 Y96.44 E.01586
G1 X81.846 Y96.047 E.01585
G1 X81.559 Y95.64 E.01586
G1 X81.292 Y95.219 E.01586
G1 X81.047 Y94.786 E.01584
G1 X80.823 Y94.341 E.01585
G1 X80.621 Y93.885 E.01586
G1 X80.442 Y93.42 E.01585
G1 X80.285 Y92.947 E.01586
G1 X80.152 Y92.467 E.01584
G1 X80.043 Y91.981 E.01585
G1 X79.958 Y91.489 E.01586
G1 X79.897 Y90.995 E.01584
G1 X79.86 Y90.498 E.01587
G1 X79.848 Y90 E.01584
G1 X79.86 Y89.502 E.01586
G1 X79.897 Y89.005 E.01587
G1 X79.958 Y88.511 E.01584
G1 X80.043 Y88.019 E.01587
G1 X80.152 Y87.533 E.01585
G1 X80.285 Y87.053 E.01586
G1 X80.442 Y86.58 E.01585
G1 X80.621 Y86.115 E.01586
G1 X80.823 Y85.659 E.01586
G1 X81.047 Y85.215 E.01585
G1 X81.292 Y84.781 E.01586
G1 X81.559 Y84.36 E.01587
G1 X81.846 Y83.953 E.01583
G1 X82.153 Y83.56 E.01586
G1 X82.478 Y83.183 E.01585
G1 X82.822 Y82.821 E.01586
G1 X83.182 Y82.478 E.01584
G1 X83.56 Y82.152 E.01587
G1 X83.953 Y81.846 E.01585
G1 X84.36 Y81.559 E.01586
G1 X84.781 Y81.293 E.01585
G1 X85.214 Y81.047 E.01586
G1 X85.66 Y80.823 E.01586
G1 X86.115 Y80.621 E.01585
G1 X86.58 Y80.442 E.01585
G1 X87.053 Y80.285 E.01586
G1 X87.533 Y80.152 E.01585
G1 X88.019 Y80.043 E.01585
G1 X88.511 Y79.958 E.01587
M73 P82 R3
G1 X89.005 Y79.897 E.01584
G1 X89.497 Y79.861 E.01571
G1 X90.257 Y79.855 E.02417
G1 X91 Y79.898 E.0237
G1 X91.489 Y79.958 E.01569
G1 X91.981 Y80.043 E.01587
G1 X92.408 Y80.139 E.01394
; COOLING_NODE: 5
M204 S250
G1 X92.562 Y79.772 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1440
M204 S5000
G1 X93.061 Y79.91 E.01525
G1 X93.552 Y80.072 E.01525
G1 X94.035 Y80.259 E.01526
G1 X94.508 Y80.468 E.01525
G1 X94.97 Y80.701 E.01525
G1 X95.421 Y80.956 E.01525
G1 X95.858 Y81.233 E.01526
G1 X96.281 Y81.531 E.01524
G1 X96.689 Y81.849 E.01527
G1 X97.081 Y82.187 E.01525
G1 X97.456 Y82.544 E.01525
G1 X97.813 Y82.919 E.01525
G1 X98.151 Y83.311 E.01526
G1 X98.469 Y83.719 E.01525
G1 X98.767 Y84.142 E.01525
G1 X99.044 Y84.579 E.01525
G1 X99.299 Y85.03 E.01526
G1 X99.532 Y85.492 E.01524
G1 X99.741 Y85.965 E.01526
G1 X99.928 Y86.448 E.01525
G1 X100.09 Y86.939 E.01525
G1 X100.228 Y87.438 E.01525
G1 X100.341 Y87.943 E.01526
G1 X100.43 Y88.453 E.01524
G1 X100.493 Y88.966 E.01526
G1 X100.531 Y89.483 E.01526
G1 X100.544 Y90 E.01525
G1 X100.531 Y90.517 E.01526
G1 X100.493 Y91.033 E.01525
G1 X100.43 Y91.547 E.01526
G1 X100.341 Y92.057 E.01525
G1 X100.228 Y92.562 E.01525
G1 X100.09 Y93.061 E.01526
G1 X99.928 Y93.552 E.01525
G1 X99.741 Y94.035 E.01525
G1 X99.532 Y94.508 E.01526
G1 X99.299 Y94.97 E.01524
G1 X99.044 Y95.421 E.01526
G1 X98.767 Y95.858 E.01525
G1 X98.469 Y96.281 E.01526
G1 X98.151 Y96.689 E.01525
G1 X97.813 Y97.081 E.01525
G1 X97.456 Y97.456 E.01526
G1 X97.081 Y97.813 E.01525
G1 X96.689 Y98.15 E.01524
G1 X96.281 Y98.469 E.01527
G1 X95.858 Y98.767 E.01524
G1 X95.421 Y99.044 E.01526
G1 X94.97 Y99.299 E.01525
G1 X94.508 Y99.532 E.01526
G1 X94.035 Y99.741 E.01526
G1 X93.552 Y99.928 E.01525
G1 X93.061 Y100.09 E.01526
G1 X92.562 Y100.228 E.01524
G1 X92.057 Y100.341 E.01526
G1 X91.547 Y100.43 E.01525
G1 X91.034 Y100.493 E.01525
G1 X90.517 Y100.531 E.01526
G1 X90 Y100.544 E.01525
G1 X89.483 Y100.531 E.01525
G1 X88.966 Y100.493 E.01526
G1 X88.453 Y100.43 E.01525
G1 X87.943 Y100.341 E.01526
G1 X87.438 Y100.228 E.01525
G1 X86.939 Y100.09 E.01525
G1 X86.448 Y99.928 E.01525
G1 X85.965 Y99.741 E.01525
G1 X85.492 Y99.532 E.01526
G1 X85.03 Y99.299 E.01525
G1 X84.579 Y99.044 E.01525
G1 X84.142 Y98.767 E.01525
G1 X83.719 Y98.469 E.01526
G1 X83.311 Y98.151 E.01525
G1 X82.919 Y97.813 E.01525
G1 X82.544 Y97.456 E.01526
G1 X82.187 Y97.081 E.01525
G1 X81.849 Y96.689 E.01525
G1 X81.531 Y96.281 E.01525
G1 X81.233 Y95.858 E.01526
G1 X80.956 Y95.421 E.01525
G1 X80.701 Y94.97 E.01525
G1 X80.468 Y94.508 E.01525
G1 X80.259 Y94.035 E.01526
G1 X80.072 Y93.552 E.01525
G1 X79.91 Y93.061 E.01526
G1 X79.772 Y92.562 E.01524
G1 X79.659 Y92.057 E.01526
G1 X79.57 Y91.547 E.01526
G1 X79.507 Y91.034 E.01525
G1 X79.469 Y90.517 E.01526
M73 P82 R2
G1 X79.456 Y90 E.01525
G1 X79.469 Y89.483 E.01525
G1 X79.507 Y88.966 E.01526
G1 X79.57 Y88.453 E.01525
G1 X79.659 Y87.943 E.01526
G1 X79.772 Y87.438 E.01525
G1 X79.91 Y86.939 E.01525
G1 X80.072 Y86.448 E.01525
G1 X80.259 Y85.965 E.01525
G1 X80.468 Y85.492 E.01526
G1 X80.701 Y85.03 E.01525
G1 X80.956 Y84.579 E.01525
G1 X81.233 Y84.142 E.01526
G1 X81.531 Y83.719 E.01524
G1 X81.849 Y83.311 E.01526
G1 X82.187 Y82.919 E.01525
G1 X82.544 Y82.544 E.01526
G1 X82.919 Y82.188 E.01524
G1 X83.311 Y81.849 E.01526
G1 X83.719 Y81.531 E.01525
G1 X84.142 Y81.233 E.01526
G1 X84.579 Y80.956 E.01525
G1 X85.03 Y80.701 E.01525
G1 X85.492 Y80.468 E.01526
G1 X85.965 Y80.259 E.01525
G1 X86.448 Y80.072 E.01526
G1 X86.939 Y79.91 E.01526
G1 X87.438 Y79.772 E.01524
G1 X87.943 Y79.659 E.01526
G1 X88.453 Y79.57 E.01526
G1 X88.966 Y79.507 E.01525
G1 X89.481 Y79.469 E.01521
M106 S124.95
M106 S127.5
G1 X89.881 Y79.466 E.01179
M106 S124.95
M106 S127.5
G1 X90.266 Y79.463 E.01136
G1 X90.636 Y79.484 E.01091
M106 S124.95
M106 S127.5
G1 X91.035 Y79.507 E.01179
M106 S124.95
M106 S127.5
G1 X91.547 Y79.57 E.0152
G1 X92.057 Y79.659 E.01526
G1 X92.503 Y79.759 E.01348
M106 S124.95
; WIPE_START
G1 F3900
M204 S6000
G1 X93.061 Y79.91 E-.21942
G1 X93.462 Y80.043 E-.16058
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z6.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z6.9 F4000
            G39.3 S1
            G0 Z6.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X89.517 Y80.221 F42000
G1 Z6.5
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.38292
G1 F1711
M204 S6000
G1 X88.103 Y80.386 E.03783
G1 X87.158 Y80.621 E.02586
G1 X86.253 Y80.944 E.02556
G1 X85.383 Y81.355 E.02555
G1 X84.558 Y81.849 E.02556
G1 X83.785 Y82.422 E.02556
G1 X83.073 Y83.068 E.02555
G1 X82.426 Y83.78 E.02557
G1 X81.853 Y84.553 E.02556
G1 X81.358 Y85.377 E.02556
G1 X80.947 Y86.247 E.02556
G1 X80.623 Y87.152 E.02556
G1 X80.389 Y88.085 E.02556
G1 X80.247 Y89.036 E.02555
G1 X80.2 Y89.997 E.02556
G1 X80.247 Y90.958 E.02557
G1 X80.387 Y91.909 E.02555
G1 X80.621 Y92.842 E.02555
G1 X80.944 Y93.747 E.02556
G1 X81.355 Y94.617 E.02556
G1 X81.849 Y95.442 E.02556
G1 X82.422 Y96.215 E.02556
G1 X83.068 Y96.928 E.02557
G1 X83.78 Y97.574 E.02555
G1 X84.553 Y98.147 E.02556
G1 X85.378 Y98.642 E.02557
G1 X86.247 Y99.053 E.02555
G1 X87.152 Y99.377 E.02556
G1 X88.085 Y99.611 E.02556
G1 X89.036 Y99.753 E.02555
G1 X89.997 Y99.8 E.02557
G1 X90.958 Y99.753 E.02557
G1 X91.909 Y99.613 E.02556
G1 X92.842 Y99.379 E.02554
G1 X93.747 Y99.056 E.02556
G1 X94.617 Y98.645 E.02557
G1 X95.442 Y98.15 E.02556
G1 X96.215 Y97.578 E.02555
G1 X96.928 Y96.932 E.02556
G1 X97.574 Y96.22 E.02556
G1 X98.147 Y95.448 E.02556
G1 X98.642 Y94.622 E.02557
G1 X99.053 Y93.754 E.02555
G1 X99.377 Y92.848 E.02556
G1 X99.611 Y91.915 E.02555
G1 X99.753 Y90.964 E.02555
G1 X99.8 Y90 E.02566
G1 X99.753 Y89.042 E.02548
G1 X99.613 Y88.091 E.02555
G1 X99.379 Y87.158 E.02556
G1 X99.056 Y86.253 E.02556
G1 X98.645 Y85.383 E.02556
G1 X98.15 Y84.558 E.02556
G1 X97.578 Y83.785 E.02555
G1 X96.932 Y83.072 E.02557
G1 X96.22 Y82.426 E.02555
G1 X95.447 Y81.853 E.02556
G1 X94.623 Y81.358 E.02555
G1 X93.753 Y80.947 E.02556
G1 X92.848 Y80.623 E.02556
G1 X91.915 Y80.389 E.02555
G1 X90.969 Y80.248 E.02543
G1 X90.25 Y80.207 E.01914
G1 X89.577 Y80.22 E.01789
; WIPE_START
G1 F11470.588
G1 X90.25 Y80.207 E-.25582
G1 X90.576 Y80.225 E-.12418
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z6.9 I-1.068 J-.583 P1  F42000
G1 X82.278 Y95.431 Z6.9
G1 Z6.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1711
M204 S6000
G1 X82.696 Y95.994 E.0223
G1 X83.318 Y96.682 E.02951
G1 X96.682 Y83.318 E.60131
G1 X95.995 Y82.696 E.02951
G1 X95.25 Y82.143 E.02951
G1 X94.454 Y81.666 E.0295
G1 X93.616 Y81.27 E.02951
G1 X92.743 Y80.958 E.0295
G1 X91.843 Y80.732 E.02951
G1 X91.624 Y80.7 E.00705
G1 X80.7 Y91.624 E.49157
G1 X80.596 Y90.927 E.02244
G1 X80.551 Y90 E.02952
G1 X80.596 Y89.074 E.02951
G1 X80.7 Y88.376 E.02245
G1 X91.624 Y99.3 E.49157
G1 X90.926 Y99.404 E.02245
G1 X90 Y99.449 E.02951
G1 X89.073 Y99.404 E.02951
G1 X88.376 Y99.3 E.02245
G1 X99.3 Y88.376 E.49157
G1 X99.404 Y89.074 E.02245
G1 X99.449 Y90 E.02951
G1 X99.404 Y90.926 E.02952
G1 X99.3 Y91.624 E.02245
G1 X88.376 Y80.7 E.49157
G1 X88.157 Y80.732 E.00705
G1 X87.257 Y80.958 E.02951
G1 X86.384 Y81.27 E.0295
G1 X85.546 Y81.666 E.0295
G1 X84.75 Y82.143 E.02951
G1 X84.005 Y82.696 E.02951
G1 X83.318 Y83.318 E.0295
G1 X96.682 Y96.682 E.60131
G1 X97.304 Y95.995 E.02951
G1 X97.722 Y95.432 E.0223
; CHANGE_LAYER
; Z_HEIGHT: 6.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X97.304 Y95.995 E-.26636
G1 X97.104 Y96.216 E-.11364
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 34/43
; update layer progress
M73 L34
M991 S0 P33 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z6.9 I1.171 J-.333 P1  F42000
G1 X92.498 Y80.025 Z6.9
G1 Z6.7
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1740
M204 S6000
G1 X92.985 Y80.16 E.01608
G1 X93.464 Y80.318 E.01605
G1 X93.935 Y80.5 E.01606
G1 X94.396 Y80.705 E.01606
G1 X94.847 Y80.931 E.01605
G1 X95.287 Y81.18 E.01607
G1 X95.713 Y81.45 E.01604
G1 X96.126 Y81.741 E.01608
G1 X96.523 Y82.051 E.01604
G1 X96.906 Y82.381 E.01608
G1 X97.271 Y82.729 E.01605
G1 X97.619 Y83.095 E.01607
G1 X97.948 Y83.477 E.01605
G1 X98.259 Y83.875 E.01608
G1 X98.55 Y84.287 E.01604
G1 X98.82 Y84.714 E.01607
G1 X99.068 Y85.153 E.01604
G1 X99.295 Y85.604 E.01607
G1 X99.5 Y86.065 E.01606
G1 X99.682 Y86.536 E.01605
G1 X99.84 Y87.015 E.01607
G1 X99.975 Y87.501 E.01606
G1 X100.085 Y87.994 E.01606
G1 X100.171 Y88.491 E.01606
G1 X100.233 Y88.992 E.01605
G1 X100.27 Y89.496 E.01607
G1 X100.283 Y90 E.01606
G1 X100.27 Y90.505 E.01606
G1 X100.233 Y91.008 E.01605
G1 X100.171 Y91.509 E.01607
G1 X100.085 Y92.006 E.01605
G1 X99.975 Y92.498 E.01606
G1 X99.84 Y92.985 E.01607
G1 X99.682 Y93.464 E.01605
G1 X99.5 Y93.935 E.01606
G1 X99.295 Y94.396 E.01605
G1 X99.069 Y94.847 E.01606
G1 X98.82 Y95.286 E.01607
G1 X98.55 Y95.713 E.01606
G1 X98.259 Y96.125 E.01606
G1 X97.949 Y96.523 E.01606
G1 X97.619 Y96.905 E.01606
G1 X97.271 Y97.271 E.01606
G1 X96.905 Y97.619 E.01607
G1 X96.523 Y97.949 E.01605
G1 X96.125 Y98.259 E.01606
G1 X95.713 Y98.55 E.01605
G1 X95.286 Y98.82 E.01607
G1 X94.847 Y99.068 E.01605
G1 X94.396 Y99.295 E.01606
G1 X93.935 Y99.5 E.01606
G1 X93.464 Y99.682 E.01606
G1 X92.985 Y99.84 E.01605
G1 X92.498 Y99.975 E.01607
G1 X92.006 Y100.085 E.01605
G1 X91.509 Y100.171 E.01605
G1 X91.008 Y100.233 E.01606
G1 X90.505 Y100.27 E.01606
G1 X90 Y100.283 E.01607
G1 X89.496 Y100.27 E.01604
G1 X88.992 Y100.233 E.01608
G1 X88.491 Y100.171 E.01605
G1 X87.994 Y100.085 E.01606
G1 X87.502 Y99.975 E.01605
G1 X87.015 Y99.84 E.01607
M73 P83 R2
G1 X86.536 Y99.682 E.01605
G1 X86.065 Y99.5 E.01606
G1 X85.603 Y99.295 E.01606
G1 X85.153 Y99.069 E.01605
G1 X84.714 Y98.82 E.01607
G1 X84.287 Y98.55 E.01605
G1 X83.874 Y98.259 E.01607
G1 X83.477 Y97.949 E.01604
G1 X83.095 Y97.619 E.01607
G1 X82.729 Y97.271 E.01605
G1 X82.381 Y96.905 E.01606
G1 X82.051 Y96.523 E.01607
G1 X81.741 Y96.126 E.01604
G1 X81.45 Y95.713 E.01607
G1 X81.18 Y95.286 E.01605
G1 X80.932 Y94.847 E.01606
G1 X80.705 Y94.396 E.01606
G1 X80.5 Y93.935 E.01605
G1 X80.318 Y93.464 E.01607
G1 X80.16 Y92.985 E.01606
G1 X80.026 Y92.499 E.01605
G1 X79.915 Y92.006 E.01606
G1 X79.829 Y91.509 E.01607
G1 X79.767 Y91.008 E.01604
G1 X79.73 Y90.504 E.01608
G1 X79.717 Y90 E.01604
G1 X79.73 Y89.496 E.01606
G1 X79.767 Y88.992 E.01607
G1 X79.829 Y88.491 E.01605
G1 X79.915 Y87.994 E.01605
G1 X80.025 Y87.502 E.01606
G1 X80.16 Y87.015 E.01606
G1 X80.318 Y86.536 E.01606
G1 X80.5 Y86.065 E.01607
G1 X80.704 Y85.604 E.01605
G1 X80.931 Y85.153 E.01606
G1 X81.18 Y84.714 E.01607
G1 X81.45 Y84.287 E.01605
G1 X81.741 Y83.874 E.01607
G1 X82.051 Y83.477 E.01604
G1 X82.381 Y83.095 E.01607
G1 X82.729 Y82.729 E.01606
G1 X83.095 Y82.381 E.01606
G1 X83.477 Y82.052 E.01605
G1 X83.875 Y81.741 E.01607
G1 X84.287 Y81.45 E.01606
G1 X84.713 Y81.18 E.01604
G1 X85.153 Y80.931 E.01608
G1 X85.603 Y80.705 E.01604
G1 X86.065 Y80.5 E.01606
G1 X86.536 Y80.318 E.01607
G1 X87.015 Y80.16 E.01605
G1 X87.502 Y80.025 E.01606
G1 X87.994 Y79.915 E.01606
G1 X88.491 Y79.829 E.01605
G1 X88.992 Y79.767 E.01605
G1 X89.495 Y79.73 E.01604
G1 X90.504 Y79.73 E.03212
G1 X91.008 Y79.767 E.01608
G1 X91.509 Y79.829 E.01605
G1 X92.006 Y79.915 E.01605
G1 X92.44 Y80.012 E.01414
; COOLING_NODE: 5
M204 S250
G1 X92.594 Y79.645 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1459
M204 S5000
G1 X93.099 Y79.785 E.01545
G1 X93.596 Y79.949 E.01544
G1 X94.085 Y80.138 E.01544
G1 X94.564 Y80.35 E.01544
G1 X95.032 Y80.586 E.01544
G1 X95.488 Y80.844 E.01545
G1 X95.931 Y81.124 E.01544
G1 X96.359 Y81.426 E.01545
G1 X96.772 Y81.748 E.01543
G1 X97.169 Y82.091 E.01546
G1 X97.548 Y82.452 E.01543
G1 X97.91 Y82.831 E.01545
G1 X98.252 Y83.228 E.01543
G1 X98.574 Y83.641 E.01545
G1 X98.876 Y84.069 E.01543
G1 X99.156 Y84.512 E.01546
G1 X99.414 Y84.968 E.01543
G1 X99.65 Y85.436 E.01546
G1 X99.862 Y85.915 E.01544
G1 X100.051 Y86.404 E.01544
G1 X100.215 Y86.901 E.01545
G1 X100.355 Y87.406 E.01544
G1 X100.47 Y87.917 E.01544
G1 X100.559 Y88.434 E.01544
G1 X100.623 Y88.954 E.01544
G1 X100.662 Y89.476 E.01545
G1 X100.675 Y90 E.01544
G1 X100.662 Y90.524 E.01545
G1 X100.623 Y91.046 E.01544
G1 X100.559 Y91.566 E.01545
G1 X100.47 Y92.083 E.01544
G1 X100.355 Y92.594 E.01544
G1 X100.215 Y93.099 E.01545
G1 X100.051 Y93.596 E.01544
G1 X99.862 Y94.085 E.01544
G1 X99.65 Y94.564 E.01544
G1 X99.414 Y95.032 E.01544
G1 X99.156 Y95.488 E.01545
G1 X98.876 Y95.931 E.01544
G1 X98.574 Y96.359 E.01544
G1 X98.252 Y96.772 E.01544
G1 X97.91 Y97.169 E.01544
G1 X97.548 Y97.548 E.01544
G1 X97.169 Y97.91 E.01545
G1 X96.772 Y98.252 E.01544
G1 X96.359 Y98.574 E.01544
G1 X95.931 Y98.876 E.01544
G1 X95.488 Y99.156 E.01545
G1 X95.032 Y99.414 E.01544
G1 X94.564 Y99.65 E.01544
G1 X94.085 Y99.862 E.01544
G1 X93.596 Y100.051 E.01544
G1 X93.099 Y100.215 E.01544
G1 X92.594 Y100.355 E.01545
G1 X92.082 Y100.47 E.01544
G1 X91.567 Y100.559 E.01543
G1 X91.046 Y100.623 E.01544
G1 X90.524 Y100.662 E.01545
G1 X90 Y100.675 E.01544
G1 X89.476 Y100.662 E.01544
G1 X88.954 Y100.623 E.01545
G1 X88.434 Y100.559 E.01544
G1 X87.917 Y100.47 E.01544
G1 X87.406 Y100.355 E.01544
G1 X86.901 Y100.215 E.01545
G1 X86.404 Y100.051 E.01544
G1 X85.915 Y99.862 E.01545
G1 X85.436 Y99.65 E.01544
G1 X84.968 Y99.414 E.01544
G1 X84.512 Y99.156 E.01545
G1 X84.069 Y98.876 E.01544
G1 X83.641 Y98.574 E.01545
G1 X83.228 Y98.252 E.01543
G1 X82.831 Y97.909 E.01546
G1 X82.452 Y97.548 E.01543
G1 X82.09 Y97.169 E.01544
G1 X81.748 Y96.772 E.01545
G1 X81.426 Y96.359 E.01543
G1 X81.124 Y95.931 E.01545
G1 X80.844 Y95.488 E.01544
G1 X80.586 Y95.032 E.01544
G1 X80.35 Y94.564 E.01545
G1 X80.138 Y94.085 E.01543
G1 X79.949 Y93.596 E.01545
G1 X79.785 Y93.099 E.01545
G1 X79.645 Y92.594 E.01544
G1 X79.53 Y92.083 E.01544
G1 X79.441 Y91.566 E.01545
G1 X79.377 Y91.046 E.01544
G1 X79.338 Y90.524 E.01545
G1 X79.325 Y90 E.01544
G1 X79.338 Y89.476 E.01544
G1 X79.377 Y88.954 E.01545
G1 X79.441 Y88.434 E.01544
G1 X79.53 Y87.918 E.01544
G1 X79.645 Y87.406 E.01544
G1 X79.785 Y86.901 E.01544
G1 X79.949 Y86.404 E.01544
G1 X80.138 Y85.915 E.01545
G1 X80.35 Y85.436 E.01543
G1 X80.586 Y84.968 E.01545
G1 X80.844 Y84.512 E.01545
G1 X81.124 Y84.069 E.01544
G1 X81.426 Y83.641 E.01545
G1 X81.748 Y83.228 E.01543
G1 X82.09 Y82.831 E.01545
G1 X82.452 Y82.452 E.01544
G1 X82.831 Y82.09 E.01544
G1 X83.228 Y81.748 E.01544
G1 X83.641 Y81.426 E.01545
G1 X84.069 Y81.124 E.01545
G1 X84.512 Y80.844 E.01544
G1 X84.968 Y80.586 E.01545
G1 X85.436 Y80.35 E.01543
G1 X85.915 Y80.138 E.01545
G1 X86.404 Y79.949 E.01545
G1 X86.901 Y79.785 E.01544
G1 X87.406 Y79.645 E.01544
G1 X87.918 Y79.53 E.01544
G1 X88.434 Y79.441 E.01544
G1 X88.954 Y79.377 E.01544
G1 X89.476 Y79.338 E.01544
G1 X90.034 Y79.326 E.01645
G1 X90.524 Y79.338 E.01444
G1 X91.046 Y79.377 E.01545
G1 X91.566 Y79.441 E.01544
G1 X92.082 Y79.53 E.01544
G1 X92.535 Y79.632 E.01367
M106 S124.95
; WIPE_START
G1 F3900
M204 S6000
G1 X93.099 Y79.785 E-.22203
G1 X93.494 Y79.915 E-.15797
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z7.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z7.1 F4000
            G39.3 S1
            G0 Z7.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X90.49 Y80.075 F42000
G1 Z6.7
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.38292
G1 F1740
M204 S6000
G1 X89.514 Y80.074 E.02593
G1 X88.55 Y80.169 E.02572
G1 X87.594 Y80.358 E.02591
G1 X86.66 Y80.64 E.02592
G1 X85.759 Y81.013 E.02592
G1 X84.899 Y81.472 E.02592
G1 X84.087 Y82.013 E.02592
G1 X83.333 Y82.63 E.0259
G1 X82.643 Y83.32 E.02593
G1 X82.023 Y84.073 E.02592
G1 X81.481 Y84.883 E.0259
G1 X81.021 Y85.743 E.02592
G1 X80.646 Y86.644 E.02592
G1 X80.362 Y87.577 E.02593
G1 X80.171 Y88.533 E.02591
G1 X80.075 Y89.504 E.02593
G1 X80.074 Y90.478 E.0259
G1 X80.169 Y91.45 E.02593
G1 X80.358 Y92.406 E.02591
G1 X80.64 Y93.34 E.02591
G1 X81.013 Y94.241 E.02592
G1 X81.472 Y95.102 E.02592
G1 X82.013 Y95.913 E.0259
G1 X82.631 Y96.667 E.02591
G1 X83.32 Y97.357 E.02593
G1 X84.073 Y97.977 E.02592
G1 X84.883 Y98.519 E.0259
G1 X85.743 Y98.98 E.02593
G1 X86.644 Y99.354 E.02593
G1 X87.577 Y99.638 E.02592
G1 X88.533 Y99.829 E.02591
G1 X89.504 Y99.925 E.02593
G1 X90.478 Y99.926 E.02589
G1 X91.449 Y99.831 E.02592
G1 X92.406 Y99.642 E.02592
G1 X93.339 Y99.36 E.02592
G1 X94.241 Y98.987 E.02593
G1 X95.102 Y98.528 E.02593
G1 X95.913 Y97.987 E.0259
G1 X96.667 Y97.37 E.0259
G1 X97.357 Y96.68 E.02592
G1 X97.977 Y95.927 E.02593
G1 X98.519 Y95.117 E.0259
G1 X98.98 Y94.257 E.02592
G1 X99.354 Y93.356 E.02593
G1 X99.638 Y92.423 E.02592
G1 X99.829 Y91.467 E.02592
G1 X99.925 Y90.497 E.02591
G1 X99.926 Y89.513 E.02615
G1 X99.831 Y88.55 E.02569
G1 X99.642 Y87.594 E.0259
G1 X99.36 Y86.661 E.02592
G1 X98.987 Y85.759 E.02592
G1 X98.528 Y84.898 E.02594
G1 X97.987 Y84.087 E.02589
G1 X97.369 Y83.333 E.02592
G1 X96.68 Y82.643 E.02592
G1 X95.927 Y82.023 E.02592
G1 X95.117 Y81.481 E.0259
G1 X94.257 Y81.02 E.02593
G1 X93.356 Y80.646 E.02591
G1 X92.423 Y80.362 E.02593
G1 X91.467 Y80.171 E.02591
G1 X90.549 Y80.08 E.0245
; WIPE_START
G1 F11470.588
G1 X91.467 Y80.171 E-.35031
G1 X91.543 Y80.187 E-.02969
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.1 I-1.129 J-.453 P1  F42000
G1 X84.462 Y97.83 Z7.1
G1 Z6.7
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1740
M204 S6000
G1 X83.914 Y97.416 E.02185
G1 X83.216 Y96.783 E.02995
G1 X96.783 Y83.217 E.61048
G1 X97.108 Y83.558 E.01499
G1 X97.706 Y84.286 E.02996
G1 X98.228 Y85.068 E.02993
G1 X98.46 Y85.477 E.01498
G1 X98.672 Y85.898 E.01499
G1 X99.033 Y86.768 E.02995
G1 X99.306 Y87.669 E.02996
G1 X99.429 Y88.246 E.01879
G1 X88.246 Y99.429 E.5032
G1 X89.059 Y99.547 E.02613
G1 X90 Y99.593 E.02997
G1 X90.941 Y99.547 E.02996
G1 X91.753 Y99.429 E.02613
G1 X80.571 Y88.246 E.5032
G1 X80.453 Y89.059 E.02613
G1 X80.407 Y90 E.02996
G1 X80.453 Y90.941 E.02997
G1 X80.571 Y91.754 E.02614
G1 X91.754 Y80.571 E.5032
G1 X90.941 Y80.453 E.02613
G1 X90.465 Y80.418 E.0152
G1 X89.524 Y80.419 E.02991
G1 X89.059 Y80.453 E.01483
G1 X88.246 Y80.571 E.02614
G1 X99.429 Y91.754 E.5032
G1 X99.306 Y92.331 E.01879
G1 X99.033 Y93.232 E.02996
G1 X98.863 Y93.671 E.01497
G1 X98.461 Y94.522 E.02996
G1 X98.228 Y94.932 E.01499
G1 X97.706 Y95.715 E.02995
G1 X97.108 Y96.443 E.02996
G1 X96.784 Y96.784 E.01498
G1 X83.217 Y83.217 E.61048
G1 X82.585 Y83.914 E.02994
G1 X82.17 Y84.462 E.02186
; CHANGE_LAYER
; Z_HEIGHT: 6.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X82.585 Y83.914 E-.26103
G1 X82.795 Y83.682 E-.11897
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 35/43
; update layer progress
M73 L35
M991 S0 P34 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z7.1 I.44 J1.135 P1  F42000
G1 X92.528 Y79.908 Z7.1
G1 Z6.9
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1448
M204 S6000
G1 X93.02 Y80.044 E.01624
G1 X93.505 Y80.204 E.01626
G1 X93.982 Y80.388 E.01625
M73 P84 R2
G1 X94.448 Y80.595 E.01624
G1 X94.905 Y80.825 E.01626
G1 X95.349 Y81.076 E.01623
G1 X95.78 Y81.349 E.01626
G1 X96.198 Y81.643 E.01624
G1 X96.6 Y81.958 E.01626
G1 X96.987 Y82.291 E.01624
G1 X97.357 Y82.643 E.01626
G1 X97.709 Y83.013 E.01625
G1 X98.042 Y83.399 E.01623
G1 X98.357 Y83.802 E.01626
G1 X98.651 Y84.22 E.01624
G1 X98.924 Y84.651 E.01626
G1 X99.176 Y85.096 E.01624
G1 X99.405 Y85.552 E.01625
G1 X99.612 Y86.019 E.01625
G1 X99.796 Y86.495 E.01625
G1 X99.956 Y86.98 E.01625
G1 X100.092 Y87.472 E.01623
G1 X100.204 Y87.97 E.01626
G1 X100.291 Y88.473 E.01623
G1 X100.354 Y88.98 E.01625
G1 X100.392 Y89.49 E.01626
G1 X100.404 Y90 E.01623
G1 X100.392 Y90.511 E.01626
G1 X100.354 Y91.02 E.01625
G1 X100.291 Y91.527 E.01625
G1 X100.204 Y92.03 E.01624
G1 X100.092 Y92.528 E.01625
G1 X99.956 Y93.02 E.01625
G1 X99.796 Y93.505 E.01624
G1 X99.612 Y93.982 E.01626
G1 X99.405 Y94.448 E.01625
G1 X99.175 Y94.905 E.01625
G1 X98.924 Y95.349 E.01624
G1 X98.651 Y95.78 E.01626
G1 X98.357 Y96.198 E.01624
G1 X98.042 Y96.6 E.01626
G1 X97.709 Y96.987 E.01624
G1 X97.357 Y97.357 E.01625
G1 X96.987 Y97.709 E.01625
G1 X96.6 Y98.042 E.01624
G1 X96.198 Y98.357 E.01625
G1 X95.78 Y98.651 E.01625
G1 X95.349 Y98.924 E.01624
G1 X94.904 Y99.176 E.01626
G1 X94.448 Y99.405 E.01624
G1 X93.981 Y99.612 E.01626
G1 X93.505 Y99.796 E.01624
G1 X93.02 Y99.956 E.01625
G1 X92.528 Y100.092 E.01625
G1 X92.03 Y100.204 E.01624
G1 X91.526 Y100.292 E.01626
G1 X91.02 Y100.354 E.01623
G1 X90.51 Y100.392 E.01627
G1 X90 Y100.404 E.01624
G1 X89.49 Y100.392 E.01624
G1 X88.98 Y100.354 E.01627
G1 X88.473 Y100.291 E.01625
G1 X87.97 Y100.204 E.01624
G1 X87.472 Y100.092 E.01625
G1 X86.98 Y99.956 E.01625
G1 X86.495 Y99.796 E.01624
G1 X86.018 Y99.612 E.01626
G1 X85.552 Y99.405 E.01625
G1 X85.096 Y99.176 E.01624
G1 X84.651 Y98.924 E.01626
G1 X84.22 Y98.651 E.01624
G1 X83.802 Y98.357 E.01625
G1 X83.4 Y98.042 E.01625
G1 X83.013 Y97.709 E.01625
G1 X82.643 Y97.357 E.01624
G1 X82.291 Y96.987 E.01626
G1 X81.958 Y96.6 E.01624
G1 X81.643 Y96.198 E.01626
G1 X81.349 Y95.78 E.01624
G1 X81.076 Y95.349 E.01626
G1 X80.824 Y94.904 E.01625
G1 X80.595 Y94.448 E.01625
G1 X80.388 Y93.981 E.01625
G1 X80.204 Y93.505 E.01624
G1 X80.044 Y93.02 E.01626
G1 X79.908 Y92.528 E.01624
G1 X79.796 Y92.03 E.01624
G1 X79.709 Y91.526 E.01626
G1 X79.646 Y91.02 E.01624
G1 X79.608 Y90.51 E.01627
G1 X79.596 Y90 E.01624
G1 X79.608 Y89.49 E.01624
G1 X79.646 Y88.98 E.01626
G1 X79.709 Y88.474 E.01623
G1 X79.796 Y87.97 E.01626
G1 X79.908 Y87.472 E.01623
G1 X80.044 Y86.98 E.01626
G1 X80.204 Y86.495 E.01625
G1 X80.388 Y86.019 E.01625
G1 X80.595 Y85.552 E.01626
G1 X80.824 Y85.096 E.01624
G1 X81.076 Y84.651 E.01625
G1 X81.349 Y84.22 E.01624
G1 X81.643 Y83.802 E.01626
G1 X81.958 Y83.4 E.01625
G1 X82.291 Y83.013 E.01625
G1 X82.643 Y82.643 E.01624
G1 X83.013 Y82.291 E.01626
G1 X83.4 Y81.958 E.01624
G1 X83.802 Y81.643 E.01625
G1 X84.22 Y81.349 E.01625
G1 X84.651 Y81.076 E.01625
G1 X85.096 Y80.824 E.01626
G1 X85.551 Y80.595 E.01623
G1 X86.019 Y80.388 E.01626
G1 X86.495 Y80.204 E.01624
G1 X86.98 Y80.044 E.01626
G1 X87.472 Y79.908 E.01625
G1 X87.97 Y79.796 E.01623
G1 X88.474 Y79.709 E.01626
G1 X88.98 Y79.646 E.01624
G1 X89.484 Y79.609 E.01608
G1 X90.294 Y79.603 E.02576
G1 X91.024 Y79.647 E.02329
G1 X91.526 Y79.709 E.0161
G1 X92.03 Y79.796 E.01626
G1 X92.469 Y79.895 E.01433
; COOLING_NODE: 5
M204 S250
G1 X92.623 Y79.527 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1448
M204 S5000
G1 X93.134 Y79.669 E.01561
G1 X93.637 Y79.835 E.01562
G1 X94.132 Y80.026 E.01562
G1 X94.616 Y80.24 E.01562
G1 X95.089 Y80.479 E.01562
G1 X95.55 Y80.74 E.01562
G1 X95.998 Y81.023 E.01562
G1 X96.431 Y81.328 E.01561
G1 X96.849 Y81.654 E.01562
G1 X97.25 Y82 E.01562
G1 X97.634 Y82.366 E.01562
G1 X98 Y82.75 E.01562
G1 X98.345 Y83.151 E.01561
G1 X98.672 Y83.569 E.01563
G1 X98.977 Y84.002 E.01561
G1 X99.26 Y84.45 E.01562
G1 X99.521 Y84.911 E.01562
G1 X99.76 Y85.384 E.01562
G1 X99.975 Y85.869 E.01562
G1 X100.165 Y86.363 E.01562
G1 X100.331 Y86.866 E.01562
G1 X100.473 Y87.377 E.01561
G1 X100.589 Y87.894 E.01562
G1 X100.679 Y88.416 E.01561
G1 X100.744 Y88.942 E.01562
G1 X100.783 Y89.47 E.01562
G1 X100.796 Y90 E.01561
G1 X100.783 Y90.53 E.01561
G1 X100.744 Y91.058 E.01562
G1 X100.679 Y91.584 E.01562
G1 X100.589 Y92.106 E.01561
G1 X100.473 Y92.623 E.01562
G1 X100.331 Y93.134 E.01562
G1 X100.165 Y93.637 E.01562
G1 X99.975 Y94.131 E.01562
G1 X99.76 Y94.616 E.01563
G1 X99.521 Y95.089 E.01561
G1 X99.26 Y95.55 E.01562
G1 X98.977 Y95.998 E.01562
G1 X98.672 Y96.431 E.01561
G1 X98.346 Y96.849 E.01562
G1 X98 Y97.25 E.01561
G1 X97.634 Y97.634 E.01562
G1 X97.25 Y98 E.01562
G1 X96.849 Y98.346 E.01561
G1 X96.431 Y98.672 E.01562
G1 X95.998 Y98.977 E.01561
G1 X95.551 Y99.26 E.01562
G1 X95.089 Y99.522 E.01562
G1 X94.616 Y99.76 E.01561
G1 X94.131 Y99.975 E.01563
G1 X93.637 Y100.165 E.01562
G1 X93.134 Y100.331 E.01562
G1 X92.623 Y100.473 E.01562
G1 X92.106 Y100.589 E.01562
G1 X91.584 Y100.679 E.01562
G1 X91.058 Y100.744 E.01561
G1 X90.53 Y100.783 E.01562
G1 X90 Y100.796 E.01561
G1 X89.47 Y100.783 E.01561
G1 X88.942 Y100.744 E.01562
G1 X88.416 Y100.679 E.01562
G1 X87.894 Y100.589 E.01561
G1 X87.377 Y100.473 E.01562
G1 X86.866 Y100.331 E.01562
G1 X86.363 Y100.165 E.01562
G1 X85.869 Y99.975 E.01562
G1 X85.384 Y99.76 E.01563
G1 X84.911 Y99.522 E.01561
G1 X84.449 Y99.26 E.01562
G1 X84.002 Y98.977 E.01561
G1 X83.569 Y98.672 E.01562
G1 X83.151 Y98.346 E.01561
G1 X82.75 Y97.999 E.01562
G1 X82.366 Y97.634 E.01561
G1 X82 Y97.25 E.01563
G1 X81.654 Y96.849 E.01561
G1 X81.328 Y96.431 E.01562
G1 X81.023 Y95.998 E.01561
G1 X80.74 Y95.55 E.01562
G1 X80.479 Y95.089 E.01561
G1 X80.24 Y94.616 E.01562
G1 X80.026 Y94.132 E.01561
G1 X79.835 Y93.637 E.01562
G1 X79.669 Y93.134 E.01562
G1 X79.527 Y92.623 E.01561
G1 X79.411 Y92.106 E.01562
G1 X79.321 Y91.584 E.01562
G1 X79.256 Y91.058 E.01561
G1 X79.217 Y90.53 E.01562
G1 X79.204 Y90 E.01562
G1 X79.217 Y89.47 E.01562
G1 X79.256 Y88.942 E.01562
G1 X79.321 Y88.416 E.01561
G1 X79.411 Y87.894 E.01562
G1 X79.527 Y87.377 E.01561
G1 X79.669 Y86.866 E.01562
G1 X79.835 Y86.363 E.01562
G1 X80.025 Y85.869 E.01561
G1 X80.24 Y85.384 E.01563
G1 X80.479 Y84.911 E.01561
G1 X80.74 Y84.45 E.01561
G1 X81.023 Y84.002 E.01562
G1 X81.328 Y83.569 E.01562
G1 X81.654 Y83.151 E.01561
G1 X82.001 Y82.75 E.01562
G1 X82.366 Y82.366 E.01562
G1 X82.75 Y82 E.01562
G1 X83.151 Y81.654 E.01561
G1 X83.569 Y81.328 E.01562
G1 X84.002 Y81.023 E.01562
G1 X84.449 Y80.74 E.01561
G1 X84.911 Y80.479 E.01563
G1 X85.384 Y80.24 E.01561
G1 X85.868 Y80.026 E.01562
G1 X86.363 Y79.835 E.01562
G1 X86.866 Y79.669 E.01562
G1 X87.377 Y79.527 E.01562
G1 X87.894 Y79.411 E.01561
G1 X88.416 Y79.321 E.01562
G1 X88.942 Y79.256 E.01561
G1 X89.468 Y79.217 E.01557
M106 S124.95
M106 S127.5
G1 X89.868 Y79.214 E.01179
M106 S124.95
M106 S127.5
G1 X90.304 Y79.211 E.01284
G1 X90.66 Y79.232 E.01052
M106 S124.95
M106 S127.5
G1 X91.06 Y79.256 E.01179
M106 S124.95
M106 S127.5
G1 X91.584 Y79.321 E.01557
G1 X92.106 Y79.411 E.01562
G1 X92.565 Y79.514 E.01385
M106 S124.95
; WIPE_START
G1 F5700
M204 S6000
G1 X93.134 Y79.669 E-.22399
G1 X93.524 Y79.797 E-.15601
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z7.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z7.3 F4000
            G39.3 S1
            G0 Z7.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X98.162 Y95.869 F42000
G1 Z6.9
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1448
M204 S6000
G1 X97.773 Y96.38 E.02041
M73 P85 R2
G1 X97.451 Y96.753 E.01569
G1 X97.11 Y97.11 E.01571
G1 X82.89 Y82.89 E.6399
G1 X82.549 Y83.247 E.0157
G1 X82.227 Y83.621 E.0157
G1 X81.923 Y84.01 E.01571
G1 X81.639 Y84.413 E.01571
G1 X81.375 Y84.83 E.0157
G1 X81.132 Y85.26 E.01571
G1 X80.91 Y85.7 E.01569
G1 X80.71 Y86.152 E.01573
G1 X80.532 Y86.612 E.01568
G1 X80.377 Y87.081 E.01571
G1 X80.179 Y87.855 E.02542
G1 X92.145 Y99.821 E.53847
G1 X91.475 Y99.947 E.0217
G1 X90.986 Y100.007 E.01568
G1 X90.493 Y100.044 E.01573
G1 X90 Y100.056 E.01569
G1 X89.507 Y100.044 E.01569
G1 X89.014 Y100.007 E.01572
G1 X88.524 Y99.947 E.0157
G1 X87.855 Y99.821 E.02168
G1 X99.821 Y87.855 E.53847
G1 X99.947 Y88.524 E.02167
G1 X100.007 Y89.014 E.01571
G1 X100.044 Y89.507 E.01572
G1 X100.056 Y90 E.01569
G1 X100.044 Y90.493 E.01571
G1 X100.007 Y90.986 E.01571
G1 X99.947 Y91.476 E.01571
G1 X99.821 Y92.145 E.02168
G1 X87.855 Y80.179 E.53847
G1 X88.525 Y80.053 E.0217
G1 X89.498 Y79.957 E.03112
G1 X90.285 Y79.952 E.02503
G1 X90.993 Y79.994 E.02257
G1 X91.475 Y80.053 E.01546
G1 X91.962 Y80.138 E.01573
G1 X92.145 Y80.179 E.00597
G1 X80.179 Y92.145 E.53847
G1 X80.377 Y92.919 E.02541
G1 X80.532 Y93.388 E.01571
G1 X80.71 Y93.848 E.01572
G1 X80.91 Y94.299 E.01569
G1 X81.132 Y94.74 E.0157
G1 X81.375 Y95.17 E.01571
G1 X81.639 Y95.587 E.01571
G1 X81.923 Y95.99 E.0157
G1 X82.227 Y96.379 E.01571
G1 X82.549 Y96.753 E.01569
G1 X82.89 Y97.11 E.01571
G1 X97.11 Y82.89 E.6399
G1 X97.451 Y83.247 E.0157
G1 X97.773 Y83.62 E.01569
G1 X98.162 Y84.131 E.02042
; CHANGE_LAYER
; Z_HEIGHT: 7.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X97.773 Y83.62 E-.24382
G1 X97.539 Y83.349 E-.13618
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 36/43
; update layer progress
M73 L36
M991 S0 P35 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z7.3 I.707 J-.99 P1  F42000
G1 X92.557 Y79.791 Z7.3
G1 Z7.1
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1464
M204 S6000
G1 X93.055 Y79.929 E.01644
G1 X93.545 Y80.091 E.01643
G1 X94.027 Y80.277 E.01644
G1 X94.5 Y80.486 E.01643
G1 X94.961 Y80.719 E.01644
G1 X95.41 Y80.973 E.01643
G1 X95.847 Y81.25 E.01643
G1 X96.269 Y81.547 E.01643
G1 X96.676 Y81.865 E.01644
G1 X97.067 Y82.202 E.01643
G1 X97.442 Y82.559 E.01644
G1 X97.798 Y82.932 E.01642
G1 X98.135 Y83.324 E.01645
G1 X98.453 Y83.731 E.01644
G1 X98.75 Y84.153 E.01642
G1 X99.027 Y84.59 E.01644
G1 X99.281 Y85.039 E.01644
G1 X99.514 Y85.5 E.01643
G1 X99.723 Y85.972 E.01643
G1 X99.909 Y86.455 E.01645
G1 X100.071 Y86.945 E.01643
G1 X100.209 Y87.443 E.01643
G1 X100.322 Y87.947 E.01644
G1 X100.41 Y88.456 E.01643
G1 X100.473 Y88.969 E.01645
G1 X100.511 Y89.483 E.01642
G1 X100.524 Y90 E.01644
G1 X100.511 Y90.516 E.01644
G1 X100.473 Y91.032 E.01644
G1 X100.41 Y91.544 E.01642
G1 X100.322 Y92.053 E.01645
G1 X100.209 Y92.557 E.01642
G1 X100.071 Y93.055 E.01644
G1 X99.909 Y93.546 E.01644
G1 X99.723 Y94.027 E.01643
G1 X99.514 Y94.5 E.01643
G1 X99.281 Y94.961 E.01644
G1 X99.027 Y95.41 E.01643
G1 X98.75 Y95.847 E.01643
G1 X98.453 Y96.269 E.01645
G1 X98.135 Y96.676 E.01642
G1 X97.798 Y97.067 E.01644
G1 X97.441 Y97.442 E.01644
G1 X97.068 Y97.798 E.01643
G1 X96.676 Y98.135 E.01644
G1 X96.269 Y98.453 E.01644
G1 X95.847 Y98.75 E.01643
G1 X95.41 Y99.027 E.01644
G1 X94.961 Y99.281 E.01644
G1 X94.5 Y99.514 E.01643
G1 X94.027 Y99.723 E.01643
G1 X93.545 Y99.909 E.01644
G1 X93.055 Y100.071 E.01643
G1 X92.557 Y100.209 E.01644
G1 X92.053 Y100.322 E.01643
G1 X91.544 Y100.41 E.01643
G1 X91.032 Y100.473 E.01644
G1 X90.516 Y100.511 E.01645
G1 X90 Y100.524 E.01643
G1 X89.484 Y100.511 E.01641
G1 X88.968 Y100.473 E.01646
G1 X88.456 Y100.41 E.01644
G1 X87.947 Y100.322 E.01642
G1 X87.443 Y100.209 E.01644
G1 X86.945 Y100.071 E.01644
G1 X86.455 Y99.909 E.01643
G1 X85.973 Y99.723 E.01643
G1 X85.5 Y99.513 E.01644
G1 X85.039 Y99.281 E.01644
G1 X84.59 Y99.027 E.01643
G1 X84.153 Y98.75 E.01643
G1 X83.731 Y98.453 E.01645
G1 X83.324 Y98.135 E.01643
G1 X82.932 Y97.798 E.01644
G1 X82.559 Y97.442 E.01643
G1 X82.202 Y97.067 E.01644
G1 X81.865 Y96.676 E.01643
G1 X81.547 Y96.269 E.01644
G1 X81.25 Y95.847 E.01643
G1 X80.973 Y95.41 E.01644
G1 X80.719 Y94.961 E.01643
G1 X80.486 Y94.5 E.01644
G1 X80.277 Y94.027 E.01643
G1 X80.091 Y93.545 E.01643
G1 X79.929 Y93.055 E.01644
G1 X79.791 Y92.557 E.01643
G1 X79.678 Y92.054 E.01642
G1 X79.59 Y91.544 E.01645
G1 X79.527 Y91.032 E.01643
G1 X79.489 Y90.516 E.01644
G1 X79.476 Y90 E.01644
G1 X79.489 Y89.484 E.01641
G1 X79.527 Y88.968 E.01646
G1 X79.59 Y88.456 E.01643
G1 X79.678 Y87.947 E.01643
G1 X79.791 Y87.443 E.01643
G1 X79.929 Y86.945 E.01644
G1 X80.091 Y86.455 E.01644
G1 X80.277 Y85.973 E.01643
G1 X80.486 Y85.5 E.01644
G1 X80.719 Y85.039 E.01644
G1 X80.973 Y84.589 E.01644
G1 X81.25 Y84.153 E.01642
G1 X81.547 Y83.731 E.01645
G1 X81.865 Y83.324 E.01643
G1 X82.202 Y82.932 E.01645
G1 X82.558 Y82.558 E.01643
G1 X82.932 Y82.202 E.01643
G1 X83.324 Y81.865 E.01644
G1 X83.731 Y81.547 E.01643
G1 X84.153 Y81.25 E.01643
G1 X84.59 Y80.973 E.01644
G1 X85.039 Y80.719 E.01643
G1 X85.5 Y80.486 E.01644
G1 X85.973 Y80.277 E.01643
G1 X86.455 Y80.091 E.01644
G1 X86.945 Y79.929 E.01643
G1 X87.443 Y79.791 E.01644
G1 X87.947 Y79.678 E.01643
G1 X88.456 Y79.59 E.01643
G1 X88.968 Y79.527 E.01643
G1 X89.483 Y79.489 E.01641
G1 X90.516 Y79.489 E.03288
G1 X91.032 Y79.527 E.01645
G1 X91.544 Y79.59 E.01643
G1 X92.053 Y79.678 E.01643
G1 X92.498 Y79.778 E.01452
; COOLING_NODE: 5
M204 S250
G1 X92.652 Y79.411 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1464
M204 S5000
G1 X93.169 Y79.554 E.01579
G1 X93.678 Y79.722 E.01579
G1 X94.177 Y79.915 E.01579
G1 X94.667 Y80.132 E.01579
G1 X95.146 Y80.373 E.01579
G1 X95.612 Y80.637 E.0158
G1 X96.065 Y80.924 E.01579
G1 X96.503 Y81.232 E.01578
G1 X96.925 Y81.562 E.0158
G1 X97.331 Y81.912 E.01579
G1 X97.719 Y82.281 E.01579
G1 X98.088 Y82.669 E.01579
G1 X98.438 Y83.075 E.0158
G1 X98.768 Y83.497 E.01579
G1 X99.076 Y83.935 E.01578
G1 X99.363 Y84.388 E.0158
G1 X99.627 Y84.854 E.0158
G1 X99.868 Y85.333 E.01579
G1 X100.085 Y85.822 E.01578
G1 X100.278 Y86.323 E.0158
G1 X100.446 Y86.831 E.0158
G1 X100.589 Y87.347 E.01578
G1 X100.706 Y87.87 E.0158
G1 X100.798 Y88.398 E.01579
G1 X100.864 Y88.93 E.0158
G1 X100.903 Y89.464 E.01579
G1 X100.916 Y90 E.01579
G1 X100.903 Y90.536 E.01579
G1 X100.864 Y91.07 E.0158
G1 X100.798 Y91.602 E.01578
G1 X100.706 Y92.13 E.0158
G1 X100.589 Y92.652 E.01578
G1 X100.446 Y93.169 E.0158
G1 X100.278 Y93.678 E.01579
G1 X100.085 Y94.177 E.01579
G1 X99.868 Y94.667 E.01579
G1 X99.627 Y95.146 E.01579
M73 P86 R2
G1 X99.363 Y95.612 E.01579
G1 X99.077 Y96.065 E.01579
G1 X98.768 Y96.503 E.0158
G1 X98.438 Y96.925 E.01579
G1 X98.088 Y97.331 E.01579
G1 X97.719 Y97.719 E.0158
G1 X97.331 Y98.088 E.01579
G1 X96.925 Y98.438 E.01579
G1 X96.503 Y98.768 E.01579
G1 X96.065 Y99.076 E.01578
G1 X95.612 Y99.363 E.01579
G1 X95.146 Y99.627 E.0158
G1 X94.667 Y99.868 E.01579
G1 X94.177 Y100.085 E.01579
G1 X93.678 Y100.278 E.01579
G1 X93.169 Y100.446 E.0158
G1 X92.652 Y100.589 E.01579
G1 X92.13 Y100.706 E.01579
G1 X91.602 Y100.798 E.01579
G1 X91.07 Y100.864 E.01579
G1 X90.536 Y100.903 E.0158
G1 X90 Y100.916 E.01579
G1 X89.465 Y100.903 E.01578
G1 X88.93 Y100.864 E.0158
G1 X88.398 Y100.798 E.01579
G1 X87.87 Y100.706 E.01579
G1 X87.348 Y100.589 E.01579
G1 X86.831 Y100.446 E.01579
G1 X86.323 Y100.278 E.01579
G1 X85.823 Y100.085 E.01579
G1 X85.333 Y99.868 E.0158
G1 X84.854 Y99.627 E.01579
G1 X84.388 Y99.363 E.01579
G1 X83.935 Y99.077 E.01579
G1 X83.497 Y98.768 E.0158
G1 X83.075 Y98.438 E.01578
G1 X82.669 Y98.088 E.0158
G1 X82.281 Y97.719 E.01579
G1 X81.912 Y97.331 E.01579
G1 X81.562 Y96.925 E.01579
G1 X81.232 Y96.503 E.0158
G1 X80.924 Y96.065 E.01578
G1 X80.637 Y95.612 E.0158
G1 X80.373 Y95.146 E.01579
G1 X80.132 Y94.667 E.01579
G1 X79.915 Y94.178 E.01579
G1 X79.722 Y93.678 E.01579
G1 X79.554 Y93.168 E.0158
G1 X79.411 Y92.653 E.01578
G1 X79.294 Y92.13 E.01579
G1 X79.202 Y91.602 E.0158
G1 X79.136 Y91.07 E.01579
G1 X79.097 Y90.536 E.0158
G1 X79.084 Y90 E.01579
G1 X79.097 Y89.465 E.01578
G1 X79.136 Y88.93 E.0158
G1 X79.202 Y88.398 E.01579
G1 X79.294 Y87.87 E.01579
G1 X79.411 Y87.348 E.01579
G1 X79.554 Y86.831 E.01579
G1 X79.722 Y86.323 E.01579
G1 X79.915 Y85.823 E.01579
G1 X80.132 Y85.333 E.01579
G1 X80.373 Y84.854 E.01579
G1 X80.637 Y84.388 E.0158
G1 X80.924 Y83.935 E.01578
G1 X81.232 Y83.497 E.0158
G1 X81.562 Y83.075 E.01578
G1 X81.912 Y82.669 E.0158
G1 X82.281 Y82.281 E.01579
G1 X82.669 Y81.912 E.01579
G1 X83.075 Y81.562 E.0158
G1 X83.497 Y81.232 E.01578
G1 X83.935 Y80.924 E.01579
G1 X84.388 Y80.637 E.0158
G1 X84.854 Y80.373 E.01578
G1 X85.333 Y80.132 E.0158
G1 X85.823 Y79.915 E.01579
G1 X86.322 Y79.722 E.01579
G1 X86.831 Y79.554 E.01579
G1 X87.348 Y79.411 E.01579
G1 X87.87 Y79.294 E.01579
G1 X88.398 Y79.202 E.01579
G1 X88.93 Y79.136 E.01579
G1 X89.464 Y79.097 E.01578
G1 X90.046 Y79.085 E.01716
G1 X90.536 Y79.097 E.01443
G1 X91.07 Y79.136 E.0158
G1 X91.602 Y79.202 E.01579
G1 X92.13 Y79.294 E.01579
G1 X92.594 Y79.398 E.01402
M106 S124.95
; WIPE_START
G1 F5700
M204 S6000
G1 X93.169 Y79.554 E-.22641
G1 X93.553 Y79.681 E-.15359
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z7.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z7.5 F4000
            G39.3 S1
            G0 Z7.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X84.045 Y98.248 F42000
G1 Z7.1
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1464
M204 S6000
G1 X83.545 Y97.866 E.02002
G1 X83.166 Y97.539 E.0159
G1 X82.805 Y97.195 E.01589
G1 X97.195 Y82.805 E.64753
G1 X97.539 Y83.166 E.01588
G1 X97.866 Y83.545 E.0159
G1 X98.173 Y83.938 E.01589
G1 X98.461 Y84.347 E.01588
G1 X98.728 Y84.769 E.0159
G1 X98.974 Y85.203 E.01589
G1 X99.199 Y85.649 E.01589
G1 X99.401 Y86.106 E.01589
G1 X99.581 Y86.572 E.0159
G1 X99.737 Y87.046 E.01589
G1 X99.922 Y87.754 E.02328
G1 X87.754 Y99.922 E.54749
G1 X88.015 Y99.98 E.0085
G1 X88.507 Y100.065 E.01587
G1 X89.002 Y100.127 E.0159
G1 X89.501 Y100.163 E.01592
G1 X90 Y100.176 E.01587
G1 X90.499 Y100.163 E.01589
G1 X90.998 Y100.127 E.01591
G1 X91.493 Y100.065 E.0159
G1 X92.246 Y99.922 E.02437
G1 X80.078 Y87.754 E.54749
G1 X79.935 Y88.507 E.02437
G1 X79.873 Y89.002 E.01589
G1 X79.837 Y89.501 E.01592
G1 X79.824 Y90 E.01587
G1 X79.837 Y90.499 E.01589
G1 X79.873 Y90.998 E.0159
G1 X79.935 Y91.493 E.01588
G1 X80.078 Y92.246 E.02439
G1 X92.246 Y80.078 E.54749
G1 X91.493 Y79.935 E.02437
G1 X90.998 Y79.873 E.01588
G1 X90.503 Y79.837 E.01577
G1 X89.496 Y79.837 E.03207
G1 X89.002 Y79.874 E.01574
G1 X88.507 Y79.935 E.01588
G1 X87.754 Y80.078 E.02438
G1 X99.922 Y92.246 E.54749
G1 X99.737 Y92.954 E.02328
G1 X99.581 Y93.428 E.0159
G1 X99.401 Y93.894 E.01589
G1 X99.199 Y94.351 E.01588
G1 X98.974 Y94.797 E.0159
G1 X98.728 Y95.231 E.01588
G1 X98.461 Y95.653 E.01589
G1 X98.173 Y96.062 E.01591
G1 X97.866 Y96.455 E.01587
G1 X97.54 Y96.833 E.01589
G1 X97.195 Y97.195 E.0159
G1 X82.805 Y82.805 E.64753
G1 X82.461 Y83.166 E.01588
G1 X82.134 Y83.545 E.0159
G1 X81.752 Y84.045 E.02003
; CHANGE_LAYER
; Z_HEIGHT: 7.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X82.134 Y83.545 E-.23917
G1 X82.376 Y83.264 E-.14083
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 37/43
; update layer progress
M73 L37
M991 S0 P36 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z7.5 I.403 J1.148 P1  F42000
G1 X92.585 Y79.682 Z7.5
G1 Z7.3
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1484
M204 S6000
G1 X93.088 Y79.821 E.01661
G1 X93.584 Y79.985 E.01661
G1 X94.071 Y80.173 E.01661
G1 X94.548 Y80.384 E.01661
G1 X95.014 Y80.619 E.01662
G1 X95.469 Y80.876 E.01661
G1 X95.91 Y81.155 E.01661
G1 X96.337 Y81.456 E.01662
G1 X96.748 Y81.777 E.0166
G1 X97.144 Y82.118 E.01662
G1 X97.522 Y82.478 E.0166
G1 X97.882 Y82.857 E.01662
G1 X98.223 Y83.252 E.0166
G1 X98.544 Y83.663 E.01661
G1 X98.844 Y84.09 E.01661
G1 X99.124 Y84.531 E.01662
G1 X99.381 Y84.986 E.01661
G1 X99.616 Y85.452 E.0166
G1 X99.827 Y85.929 E.01662
G1 X100.015 Y86.417 E.01662
G1 X100.179 Y86.912 E.01661
G1 X100.318 Y87.415 E.01661
G1 X100.433 Y87.925 E.01661
G1 X100.522 Y88.439 E.01662
G1 X100.586 Y88.957 E.01662
G1 X100.624 Y89.478 E.01661
G1 X100.637 Y90 E.0166
G1 X100.624 Y90.522 E.01662
G1 X100.586 Y91.043 E.01661
G1 X100.522 Y91.561 E.0166
G1 X100.433 Y92.075 E.01662
G1 X100.318 Y92.585 E.0166
G1 X100.179 Y93.088 E.01661
G1 X100.015 Y93.584 E.01662
G1 X99.828 Y94.071 E.01661
G1 X99.616 Y94.548 E.01662
G1 X99.381 Y95.014 E.01661
G1 X99.124 Y95.469 E.01662
G1 X98.844 Y95.91 E.01661
G1 X98.544 Y96.337 E.01661
G1 X98.223 Y96.748 E.01661
G1 X97.882 Y97.143 E.01661
G1 X97.522 Y97.522 E.01661
G1 X97.143 Y97.882 E.01661
G1 X96.748 Y98.223 E.01661
G1 X96.336 Y98.544 E.01662
G1 X95.91 Y98.844 E.0166
G1 X95.468 Y99.124 E.01663
G1 X95.015 Y99.381 E.01659
G1 X94.548 Y99.616 E.01662
G1 X94.071 Y99.828 E.01661
G1 X93.584 Y100.015 E.01661
G1 X93.088 Y100.179 E.01661
G1 X92.584 Y100.318 E.01662
G1 X92.075 Y100.433 E.01661
G1 X91.561 Y100.522 E.01661
G1 X91.043 Y100.586 E.01661
G1 X90.522 Y100.624 E.01663
G1 X90 Y100.637 E.0166
G1 X89.478 Y100.624 E.0166
G1 X88.957 Y100.586 E.01663
G1 X88.439 Y100.522 E.01661
G1 X87.925 Y100.433 E.01661
G1 X87.415 Y100.318 E.01661
G1 X86.912 Y100.179 E.01661
G1 X86.417 Y100.015 E.01661
G1 X85.929 Y99.827 E.01662
G1 X85.452 Y99.616 E.01661
G1 X84.985 Y99.381 E.01662
G1 X84.532 Y99.124 E.01659
G1 X84.09 Y98.845 E.01662
G1 X83.663 Y98.544 E.01662
G1 X83.252 Y98.223 E.01661
G1 X82.856 Y97.882 E.01662
G1 X82.478 Y97.522 E.0166
G1 X82.118 Y97.143 E.01662
G1 X81.777 Y96.748 E.0166
G1 X81.456 Y96.337 E.01662
G1 X81.155 Y95.91 E.01662
G1 X80.876 Y95.468 E.01661
M73 P87 R2
G1 X80.619 Y95.014 E.0166
G1 X80.384 Y94.548 E.01661
G1 X80.173 Y94.071 E.01661
G1 X79.985 Y93.584 E.01661
G1 X79.821 Y93.088 E.01662
G1 X79.682 Y92.585 E.0166
G1 X79.567 Y92.076 E.01661
G1 X79.478 Y91.561 E.01663
G1 X79.414 Y91.043 E.0166
G1 X79.376 Y90.522 E.01662
G1 X79.363 Y90 E.01661
G1 X79.376 Y89.478 E.0166
G1 X79.414 Y88.957 E.01663
G1 X79.478 Y88.439 E.01661
G1 X79.567 Y87.925 E.0166
G1 X79.682 Y87.415 E.01662
G1 X79.821 Y86.912 E.0166
G1 X79.985 Y86.416 E.01662
G1 X80.173 Y85.929 E.01661
G1 X80.384 Y85.452 E.01661
G1 X80.619 Y84.986 E.0166
G1 X80.876 Y84.531 E.01662
G1 X81.156 Y84.09 E.01661
G1 X81.456 Y83.663 E.01662
G1 X81.777 Y83.252 E.01661
G1 X82.118 Y82.857 E.01661
G1 X82.478 Y82.478 E.01661
G1 X82.857 Y82.118 E.01662
G1 X83.252 Y81.777 E.01661
G1 X83.663 Y81.456 E.0166
G1 X84.09 Y81.155 E.01663
G1 X84.531 Y80.876 E.01659
G1 X84.986 Y80.619 E.01663
G1 X85.452 Y80.384 E.0166
G1 X85.929 Y80.172 E.01662
G1 X86.416 Y79.985 E.01661
G1 X86.912 Y79.821 E.01661
G1 X87.415 Y79.682 E.01662
G1 X87.925 Y79.567 E.01661
G1 X88.439 Y79.478 E.0166
G1 X88.957 Y79.414 E.01661
G1 X89.473 Y79.376 E.01646
G1 X90.284 Y79.37 E.02582
G1 X91.047 Y79.415 E.02432
G1 X91.561 Y79.478 E.01646
G1 X92.075 Y79.567 E.0166
G1 X92.526 Y79.668 E.0147
; COOLING_NODE: 5
M204 S250
G1 X92.68 Y79.301 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1484
M204 S5000
G1 X93.201 Y79.446 E.01594
G1 X93.716 Y79.615 E.01596
G1 X94.221 Y79.81 E.01596
G1 X94.716 Y80.03 E.01595
G1 X95.199 Y80.273 E.01596
G1 X95.67 Y80.54 E.01595
G1 X96.128 Y80.829 E.01595
G1 X96.57 Y81.141 E.01596
G1 X96.997 Y81.474 E.01595
G1 X97.407 Y81.828 E.01596
G1 X97.799 Y82.201 E.01595
G1 X98.172 Y82.593 E.01596
G1 X98.526 Y83.003 E.01595
G1 X98.859 Y83.43 E.01596
G1 X99.171 Y83.872 E.01595
G1 X99.46 Y84.33 E.01596
G1 X99.727 Y84.801 E.01595
G1 X99.97 Y85.284 E.01595
G1 X100.19 Y85.779 E.01596
G1 X100.385 Y86.284 E.01595
G1 X100.554 Y86.799 E.01596
G1 X100.699 Y87.32 E.01594
G1 X100.817 Y87.848 E.01596
G1 X100.91 Y88.382 E.01596
G1 X100.976 Y88.919 E.01595
G1 X101.016 Y89.459 E.01596
G1 X101.029 Y90 E.01595
G1 X101.016 Y90.541 E.01595
G1 X100.976 Y91.081 E.01596
G1 X100.91 Y91.618 E.01595
G1 X100.817 Y92.152 E.01596
G1 X100.699 Y92.68 E.01595
G1 X100.554 Y93.202 E.01595
G1 X100.385 Y93.716 E.01596
G1 X100.19 Y94.221 E.01595
G1 X99.97 Y94.716 E.01595
G1 X99.727 Y95.199 E.01595
G1 X99.46 Y95.67 E.01596
G1 X99.171 Y96.128 E.01596
G1 X98.859 Y96.57 E.01595
G1 X98.526 Y96.997 E.01595
G1 X98.172 Y97.407 E.01596
G1 X97.799 Y97.799 E.01595
G1 X97.407 Y98.172 E.01596
G1 X96.997 Y98.526 E.01595
G1 X96.57 Y98.859 E.01597
G1 X96.128 Y99.171 E.01595
G1 X95.67 Y99.46 E.01596
G1 X95.199 Y99.727 E.01595
G1 X94.716 Y99.97 E.01595
G1 X94.221 Y100.19 E.01596
G1 X93.716 Y100.385 E.01595
G1 X93.202 Y100.554 E.01596
G1 X92.68 Y100.699 E.01595
G1 X92.152 Y100.817 E.01596
G1 X91.618 Y100.91 E.01595
G1 X91.081 Y100.976 E.01596
G1 X90.541 Y101.016 E.01596
G1 X90 Y101.029 E.01595
G1 X89.459 Y101.016 E.01595
G1 X88.919 Y100.976 E.01596
G1 X88.381 Y100.91 E.01596
G1 X87.848 Y100.817 E.01595
G1 X87.32 Y100.699 E.01596
G1 X86.799 Y100.554 E.01594
G1 X86.284 Y100.385 E.01596
G1 X85.779 Y100.19 E.01596
G1 X85.284 Y99.97 E.01596
G1 X84.801 Y99.727 E.01596
G1 X84.33 Y99.46 E.01594
G1 X83.872 Y99.171 E.01596
G1 X83.43 Y98.859 E.01596
G1 X83.003 Y98.526 E.01595
G1 X82.593 Y98.172 E.01596
G1 X82.201 Y97.799 E.01595
G1 X81.828 Y97.407 E.01596
G1 X81.474 Y96.997 E.01595
G1 X81.141 Y96.57 E.01595
G1 X80.829 Y96.128 E.01596
G1 X80.54 Y95.67 E.01596
G1 X80.273 Y95.199 E.01595
G1 X80.03 Y94.716 E.01596
G1 X79.81 Y94.221 E.01596
G1 X79.615 Y93.716 E.01596
G1 X79.446 Y93.201 E.01596
G1 X79.301 Y92.68 E.01595
G1 X79.183 Y92.152 E.01595
G1 X79.09 Y91.618 E.01596
G1 X79.024 Y91.081 E.01595
G1 X78.984 Y90.541 E.01596
G1 X78.971 Y90 E.01595
G1 X78.984 Y89.459 E.01595
G1 X79.024 Y88.919 E.01596
G1 X79.09 Y88.382 E.01596
G1 X79.183 Y87.848 E.01595
G1 X79.301 Y87.32 E.01596
G1 X79.446 Y86.799 E.01594
G1 X79.615 Y86.284 E.01596
G1 X79.81 Y85.779 E.01596
G1 X80.03 Y85.284 E.01595
G1 X80.273 Y84.801 E.01595
G1 X80.54 Y84.33 E.01596
G1 X80.829 Y83.872 E.01595
G1 X81.141 Y83.43 E.01596
G1 X81.474 Y83.003 E.01596
G1 X81.828 Y82.593 E.01595
G1 X82.201 Y82.201 E.01595
G1 X82.593 Y81.828 E.01596
G1 X83.003 Y81.474 E.01596
G1 X83.43 Y81.141 E.01595
G1 X83.872 Y80.829 E.01596
G1 X84.33 Y80.54 E.01595
G1 X84.801 Y80.273 E.01596
G1 X85.284 Y80.03 E.01595
G1 X85.779 Y79.81 E.01596
G1 X86.284 Y79.615 E.01595
G1 X86.798 Y79.446 E.01596
G1 X87.32 Y79.301 E.01595
G1 X87.848 Y79.183 E.01596
G1 X88.382 Y79.09 E.01595
G1 X88.919 Y79.024 E.01596
G1 X89.457 Y78.984 E.01591
M106 S124.95
M106 S127.5
G1 X89.857 Y78.981 E.01179
M106 S124.95
M106 S127.5
G1 X90.294 Y78.978 E.01289
G1 X90.683 Y79.001 E.01149
M106 S124.95
M106 S127.5
G1 X91.083 Y79.024 E.01179
M106 S124.95
M106 S127.5
G1 X91.618 Y79.09 E.01591
G1 X92.152 Y79.183 E.01595
G1 X92.622 Y79.288 E.01419
M106 S124.95
; WIPE_START
G1 F6600
M204 S6000
G1 X93.201 Y79.446 E-.22834
G1 X93.58 Y79.571 E-.15166
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z7.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z7.7 F4000
            G39.3 S1
            G0 Z7.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X83.964 Y81.671 F42000
G1 Z7.3
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1484
M204 S6000
G1 X83.473 Y82.047 E.01967
G1 X83.09 Y82.377 E.01607
G1 X82.725 Y82.725 E.01607
G1 X97.275 Y97.275 E.65474
G1 X97.624 Y96.909 E.01608
G1 X97.953 Y96.527 E.01605
G1 X98.264 Y96.129 E.01608
G1 X98.555 Y95.716 E.01605
G1 X98.825 Y95.289 E.01608
G1 X99.074 Y94.85 E.01607
G1 X99.301 Y94.399 E.01606
G1 X99.506 Y93.937 E.01607
G1 X99.687 Y93.466 E.01606
G1 X99.846 Y92.987 E.01607
G1 X100.016 Y92.34 E.02127
G1 X87.66 Y79.984 E.55602
G1 X88.49 Y79.823 E.02692
G1 X88.991 Y79.761 E.01606
G1 X89.487 Y79.724 E.01582
G1 X90.275 Y79.718 E.02508
G1 X91.016 Y79.762 E.0236
G1 X91.51 Y79.823 E.01584
G1 X92.34 Y79.984 E.02692
G1 X79.984 Y92.34 E.55602
G1 X79.823 Y91.509 E.02693
G1 X79.761 Y91.009 E.01605
G1 X79.724 Y90.505 E.01608
G1 X79.711 Y90 E.01606
G1 X79.724 Y89.495 E.01606
G1 X79.761 Y88.991 E.01609
G1 X79.823 Y88.49 E.01606
G1 X79.984 Y87.66 E.02692
G1 X92.34 Y100.016 E.55602
G1 X91.51 Y100.177 E.02692
G1 X91.009 Y100.239 E.01606
G1 X90.505 Y100.276 E.01609
G1 X90 Y100.289 E.01606
G1 X89.496 Y100.276 E.01606
G1 X88.991 Y100.239 E.01609
G1 X88.49 Y100.177 E.01606
G1 X87.66 Y100.016 E.02692
G1 X100.016 Y87.66 E.55602
G1 X99.846 Y87.013 E.02126
G1 X99.687 Y86.534 E.01607
G1 X99.506 Y86.063 E.01607
G1 X99.301 Y85.601 E.01609
G1 X99.074 Y85.15 E.01605
G1 X98.825 Y84.711 E.01607
G1 X98.555 Y84.284 E.01607
G1 X98.264 Y83.871 E.01606
G1 X97.953 Y83.473 E.01609
G1 X97.624 Y83.091 E.01604
G1 X97.275 Y82.725 E.01608
G1 X82.725 Y97.275 E.65474
G1 X83.091 Y97.624 E.01607
G1 X83.473 Y97.953 E.01606
G1 X83.964 Y98.329 E.01968
; CHANGE_LAYER
; Z_HEIGHT: 7.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X83.473 Y97.953 E-.23498
G1 X83.184 Y97.704 E-.14502
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 38/43
; update layer progress
M73 L38
M991 S0 P37 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z7.7 I1.08 J.561 P1  F42000
G1 X92.612 Y79.573 Z7.7
G1 Z7.5
G1 E.4 F1800
M106 S127.5
; FEATURE: Inner wall
G1 F1690
M204 S6000
G1 X93.12 Y79.714 E.01679
G1 X93.621 Y79.879 E.01679
G1 X94.114 Y80.069 E.01679
G1 X94.596 Y80.283 E.01678
G1 X95.067 Y80.52 E.01678
G1 X95.526 Y80.78 E.0168
G1 X95.972 Y81.062 E.01678
G1 X96.403 Y81.366 E.0168
G1 X96.819 Y81.691 E.01678
G1 X97.219 Y82.035 E.01678
G1 X97.601 Y82.399 E.01679
G1 X97.964 Y82.781 E.01679
G1 X98.309 Y83.181 E.01679
G1 X98.634 Y83.597 E.01678
G1 X98.937 Y84.028 E.01679
G1 X99.22 Y84.474 E.01679
G1 X99.48 Y84.933 E.01679
G1 X99.717 Y85.404 E.01678
G1 X99.931 Y85.886 E.01679
G1 X100.121 Y86.379 E.01679
G1 X100.286 Y86.88 E.01678
G1 X100.427 Y87.388 E.0168
G1 X100.542 Y87.903 E.01677
G1 X100.633 Y88.423 E.01679
G1 X100.697 Y88.946 E.01678
G1 X100.736 Y89.473 E.0168
G1 X100.749 Y90 E.01677
G1 X100.736 Y90.527 E.01678
G1 X100.697 Y91.054 E.0168
G1 X100.633 Y91.577 E.01678
G1 X100.542 Y92.097 E.01679
G1 X100.427 Y92.612 E.01678
G1 X100.286 Y93.12 E.01679
G1 X100.121 Y93.621 E.01678
G1 X99.931 Y94.114 E.0168
G1 X99.717 Y94.596 E.01677
G1 X99.48 Y95.067 E.01679
G1 X99.22 Y95.526 E.01679
G1 X98.937 Y95.972 E.01679
G1 X98.634 Y96.403 E.01677
G1 X98.309 Y96.819 E.0168
G1 X97.964 Y97.219 E.01679
G1 X97.601 Y97.601 E.01677
G1 X97.219 Y97.964 E.0168
G1 X96.819 Y98.309 E.01679
G1 X96.403 Y98.633 E.01677
G1 X95.972 Y98.938 E.0168
G1 X95.526 Y99.22 E.01678
G1 X95.067 Y99.48 E.01679
G1 X94.596 Y99.717 E.01678
G1 X94.113 Y99.931 E.01679
G1 X93.621 Y100.121 E.01678
G1 X93.12 Y100.286 E.01679
G1 X92.612 Y100.427 E.01679
G1 X92.097 Y100.543 E.01679
G1 X91.577 Y100.633 E.01678
G1 X91.054 Y100.697 E.01678
M73 P88 R2
G1 X90.527 Y100.736 E.0168
G1 X90 Y100.749 E.01678
G1 X89.473 Y100.736 E.01678
G1 X88.946 Y100.697 E.0168
G1 X88.423 Y100.633 E.01678
G1 X87.903 Y100.542 E.0168
G1 X87.388 Y100.427 E.01678
G1 X86.88 Y100.286 E.01679
G1 X86.379 Y100.121 E.01679
G1 X85.887 Y99.931 E.01678
G1 X85.404 Y99.717 E.0168
G1 X84.933 Y99.48 E.01679
M73 P88 R1
G1 X84.474 Y99.22 E.01678
G1 X84.028 Y98.938 E.01678
G1 X83.597 Y98.634 E.01679
G1 X83.181 Y98.309 E.01679
G1 X82.782 Y97.965 E.01677
G1 X82.399 Y97.601 E.01679
G1 X82.035 Y97.218 E.01679
G1 X81.691 Y96.819 E.01679
G1 X81.367 Y96.403 E.01677
G1 X81.062 Y95.972 E.0168
G1 X80.78 Y95.526 E.01678
G1 X80.52 Y95.067 E.0168
G1 X80.283 Y94.596 E.01678
G1 X80.069 Y94.113 E.0168
G1 X79.879 Y93.621 E.01677
G1 X79.714 Y93.12 E.0168
G1 X79.573 Y92.612 E.01678
G1 X79.458 Y92.097 E.01678
G1 X79.367 Y91.577 E.0168
G1 X79.303 Y91.054 E.01678
G1 X79.264 Y90.527 E.01679
G1 X79.251 Y90 E.01678
G1 X79.264 Y89.473 E.01678
G1 X79.303 Y88.946 E.0168
G1 X79.367 Y88.423 E.01678
G1 X79.458 Y87.903 E.01679
G1 X79.573 Y87.388 E.01678
G1 X79.714 Y86.88 E.01678
G1 X79.879 Y86.379 E.0168
G1 X80.069 Y85.887 E.01678
G1 X80.283 Y85.404 E.01679
G1 X80.52 Y84.933 E.01678
G1 X80.78 Y84.474 E.0168
G1 X81.063 Y84.028 E.01678
G1 X81.366 Y83.597 E.01679
G1 X81.691 Y83.181 E.01678
G1 X82.036 Y82.781 E.01679
G1 X82.399 Y82.399 E.01679
G1 X82.781 Y82.036 E.01679
G1 X83.181 Y81.691 E.01679
G1 X83.597 Y81.366 E.01679
G1 X84.028 Y81.063 E.01678
G1 X84.474 Y80.78 E.01678
G1 X84.933 Y80.52 E.01679
G1 X85.404 Y80.283 E.01679
G1 X85.886 Y80.069 E.01679
G1 X86.379 Y79.879 E.01679
G1 X86.88 Y79.714 E.01678
G1 X87.388 Y79.573 E.0168
G1 X87.903 Y79.458 E.01679
G1 X88.422 Y79.367 E.01677
G1 X88.946 Y79.303 E.0168
G1 X89.473 Y79.264 E.01679
G1 X90 Y79.251 E.01678
G1 X90.526 Y79.264 E.01676
G1 X91.053 Y79.303 E.01681
G1 X91.577 Y79.367 E.0168
G1 X92.097 Y79.458 E.01677
G1 X92.553 Y79.56 E.01488
; COOLING_NODE: 5
M204 S250
G1 X92.707 Y79.193 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1690
M204 S5000
G1 X93.234 Y79.339 E.01612
G1 X93.753 Y79.51 E.01612
G1 X94.264 Y79.707 E.01612
G1 X94.763 Y79.928 E.01611
G1 X95.252 Y80.174 E.01611
G1 X95.728 Y80.444 E.01612
G1 X96.19 Y80.736 E.01612
G1 X96.637 Y81.051 E.01612
G1 X97.068 Y81.388 E.01612
G1 X97.482 Y81.745 E.01611
G1 X97.878 Y82.122 E.01612
G1 X98.255 Y82.518 E.01612
G1 X98.612 Y82.932 E.01612
G1 X98.948 Y83.363 E.01611
G1 X99.264 Y83.81 E.01613
G1 X99.556 Y84.272 E.01612
G1 X99.826 Y84.748 E.01612
G1 X100.071 Y85.237 E.01611
G1 X100.293 Y85.736 E.01611
G1 X100.49 Y86.247 E.01612
G1 X100.661 Y86.766 E.01612
G1 X100.807 Y87.293 E.01612
G1 X100.927 Y87.826 E.01611
G1 X101.021 Y88.365 E.01612
G1 X101.088 Y88.908 E.01611
G1 X101.128 Y89.453 E.01612
G1 X101.141 Y90 E.01611
G1 X101.128 Y90.546 E.01612
G1 X101.088 Y91.092 E.01613
G1 X101.021 Y91.635 E.01612
G1 X100.927 Y92.174 E.01612
G1 X100.807 Y92.707 E.01611
G1 X100.661 Y93.234 E.01612
G1 X100.49 Y93.753 E.01612
G1 X100.293 Y94.264 E.01612
G1 X100.072 Y94.763 E.01611
G1 X99.826 Y95.252 E.01612
G1 X99.556 Y95.728 E.01612
G1 X99.263 Y96.19 E.01612
G1 X98.949 Y96.637 E.0161
G1 X98.612 Y97.068 E.01613
G1 X98.255 Y97.482 E.01612
G1 X97.878 Y97.878 E.01611
G1 X97.482 Y98.255 E.01613
G1 X97.068 Y98.612 E.01612
G1 X96.637 Y98.949 E.0161
G1 X96.19 Y99.264 E.01612
G1 X95.728 Y99.556 E.01612
G1 X95.252 Y99.826 E.01612
G1 X94.764 Y100.071 E.01611
G1 X94.264 Y100.293 E.01612
G1 X93.753 Y100.49 E.01611
G1 X93.234 Y100.661 E.01612
G1 X92.707 Y100.807 E.01612
G1 X92.173 Y100.927 E.01612
G1 X91.635 Y101.021 E.01611
G1 X91.092 Y101.087 E.01612
G1 X90.547 Y101.128 E.01612
G1 X90 Y101.141 E.01611
G1 X89.454 Y101.128 E.01611
G1 X88.908 Y101.088 E.01612
G1 X88.365 Y101.021 E.01612
G1 X87.826 Y100.927 E.01612
G1 X87.293 Y100.807 E.01611
G1 X86.766 Y100.661 E.01612
G1 X86.247 Y100.49 E.01612
G1 X85.737 Y100.293 E.01611
G1 X85.237 Y100.072 E.01612
G1 X84.748 Y99.826 E.01612
G1 X84.272 Y99.556 E.01612
G1 X83.81 Y99.264 E.01611
G1 X83.363 Y98.949 E.01612
G1 X82.932 Y98.612 E.01612
G1 X82.518 Y98.255 E.0161
G1 X82.122 Y97.878 E.01613
G1 X81.745 Y97.482 E.01612
G1 X81.388 Y97.068 E.01611
G1 X81.052 Y96.637 E.01611
G1 X80.736 Y96.19 E.01613
G1 X80.444 Y95.728 E.01611
G1 X80.174 Y95.252 E.01612
G1 X79.929 Y94.764 E.01611
G1 X79.707 Y94.263 E.01612
G1 X79.51 Y93.753 E.01611
G1 X79.339 Y93.234 E.01613
G1 X79.193 Y92.707 E.01611
G1 X79.073 Y92.174 E.01612
G1 X78.979 Y91.635 E.01612
G1 X78.913 Y91.092 E.01611
G1 X78.872 Y90.547 E.01612
G1 X78.859 Y90 E.01611
G1 X78.872 Y89.454 E.01612
G1 X78.912 Y88.908 E.01612
G1 X78.979 Y88.365 E.01612
G1 X79.073 Y87.826 E.01612
G1 X79.193 Y87.293 E.01611
G1 X79.339 Y86.766 E.01612
G1 X79.51 Y86.247 E.01611
G1 X79.707 Y85.737 E.01612
G1 X79.928 Y85.237 E.01612
G1 X80.174 Y84.748 E.01612
G1 X80.444 Y84.272 E.01612
G1 X80.736 Y83.81 E.01612
G1 X81.051 Y83.363 E.01612
G1 X81.388 Y82.932 E.01612
G1 X81.745 Y82.518 E.01611
G1 X82.122 Y82.122 E.01612
G1 X82.518 Y81.745 E.01612
G1 X82.932 Y81.388 E.01612
G1 X83.363 Y81.051 E.01612
G1 X83.81 Y80.737 E.01611
G1 X84.272 Y80.444 E.01612
G1 X84.748 Y80.174 E.01612
G1 X85.236 Y79.929 E.01612
G1 X85.736 Y79.707 E.01612
G1 X86.247 Y79.51 E.01612
G1 X86.766 Y79.339 E.01612
G1 X87.293 Y79.193 E.01612
G1 X87.827 Y79.073 E.01612
G1 X88.365 Y78.979 E.01611
G1 X88.908 Y78.913 E.01612
G1 X89.453 Y78.872 E.01612
G1 X90 Y78.859 E.01611
G1 X90.546 Y78.872 E.0161
G1 X91.092 Y78.912 E.01614
G1 X91.635 Y78.979 E.01612
G1 X92.173 Y79.073 E.01611
G1 X92.649 Y79.18 E.01435
M106 S124.95
; WIPE_START
G1 F6600
M204 S6000
G1 X93.234 Y79.339 E-.23059
G1 X93.607 Y79.462 E-.14941
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z7.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z7.9 F4000
            G39.3 S1
            G0 Z7.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X97.382 Y82.618 F42000
G1 Z7.5
G1 E.4 F1800
; Slow Down Start
; FEATURE: Floating vertical shell
; LINE_WIDTH: 0.38292
G1 F3000;_EXTRUDE_SET_SPEED
M204 S6000
G1 X97.735 Y82.989 E.01361
G1 X98.07 Y83.377 E.01363
G1 X98.385 Y83.781 E.01361
G1 X98.68 Y84.2 E.01362
G1 X98.954 Y84.633 E.0136
G1 X99.207 Y85.079 E.01363
G1 X99.437 Y85.537 E.01362
G1 X99.645 Y86.005 E.01362
G1 X99.829 Y86.483 E.0136
G1 X99.99 Y86.969 E.01362
G1 X100.127 Y87.464 E.01363
G1 X100.239 Y87.963 E.0136
G1 X100.327 Y88.468 E.01363
G1 X100.389 Y88.976 E.01361
G1 X100.427 Y89.488 E.01363
G1 X100.44 Y90 E.01361
G1 X100.427 Y90.514 E.01368
G1 X100.389 Y91.026 E.01363
G1 X100.326 Y91.534 E.01361
G1 X100.238 Y92.04 E.01363
G1 X100.126 Y92.539 E.01361
G1 X99.989 Y93.033 E.0136
G1 X99.829 Y93.519 E.01363
G1 X99.644 Y93.998 E.01363
G1 X99.436 Y94.466 E.01361
G1 X99.206 Y94.923 E.01361
G1 X98.953 Y95.369 E.01362
G1 X98.679 Y95.802 E.01363
G1 X98.384 Y96.221 E.01361
G1 X98.068 Y96.625 E.01361
G1 X97.734 Y97.013 E.01362
G1 X97.38 Y97.384 E.01362
G1 X97.009 Y97.737 E.01362
G1 X96.621 Y98.071 E.0136
G1 X96.217 Y98.387 E.01363
G1 X95.798 Y98.682 E.01361
G1 X95.365 Y98.956 E.01362
G1 X94.919 Y99.208 E.01362
G1 X94.462 Y99.438 E.0136
G1 X93.993 Y99.646 E.01364
G1 X93.514 Y99.83 E.01362
G1 X93.028 Y99.991 E.0136
G1 X92.533 Y100.128 E.01364
G1 X92.035 Y100.239 E.01358
G1 X91.529 Y100.327 E.01364
G1 X91.021 Y100.39 E.01361
G1 X90.509 Y100.427 E.01363
G1 X89.997 Y100.44 E.01361
G1 X89.486 Y100.427 E.0136
G1 X88.974 Y100.389 E.01363
G1 X88.466 Y100.326 E.01362
G1 X87.96 Y100.238 E.01363
G1 X87.461 Y100.126 E.0136
G1 X86.967 Y99.989 E.01362
G1 X86.48 Y99.828 E.01363
G1 X86.003 Y99.644 E.01361
G1 X85.534 Y99.436 E.01363
G1 X85.077 Y99.206 E.01361
G1 X84.63 Y98.953 E.01363
G1 X84.198 Y98.679 E.0136
G1 X83.779 Y98.384 E.01362
G1 X83.375 Y98.068 E.01362
G1 X82.987 Y97.733 E.01364
G1 X82.617 Y97.38 E.01359
G1 X82.263 Y97.009 E.01362
G1 X81.929 Y96.621 E.01362
G1 X81.613 Y96.217 E.01363
G1 X81.318 Y95.798 E.01362
G1 X81.044 Y95.365 E.01362
G1 X80.792 Y94.919 E.01362
G1 X80.562 Y94.462 E.0136
G1 X80.354 Y93.993 E.01363
G1 X80.17 Y93.514 E.01363
G1 X80.009 Y93.028 E.01361
G1 X79.873 Y92.534 E.01361
G1 X79.761 Y92.035 E.0136
G1 X79.673 Y91.529 E.01364
G1 X79.61 Y91.021 E.0136
G1 X79.573 Y90.509 E.01364
G1 X79.56 Y89.997 E.01361
G1 X79.573 Y89.485 E.01361
G1 X79.611 Y88.974 E.01362
G1 X79.674 Y88.466 E.01361
G1 X79.762 Y87.96 E.01363
G1 X79.874 Y87.461 E.01362
G1 X80.011 Y86.967 E.0136
G1 X80.171 Y86.481 E.01362
G1 X80.356 Y86.003 E.01362
G1 X80.564 Y85.534 E.01362
G1 X80.794 Y85.077 E.01361
G1 X81.047 Y84.631 E.01362
G1 X81.321 Y84.198 E.01362
G1 X81.616 Y83.779 E.0136
G1 X81.932 Y83.375 E.01363
G1 X82.267 Y82.987 E.01363
G1 X82.62 Y82.616 E.0136
G1 X82.991 Y82.263 E.01362
G1 X83.379 Y81.929 E.01361
G1 X83.784 Y81.613 E.01364
G1 X84.202 Y81.318 E.0136
G1 X84.635 Y81.044 E.01362
M73 P89 R1
G1 X85.081 Y80.792 E.01363
G1 X85.539 Y80.562 E.0136
G1 X86.007 Y80.354 E.01362
G1 X86.485 Y80.17 E.01361
G1 X86.972 Y80.009 E.01362
G1 X87.466 Y79.873 E.01362
G1 X87.966 Y79.761 E.0136
G1 X88.471 Y79.673 E.01363
G1 X88.979 Y79.61 E.01361
G1 X89.485 Y79.573 E.01347
G1 X90 Y79.564 E.01369
G1 X91.022 Y79.621 E.02721
G1 X91.532 Y79.673 E.01363
G1 X92.037 Y79.761 E.01361
G1 X92.536 Y79.873 E.01362
G1 X93.031 Y80.01 E.01362
G1 X93.517 Y80.171 E.01362
G1 X93.995 Y80.355 E.01361
G1 X94.463 Y80.563 E.01362
G1 X94.921 Y80.793 E.01361
G1 X95.367 Y81.046 E.01364
G1 X95.8 Y81.32 E.0136
G1 X96.219 Y81.615 E.01363
G1 X96.623 Y81.93 E.01362
G1 X97.338 Y82.577 E.02563
; Slow Down End
; WIPE_START
G1 X96.623 Y81.93 E-.36644
G1 X96.595 Y81.908 E-.01356
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z7.9 I-.176 J-1.204 P1  F42000
G1 X81.784 Y84.077 Z7.9
G1 Z7.5
G1 E.4 F1800
; FEATURE: Sparse infill
; LINE_WIDTH: 0.45
G1 F1690
M204 S6000
G1 X82.169 Y83.573 E.02017
G1 X82.494 Y83.197 E.01582
G1 X82.837 Y82.837 E.01581
G1 X97.163 Y97.163 E.64466
G1 X96.803 Y97.506 E.01583
G1 X96.427 Y97.831 E.0158
G1 X96.035 Y98.137 E.01584
G1 X95.628 Y98.423 E.01582
G1 X95.208 Y98.689 E.01581
G1 X94.775 Y98.934 E.01583
G1 X94.332 Y99.157 E.01579
G1 X93.876 Y99.359 E.01585
G1 X93.413 Y99.538 E.01581
G1 X92.941 Y99.694 E.01581
G1 X92.208 Y99.884 E.0241
G1 X80.116 Y87.792 E.54409
G1 X79.979 Y88.514 E.02337
G1 X79.918 Y89.007 E.01581
G1 X79.882 Y89.503 E.01583
G1 X79.87 Y90 E.01581
G1 X79.882 Y90.497 E.01581
G1 X79.918 Y90.993 E.01584
G1 X79.979 Y91.486 E.0158
G1 X80.116 Y92.208 E.02337
G1 X92.208 Y80.116 E.54409
G1 X91.487 Y79.979 E.02336
G1 X91.006 Y79.92 E.01542
G1 X90.17 Y79.874 E.02662
G1 X89.495 Y79.882 E.02149
G1 X89.007 Y79.918 E.01557
G1 X88.514 Y79.979 E.01581
G1 X87.792 Y80.116 E.02336
G1 X99.884 Y92.208 E.54409
G1 X100.021 Y91.486 E.02337
G1 X100.082 Y90.993 E.01581
G1 X100.118 Y90.497 E.01584
G1 X100.13 Y90 E.01581
G1 X100.118 Y89.503 E.01581
G1 X100.082 Y89.007 E.01584
G1 X100.021 Y88.514 E.01581
G1 X99.884 Y87.792 E.02337
G1 X87.792 Y99.884 E.54409
G1 X87.059 Y99.694 E.02408
G1 X86.587 Y99.538 E.01583
G1 X86.124 Y99.359 E.0158
G1 X85.668 Y99.158 E.01584
G1 X85.225 Y98.934 E.01581
G1 X84.792 Y98.689 E.01584
G1 X84.372 Y98.423 E.0158
G1 X83.965 Y98.137 E.01582
G1 X83.574 Y97.831 E.01581
G1 X83.196 Y97.506 E.01585
G1 X82.837 Y97.163 E.0158
G1 X97.163 Y82.837 E.64466
G1 X97.506 Y83.197 E.01582
G1 X97.831 Y83.573 E.01581
G1 X98.215 Y84.077 E.02017
M106 S127.5
; CHANGE_LAYER
; Z_HEIGHT: 7.7
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F9580.435
G1 X97.831 Y83.573 E-.24094
G1 X97.592 Y83.296 E-.13906
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 39/43
; update layer progress
M73 L39
M991 S0 P38 ;notify layer change
M106 S127.5
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z7.9 I.744 J-.963 P1  F42000
G1 X92.638 Y79.469 Z7.9
G1 Z7.7
G1 E.4 F1800
; FEATURE: Inner wall
G1 F9580.435
M204 S6000
G1 X93.151 Y79.611 E.01696
G1 X93.657 Y79.778 E.01695
G1 X94.154 Y79.97 E.01695
G1 X94.642 Y80.186 E.01696
G1 X95.117 Y80.426 E.01695
G1 X95.581 Y80.688 E.01696
G1 X96.032 Y80.974 E.01696
G1 X96.467 Y81.28 E.01694
G1 X96.887 Y81.608 E.01695
G1 X97.291 Y81.956 E.01696
G1 X97.676 Y82.324 E.01695
G1 X98.044 Y82.709 E.01695
G1 X98.392 Y83.113 E.01695
G1 X98.72 Y83.533 E.01697
G1 X99.027 Y83.969 E.01695
G1 X99.312 Y84.419 E.01695
G1 X99.574 Y84.882 E.01695
G1 X99.814 Y85.358 E.01695
G1 X100.03 Y85.846 E.01696
G1 X100.222 Y86.343 E.01695
G1 X100.389 Y86.849 E.01696
G1 X100.531 Y87.362 E.01695
G1 X100.648 Y87.882 E.01694
G1 X100.739 Y88.407 E.01697
G1 X100.804 Y88.936 E.01694
G1 X100.843 Y89.468 E.01697
G1 X100.856 Y90 E.01695
G1 X100.843 Y90.532 E.01694
G1 X100.804 Y91.064 E.01697
G1 X100.739 Y91.593 E.01694
G1 X100.648 Y92.118 E.01697
G1 X100.531 Y92.638 E.01695
G1 X100.389 Y93.151 E.01694
G1 X100.222 Y93.657 E.01696
G1 X100.03 Y94.155 E.01696
G1 X99.814 Y94.642 E.01695
G1 X99.574 Y95.117 E.01695
G1 X99.312 Y95.581 E.01696
G1 X99.027 Y96.031 E.01695
G1 X98.72 Y96.467 E.01695
G1 X98.392 Y96.887 E.01695
G1 X98.044 Y97.291 E.01696
G1 X97.676 Y97.677 E.01696
G1 X97.29 Y98.044 E.01696
G1 X96.887 Y98.392 E.01694
G1 X96.467 Y98.72 E.01697
G1 X96.031 Y99.027 E.01695
G1 X95.581 Y99.312 E.01695
G1 X95.117 Y99.574 E.01696
G1 X94.642 Y99.814 E.01694
G1 X94.154 Y100.03 E.01697
G1 X93.657 Y100.222 E.01695
G1 X93.151 Y100.389 E.01695
G1 X92.638 Y100.531 E.01696
G1 X92.118 Y100.648 E.01694
G1 X91.593 Y100.739 E.01697
G1 X91.064 Y100.804 E.01694
G1 X90.533 Y100.843 E.01697
G1 X90 Y100.856 E.01696
G1 X89.468 Y100.843 E.01694
G1 X88.936 Y100.804 E.01697
G1 X88.407 Y100.739 E.01696
G1 X87.882 Y100.648 E.01694
G1 X87.362 Y100.531 E.01696
G1 X86.849 Y100.389 E.01695
G1 X86.343 Y100.222 E.01696
G1 X85.846 Y100.03 E.01695
G1 X85.358 Y99.814 E.01695
G1 X84.882 Y99.574 E.01695
G1 X84.419 Y99.312 E.01696
G1 X83.969 Y99.027 E.01695
G1 X83.533 Y98.72 E.01695
G1 X83.113 Y98.392 E.01695
G1 X82.709 Y98.044 E.01696
G1 X82.324 Y97.677 E.01695
G1 X81.956 Y97.291 E.01696
G1 X81.608 Y96.887 E.01695
G1 X81.28 Y96.467 E.01695
G1 X80.973 Y96.031 E.01696
G1 X80.688 Y95.581 E.01695
G1 X80.426 Y95.118 E.01695
G1 X80.186 Y94.642 E.01696
G1 X79.97 Y94.154 E.01696
G1 X79.778 Y93.657 E.01695
G1 X79.611 Y93.151 E.01696
G1 X79.469 Y92.638 E.01695
G1 X79.352 Y92.118 E.01695
G1 X79.261 Y91.593 E.01697
G1 X79.196 Y91.064 E.01694
G1 X79.157 Y90.532 E.01697
G1 X79.144 Y90 E.01694
G1 X79.157 Y89.467 E.01696
G1 X79.196 Y88.936 E.01695
G1 X79.261 Y88.407 E.01695
G1 X79.352 Y87.882 E.01696
G1 X79.469 Y87.362 E.01695
G1 X79.611 Y86.849 E.01695
G1 X79.778 Y86.343 E.01696
G1 X79.97 Y85.846 E.01695
G1 X80.186 Y85.358 E.01696
G1 X80.426 Y84.882 E.01695
G1 X80.688 Y84.419 E.01696
G1 X80.974 Y83.969 E.01696
G1 X81.28 Y83.533 E.01694
G1 X81.608 Y83.113 E.01696
G1 X81.956 Y82.709 E.01695
G1 X82.324 Y82.323 E.01696
G1 X82.709 Y81.956 E.01694
G1 X83.113 Y81.608 E.01694
G1 X83.533 Y81.28 E.01698
G1 X83.969 Y80.974 E.01694
G1 X84.419 Y80.688 E.01695
G1 X84.883 Y80.426 E.01696
G1 X85.358 Y80.186 E.01695
G1 X85.846 Y79.97 E.01696
G1 X86.343 Y79.778 E.01695
G1 X86.849 Y79.611 E.01695
G1 X87.362 Y79.469 E.01696
G1 X87.882 Y79.352 E.01695
G1 X88.407 Y79.261 E.01695
G1 X88.936 Y79.196 E.01695
G1 X89.464 Y79.157 E.01685
G1 X90.185 Y79.149 E.02296
G1 X91.07 Y79.197 E.02819
G1 X91.593 Y79.261 E.01677
G1 X92.118 Y79.352 E.01695
G1 X92.579 Y79.456 E.01504
; COOLING_NODE: 5
M204 S250
G1 X92.733 Y79.089 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F1948
M204 S5000
G1 X93.265 Y79.236 E.01627
G1 X93.789 Y79.409 E.01627
G1 X94.304 Y79.608 E.01627
G1 X94.809 Y79.832 E.01628
G1 X95.302 Y80.08 E.01627
G1 X95.783 Y80.352 E.01627
G1 X96.249 Y80.647 E.01628
G1 X96.7 Y80.965 E.01626
G1 X97.136 Y81.305 E.01627
G1 X97.554 Y81.666 E.01628
G1 X97.954 Y82.046 E.01627
G1 X98.334 Y82.446 E.01627
G1 X98.695 Y82.864 E.01627
G1 X99.035 Y83.299 E.01628
G1 X99.353 Y83.751 E.01627
G1 X99.648 Y84.217 E.01627
G1 X99.92 Y84.698 E.01627
G1 X100.168 Y85.191 E.01627
G1 X100.392 Y85.695 E.01628
G1 X100.591 Y86.211 E.01627
G1 X100.764 Y86.735 E.01628
G1 X100.911 Y87.267 E.01626
G1 X101.032 Y87.805 E.01627
G1 X101.127 Y88.35 E.01628
G1 X101.194 Y88.897 E.01627
G1 X101.235 Y89.448 E.01628
G1 X101.248 Y90 E.01627
G1 X101.235 Y90.552 E.01627
G1 X101.194 Y91.103 E.01628
G1 X101.127 Y91.65 E.01627
G1 X101.032 Y92.195 E.01628
G1 X100.911 Y92.733 E.01628
G1 X100.764 Y93.265 E.01626
G1 X100.591 Y93.789 E.01628
G1 X100.392 Y94.305 E.01628
G1 X100.168 Y94.809 E.01627
G1 X99.92 Y95.302 E.01627
G1 X99.648 Y95.783 E.01628
G1 X99.353 Y96.249 E.01627
G1 X99.035 Y96.701 E.01627
G1 X98.695 Y97.136 E.01627
G1 X98.335 Y97.554 E.01627
G1 X97.954 Y97.954 E.01628
G1 X97.554 Y98.335 E.01627
G1 X97.136 Y98.695 E.01626
G1 X96.701 Y99.035 E.01628
G1 X96.249 Y99.353 E.01627
G1 X95.783 Y99.648 E.01627
G1 X95.302 Y99.92 E.01628
G1 X94.809 Y100.168 E.01626
G1 X94.304 Y100.392 E.01628
G1 X93.789 Y100.591 E.01627
G1 X93.265 Y100.764 E.01627
G1 X92.733 Y100.911 E.01627
G1 X92.195 Y101.032 E.01627
G1 X91.65 Y101.127 E.01628
G1 X91.103 Y101.194 E.01627
G1 X90.552 Y101.235 E.01628
G1 X90 Y101.248 E.01627
G1 X89.448 Y101.235 E.01627
G1 X88.897 Y101.194 E.01628
G1 X88.349 Y101.127 E.01627
G1 X87.806 Y101.032 E.01626
G1 X87.267 Y100.911 E.01628
G1 X86.735 Y100.764 E.01626
G1 X86.211 Y100.591 E.01628
G1 X85.695 Y100.392 E.01627
G1 X85.191 Y100.168 E.01628
G1 X84.698 Y99.92 E.01627
G1 X84.217 Y99.648 E.01627
G1 X83.751 Y99.353 E.01628
G1 X83.3 Y99.035 E.01626
G1 X82.864 Y98.695 E.01627
G1 X82.446 Y98.334 E.01628
G1 X82.046 Y97.954 E.01626
G1 X81.665 Y97.554 E.01628
G1 X81.305 Y97.136 E.01627
G1 X80.965 Y96.701 E.01627
G1 X80.647 Y96.249 E.01628
G1 X80.352 Y95.783 E.01627
G1 X80.08 Y95.302 E.01627
G1 X79.832 Y94.809 E.01627
G1 X79.608 Y94.304 E.01628
G1 X79.409 Y93.789 E.01627
G1 X79.236 Y93.265 E.01628
G1 X79.089 Y92.733 E.01626
G1 X78.968 Y92.195 E.01627
G1 X78.873 Y91.65 E.01628
G1 X78.806 Y91.103 E.01627
G1 X78.765 Y90.552 E.01628
G1 X78.752 Y90 E.01627
G1 X78.765 Y89.448 E.01627
G1 X78.806 Y88.897 E.01628
G1 X78.873 Y88.35 E.01627
G1 X78.968 Y87.805 E.01628
G1 X79.089 Y87.267 E.01627
G1 X79.236 Y86.735 E.01626
G1 X79.409 Y86.211 E.01628
G1 X79.608 Y85.695 E.01627
G1 X79.832 Y85.191 E.01628
G1 X80.08 Y84.698 E.01627
G1 X80.352 Y84.217 E.01627
G1 X80.647 Y83.751 E.01628
G1 X80.965 Y83.3 E.01626
G1 X81.305 Y82.864 E.01628
G1 X81.665 Y82.446 E.01627
G1 X82.046 Y82.046 E.01628
G1 X82.446 Y81.666 E.01626
G1 X82.864 Y81.305 E.01626
G1 X83.3 Y80.965 E.01629
G1 X83.751 Y80.647 E.01626
G1 X84.217 Y80.352 E.01627
G1 X84.698 Y80.08 E.01628
G1 X85.191 Y79.832 E.01627
G1 X85.695 Y79.608 E.01628
G1 X86.211 Y79.409 E.01627
G1 X86.735 Y79.236 E.01627
G1 X87.267 Y79.089 E.01627
G1 X87.806 Y78.968 E.01627
G1 X88.349 Y78.873 E.01627
G1 X88.897 Y78.806 E.01627
G1 X89.447 Y78.765 E.01624
M106 S104.55
M106 S127.5
G1 X89.847 Y78.761 E.01179
M106 S104.55
M106 S127.5
G1 X90.194 Y78.757 E.01023
M106 S104.55
M106 S127.5
G1 X90.593 Y78.778 E.01179
M106 S104.55
M106 S127.5
G1 X91.105 Y78.806 E.01509
M106 S104.55
M106 S127.5
G1 X91.651 Y78.873 E.01622
G1 X92.194 Y78.968 E.01626
G1 X92.675 Y79.076 E.01451
M106 S104.55
M106 S127.5
; WIPE_START
G1 F7500
M204 S6000
G1 X93.265 Y79.236 E-.23256
G1 X93.634 Y79.358 E-.14744
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.1 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z8.1
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z8.1 F4000
            G39.3 S1
            G0 Z8.1 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X84.062 Y81.112 F42000
G1 Z7.7
G1 E.4 F1800
; FEATURE: Bridge
; LINE_WIDTH: 0.4058
; LAYER_HEIGHT: 0.4
G1 F3000
M204 S6000
G1 X82.502 Y82.672 E.11152
G1 X81.894 Y83.347 E.04592
G1 X81.577 Y83.754 E.02604
G1 X81.281 Y84.174 E.02599
G1 X81.006 Y84.609 E.02603
G1 X80.742 Y85.077 E.02716
G1 X85.077 Y80.742 E.30992
G1 X85.516 Y80.521 E.02485
G1 X86.255 Y80.209 E.04052
G1 X80.209 Y86.255 E.43219
G1 X79.965 Y86.956 E.03752
G1 X79.894 Y87.214 E.01355
G1 X87.214 Y79.894 E.52328
G1 X88.055 Y79.698 E.04363
G1 X79.698 Y88.055 E.59739
G1 X79.584 Y88.813 E.03877
G1 X88.814 Y79.584 E.65975
G1 X89.515 Y79.526 E.0356
G1 X79.526 Y89.516 E.71409
G1 X79.518 Y90.169 E.03297
G1 X90.168 Y79.519 E.76127
G1 X90.78 Y79.551 E.03097
G1 X79.546 Y90.785 E.80298
G1 X79.606 Y91.369 E.0297
G1 X91.369 Y79.606 E.84083
G1 X91.926 Y79.694 E.02848
G1 X79.694 Y91.926 E.87432
G1 X79.808 Y92.457 E.02746
G1 X92.457 Y79.808 E.90421
G1 X92.966 Y79.944 E.02661
G1 X79.944 Y92.966 E.93084
G1 X80.101 Y93.454 E.02589
G1 X93.454 Y80.101 E.95449
G1 X93.922 Y80.277 E.02529
G1 X80.277 Y93.922 E.97536
G1 X80.471 Y94.372 E.02479
G1 X94.372 Y80.471 E.99366
G1 X94.805 Y80.683 E.02436
G1 X80.683 Y94.805 E1.00951
G1 X80.91 Y95.222 E.02401
G1 X95.222 Y80.91 E1.02306
G1 X95.624 Y81.153 E.02373
G1 X81.153 Y95.624 E1.03439
G1 X81.281 Y95.826 E.01207
G1 X81.411 Y96.011 E.01143
G1 X96.011 Y81.411 E1.04359
G1 X96.383 Y81.684 E.02331
G1 X81.684 Y96.383 E1.05071
G1 X81.97 Y96.741 E.02318
G1 X96.741 Y81.97 E1.05581
G1 X97.085 Y82.271 E.02309
G1 X82.271 Y97.085 E1.05893
G1 X82.585 Y97.415 E.02305
G1 X97.415 Y82.585 E1.06006
G1 X97.729 Y82.915 E.02305
G1 X82.915 Y97.729 E1.05893
G1 X83.259 Y98.03 E.02309
G1 X98.03 Y83.259 E1.05582
G1 X98.317 Y83.617 E.02318
G1 X83.617 Y98.317 E1.05072
G1 X83.989 Y98.589 E.02331
G1 X98.589 Y83.989 E1.04359
G1 X98.847 Y84.376 E.02349
G1 X84.376 Y98.847 E1.03439
G1 X84.778 Y99.09 E.02373
G1 X99.09 Y84.778 E1.02306
G1 X99.318 Y85.195 E.02401
G1 X85.195 Y99.318 E1.00951
G1 X85.628 Y99.529 E.02436
G1 X99.529 Y85.628 E.99365
G1 X99.723 Y86.078 E.02479
G1 X86.078 Y99.723 E.97536
G1 X86.547 Y99.9 E.02529
G1 X99.899 Y86.547 E.95448
G1 X100.056 Y87.034 E.02589
G1 X87.034 Y100.056 E.93084
G1 X87.543 Y100.192 E.02661
G1 X100.192 Y87.543 E.90421
G1 X100.306 Y88.074 E.02746
G1 X88.074 Y100.306 E.87432
G1 X88.631 Y100.394 E.02848
G1 X100.394 Y88.631 E.84083
G1 X100.454 Y89.215 E.0297
G1 X89.215 Y100.454 E.80333
G1 X89.831 Y100.482 E.03117
G1 X100.482 Y89.831 E.76133
G1 X100.474 Y90.484 E.03298
G1 X90.484 Y100.474 E.71414
G1 X91.187 Y100.416 E.03565
G1 X100.416 Y91.187 E.65974
G1 X100.302 Y91.945 E.03877
G1 X91.945 Y100.302 E.59738
G1 X92.786 Y100.106 E.04363
G1 X100.106 Y92.786 E.52327
G1 X100.035 Y93.044 E.01352
G1 X99.791 Y93.745 E.03754
G1 X93.745 Y99.791 E.43218
G1 X94.484 Y99.479 E.04053
M73 P90 R1
G1 X94.923 Y99.258 E.02484
G1 X99.258 Y94.923 E.3099
G1 X98.994 Y95.391 E.02716
G1 X98.719 Y95.826 E.02602
G1 X98.423 Y96.247 E.02601
G1 X98.106 Y96.652 E.02601
G1 X97.496 Y97.33 E.04607
G1 X95.938 Y98.888 E.11136
M106 S104.55
M106 S127.5
; CHANGE_LAYER
; Z_HEIGHT: 7.9
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F3000
G1 X96.645 Y98.181 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 40/43
; update layer progress
M73 L40
M991 S0 P39 ;notify layer change
M106 S127.5
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z8.1 I1.191 J-.252 P1  F42000
G1 X92.663 Y79.366 Z8.1
G1 Z7.9
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
G1 F5979
M204 S6000
G1 X93.182 Y79.51 E.01713
G1 X93.693 Y79.679 E.01712
G1 X94.195 Y79.872 E.01712
G1 X94.687 Y80.09 E.01713
G1 X95.167 Y80.332 E.01711
G1 X95.636 Y80.597 E.01712
G1 X96.09 Y80.885 E.01712
G1 X96.53 Y81.195 E.01712
G1 X96.954 Y81.526 E.01712
G1 X97.362 Y81.878 E.01712
G1 X97.751 Y82.249 E.01712
G1 X98.123 Y82.638 E.01713
G1 X98.474 Y83.046 E.01711
G1 X98.805 Y83.47 E.01712
G1 X99.115 Y83.91 E.01712
G1 X99.403 Y84.364 E.01712
G1 X99.668 Y84.833 E.01712
G1 X99.91 Y85.313 E.01712
G1 X100.128 Y85.805 E.01711
G1 X100.321 Y86.307 E.01712
G1 X100.49 Y86.818 E.01713
G1 X100.634 Y87.336 E.0171
G1 X100.752 Y87.862 E.01714
G1 X100.843 Y88.391 E.01711
G1 X100.909 Y88.925 E.01711
G1 X100.949 Y89.462 E.01714
G1 X100.962 Y90 E.01711
G1 X100.949 Y90.538 E.01711
G1 X100.909 Y91.075 E.01714
G1 X100.844 Y91.608 E.01711
G1 X100.751 Y92.139 E.01714
G1 X100.634 Y92.664 E.01711
G1 X100.49 Y93.182 E.01711
G1 X100.321 Y93.693 E.01713
G1 X100.128 Y94.195 E.01712
G1 X99.91 Y94.687 E.01712
G1 X99.668 Y95.168 E.01712
G1 X99.403 Y95.636 E.01712
G1 X99.115 Y96.09 E.01712
G1 X98.805 Y96.53 E.01713
G1 X98.474 Y96.954 E.01711
G1 X98.123 Y97.362 E.01711
G1 X97.751 Y97.751 E.01712
G1 X97.362 Y98.123 E.01713
G1 X96.954 Y98.474 E.01711
G1 X96.53 Y98.805 E.01713
G1 X96.09 Y99.115 E.01711
G1 X95.636 Y99.403 E.01712
G1 X95.168 Y99.668 E.01712
G1 X94.687 Y99.91 E.01711
G1 X94.195 Y100.128 E.01713
G1 X93.693 Y100.321 E.01712
M73 P91 R1
G1 X93.182 Y100.49 E.01711
G1 X92.664 Y100.634 E.01712
G1 X92.138 Y100.752 E.01713
G1 X91.609 Y100.843 E.01711
G1 X91.075 Y100.909 E.01711
G1 X90.538 Y100.949 E.01713
G1 X90 Y100.962 E.01712
G1 X89.462 Y100.949 E.0171
G1 X88.925 Y100.909 E.01714
G1 X88.391 Y100.843 E.01711
G1 X87.861 Y100.752 E.01711
G1 X87.336 Y100.634 E.01712
G1 X86.818 Y100.49 E.01712
G1 X86.307 Y100.321 E.01712
G1 X85.805 Y100.128 E.01712
G1 X85.313 Y99.91 E.01713
G1 X84.833 Y99.668 E.01711
G1 X84.364 Y99.403 E.01712
G1 X83.91 Y99.115 E.01712
G1 X83.47 Y98.805 E.01712
G1 X83.046 Y98.474 E.01712
G1 X82.638 Y98.122 E.01712
G1 X82.248 Y97.751 E.01713
G1 X81.878 Y97.362 E.01711
G1 X81.526 Y96.954 E.01712
G1 X81.195 Y96.53 E.01712
G1 X80.885 Y96.09 E.01712
G1 X80.597 Y95.636 E.01712
G1 X80.332 Y95.167 E.01712
G1 X80.09 Y94.687 E.01712
G1 X79.872 Y94.195 E.01713
G1 X79.679 Y93.693 E.01712
G1 X79.51 Y93.182 E.01712
G1 X79.366 Y92.664 E.01712
G1 X79.249 Y92.139 E.01711
G1 X79.156 Y91.608 E.01713
G1 X79.091 Y91.074 E.01712
G1 X79.051 Y90.538 E.01711
G1 X79.038 Y90 E.01713
G1 X79.051 Y89.462 E.01712
G1 X79.091 Y88.925 E.01713
G1 X79.156 Y88.392 E.01711
G1 X79.249 Y87.861 E.01713
G1 X79.366 Y87.336 E.01711
G1 X79.51 Y86.818 E.01712
G1 X79.679 Y86.307 E.01712
G1 X79.872 Y85.805 E.01711
G1 X80.09 Y85.313 E.01713
G1 X80.332 Y84.833 E.01711
G1 X80.598 Y84.364 E.01714
G1 X80.885 Y83.91 E.01711
G1 X81.195 Y83.47 E.01712
G1 X81.526 Y83.046 E.01713
G1 X81.877 Y82.638 E.01711
G1 X82.249 Y82.249 E.01713
G1 X82.638 Y81.877 E.01712
G1 X83.046 Y81.526 E.01711
G1 X83.47 Y81.195 E.01712
G1 X83.91 Y80.885 E.01713
G1 X84.364 Y80.598 E.01711
G1 X84.833 Y80.332 E.01712
G1 X85.313 Y80.09 E.01712
G1 X85.805 Y79.872 E.01712
G1 X86.307 Y79.679 E.01713
G1 X86.818 Y79.51 E.01711
G1 X87.336 Y79.366 E.01712
G1 X87.861 Y79.249 E.01711
G1 X88.392 Y79.156 E.01713
G1 X88.925 Y79.091 E.01711
G1 X89.457 Y79.051 E.01695
G1 X90.347 Y79.047 E.02832
G1 X91.078 Y79.091 E.02333
G1 X91.609 Y79.157 E.017
G1 X92.139 Y79.248 E.01711
G1 X92.605 Y79.353 E.01521
; COOLING_NODE: 5
M204 S250
G1 X92.759 Y78.986 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F2102
M204 S5000
G1 X93.296 Y79.135 E.01642
G1 X93.825 Y79.309 E.01643
G1 X94.345 Y79.51 E.01642
G1 X94.855 Y79.736 E.01643
G1 X95.352 Y79.986 E.01642
G1 X95.837 Y80.261 E.01642
G1 X96.308 Y80.559 E.01642
G1 X96.764 Y80.88 E.01643
G1 X97.203 Y81.223 E.01642
G1 X97.625 Y81.587 E.01642
G1 X98.029 Y81.971 E.01643
G1 X98.413 Y82.375 E.01643
G1 X98.777 Y82.797 E.01641
G1 X99.12 Y83.236 E.01643
G1 X99.441 Y83.692 E.01643
G1 X99.739 Y84.163 E.01642
G1 X100.014 Y84.648 E.01643
G1 X100.264 Y85.145 E.01642
G1 X100.49 Y85.655 E.01642
G1 X100.691 Y86.175 E.01643
G1 X100.865 Y86.704 E.01643
G1 X101.014 Y87.241 E.01641
G1 X101.136 Y87.785 E.01643
G1 X101.231 Y88.334 E.01642
G1 X101.3 Y88.887 E.01643
G1 X101.341 Y89.443 E.01643
G1 X101.354 Y90 E.01642
G1 X101.341 Y90.557 E.01642
G1 X101.3 Y91.113 E.01643
G1 X101.231 Y91.666 E.01642
G1 X101.136 Y92.215 E.01643
G1 X101.014 Y92.759 E.01643
G1 X100.865 Y93.296 E.01641
G1 X100.691 Y93.825 E.01643
G1 X100.49 Y94.345 E.01643
G1 X100.264 Y94.855 E.01643
G1 X100.014 Y95.352 E.01642
G1 X99.739 Y95.837 E.01642
G1 X99.441 Y96.308 E.01642
G1 X99.12 Y96.764 E.01643
G1 X98.777 Y97.203 E.01643
G1 X98.413 Y97.625 E.01642
G1 X98.029 Y98.029 E.01643
G1 X97.625 Y98.413 E.01643
G1 X97.203 Y98.777 E.01641
G1 X96.764 Y99.12 E.01643
G1 X96.308 Y99.441 E.01642
G1 X95.837 Y99.739 E.01643
G1 X95.352 Y100.014 E.01642
G1 X94.855 Y100.264 E.01642
G1 X94.345 Y100.49 E.01643
G1 X93.825 Y100.691 E.01642
G1 X93.296 Y100.865 E.01643
G1 X92.759 Y101.014 E.01643
G1 X92.215 Y101.136 E.01643
G1 X91.666 Y101.231 E.01642
G1 X91.113 Y101.3 E.01643
G1 X90.557 Y101.341 E.01643
G1 X90 Y101.354 E.01642
G1 X89.443 Y101.341 E.01642
G1 X88.887 Y101.3 E.01643
G1 X88.334 Y101.231 E.01643
G1 X87.785 Y101.136 E.01642
G1 X87.241 Y101.014 E.01644
G1 X86.704 Y100.865 E.01641
G1 X86.175 Y100.691 E.01643
G1 X85.655 Y100.49 E.01642
G1 X85.145 Y100.264 E.01643
G1 X84.648 Y100.014 E.01642
G1 X84.163 Y99.739 E.01642
G1 X83.692 Y99.441 E.01642
G1 X83.236 Y99.12 E.01643
G1 X82.797 Y98.777 E.01642
G1 X82.375 Y98.413 E.01642
G1 X81.971 Y98.029 E.01643
G1 X81.587 Y97.625 E.01642
G1 X81.223 Y97.203 E.01643
G1 X80.88 Y96.764 E.01642
G1 X80.559 Y96.308 E.01643
G1 X80.261 Y95.837 E.01642
G1 X79.986 Y95.352 E.01643
G1 X79.736 Y94.855 E.01642
G1 X79.51 Y94.345 E.01642
G1 X79.309 Y93.825 E.01643
G1 X79.135 Y93.296 E.01643
G1 X78.986 Y92.759 E.01641
G1 X78.864 Y92.215 E.01643
G1 X78.769 Y91.666 E.01643
G1 X78.7 Y91.113 E.01643
G1 X78.659 Y90.557 E.01642
G1 X78.646 Y90 E.01643
G1 X78.659 Y89.443 E.01642
G1 X78.7 Y88.887 E.01643
G1 X78.769 Y88.334 E.01642
G1 X78.864 Y87.785 E.01643
G1 X78.986 Y87.241 E.01642
G1 X79.135 Y86.704 E.01642
G1 X79.309 Y86.175 E.01643
G1 X79.51 Y85.655 E.01642
G1 X79.736 Y85.145 E.01643
G1 X79.986 Y84.648 E.01642
G1 X80.261 Y84.163 E.01643
G1 X80.559 Y83.692 E.01642
G1 X80.88 Y83.236 E.01643
G1 X81.223 Y82.797 E.01642
G1 X81.587 Y82.375 E.01642
G1 X81.971 Y81.971 E.01643
G1 X82.375 Y81.587 E.01643
G1 X82.797 Y81.223 E.01642
G1 X83.236 Y80.88 E.01643
G1 X83.692 Y80.559 E.01643
G1 X84.163 Y80.261 E.01642
G1 X84.648 Y79.986 E.01642
G1 X85.145 Y79.736 E.01643
G1 X85.655 Y79.51 E.01642
G1 X86.175 Y79.309 E.01643
G1 X86.704 Y79.135 E.01643
G1 X87.241 Y78.986 E.01642
G1 X87.785 Y78.864 E.01642
G1 X88.334 Y78.769 E.01643
G1 X88.887 Y78.7 E.01642
G1 X89.441 Y78.659 E.01637
M106 S122.4
M106 S127.5
G1 X89.841 Y78.657 E.01179
M106 S122.4
M106 S127.5
G1 X90.357 Y78.655 E.01522
M106 S122.4
M106 S127.5
G1 X90.715 Y78.676 E.01056
M106 S122.4
M106 S127.5
G1 X91.114 Y78.701 E.01179
M106 S122.4
M106 S127.5
G1 X91.666 Y78.769 E.01639
G1 X92.215 Y78.864 E.01642
G1 X92.7 Y78.973 E.01466
M106 S122.4
; WIPE_START
G1 F7500
M204 S6000
G1 X93.296 Y79.135 E-.23454
G1 X93.659 Y79.255 E-.14546
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.3 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z8.3
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z8.3 F4000
            G39.3 S1
            G0 Z8.3 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X98.856 Y83.826 F42000
G1 Z7.9
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.42247
G1 F5979
M204 S6000
G1 X97.479 Y82.449 E.05775
G1 X97.138 Y82.125 E.01397
G1 X96.743 Y81.784 E.01548
G1 X96.331 Y81.463 E.01548
G1 X95.905 Y81.163 E.01547
G1 X95.261 Y80.768 E.0224
G1 X99.232 Y84.739 E.16657
G1 X99.608 Y85.456 E.02402
G1 X99.765 Y85.809 E.01146
G1 X94.191 Y80.235 E.23383
G1 X93.329 Y79.909 E.02736
G1 X100.091 Y86.671 E.2837
G1 X100.313 Y87.431 E.02347
G1 X92.569 Y79.687 E.32489
G1 X91.889 Y79.543 E.02062
G1 X100.457 Y88.111 E.35943
G1 X100.551 Y88.742 E.01893
G1 X91.258 Y79.449 E.38987
G1 X90.673 Y79.4 E.01743
G1 X100.605 Y89.333 E.41671
G1 X100.626 Y89.891 E.01655
G1 X90.117 Y79.381 E.44091
G1 X89.583 Y79.384 E.01584
G1 X100.619 Y90.42 E.46299
G1 X100.586 Y90.924 E.015
G1 X89.076 Y79.414 E.48292
G1 X88.593 Y79.467 E.01442
G1 X100.533 Y91.407 E.50093
G1 X100.46 Y91.871 E.01393
G1 X88.129 Y79.54 E.51733
G1 X87.682 Y79.63 E.01352
G1 X100.37 Y92.318 E.5323
G1 X100.264 Y92.749 E.01317
G1 X87.251 Y79.736 E.54597
G1 X86.834 Y79.856 E.01287
G1 X100.144 Y93.166 E.55844
G1 X100.011 Y93.57 E.01261
G1 X86.43 Y79.989 E.56977
G1 X86.043 Y80.138 E.01232
G1 X99.862 Y93.958 E.57979
G1 X99.702 Y94.334 E.01214
G1 X85.666 Y80.298 E.58885
G1 X85.301 Y80.47 E.01197
G1 X99.53 Y94.699 E.59699
G1 X99.349 Y95.054 E.01183
G1 X84.946 Y80.651 E.60427
G1 X84.603 Y80.845 E.01168
G1 X99.155 Y95.397 E.61051
G1 X98.949 Y95.729 E.01157
G1 X84.272 Y81.051 E.6158
G1 X83.949 Y81.265 E.01148
G1 X98.735 Y96.051 E.62032
G1 X98.512 Y96.364 E.01142
G1 X83.636 Y81.488 E.62411
G1 X83.334 Y81.724 E.01134
G1 X98.276 Y96.666 E.62688
G1 X98.031 Y96.957 E.0113
G1 X83.043 Y81.969 E.62883
G1 X82.759 Y82.222 E.01128
G1 X97.778 Y97.241 E.63008
G1 X97.516 Y97.516 E.01126
G1 X82.484 Y82.484 E.63063
G1 X82.223 Y82.759 E.01126
G1 X97.241 Y97.777 E.63008
G1 X96.958 Y98.031 E.01128
G1 X81.969 Y83.042 E.62883
G1 X81.724 Y83.334 E.0113
G1 X96.666 Y98.276 E.62688
G1 X96.365 Y98.511 E.01135
G1 X81.489 Y83.635 E.62411
G1 X81.265 Y83.949 E.01142
G1 X96.051 Y98.735 E.62033
G1 X95.905 Y98.837 E.00529
G1 X95.729 Y98.949 E.00619
G1 X81.051 Y84.271 E.6158
G1 X80.845 Y84.603 E.01157
G1 X95.397 Y99.155 E.61051
G1 X95.055 Y99.349 E.01168
G1 X80.651 Y84.945 E.60428
G1 X80.47 Y85.3 E.01183
G1 X94.7 Y99.53 E.597
G1 X94.334 Y99.702 E.01197
G1 X80.298 Y85.666 E.58885
G1 X80.138 Y86.042 E.01214
G1 X93.958 Y99.862 E.5798
G1 X93.57 Y100.011 E.01232
G1 X79.989 Y86.43 E.56978
G1 X79.856 Y86.833 E.01261
G1 X93.167 Y100.144 E.55845
M73 P92 R1
G1 X92.75 Y100.264 E.01287
G1 X79.736 Y87.25 E.54598
G1 X79.63 Y87.682 E.01317
G1 X92.318 Y100.37 E.53231
G1 X91.872 Y100.46 E.01352
G1 X79.54 Y88.128 E.51735
G1 X79.468 Y88.592 E.01393
G1 X91.408 Y100.532 E.50094
G1 X90.925 Y100.586 E.01442
G1 X79.414 Y89.075 E.48293
G1 X79.382 Y89.58 E.015
G1 X90.42 Y100.618 E.46312
G1 X89.891 Y100.626 E.0157
G1 X79.374 Y90.109 E.44124
G1 X79.395 Y90.667 E.01655
G1 X89.333 Y100.605 E.41697
G1 X88.742 Y100.551 E.01761
G1 X79.449 Y91.258 E.3899
G1 X79.543 Y91.889 E.01893
G1 X88.111 Y100.457 E.35946
G1 X87.431 Y100.313 E.02062
G1 X79.687 Y92.569 E.32492
G1 X79.909 Y93.328 E.02347
G1 X86.672 Y100.091 E.28374
G1 X85.81 Y99.765 E.02735
G1 X80.235 Y94.19 E.23389
G1 X80.392 Y94.544 E.01148
G1 X80.768 Y95.26 E.02399
G1 X84.74 Y99.232 E.16664
G1 X84.095 Y98.837 E.02244
G1 X83.669 Y98.537 E.01546
G1 X83.257 Y98.216 E.01548
G1 X82.531 Y97.56 E.02905
G1 X81.143 Y96.172 E.05821
M106 S127.5
; CHANGE_LAYER
; Z_HEIGHT: 8.1
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F10275.326
G1 X81.85 Y96.88 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 41/43
; update layer progress
M73 L41
M991 S0 P40 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z8.3 I1.036 J.638 P1  F42000
G1 X92.688 Y79.267 Z8.3
G1 Z8.1
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
G1 F6017
M204 S6000
G1 X93.212 Y79.412 E.01728
G1 X93.727 Y79.583 E.01728
G1 X94.234 Y79.778 E.01728
G1 X94.731 Y79.998 E.01728
G1 X95.216 Y80.242 E.01728
G1 X95.688 Y80.51 E.01729
G1 X96.147 Y80.801 E.01727
G1 X96.591 Y81.113 E.01727
G1 X97.019 Y81.447 E.01729
G1 X97.43 Y81.802 E.01727
G1 X97.824 Y82.177 E.01729
G1 X98.198 Y82.57 E.01728
G1 X98.553 Y82.981 E.01727
G1 X98.887 Y83.409 E.01728
G1 X99.2 Y83.853 E.01729
G1 X99.49 Y84.312 E.01727
G1 X99.758 Y84.784 E.01728
G1 X100.002 Y85.269 E.01728
G1 X100.222 Y85.766 E.01728
G1 X100.417 Y86.273 E.01728
G1 X100.588 Y86.788 E.01729
G1 X100.733 Y87.312 E.01727
G1 X100.852 Y87.842 E.01728
G1 X100.944 Y88.376 E.01727
G1 X101.011 Y88.916 E.01729
G1 X101.051 Y89.457 E.01726
G1 X101.064 Y90 E.01729
G1 X101.051 Y90.543 E.01728
G1 X101.011 Y91.085 E.01729
G1 X100.944 Y91.624 E.01728
G1 X100.852 Y92.158 E.01727
G1 X100.733 Y92.688 E.01728
G1 X100.588 Y93.212 E.01727
G1 X100.417 Y93.727 E.01729
G1 X100.222 Y94.234 E.01727
G1 X100.002 Y94.731 E.01729
G1 X99.758 Y95.216 E.01727
G1 X99.49 Y95.688 E.01729
G1 X99.2 Y96.147 E.01727
G1 X98.887 Y96.591 E.01729
G1 X98.553 Y97.019 E.01728
G1 X98.198 Y97.43 E.01727
G1 X97.824 Y97.823 E.01728
G1 X97.43 Y98.198 E.01729
G1 X97.019 Y98.553 E.01727
G1 X96.591 Y98.887 E.01728
G1 X96.147 Y99.2 E.01728
G1 X95.688 Y99.49 E.01728
G1 X95.216 Y99.758 E.01728
G1 X94.73 Y100.002 E.01728
G1 X94.234 Y100.222 E.01727
G1 X93.727 Y100.417 E.01728
G1 X93.212 Y100.588 E.01728
G1 X92.688 Y100.733 E.01728
G1 X92.158 Y100.852 E.01729
G1 X91.624 Y100.944 E.01727
G1 X91.085 Y101.011 E.01727
G1 X90.543 Y101.051 E.0173
G1 X90 Y101.064 E.01727
G1 X89.457 Y101.051 E.01727
G1 X88.915 Y101.011 E.0173
G1 X88.377 Y100.944 E.01727
G1 X87.841 Y100.852 E.01729
G1 X87.312 Y100.733 E.01727
G1 X86.788 Y100.588 E.01728
G1 X86.273 Y100.417 E.01727
G1 X85.766 Y100.222 E.01729
G1 X85.269 Y100.002 E.01728
G1 X84.784 Y99.758 E.01727
G1 X84.312 Y99.49 E.01728
M73 P93 R1
G1 X83.853 Y99.2 E.01727
G1 X83.409 Y98.887 E.01729
G1 X82.981 Y98.553 E.01728
G1 X82.57 Y98.198 E.01726
G1 X82.176 Y97.823 E.01729
G1 X81.802 Y97.43 E.01728
G1 X81.447 Y97.019 E.01727
G1 X81.113 Y96.591 E.01728
G1 X80.8 Y96.147 E.01728
G1 X80.51 Y95.688 E.01727
G1 X80.242 Y95.215 E.01729
G1 X79.998 Y94.731 E.01727
G1 X79.778 Y94.234 E.01728
G1 X79.583 Y93.727 E.01728
G1 X79.412 Y93.212 E.01729
G1 X79.267 Y92.688 E.01727
G1 X79.148 Y92.158 E.01728
G1 X79.056 Y91.624 E.01728
G1 X78.989 Y91.084 E.01728
G1 X78.949 Y90.543 E.01727
G1 X78.936 Y90 E.01729
G1 X78.949 Y89.457 E.01727
G1 X78.989 Y88.915 E.01729
G1 X79.056 Y88.377 E.01727
G1 X79.148 Y87.841 E.01729
G1 X79.267 Y87.312 E.01726
G1 X79.412 Y86.788 E.01729
G1 X79.583 Y86.273 E.01728
G1 X79.778 Y85.766 E.01727
G1 X79.998 Y85.269 E.01729
G1 X80.242 Y84.784 E.01727
G1 X80.51 Y84.312 E.01729
G1 X80.8 Y83.853 E.01727
G1 X81.113 Y83.409 E.01728
G1 X81.447 Y82.981 E.01729
G1 X81.802 Y82.57 E.01727
G1 X82.176 Y82.176 E.01729
G1 X82.57 Y81.802 E.01728
G1 X82.981 Y81.447 E.01727
G1 X83.409 Y81.113 E.01727
G1 X83.853 Y80.8 E.01729
G1 X84.312 Y80.51 E.01727
G1 X84.784 Y80.242 E.01728
G1 X85.269 Y79.998 E.01727
G1 X85.766 Y79.778 E.01728
G1 X86.273 Y79.583 E.01728
G1 X86.789 Y79.412 E.01729
G1 X87.312 Y79.267 E.01727
G1 X87.841 Y79.149 E.01727
G1 X88.377 Y79.056 E.0173
G1 X88.915 Y78.989 E.01727
G1 X89.457 Y78.949 E.01728
G1 X90.428 Y78.946 E.03089
G1 X91.087 Y78.989 E.02101
G1 X91.623 Y79.056 E.0172
G1 X92.159 Y79.149 E.01729
G1 X92.63 Y79.254 E.01536
; COOLING_NODE: 5
M204 S250
G1 X92.784 Y78.887 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F2236
M204 S5000
G1 X93.325 Y79.037 E.01656
G1 X93.859 Y79.213 E.01658
G1 X94.384 Y79.416 E.01657
G1 X94.898 Y79.644 E.01658
G1 X95.4 Y79.896 E.01657
G1 X95.89 Y80.174 E.01658
G1 X96.365 Y80.474 E.01657
G1 X96.824 Y80.798 E.01657
G1 X97.268 Y81.144 E.01657
G1 X97.694 Y81.511 E.01657
G1 X98.101 Y81.899 E.01658
G1 X98.489 Y82.306 E.01657
G1 X98.856 Y82.732 E.01657
G1 X99.202 Y83.175 E.01657
G1 X99.526 Y83.635 E.01658
G1 X99.826 Y84.11 E.01657
G1 X100.104 Y84.6 E.01658
G1 X100.356 Y85.102 E.01657
G1 X100.584 Y85.616 E.01658
G1 X100.787 Y86.141 E.01657
G1 X100.963 Y86.675 E.01657
G1 X101.113 Y87.216 E.01657
G1 X101.236 Y87.765 E.01658
G1 X101.332 Y88.319 E.01657
G1 X101.401 Y88.877 E.01658
G1 X101.443 Y89.438 E.01657
G1 X101.456 Y90 E.01658
G1 X101.443 Y90.562 E.01657
G1 X101.401 Y91.123 E.01658
G1 X101.332 Y91.681 E.01657
G1 X101.236 Y92.235 E.01656
G1 X101.113 Y92.784 E.01658
G1 X100.963 Y93.325 E.01656
G1 X100.787 Y93.859 E.01658
G1 X100.584 Y94.384 E.01657
G1 X100.356 Y94.898 E.01658
G1 X100.104 Y95.4 E.01656
G1 X99.826 Y95.89 E.01658
G1 X99.526 Y96.365 E.01657
G1 X99.202 Y96.825 E.01658
G1 X98.856 Y97.268 E.01657
G1 X98.489 Y97.694 E.01657
G1 X98.101 Y98.101 E.01657
G1 X97.693 Y98.489 E.01658
G1 X97.268 Y98.856 E.01656
G1 X96.825 Y99.202 E.01657
G1 X96.365 Y99.526 E.01658
G1 X95.89 Y99.826 E.01657
G1 X95.401 Y100.104 E.01657
G1 X94.898 Y100.356 E.01658
G1 X94.384 Y100.584 E.01656
G1 X93.859 Y100.787 E.01658
G1 X93.326 Y100.963 E.01657
G1 X92.784 Y101.113 E.01657
G1 X92.235 Y101.236 E.01657
G1 X91.681 Y101.332 E.01657
G1 X91.123 Y101.401 E.01657
G1 X90.562 Y101.443 E.01658
G1 X90 Y101.456 E.01657
G1 X89.438 Y101.443 E.01657
G1 X88.877 Y101.401 E.01658
G1 X88.319 Y101.332 E.01657
G1 X87.765 Y101.236 E.01658
G1 X87.216 Y101.113 E.01657
G1 X86.675 Y100.963 E.01656
G1 X86.141 Y100.787 E.01658
G1 X85.616 Y100.584 E.01658
G1 X85.102 Y100.356 E.01657
G1 X84.599 Y100.104 E.01657
G1 X84.11 Y99.826 E.01657
G1 X83.635 Y99.526 E.01657
G1 X83.175 Y99.202 E.01658
G1 X82.732 Y98.856 E.01657
G1 X82.307 Y98.489 E.01656
G1 X81.899 Y98.101 E.01658
G1 X81.511 Y97.694 E.01657
G1 X81.144 Y97.268 E.01656
G1 X80.798 Y96.824 E.01658
G1 X80.474 Y96.365 E.01657
G1 X80.174 Y95.89 E.01657
G1 X79.896 Y95.4 E.01658
G1 X79.644 Y94.898 E.01656
G1 X79.416 Y94.384 E.01658
G1 X79.213 Y93.86 E.01657
G1 X79.037 Y93.325 E.01658
G1 X78.887 Y92.784 E.01657
G1 X78.764 Y92.235 E.01658
G1 X78.668 Y91.681 E.01657
G1 X78.599 Y91.123 E.01658
G1 X78.557 Y90.562 E.01657
G1 X78.544 Y90 E.01658
G1 X78.557 Y89.438 E.01657
G1 X78.599 Y88.877 E.01658
G1 X78.668 Y88.319 E.01657
G1 X78.764 Y87.765 E.01658
G1 X78.887 Y87.216 E.01657
G1 X79.037 Y86.674 E.01657
G1 X79.213 Y86.141 E.01657
G1 X79.416 Y85.616 E.01657
G1 X79.644 Y85.102 E.01658
G1 X79.896 Y84.599 E.01657
G1 X80.174 Y84.11 E.01657
G1 X80.474 Y83.635 E.01657
G1 X80.798 Y83.175 E.01657
G1 X81.144 Y82.732 E.01657
G1 X81.511 Y82.306 E.01657
G1 X81.899 Y81.899 E.01658
G1 X82.307 Y81.511 E.01658
G1 X82.732 Y81.144 E.01656
G1 X83.175 Y80.798 E.01657
G1 X83.635 Y80.474 E.01658
G1 X84.11 Y80.174 E.01657
G1 X84.6 Y79.896 E.01658
G1 X85.102 Y79.644 E.01656
G1 X85.616 Y79.416 E.01658
G1 X86.141 Y79.213 E.01657
G1 X86.675 Y79.037 E.01658
G1 X87.216 Y78.887 E.01656
G1 X87.765 Y78.764 E.01657
G1 X88.319 Y78.668 E.01658
G1 X88.877 Y78.599 E.01657
G1 X89.438 Y78.557 E.01658
G1 X90 Y78.544 E.01657
G1 X90.445 Y78.555 E.01313
M106 S122.4
M106 S127.5
G1 X90.845 Y78.581 E.01179
M106 S122.4
M106 S127.5
G1 X91.124 Y78.599 E.00825
M106 S122.4
M106 S127.5
G1 X91.681 Y78.668 E.01654
G1 X92.235 Y78.764 E.01658
G1 X92.725 Y78.874 E.01481
M106 S122.4
; WIPE_START
G1 F8400
M204 S6000
G1 X93.325 Y79.037 E-.2363
G1 X93.684 Y79.156 E-.1437
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.5 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z8.5
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z8.5 F4000
            G39.3 S1
            G0 Z8.5 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X83.77 Y81.059 F42000
G1 Z8.1
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.42605
G1 F6017
M204 S6000
G1 X82.32 Y82.509 E.06139
G1 X81.705 Y83.192 E.02752
G1 X81.381 Y83.608 E.0158
G1 X81.078 Y84.038 E.01576
G1 X80.676 Y84.695 E.02304
G1 X84.695 Y80.676 E.17018
G1 X85.412 Y80.3 E.02426
G1 X85.773 Y80.139 E.01184
G1 X80.139 Y85.773 E.2386
G1 X79.811 Y86.643 E.02784
G1 X86.643 Y79.811 E.28934
G1 X87.409 Y79.587 E.0239
G1 X79.587 Y87.409 E.33127
G1 X79.443 Y88.096 E.021
G1 X88.096 Y79.443 E.36645
G1 X88.732 Y79.348 E.01929
G1 X79.348 Y88.732 E.39745
G1 X79.293 Y89.329 E.01794
G1 X89.329 Y79.293 E.42503
G1 X89.882 Y79.281 E.01658
G1 X79.272 Y89.892 E.44935
G1 X79.28 Y90.426 E.01599
G1 X90.425 Y79.28 E.47202
G1 X90.934 Y79.313 E.01526
G1 X79.312 Y90.935 E.49217
G1 X79.367 Y91.422 E.01469
G1 X91.422 Y79.367 E.51056
G1 X91.891 Y79.44 E.01419
G1 X79.44 Y91.891 E.52728
G1 X79.531 Y92.342 E.01377
G1 X92.342 Y79.531 E.54252
G1 X92.777 Y79.638 E.01342
G1 X79.638 Y92.777 E.55644
G1 X79.759 Y93.198 E.01311
G1 X93.198 Y79.759 E.56915
G1 X93.605 Y79.893 E.01284
G1 X79.893 Y93.605 E.5807
G1 X80.044 Y93.996 E.01256
G1 X93.996 Y80.044 E.5909
G1 X94.376 Y80.206 E.01237
G1 X80.206 Y94.376 E.60012
G1 X80.379 Y94.745 E.0122
G1 X94.745 Y80.379 E.60842
G1 X95.104 Y80.562 E.01206
G1 X80.562 Y95.104 E.61584
G1 X80.758 Y95.45 E.0119
G1 X95.45 Y80.758 E.62219
G1 X95.784 Y80.965 E.01178
G1 X80.965 Y95.784 E.62758
G1 X81.078 Y95.962 E.0063
G1 X81.182 Y96.109 E.00541
G1 X96.109 Y81.182 E.63219
G1 X96.426 Y81.407 E.01163
G1 X81.407 Y96.426 E.63604
G1 X81.645 Y96.73 E.01156
G1 X96.73 Y81.645 E.63887
G1 X97.024 Y81.892 E.01152
G1 X81.892 Y97.024 E.64085
G1 X82.148 Y97.31 E.01149
G1 X97.31 Y82.148 E.64212
G1 X97.588 Y82.412 E.01148
G1 X82.412 Y97.588 E.64269
G1 X82.69 Y97.852 E.01148
G1 X97.852 Y82.69 E.64212
G1 X98.108 Y82.976 E.01149
G1 X82.976 Y98.108 E.64085
G1 X83.27 Y98.356 E.01152
G1 X98.356 Y83.27 E.63887
G1 X98.593 Y83.574 E.01156
G1 X83.574 Y98.593 E.63604
G1 X83.891 Y98.818 E.01163
G1 X98.818 Y83.891 E.63218
G1 X99.035 Y84.216 E.0117
G1 X84.216 Y99.035 E.62757
G1 X84.551 Y99.242 E.01179
G1 X99.242 Y84.551 E.62219
G1 X99.438 Y84.897 E.0119
G1 X84.897 Y99.438 E.61584
G1 X85.255 Y99.622 E.01206
G1 X99.622 Y85.255 E.60842
G1 X99.794 Y85.624 E.0122
G1 X85.624 Y99.794 E.60012
G1 X86.004 Y99.956 E.01237
G1 X99.956 Y86.004 E.5909
G1 X100.107 Y86.395 E.01256
G1 X86.395 Y100.107 E.58069
G1 X86.803 Y100.241 E.01285
G1 X100.241 Y86.803 E.56914
G1 X100.362 Y87.223 E.01311
G1 X87.223 Y100.362 E.55643
G1 X87.659 Y100.469 E.01342
G1 X100.469 Y87.659 E.54252
G1 X100.56 Y88.11 E.01378
G1 X88.11 Y100.56 E.52727
G1 X88.578 Y100.633 E.01419
G1 X100.633 Y88.578 E.51055
G1 X100.688 Y89.065 E.01469
G1 X89.065 Y100.688 E.49221
G1 X89.575 Y100.72 E.01528
G1 X100.72 Y89.575 E.47202
G1 X100.728 Y90.109 E.01599
G1 X90.109 Y100.728 E.44974
G1 X90.671 Y100.707 E.01687
G1 X100.707 Y90.671 E.42501
G1 X100.652 Y91.268 E.01794
G1 X91.268 Y100.652 E.39743
G1 X91.905 Y100.557 E.01929
G1 X100.557 Y91.905 E.36643
G1 X100.413 Y92.591 E.021
G1 X92.591 Y100.413 E.33124
G1 X93.357 Y100.189 E.0239
G1 X100.189 Y93.357 E.28931
G1 X99.86 Y94.227 E.02785
G1 X94.227 Y99.86 E.23856
G1 X94.588 Y99.701 E.0118
G1 X95.306 Y99.323 E.0243
G1 X99.323 Y95.306 E.17013
G1 X98.922 Y95.962 E.02301
G1 X98.619 Y96.392 E.01577
G1 X98.295 Y96.808 E.01578
G1 X97.673 Y97.499 E.02785
G1 X96.232 Y98.94 E.06103
M106 S127.5
; CHANGE_LAYER
; Z_HEIGHT: 8.3
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F10179.312
G1 X96.939 Y98.233 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 42/43
; update layer progress
M73 L42
M991 S0 P41 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z8.5 I1.188 J-.264 P1  F42000
G1 X92.708 Y79.188 Z8.5
G1 Z8.3
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.45
G1 F6163
M204 S6000
G1 X93.235 Y79.334 E.01739
G1 X93.755 Y79.505 E.01742
G1 X94.265 Y79.702 E.0174
G1 X94.766 Y79.924 E.01741
G1 X95.254 Y80.17 E.01739
G1 X95.73 Y80.44 E.01742
G1 X96.192 Y80.732 E.01739
G1 X96.64 Y81.047 E.01741
G1 X97.071 Y81.384 E.01741
G1 X97.485 Y81.741 E.01741
G1 X97.881 Y82.118 E.0174
G1 X98.259 Y82.515 E.0174
G1 X98.616 Y82.929 E.01741
G1 X98.953 Y83.36 E.01741
G1 X99.268 Y83.808 E.01741
G1 X99.56 Y84.27 E.0174
G1 X99.83 Y84.746 E.01741
G1 X100.076 Y85.235 E.01741
G1 X100.298 Y85.734 E.0174
G1 X100.494 Y86.245 E.01741
G1 X100.666 Y86.765 E.01742
G1 X100.812 Y87.292 E.01739
G1 X100.932 Y87.826 E.01742
G1 X101.025 Y88.364 E.0174
G1 X101.092 Y88.907 E.0174
G1 X101.133 Y89.453 E.01742
G1 X101.146 Y90 E.0174
M73 P94 R1
G1 X101.133 Y90.547 E.0174
G1 X101.092 Y91.093 E.01742
G1 X101.025 Y91.636 E.0174
G1 X100.932 Y92.174 E.0174
G1 X100.812 Y92.708 E.01741
G1 X100.666 Y93.235 E.0174
G1 X100.495 Y93.755 E.01741
G1 X100.298 Y94.266 E.01741
G1 X100.076 Y94.765 E.0174
M73 P94 R0
G1 X99.83 Y95.254 E.01742
G1 X99.56 Y95.73 E.01739
G1 X99.268 Y96.192 E.01741
G1 X98.953 Y96.64 E.01741
G1 X98.616 Y97.071 E.0174
G1 X98.259 Y97.485 E.01741
G1 X97.882 Y97.881 E.0174
G1 X97.485 Y98.259 E.01741
G1 X97.071 Y98.616 E.01741
G1 X96.64 Y98.953 E.0174
G1 X96.192 Y99.268 E.01741
G1 X95.73 Y99.56 E.01739
G1 X95.254 Y99.83 E.01742
G1 X94.766 Y100.076 E.0174
G1 X94.265 Y100.298 E.01741
G1 X93.755 Y100.495 E.0174
G1 X93.236 Y100.666 E.01741
G1 X92.708 Y100.812 E.01741
G1 X92.174 Y100.932 E.01741
G1 X91.636 Y101.025 E.0174
G1 X91.093 Y101.092 E.0174
G1 X90.547 Y101.133 E.01743
G1 X90 Y101.146 E.0174
G1 X89.453 Y101.133 E.0174
G1 X88.907 Y101.092 E.01743
G1 X88.365 Y101.025 E.01739
G1 X87.825 Y100.932 E.01742
G1 X87.292 Y100.812 E.0174
G1 X86.765 Y100.666 E.0174
G1 X86.245 Y100.495 E.01741
G1 X85.735 Y100.298 E.01741
G1 X85.234 Y100.076 E.01741
G1 X84.746 Y99.83 E.01741
G1 X84.27 Y99.56 E.0174
G1 X83.808 Y99.268 E.01741
G1 X83.36 Y98.953 E.01741
G1 X82.929 Y98.616 E.01741
G1 X82.515 Y98.259 E.01739
G1 X82.118 Y97.881 E.01743
G1 X81.741 Y97.485 E.01739
G1 X81.384 Y97.071 E.01741
G1 X81.047 Y96.64 E.01741
G1 X80.732 Y96.192 E.01742
G1 X80.44 Y95.73 E.0174
G1 X80.17 Y95.254 E.01741
G1 X79.924 Y94.766 E.01741
G1 X79.702 Y94.265 E.01741
G1 X79.505 Y93.755 E.01741
G1 X79.334 Y93.235 E.01741
G1 X79.188 Y92.708 E.0174
G1 X79.068 Y92.174 E.01742
G1 X78.975 Y91.636 E.0174
G1 X78.908 Y91.093 E.0174
G1 X78.867 Y90.547 E.01742
G1 X78.854 Y90 E.0174
G1 X78.867 Y89.453 E.0174
G1 X78.908 Y88.907 E.01743
G1 X78.975 Y88.364 E.0174
G1 X79.068 Y87.826 E.0174
G1 X79.188 Y87.292 E.01742
G1 X79.334 Y86.764 E.01741
G1 X79.505 Y86.245 E.0174
G1 X79.702 Y85.735 E.01741
G1 X79.924 Y85.234 E.01741
G1 X80.17 Y84.746 E.0174
G1 X80.44 Y84.27 E.01741
G1 X80.732 Y83.808 E.01739
G1 X81.047 Y83.36 E.01741
G1 X81.384 Y82.929 E.01741
G1 X81.741 Y82.515 E.0174
G1 X82.119 Y82.118 E.01741
G1 X82.515 Y81.741 E.01741
G1 X82.929 Y81.384 E.0174
G1 X83.36 Y81.047 E.01741
G1 X83.808 Y80.732 E.01741
G1 X84.27 Y80.44 E.01739
G1 X84.746 Y80.17 E.01742
G1 X85.235 Y79.924 E.01741
G1 X85.734 Y79.702 E.01739
G1 X86.245 Y79.506 E.01741
G1 X86.765 Y79.334 E.01742
G1 X87.292 Y79.188 E.0174
G1 X87.826 Y79.068 E.01742
G1 X88.364 Y78.975 E.0174
G1 X88.907 Y78.908 E.01741
G1 X89.447 Y78.868 E.01722
G1 X90.369 Y78.863 E.02931
G1 X91.096 Y78.908 E.0232
G1 X91.636 Y78.975 E.01729
G1 X92.174 Y79.068 E.0174
G1 X92.65 Y79.175 E.01551
; COOLING_NODE: 5
M204 S250
G1 X92.804 Y78.808 F42000
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F2339
M204 S5000
G1 X93.349 Y78.959 E.01668
G1 X93.887 Y79.136 E.0167
G1 X94.415 Y79.34 E.01669
G1 X94.933 Y79.57 E.0167
G1 X95.439 Y79.824 E.01668
G1 X95.932 Y80.103 E.0167
G1 X96.41 Y80.406 E.01668
G1 X96.873 Y80.732 E.01669
G1 X97.32 Y81.081 E.0167
G1 X97.749 Y81.451 E.01669
G1 X98.159 Y81.841 E.01669
G1 X98.549 Y82.251 E.01669
G1 X98.919 Y82.68 E.01669
G1 X99.268 Y83.127 E.01669
G1 X99.594 Y83.59 E.0167
G1 X99.897 Y84.068 E.01669
G1 X100.176 Y84.561 E.01669
G1 X100.431 Y85.067 E.0167
G1 X100.66 Y85.584 E.01668
G1 X100.864 Y86.113 E.0167
G1 X101.042 Y86.651 E.0167
G1 X101.192 Y87.196 E.01668
G1 X101.317 Y87.749 E.0167
G1 X101.413 Y88.307 E.01669
G1 X101.483 Y88.869 E.01669
G1 X101.524 Y89.434 E.0167
G1 X101.538 Y90 E.01669
G1 X101.524 Y90.566 E.01669
G1 X101.483 Y91.131 E.0167
G1 X101.413 Y91.693 E.01669
G1 X101.317 Y92.251 E.01669
G1 X101.192 Y92.804 E.01669
G1 X101.041 Y93.349 E.01668
G1 X100.864 Y93.887 E.0167
G1 X100.66 Y94.416 E.01669
G1 X100.43 Y94.933 E.01669
G1 X100.176 Y95.439 E.0167
G1 X99.897 Y95.932 E.01668
G1 X99.594 Y96.41 E.01669
G1 X99.268 Y96.873 E.01669
G1 X98.919 Y97.32 E.0167
G1 X98.549 Y97.749 E.01669
G1 X98.159 Y98.159 E.01669
G1 X97.749 Y98.549 E.0167
G1 X97.32 Y98.919 E.01669
G1 X96.873 Y99.268 E.01669
G1 X96.41 Y99.594 E.0167
G1 X95.932 Y99.897 E.01668
G1 X95.439 Y100.176 E.0167
G1 X94.933 Y100.43 E.01669
G1 X94.416 Y100.66 E.01669
G1 X93.887 Y100.864 E.01669
G1 X93.349 Y101.041 E.0167
G1 X92.804 Y101.193 E.01669
G1 X92.251 Y101.317 E.01669
G1 X91.693 Y101.413 E.01668
G1 X91.131 Y101.483 E.01669
G1 X90.566 Y101.524 E.0167
G1 X90 Y101.538 E.01669
G1 X89.434 Y101.524 E.01669
G1 X88.869 Y101.483 E.0167
G1 X88.307 Y101.413 E.01669
G1 X87.749 Y101.317 E.01669
G1 X87.196 Y101.192 E.0167
G1 X86.651 Y101.042 E.01668
G1 X86.113 Y100.864 E.0167
G1 X85.585 Y100.66 E.01669
G1 X85.067 Y100.43 E.0167
G1 X84.561 Y100.176 E.01669
G1 X84.068 Y99.897 E.01669
G1 X83.59 Y99.594 E.01669
G1 X83.127 Y99.268 E.01669
G1 X82.68 Y98.919 E.01669
G1 X82.251 Y98.549 E.01669
G1 X81.841 Y98.159 E.0167
G1 X81.451 Y97.749 E.01669
G1 X81.081 Y97.32 E.01669
G1 X80.732 Y96.873 E.01669
G1 X80.406 Y96.41 E.0167
G1 X80.103 Y95.932 E.01669
G1 X79.824 Y95.439 E.01669
G1 X79.57 Y94.933 E.01669
G1 X79.34 Y94.416 E.01669
G1 X79.136 Y93.887 E.0167
G1 X78.959 Y93.349 E.0167
G1 X78.808 Y92.804 E.01668
G1 X78.683 Y92.251 E.0167
G1 X78.587 Y91.693 E.01669
G1 X78.517 Y91.131 E.01669
G1 X78.476 Y90.566 E.0167
G1 X78.462 Y90 E.01669
G1 X78.476 Y89.434 E.01669
G1 X78.517 Y88.869 E.0167
G1 X78.587 Y88.307 E.01669
G1 X78.683 Y87.749 E.01668
G1 X78.807 Y87.196 E.01669
G1 X78.959 Y86.651 E.01669
G1 X79.136 Y86.113 E.01669
G1 X79.34 Y85.585 E.01669
G1 X79.57 Y85.067 E.0167
G1 X79.824 Y84.561 E.01669
G1 X80.103 Y84.068 E.01669
G1 X80.406 Y83.59 E.01669
G1 X80.732 Y83.127 E.01669
G1 X81.081 Y82.68 E.01669
G1 X81.451 Y82.251 E.01669
G1 X81.841 Y81.841 E.01669
G1 X82.251 Y81.451 E.01669
G1 X82.68 Y81.081 E.01669
G1 X83.127 Y80.732 E.01669
G1 X83.59 Y80.406 E.0167
G1 X84.068 Y80.103 E.01668
G1 X84.561 Y79.824 E.0167
G1 X85.067 Y79.569 E.01669
G1 X85.584 Y79.34 E.01668
G1 X86.113 Y79.136 E.0167
G1 X86.651 Y78.959 E.0167
G1 X87.196 Y78.808 E.01668
G1 X87.749 Y78.683 E.0167
G1 X88.307 Y78.587 E.01669
G1 X88.869 Y78.517 E.01669
G1 X89.432 Y78.476 E.01664
M106 S122.4
M106 S127.5
G1 X90.38 Y78.471 E.02794
G1 X91.132 Y78.517 E.02222
G1 X91.693 Y78.587 E.01666
G1 X92.251 Y78.683 E.01668
G1 X92.745 Y78.794 E.01493
M106 S122.4
; WIPE_START
G1 F10342.643
M204 S6000
G1 X93.349 Y78.959 E-.23783
G1 X93.704 Y79.076 E-.14217
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.7 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z8.7
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z8.7 F4000
            G39.3 S1
            G0 Z8.7 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X98.996 Y83.706 F42000
G1 Z8.3
G1 E.4 F1800
; FEATURE: Internal solid infill
; LINE_WIDTH: 0.42227
G1 F6163
M204 S6000
G1 X97.612 Y82.322 E.05804
G1 X97.262 Y81.988 E.01435
G1 X96.859 Y81.642 E.01575
G1 X96.441 Y81.315 E.01573
G1 X96.007 Y81.009 E.01574
G1 X95.375 Y80.621 E.022
G1 X99.379 Y84.625 E.16789
G1 X99.775 Y85.377 E.02521
G1 X99.918 Y85.702 E.01051
G1 X94.299 Y80.082 E.23566
G1 X93.429 Y79.749 E.0276
G1 X100.251 Y86.571 E.28605
G1 X100.478 Y87.334 E.02361
G1 X92.666 Y79.522 E.32757
G1 X91.98 Y79.373 E.02082
G1 X100.627 Y88.02 E.3626
G1 X100.725 Y88.654 E.01903
G1 X91.346 Y79.275 E.39331
G1 X90.756 Y79.221 E.01757
G1 X100.783 Y89.249 E.42049
G1 X100.808 Y89.81 E.01665
G1 X90.195 Y79.197 E.44501
G1 X89.662 Y79.2 E.01583
G1 X100.804 Y90.343 E.46724
G1 X100.776 Y90.851 E.01509
G1 X89.149 Y79.224 E.48755
G1 X88.662 Y79.274 E.01451
G1 X100.726 Y91.338 E.50587
G1 X100.658 Y91.806 E.01402
G1 X88.194 Y79.342 E.52261
G1 X87.744 Y79.428 E.01361
G1 X100.572 Y92.256 E.53792
G1 X100.471 Y92.692 E.01325
G1 X87.308 Y79.529 E.55193
G1 X86.888 Y79.645 E.01293
G1 X100.355 Y93.112 E.56468
G1 X100.222 Y93.516 E.01261
G1 X86.484 Y79.778 E.57609
G1 X86.092 Y79.922 E.01239
G1 X100.078 Y93.908 E.58649
G1 X99.923 Y94.289 E.0122
G1 X85.711 Y80.077 E.59593
G1 X85.341 Y80.244 E.01203
G1 X99.756 Y94.659 E.60448
G1 X99.577 Y95.016 E.01185
G1 X84.984 Y80.423 E.61191
G1 X84.638 Y80.614 E.01171
G1 X99.386 Y95.362 E.61841
G1 X99.186 Y95.699 E.01161
G1 X84.301 Y80.814 E.62414
G1 X83.974 Y81.023 E.01152
G1 X98.977 Y96.026 E.62912
G1 X98.755 Y96.341 E.01142
G1 X83.659 Y81.245 E.63302
G1 X83.353 Y81.476 E.01136
G1 X98.524 Y96.647 E.63616
G1 X98.285 Y96.944 E.01131
G1 X83.056 Y81.715 E.6386
G1 X82.768 Y81.963 E.01128
G1 X98.037 Y97.232 E.64026
G1 X97.777 Y97.508 E.01125
G1 X82.492 Y82.223 E.64093
G1 X82.354 Y82.354 E.00563
G1 X82.224 Y82.492 E.00562
G1 X97.508 Y97.776 E.64093
G1 X97.232 Y98.037 E.01125
G1 X81.963 Y82.768 E.64027
G1 X81.715 Y83.056 E.01128
G1 X96.944 Y98.285 E.63861
G1 X96.647 Y98.524 E.01131
G1 X81.476 Y83.353 E.63616
G1 X81.245 Y83.659 E.01136
G1 X96.341 Y98.755 E.63303
G1 X96.026 Y98.977 E.01142
G1 X81.023 Y83.974 E.62913
M73 P95 R0
G1 X80.814 Y84.301 E.01152
G1 X95.699 Y99.186 E.62414
G1 X95.362 Y99.386 E.01161
G1 X80.614 Y84.638 E.61842
G1 X80.424 Y84.983 E.01171
G1 X95.017 Y99.576 E.61192
G1 X94.66 Y99.756 E.01185
G1 X80.244 Y85.34 E.60449
G1 X80.078 Y85.71 E.01203
G1 X94.29 Y99.922 E.59594
G1 X93.909 Y100.078 E.0122
G1 X79.922 Y86.091 E.5865
G1 X79.778 Y86.483 E.01239
G1 X93.517 Y100.222 E.5761
G1 X93.112 Y100.354 E.01261
G1 X79.646 Y86.888 E.56469
G1 X79.529 Y87.308 E.01293
G1 X92.692 Y100.471 E.55194
G1 X92.257 Y100.572 E.01325
G1 X79.428 Y87.743 E.53793
G1 X79.342 Y88.194 E.0136
G1 X91.806 Y100.658 E.52262
G1 X91.338 Y100.726 E.01402
G1 X79.274 Y88.662 E.50589
G1 X79.224 Y89.149 E.01451
G1 X90.851 Y100.776 E.48756
G1 X90.343 Y100.804 E.01509
G1 X79.196 Y89.657 E.46744
G1 X79.192 Y90.19 E.0158
G1 X89.81 Y100.808 E.44526
G1 X89.249 Y100.783 E.01665
G1 X79.217 Y90.751 E.4207
G1 X79.275 Y91.345 E.01771
G1 X88.655 Y100.725 E.39334
G1 X88.02 Y100.628 E.01903
G1 X79.372 Y91.98 E.36263
G1 X79.522 Y92.665 E.02081
G1 X87.335 Y100.478 E.3276
G1 X86.571 Y100.251 E.02361
G1 X79.749 Y93.429 E.28609
G1 X80.081 Y94.298 E.02759
G1 X85.702 Y99.919 E.2357
G1 X85.377 Y99.775 E.01055
G1 X84.626 Y99.379 E.02516
G1 X80.621 Y95.374 E.16795
G1 X81.009 Y96.007 E.02203
G1 X81.315 Y96.441 E.01574
G1 X81.642 Y96.86 E.01574
G1 X81.988 Y97.262 E.01574
G1 X82.314 Y97.603 E.01398
G1 X83.708 Y98.997 E.05846
; CHANGE_LAYER
; Z_HEIGHT: 8.5
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F10280.742
G1 X83.001 Y98.29 E-.38
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 43/43
; update layer progress
M73 L43
M991 S0 P42 ;notify layer change
; OBJECT_ID: 346
; COOLING_NODE: 5
M204 S10000
G17
G3 Z8.7 I1.087 J.547 P1  F42000
G1 X92.799 Y78.825 Z8.7
G1 Z8.5
G1 E.4 F1800
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F2408
M204 S5000
G1 X93.344 Y78.977 E.01667
G1 X93.881 Y79.154 E.01668
G1 X94.409 Y79.356 E.01665
G1 X94.925 Y79.586 E.01666
G1 X95.43 Y79.841 E.01667
G1 X95.923 Y80.119 E.01668
G1 X96.401 Y80.421 E.01666
G1 X96.862 Y80.747 E.01665
G1 X97.308 Y81.096 E.01668
G1 X97.736 Y81.465 E.01666
G1 X98.146 Y81.854 E.01667
G1 X98.535 Y82.264 E.01665
G1 X98.904 Y82.692 E.01667
G1 X99.253 Y83.138 E.01668
G1 X99.579 Y83.6 E.01665
G1 X99.881 Y84.077 E.01665
G1 X100.159 Y84.57 E.01667
G1 X100.414 Y85.075 E.01668
G1 X100.644 Y85.591 E.01666
G1 X100.846 Y86.119 E.01665
G1 X101.023 Y86.656 E.01667
G1 X101.175 Y87.201 E.01668
G1 X101.299 Y87.753 E.01666
G1 X101.395 Y88.309 E.01665
G1 X101.463 Y88.871 E.01668
G1 X101.506 Y89.435 E.01668
G1 X101.521 Y90 E.01665
G1 X101.506 Y90.565 E.01665
G1 X101.463 Y91.129 E.01667
G1 X101.395 Y91.691 E.01667
G1 X101.299 Y92.248 E.01666
G1 X101.175 Y92.799 E.01665
G1 X101.023 Y93.344 E.01667
G1 X100.846 Y93.881 E.01668
G1 X100.644 Y94.409 E.01665
G1 X100.414 Y94.925 E.01666
G1 X100.159 Y95.43 E.01667
G1 X99.88 Y95.923 E.01669
G1 X99.579 Y96.401 E.01665
G1 X99.253 Y96.862 E.01666
G1 X98.904 Y97.307 E.01667
G1 X98.535 Y97.736 E.01667
G1 X98.146 Y98.146 E.01666
G1 X97.737 Y98.535 E.01665
G1 X97.307 Y98.904 E.01668
G1 X96.862 Y99.253 E.01667
G1 X96.4 Y99.579 E.01665
G1 X95.923 Y99.881 E.01665
G1 X95.43 Y100.159 E.01667
G1 X94.925 Y100.414 E.01668
G1 X94.409 Y100.644 E.01666
G1 X93.881 Y100.846 E.01665
G1 X93.344 Y101.023 E.01668
G1 X92.799 Y101.175 E.01666
G1 X92.247 Y101.299 E.01666
G1 X91.691 Y101.395 E.01665
G1 X91.129 Y101.463 E.01668
G1 X90.565 Y101.506 E.01668
G1 X90 Y101.521 E.01665
G1 X89.435 Y101.506 E.01665
G1 X88.871 Y101.463 E.01667
G1 X88.31 Y101.395 E.01667
G1 X87.752 Y101.299 E.01666
G1 X87.201 Y101.175 E.01666
G1 X86.656 Y101.023 E.01667
G1 X86.119 Y100.846 E.01668
G1 X85.591 Y100.644 E.01665
G1 X85.075 Y100.414 E.01666
G1 X84.57 Y100.159 E.01667
M73 P96 R0
G1 X84.077 Y99.88 E.01668
G1 X83.6 Y99.579 E.01665
G1 X83.138 Y99.253 E.01666
G1 X82.692 Y98.904 E.01668
G1 X82.264 Y98.535 E.01666
G1 X81.854 Y98.146 E.01667
G1 X81.465 Y97.736 E.01666
G1 X81.096 Y97.308 E.01666
G1 X80.747 Y96.862 E.01668
G1 X80.421 Y96.4 E.01665
G1 X80.119 Y95.923 E.01665
G1 X79.841 Y95.43 E.01667
G1 X79.586 Y94.925 E.01668
G1 X79.356 Y94.409 E.01665
G1 X79.154 Y93.881 E.01666
G1 X78.977 Y93.344 E.01667
G1 X78.825 Y92.799 E.01667
G1 X78.701 Y92.247 E.01666
G1 X78.605 Y91.691 E.01664
G1 X78.537 Y91.129 E.01668
G1 X78.494 Y90.565 E.01668
G1 X78.479 Y90 E.01665
G1 X78.494 Y89.435 E.01665
G1 X78.537 Y88.871 E.01667
G1 X78.605 Y88.309 E.01668
G1 X78.701 Y87.753 E.01665
G1 X78.825 Y87.201 E.01666
G1 X78.977 Y86.656 E.01667
G1 X79.154 Y86.119 E.01668
G1 X79.356 Y85.591 E.01665
G1 X79.586 Y85.075 E.01666
G1 X79.841 Y84.57 E.01668
G1 X80.119 Y84.077 E.01667
G1 X80.421 Y83.6 E.01665
G1 X80.747 Y83.138 E.01665
G1 X81.096 Y82.692 E.01668
G1 X81.465 Y82.264 E.01667
G1 X81.854 Y81.854 E.01665
G1 X82.264 Y81.465 E.01666
G1 X82.693 Y81.096 E.01667
G1 X83.138 Y80.747 E.01668
G1 X83.6 Y80.421 E.01665
G1 X84.077 Y80.119 E.01665
G1 X84.57 Y79.841 E.01667
G1 X85.075 Y79.586 E.01668
G1 X85.591 Y79.356 E.01666
G1 X86.119 Y79.154 E.01665
G1 X86.656 Y78.977 E.01668
G1 X87.201 Y78.825 E.01667
G1 X87.753 Y78.701 E.01666
G1 X88.309 Y78.605 E.01664
G1 X88.871 Y78.537 E.01668
G1 X89.434 Y78.494 E.01665
G1 X90.565 Y78.494 E.03333
G1 X91.129 Y78.537 E.01667
G1 X91.69 Y78.605 E.01667
G1 X92.248 Y78.701 E.01666
G1 X92.74 Y78.812 E.01489
; WIPE_START
G1 F10342.643
M204 S6000
G1 X93.344 Y78.977 E-.2377
G1 X93.699 Y79.094 E-.1423
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z8.9
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos
M400
M1004 S5 P1  ; external shutter
M400 P300
M971 S11 C11 O0
G92 E0
G1 X0 F18000
M623

; SKIPTYPE: head_wrap_detect
M622.1 S1
M1002 judge_flag g39_3rd_layer_detect_flag
M622 J1
    ; enable nozzle clog detect at 3rd layer
    


    M622.1 S1
    M1002 judge_flag g39_detection_flag
    M622 J1
      
        M622.1 S0
        M1002 judge_flag g39_mass_exceed_flag
        M622 J1
        
            G392 S0
            M400
            G90
            M83
            M204 S5000
            G0 Z8.9 F4000
            G39.3 S1
            G0 Z8.9 F4000
            G392 S0
          
        M623
    
    M623
M623
; SKIPPABLE_END




G1 X99.312 Y96.419 F42000
G1 Z8.5
G1 E.4 F1800
; FEATURE: Top surface
G1 F6974
M204 S2000
G1 X96.419 Y99.312 E.12057
; WIPE_START
G1 F10342.643
M204 S6000
G1 X97.126 Y98.605 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I-.721 J-.98 P1  F42000
G1 X95.109 Y100.088 Z8.9
G1 Z8.5
G1 E.4 F1800
G1 F6974
M204 S2000
G1 X100.088 Y95.109 E.20752
G1 X100.525 Y94.14
G1 X94.14 Y100.525 E.26613
G1 X93.318 Y100.813
G1 X100.813 Y93.318 E.31239
G1 X101.009 Y92.589
G1 X92.589 Y101.009 E.35099
G1 X91.92 Y101.145
G1 X101.145 Y91.92 E.38453
G1 X101.234 Y91.298
G1 X91.297 Y101.234 E.41416
G1 X90.711 Y101.287
G1 X101.287 Y90.711 E.4408
G1 X101.309 Y90.156
G1 X90.156 Y101.309 E.46487
G1 X89.628 Y101.303
G1 X101.303 Y89.628 E.48663
G1 X101.274 Y89.124
G1 X89.124 Y101.274 E.50645
G1 X88.639 Y101.226
G1 X101.226 Y88.639 E.52465
G1 X101.161 Y88.171
G1 X88.171 Y101.161 E.54142
G1 X87.72 Y101.079
G1 X101.079 Y87.72 E.55684
G1 X100.981 Y87.285
G1 X87.285 Y100.981 E.57087
G1 X86.866 Y100.866
G1 X100.866 Y86.866 E.58353
G1 X100.74 Y86.459
G1 X86.459 Y100.74 E.59523
G1 X86.063 Y100.603
G1 X100.603 Y86.063 E.60603
G1 X100.455 Y85.678
G1 X85.678 Y100.455 E.61592
G1 X85.308 Y100.291
G1 X100.291 Y85.308 E.62448
G1 X100.118 Y84.948
G1 X84.948 Y100.118 E.63227
G1 X84.597 Y99.936
G1 X99.936 Y84.597 E.63934
G1 X99.743 Y84.256
G1 X84.256 Y99.743 E.64553
G1 X83.926 Y99.54
G1 X99.54 Y83.926 E.6508
G1 X99.328 Y83.604
G1 X83.604 Y99.328 E.6554
G1 X83.292 Y99.108
G1 X99.108 Y83.292 E.65922
G1 X98.875 Y82.991
G1 X82.991 Y98.875 E.66204
G1 X82.698 Y98.635
G1 X98.635 Y82.698 E.66429
G1 X98.389 Y82.411
G1 X82.411 Y98.389 E.66596
G1 X82.137 Y98.129
G1 X98.129 Y82.137 E.66657
G1 X97.863 Y81.871
G1 X81.871 Y97.863 E.66657
G1 X81.611 Y97.589
G1 X97.589 Y81.611 E.66596
G1 X97.302 Y81.365
G1 X81.365 Y97.302 E.66429
G1 X81.125 Y97.008
G1 X97.008 Y81.125 E.66204
G1 X96.708 Y80.892
G1 X80.892 Y96.708 E.65922
G1 X80.671 Y96.396
G1 X96.396 Y80.671 E.6554
G1 X96.074 Y80.46
G1 X80.46 Y96.074 E.6508
G1 X80.257 Y95.744
G1 X95.744 Y80.257 E.64553
G1 X95.403 Y80.064
G1 X80.064 Y95.403 E.63934
G1 X79.882 Y95.052
G1 X95.052 Y79.882 E.63227
G1 X94.691 Y79.709
G1 X79.709 Y94.691 E.62448
G1 X79.545 Y94.322
G1 X94.322 Y79.545 E.61592
G1 X93.937 Y79.397
G1 X79.397 Y93.937 E.60602
G1 X79.26 Y93.541
G1 X93.541 Y79.26 E.59523
G1 X93.134 Y79.134
G1 X79.134 Y93.134 E.58353
G1 X79.019 Y92.715
G1 X92.715 Y79.019 E.57087
G1 X92.28 Y78.921
G1 X78.921 Y92.28 E.55684
G1 X78.839 Y91.829
G1 X91.829 Y78.839 E.54142
G1 X91.361 Y78.774
G1 X78.774 Y91.361 E.52465
G1 X78.726 Y90.876
G1 X90.876 Y78.726 E.50644
G1 X90.367 Y78.702
G1 X78.697 Y90.372 E.48642
G1 X78.691 Y89.844
G1 X89.833 Y78.702 E.46443
G1 X89.289 Y78.713
G1 X78.713 Y89.288 E.44079
G1 X78.766 Y88.702
G1 X88.702 Y78.766 E.41416
G1 X88.08 Y78.855
G1 X78.855 Y88.08 E.38452
G1 X78.991 Y87.411
G1 X87.411 Y78.991 E.35098
G1 X86.681 Y79.187
G1 X79.187 Y86.682 E.31238
G1 X79.475 Y85.86
G1 X85.86 Y79.475 E.26612
G1 X84.89 Y79.912
G1 X79.912 Y84.89 E.20751
; WIPE_START
G1 F10342.643
M204 S6000
G1 X80.619 Y84.183 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.209 J.139 P1  F42000
G1 X80.689 Y83.58 Z8.9
G1 Z8.5
G1 E.4 F1800
G1 F6974
M204 S2000
G1 X83.58 Y80.689 E.12053
; WIPE_START
G1 F10342.643
M204 S6000
G1 X82.873 Y81.396 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I-.971 J.733 P1  F42000
G1 X96.359 Y99.251 Z8.9
G1 Z8.5
G1 E.4 F1800
; FEATURE: Gap infill
; LINE_WIDTH: 0.228243
G1 F6974
M204 S6000
G1 X96.245 Y99.347 E.00215
; LINE_WIDTH: 0.203167
G1 X96.101 Y99.463 E.00232
; LINE_WIDTH: 0.162063
G1 X95.956 Y99.578 E.00173
; LINE_WIDTH: 0.120959
G1 X95.811 Y99.694 E.00113
; LINE_WIDTH: 0.0942857
G1 X95.757 Y99.735 E.00027
M204 S10000
G1 X95.048 Y100.027 F42000
; LINE_WIDTH: 0.202583
G1 F6974
M204 S6000
G1 X94.934 Y100.109 E.00176
; LINE_WIDTH: 0.172041
G1 X94.82 Y100.192 E.00142
; LINE_WIDTH: 0.139618
G1 X94.733 Y100.252 E.0008
; LINE_WIDTH: 0.105314
G1 X94.646 Y100.311 E.00051
; WIPE_START
G1 F15000
G1 X94.733 Y100.252 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I-.338 J-1.169 P1  F42000
G1 X91.858 Y101.083 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.0943577
G1 F6974
M204 S6000
M73 P97 R0
G1 X91.622 Y101.198 E.00105
; WIPE_START
G1 F15000
G1 X91.858 Y101.083 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I.149 J-1.208 P1  F42000
G1 X85.262 Y100.27 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.107937
G1 F6974
M204 S6000
G1 X85.175 Y100.211 E.00054
G1 X85.114 Y100.222 E.00032
; WIPE_START
G1 F15000
G1 X85.175 Y100.211 E-.14147
G1 X85.262 Y100.27 E-.23853
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I.853 J-.868 P1  F42000
G1 X79.778 Y94.886 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.107972
G1 F6974
M204 S6000
G1 X79.789 Y94.825 E.00032
G1 X79.73 Y94.738 E.00054
; WIPE_START
G1 F15000
G1 X79.789 Y94.825 E-.23853
G1 X79.778 Y94.886 E-.14147
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.207 J-.154 P1  F42000
G1 X78.917 Y88.142 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.0942883
G1 F6974
M204 S6000
G1 X78.802 Y88.378 E.00105
; WIPE_START
G1 F15000
G1 X78.917 Y88.142 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.155 J.383 P1  F42000
G1 X79.973 Y84.952 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.202552
G1 F6974
M204 S6000
G1 X79.891 Y85.066 E.00176
; LINE_WIDTH: 0.171971
G1 X79.808 Y85.18 E.00142
; LINE_WIDTH: 0.139568
G1 X79.748 Y85.267 E.0008
; LINE_WIDTH: 0.105303
G1 X79.689 Y85.354 E.00051
; WIPE_START
G1 F15000
G1 X79.748 Y85.267 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.036 J.638 P1  F42000
G1 X80.749 Y83.641 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.228185
G1 F6974
M204 S6000
G1 X80.653 Y83.755 E.00216
; LINE_WIDTH: 0.203072
G1 X80.537 Y83.899 E.00232
; LINE_WIDTH: 0.161959
G1 X80.422 Y84.044 E.00172
; LINE_WIDTH: 0.120847
G1 X80.306 Y84.189 E.00113
; LINE_WIDTH: 0.0942327
G1 X80.265 Y84.243 E.00027
; WIPE_START
G1 F15000
G1 X80.306 Y84.189 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I.859 J.862 P1  F42000
G1 X84.242 Y80.265 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.0942324
G1 F6974
M204 S6000
G1 X84.189 Y80.306 E.00027
; LINE_WIDTH: 0.120849
G1 X84.044 Y80.422 E.00113
; LINE_WIDTH: 0.161957
G1 X83.899 Y80.537 E.00172
; LINE_WIDTH: 0.203066
G1 X83.755 Y80.653 E.00232
; LINE_WIDTH: 0.228192
G1 X83.641 Y80.749 E.00216
; WIPE_START
G1 F15000
G1 X83.755 Y80.653 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I.571 J1.075 P1  F42000
G1 X86.25 Y79.328 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.0983528
G1 F6974
M204 S6000
G1 X85.917 Y79.532 E.00169
; WIPE_START
G1 F15000
G1 X86.25 Y79.328 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I-.861 J.861 P1  F42000
G1 X100.673 Y93.75 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.0984463
G1 F6974
M204 S6000
G1 X100.468 Y94.083 E.0017
; WIPE_START
G1 F15000
G1 X100.673 Y93.75 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I-1.103 J-.515 P1  F42000
G1 X99.735 Y95.757 Z8.9
G1 Z8.5
G1 E.4 F1800
; LINE_WIDTH: 0.09432
G1 F6974
M204 S6000
G1 X99.694 Y95.811 E.00027
; LINE_WIDTH: 0.121019
G1 X99.578 Y95.956 E.00113
; LINE_WIDTH: 0.162119
G1 X99.463 Y96.101 E.00173
; LINE_WIDTH: 0.20322
G1 X99.347 Y96.246 E.00232
; LINE_WIDTH: 0.228264
G1 X99.251 Y96.359 E.00215
; close powerlost recovery
M1003 S0
; WIPE_START
G1 F15000
G1 X99.347 Y96.246 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z8.9 I1.217 J0 P1  F42000
M106 S0
M981 S0 P20000 ; close spaghetti detector
; FEATURE: Custom
; MACHINE_END_GCODE_START
; filament end gcode 

;===== date: 20260513 =====================
;turn off nozzle clog detect
G392 S0

M400 ; wait for buffer to clear
G92 E0 ; zero the extruder
G90
G1 Z8.9 F900 ; lower z a little
G1 X0 Y90 F18000 ; move to safe pos
G1 X-13.0 F3000 ; move to safe pos

M1002 judge_flag timelapse_record_flag
M622 J1
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M400 P100
M971 S11 C11 O0
M991 S0 P-1 ;end timelapse at safe pos
M623


M140 S0 ; turn off bed
M106 S0 ; turn off fan
M106 P2 S0 ; turn off remote part cooling fan
M106 P3 S0 ; turn off chamber cooling fan

;G1 X27 F15000 ; wipe

; pull back filament to AMS
M620 S255
G1 X181 F12000
T255
G1 X0 F18000
M73 P98 R0
G1 X-13.0 F3000
G1 X0 F18000 ; wipe
M621 S255

M104 S0 ; turn off hotend

M400 ; wait all motion done
M17 S
M17 Z0.4 ; lower z motor current to reduce impact if there is something in the bottom

    G1 Z108.5 F600
    G1 Z106.5

M400 P100
M17 R ; restore z current

G90
G1 X-13 Y180 F3600

G91
G1 Z-1 F600
G90
M83

M220 S100  ; Reset feedrate magnitude
M201.2 K1.0 ; Reset acc magnitude
M73.2   R1.0 ;Reset left time magnitude
M1002 set_gcode_claim_speed_level : 0

;=====printer finish  sound=========
M17
M400 S1
M1006 S1
M1006 A0 B20 L100 C37 D20 M100 E42 F20 N100
M1006 A0 B10 L100 C44 D10 M100 E44 F10 N100
M1006 A0 B10 L100 C46 D10 M100 E46 F10 N100
M1006 A44 B20 L100 C39 D20 M100 E48 F20 N100
M1006 A0 B10 L100 C44 D10 M100 E44 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B10 L100 C39 D10 M100 E39 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B10 L100 C44 D10 M100 E44 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A0 B10 L100 C39 D10 M100 E39 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A44 B10 L100 C0 D10 M100 E48 F10 N100
M1006 A0 B10 L100 C0 D10 M100 E0 F10 N100
M1006 A44 B20 L100 C41 D20 M100 E49 F20 N100
M1006 A0 B20 L100 C0 D20 M100 E0 F20 N100
M1006 A0 B20 L100 C37 D20 M100 E37 F20 N100
M1006 W
;=====printer finish  sound=========
M400 S1
M18 X Y Z
M73 P100 R0
; EXECUTABLE_BLOCK_END

