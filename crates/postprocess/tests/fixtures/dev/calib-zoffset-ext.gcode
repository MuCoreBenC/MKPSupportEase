; HEADER_BLOCK_START
; BambuStudio 02.08.02.60
; model printing time: 3m 34s; total estimated time: 9m 50s
; total layer number: 2
; total filament length [mm] : 305.87
; total filament volume [cm^3] : 735.71
; total filament weight [g] : 0.93
; filament_density: 1.26
; filament_diameter: 1.75
; max_z_height: 0.40
; filament: 1
; support_material_on_wipe_tower: 0
; HEADER_BLOCK_END

; CONFIG_BLOCK_START
; accel_to_decel_enable = 0
; accel_to_decel_factor = 50%
; activate_air_filtration = 0
; additional_cooling_fan_speed = 70
; additional_fan_full_speed_layer = 0
; alternate_extra_wall = 0
; ams_filament_load_time_ams = 0
; ams_filament_load_time_ams_lite = 0
; ams_filament_load_time_n3f_s = 0
; ams_filament_unload_time_ams = 0
; ams_filament_unload_time_ams_lite = 0
; ams_filament_unload_time_n3f_s = 0
; apply_scarf_seam_on_circles = 1
; auxiliary_fan = 0
; avoid_crossing_wall_includes_support = 0
; bed_custom_model = 
; bed_custom_texture = 
; bed_exclude_area = 
; bed_heat_soak_area = 
; bed_temperature_formula = by_first_filament
; before_layer_change_gcode = 
; best_object_pos = 0.5,0.5
; bottom_color_penetration_layers = 3
; bottom_shell_layers = 3
; bottom_shell_thickness = 0
; bottom_surface_density = 100%
; bottom_surface_pattern = monotonic
; bridge_angle = 0
; bridge_flow = 1
; bridge_no_support = 0
; bridge_speed = 50
; brim_object_gap = 0.1
; brim_type = auto_brim
; brim_width = 5
; chamber_temperatures = 0
; change_filament_gcode = ;===== A1 20251031 =======================\nM1007 S0 ; turn off mass estimation\nG392 S0\nM620 S[next_extruder]A\nM204 S9000\nG1 Z{max_layer_z + 3.0} F1200\n\nM400\nM106 P1 S0\nM106 P2 S0\n{if old_filament_temp > 142 && next_extruder < 255}\nM104 S[old_filament_temp]\n{endif}\n\nG1 X267 F18000\n\n{if long_retractions_when_cut[previous_extruder]}\nM620.11 S1 I[previous_extruder] E-{retraction_distances_when_cut[previous_extruder]} F1200\n{else}\nM620.11 S0\n{endif}\nM400\n\nM620.1 E F{flush_volumetric_speeds[previous_extruder]/2.4053*60} T{flush_temperatures[previous_extruder]}\nM620.10 A0 F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nT[next_extruder]\nM620.1 E F{flush_volumetric_speeds[next_extruder]/2.4053*60} T{flush_temperatures[next_extruder]}\nM620.10 A1 F{flush_volumetric_speeds[next_extruder]/2.4053*60} L[flush_length] H[nozzle_diameter] T{flush_temperatures[next_extruder]}\n\nG1 Y128 F9000\n\n{if next_extruder < 255}\n\n{if long_retractions_when_cut[previous_extruder]}\nM620.11 S1 I[previous_extruder] E{retraction_distances_when_cut[previous_extruder]} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nM628 S1\nG92 E0\nG1 E{retraction_distances_when_cut[previous_extruder]} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nM400\nM629 S1\n{else}\nM620.11 S0\n{endif}\n\nM400\nG92 E0\nM628 S0\n\n{if flush_length_1 > 1}\n; FLUSH_START\n; always use highest temperature to flush\nM400\nM1002 set_filament_type:UNKNOWN\nM109 S[flush_temperatures[next_extruder]]\nM106 P1 S60\n{if flush_length_1 > 23.7}\nG1 E23.7 F{flush_volumetric_speeds[previous_extruder]/2.4053*60} ; do not need pulsatile flushing for start part\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{(flush_length_1 - 23.7) * 0.02} F50\nG1 E{(flush_length_1 - 23.7) * 0.23} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\n{else}\nG1 E{flush_length_1} F{flush_volumetric_speeds[previous_extruder]/2.4053*60}\n{endif}\n; FLUSH_END\nG1 E-[old_retract_length_toolchange] F1800\nG1 E[old_retract_length_toolchange] F300\nM400\nM1002 set_filament_type:{filament_type[next_extruder]}\n{endif}\n\n{if flush_length_1 > 45 && flush_length_2 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_2 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\nG1 E{flush_length_2 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_2 * 0.02} F50\n; FLUSH_END\nG1 E-[new_retract_length_toolchange] F1800\nG1 E[new_retract_length_toolchange] F300\n{endif}\n\n{if flush_length_2 > 45 && flush_length_3 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_3 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\nG1 E{flush_length_3 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_3 * 0.02} F50\n; FLUSH_END\nG1 E-[new_retract_length_toolchange] F1800\nG1 E[new_retract_length_toolchange] F300\n{endif}\n\n{if flush_length_3 > 45 && flush_length_4 > 1}\n; WIPE\nM400\nM106 P1 S178\nM400 S3\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nM400\nM106 P1 S0\n{endif}\n\n{if flush_length_4 > 1}\nM106 P1 S60\n; FLUSH_START\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\nG1 E{flush_length_4 * 0.18} F{flush_volumetric_speeds[next_extruder]/2.4053*60}\nG1 E{flush_length_4 * 0.02} F50\n; FLUSH_END\n{endif}\n\nM629\n\nM400\nM106 P1 S60\nM109 S[new_filament_temp]\nG1 E6 F{flush_volumetric_speeds[next_extruder]/2.4053*60} ;Compensate for filament spillage during waiting temperature\nM400\nG92 E0\nG1 E-[new_retract_length_toolchange] F1800\nM400\nM106 P1 S178\nM400 S3\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nG1 X-38.2 F18000\nG1 X-48.2 F3000\nM400\nG1 Z{max_layer_z + 3.0} F3000\nM106 P1 S0\n{if layer_z <= (initial_layer_print_height + 0.001)}\nM204 S[initial_layer_acceleration]\n{else}\nM204 S[default_acceleration]\n{endif}\n{else}\nG1 X[x_after_toolchange] Y[y_after_toolchange] Z[z_after_toolchange] F12000\n{endif}\n\nM622.1 S0\nM9833 F{outer_wall_volumetric_speed/2.4} A0.3 ; cali dynamic extrusion compensation\nM1002 judge_flag filament_need_cali_flag\nM622 J1\n  G92 E0\n  G1 E-[new_retract_length_toolchange] F1800\n  M400\n  \n  M106 P1 S178\n  M400 S4\n  G1 X-38.2 F18000\n  G1 X-48.2 F3000\n  G1 X-38.2 F18000 ;wipe and shake\n  G1 X-48.2 F3000\n  G1 X-38.2 F12000 ;wipe and shake\n  G1 X-48.2 F3000\n  M400\n  M106 P1 S0 \nM623\n\nM621 S[next_extruder]A\nG392 S0\n\nM1007 S1\n
; circle_compensation_manual_offset = 0
; circle_compensation_speed = 200
; close_additional_fan_first_x_layers = 1
; close_fan_the_first_x_layers = 1
; complete_print_exhaust_fan_speed = 70
; cool_plate_temp = 35
; cool_plate_temp_initial_layer = 35
; cooling_filter_enabled = 0
; cooling_perimeter_transition_distance = 10
; cooling_slowdown_logic = uniform_cooling
; counter_coef_1 = 0
; counter_coef_2 = 0.008
; counter_coef_3 = -0.041
; counter_limit_max = 0.033
; counter_limit_min = -0.035
; counterbore_hole_bridging = none
; curr_bed_type = Textured PEI Plate
; default_acceleration = 6000
; default_ams_type = -1
; default_filament_colour = ""
; default_filament_profile = "Bambu PLA Basic @BBL A1"
; default_jerk = 0
; default_nozzle_volume_type = Standard
; default_print_profile = 0.20mm Standard @BBL A1
; deretraction_speed = 30
; detect_floating_vertical_shell = 1
; detect_narrow_internal_solid_infill = 1
; detect_overhang_wall = 1
; detect_thin_wall = 0
; diameter_limit = 50
; different_settings_to_system = ;;
; draft_shield = disabled
; during_print_exhaust_fan_speed = 70
; elefant_foot_compensation = 0.075
; embedding_wall_into_infill = 0
; enable_arc_fitting = 1
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
; enable_prime_tower = 1
; enable_support = 0
; enable_support_ironing = 0
; enable_tower_interface_features = 0
; enable_wrapping_detection = 0
; enforce_support_layers = 0
; eng_plate_temp = 0
; eng_plate_temp_initial_layer = 0
; ensure_vertical_shell_thickness = enabled
; exclude_object = 1
; extruder_ams_count = 1#0|4#0;1#0|4#0
; extruder_clearance_dist_to_rod = 56.5
; extruder_clearance_height_to_lid = 256
; extruder_clearance_height_to_rod = 25
; extruder_clearance_max_radius = 73
; extruder_colour = #018001
; extruder_max_nozzle_count = 1
; extruder_nozzle_stats = Standard#1
; extruder_nozzle_stats_new = Standard#1
; extruder_offset = 0x0
; extruder_printable_area = 
; extruder_type = Direct Drive
; extruder_variant_list = "Direct Drive Standard"
; fan_cooling_layer_time = 80
; fan_direction = undefine
; fan_max_speed = 80
; fan_min_speed = 60
; farthest_point_timelapse = 0
; filament_adaptive_volumetric_speed = 0
; filament_adhesiveness_category = 100
; filament_bridge_speed = 25
; filament_change_length = 5
; filament_change_length_nc = 10
; filament_colour = #FFFFFF
; filament_colour_type = 0
; filament_cooling_before_tower = 0
; filament_cost = 19.99
; filament_density = 1.26
; filament_dev_ams_drying_ams_limitations = 1;0
; filament_dev_ams_drying_heat_distortion_temperature = 45
; filament_dev_ams_drying_temperature = 45,45,45,45
; filament_dev_ams_drying_time = 12,12,12,12
; filament_dev_chamber_drying_bed_temperature = 70
; filament_dev_chamber_drying_time = 12
; filament_dev_drying_cooling_temperature = 45
; filament_dev_drying_softening_temperature = 50
; filament_diameter = 1.75
; filament_enable_overhang_speed = 1
; filament_end_gcode = "; filament end gcode \n\n"
; filament_extruder_compatibility = 0
; filament_extruder_variant = "Direct Drive Standard"
; filament_flow_ratio = 0.98
; filament_flush_temp = 0
; filament_flush_temp_fast = 0
; filament_flush_volumetric_speed = 0
; filament_ids = GFA00
; filament_is_mixed = 0
; filament_is_support = 0
; filament_long_retractions_when_cut = 1
; filament_map = 1
; filament_map_2 = 0
; filament_map_mode = Auto For Flush
; filament_max_volumetric_speed = 21
; filament_metal_stickiness = None
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
; filament_scarf_gap = 0%
; filament_scarf_height = 10%
; filament_scarf_length = 10
; filament_scarf_seam_type = none
; filament_self_index = 1
; filament_settings_id = "Bambu PLA Basic @BBL A1"
; filament_shrink = 100%
; filament_soluble = 0
; filament_start_gcode = "; filament start gcode\n{if  (bed_temperature[current_extruder] >55)||(bed_temperature_initial_layer[current_extruder] >55)}M106 P3 S200\n{elsif(bed_temperature[current_extruder] >50)||(bed_temperature_initial_layer[current_extruder] >50)}M106 P3 S150\n{elsif(bed_temperature[current_extruder] >45)||(bed_temperature_initial_layer[current_extruder] >45)}M106 P3 S50\n{endif}\n\n{if activate_air_filtration[current_extruder] && support_air_filtration}\nM106 P3 S{during_print_exhaust_fan_speed_num[current_extruder]} \n{endif}"
; filament_tower_interface_pre_extrusion_dist = 10
; filament_tower_interface_pre_extrusion_length = 0
; filament_tower_interface_print_temp = -1
; filament_tower_interface_purge_volume = 20
; filament_tower_ironing_area = 4
; filament_type = PLA
; filament_velocity_adaptation_factor = 1
; filament_vendor = "Bambu Lab"
; filament_volume_map = 0
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
; head_wrap_detect_zone = 226x224,256x224,256x256,226x256
; hole_coef_1 = 0
; hole_coef_2 = -0.008
; hole_coef_3 = 0.23415
; hole_limit_max = 0.22
; hole_limit_min = 0.088
; host_type = octoprint
; hot_plate_temp = 65
; hot_plate_temp_initial_layer = 65
; hotend_cooling_rate = 2
; hotend_heating_rate = 2
; impact_strength_z = 13.8
; independent_support_layer_height = 1
; infill_combination = 0
; infill_direction = 45
; infill_instead_top_bottom_surfaces = 0
; infill_jerk = 9
; infill_lock_depth = 1
; infill_rotate_step = 0
; infill_shift_step = 0.4
; infill_wall_overlap = 15%
; inherits_group = ;;
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
; machine_end_gcode = ;===== date: 20260513 =====================\nG392 S0 ;turn off nozzle clog detect\n\nM400 ; wait for buffer to clear\nG92 E0 ; zero the extruder\nG90\nG1 Z{max_layer_z + 0.4} F900 ; lower z a little\nG1 X0 Y{first_layer_center_no_wipe_tower[1]} F18000 ; move to safe pos\nG1 X-13.0 F3000 ; move to safe pos\n{if !spiral_mode && print_sequence != \"by object\"}\nM1002 judge_flag timelapse_record_flag\nM622 J1\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM400 P100\nM971 S11 C11 O0\nM991 S0 P-1 ;end timelapse at safe pos\nM623\n{endif}\n\nM140 S0 ; turn off bed\nM106 S0 ; turn off fan\nM106 P2 S0 ; turn off remote part cooling fan\nM106 P3 S0 ; turn off chamber cooling fan\n\n;G1 X27 F15000 ; wipe\n\n; pull back filament to AMS\nM620 S255\nG1 X267 F15000\nT255\nG1 X-28.5 F18000\nG1 X-48.2 F3000\nG1 X-28.5 F18000\nG1 X-48.2 F3000\nM621 S255\n\nM104 S0 ; turn off hotend\n\nM400 ; wait all motion done\nM17 S\nM17 Z0.4 ; lower z motor current to reduce impact if there is something in the bottom\n{if (max_layer_z + 100.0) < 256}\n    G1 Z{max_layer_z + 100.0} F600\n    G1 Z{max_layer_z +98.0}\n{else}\n    G1 Z256 F600\n    G1 Z256\n{endif}\nM400 P100\nM17 R ; restore z current\n\nG90\nG1 X-48 Y180 F3600\n\nM220 S100  ; Reset feedrate magnitude\nM201.2 K1.0 ; Reset acc magnitude\nM73.2   R1.0 ;Reset left time magnitude\nM1002 set_gcode_claim_speed_level : 0\n\n;=====printer finish  sound=========\nM17\nM400 S1\nM1006 S1\nM1006 A0 B20 L100 C37 D20 M40 E42 F20 N60\nM1006 A0 B10 L100 C44 D10 M60 E44 F10 N60\nM1006 A0 B10 L100 C46 D10 M80 E46 F10 N80\nM1006 A44 B20 L100 C39 D20 M60 E48 F20 N60\nM1006 A0 B10 L100 C44 D10 M60 E44 F10 N60\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N60\nM1006 A0 B10 L100 C39 D10 M60 E39 F10 N60\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N60\nM1006 A0 B10 L100 C44 D10 M60 E44 F10 N60\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N60\nM1006 A0 B10 L100 C39 D10 M60 E39 F10 N60\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N60\nM1006 A0 B10 L100 C48 D10 M60 E44 F10 N80\nM1006 A0 B10 L100 C0 D10 M60 E0 F10  N80\nM1006 A44 B20 L100 C49 D20 M80 E41 F20 N80\nM1006 A0 B20 L100 C0 D20 M60 E0 F20 N80\nM1006 A0 B20 L100 C37 D20 M30 E37 F20 N60\nM1006 W\n;=====printer finish  sound=========\n\n;M17 X0.8 Y0.8 Z0.5 ; lower motor current to 45% power\nM400\nM18 X Y Z\n\n
; machine_hotend_change_time = 0
; machine_load_filament_time = 25
; machine_max_acceleration_e = 5000,5000
; machine_max_acceleration_extruding = 12000,12000
; machine_max_acceleration_retracting = 5000,5000
; machine_max_acceleration_travel = 9000,9000
; machine_max_acceleration_x = 12000,12000
; machine_max_acceleration_y = 12000,12000
; machine_max_acceleration_z = 1500,1500
; machine_max_force_Y = 0
; machine_max_jerk_e = 3,3
; machine_max_jerk_x = 9,9
; machine_max_jerk_y = 9,9
; machine_max_jerk_z = 3,3
; machine_max_printed_mass = 0
; machine_max_speed_e = 30,30
; machine_max_speed_x = 500,200
; machine_max_speed_y = 500,200
; machine_max_speed_z = 30,30
; machine_min_extruding_rate = 0,0
; machine_min_travel_rate = 0,0
; machine_pause_gcode = M400 U1
; machine_prepare_compensation_time = 260
; machine_start_gcode = ;===== machine: A1 =========================\n;===== date: 20260513 ==================\nG392 S0\nM9833.2\n;M400\n;M73 P1.717\n\n;===== start to heat heatbead&hotend==========\nM1002 gcode_claim_action : 2\nM1002 set_filament_type:{filament_type[initial_no_support_extruder]}\nM104 S140\nM140 S[bed_temperature_initial_layer_single]\n\n;=====start printer sound ===================\nM17\nM400 S1\nM1006 S1\nM1006 A0 B10 L100 C37 D10 M60 E37 F10 N60\nM1006 A0 B10 L100 C41 D10 M60 E41 F10 N60\nM1006 A0 B10 L100 C44 D10 M60 E44 F10 N60\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N60\nM1006 A43 B10 L100 C46 D10 M70 E39 F10 N80\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N80\nM1006 A0 B10 L100 C43 D10 M60 E39 F10 N80\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N80\nM1006 A0 B10 L100 C41 D10 M80 E41 F10 N80\nM1006 A0 B10 L100 C44 D10 M80 E44 F10 N80\nM1006 A0 B10 L100 C49 D10 M80 E49 F10 N80\nM1006 A0 B10 L100 C0 D10 M80 E0 F10 N80\nM1006 A44 B10 L100 C48 D10 M60 E39 F10 N80\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N80\nM1006 A0 B10 L100 C44 D10 M80 E39 F10 N80\nM1006 A0 B10 L100 C0 D10 M60 E0 F10 N80\nM1006 A43 B10 L100 C46 D10 M60 E39 F10 N80\nM1006 W\nM18 \n;=====start printer sound ===================\n\n;=====avoid end stop =================\nG91\nG380 S2 Z40 F1200\nG380 S3 Z-15 F1200\nG90\n\n;===== reset machine status =================\n;M290 X39 Y39 Z8\nM204 S6000\n\nM630 S0 P0\nG91\nM17 Z0.3 ; lower the z-motor current\n\nG90\nM17 X0.65 Y1.2 Z0.6 ; reset motor current to default\nM960 S5 P1 ; turn on logo lamp\nG90\nM220 S100 ;Reset Feedrate\nM221 S100 ;Reset Flowrate\nM73.2   R1.0 ;Reset left time magnitude\n;M211 X0 Y0 Z0 ; turn off soft endstop to prevent protential logic problem\n\n;====== cog noise reduction=================\nM982.2 S1 ; turn on cog noise reduction\n\nM1002 gcode_claim_action : 13\n\nG28 X\nG91\nG1 Z5 F1200\nG90\nG0 X128 F30000\nG0 Y254 F3000\nG91\nG1 Z-5 F1200\n\nM109 S25 H140\n\nM17 E0.3\nM83\nG1 E10 F1200\nG1 E-0.5 F30\nM17 D\n\nG28 Z P0 T140; home z with low precision,permit 300deg temperature\nM104 S{nozzle_temperature_initial_layer[initial_extruder]}\n\nM1002 judge_flag build_plate_detect_flag\nM622 S1\n  G39.4\n  G90\n  G1 Z5 F1200\nM623\n\n;M400\n;M73 P1.717\n\n;===== prepare print temperature and material ==========\nM1002 gcode_claim_action : 24\n\nM400\n;G392 S1\nM211 X0 Y0 Z0 ;turn off soft endstop\nM975 S1 ; turn on\n\nG90\nG1 X-28.5 F30000\nG1 X-48.2 F3000\n\nM620 M ;enable remap\nM620 S[initial_no_support_extruder]A   ; switch material if AMS exist\n    M1002 gcode_claim_action : 4\n    M400\n    M1002 set_filament_type:UNKNOWN\n    M109 S[nozzle_temperature_initial_layer]\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M104 S220\n{else}\n    M104 S250\n{endif}\n    M400\n    T[initial_no_support_extruder]\n    G1 X-48.2 F3000\n    M400\n\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M620.1 E F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60} T220\n    M109 S220 ;set nozzle to common flush temp\n{else}\n    M620.1 E F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60} T{flush_temperatures[initial_no_support_extruder]}\n    M109 S250 ;set nozzle to common flush temp\n{endif}\n    M106 P1 S0\n    G92 E0\n    G1 E50 F200\n    M400\n    M1002 set_filament_type:{filament_type[initial_no_support_extruder]}\nM621 S[initial_no_support_extruder]A\n\n{if (filament_type[initial_no_support_extruder] == \"PLA\") && (nozzle_diameter != 0.2)}\n    M109 S220 H300\n{else}\n    M109 S{flush_temperatures[initial_no_support_extruder]} H300\n{endif}\nG92 E0\nG1 E50 F200 ; lower extrusion speed to avoid clog\nM400\nM106 P1 S178\nG92 E0\nG1 E5 F200\nM104 S{nozzle_temperature_initial_layer[initial_no_support_extruder]}\nG92 E0\nG1 E-0.5 F300\n\nG1 X-28.5 F30000\nG1 X-48.2 F3000\nG1 X-28.5 F30000 ;wipe and shake\nG1 X-48.2 F3000\nG1 X-28.5 F30000 ;wipe and shake\nG1 X-48.2 F3000\n\n;G392 S0\n\nM400\nM106 P1 S0\n;===== prepare print temperature and material end =====\n\n;M400\n;M73 P1.717\n\n;===== auto extrude cali start =========================\nM975 S1\n;G392 S1\n\nG90\nM83\nT1000\nG1 X-48.2 Y0 Z10 F10000\nM400\nM1002 set_filament_type:UNKNOWN\n\nM412 S1 ;  ===turn on  filament runout detection===\nM400 P10\nM620.3 W1; === turn on filament tangle detection===\nM400 S2\n\nM1002 set_filament_type:{filament_type[initial_no_support_extruder]}\n\n;M1002 set_flag extrude_cali_flag=1\nM1002 judge_flag extrude_cali_flag\n\nM622 J1\n    M1002 gcode_claim_action : 8\n\n    M109 S{nozzle_temperature[initial_extruder]}\n    G1 E10 F{outer_wall_volumetric_speed/2.4*60}\n    M983 F{outer_wall_volumetric_speed/2.4} A0.3 H[nozzle_diameter]; cali dynamic extrusion compensation\n\n    M106 P1 S255\n    M400 S5\n    G1 X-28.5 F18000\n    G1 X-48.2 F3000\n    G1 X-28.5 F18000 ;wipe and shake\n    G1 X-48.2 F3000\n    G1 X-28.5 F12000 ;wipe and shake\n    G1 X-48.2 F3000\n    M400\n    M106 P1 S0\n\n    M1002 judge_last_extrude_cali_success\n    M622 J0\n        M983 F{outer_wall_volumetric_speed/2.4} A0.3 H[nozzle_diameter]; cali dynamic extrusion compensation\n        M106 P1 S255\n        M400 S5\n        G1 X-28.5 F18000\n        G1 X-48.2 F3000\n        G1 X-28.5 F18000 ;wipe and shake\n        G1 X-48.2 F3000\n        G1 X-28.5 F12000 ;wipe and shake\n        M400\n        M106 P1 S0\n    M623\n    \n    G1 X-48.2 F3000\n    M400\n    M984 A0.1 E1 S1 F{outer_wall_volumetric_speed/2.4} H[nozzle_diameter]\n    M106 P1 S178\n    M400 S7\n    G1 X-28.5 F18000\n    G1 X-48.2 F3000\n    G1 X-28.5 F18000 ;wipe and shake\n    G1 X-48.2 F3000\n    G1 X-28.5 F12000 ;wipe and shake\n    G1 X-48.2 F3000\n    M400\n    M106 P1 S0\nM623 ; end of \"draw extrinsic para cali paint\"\n\n;G392 S0\n;===== auto extrude cali end ========================\n\n;M400\n;M73 P1.717\n\nM104 S170 ; prepare to wipe nozzle\nM106 S255 ; turn on fan\n\n;===== mech mode fast check start =====================\nM1002 gcode_claim_action : 3\n\nG1 X128 Y128 F20000\nG1 Z5 F1200\nM400 P200\nM970.3 Q1 A5 K0 O3\nM974 Q1 S2 P0\n\nM970.2 Q1 K1 W58 Z0.1\nM974 S2\n\nG1 X128 Y128 F20000\nG1 Z5 F1200\nM400 P200\nM970.3 Q0 A10 K0 O1\nM974 Q0 S2 P0\n\nM970.2 Q0 K1 W78 Z0.1\nM974 S2\n\nM975 S1\nG1 F30000\nG1 X0 Y5\nG28 X ; re-home XY\n\nG1 Z4 F1200\n\n;===== mech mode fast check end =======================\n\n;M400\n;M73 P1.717\n\n;===== wipe nozzle ===============================\nM1002 gcode_claim_action : 14\n\nM975 S1\nM106 S255 ; turn on fan (G28 has turn off fan)\nM211 S; push soft endstop status\nM211 X0 Y0 Z0 ;turn off Z axis endstop\n\n;===== remove waste by touching start =====\n\nM104 S170 ; set temp down to heatbed acceptable\n\nM83\nG1 E-1 F500\nG90\nM83\n\nM109 S170\nG0 X108 Y-0.5 F30000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X110 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X112 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X114 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X116 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X118 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X120 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X122 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X124 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X126 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X128 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X130 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X132 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X134 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X136 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X138 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X140 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X142 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X144 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X146 F10000\nG380 S3 Z-5 F1200\nG1 Z2 F1200\nG1 X148 F10000\nG380 S3 Z-5 F1200\n\nG1 Z5 F30000\n;===== remove waste by touching end =====\n\nG1 Z10 F1200\nG0 X118 Y261 F30000\nG1 Z5 F1200\nM109 S{nozzle_temperature_initial_layer[initial_extruder]-50}\n\nG28 Z P0 T300; home z with low precision,permit 300deg temperature\nG29.2 S0 ; turn off ABL\nM104 S140 ; prepare to abl\nG0 Z5 F20000\n\nG0 X128 Y261 F20000  ; move to exposed steel surface\nG0 Z-1.01 F1200      ; stop the nozzle\n\nG91\nG2 I1 J0 X2 Y0 F2000.1\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\n\nG90\nG1 Z10 F1200\n\n;===== brush material wipe nozzle =====\n\nG90\nG1 Y250 F30000\nG1 X55\nG1 Z1.300 F1200\nG1 Y262.5 F6000\nG91\nG1 X-35 F30000\nG1 Y-0.5\nG1 X45\nG1 Y-0.5\nG1 X-45\nG1 Y-0.5\nG1 X45\nG1 Y-0.5\nG1 X-45\nG1 Y-0.5\nG1 X45\nG1 Z5.000 F1200\n\nG90\nG1 X30 Y250.000 F30000\nG1 Z1.300 F1200\nG1 Y262.5 F6000\nG91\nG1 X35 F30000\nG1 Y-0.5\nG1 X-45\nG1 Y-0.5\nG1 X45\nG1 Y-0.5\nG1 X-45\nG1 Y-0.5\nG1 X45\nG1 Y-0.5\nG1 X-45\nG1 Z10.000 F1200\n\n;===== brush material wipe nozzle end =====\n\nG90\n;G0 X128 Y261 F20000  ; move to exposed steel surface\nG1 Y250 F30000\nG1 X138\nG1 Y261\nG0 Z-1.01 F1200      ; stop the nozzle\n\nG91\nG2 I1 J0 X2 Y0 F2000.1\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\nG2 I1 J0 X2\nG2 I-0.75 J0 X-1.5\n\nM109 S140\nM106 S255 ; turn on fan (G28 has turn off fan)\n\nM211 R; pop softend status\n\n;===== wipe nozzle end ================================\n\n;M400\n;M73 P1.717\n\n;===== bed leveling ==================================\nM1002 judge_flag g29_before_print_flag\n\nG90\nG1 Z5 F1200\nG1 X0 Y0 F30000\nG29.2 S1 ; turn on ABL\n\nM190 S[bed_temperature_initial_layer_single]; ensure bed temp\nM109 S140\nM106 S0 ; turn off fan , too noisy\n\nM622 J1\n    M1002 gcode_claim_action : 1\n    G29 A1 X{first_layer_print_min[0]} Y{first_layer_print_min[1]} I{first_layer_print_size[0]} J{first_layer_print_size[1]}\n    M400\n    M500 ; save cali data\nM623\n;===== bed leveling end ================================\n\n;===== home after wipe mouth============================\nM1002 judge_flag g29_before_print_flag\nM622 J0\n\n    M1002 gcode_claim_action : 13\n    G28\n\nM623\n\n;===== home after wipe mouth end =======================\n\n;M400\n;M73 P1.717\n\nG1 X108.000 Y-0.500 F30000\nG1 Z0.300 F1200\nM400\nG2814 Z0.32\n\nM104 S{nozzle_temperature_initial_layer[initial_extruder]} ; prepare to print\n\n;===== nozzle load line ===============================\n;G90\n;M83\n;G1 Z5 F1200\n;G1 X88 Y-0.5 F20000\n;G1 Z0.3 F1200\n\n;M109 S{nozzle_temperature_initial_layer[initial_extruder]}\n\n;G1 E2 F300\n;G1 X168 E4.989 F6000\n;G1 Z1 F1200\n;===== nozzle load line end ===========================\n\n;===== extrude cali test ===============================\n\nM400\n    M900 S\n    M900 C\n    G90\n    M83\n\n    M109 S{nozzle_temperature_initial_layer[initial_extruder]}\n    G0 X128 E8  F{outer_wall_volumetric_speed/(24/20)    * 60}\n    G0 X133 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X138 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X143 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X148 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X153 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G91\n    G1 X1 Z-0.300\n    G1 X4\n    G1 Z1 F1200\n    G90\n    M400\n\nM900 R\n\nM1002 judge_flag extrude_cali_flag\nM622 J1\n    G90\n    G1 X108.000 Y1.000 F30000\n    G91\n    G1 Z-0.700 F1200\n    G90\n    M83\n    G0 X128 E10  F{outer_wall_volumetric_speed/(24/20)    * 60}\n    G0 X133 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X138 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X143 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G0 X148 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)     * 60}\n    G0 X153 E.3742  F{outer_wall_volumetric_speed/(0.3*0.5)/4     * 60}\n    G91\n    G1 X1 Z-0.300\n    G1 X4\n    G1 Z1 F1200\n    G90\n    M400\nM623\n\nG1 Z0.2\n\n;M400\n;M73 P1.717\n\n;========turn off light and wait extrude temperature =============\nM1002 gcode_claim_action : 0\nM400\n\n;===== for Textured PEI Plate , lower the nozzle as the nozzle was touching topmost of the texture when homing ==\n;curr_bed_type={curr_bed_type}\n{if curr_bed_type==\"Textured PEI Plate\"}\nG29.1 Z{-0.02} ; for Textured PEI Plate\n{endif}\n\nM960 S1 P0 ; turn off laser\nM960 S2 P0 ; turn off laser\nM106 S0 ; turn off fan\nM106 P2 S0 ; turn off big fan\nM106 P3 S0 ; turn off chamber fan\n\nM975 S1 ; turn on mech mode supression\nG90\nM83\nT1000\n\nM211 X0 Y0 Z0 ;turn off soft endstop\n;G392 S1 ; turn on clog detection\nM1007 S1 ; turn on mass estimation\nG29.4\n
; machine_switch_extruder_time = 0
; machine_unload_filament_time = 29
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
; nozzle_temperature = 220
; nozzle_temperature_initial_layer = 220
; nozzle_temperature_range_high = 240
; nozzle_temperature_range_low = 190
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
; overhang_fan_speed = 100
; overhang_fan_threshold = 50%
; overhang_threshold_participating_cooling = 95%
; overhang_totally_speed = 10
; override_filament_scarf_seam_setting = 0
; override_process_overhang_speed = 0
; physical_extruder_map = 0
; post_process = 
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
; print_compatible_printers = "Bambu Lab A1 0.4 nozzle"
; print_extruder_id = 1
; print_extruder_variant = "Direct Drive Standard"
; print_flow_ratio = 1
; print_in_clockwise = 0
; print_sequence = by layer
; print_settings_id = 0.20mm Standard @BBL A1
; printable_area = 0x0,256x0,256x256,0x256
; printable_height = 256
; printer_extruder_id = 1
; printer_extruder_variant = "Direct Drive Standard"
; printer_model = Bambu Lab A1
; printer_notes = 
; printer_settings_id = Bambu Lab A1 0.4 nozzle
; printer_structure = i3
; printer_technology = FFF
; printer_variant = 0.4
; printhost_authorization_type = key
; printhost_ssl_ignore_revoke = 0
; printing_by_object_gcode = 
; process_notes = 
; raft_contact_distance = 0.1
; raft_expansion = 1.5
; raft_first_layer_density = 90%
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
; retract_lift_below = 255
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
; slow_down_layer_time = 6
; slow_down_min_speed = 20
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
; supertack_plate_temp = 45
; supertack_plate_temp_initial_layer = 45
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
; support_interface_bottom_layers = 2
; support_interface_filament = 0
; support_interface_loop_pattern = 0
; support_interface_not_for_body = 1
; support_interface_pattern = auto
; support_interface_spacing = 0.5
; support_interface_speed = 80
; support_interface_top_layers = 2
; support_ironing_direction = 0
; support_ironing_flow = 10%
; support_ironing_inset = 0
; support_ironing_pattern = zig-zag
; support_ironing_spacing = 0.15
; support_ironing_speed = 30
; support_line_width = 0.42
; support_object_first_layer_gap = 0.2
; support_object_skip_flush = 0
; support_object_xy_distance = 0.35
; support_on_build_plate_only = 0
; support_remove_small_overhang = 1
; support_speed = 150
; support_style = default
; support_threshold_angle = 30
; support_top_z_distance = 0.2
; support_type = tree(auto)
; symmetric_infill_y_axis = 0
; temperature_vitrification = 45
; template_custom_gcode = 
; textured_plate_temp = 65
; textured_plate_temp_initial_layer = 65
; thick_bridges = 0
; thumbnail_size = 50x50
; time_lapse_gcode = ;===================== date: 20250206 =====================\n{if !spiral_mode && print_sequence != \"by object\"}\n; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer\n; SKIPPABLE_START\n; SKIPTYPE: timelapse\nM622.1 S1 ; for prev firmware, default turned on\nM1002 judge_flag timelapse_record_flag\nM622 J1\nG92 E0\nG1 Z{max_layer_z + 0.4}\nG1 X0 Y{first_layer_center_no_wipe_tower[1]} F18000 ; move to safe pos\nG1 X-48.2 F3000 ; move to safe pos\nM400\nM1004 S5 P1  ; external shutter\nM400 P300\nM971 S11 C11 O0\nG92 E0\nG1 X0 F18000\nM623\n\n; SKIPTYPE: head_wrap_detect\nM622.1 S1\nM1002 judge_flag g39_3rd_layer_detect_flag\nM622 J1\n    ; enable nozzle clog detect at 3rd layer\n    {if layer_num == 2}\n      M400\n      G90\n      M83\n      M204 S5000\n      G0 Z2 F4000\n      G0 X261 Y250 F20000\n      M400 P200\n      G39 S1\n      G0 Z2 F4000\n    {endif}\n\n\n    M622.1 S1\n    M1002 judge_flag g39_detection_flag\n    M622 J1\n      {if !in_head_wrap_detect_zone}\n        M622.1 S0\n        M1002 judge_flag g39_mass_exceed_flag\n        M622 J1\n        {if layer_num > 2}\n            G392 S0\n            M400\n            G90\n            M83\n            M204 S5000\n            G0 Z{max_layer_z + 0.4} F4000\n            G39.3 S1\n            G0 Z{max_layer_z + 0.4} F4000\n            G392 S0\n          {endif}\n        M623\n    {endif}\n    M623\nM623\n; SKIPPABLE_END\n{endif}\n
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
; tree_support_wall_count = -1
; upward_compatible_machine = "Bambu Lab H2D 0.4 nozzle";"Bambu Lab H2D Pro 0.4 nozzle";"Bambu Lab H2S 0.4 nozzle";"Bambu Lab P2S 0.4 nozzle";"Bambu Lab H2C 0.4 nozzle";"Bambu Lab X2D 0.4 nozzle";"Bambu Lab A2L 0.4 nozzle"
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
; wipe_tower_x = 15
; wipe_tower_y = 220.697
; wrapping_detection_gcode = 
; wrapping_detection_layers = 20
; wrapping_exclude_area = 
; xy_contour_compensation = 0
; xy_hole_compensation = 0
; z_direction_outwall_speed_continuous = 0
; z_hop = 0.4
; z_hop_types = Auto Lift
; CONFIG_BLOCK_END

; EXECUTABLE_BLOCK_START
M73 P0 R9
M201 X12000 Y12000 Z1500 E5000
M203 X500 Y500 Z30 E30
M204 P12000 R5000 T12000
M205 X9.00 Y9.00 Z3.00 E3.00
M106 S0
; FEATURE: Custom
;===== machine: A1 =========================
;===== date: 20260513 ==================
G392 S0
M9833.2
;M400
;M73 P1.717

;===== start to heat heatbead&hotend==========
M1002 gcode_claim_action : 2
M1002 set_filament_type:PLA
M104 S140
M140 S65

;=====start printer sound ===================
M17
M400 S1
M1006 S1
M1006 A0 B10 L100 C37 D10 M60 E37 F10 N60
M1006 A0 B10 L100 C41 D10 M60 E41 F10 N60
M1006 A0 B10 L100 C44 D10 M60 E44 F10 N60
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N60
M1006 A43 B10 L100 C46 D10 M70 E39 F10 N80
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N80
M1006 A0 B10 L100 C43 D10 M60 E39 F10 N80
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N80
M1006 A0 B10 L100 C41 D10 M80 E41 F10 N80
M1006 A0 B10 L100 C44 D10 M80 E44 F10 N80
M1006 A0 B10 L100 C49 D10 M80 E49 F10 N80
M1006 A0 B10 L100 C0 D10 M80 E0 F10 N80
M1006 A44 B10 L100 C48 D10 M60 E39 F10 N80
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N80
M1006 A0 B10 L100 C44 D10 M80 E39 F10 N80
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N80
M1006 A43 B10 L100 C46 D10 M60 E39 F10 N80
M1006 W
M18 
;=====start printer sound ===================

;=====avoid end stop =================
G91
G380 S2 Z40 F1200
G380 S3 Z-15 F1200
G90

;===== reset machine status =================
;M290 X39 Y39 Z8
M204 S6000

M630 S0 P0
G91
M17 Z0.3 ; lower the z-motor current

G90
M17 X0.65 Y1.2 Z0.6 ; reset motor current to default
M960 S5 P1 ; turn on logo lamp
G90
M220 S100 ;Reset Feedrate
M221 S100 ;Reset Flowrate
M73.2   R1.0 ;Reset left time magnitude
;M211 X0 Y0 Z0 ; turn off soft endstop to prevent protential logic problem

;====== cog noise reduction=================
M982.2 S1 ; turn on cog noise reduction

M1002 gcode_claim_action : 13

G28 X
G91
G1 Z5 F1200
G90
G0 X128 F30000
G0 Y254 F3000
G91
G1 Z-5 F1200

M109 S25 H140

M17 E0.3
M83
G1 E10 F1200
G1 E-0.5 F30
M17 D

G28 Z P0 T140; home z with low precision,permit 300deg temperature
M104 S220

M1002 judge_flag build_plate_detect_flag
M622 S1
  G39.4
  G90
M73 P1 R9
  G1 Z5 F1200
M623

;M400
;M73 P1.717

;===== prepare print temperature and material ==========
M1002 gcode_claim_action : 24

M400
;G392 S1
M211 X0 Y0 Z0 ;turn off soft endstop
M975 S1 ; turn on

G90
G1 X-28.5 F30000
G1 X-48.2 F3000

M620 M ;enable remap
M620 S0A   ; switch material if AMS exist
    M1002 gcode_claim_action : 4
    M400
    M1002 set_filament_type:UNKNOWN
    M109 S220

    M104 S220

    M400
    T0
    G1 X-48.2 F3000
    M400


    M620.1 E F523.843 T220
    M109 S220 ;set nozzle to common flush temp

    M106 P1 S0
    G92 E0
    G1 E50 F200
    M400
    M1002 set_filament_type:PLA
M621 S0A


    M109 S220 H300

G92 E0
G1 E50 F200 ; lower extrusion speed to avoid clog
M400
M106 P1 S178
G92 E0
G1 E5 F200
M104 S220
G92 E0
M73 P5 R9
G1 E-0.5 F300

G1 X-28.5 F30000
M73 P8 R9
G1 X-48.2 F3000
M73 P10 R8
G1 X-28.5 F30000 ;wipe and shake
M73 P11 R8
G1 X-48.2 F3000
G1 X-28.5 F30000 ;wipe and shake
G1 X-48.2 F3000

;G392 S0

M400
M106 P1 S0
;===== prepare print temperature and material end =====

;M400
;M73 P1.717

;===== auto extrude cali start =========================
M975 S1
;G392 S1

G90
M83
T1000
G1 X-48.2 Y0 Z10 F10000
M400
M1002 set_filament_type:UNKNOWN

M412 S1 ;  ===turn on  filament runout detection===
M400 P10
M620.3 W1; === turn on filament tangle detection===
M400 S2

M1002 set_filament_type:PLA

;M1002 set_flag extrude_cali_flag=1
M1002 judge_flag extrude_cali_flag

M622 J1
    M1002 gcode_claim_action : 8

    M109 S220
    G1 E10 F377.08
    M983 F6.28466 A0.3 H0.4; cali dynamic extrusion compensation

    M106 P1 S255
    M400 S5
    G1 X-28.5 F18000
    G1 X-48.2 F3000
    G1 X-28.5 F18000 ;wipe and shake
    G1 X-48.2 F3000
M73 P14 R8
    G1 X-28.5 F12000 ;wipe and shake
    G1 X-48.2 F3000
    M400
    M106 P1 S0

    M1002 judge_last_extrude_cali_success
    M622 J0
        M983 F6.28466 A0.3 H0.4; cali dynamic extrusion compensation
        M106 P1 S255
        M400 S5
        G1 X-28.5 F18000
        G1 X-48.2 F3000
        G1 X-28.5 F18000 ;wipe and shake
        G1 X-48.2 F3000
        G1 X-28.5 F12000 ;wipe and shake
        M400
        M106 P1 S0
    M623
    
M73 P15 R8
    G1 X-48.2 F3000
    M400
    M984 A0.1 E1 S1 F6.28466 H0.4
    M106 P1 S178
    M400 S7
    G1 X-28.5 F18000
    G1 X-48.2 F3000
    G1 X-28.5 F18000 ;wipe and shake
    G1 X-48.2 F3000
    G1 X-28.5 F12000 ;wipe and shake
    G1 X-48.2 F3000
    M400
    M106 P1 S0
M623 ; end of "draw extrinsic para cali paint"

;G392 S0
;===== auto extrude cali end ========================

;M400
;M73 P1.717

M104 S170 ; prepare to wipe nozzle
M106 S255 ; turn on fan

;===== mech mode fast check start =====================
M1002 gcode_claim_action : 3

G1 X128 Y128 F20000
G1 Z5 F1200
M400 P200
M970.3 Q1 A5 K0 O3
M974 Q1 S2 P0

M970.2 Q1 K1 W58 Z0.1
M974 S2

G1 X128 Y128 F20000
G1 Z5 F1200
M400 P200
M970.3 Q0 A10 K0 O1
M974 Q0 S2 P0

M970.2 Q0 K1 W78 Z0.1
M974 S2

M975 S1
M73 P16 R8
G1 F30000
G1 X0 Y5
G28 X ; re-home XY

G1 Z4 F1200

;===== mech mode fast check end =======================

;M400
;M73 P1.717

;===== wipe nozzle ===============================
M1002 gcode_claim_action : 14

M975 S1
M106 S255 ; turn on fan (G28 has turn off fan)
M211 S; push soft endstop status
M211 X0 Y0 Z0 ;turn off Z axis endstop

;===== remove waste by touching start =====

M104 S170 ; set temp down to heatbed acceptable

M83
G1 E-1 F500
G90
M83

M109 S170
G0 X108 Y-0.5 F30000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X110 F10000
G380 S3 Z-5 F1200
M73 P60 R3
G1 Z2 F1200
G1 X112 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X114 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X116 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X118 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X120 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X122 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X124 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X126 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X128 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X130 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X132 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X134 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X136 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X138 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X140 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X142 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X144 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X146 F10000
G380 S3 Z-5 F1200
G1 Z2 F1200
G1 X148 F10000
G380 S3 Z-5 F1200

G1 Z5 F30000
;===== remove waste by touching end =====

G1 Z10 F1200
G0 X118 Y261 F30000
G1 Z5 F1200
M109 S170

G28 Z P0 T300; home z with low precision,permit 300deg temperature
G29.2 S0 ; turn off ABL
M104 S140 ; prepare to abl
G0 Z5 F20000

G0 X128 Y261 F20000  ; move to exposed steel surface
G0 Z-1.01 F1200      ; stop the nozzle

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
G1 Z10 F1200

;===== brush material wipe nozzle =====

G90
G1 Y250 F30000
G1 X55
G1 Z1.300 F1200
M73 P61 R3
G1 Y262.5 F6000
G91
G1 X-35 F30000
G1 Y-0.5
G1 X45
G1 Y-0.5
G1 X-45
G1 Y-0.5
G1 X45
G1 Y-0.5
G1 X-45
G1 Y-0.5
G1 X45
G1 Z5.000 F1200

G90
G1 X30 Y250.000 F30000
G1 Z1.300 F1200
G1 Y262.5 F6000
G91
G1 X35 F30000
G1 Y-0.5
G1 X-45
G1 Y-0.5
G1 X45
G1 Y-0.5
G1 X-45
G1 Y-0.5
G1 X45
G1 Y-0.5
G1 X-45
G1 Z10.000 F1200

;===== brush material wipe nozzle end =====

G90
;G0 X128 Y261 F20000  ; move to exposed steel surface
G1 Y250 F30000
G1 X138
G1 Y261
G0 Z-1.01 F1200      ; stop the nozzle

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
M73 P62 R3
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

M109 S140
M106 S255 ; turn on fan (G28 has turn off fan)

M211 R; pop softend status

;===== wipe nozzle end ================================

;M400
;M73 P1.717

;===== bed leveling ==================================
M1002 judge_flag g29_before_print_flag

G90
G1 Z5 F1200
G1 X0 Y0 F30000
G29.2 S1 ; turn on ABL

M190 S65; ensure bed temp
M109 S140
M106 S0 ; turn off fan , too noisy

M622 J1
    M1002 gcode_claim_action : 1
    G29 A1 X59.8904 Y110.991 I136.219 J34.0187
    M400
    M500 ; save cali data
M623
;===== bed leveling end ================================

;===== home after wipe mouth============================
M1002 judge_flag g29_before_print_flag
M622 J0

    M1002 gcode_claim_action : 13
    G28

M623

;===== home after wipe mouth end =======================

;M400
;M73 P1.717

G1 X108.000 Y-0.500 F30000
G1 Z0.300 F1200
M400
G2814 Z0.32

M104 S220 ; prepare to print

;===== nozzle load line ===============================
;G90
;M83
;G1 Z5 F1200
;G1 X88 Y-0.5 F20000
;G1 Z0.3 F1200

;M109 S220

;G1 E2 F300
;G1 X168 E4.989 F6000
;G1 Z1 F1200
;===== nozzle load line end ===========================

;===== extrude cali test ===============================

M400
    M900 S
    M900 C
    G90
    M83

    M109 S220
    G0 X128 E8  F904.991
    G0 X133 E.3742  F1508.32
    G0 X138 E.3742  F6033.27
    G0 X143 E.3742  F1508.32
    G0 X148 E.3742  F6033.27
    G0 X153 E.3742  F1508.32
    G91
    G1 X1 Z-0.300
    G1 X4
    G1 Z1 F1200
    G90
    M400

M900 R

M1002 judge_flag extrude_cali_flag
M622 J1
    G90
    G1 X108.000 Y1.000 F30000
    G91
    G1 Z-0.700 F1200
    G90
    M83
    G0 X128 E10  F904.991
    G0 X133 E.3742  F1508.32
    G0 X138 E.3742  F6033.27
    G0 X143 E.3742  F1508.32
    G0 X148 E.3742  F6033.27
    G0 X153 E.3742  F1508.32
    G91
    G1 X1 Z-0.300
    G1 X4
    G1 Z1 F1200
    G90
    M400
M623

G1 Z0.2

;M400
;M73 P1.717

;========turn off light and wait extrude temperature =============
M1002 gcode_claim_action : 0
M400

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
;G392 S1 ; turn on clog detection
M1007 S1 ; turn on mass estimation
G29.4
; MACHINE_START_GCODE_END
; filament start gcode
M106 P3 S200


;VT0 H-1
G90
G21
M83 ; use relative distances for extrusion
M981 S1 P20000 ;open spaghetti detector
; CHANGE_LAYER
; Z_HEIGHT: 0.2
; LAYER_HEIGHT: 0.2
G1 E-.8 F1800
; layer num/total_layer_count: 1/2
; update layer progress
M73 L1
M991 S0 P0 ;notify layer change
M106 S0
; CUSTOM_GCODE
;MKPCalibration ZOffset Calibration
; OBJECT_ID: 95
G1 X127.693 Y124.368 F42000
M204 S6000
G1 Z.4
G1 Z.2
G1 E.8 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.5
G1 F3000
M204 S500
G1 X127.878 Y124.351 E.00693
G1 X128.483 Y124.404 E.02263
M73 P63 R3
G1 X129.07 Y124.562 E.02263
G1 X129.621 Y124.818 E.02263
G1 X130.118 Y125.167 E.02263
G1 X130.548 Y125.596 E.02263
G1 X130.896 Y126.094 E.02263
G1 X131.074 Y126.476 E.0157
G1 X131.153 Y126.645 E.00693
G1 X131.31 Y127.231 E.02263
G1 X131.363 Y127.836 E.02263
G1 X131.31 Y128.442 E.02263
G1 X131.153 Y129.028 E.02263
G1 X130.975 Y129.41 E.01569
G1 X130.896 Y129.579 E.00693
G1 X130.548 Y130.077 E.02263
G1 X130.118 Y130.506 E.02263
G1 X129.621 Y130.855 E.02263
G1 X129.07 Y131.111 E.02262
G1 X128.483 Y131.269 E.02263
G1 X127.878 Y131.322 E.02263
G1 X127.273 Y131.269 E.02263
G1 X126.686 Y131.111 E.02263
G1 X126.136 Y130.855 E.02262
G1 X125.638 Y130.506 E.02263
G1 X125.208 Y130.077 E.02263
G1 X124.86 Y129.579 E.02263
G1 X124.603 Y129.028 E.02263
G1 X124.446 Y128.442 E.02263
G1 X124.393 Y127.836 E.02263
G1 X124.446 Y127.231 E.02263
G1 X124.603 Y126.644 E.02263
G1 X124.86 Y126.094 E.02262
G1 X125.208 Y125.596 E.02263
G1 X125.638 Y125.167 E.02263
G1 X126.136 Y124.818 E.02263
G1 X126.686 Y124.562 E.02263
G1 X127.273 Y124.404 E.02263
G1 X127.633 Y124.373 E.01346
M204 S6000
G1 X127.733 Y124.823 F42000
; FEATURE: Outer wall
G1 F3000
M204 S500
G3 X130.499 Y126.323 I.132 J3.057 E.12312
G1 X130.722 Y126.801 E.01965
G3 X127.353 Y124.856 I-2.84 J1.029 E.54961
G1 X127.673 Y124.828 E.01197
; WIPE_START
G1 X127.878 Y124.81 E-.07832
G1 X128.404 Y124.856 E-.20045
G1 X128.913 Y124.993 E-.20046
G1 X129.391 Y125.216 E-.20046
G1 X129.564 Y125.337 E-.08031
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X137.192 Y125.612 Z.6 F42000
G1 X178.703 Y127.109 Z.6
G1 Z.2
G1 E.8 F1800
; FEATURE: Inner wall
G1 F3000
M204 S500
G1 X176.139 Y127.109 E.0955
G1 X176.139 Y137.109 E.37246
G1 X167.703 Y137.109 E.3142
G1 X167.703 Y127.109 E.37246
G1 X165.139 Y127.109 E.0955
G1 X165.139 Y137.109 E.37246
G1 X156.703 Y137.109 E.3142
G1 X156.703 Y127.109 E.37246
G1 X154.139 Y127.109 E.0955
G1 X154.139 Y137.109 E.37246
G1 X145.703 Y137.109 E.3142
G1 X145.703 Y127.109 E.37246
G1 X143.139 Y127.109 E.0955
G1 X143.139 Y137.109 E.37246
G1 X134.703 Y137.109 E.3142
G1 X134.703 Y127.109 E.37246
G1 X132.139 Y127.109 E.0955
G1 X132.139 Y137.109 E.37246
G1 X123.703 Y137.109 E.3142
G1 X123.703 Y127.109 E.37246
G1 X121.139 Y127.109 E.0955
G1 X121.139 Y137.109 E.37246
G1 X112.703 Y137.109 E.3142
G1 X112.703 Y127.109 E.37246
G1 X110.139 Y127.109 E.0955
G1 X110.139 Y137.109 E.37246
G1 X101.703 Y137.109 E.3142
G1 X101.703 Y127.109 E.37246
G1 X99.139 Y127.109 E.0955
G1 X99.139 Y137.109 E.37246
M73 P64 R3
G1 X90.703 Y137.109 E.3142
G1 X90.703 Y127.109 E.37246
G1 X88.139 Y127.109 E.0955
G1 X88.139 Y137.109 E.37246
G1 X79.703 Y137.109 E.3142
G1 X79.703 Y127.109 E.37246
G1 X77.139 Y127.109 E.0955
G1 X77.139 Y137.109 E.37246
G1 X68.704 Y137.109 E.31418
G1 X68.771 Y122.982 E.52621
G1 X125.549 Y122.982 E2.11478
G1 X126.72 Y122.19 E.05264
G1 X129.194 Y122.19 E.09213
G1 X129.58 Y122.451 E.01739
G1 X130.364 Y122.982 E.03525
G1 X187.139 Y122.982 E2.11464
G1 X187.139 Y137.109 E.52621
G1 X178.703 Y137.109 E.3142
G1 X178.703 Y127.169 E.37023
M204 S6000
G1 X178.246 Y127.566 F42000
; FEATURE: Outer wall
G1 F3000
M204 S500
G1 X176.596 Y127.566 E.06146
G1 X176.596 Y137.223 E.35968
G3 X176.253 Y137.566 I-.335 J.008 E.02022
G1 X167.589 Y137.566 E.32268
G3 X167.246 Y137.223 I-.008 J-.335 E.02022
G1 X167.246 Y127.566 E.35968
M73 P65 R3
G1 X165.596 Y127.566 E.06146
G1 X165.596 Y137.223 E.35968
G3 X165.253 Y137.566 I-.335 J.008 E.02022
G1 X156.589 Y137.566 E.32268
G3 X156.246 Y137.223 I-.008 J-.335 E.02022
G1 X156.246 Y127.566 E.35968
G1 X154.596 Y127.566 E.06146
G1 X154.596 Y137.223 E.35968
G3 X154.253 Y137.566 I-.335 J.008 E.02022
G1 X145.589 Y137.566 E.32268
G3 X145.246 Y137.223 I-.008 J-.335 E.02022
G1 X145.246 Y127.566 E.35968
G1 X143.596 Y127.566 E.06146
G1 X143.596 Y137.223 E.35968
G3 X143.253 Y137.566 I-.335 J.008 E.02022
G1 X134.589 Y137.566 E.32268
G3 X134.246 Y137.223 I-.008 J-.335 E.02022
G1 X134.246 Y127.566 E.35968
G1 X132.596 Y127.566 E.06146
G1 X132.596 Y137.223 E.35968
G3 X132.253 Y137.566 I-.335 J.008 E.02022
G1 X123.589 Y137.566 E.32268
G3 X123.246 Y137.223 I-.008 J-.335 E.02022
M73 P66 R3
G1 X123.246 Y127.566 E.35968
G1 X121.596 Y127.566 E.06146
G1 X121.596 Y137.223 E.35968
G3 X121.253 Y137.566 I-.335 J.008 E.02022
G1 X112.589 Y137.566 E.32268
G3 X112.246 Y137.223 I-.008 J-.335 E.02022
G1 X112.246 Y127.566 E.35968
G1 X110.596 Y127.566 E.06146
G1 X110.596 Y137.223 E.35968
G3 X110.253 Y137.566 I-.335 J.008 E.02022
G1 X101.589 Y137.566 E.32268
G3 X101.246 Y137.223 I-.008 J-.335 E.02022
G1 X101.246 Y127.566 E.35968
G1 X99.596 Y127.566 E.06146
G1 X99.596 Y137.223 E.35968
G3 X99.253 Y137.566 I-.335 J.008 E.02022
G1 X90.589 Y137.566 E.32268
G3 X90.246 Y137.223 I-.008 J-.335 E.02022
G1 X90.246 Y127.566 E.35968
G1 X88.596 Y127.566 E.06146
G1 X88.596 Y137.223 E.35968
G3 X88.253 Y137.566 I-.335 J.008 E.02022
G1 X79.589 Y137.566 E.32268
G3 X79.246 Y137.223 I-.008 J-.335 E.02022
G1 X79.246 Y127.566 E.35968
G1 X77.596 Y127.566 E.06146
G1 X77.596 Y137.223 E.35968
G3 X77.253 Y137.566 I-.335 J.008 E.02022
G1 X68.589 Y137.566 E.32268
G3 X68.246 Y137.224 I-.008 J-.335 E.02019
G1 X68.315 Y122.768 E.53844
G3 X68.559 Y122.524 I.235 J-.008 E.01441
G1 X125.409 Y122.524 E2.11745
G1 X126.58 Y121.733 E.05264
G1 X129.334 Y121.733 E.10256
G1 X129.836 Y122.073 E.02261
G1 X130.504 Y122.524 E.03003
G1 X187.354 Y122.524 E2.11745
G3 X187.596 Y122.766 I.009 J.232 E.0143
G1 X187.596 Y137.223 E.53846
G3 X187.253 Y137.566 I-.335 J.008 E.02022
G1 X178.589 Y137.566 E.32268
G3 X178.246 Y137.223 I-.008 J-.335 E.02022
G1 X178.246 Y127.626 E.35744
; WIPE_START
M73 P67 R3
G1 X176.596 Y127.566 E-.62741
G1 X176.596 Y127.915 E-.13259
; WIPE_END
G1 E-.04 F1800
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
G1 X0 Y129.65 F18000 ; move to safe pos
G1 X-48.2 F3000 ; move to safe pos
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


G1 X186.956 Y124.089 F42000
G1 Z.2
G1 E.8 F1800
; FEATURE: Bottom surface
; LINE_WIDTH: 0.50019
G1 F6300
M204 S500
G1 X186.238 Y123.37 E.03787
G1 X185.591 Y123.37 E.0241
G1 X186.751 Y124.53 E.06111
G1 X186.751 Y125.176 E.0241
G1 X184.944 Y123.37 E.09519
G1 X184.298 Y123.37 E.0241
G1 X186.751 Y125.823 E.12927
G1 X186.751 Y126.47 E.0241
G1 X183.651 Y123.37 E.16334
G1 X183.004 Y123.37 E.0241
G1 X186.751 Y127.116 E.19742
G1 X186.751 Y127.763 E.0241
G1 X182.357 Y123.37 E.2315
G1 X181.711 Y123.37 E.0241
G1 X186.751 Y128.41 E.26557
G1 X186.751 Y129.056 E.0241
G1 X181.064 Y123.37 E.29965
G1 X180.417 Y123.37 E.0241
G1 X186.751 Y129.703 E.33373
M73 P68 R3
G1 X186.751 Y130.35 E.0241
G1 X179.771 Y123.37 E.36781
G1 X179.124 Y123.37 E.0241
G1 X186.751 Y130.996 E.40188
G1 X186.751 Y131.643 E.0241
G1 X178.477 Y123.37 E.43596
G1 X177.831 Y123.37 E.0241
G1 X186.751 Y132.29 E.47004
G1 X186.751 Y132.936 E.0241
G1 X177.184 Y123.37 E.50412
G1 X176.537 Y123.37 E.0241
G1 X186.751 Y133.583 E.53819
G1 X186.751 Y134.23 E.0241
G1 X175.891 Y123.37 E.57227
G1 X175.244 Y123.37 E.0241
G1 X178.595 Y126.721 E.17658
G1 X179.092 Y126.721 E.0185
G1 X179.092 Y127.217 E.0185
G1 X186.751 Y134.876 E.4036
G1 X186.751 Y135.523 E.0241
G1 X179.092 Y127.864 E.4036
G1 X179.092 Y128.511 E.0241
G1 X186.751 Y136.17 E.4036
G1 X186.751 Y136.721 E.02053
G1 X186.655 Y136.721 E.00356
G1 X179.092 Y129.157 E.39856
G1 X179.092 Y129.804 E.0241
G1 X186.008 Y136.721 E.36448
G1 X185.362 Y136.721 E.0241
G1 X179.092 Y130.451 E.33041
G1 X179.092 Y131.097 E.0241
G1 X184.715 Y136.721 E.29633
G1 X184.068 Y136.721 E.0241
G1 X179.092 Y131.744 E.26225
G1 X179.092 Y132.391 E.0241
G1 X183.422 Y136.721 E.22818
G1 X182.775 Y136.721 E.0241
G1 X179.092 Y133.038 E.1941
G1 X179.092 Y133.684 E.0241
M73 P69 R3
G1 X182.128 Y136.721 E.16002
G1 X181.482 Y136.721 E.0241
G1 X179.092 Y134.331 E.12594
G1 X179.092 Y134.978 E.0241
G1 X180.835 Y136.721 E.09187
G1 X180.188 Y136.721 E.0241
G1 X179.092 Y135.624 E.05779
G1 X179.092 Y136.271 E.0241
G1 X179.747 Y136.927 E.03455
; WIPE_START
G1 X179.092 Y136.271 E-.35235
G1 X179.092 Y135.624 E-.24574
G1 X179.393 Y135.926 E-.16191
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X176.608 Y128.819 Z.6 F42000
G1 X174.392 Y123.164 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X177.948 Y126.721 E.18742
G1 X177.302 Y126.721 E.0241
G1 X173.951 Y123.37 E.17658
G1 X173.304 Y123.37 E.0241
G1 X176.655 Y126.721 E.17658
G1 X176.008 Y126.721 E.0241
G1 X172.657 Y123.37 E.17658
G1 X172.011 Y123.37 E.0241
G1 X175.751 Y127.11 E.19708
G1 X175.751 Y127.757 E.0241
G1 X171.364 Y123.37 E.23116
G1 X170.717 Y123.37 E.0241
M73 P69 R2
G1 X175.751 Y128.403 E.26523
G1 X175.751 Y129.05 E.0241
G1 X170.071 Y123.37 E.29931
G1 X169.424 Y123.37 E.0241
G1 X175.751 Y129.697 E.33339
G1 X175.751 Y130.343 E.0241
G1 X168.777 Y123.37 E.36747
G1 X168.131 Y123.37 E.0241
G1 X175.751 Y130.99 E.40154
G1 X175.751 Y131.637 E.0241
G1 X167.484 Y123.37 E.43562
G1 X166.837 Y123.37 E.0241
G1 X175.751 Y132.283 E.4697
G1 X175.751 Y132.93 E.0241
G1 X166.191 Y123.37 E.50377
G1 X165.544 Y123.37 E.0241
G1 X175.751 Y133.577 E.53785
G1 X175.751 Y134.223 E.0241
G1 X164.897 Y123.37 E.57193
G1 X164.251 Y123.37 E.0241
G1 X167.601 Y126.721 E.17658
G1 X168.092 Y126.721 E.01826
G1 X168.092 Y127.211 E.01826
G1 X175.751 Y134.87 E.4036
G1 X175.751 Y135.517 E.0241
G1 X168.092 Y127.858 E.4036
G1 X168.092 Y128.504 E.0241
G1 X175.751 Y136.163 E.4036
G1 X175.751 Y136.721 E.02077
M73 P70 R2
G1 X175.661 Y136.721 E.00332
G1 X168.092 Y129.151 E.39891
G1 X168.092 Y129.798 E.0241
G1 X175.015 Y136.721 E.36483
G1 X174.368 Y136.721 E.0241
G1 X168.092 Y130.444 E.33075
G1 X168.092 Y131.091 E.0241
G1 X173.721 Y136.721 E.29667
G1 X173.075 Y136.721 E.0241
G1 X168.092 Y131.738 E.2626
G1 X168.092 Y132.384 E.0241
G1 X172.428 Y136.721 E.22852
G1 X171.781 Y136.721 E.0241
G1 X168.092 Y133.031 E.19444
G1 X168.092 Y133.678 E.0241
G1 X171.135 Y136.721 E.16036
G1 X170.488 Y136.721 E.0241
G1 X168.092 Y134.324 E.12628
G1 X168.092 Y134.971 E.0241
G1 X169.841 Y136.721 E.09221
G1 X169.195 Y136.721 E.0241
G1 X168.092 Y135.618 E.05813
G1 X168.092 Y136.264 E.0241
G1 X168.754 Y136.927 E.03489
; WIPE_START
G1 X168.092 Y136.264 E-.35583
G1 X168.092 Y135.618 E-.24574
G1 X168.386 Y135.913 E-.15843
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X167.355 Y128.35 Z.6 F42000
G1 X167.16 Y126.927 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X163.604 Y123.37 E.18742
G1 X162.957 Y123.37 E.0241
G1 X166.308 Y126.721 E.17658
G1 X165.661 Y126.721 E.0241
G1 X162.31 Y123.37 E.17658
G1 X161.664 Y123.37 E.0241
G1 X165.015 Y126.721 E.17658
G1 X164.751 Y126.721 E.00984
G1 X164.751 Y127.103 E.01425
G1 X161.017 Y123.37 E.19674
M73 P71 R2
G1 X160.37 Y123.37 E.0241
G1 X164.751 Y127.75 E.23082
G1 X164.751 Y128.397 E.0241
G1 X159.724 Y123.37 E.26489
G1 X159.077 Y123.37 E.0241
G1 X164.751 Y129.043 E.29897
G1 X164.751 Y129.69 E.0241
G1 X158.43 Y123.37 E.33305
G1 X157.784 Y123.37 E.0241
G1 X164.751 Y130.337 E.36712
G1 X164.751 Y130.983 E.0241
G1 X157.137 Y123.37 E.4012
G1 X156.49 Y123.37 E.0241
G1 X164.751 Y131.63 E.43528
G1 X164.751 Y132.277 E.0241
G1 X155.844 Y123.37 E.46936
G1 X155.197 Y123.37 E.0241
G1 X164.751 Y132.923 E.50343
G1 X164.751 Y133.57 E.0241
G1 X154.55 Y123.37 E.53751
G1 X153.904 Y123.37 E.0241
G1 X164.751 Y134.217 E.57159
G1 X164.751 Y134.864 E.0241
G1 X157.092 Y127.204 E.4036
G1 X157.092 Y126.721 E.01802
G1 X156.608 Y126.721 E.01802
G1 X153.257 Y123.37 E.17658
G1 X152.61 Y123.37 E.0241
G1 X155.961 Y126.721 E.17658
G1 X155.315 Y126.721 E.0241
G1 X151.964 Y123.37 E.17658
G1 X151.317 Y123.37 E.0241
G1 X154.668 Y126.721 E.17658
G1 X154.021 Y126.721 E.0241
G1 X150.67 Y123.37 E.17658
G1 X150.024 Y123.37 E.0241
G1 X153.751 Y127.097 E.1964
G1 X153.751 Y127.744 E.0241
G1 X149.377 Y123.37 E.23047
G1 X148.73 Y123.37 E.0241
G1 X153.751 Y128.39 E.26455
G1 X153.751 Y129.037 E.0241
G1 X148.084 Y123.37 E.29863
G1 X147.437 Y123.37 E.0241
G1 X153.751 Y129.684 E.33271
G1 X153.751 Y130.33 E.0241
G1 X146.79 Y123.37 E.36678
G1 X146.144 Y123.37 E.0241
G1 X153.751 Y130.977 E.40086
G1 X153.751 Y131.624 E.0241
G1 X145.497 Y123.37 E.43494
G1 X144.85 Y123.37 E.0241
G1 X153.751 Y132.27 E.46901
G1 X153.751 Y132.917 E.0241
G1 X144.204 Y123.37 E.50309
G1 X143.557 Y123.37 E.0241
M73 P72 R2
G1 X153.751 Y133.564 E.53717
G1 X153.751 Y134.21 E.0241
G1 X142.91 Y123.37 E.57125
G1 X142.263 Y123.37 E.0241
G1 X145.614 Y126.721 E.17658
G1 X146.092 Y126.721 E.01778
G1 X146.092 Y127.198 E.01778
G1 X153.751 Y134.857 E.4036
G1 X153.751 Y135.504 E.0241
G1 X146.092 Y127.845 E.4036
G1 X146.092 Y128.491 E.0241
G1 X153.751 Y136.15 E.4036
G1 X153.751 Y136.721 E.02126
G1 X153.674 Y136.721 E.00284
G1 X146.092 Y129.138 E.39959
G1 X146.092 Y129.785 E.0241
G1 X153.028 Y136.721 E.36551
G1 X152.381 Y136.721 E.0241
G1 X146.092 Y130.431 E.33143
G1 X146.092 Y131.078 E.0241
G1 X151.734 Y136.721 E.29735
G1 X151.088 Y136.721 E.0241
G1 X146.092 Y131.725 E.26328
G1 X146.092 Y132.371 E.0241
G1 X150.441 Y136.721 E.2292
G1 X149.794 Y136.721 E.0241
G1 X146.092 Y133.018 E.19512
G1 X146.092 Y133.665 E.0241
G1 X149.148 Y136.721 E.16105
G1 X148.501 Y136.721 E.0241
G1 X146.092 Y134.311 E.12697
G1 X146.092 Y134.958 E.0241
G1 X147.854 Y136.721 E.09289
G1 X147.208 Y136.721 E.0241
G1 X146.092 Y135.605 E.05881
G1 X146.092 Y136.251 E.0241
G1 X146.767 Y136.927 E.03557
; WIPE_START
G1 X146.092 Y136.251 E-.36279
G1 X146.092 Y135.605 E-.24574
G1 X146.373 Y135.887 E-.15147
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X145.36 Y128.322 Z.6 F42000
G1 X145.173 Y126.927 Z.6
G1 Z.2
M73 P73 R2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X141.617 Y123.37 E.18742
G1 X140.97 Y123.37 E.0241
G1 X144.321 Y126.721 E.17658
G1 X143.674 Y126.721 E.0241
G1 X140.323 Y123.37 E.17658
G1 X139.677 Y123.37 E.0241
G1 X143.028 Y126.721 E.17658
G1 X142.751 Y126.721 E.01033
G1 X142.751 Y127.09 E.01377
G1 X139.03 Y123.37 E.19606
G1 X138.383 Y123.37 E.0241
G1 X142.751 Y127.737 E.23013
G1 X142.751 Y128.384 E.0241
G1 X137.737 Y123.37 E.26421
G1 X137.09 Y123.37 E.0241
G1 X142.751 Y129.03 E.29829
G1 X142.751 Y129.677 E.0241
G1 X136.443 Y123.37 E.33236
G1 X135.797 Y123.37 E.0241
G1 X142.751 Y130.324 E.36644
G1 X142.751 Y130.971 E.0241
G1 X135.15 Y123.37 E.40052
G1 X134.503 Y123.37 E.0241
G1 X142.751 Y131.617 E.4346
G1 X142.751 Y132.264 E.0241
G1 X133.857 Y123.37 E.46867
G1 X133.21 Y123.37 E.0241
G1 X142.751 Y132.911 E.50275
G1 X142.751 Y133.557 E.0241
G1 X132.563 Y123.37 E.53683
G1 X131.917 Y123.37 E.0241
G1 X142.751 Y134.204 E.57091
G1 X142.751 Y134.851 E.0241
G1 X135.092 Y127.192 E.4036
G1 X135.092 Y126.721 E.01754
G1 X134.621 Y126.721 E.01754
G1 X131.27 Y123.37 E.17658
G1 X130.623 Y123.37 E.0241
G1 X133.974 Y126.721 E.17658
G1 X133.328 Y126.721 E.0241
G1 X129.415 Y122.809 E.20616
G1 X129.075 Y122.578 E.01533
G1 X128.538 Y122.578 E.01999
G1 X132.681 Y126.721 E.2183
G1 X132.274 Y126.721 E.01515
G1 X132.234 Y126.417 E.01142
G1 X131.789 Y126.476 E.01674
G1 X127.891 Y122.578 E.20537
G1 X127.245 Y122.578 E.0241
G1 X128.737 Y124.07 E.07862
G1 X128.551 Y124.02 E.00717
G1 X127.991 Y123.971 E.02093
G1 X126.695 Y122.675 E.06829
G1 X126.31 Y122.936 E.01735
G1 X127.378 Y124.005 E.05633
M73 P74 R2
G2 X126.844 Y124.117 I.05 J1.572 E.02046
G1 X125.924 Y123.197 E.04848
G1 X125.668 Y123.37 E.01149
G1 X125.45 Y123.37 E.00814
G1 X126.363 Y124.283 E.04814
G1 X125.925 Y124.492 E.01809
G1 X124.803 Y123.37 E.05911
G1 X124.157 Y123.37 E.0241
G1 X125.545 Y124.758 E.07314
G2 X125.198 Y125.058 I.677 J1.134 E.01717
G1 X123.51 Y123.37 E.08894
G1 X122.863 Y123.37 E.0241
G1 X124.881 Y125.387 E.10631
G1 X124.614 Y125.768 E.0173
G1 X122.216 Y123.37 E.12635
G1 X121.57 Y123.37 E.0241
G1 X124.388 Y126.188 E.14849
G1 X124.239 Y126.506 E.01307
G1 X124.031 Y126.478 E.00782
G1 X120.923 Y123.37 E.16379
G1 X120.276 Y123.37 E.0241
G1 X123.574 Y126.668 E.17378
G1 X123.567 Y126.721 E.002
G1 X122.981 Y126.721 E.02185
G1 X119.63 Y123.37 E.17658
G1 X118.983 Y123.37 E.0241
G1 X122.334 Y126.721 E.17658
G1 X121.687 Y126.721 E.0241
G1 X118.336 Y123.37 E.17658
G1 X117.69 Y123.37 E.0241
G1 X121.041 Y126.721 E.17658
G1 X120.751 Y126.721 E.01081
G1 X120.751 Y127.077 E.01329
G1 X117.043 Y123.37 E.19537
G1 X116.396 Y123.37 E.0241
G1 X120.751 Y127.724 E.22945
G1 X120.751 Y128.371 E.0241
G1 X115.75 Y123.37 E.26353
G1 X115.103 Y123.37 E.0241
G1 X120.751 Y129.017 E.2976
G1 X120.751 Y129.664 E.0241
G1 X114.456 Y123.37 E.33168
G1 X113.81 Y123.37 E.0241
G1 X120.751 Y130.311 E.36576
G1 X120.751 Y130.958 E.0241
G1 X113.163 Y123.37 E.39984
G1 X112.516 Y123.37 E.0241
G1 X120.751 Y131.604 E.43391
G1 X120.751 Y132.251 E.0241
G1 X111.87 Y123.37 E.46799
G1 X111.223 Y123.37 E.0241
G1 X120.751 Y132.898 E.50207
G1 X120.751 Y133.544 E.0241
G1 X110.576 Y123.37 E.53614
G1 X109.93 Y123.37 E.0241
G1 X120.751 Y134.191 E.57022
G1 X120.751 Y134.838 E.0241
G1 X113.092 Y127.179 E.4036
G1 X113.092 Y126.721 E.01705
G1 X112.634 Y126.721 E.01705
G1 X109.283 Y123.37 E.17658
G1 X108.636 Y123.37 E.0241
G1 X111.987 Y126.721 E.17658
G1 X111.341 Y126.721 E.0241
G1 X107.99 Y123.37 E.17658
M73 P75 R2
G1 X107.343 Y123.37 E.0241
G1 X110.694 Y126.721 E.17658
G1 X110.047 Y126.721 E.0241
G1 X106.696 Y123.37 E.17658
G1 X106.05 Y123.37 E.0241
G1 X109.751 Y127.071 E.19503
G1 X109.751 Y127.718 E.0241
G1 X105.403 Y123.37 E.22911
G1 X104.756 Y123.37 E.0241
G1 X109.751 Y128.364 E.26318
G1 X109.751 Y129.011 E.0241
G1 X104.11 Y123.37 E.29726
G1 X103.463 Y123.37 E.0241
G1 X109.751 Y129.658 E.33134
G1 X109.751 Y130.304 E.0241
G1 X102.816 Y123.37 E.36542
G1 X102.169 Y123.37 E.0241
G1 X109.751 Y130.951 E.39949
G1 X109.751 Y131.598 E.0241
G1 X101.523 Y123.37 E.43357
G1 X100.876 Y123.37 E.0241
G1 X109.751 Y132.244 E.46765
G1 X109.751 Y132.891 E.0241
G1 X100.229 Y123.37 E.50172
G1 X99.583 Y123.37 E.0241
G1 X109.751 Y133.538 E.5358
G1 X109.751 Y134.184 E.0241
G1 X98.936 Y123.37 E.56988
G1 X98.289 Y123.37 E.0241
G1 X101.64 Y126.721 E.17658
G1 X102.092 Y126.721 E.01681
G1 X102.092 Y127.172 E.01681
G1 X109.751 Y134.831 E.4036
G1 X109.751 Y135.478 E.0241
G1 X102.092 Y127.819 E.4036
G1 X102.092 Y128.465 E.0241
G1 X109.751 Y136.124 E.4036
G1 X109.751 Y136.721 E.02222
G1 X109.7 Y136.721 E.00187
G1 X102.092 Y129.112 E.40095
G1 X102.092 Y129.759 E.0241
G1 X109.054 Y136.721 E.36688
G1 X108.407 Y136.721 E.0241
G1 X102.092 Y130.405 E.3328
G1 X102.092 Y131.052 E.0241
M73 P76 R2
G1 X107.76 Y136.721 E.29872
G1 X107.114 Y136.721 E.0241
G1 X102.092 Y131.699 E.26464
G1 X102.092 Y132.345 E.0241
G1 X106.467 Y136.721 E.23057
G1 X105.82 Y136.721 E.0241
G1 X102.092 Y132.992 E.19649
G1 X102.092 Y133.639 E.0241
G1 X105.174 Y136.721 E.16241
G1 X104.527 Y136.721 E.0241
G1 X102.092 Y134.286 E.12833
G1 X102.092 Y134.932 E.0241
G1 X103.88 Y136.721 E.09426
G1 X103.234 Y136.721 E.0241
G1 X102.092 Y135.579 E.06018
G1 X102.092 Y136.226 E.0241
G1 X102.793 Y136.927 E.03694
; WIPE_START
G1 X102.092 Y136.226 E-.37673
G1 X102.092 Y135.579 E-.24574
G1 X102.347 Y135.835 E-.13753
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X101.372 Y128.265 Z.6 F42000
G1 X101.199 Y126.927 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X97.643 Y123.37 E.18742
G1 X96.996 Y123.37 E.0241
G1 X100.347 Y126.721 E.17658
G1 X99.7 Y126.721 E.0241
G1 X96.349 Y123.37 E.17658
G1 X95.703 Y123.37 E.0241
G1 X99.054 Y126.721 E.17658
G1 X98.751 Y126.721 E.01129
G1 X98.751 Y127.064 E.0128
G1 X95.056 Y123.37 E.19469
G1 X94.409 Y123.37 E.0241
G1 X98.751 Y127.711 E.22876
G1 X98.751 Y128.358 E.0241
G1 X93.763 Y123.37 E.26284
G1 X93.116 Y123.37 E.0241
G1 X98.751 Y129.005 E.29692
G1 X98.751 Y129.651 E.0241
G1 X92.469 Y123.37 E.331
G1 X91.823 Y123.37 E.0241
G1 X98.751 Y130.298 E.36507
G1 X98.751 Y130.945 E.0241
G1 X91.176 Y123.37 E.39915
G1 X90.529 Y123.37 E.0241
G1 X98.751 Y131.591 E.43323
G1 X98.751 Y132.238 E.0241
G1 X89.883 Y123.37 E.46731
G1 X89.236 Y123.37 E.0241
G1 X98.751 Y132.885 E.50138
M73 P77 R2
G1 X98.751 Y133.531 E.0241
G1 X88.589 Y123.37 E.53546
G1 X87.943 Y123.37 E.0241
G1 X98.751 Y134.178 E.56954
G1 X98.751 Y134.825 E.0241
G1 X91.092 Y127.166 E.4036
G1 X91.092 Y126.721 E.01657
G1 X90.647 Y126.721 E.01657
G1 X87.296 Y123.37 E.17658
G1 X86.649 Y123.37 E.0241
G1 X90 Y126.721 E.17658
G1 X89.353 Y126.721 E.0241
G1 X86.003 Y123.37 E.17658
G1 X85.356 Y123.37 E.0241
G1 X88.707 Y126.721 E.17658
G1 X88.06 Y126.721 E.0241
G1 X84.709 Y123.37 E.17658
G1 X84.063 Y123.37 E.0241
G1 X87.751 Y127.058 E.19435
G1 X87.751 Y127.705 E.0241
G1 X83.416 Y123.37 E.22842
G1 X82.769 Y123.37 E.0241
G1 X87.751 Y128.351 E.2625
G1 X87.751 Y128.998 E.0241
G1 X82.122 Y123.37 E.29658
G1 X81.476 Y123.37 E.0241
G1 X87.751 Y129.645 E.33065
G1 X87.751 Y130.291 E.0241
G1 X80.829 Y123.37 E.36473
G1 X80.182 Y123.37 E.0241
G1 X87.751 Y130.938 E.39881
G1 X87.751 Y131.585 E.0241
G1 X79.536 Y123.37 E.43289
G1 X78.889 Y123.37 E.0241
G1 X87.751 Y132.231 E.46696
G1 X87.751 Y132.878 E.0241
G1 X78.242 Y123.37 E.50104
G1 X77.596 Y123.37 E.0241
G1 X87.751 Y133.525 E.53512
G1 X87.751 Y134.171 E.0241
G1 X76.949 Y123.37 E.5692
G1 X76.302 Y123.37 E.0241
G1 X79.653 Y126.721 E.17658
G1 X80.092 Y126.721 E.01633
G1 X80.092 Y127.159 E.01633
G1 X87.751 Y134.818 E.4036
G1 X87.751 Y135.465 E.0241
M73 P78 R2
G1 X80.092 Y127.806 E.4036
G1 X80.092 Y128.452 E.0241
G1 X87.751 Y136.111 E.4036
G1 X87.751 Y136.721 E.02271
G1 X87.713 Y136.721 E.00139
G1 X80.092 Y129.099 E.40164
G1 X80.092 Y129.746 E.0241
G1 X87.067 Y136.721 E.36756
G1 X86.42 Y136.721 E.0241
G1 X80.092 Y130.392 E.33348
G1 X80.092 Y131.039 E.0241
G1 X85.773 Y136.721 E.29941
G1 X85.127 Y136.721 E.0241
G1 X80.092 Y131.686 E.26533
G1 X80.092 Y132.333 E.0241
G1 X84.48 Y136.721 E.23125
G1 X83.833 Y136.721 E.0241
G1 X80.092 Y132.979 E.19717
G1 X80.092 Y133.626 E.0241
G1 X83.187 Y136.721 E.1631
G1 X82.54 Y136.721 E.0241
G1 X80.092 Y134.273 E.12902
G1 X80.092 Y134.919 E.0241
G1 X81.893 Y136.721 E.09494
G1 X81.247 Y136.721 E.0241
G1 X80.092 Y135.566 E.06086
G1 X80.092 Y136.213 E.0241
G1 X80.806 Y136.927 E.03763
; WIPE_START
G1 X80.092 Y136.213 E-.38371
G1 X80.092 Y135.566 E-.24574
G1 X80.334 Y135.809 E-.13055
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X79.378 Y128.237 Z.6 F42000
G1 X79.212 Y126.927 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X75.656 Y123.37 E.18742
G1 X75.009 Y123.37 E.0241
G1 X78.36 Y126.721 E.17658
G1 X77.713 Y126.721 E.0241
G1 X74.362 Y123.37 E.17658
G1 X73.716 Y123.37 E.0241
M73 P79 R2
G1 X77.067 Y126.721 E.17658
G1 X76.751 Y126.721 E.01178
G1 X76.751 Y127.052 E.01232
G1 X73.069 Y123.37 E.194
G1 X72.422 Y123.37 E.0241
G1 X76.751 Y127.698 E.22808
G1 X76.751 Y128.345 E.0241
G1 X71.776 Y123.37 E.26216
G1 X71.129 Y123.37 E.0241
G1 X76.751 Y128.992 E.29624
G1 X76.751 Y129.638 E.0241
G1 X70.482 Y123.37 E.33031
G1 X69.836 Y123.37 E.0241
G1 X76.751 Y130.285 E.36439
G1 X76.751 Y130.932 E.0241
G1 X69.189 Y123.37 E.39847
G1 X69.157 Y123.37 E.00117
G1 X69.155 Y123.982 E.02282
G1 X76.751 Y131.578 E.40028
G1 X76.751 Y132.225 E.0241
G1 X69.152 Y124.626 E.40044
G1 X69.148 Y125.27 E.02398
G1 X76.751 Y132.872 E.4006
G1 X76.751 Y133.518 E.0241
G1 X69.145 Y125.913 E.40076
G1 X69.142 Y126.557 E.02398
G1 X76.751 Y134.165 E.40092
G1 X76.751 Y134.812 E.0241
G1 X69.139 Y127.2 E.40108
G1 X69.136 Y127.844 E.02398
G1 X76.751 Y135.458 E.40125
G1 X76.751 Y136.105 E.0241
G1 X69.133 Y128.488 E.40141
G1 X69.13 Y129.131 E.02398
G1 X76.72 Y136.721 E.39995
G1 X76.073 Y136.721 E.0241
G1 X69.127 Y129.775 E.36603
G1 X69.124 Y130.418 E.02398
G1 X75.426 Y136.721 E.33211
G1 X74.78 Y136.721 E.0241
G1 X69.121 Y131.062 E.2982
G1 X69.118 Y131.706 E.02398
G1 X74.133 Y136.721 E.26428
G1 X73.486 Y136.721 E.0241
G1 X69.115 Y132.349 E.23037
G1 X69.112 Y132.993 E.02398
G1 X72.84 Y136.721 E.19645
G1 X72.193 Y136.721 E.0241
M73 P79 R1
G1 X69.109 Y133.637 E.16253
G1 X69.106 Y134.28 E.02398
G1 X71.546 Y136.721 E.12862
G1 X70.9 Y136.721 E.0241
G1 X69.103 Y134.924 E.0947
G1 X69.1 Y135.567 E.02398
G1 X70.253 Y136.721 E.06079
G1 X69.606 Y136.721 E.0241
M73 P80 R1
G1 X68.892 Y136.006 E.03766
; WIPE_START
G1 X69.606 Y136.721 E-.38403
G1 X70.253 Y136.721 E-.24574
G1 X70.011 Y136.479 E-.13024
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X77.035 Y133.493 Z.6 F42000
G1 X90.886 Y127.607 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X98.751 Y135.471 E.41444
G1 X98.751 Y136.118 E.0241
G1 X91.092 Y128.459 E.4036
G1 X91.092 Y129.106 E.0241
G1 X98.707 Y136.721 E.4013
G1 X98.06 Y136.721 E.0241
G1 X91.092 Y129.752 E.36722
G1 X91.092 Y130.399 E.0241
G1 X97.413 Y136.721 E.33314
G1 X96.767 Y136.721 E.0241
G1 X91.092 Y131.046 E.29906
G1 X91.092 Y131.692 E.0241
G1 X96.12 Y136.721 E.26499
G1 X95.473 Y136.721 E.0241
G1 X91.092 Y132.339 E.23091
G1 X91.092 Y132.986 E.0241
G1 X94.827 Y136.721 E.19683
G1 X94.18 Y136.721 E.0241
G1 X91.092 Y133.632 E.16275
G1 X91.092 Y134.279 E.0241
G1 X93.533 Y136.721 E.12868
G1 X92.887 Y136.721 E.0241
G1 X91.092 Y134.926 E.0946
G1 X91.092 Y135.572 E.0241
G1 X92.24 Y136.721 E.06052
G1 X91.593 Y136.721 E.0241
G1 X90.886 Y136.013 E.03728
; WIPE_START
G1 X91.593 Y136.721 E-.38022
G1 X92.24 Y136.721 E-.24574
G1 X91.991 Y136.471 E-.13404
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X99.018 Y133.494 Z.6 F42000
G1 X112.886 Y127.62 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X120.751 Y135.484 E.41444
G1 X120.751 Y136.131 E.0241
G1 X113.092 Y128.472 E.4036
G1 X113.092 Y129.119 E.0241
G1 X120.694 Y136.721 E.40061
G1 X120.047 Y136.721 E.0241
G1 X113.092 Y129.765 E.36653
G1 X113.092 Y130.412 E.0241
M73 P81 R1
G1 X119.4 Y136.721 E.33246
G1 X118.754 Y136.721 E.0241
G1 X113.092 Y131.059 E.29838
G1 X113.092 Y131.705 E.0241
G1 X118.107 Y136.721 E.2643
G1 X117.46 Y136.721 E.0241
G1 X113.092 Y132.352 E.23022
G1 X113.092 Y132.999 E.0241
G1 X116.814 Y136.721 E.19615
G1 X116.167 Y136.721 E.0241
G1 X113.092 Y133.645 E.16207
G1 X113.092 Y134.292 E.0241
G1 X115.52 Y136.721 E.12799
G1 X114.874 Y136.721 E.0241
G1 X113.092 Y134.939 E.09392
G1 X113.092 Y135.585 E.0241
G1 X114.227 Y136.721 E.05984
G1 X113.58 Y136.721 E.0241
G1 X112.886 Y136.026 E.0366
; WIPE_START
G1 X113.58 Y136.721 E-.37325
G1 X114.227 Y136.721 E-.24574
G1 X113.965 Y136.458 E-.14101
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X120.997 Y133.492 Z.6 F42000
G1 X134.886 Y127.633 Z.6
G1 Z.2
G1 E.8 F1800
G1 F6300
M204 S500
G1 X142.751 Y135.497 E.41444
G1 X142.751 Y136.144 E.0241
G1 X135.092 Y128.485 E.4036
G1 X135.092 Y129.132 E.0241
G1 X142.681 Y136.721 E.39993
G1 X142.034 Y136.721 E.0241
G1 X135.092 Y129.778 E.36585
G1 X135.092 Y130.425 E.0241
G1 X141.388 Y136.721 E.33177
G1 X140.741 Y136.721 E.0241
G1 X135.092 Y131.072 E.2977
G1 X135.092 Y131.718 E.0241
G1 X140.094 Y136.721 E.26362
G1 X139.447 Y136.721 E.0241
G1 X135.092 Y132.365 E.22954
G1 X135.092 Y133.012 E.0241
G1 X138.801 Y136.721 E.19546
G1 X138.154 Y136.721 E.0241
G1 X135.092 Y133.658 E.16139
G1 X135.092 Y134.305 E.0241
G1 X137.507 Y136.721 E.12731
G1 X136.861 Y136.721 E.0241
G1 X135.092 Y134.952 E.09323
G1 X135.092 Y135.598 E.0241
G1 X136.214 Y136.721 E.05915
G1 X135.567 Y136.721 E.0241
G1 X134.886 Y136.039 E.03592
; WIPE_START
G1 X135.567 Y136.721 E-.36628
G1 X136.214 Y136.721 E-.24574
G1 X135.939 Y136.446 E-.14799
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X142.975 Y133.489 Z.6 F42000
G1 X156.886 Y127.645 Z.6
G1 Z.2
M73 P82 R1
G1 E.8 F1800
G1 F6300
M204 S500
G1 X164.751 Y135.51 E.41444
G1 X164.751 Y136.157 E.0241
G1 X157.092 Y128.498 E.4036
G1 X157.092 Y129.145 E.0241
G1 X164.668 Y136.721 E.39925
G1 X164.021 Y136.721 E.0241
G1 X157.092 Y129.791 E.36517
G1 X157.092 Y130.438 E.0241
G1 X163.375 Y136.721 E.33109
G1 X162.728 Y136.721 E.0241
G1 X157.092 Y131.085 E.29701
G1 X157.092 Y131.731 E.0241
G1 X162.081 Y136.721 E.26294
G1 X161.435 Y136.721 E.0241
G1 X157.092 Y132.378 E.22886
G1 X157.092 Y133.025 E.0241
G1 X160.788 Y136.721 E.19478
G1 X160.141 Y136.721 E.0241
G1 X157.092 Y133.671 E.1607
G1 X157.092 Y134.318 E.0241
G1 X159.494 Y136.721 E.12663
G1 X158.848 Y136.721 E.0241
G1 X157.092 Y134.965 E.09255
G1 X157.092 Y135.611 E.0241
G1 X158.201 Y136.721 E.05847
G1 X157.554 Y136.721 E.0241
G1 X156.886 Y136.052 E.03523
; WIPE_START
G1 X157.554 Y136.721 E-.35931
G1 X158.201 Y136.721 E-.24574
G1 X157.913 Y136.433 E-.15496
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X150.741 Y133.821 Z.6 F42000
G1 X131.686 Y126.881 Z.6
G1 Z.2
G1 E.8 F1800
; FEATURE: Gap infill
; LINE_WIDTH: 0.431998
G1 F3000
M204 S500
G1 X131.72 Y127.142 E.00836
; LINE_WIDTH: 0.405425
G3 X131.737 Y127.499 I-4.052 J.372 E.01055
; LINE_WIDTH: 0.375584
G1 X131.751 Y127.823 E.00878
; LINE_WIDTH: 0.374477
G3 X131.737 Y128.174 I-4.034 J.014 E.00951
; LINE_WIDTH: 0.403855
G1 X131.723 Y128.498 E.00953
; LINE_WIDTH: 0.441028
G3 X131.689 Y128.766 I-3.105 J-.256 E.00878
; LINE_WIDTH: 0.48398
G1 X131.658 Y129.002 E.00854
; LINE_WIDTH: 0.524127
G1 X131.627 Y129.238 E.00932
M204 S6000
G1 X131.11 Y129.553 F42000
; FEATURE: Bottom surface
; LINE_WIDTH: 0.50313
G1 F6300
M204 S500
G1 X131.751 Y130.193 E.03397
G1 X131.751 Y130.844 E.02441
G1 X131.006 Y130.099 E.03949
G3 X130.715 Y130.459 I-1.165 J-.646 E.01744
G1 X131.751 Y131.495 E.05493
G1 X131.751 Y132.146 E.02441
G1 X130.389 Y130.784 E.07219
G1 X130.01 Y131.056 E.01749
G1 X131.751 Y132.796 E.0923
G1 X131.751 Y133.447 E.02441
G1 X129.597 Y131.294 E.1142
G1 X129.146 Y131.493 E.01851
M73 P83 R1
G1 X131.751 Y134.098 E.13815
G1 X131.751 Y134.749 E.02441
G1 X128.632 Y131.631 E.16537
G1 X128.551 Y131.653 E.00316
G1 X128.047 Y131.697 E.01895
G1 X131.751 Y135.4 E.19639
G1 X131.751 Y136.051 E.02441
G1 X127.367 Y131.667 E.2325
G1 X127.205 Y131.653 E.00608
G1 X126.504 Y131.455 E.02731
G1 X131.751 Y136.701 E.27824
G1 X131.751 Y136.721 E.00073
G1 X131.119 Y136.721 E.02368
G1 X124.459 Y130.061 E.35323
G1 X124.092 Y129.871 E.0155
G1 X124.092 Y130.344 E.01773
G1 X130.468 Y136.721 E.3382
G1 X129.818 Y136.721 E.02441
G1 X124.092 Y130.995 E.30368
G1 X124.092 Y131.646 E.02441
G1 X129.167 Y136.721 E.26916
G1 X128.516 Y136.721 E.02441
G1 X124.092 Y132.297 E.23465
G1 X124.092 Y132.947 E.02441
G1 X127.865 Y136.721 E.20013
G1 X127.214 Y136.721 E.02441
G1 X124.092 Y133.598 E.16561
G1 X124.092 Y134.249 E.02441
G1 X126.563 Y136.721 E.1311
G1 X125.913 Y136.721 E.02441
G1 X124.092 Y134.9 E.09658
G1 X124.092 Y135.551 E.02441
G1 X125.262 Y136.721 E.06206
G1 X124.611 Y136.721 E.02441
G1 X123.886 Y135.996 E.03845
; WIPE_START
G1 X124.611 Y136.721 E-.38965
G1 X125.262 Y136.721 E-.24732
G1 X125.033 Y136.492 E-.12304
; WIPE_END
G1 E-.04 F1800
M204 S6000
G1 X124.495 Y129.36 Z.6 F42000
G1 Z.2
G1 E.8 F1800
; FEATURE: Gap infill
; LINE_WIDTH: 0.501454
G1 F3000
M204 S500
G1 X124.167 Y129.191 E.01377
G1 X124.164 Y129.167 E.00091
; LINE_WIDTH: 0.485493
G1 X124.142 Y129.005 E.00589
; LINE_WIDTH: 0.442877
G1 X124.121 Y128.843 E.00532
; LINE_WIDTH: 0.400261
G1 X124.1 Y128.681 E.00475
; LINE_WIDTH: 0.357646
G1 X124.078 Y128.519 E.00419
; LINE_WIDTH: 0.318939
G3 X124.062 Y128.17 I3.99 J-.362 E.00786
; LINE_WIDTH: 0.289479
G1 X124.048 Y127.847 E.00651
; LINE_WIDTH: 0.288644
G3 X124.062 Y127.503 I3.945 J-.01 E.0069
; LINE_WIDTH: 0.317745
G1 X124.076 Y127.179 E.00726
; LINE_WIDTH: 0.349543
G3 X124.114 Y126.881 I3.427 J.287 E.00751
; CHANGE_LAYER
; Z_HEIGHT: 0.4
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F3000
G1 X124.076 Y127.179 E-.76
; WIPE_END
G1 E-.04 F1800
; layer num/total_layer_count: 2/2
; update layer progress
M73 L2
M991 S0 P1 ;notify layer change
M106 S186.15
; open powerlost recovery
M1003 S1
; OBJECT_ID: 95
M204 S10000
G17
G3 Z.6 I.661 J1.022 P1  F42000
G1 X127.508 Y124.958 Z.6
G1 Z.4
G1 E.8 F1800
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F12000
M204 S5000
G1 X127.878 Y124.926 E.01141
G3 X127.373 Y124.97 I0 J2.906 E.5454
G1 X127.448 Y124.963 E.00234
; WIPE_START
M204 S6000
G1 X127.878 Y124.926 E-.16391
G1 X128.384 Y124.97 E-.1928
G1 X128.874 Y125.101 E-.19281
G1 X129.333 Y125.316 E-.1928
G1 X129.372 Y125.342 E-.01768
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X136.995 Y125.708 Z.8 F42000
G1 X178.131 Y127.681 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S5000
G1 X176.711 Y127.681 E.04363
G1 X176.711 Y137.23 E.29339
G3 X176.259 Y137.681 I-.45 J.002 E.02183
G1 X167.583 Y137.681 E.2666
G3 X167.131 Y137.23 I.02 J-.471 E.02157
G1 X167.131 Y127.681 E.29339
G1 X165.711 Y127.681 E.04363
G1 X165.711 Y137.23 E.29339
G3 X165.259 Y137.681 I-.45 J.002 E.02183
G1 X156.583 Y137.681 E.2666
G3 X156.131 Y137.23 I.02 J-.471 E.02157
G1 X156.131 Y127.681 E.29339
G1 X154.711 Y127.681 E.04363
G1 X154.711 Y137.23 E.29339
G3 X154.259 Y137.681 I-.45 J.002 E.02183
G1 X145.583 Y137.681 E.2666
G3 X145.131 Y137.23 I.02 J-.471 E.02157
G1 X145.131 Y127.681 E.29339
G1 X143.711 Y127.681 E.04363
G1 X143.711 Y137.23 E.29339
G3 X143.259 Y137.681 I-.45 J.002 E.02183
G1 X134.583 Y137.681 E.2666
G3 X134.131 Y137.23 I.02 J-.471 E.02157
G1 X134.131 Y127.681 E.29339
G1 X132.711 Y127.681 E.04363
G1 X132.711 Y137.23 E.29339
G3 X132.259 Y137.681 I-.45 J.002 E.02183
G1 X123.583 Y137.681 E.2666
G3 X123.131 Y137.23 I-.002 J-.45 E.02183
G1 X123.131 Y127.681 E.29339
G1 X121.711 Y127.681 E.04363
G1 X121.711 Y137.23 E.29339
G3 X121.259 Y137.681 I-.45 J.002 E.02183
G1 X112.583 Y137.681 E.2666
G3 X112.131 Y137.23 I.02 J-.471 E.02157
G1 X112.131 Y127.681 E.29339
G1 X110.711 Y127.681 E.04363
G1 X110.711 Y137.23 E.29339
M73 P84 R1
G3 X110.259 Y137.681 I-.45 J.002 E.02183
G1 X101.583 Y137.681 E.2666
G3 X101.131 Y137.23 I.02 J-.471 E.02157
G1 X101.131 Y127.681 E.29339
G1 X99.711 Y127.681 E.04363
G1 X99.711 Y137.23 E.29339
G3 X99.259 Y137.681 I-.45 J.002 E.02183
G1 X90.583 Y137.681 E.2666
G3 X90.131 Y137.23 I-.002 J-.45 E.02183
G1 X90.131 Y127.681 E.29339
G1 X88.711 Y127.681 E.04363
G1 X88.711 Y137.23 E.29339
G3 X88.259 Y137.681 I-.45 J.002 E.02183
G1 X79.583 Y137.681 E.2666
G3 X79.131 Y137.23 I.02 J-.471 E.02157
G1 X79.131 Y127.681 E.29339
G1 X77.711 Y127.681 E.04363
G1 X77.711 Y137.23 E.29339
G3 X77.259 Y137.681 I-.45 J.002 E.02183
G1 X68.583 Y137.681 E.2666
G3 X68.131 Y137.23 I-.002 J-.45 E.02182
G1 X68.2 Y122.761 E.44461
G3 X68.553 Y122.409 I.35 J-.001 E.01702
G1 X125.374 Y122.409 E1.74596
G1 X126.545 Y121.618 E.04343
G1 X129.369 Y121.618 E.08678
G1 X130.194 Y122.175 E.03059
G1 X130.54 Y122.409 E.01283
G1 X187.361 Y122.409 E1.74596
G3 X187.711 Y122.76 I.003 J.347 E.01695
G1 X187.711 Y137.229 E.44462
G3 X187.259 Y137.681 I-.45 J.002 E.02183
G1 X178.583 Y137.681 E.2666
G3 X178.131 Y137.23 I-.002 J-.45 E.02183
G1 X178.131 Y127.741 E.29155
; WIPE_START
M204 S6000
G1 X176.711 Y127.681 E-.54008
G1 X176.711 Y128.26 E-.21992
; WIPE_END
G1 E-.04 F1800
M204 S10000
G17
G3 Z.8 I1.217 J0 P1  F42000
;===================== date: 20250206 =====================

; don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer
; SKIPPABLE_START
; SKIPTYPE: timelapse
M622.1 S1 ; for prev firmware, default turned on
M1002 judge_flag timelapse_record_flag
M622 J1
G92 E0
G1 Z0.8
G1 X0 Y129.65 F18000 ; move to safe pos
G1 X-48.2 F3000 ; move to safe pos
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


G1 X187.504 Y137.02 F42000
G1 Z.4
G1 E.8 F1800
; FEATURE: Top surface
G1 F12000
M204 S2000
G1 X187.05 Y137.474 E.01973
G1 X186.516 Y137.474
G1 X187.504 Y136.487 E.04291
G1 X187.504 Y135.953
G1 X185.983 Y137.474 E.06608
G1 X185.45 Y137.474
G1 X187.504 Y135.42 E.08925
G1 X187.504 Y134.887
G1 X184.916 Y137.474 E.11243
G1 X184.383 Y137.474
G1 X187.504 Y134.354 E.1356
G1 X187.504 Y133.82
G1 X183.85 Y137.474 E.15877
G1 X183.317 Y137.474
G1 X187.504 Y133.287 E.18194
G1 X187.504 Y132.754
G1 X182.783 Y137.474 E.20512
G1 X182.25 Y137.474
G1 X187.504 Y132.221 E.22829
G1 X187.504 Y131.687
G1 X181.717 Y137.474 E.25146
G1 X181.184 Y137.474
G1 X187.504 Y131.154 E.27463
G1 X187.504 Y130.621
G1 X180.65 Y137.474 E.29781
G1 X180.117 Y137.474
G1 X187.504 Y130.088 E.32098
G1 X187.504 Y129.554
G1 X179.584 Y137.474 E.34415
G1 X179.051 Y137.474
G1 X187.504 Y129.021 E.36732
G1 X187.504 Y128.488
G1 X178.533 Y137.458 E.3898
G1 X178.338 Y137.12
G1 X187.504 Y127.954 E.39827
G1 X187.504 Y127.421
G1 X178.338 Y136.586 E.39827
G1 X178.338 Y136.053
G1 X187.504 Y126.888 E.39827
G1 X187.504 Y126.355
G1 X178.338 Y135.52 E.39827
G1 X178.338 Y134.987
M73 P85 R1
G1 X187.504 Y125.821 E.39827
G1 X187.504 Y125.288
G1 X178.338 Y134.453 E.39827
G1 X178.338 Y133.92
G1 X187.504 Y124.755 E.39827
G1 X187.504 Y124.222
G1 X178.338 Y133.387 E.39827
G1 X178.338 Y132.854
G1 X187.504 Y123.688 E.39827
G1 X187.504 Y123.155
G1 X178.338 Y132.32 E.39827
G1 X178.338 Y131.787
G1 X187.465 Y122.66 E.3966
G1 X186.966 Y122.627
G1 X178.338 Y131.254 E.3749
G1 X178.338 Y130.721
G1 X186.433 Y122.626 E.35173
G1 X185.899 Y122.626
G1 X178.338 Y130.187 E.32856
G1 X178.338 Y129.654
G1 X185.366 Y122.626 E.30539
G1 X184.833 Y122.626
G1 X178.338 Y129.121 E.28223
G1 X178.338 Y128.588
G1 X184.3 Y122.626 E.25906
G1 X183.767 Y122.626
G1 X178.338 Y128.054 E.23589
G1 X178.338 Y127.521
G1 X183.234 Y122.626 E.21272
G1 X182.7 Y122.626
G1 X177.852 Y127.474 E.21068
G1 X177.319 Y127.474
G1 X182.167 Y122.626 E.21068
G1 X181.634 Y122.626
G1 X176.786 Y127.474 E.21068
; WIPE_START
M204 S6000
G1 X178.2 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X177.012 Y133.599 Z.8 F42000
G1 X176.504 Y136.821 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X175.851 Y137.474 E.02836
G1 X175.318 Y137.474
G1 X176.504 Y136.288 E.05153
G1 X176.504 Y135.755
G1 X174.785 Y137.474 E.0747
G1 X174.251 Y137.474
G1 X176.504 Y135.222 E.09787
G1 X176.504 Y134.688
G1 X173.718 Y137.474 E.12105
G1 X173.185 Y137.474
G1 X176.504 Y134.155 E.14422
G1 X176.504 Y133.622
G1 X172.652 Y137.474 E.16739
G1 X172.118 Y137.474
G1 X176.504 Y133.089 E.19056
G1 X176.504 Y132.555
G1 X171.585 Y137.474 E.21374
G1 X171.052 Y137.474
G1 X176.504 Y132.022 E.23691
G1 X176.504 Y131.489
G1 X170.519 Y137.474 E.26008
G1 X169.985 Y137.474
G1 X176.504 Y130.956 E.28326
G1 X176.504 Y130.422
G1 X169.452 Y137.474 E.30643
G1 X168.919 Y137.474
G1 X176.504 Y129.889 E.3296
G1 X176.504 Y129.356
G1 X168.386 Y137.474 E.35277
G1 X167.852 Y137.474
G1 X176.504 Y128.823 E.37595
G1 X176.504 Y128.289
G1 X167.403 Y137.39 E.39545
G1 X167.338 Y136.921
G1 X176.504 Y127.756 E.39827
; WIPE_START
M204 S6000
G1 X175.089 Y129.17 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X180.253 Y123.549 Z.8 F42000
G1 X181.101 Y122.626 Z.8
G1 Z.4
G1 E.8 F1800
M73 P86 R1
G1 F12000
M204 S2000
G1 X167.338 Y136.388 E.59805
G1 X167.338 Y135.855
G1 X180.568 Y122.625 E.57488
G1 X180.035 Y122.625
G1 X167.338 Y135.322 E.55171
G1 X167.338 Y134.788
G1 X179.501 Y122.625 E.52854
G1 X178.968 Y122.625
G1 X167.338 Y134.255 E.50537
G1 X167.338 Y133.722
G1 X178.435 Y122.625 E.48221
G1 X177.902 Y122.625
G1 X167.338 Y133.189 E.45904
G1 X167.338 Y132.655
G1 X177.369 Y122.625 E.43587
G1 X176.836 Y122.625
G1 X167.338 Y132.122 E.4127
G1 X167.338 Y131.589
G1 X176.302 Y122.625 E.38953
G1 X175.769 Y122.625
G1 X167.338 Y131.055 E.36636
G1 X167.338 Y130.522
G1 X175.236 Y122.625 E.34319
G1 X174.703 Y122.624
G1 X167.338 Y129.989 E.32002
G1 X167.338 Y129.456
G1 X174.17 Y122.624 E.29686
G1 X173.637 Y122.624
G1 X167.338 Y128.922 E.27369
G1 X167.338 Y128.389
G1 X173.103 Y122.624 E.25052
G1 X172.57 Y122.624
G1 X167.338 Y127.856 E.22735
G1 X167.187 Y127.474
G1 X172.037 Y122.624 E.21076
G1 X171.504 Y122.624
G1 X166.654 Y127.474 E.21076
G1 X166.121 Y127.474
G1 X170.971 Y122.624 E.21076
G1 X170.438 Y122.624
G1 X165.587 Y127.474 E.21077
; WIPE_START
M204 S6000
G1 X167.002 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X165.463 Y133.536 Z.8 F42000
G1 X164.653 Y137.474 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X165.504 Y136.623 E.03698
G1 X165.504 Y136.09
G1 X164.119 Y137.474 E.06015
G1 X163.586 Y137.474
G1 X165.504 Y135.557 E.08332
G1 X165.504 Y135.023
G1 X163.053 Y137.474 E.1065
G1 X162.52 Y137.474
G1 X165.504 Y134.49 E.12967
G1 X165.504 Y133.957
G1 X161.986 Y137.474 E.15284
G1 X161.453 Y137.474
G1 X165.504 Y133.423 E.17601
G1 X165.504 Y132.89
G1 X160.92 Y137.474 E.19919
G1 X160.387 Y137.474
G1 X165.504 Y132.357 E.22236
G1 X165.504 Y131.824
G1 X159.853 Y137.474 E.24553
G1 X159.32 Y137.474
G1 X165.504 Y131.29 E.2687
G1 X165.504 Y130.757
G1 X158.787 Y137.474 E.29188
G1 X158.254 Y137.474
G1 X165.504 Y130.224 E.31505
G1 X165.504 Y129.691
G1 X157.72 Y137.474 E.33822
G1 X157.187 Y137.474
G1 X165.504 Y129.157 E.3614
G1 X165.504 Y128.624
G1 X156.654 Y137.474 E.38457
G1 X156.342 Y137.252
G1 X165.504 Y128.091 E.39811
G1 X165.504 Y127.558
G1 X156.338 Y136.723 E.39827
G1 X156.338 Y136.19
M73 P87 R1
G1 X169.904 Y122.624 E.58951
G1 X169.371 Y122.624
G1 X156.338 Y135.656 E.56634
G1 X156.338 Y135.123
G1 X168.838 Y122.623 E.54317
G1 X168.305 Y122.623
G1 X156.338 Y134.59 E.52
G1 X156.338 Y134.057
G1 X167.772 Y122.623 E.49684
G1 X167.239 Y122.623
G1 X156.338 Y133.523 E.47367
G1 X156.338 Y132.99
G1 X166.705 Y122.623 E.4505
G1 X166.172 Y122.623
G1 X156.338 Y132.457 E.42733
G1 X156.338 Y131.924
G1 X165.639 Y122.623 E.40416
G1 X165.106 Y122.623
G1 X156.338 Y131.39 E.38099
G1 X156.338 Y130.857
G1 X164.573 Y122.623 E.35782
G1 X164.04 Y122.623
G1 X156.338 Y130.324 E.33465
G1 X156.338 Y129.791
G1 X163.506 Y122.623 E.31149
G1 X162.973 Y122.622
G1 X156.338 Y129.257 E.28832
G1 X156.338 Y128.724
G1 X162.44 Y122.622 E.26515
G1 X161.907 Y122.622
G1 X156.338 Y128.191 E.24198
G1 X156.338 Y127.658
G1 X161.374 Y122.622 E.21881
G1 X160.841 Y122.622
G1 X155.989 Y127.474 E.21084
G1 X155.455 Y127.474
G1 X160.307 Y122.622 E.21084
G1 X159.774 Y122.622
G1 X154.922 Y127.474 E.21085
; WIPE_START
M204 S6000
G1 X156.336 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X155.071 Y133.587 Z.8 F42000
G1 X154.504 Y136.958 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X153.988 Y137.474 E.02243
G1 X153.454 Y137.474
G1 X154.504 Y136.425 E.0456
G1 X154.504 Y135.891
G1 X152.921 Y137.474 E.06877
G1 X152.388 Y137.474
G1 X154.504 Y135.358 E.09195
G1 X154.504 Y134.825
G1 X151.855 Y137.474 E.11512
G1 X151.321 Y137.474
G1 X154.504 Y134.292 E.13829
G1 X154.504 Y133.758
G1 X150.788 Y137.474 E.16146
G1 X150.255 Y137.474
G1 X154.504 Y133.225 E.18464
G1 X154.504 Y132.692
G1 X149.722 Y137.474 E.20781
G1 X149.188 Y137.474
G1 X154.504 Y132.159 E.23098
G1 X154.504 Y131.625
G1 X148.655 Y137.474 E.25415
G1 X148.122 Y137.474
G1 X154.504 Y131.092 E.27733
G1 X154.504 Y130.559
G1 X147.588 Y137.474 E.3005
G1 X147.055 Y137.474
G1 X154.504 Y130.026 E.32367
G1 X154.504 Y129.492
G1 X146.522 Y137.474 E.34684
G1 X145.989 Y137.474
G1 X154.504 Y128.959 E.37002
G1 X154.504 Y128.426
G1 X145.483 Y137.447 E.39201
G1 X145.338 Y137.058
M73 P88 R1
G1 X154.504 Y127.892 E.39827
; WIPE_START
M204 S6000
G1 X153.089 Y129.307 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X158.258 Y123.69 Z.8 F42000
G1 X159.241 Y122.622 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X145.338 Y136.524 E.60414
G1 X145.338 Y135.991
G1 X158.708 Y122.622 E.58097
G1 X158.175 Y122.622
G1 X145.338 Y135.458 E.5578
G1 X145.338 Y134.925
G1 X157.642 Y122.622 E.53463
G1 X157.108 Y122.621
G1 X145.338 Y134.391 E.51147
G1 X145.338 Y133.858
G1 X156.575 Y122.621 E.4883
G1 X156.042 Y122.621
G1 X145.338 Y133.325 E.46513
G1 X145.338 Y132.792
G1 X155.509 Y122.621 E.44196
G1 X154.976 Y122.621
G1 X145.338 Y132.258 E.41879
G1 X145.338 Y131.725
G1 X154.443 Y122.621 E.39562
G1 X153.909 Y122.621
G1 X145.338 Y131.192 E.37245
G1 X145.338 Y130.659
G1 X153.376 Y122.621 E.34928
G1 X152.843 Y122.621
G1 X145.338 Y130.125 E.32612
G1 X145.338 Y129.592
G1 X152.31 Y122.621 E.30295
G1 X151.777 Y122.62
G1 X145.338 Y129.059 E.27978
G1 X145.338 Y128.526
G1 X151.244 Y122.62 E.25661
G1 X150.71 Y122.62
G1 X145.338 Y127.992 E.23344
G1 X145.324 Y127.474
G1 X150.177 Y122.62 E.21092
G1 X149.644 Y122.62
G1 X144.79 Y127.474 E.21092
G1 X144.257 Y127.474
G1 X149.111 Y122.62 E.21093
G1 X148.578 Y122.62
G1 X143.724 Y127.474 E.21093
; WIPE_START
M204 S6000
G1 X145.138 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X143.986 Y133.605 Z.8 F42000
G1 X143.504 Y136.759 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X142.789 Y137.474 E.03105
G1 X142.256 Y137.474
G1 X143.504 Y136.226 E.05422
G1 X143.504 Y135.693
G1 X141.723 Y137.474 E.0774
G1 X141.189 Y137.474
G1 X143.504 Y135.16 E.10057
G1 X143.504 Y134.626
G1 X140.656 Y137.474 E.12374
G1 X140.123 Y137.474
G1 X143.504 Y134.093 E.14691
G1 X143.504 Y133.56
G1 X139.59 Y137.474 E.17009
G1 X139.056 Y137.474
G1 X143.504 Y133.027 E.19326
G1 X143.504 Y132.493
G1 X138.523 Y137.474 E.21643
G1 X137.99 Y137.474
G1 X143.504 Y131.96 E.2396
G1 X143.504 Y131.427
G1 X137.457 Y137.474 E.26278
G1 X136.923 Y137.474
G1 X143.504 Y130.894 E.28595
G1 X143.504 Y130.36
G1 X136.39 Y137.474 E.30912
G1 X135.857 Y137.474
G1 X143.504 Y129.827 E.33229
M73 P89 R1
G1 X143.504 Y129.294
G1 X135.324 Y137.474 E.35547
G1 X134.79 Y137.474
G1 X143.504 Y128.761 E.37864
G1 X143.504 Y128.227
G1 X134.38 Y137.351 E.39649
G1 X134.338 Y136.859
G1 X143.504 Y127.694 E.39827
; WIPE_START
M204 S6000
G1 X142.089 Y129.108 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X147.25 Y123.485 Z.8 F42000
G1 X148.045 Y122.62 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X134.338 Y136.326 E.5956
G1 X134.338 Y135.793
G1 X147.511 Y122.62 E.57243
G1 X146.978 Y122.62
G1 X134.338 Y135.26 E.54926
G1 X134.338 Y134.726
G1 X146.445 Y122.62 E.5261
G1 X145.912 Y122.619
G1 X134.338 Y134.193 E.50293
G1 X134.338 Y133.66
G1 X145.379 Y122.619 E.47976
G1 X144.846 Y122.619
G1 X134.338 Y133.127 E.45659
G1 X134.338 Y132.593
G1 X144.312 Y122.619 E.43342
G1 X143.779 Y122.619
G1 X134.338 Y132.06 E.41025
G1 X134.338 Y131.527
G1 X143.246 Y122.619 E.38708
G1 X142.713 Y122.619
G1 X134.338 Y130.993 E.36392
G1 X134.338 Y130.46
G1 X142.18 Y122.619 E.34075
G1 X141.647 Y122.619
G1 X134.338 Y129.927 E.31758
G1 X134.338 Y129.394
G1 X141.114 Y122.619 E.29441
G1 X140.58 Y122.619
G1 X134.338 Y128.86 E.27124
G1 X134.338 Y128.327
G1 X140.047 Y122.618 E.24807
G1 X139.514 Y122.618
G1 X134.338 Y127.794 E.2249
G1 X134.125 Y127.474
G1 X138.981 Y122.618 E.211
G1 X138.448 Y122.618
G1 X133.592 Y127.474 E.21101
G1 X133.059 Y127.474
G1 X137.915 Y122.618 E.21101
G1 X137.381 Y122.618
G1 X132.525 Y127.474 E.21101
; WIPE_START
M204 S6000
G1 X133.94 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X132.906 Y133.622 Z.8 F42000
G1 X132.504 Y136.561 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X131.591 Y137.474 E.03967
G1 X131.057 Y137.474
G1 X132.504 Y136.028 E.06284
G1 X132.504 Y135.495
G1 X130.524 Y137.474 E.08602
G1 X129.991 Y137.474
G1 X132.504 Y134.961 E.10919
G1 X132.504 Y134.428
G1 X129.458 Y137.474 E.13236
G1 X128.924 Y137.474
M73 P89 R0
G1 X132.504 Y133.895 E.15553
G1 X132.504 Y133.362
G1 X128.391 Y137.474 E.17871
G1 X127.858 Y137.474
G1 X132.504 Y132.828 E.20188
G1 X132.504 Y132.295
G1 X127.325 Y137.474 E.22505
M73 P90 R0
G1 X126.791 Y137.474
G1 X132.504 Y131.762 E.24823
G1 X132.504 Y131.228
G1 X126.258 Y137.474 E.2714
G1 X125.725 Y137.474
G1 X132.504 Y130.695 E.29457
G1 X132.504 Y130.162
G1 X125.192 Y137.474 E.31774
G1 X124.658 Y137.474
G1 X132.504 Y129.629 E.34092
G1 X132.504 Y129.095
G1 X124.125 Y137.474 E.36409
G1 X123.594 Y137.472
G1 X132.504 Y128.562 E.38715
G1 X132.504 Y128.029
G1 X123.338 Y137.194 E.39827
G1 X123.338 Y136.661
G1 X129.518 Y130.481 E.26855
G1 X128.609 Y130.857
G1 X123.338 Y136.128 E.22902
G1 X123.338 Y135.594
G1 X127.987 Y130.946 E.202
G1 X127.479 Y130.921
G1 X123.338 Y135.061 E.17993
G1 X123.338 Y134.528
G1 X127.038 Y130.828 E.16077
G1 X126.644 Y130.689
G1 X123.338 Y133.995 E.14364
G1 X123.338 Y133.461
G1 X126.285 Y130.514 E.12806
G1 X125.972 Y130.295
G1 X123.338 Y132.928 E.11443
G1 X123.338 Y132.395
G1 X125.69 Y130.043 E.10221
G1 X125.435 Y129.765
G1 X123.338 Y131.862 E.09112
G1 X123.338 Y131.328
G1 X125.216 Y129.451 E.08157
G1 X125.037 Y129.096
G1 X123.338 Y130.795 E.07382
G1 X123.338 Y130.262
G1 X124.894 Y128.706 E.06761
G1 X124.797 Y128.27
G1 X123.338 Y129.729 E.06338
G1 X123.338 Y129.195
G1 X124.765 Y127.769 E.06199
G1 X124.843 Y127.157
G1 X123.338 Y128.662 E.0654
G1 X123.338 Y128.129
G1 X125.165 Y126.302 E.07939
; WIPE_START
M204 S6000
G1 X123.751 Y127.716 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X130.523 Y129.477 Z.8 F42000
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X132.504 Y127.496 E.08609
; WIPE_START
M204 S6000
G1 X131.089 Y128.91 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X130.899 Y128.567 Z.8 F42000
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X136.848 Y122.618 E.25852
G1 X136.315 Y122.618
G1 X130.988 Y127.945 E.2315
G1 X130.962 Y127.437
G1 X135.782 Y122.618 E.20944
G1 X135.249 Y122.618
G1 X130.87 Y126.997 E.19029
G1 X130.731 Y126.602
G1 X134.716 Y122.618 E.17315
G1 X134.182 Y122.617
G1 X130.556 Y126.244 E.15758
G1 X130.336 Y125.93
G1 X133.649 Y122.617 E.14396
G1 X133.116 Y122.617
G1 X130.085 Y125.649 E.13173
G1 X129.806 Y125.394
G1 X132.583 Y122.617 E.12065
G1 X132.05 Y122.617
G1 X129.493 Y125.174 E.11111
G1 X129.138 Y124.996
G1 X131.517 Y122.617 E.10336
G1 X130.983 Y122.617
G1 X128.748 Y124.853 E.09716
G1 X128.312 Y124.755
G1 X130.461 Y122.606 E.09338
G1 X130.142 Y122.391
G1 X127.81 Y124.723 E.10134
G1 X127.199 Y124.802
G1 X129.824 Y122.176 E.1141
G1 X129.506 Y121.961
G1 X126.344 Y125.124 E.13743
; WIPE_START
M204 S6000
G1 X127.758 Y123.709 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X129.109 Y121.825 Z.8 F42000
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X123.338 Y127.596 E.25076
G1 X122.927 Y127.474
G1 X128.576 Y121.825 E.24547
G1 X128.042 Y121.825
G1 X122.393 Y127.474 E.24547
G1 X121.86 Y127.474
G1 X127.509 Y121.825 E.24547
; WIPE_START
M204 S6000
G1 X126.095 Y123.239 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
M73 P91 R0
G1 X123.663 Y130.474 Z.8 F42000
G1 X121.504 Y136.896 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X120.926 Y137.474 E.02512
G1 X120.392 Y137.474
G1 X121.504 Y136.363 E.04829
G1 X121.504 Y135.829
G1 X119.859 Y137.474 E.07147
G1 X119.326 Y137.474
G1 X121.504 Y135.296 E.09464
G1 X121.504 Y134.763
G1 X118.793 Y137.474 E.11781
G1 X118.259 Y137.474
G1 X121.504 Y134.23 E.14098
G1 X121.504 Y133.696
G1 X117.726 Y137.474 E.16416
G1 X117.193 Y137.474
G1 X121.504 Y133.163 E.18733
G1 X121.504 Y132.63
G1 X116.66 Y137.474 E.2105
G1 X116.126 Y137.474
G1 X121.504 Y132.097 E.23367
G1 X121.504 Y131.563
G1 X115.593 Y137.474 E.25685
G1 X115.06 Y137.474
G1 X121.504 Y131.03 E.28002
G1 X121.504 Y130.497
G1 X114.526 Y137.474 E.30319
G1 X113.993 Y137.474
G1 X121.504 Y129.964 E.32636
G1 X121.504 Y129.43
G1 X113.46 Y137.474 E.34954
G1 X112.927 Y137.474
G1 X121.504 Y128.897 E.37271
G1 X121.504 Y128.364
G1 X112.445 Y137.423 E.39365
G1 X112.338 Y136.996
G1 X121.504 Y127.831 E.39827
; WIPE_START
M204 S6000
G1 X120.089 Y129.245 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X125.282 Y123.65 Z.8 F42000
G1 X126.976 Y121.825 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X112.338 Y136.463 E.63607
G1 X112.338 Y135.929
G1 X126.096 Y122.171 E.59785
G1 X125.118 Y122.617
G1 X112.338 Y135.396 E.55531
G1 X112.338 Y134.863
G1 X124.584 Y122.617 E.53214
G1 X124.051 Y122.617
G1 X112.338 Y134.329 E.50896
G1 X112.338 Y133.796
G1 X123.517 Y122.617 E.48578
G1 X122.984 Y122.617
G1 X112.338 Y133.263 E.46261
G1 X112.338 Y132.73
G1 X122.451 Y122.617 E.43943
G1 X121.917 Y122.617
G1 X112.338 Y132.196 E.41625
G1 X112.338 Y131.663
G1 X121.384 Y122.618 E.39308
G1 X120.851 Y122.618
G1 X112.338 Y131.13 E.3699
G1 X112.338 Y130.597
G1 X120.317 Y122.618 E.34673
G1 X119.784 Y122.618
G1 X112.338 Y130.063 E.32355
G1 X112.338 Y129.53
G1 X119.251 Y122.618 E.30037
G1 X118.717 Y122.618
G1 X112.338 Y128.997 E.2772
G1 X112.338 Y128.464
G1 X118.184 Y122.618 E.25402
G1 X117.651 Y122.618
G1 X112.338 Y127.93 E.23084
G1 X112.262 Y127.474
G1 X117.117 Y122.618 E.211
G1 X116.584 Y122.618
G1 X111.728 Y127.474 E.211
G1 X111.195 Y127.474
G1 X116.051 Y122.618 E.211
G1 X115.517 Y122.619
G1 X110.662 Y127.474 E.21099
; WIPE_START
M204 S6000
G1 X112.076 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X110.96 Y133.61 Z.8 F42000
G1 X110.504 Y136.697 Z.8
G1 Z.4
M73 P92 R0
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X109.727 Y137.474 E.03374
G1 X109.194 Y137.474
G1 X110.504 Y136.164 E.05692
G1 X110.504 Y135.631
G1 X108.661 Y137.474 E.08009
G1 X108.127 Y137.474
G1 X110.504 Y135.098 E.10326
G1 X110.504 Y134.564
G1 X107.594 Y137.474 E.12643
G1 X107.061 Y137.474
G1 X110.504 Y134.031 E.14961
G1 X110.504 Y133.498
G1 X106.528 Y137.474 E.17278
G1 X105.994 Y137.474
G1 X110.504 Y132.965 E.19595
G1 X110.504 Y132.431
G1 X105.461 Y137.474 E.21912
G1 X104.928 Y137.474
G1 X110.504 Y131.898 E.2423
G1 X110.504 Y131.365
G1 X104.395 Y137.474 E.26547
G1 X103.861 Y137.474
G1 X110.504 Y130.832 E.28864
G1 X110.504 Y130.298
G1 X103.328 Y137.474 E.31181
G1 X102.795 Y137.474
G1 X110.504 Y129.765 E.33499
G1 X110.504 Y129.232
G1 X102.262 Y137.474 E.35816
G1 X101.728 Y137.474
G1 X110.504 Y128.699 E.38133
G1 X110.504 Y128.165
G1 X101.359 Y137.31 E.39736
G1 X101.338 Y136.797
G1 X110.504 Y127.632 E.39827
; WIPE_START
M204 S6000
G1 X109.089 Y129.046 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X114.248 Y123.421 Z.8 F42000
G1 X114.984 Y122.619 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X101.338 Y136.264 E.59296
G1 X101.338 Y135.731
G1 X114.451 Y122.619 E.56979
G1 X113.917 Y122.619
G1 X101.338 Y135.198 E.54661
G1 X101.338 Y134.664
G1 X113.384 Y122.619 E.52343
G1 X112.851 Y122.619
G1 X101.338 Y134.131 E.50026
G1 X101.338 Y133.598
G1 X112.317 Y122.619 E.47708
G1 X111.784 Y122.619
G1 X101.338 Y133.065 E.4539
G1 X101.338 Y132.531
G1 X111.25 Y122.619 E.43073
G1 X110.717 Y122.619
G1 X101.338 Y131.998 E.40755
G1 X101.338 Y131.465
G1 X110.184 Y122.619 E.38437
G1 X109.65 Y122.62
G1 X101.338 Y130.932 E.3612
G1 X101.338 Y130.398
G1 X109.117 Y122.62 E.33802
G1 X108.584 Y122.62
G1 X101.338 Y129.865 E.31484
G1 X101.338 Y129.332
G1 X108.05 Y122.62 E.29167
G1 X107.517 Y122.62
G1 X101.338 Y128.799 E.26849
G1 X101.338 Y128.265
G1 X106.984 Y122.62 E.24531
G1 X106.45 Y122.62
G1 X101.338 Y127.732 E.22214
G1 X101.063 Y127.474
G1 X105.917 Y122.62 E.21092
G1 X105.384 Y122.62
G1 X100.53 Y127.474 E.21092
G1 X99.997 Y127.474
G1 X104.85 Y122.62 E.21091
; WIPE_START
M204 S6000
G1 X103.436 Y124.035 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X97.658 Y129.021 Z.8 F42000
M73 P93 R0
G1 X87.864 Y137.474 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X88.504 Y136.834 E.02781
G1 X88.504 Y136.301
G1 X87.33 Y137.474 E.05099
G1 X86.797 Y137.474
G1 X88.504 Y135.767 E.07416
G1 X88.504 Y135.234
G1 X86.264 Y137.474 E.09733
G1 X85.731 Y137.474
G1 X88.504 Y134.701 E.1205
G1 X88.504 Y134.168
G1 X85.197 Y137.474 E.14368
G1 X84.664 Y137.474
G1 X88.504 Y133.634 E.16685
G1 X88.504 Y133.101
G1 X84.131 Y137.474 E.19002
G1 X83.598 Y137.474
G1 X88.504 Y132.568 E.21319
G1 X88.504 Y132.035
G1 X83.064 Y137.474 E.23637
G1 X82.531 Y137.474
G1 X88.504 Y131.501 E.25954
G1 X88.504 Y130.968
G1 X81.998 Y137.474 E.28271
G1 X81.465 Y137.474
G1 X88.504 Y130.435 E.30588
G1 X88.504 Y129.902
G1 X80.931 Y137.474 E.32906
G1 X80.398 Y137.474
G1 X88.504 Y129.368 E.35223
G1 X88.504 Y128.835
G1 X79.865 Y137.474 E.3754
G1 X79.41 Y137.395
G1 X88.504 Y128.302 E.39514
G1 X88.504 Y127.769
G1 X79.338 Y136.934 E.39827
; WIPE_START
M204 S6000
G1 X80.753 Y135.52 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X77.504 Y136.636 Z.8 F42000
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X76.665 Y137.474 E.03643
G1 X76.132 Y137.474
G1 X77.504 Y136.102 E.05961
G1 X77.504 Y135.569
G1 X75.599 Y137.474 E.08278
G1 X75.065 Y137.474
G1 X77.504 Y135.036 E.10595
G1 X77.504 Y134.503
G1 X74.532 Y137.474 E.12913
G1 X73.999 Y137.474
G1 X77.504 Y133.969 E.1523
G1 X77.504 Y133.436
G1 X73.466 Y137.474 E.17547
G1 X72.932 Y137.474
G1 X77.504 Y132.903 E.19864
G1 X77.504 Y132.369
G1 X72.399 Y137.474 E.22182
G1 X71.866 Y137.474
G1 X77.504 Y131.836 E.24499
G1 X77.504 Y131.303
G1 X71.333 Y137.474 E.26816
G1 X70.799 Y137.474
G1 X77.504 Y130.77 E.29133
G1 X77.504 Y130.236
G1 X70.266 Y137.474 E.31451
G1 X69.733 Y137.474
G1 X77.504 Y129.703 E.33768
G1 X77.504 Y129.17
G1 X69.2 Y137.474 E.36085
G1 X68.666 Y137.474
G1 X77.504 Y128.637 E.38402
G1 X77.504 Y128.103
G1 X68.344 Y137.264 E.39805
G1 X68.341 Y136.733
G1 X77.504 Y127.57 E.39817
; WIPE_START
M204 S6000
G1 X76.089 Y128.984 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X83.249 Y131.63 Z.8 F42000
G1 X99.062 Y137.474 Z.8
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X99.504 Y137.032 E.01919
G1 X99.504 Y136.499
G1 X98.529 Y137.474 E.04236
G1 X97.996 Y137.474
G1 X99.504 Y135.966 E.06554
G1 X99.504 Y135.433
G1 X97.462 Y137.474 E.08871
G1 X96.929 Y137.474
M73 P94 R0
G1 X99.504 Y134.899 E.11188
G1 X99.504 Y134.366
G1 X96.396 Y137.474 E.13505
G1 X95.862 Y137.474
G1 X99.504 Y133.833 E.15823
G1 X99.504 Y133.3
G1 X95.329 Y137.474 E.1814
G1 X94.796 Y137.474
G1 X99.504 Y132.766 E.20457
G1 X99.504 Y132.233
G1 X94.263 Y137.474 E.22774
G1 X93.729 Y137.474
G1 X99.504 Y131.7 E.25092
G1 X99.504 Y131.167
G1 X93.196 Y137.474 E.27409
G1 X92.663 Y137.474
G1 X99.504 Y130.633 E.29726
G1 X99.504 Y130.1
G1 X92.13 Y137.474 E.32043
G1 X91.596 Y137.474
G1 X99.504 Y129.567 E.34361
G1 X99.504 Y129.033
G1 X91.063 Y137.474 E.36678
G1 X90.544 Y137.46
G1 X99.504 Y128.5 E.38936
G1 X99.504 Y127.967
G1 X90.338 Y137.132 E.39827
G1 X90.338 Y136.599
G1 X104.317 Y122.62 E.60743
G1 X103.784 Y122.621
G1 X90.338 Y136.066 E.58426
G1 X90.338 Y135.532
G1 X103.25 Y122.621 E.56108
G1 X102.717 Y122.621
G1 X90.338 Y134.999 E.5379
G1 X90.338 Y134.466
G1 X102.184 Y122.621 E.51473
G1 X101.65 Y122.621
G1 X90.338 Y133.933 E.49155
G1 X90.338 Y133.399
G1 X101.117 Y122.621 E.46837
G1 X100.583 Y122.621
G1 X90.338 Y132.866 E.4452
G1 X90.338 Y132.333
G1 X100.05 Y122.621 E.42202
G1 X99.517 Y122.621
G1 X90.338 Y131.8 E.39884
G1 X90.338 Y131.266
G1 X98.983 Y122.621 E.37567
G1 X98.45 Y122.621
G1 X90.338 Y130.733 E.35249
G1 X90.338 Y130.2
G1 X97.917 Y122.622 E.32931
G1 X97.383 Y122.622
G1 X90.338 Y129.667 E.30614
G1 X90.338 Y129.133
G1 X96.85 Y122.622 E.28296
G1 X96.317 Y122.622
G1 X90.338 Y128.6 E.25978
G1 X90.338 Y128.067
G1 X95.783 Y122.622 E.23661
G1 X95.25 Y122.622
G1 X90.338 Y127.534 E.21343
G1 X89.865 Y127.474
G1 X94.717 Y122.622 E.21084
G1 X94.183 Y122.622
G1 X89.332 Y127.474 E.21083
G1 X88.798 Y127.474
G1 X93.65 Y122.622 E.21083
G1 X93.117 Y122.622
G1 X79.338 Y136.401 E.59873
G1 X79.338 Y135.867
G1 X92.583 Y122.622 E.57555
G1 X92.05 Y122.623
G1 X79.338 Y135.334 E.55237
G1 X79.338 Y134.801
G1 X91.517 Y122.623 E.5292
G1 X90.983 Y122.623
M73 P95 R0
G1 X79.338 Y134.268 E.50602
G1 X79.338 Y133.734
G1 X90.45 Y122.623 E.48284
G1 X89.916 Y122.623
G1 X79.338 Y133.201 E.45967
G1 X79.338 Y132.668
G1 X89.383 Y122.623 E.43649
G1 X88.85 Y122.623
G1 X79.338 Y132.134 E.41331
G1 X79.338 Y131.601
G1 X88.316 Y122.623 E.39014
G1 X87.783 Y122.623
G1 X79.338 Y131.068 E.36696
G1 X79.338 Y130.535
G1 X87.25 Y122.623 E.34379
G1 X86.716 Y122.624
G1 X79.338 Y130.001 E.32061
G1 X79.338 Y129.468
G1 X86.183 Y122.624 E.29743
G1 X85.65 Y122.624
G1 X79.338 Y128.935 E.27426
G1 X79.338 Y128.402
G1 X85.116 Y122.624 E.25108
G1 X84.583 Y122.624
G1 X79.338 Y127.868 E.2279
G1 X79.2 Y127.474
G1 X84.05 Y122.624 E.21076
G1 X83.516 Y122.624
G1 X78.666 Y127.474 E.21075
G1 X78.133 Y127.474
G1 X82.983 Y122.624 E.21075
G1 X82.45 Y122.624
G1 X77.6 Y127.474 E.21075
; WIPE_START
M204 S6000
G1 X79.014 Y126.06 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X81.916 Y122.624 Z.8 F42000
G1 Z.4
G1 E.8 F1800
G1 F12000
M204 S2000
G1 X68.343 Y136.197 E.58981
G1 X68.346 Y135.661
G1 X81.383 Y122.624 E.56652
G1 X80.85 Y122.625
G1 X68.349 Y135.126 E.54323
G1 X68.351 Y134.59
G1 X80.316 Y122.625 E.51994
G1 X79.783 Y122.625
G1 X68.354 Y134.054 E.49666
G1 X68.356 Y133.518
G1 X79.249 Y122.625 E.47337
G1 X78.716 Y122.625
G1 X68.359 Y132.982 E.45008
G1 X68.361 Y132.447
G1 X78.183 Y122.625 E.42679
G1 X77.649 Y122.625
G1 X68.364 Y131.911 E.40351
G1 X68.366 Y131.375
G1 X77.116 Y122.625 E.38022
G1 X76.583 Y122.625
G1 X68.369 Y130.839 E.35693
G1 X68.371 Y130.303
G1 X76.049 Y122.625 E.33364
G1 X75.516 Y122.625
G1 X68.374 Y129.768 E.31036
G1 X68.377 Y129.232
G1 X74.983 Y122.626 E.28707
G1 X74.449 Y122.626
G1 X68.379 Y128.696 E.26378
G1 X68.382 Y128.16
G1 X73.916 Y122.626 E.2405
G1 X73.383 Y122.626
G1 X68.384 Y127.624 E.21721
G1 X68.387 Y127.088
G1 X72.849 Y122.626 E.19392
G1 X72.316 Y122.626
M73 P96 R0
G1 X68.389 Y126.553 E.17063
G1 X68.392 Y126.017
G1 X71.783 Y122.626 E.14735
G1 X71.249 Y122.626
G1 X68.394 Y125.481 E.12406
G1 X68.397 Y124.945
G1 X70.716 Y122.626 E.10077
G1 X70.183 Y122.626
G1 X68.399 Y124.409 E.07748
G1 X68.402 Y123.874
G1 X69.649 Y122.626 E.0542
G1 X69.116 Y122.627
G1 X68.405 Y123.338 E.03091
; WIPE_START
M204 S6000
G1 X69.116 Y122.627 E-.38226
G1 X69.649 Y122.626 E-.20267
G1 X69.323 Y122.952 E-.17507
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X76.886 Y123.985 Z.8 F42000
G1 X127.211 Y130.855 Z.8
G1 Z.4
G1 E.8 F1800
; FEATURE: Gap infill
; LINE_WIDTH: 0.0967262
G1 F15000
M204 S6000
G3 X127.068 Y130.834 I-.053 J-.134 E.00066
; WIPE_START
G1 X127.104 Y130.855 E-.21425
G1 X127.211 Y130.855 E-.54575
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X126.281 Y125.062 Z.8 F42000
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.20771
G1 F15000
M204 S6000
G1 X126.068 Y125.24 E.00373
; LINE_WIDTH: 0.153358
G2 X125.461 Y125.814 I6.073 J7.045 E.00752
; LINE_WIDTH: 0.167357
G1 X125.282 Y126.027 E.00282
; LINE_WIDTH: 0.207719
G1 X125.103 Y126.24 E.00373
; WIPE_START
G1 X125.282 Y126.027 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X130.583 Y129.537 Z.8 F42000
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.217712
G1 F15000
M204 S6000
G1 X130.445 Y129.702 E.00306
; LINE_WIDTH: 0.177245
G3 X129.908 Y130.265 I-7.378 J-6.494 E.00851
; LINE_WIDTH: 0.18852
G1 X129.744 Y130.403 E.00255
; LINE_WIDTH: 0.217723
G1 X129.579 Y130.541 E.00306
; WIPE_START
G1 X129.744 Y130.403 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X127.176 Y123.215 Z.8 F42000
G1 X126.673 Y121.806 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.158982
G1 F15000
M204 S6000
G1 X126.415 Y122.019 E.00316
; LINE_WIDTH: 0.206776
G1 X126.157 Y122.232 E.00446
; WIPE_START
G1 X126.415 Y122.019 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X134.031 Y122.514 Z.8 F42000
G1 X136.023 Y122.643 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.131881
G1 F15000
M204 S6000
G1 X135.808 Y122.643 E.00156
; WIPE_START
G1 X136.023 Y122.643 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X143.655 Y122.638 Z.8 F42000
G1 X149.364 Y122.634 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.107616
G1 F15000
M204 S6000
G1 X149.125 Y122.634 E.00126
; WIPE_START
G1 X149.364 Y122.634 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X156.483 Y125.385 Z.8 F42000
G1 X187.46 Y137.357 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.0916832
G1 F15000
M204 S6000
G3 X187.387 Y137.43 I-.177 J-.104 E.00042
; WIPE_START
G1 X187.46 Y137.357 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X179.829 Y137.21 Z.8 F42000
G1 X176.523 Y137.146 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.219592
G1 F15000
M204 S6000
G3 X176.212 Y137.493 I-.916 J-.508 E.00676
; WIPE_START
G1 X176.388 Y137.335 E-.38417
G1 X176.523 Y137.146 E-.37583
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X168.892 Y136.988 Z.8 F42000
G1 X165.523 Y136.918 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.350481
G1 F15000
M204 S6000
G1 X165.333 Y137.29 E.01047
G1 X165.106 Y137.493 E.00761
; WIPE_START
G1 X165.333 Y137.29 E-.31989
G1 X165.523 Y136.918 E-.44011
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X157.893 Y137.145 Z.8 F42000
G1 X154.504 Y137.245 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.119639
G1 F15000
M204 S6000
G3 X154.299 Y137.479 I-.582 J-.303 E.00196
; WIPE_START
G1 X154.427 Y137.365 E-.41415
G1 X154.504 Y137.245 E-.34585
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X149.276 Y131.685 Z.8 F42000
G1 X145.226 Y127.377 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.102568
G1 F15000
M204 S6000
G1 X145.11 Y127.463 E.0007
G1 X145.131 Y127.493 E.00018
; WIPE_START
G1 X145.11 Y127.463 E-.15297
G1 X145.226 Y127.377 E-.60703
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X143.615 Y127.365 Z.8 F42000
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.112575
G1 F15000
M204 S6000
G1 X143.395 Y127.585 E.00177
; WIPE_START
G1 X143.615 Y127.365 E-.76
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X143.543 Y134.997 Z.8 F42000
G1 X143.523 Y137.107 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.260772
G1 F15000
M204 S6000
G3 X143.121 Y137.493 I-.744 J-.373 E.01007
; WIPE_START
G1 X143.371 Y137.321 E-.40772
G1 X143.523 Y137.107 E-.35228
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X135.891 Y137.189 Z.8 F42000
G1 X123.467 Y137.323 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.0913925
G1 F15000
M204 S6000
G1 X123.438 Y137.331 E.00012
; LINE_WIDTH: 0.134498
G1 X123.409 Y137.338 E.00022
G2 X123.427 Y137.372 I.046 J-.003 E.00029
G1 X123.549 Y137.481 E.00122
; WIPE_START
G1 X123.427 Y137.372 E-.53687
G1 X123.409 Y137.338 E-.12512
G1 X123.438 Y137.331 E-.09801
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X121.523 Y137.192 Z.8 F42000
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.162326
G1 F15000
M204 S6000
G3 X121.214 Y137.493 I-.627 J-.334 E.00426
; WIPE_START
G1 X121.409 Y137.352 E-.41928
G1 X121.523 Y137.192 E-.34072
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X113.891 Y137.047 Z.8 F42000
G1 X110.523 Y136.983 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.278439
G1 F15000
M204 S6000
G1 X110.393 Y137.251 E.00571
; LINE_WIDTH: 0.30679
G3 X110.304 Y137.352 I-.265 J-.146 E.00292
G1 X110.079 Y137.493 E.00571
; WIPE_START
G1 X110.304 Y137.352 E-.50433
G1 X110.393 Y137.251 E-.25567
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X102.761 Y137.232 Z.8 F42000
G1 X90.408 Y137.202 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.157486
G1 F15000
M204 S6000
G2 X90.38 Y137.297 I.265 J.13 E.00093
; LINE_WIDTH: 0.10615
G2 X90.517 Y137.457 I.262 J-.085 E.00112
; WIPE_START
G1 X90.407 Y137.372 E-.48181
G1 X90.38 Y137.297 E-.27819
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X88.523 Y137.189 Z.8 F42000
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.218878
G1 F15000
M204 S6000
G3 X88.207 Y137.493 I-1.045 J-.767 E.00631
; WIPE_START
G1 X88.357 Y137.374 E-.32982
G1 X88.523 Y137.189 E-.43018
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X80.89 Y137.115 Z.8 F42000
G1 X77.523 Y137.083 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.341302
G1 F15000
M204 S6000
G1 X77.323 Y137.306 E.00728
G1 X76.959 Y137.493 E.00994
; WIPE_START
G1 X77.323 Y137.306 E-.43869
M73 P97 R0
G1 X77.523 Y137.083 E-.32131
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X69.898 Y137.434 Z.8 F42000
G1 X68.631 Y137.493 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.155843
G1 F15000
M204 S6000
G1 X68.533 Y137.424 E.0011
G1 X68.508 Y137.428 E.00024
; LINE_WIDTH: 0.107507
G3 X68.356 Y137.303 I.028 J-.189 E.00109
; WIPE_START
G1 X68.429 Y137.399 E-.44879
G1 X68.508 Y137.428 E-.31121
; WIPE_END
G1 E-.04 F1800
M204 S10000
G1 X68.623 Y129.796 Z.8 F42000
G1 X68.732 Y122.608 Z.8
G1 Z.4
G1 E.8 F1800
; LINE_WIDTH: 0.308465
G1 F15000
M204 S6000
G1 X68.537 Y122.746 E.00518
G1 X68.387 Y123.045 E.00723
; close powerlost recovery
M1003 S0
; WIPE_START
G1 F15000
G1 X68.537 Y122.746 E-.44303
G1 X68.732 Y122.608 E-.31697
; WIPE_END
G1 E-.04 F1800
M204 S10000
G17
G3 Z.8 I1.217 J0 P1  F42000
M106 S0
M981 S0 P20000 ; close spaghetti detector
; FEATURE: Custom
; MACHINE_END_GCODE_START
; filament end gcode 

;===== date: 20260513 =====================
G392 S0 ;turn off nozzle clog detect

M400 ; wait for buffer to clear
G92 E0 ; zero the extruder
G90
G1 Z0.8 F900 ; lower z a little
G1 X0 Y129.65 F18000 ; move to safe pos
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
G1 X267 F15000
T255
G1 X-28.5 F18000
G1 X-48.2 F3000
G1 X-28.5 F18000
G1 X-48.2 F3000
M621 S255

M104 S0 ; turn off hotend

M400 ; wait all motion done
M17 S
M17 Z0.4 ; lower z motor current to reduce impact if there is something in the bottom

    G1 Z100.4 F600
    G1 Z98.4

M400 P100
M17 R ; restore z current

G90
G1 X-48 Y180 F3600

M220 S100  ; Reset feedrate magnitude
M201.2 K1.0 ; Reset acc magnitude
M73.2   R1.0 ;Reset left time magnitude
M1002 set_gcode_claim_speed_level : 0

;=====printer finish  sound=========
M17
M400 S1
M1006 S1
M1006 A0 B20 L100 C37 D20 M40 E42 F20 N60
M1006 A0 B10 L100 C44 D10 M60 E44 F10 N60
M1006 A0 B10 L100 C46 D10 M80 E46 F10 N80
M1006 A44 B20 L100 C39 D20 M60 E48 F20 N60
M1006 A0 B10 L100 C44 D10 M60 E44 F10 N60
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N60
M1006 A0 B10 L100 C39 D10 M60 E39 F10 N60
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N60
M1006 A0 B10 L100 C44 D10 M60 E44 F10 N60
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N60
M1006 A0 B10 L100 C39 D10 M60 E39 F10 N60
M1006 A0 B10 L100 C0 D10 M60 E0 F10 N60
M1006 A0 B10 L100 C48 D10 M60 E44 F10 N80
M1006 A0 B10 L100 C0 D10 M60 E0 F10  N80
M1006 A44 B20 L100 C49 D20 M80 E41 F20 N80
M1006 A0 B20 L100 C0 D20 M60 E0 F20 N80
M1006 A0 B20 L100 C37 D20 M30 E37 F20 N60
M1006 W
;=====printer finish  sound=========

;M17 X0.8 Y0.8 Z0.5 ; lower motor current to 45% power
M400
M18 X Y Z

M73 P100 R0
; EXECUTABLE_BLOCK_END

