; HEADER_BLOCK_START
; BambuStudio 02.08.02.61
; model printing time: 3m 30s; total estimated time: 9m 47s
; total layer number: 2
; total filament length [mm] : 296.03
; total filament volume [cm^3] : 712.03
; total filament weight [g] : 0.89
; filament_density: 1.25
; filament_diameter: 1.75
; max_z_height: 0.40
; filament: 1
; support_material_on_wipe_tower: 0
; HEADER_BLOCK_END

; CONFIG_BLOCK_START
; accel_to_decel_enable = 0
; accel_to_decel_factor = 50%
; activate_air_filtration = 0
; additional_cooling_fan_speed = 0
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
; different_settings_to_system = post_process;;
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
; enable_prime_tower = 0
; enable_support = 0
; enable_support_ironing = 0
; enable_tower_interface_features = 0
; enable_wrapping_detection = 0
; enforce_support_layers = 0
; eng_plate_temp = 70
; eng_plate_temp_initial_layer = 70
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
; fan_cooling_layer_time = 30
; fan_direction = undefine
; fan_max_speed = 50
; fan_min_speed = 30
; farthest_point_timelapse = 0
; filament_adaptive_volumetric_speed = 0
; filament_adhesiveness_category = 300
; filament_bridge_speed = 25
; filament_change_length = 10
; filament_change_length_nc = 10
; filament_colour = #FFFFFF
; filament_colour_type = 1
; filament_cooling_before_tower = 0
; filament_cost = 17.99
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
; filament_settings_id = "Bambu PETG Basic @BBL A1"
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
; head_wrap_detect_zone = 226x224,256x224,256x256,226x256
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
; post_process = "\"G:\\project\\MKPSupportEase\\target\\debug\\mkp-support-ease.exe\" --Toml \"C:\\Users\\WZY\\AppData\\Roaming\\SupportEase\\user\\presets-local/A1-fast261111.toml\" --Gcode"
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
; temperature_vitrification = 60
; template_custom_gcode = 
; textured_plate_temp = 70
; textured_plate_temp_initial_layer = 70
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
; wipe_tower_x = 15,15
; wipe_tower_y = 216.972,216.972
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
M1002 set_filament_type:PETG
M104 S140
M140 S70

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
M104 S245

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
    M109 S245

    M104 S250

    M400
    T0
    G1 X-48.2 F3000
    M400


    M620.1 E F324.284 T270
    M109 S250 ;set nozzle to common flush temp

    M106 P1 S0
    G92 E0
    G1 E50 F200
    M400
    M1002 set_filament_type:PETG
M621 S0A


    M109 S270 H300

G92 E0
G1 E50 F200 ; lower extrusion speed to avoid clog
M400
M106 P1 S178
G92 E0
G1 E5 F200
M104 S245
G92 E0
M73 P5 R9
G1 E-0.5 F300

G1 X-28.5 F30000
M73 P8 R8
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

M1002 set_filament_type:PETG

;M1002 set_flag extrude_cali_flag=1
M1002 judge_flag extrude_cali_flag

M622 J1
    M1002 gcode_claim_action : 8

    M109 S245
    G1 E10 F325
    M983 F5.41667 A0.3 H0.4; cali dynamic extrusion compensation

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
        M983 F5.41667 A0.3 H0.4; cali dynamic extrusion compensation
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
    M984 A0.1 E1 S1 F5.41667 H0.4
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

M73 P16 R8
G1 X128 Y128 F20000
G1 Z5 F1200
M400 P200
M970.3 Q0 A10 K0 O1
M974 Q0 S2 P0

M970.2 Q0 K1 W78 Z0.1
M974 S2

M975 S1
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
M109 S195

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
M73 P61 R3
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
M73 P62 R3
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

M190 S70; ensure bed temp
M109 S140
M106 S0 ; turn off fan , too noisy

M622 J1
    M1002 gcode_claim_action : 1
    G29 A1 X59.8904 Y107.991 I136.219 J40.0187
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

M104 S245 ; prepare to print

;===== nozzle load line ===============================
;G90
;M83
;G1 Z5 F1200
;G1 X88 Y-0.5 F20000
;G1 Z0.3 F1200

;M109 S245

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

    M109 S245
    G0 X128 E8  F780
    G0 X133 E.3742  F1300
    G0 X138 E.3742  F5200
    G0 X143 E.3742  F1300
    G0 X148 E.3742  F5200
    G0 X153 E.3742  F1300
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
    G0 X128 E10  F780
    G0 X133 E.3742  F1300
    G0 X138 E.3742  F5200
    G0 X143 E.3742  F1300
    G0 X148 E.3742  F5200
    G0 X153 E.3742  F1300
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
; layer num/total_layer_count: 1/2
; update layer progress
M73 L1
M991 S0 P0 ;notify layer change
M106 S0
; CUSTOM_GCODE
;MKPCalibration Precise Calibration
; OBJECT_ID: 121
G1 X132.139 Y133.109 F42000
M204 S6000
M73 P63 R3
G1 Z.4
G1 Z.2
G1 E.4 F1800
; FEATURE: Inner wall
; LINE_WIDTH: 0.5
G1 F3000
M204 S500
G1 X132.139 Y143.109 E.35726
G1 X123.703 Y143.109 E.30138
G1 X123.703 Y133.109 E.35726
G1 X121.139 Y133.109 E.09161
G1 X121.139 Y143.109 E.35726
G1 X112.703 Y143.109 E.30138
G1 X112.703 Y133.109 E.35726
G1 X110.139 Y133.109 E.09161
G1 X110.139 Y143.109 E.35726
G1 X101.703 Y143.109 E.30138
G1 X101.703 Y133.109 E.35726
G1 X99.139 Y133.109 E.09161
G1 X99.139 Y143.109 E.35726
G1 X90.703 Y143.109 E.30138
G1 X90.703 Y133.109 E.35726
G1 X88.139 Y133.109 E.09161
G1 X88.139 Y143.109 E.35726
M73 P64 R3
G1 X79.703 Y143.109 E.30138
G1 X79.703 Y133.109 E.35726
G1 X77.139 Y133.109 E.09161
G1 X77.139 Y143.109 E.35726
G1 X68.704 Y143.109 E.30136
G1 X68.771 Y128.982 E.50473
G1 X124.581 Y128.982 E1.99388
G1 X124.86 Y129.579 E.02355
G1 X125.208 Y130.077 E.02171
G1 X125.638 Y130.506 E.0217
G1 X126.136 Y130.855 E.0217
G1 X126.686 Y131.111 E.0217
G1 X127.273 Y131.269 E.0217
G1 X127.878 Y131.322 E.0217
G1 X128.483 Y131.269 E.0217
G1 X129.07 Y131.111 E.0217
G1 X129.621 Y130.855 E.0217
G1 X130.118 Y130.506 E.0217
G1 X130.548 Y130.077 E.02171
G1 X130.896 Y129.579 E.0217
G1 X131.175 Y128.982 E.02355
G1 X187.139 Y128.982 E1.99937
G1 X187.139 Y143.109 E.50473
G1 X178.703 Y143.109 E.30138
G1 X178.703 Y133.109 E.35726
G1 X176.139 Y133.109 E.09161
G1 X176.139 Y143.109 E.35726
G1 X167.703 Y143.109 E.30138
G1 X167.703 Y133.109 E.35726
G1 X165.139 Y133.109 E.09161
G1 X165.139 Y143.109 E.35726
G1 X156.703 Y143.109 E.30138
M73 P65 R3
G1 X156.703 Y133.109 E.35726
G1 X154.139 Y133.109 E.09161
G1 X154.139 Y143.109 E.35726
G1 X145.703 Y143.109 E.30138
G1 X145.703 Y133.109 E.35726
G1 X143.139 Y133.109 E.09161
G1 X143.139 Y143.109 E.35726
G1 X134.703 Y143.109 E.30138
G1 X134.703 Y133.109 E.35726
G1 X132.199 Y133.109 E.08946
M204 S6000
G1 X132.596 Y133.566 F42000
; FEATURE: Outer wall
G1 F3000
M204 S500
G1 X132.596 Y143.223 E.34499
G3 X132.253 Y143.566 I-.335 J.008 E.0194
G1 X123.589 Y143.566 E.30951
G3 X123.246 Y143.223 I-.008 J-.335 E.0194
G1 X123.246 Y133.566 E.34499
G1 X121.596 Y133.566 E.05895
G1 X121.596 Y143.223 E.345
G3 X121.253 Y143.566 I-.335 J.008 E.01939
G1 X112.589 Y143.566 E.30951
G3 X112.246 Y143.223 I-.008 J-.335 E.0194
G1 X112.246 Y133.566 E.34499
G1 X110.596 Y133.566 E.05895
G1 X110.596 Y143.223 E.345
G3 X110.253 Y143.566 I-.335 J.008 E.01939
G1 X101.589 Y143.566 E.30951
G3 X101.246 Y143.223 I-.008 J-.335 E.0194
G1 X101.246 Y133.566 E.34499
G1 X99.596 Y133.566 E.05895
M73 P66 R3
G1 X99.596 Y143.223 E.345
G3 X99.253 Y143.566 I-.335 J.008 E.01939
G1 X90.589 Y143.566 E.30951
G3 X90.246 Y143.223 I-.008 J-.335 E.0194
G1 X90.246 Y133.566 E.34499
G1 X88.596 Y133.566 E.05895
G1 X88.596 Y143.223 E.345
G3 X88.253 Y143.566 I-.335 J.008 E.01939
G1 X79.589 Y143.566 E.30951
G3 X79.246 Y143.223 I-.008 J-.335 E.0194
G1 X79.246 Y133.566 E.34499
G1 X77.596 Y133.566 E.05895
G1 X77.596 Y143.223 E.345
G3 X77.253 Y143.566 I-.335 J.008 E.0194
G1 X68.589 Y143.566 E.30951
G3 X68.246 Y143.224 I-.008 J-.335 E.01937
G1 X68.315 Y128.768 E.51647
G3 X68.559 Y128.524 I.235 J-.008 E.01382
G1 X124.941 Y128.524 E2.01431
G1 X125.034 Y128.871 E.01283
G2 X130.815 Y128.524 I2.839 J-1.033 E.27666
G1 X187.354 Y128.524 E2.01993
G3 X187.596 Y128.766 I.009 J.232 E.01372
G1 X187.596 Y143.223 E.51649
G3 X187.253 Y143.566 I-.335 J.008 E.0194
G1 X178.589 Y143.566 E.30951
G3 X178.246 Y143.223 I-.008 J-.335 E.01939
G1 X178.246 Y133.566 E.345
G1 X176.596 Y133.566 E.05895
G1 X176.596 Y143.223 E.345
G3 X176.253 Y143.566 I-.335 J.008 E.01939
G1 X167.589 Y143.566 E.30951
G3 X167.246 Y143.223 I-.008 J-.335 E.01939
G1 X167.246 Y133.566 E.345
G1 X165.596 Y133.566 E.05895
G1 X165.596 Y143.223 E.34499
G3 X165.253 Y143.566 I-.335 J.008 E.0194
G1 X156.589 Y143.566 E.30951
G3 X156.246 Y143.223 I-.008 J-.335 E.01939
G1 X156.246 Y133.566 E.345
G1 X154.596 Y133.566 E.05895
G1 X154.596 Y143.223 E.34499
M73 P67 R3
G3 X154.253 Y143.566 I-.335 J.008 E.0194
G1 X145.589 Y143.566 E.30951
G3 X145.246 Y143.223 I-.008 J-.335 E.01939
G1 X145.246 Y133.566 E.345
G1 X143.596 Y133.566 E.05895
G1 X143.596 Y143.223 E.34499
G3 X143.253 Y143.566 I-.335 J.008 E.0194
G1 X134.589 Y143.566 E.30951
G3 X134.246 Y143.223 I-.008 J-.335 E.01939
G1 X134.246 Y133.566 E.345
G1 X132.656 Y133.566 E.0568
; WIPE_START
G1 X132.65 Y134.566 E-.38
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
G1 X0 Y136.027 F18000 ; move to safe pos
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


G1 X186.956 Y130.089 F42000
G1 Z.2
G1 E.4 F1800
; FEATURE: Bottom surface
; LINE_WIDTH: 0.50019
G1 F6300
M204 S500
G1 X186.237 Y129.37 E.03633
G1 X185.591 Y129.37 E.02311
G1 X186.75 Y130.53 E.05862
G1 X186.75 Y131.176 E.02311
G1 X184.944 Y129.37 E.0913
G1 X184.297 Y129.37 E.02311
G1 X186.75 Y131.823 E.12399
G1 X186.75 Y132.47 E.02311
M73 P68 R3
G1 X183.651 Y129.37 E.15668
G1 X183.004 Y129.37 E.02311
G1 X186.75 Y133.116 E.18936
G1 X186.75 Y133.763 E.02311
G1 X182.357 Y129.37 E.22205
G1 X181.711 Y129.37 E.02311
G1 X186.75 Y134.41 E.25473
G1 X186.75 Y135.056 E.02311
G1 X181.064 Y129.37 E.28742
G1 X180.417 Y129.37 E.02311
G1 X186.75 Y135.703 E.32011
G1 X186.75 Y136.35 E.02311
G1 X179.771 Y129.37 E.35279
G1 X179.124 Y129.37 E.02311
G1 X186.75 Y136.997 E.38548
G1 X186.75 Y137.643 E.02311
G1 X178.477 Y129.37 E.41817
G1 X177.831 Y129.37 E.02311
G1 X186.75 Y138.29 E.45085
G1 X186.75 Y138.937 E.02311
G1 X177.184 Y129.37 E.48354
G1 X176.537 Y129.37 E.02311
G1 X186.75 Y139.583 E.51623
G1 X186.75 Y140.23 E.02311
G1 X175.891 Y129.37 E.54891
G1 X175.244 Y129.37 E.02311
G1 X178.595 Y132.721 E.16936
G1 X179.092 Y132.721 E.01776
G1 X179.092 Y133.218 E.01776
G1 X186.75 Y140.877 E.38712
G1 X186.75 Y141.523 E.02311
G1 X179.092 Y133.864 E.38712
G1 X179.092 Y134.511 E.02311
G1 X186.75 Y142.17 E.38712
G1 X186.75 Y142.721 E.01969
G1 X186.655 Y142.721 E.00343
G1 X179.092 Y135.158 E.38227
G1 X179.092 Y135.804 E.02311
G1 X186.008 Y142.721 E.34959
G1 X185.361 Y142.721 E.02311
G1 X179.092 Y136.451 E.3169
G1 X179.092 Y137.098 E.02311
M73 P69 R3
G1 X184.715 Y142.721 E.28421
G1 X184.068 Y142.721 E.02311
G1 X179.092 Y137.744 E.25153
G1 X179.092 Y138.391 E.02311
G1 X183.421 Y142.721 E.21884
G1 X182.775 Y142.721 E.02311
G1 X179.092 Y139.038 E.18615
G1 X179.092 Y139.685 E.02311
G1 X182.128 Y142.721 E.15347
G1 X181.481 Y142.721 E.02311
G1 X179.092 Y140.331 E.12078
G1 X179.092 Y140.978 E.02311
G1 X180.835 Y142.721 E.0881
G1 X180.188 Y142.721 E.02311
G1 X179.092 Y141.625 E.05541
G1 X179.092 Y142.271 E.02311
G1 X179.747 Y142.926 E.03312
; WIPE_START
M73 P69 R2
G1 X179.092 Y142.271 E-.35212
G1 X179.092 Y142.198 E-.02788
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.211 J-.122 P1  F42000
G1 X178.154 Y132.926 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X174.597 Y129.37 E.17976
G1 X173.951 Y129.37 E.02311
G1 X177.301 Y132.721 E.16936
G1 X176.655 Y132.721 E.02311
G1 X173.304 Y129.37 E.16936
G1 X172.657 Y129.37 E.02311
G1 X176.008 Y132.721 E.16936
G1 X175.75 Y132.721 E.0092
G1 X175.75 Y133.11 E.01391
G1 X172.011 Y129.37 E.18903
G1 X171.364 Y129.37 E.02311
G1 X175.75 Y133.757 E.22172
G1 X175.75 Y134.403 E.02311
G1 X170.717 Y129.37 E.25441
G1 X170.071 Y129.37 E.02311
G1 X175.75 Y135.05 E.28709
G1 X175.75 Y135.697 E.02311
G1 X169.424 Y129.37 E.31978
G1 X168.777 Y129.37 E.02311
G1 X175.75 Y136.343 E.35247
G1 X175.75 Y136.99 E.02311
G1 X168.13 Y129.37 E.38515
G1 X167.484 Y129.37 E.02311
G1 X175.75 Y137.637 E.41784
G1 X175.75 Y138.283 E.02311
G1 X166.837 Y129.37 E.45053
G1 X166.19 Y129.37 E.02311
G1 X175.75 Y138.93 E.48321
M73 P70 R2
G1 X175.75 Y139.577 E.02311
G1 X165.544 Y129.37 E.5159
G1 X164.897 Y129.37 E.02311
G1 X175.75 Y140.223 E.54859
G1 X175.75 Y140.87 E.02311
G1 X168.092 Y133.211 E.38712
G1 X168.092 Y132.721 E.01753
G1 X167.601 Y132.721 E.01753
G1 X164.25 Y129.37 E.16936
G1 X163.604 Y129.37 E.02311
G1 X166.954 Y132.721 E.16936
G1 X166.308 Y132.721 E.02311
G1 X162.957 Y129.37 E.16936
G1 X162.31 Y129.37 E.02311
G1 X165.661 Y132.721 E.16936
G1 X165.014 Y132.721 E.02311
G1 X161.664 Y129.37 E.16936
G1 X161.017 Y129.37 E.02311
G1 X164.75 Y133.104 E.18871
G1 X164.75 Y133.75 E.02311
G1 X160.37 Y129.37 E.22139
G1 X159.724 Y129.37 E.02311
G1 X164.75 Y134.397 E.25408
G1 X164.75 Y135.044 E.02311
G1 X159.077 Y129.37 E.28677
G1 X158.43 Y129.37 E.02311
G1 X164.75 Y135.69 E.31945
G1 X164.75 Y136.337 E.02311
G1 X157.784 Y129.37 E.35214
G1 X157.137 Y129.37 E.02311
G1 X164.75 Y136.984 E.38483
G1 X164.75 Y137.63 E.02311
G1 X156.49 Y129.37 E.41751
G1 X155.844 Y129.37 E.02311
G1 X164.75 Y138.277 E.4502
G1 X164.75 Y138.924 E.02311
G1 X155.197 Y129.37 E.48288
G1 X154.55 Y129.37 E.02311
G1 X164.75 Y139.57 E.51557
G1 X164.75 Y140.217 E.02311
G1 X153.904 Y129.37 E.54826
G1 X153.257 Y129.37 E.02311
G1 X156.608 Y132.721 E.16936
G1 X157.092 Y132.721 E.0173
G1 X157.092 Y133.205 E.0173
G1 X164.75 Y140.864 E.38712
G1 X164.75 Y141.51 E.02311
M73 P71 R2
G1 X157.092 Y133.851 E.38712
G1 X157.092 Y134.498 E.02311
G1 X164.75 Y142.157 E.38712
G1 X164.75 Y142.721 E.02015
G1 X164.668 Y142.721 E.00296
G1 X157.092 Y135.145 E.38293
G1 X157.092 Y135.792 E.02311
G1 X164.021 Y142.721 E.35024
G1 X163.374 Y142.721 E.02311
G1 X157.092 Y136.438 E.31756
G1 X157.092 Y137.085 E.02311
G1 X162.728 Y142.721 E.28487
G1 X162.081 Y142.721 E.02311
G1 X157.092 Y137.732 E.25218
G1 X157.092 Y138.378 E.02311
G1 X161.434 Y142.721 E.2195
G1 X160.788 Y142.721 E.02311
G1 X157.092 Y139.025 E.18681
G1 X157.092 Y139.672 E.02311
G1 X160.141 Y142.721 E.15412
G1 X159.494 Y142.721 E.02311
G1 X157.092 Y140.318 E.12144
G1 X157.092 Y140.965 E.02311
G1 X158.847 Y142.721 E.08875
G1 X158.201 Y142.721 E.02311
G1 X157.092 Y141.612 E.05606
G1 X157.092 Y142.258 E.02311
G1 X157.76 Y142.926 E.03377
; WIPE_START
G1 X157.092 Y142.258 E-.35908
G1 X157.092 Y142.203 E-.02092
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I.756 J.954 P1  F42000
G1 X167.886 Y133.652 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X175.75 Y141.517 E.39751
G1 X175.75 Y142.163 E.02311
G1 X168.092 Y134.505 E.38712
G1 X168.092 Y135.151 E.02311
G1 X175.661 Y142.721 E.3826
G1 X175.014 Y142.721 E.02311
G1 X168.092 Y135.798 E.34992
M73 P72 R2
G1 X168.092 Y136.445 E.02311
G1 X174.368 Y142.721 E.31723
G1 X173.721 Y142.721 E.02311
G1 X168.092 Y137.091 E.28454
G1 X168.092 Y137.738 E.02311
G1 X173.074 Y142.721 E.25186
G1 X172.428 Y142.721 E.02311
G1 X168.092 Y138.385 E.21917
G1 X168.092 Y139.031 E.02311
G1 X171.781 Y142.721 E.18648
G1 X171.134 Y142.721 E.02311
G1 X168.092 Y139.678 E.1538
G1 X168.092 Y140.325 E.02311
G1 X170.488 Y142.721 E.12111
G1 X169.841 Y142.721 E.02311
G1 X168.092 Y140.971 E.08842
G1 X168.092 Y141.618 E.02311
G1 X169.194 Y142.721 E.05574
G1 X168.548 Y142.721 E.02311
G1 X167.886 Y142.059 E.03345
; WIPE_START
G1 X168.548 Y142.721 E-.3556
G1 X168.612 Y142.721 E-.0244
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I.753 J-.956 P1  F42000
G1 X156.167 Y132.926 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X152.61 Y129.37 E.17976
G1 X151.964 Y129.37 E.02311
G1 X155.314 Y132.721 E.16936
G1 X154.668 Y132.721 E.02311
G1 X151.317 Y129.37 E.16936
G1 X150.67 Y129.37 E.02311
G1 X154.021 Y132.721 E.16936
G1 X153.75 Y132.721 E.00967
G1 X153.75 Y133.097 E.01345
G1 X150.024 Y129.37 E.18838
G1 X149.377 Y129.37 E.02311
G1 X153.75 Y133.744 E.22107
G1 X153.75 Y134.39 E.02311
G1 X148.73 Y129.37 E.25375
G1 X148.083 Y129.37 E.02311
G1 X153.75 Y135.037 E.28644
G1 X153.75 Y135.684 E.02311
G1 X147.437 Y129.37 E.31913
G1 X146.79 Y129.37 E.02311
G1 X153.75 Y136.33 E.35181
G1 X153.75 Y136.977 E.02311
G1 X146.143 Y129.37 E.3845
G1 X145.497 Y129.37 E.02311
G1 X153.75 Y137.624 E.41718
G1 X153.75 Y138.27 E.02311
G1 X144.85 Y129.37 E.44987
G1 X144.203 Y129.37 E.02311
G1 X153.75 Y138.917 E.48256
G1 X153.75 Y139.564 E.02311
G1 X143.557 Y129.37 E.51524
G1 X142.91 Y129.37 E.02311
G1 X153.75 Y140.21 E.54793
G1 X153.75 Y140.857 E.02311
G1 X146.092 Y133.198 E.38712
G1 X146.092 Y132.721 E.01707
G1 X145.614 Y132.721 E.01707
G1 X142.263 Y129.37 E.16936
M73 P73 R2
G1 X141.617 Y129.37 E.02311
G1 X144.967 Y132.721 E.16936
G1 X144.321 Y132.721 E.02311
G1 X140.97 Y129.37 E.16936
G1 X140.323 Y129.37 E.02311
G1 X143.674 Y132.721 E.16936
G1 X143.027 Y132.721 E.02311
G1 X139.677 Y129.37 E.16936
G1 X139.03 Y129.37 E.02311
G1 X142.75 Y133.091 E.18805
G1 X142.75 Y133.737 E.02311
G1 X138.383 Y129.37 E.22074
G1 X137.737 Y129.37 E.02311
G1 X142.75 Y134.384 E.25342
G1 X142.75 Y135.031 E.02311
G1 X137.09 Y129.37 E.28611
G1 X136.443 Y129.37 E.02311
G1 X142.75 Y135.677 E.3188
G1 X142.75 Y136.324 E.02311
G1 X135.797 Y129.37 E.35148
G1 X135.15 Y129.37 E.02311
G1 X142.75 Y136.971 E.38417
G1 X142.75 Y137.617 E.02311
G1 X134.503 Y129.37 E.41686
G1 X133.857 Y129.37 E.02311
G1 X142.75 Y138.264 E.44954
G1 X142.75 Y138.911 E.02311
G1 X133.21 Y129.37 E.48223
G1 X132.563 Y129.37 E.02311
G1 X142.75 Y139.557 E.51492
G1 X142.75 Y140.204 E.02311
G1 X131.917 Y129.37 E.5476
G1 X131.422 Y129.37 E.01766
G1 X131.374 Y129.474 E.0041
G1 X134.621 Y132.721 E.16411
G1 X135.092 Y132.721 E.01684
G1 X135.092 Y133.192 E.01684
G1 X142.75 Y140.851 E.38712
G1 X142.75 Y141.497 E.02311
G1 X135.092 Y133.839 E.38712
G1 X135.092 Y134.485 E.02311
G1 X142.75 Y142.144 E.38712
G1 X142.75 Y142.721 E.02061
G1 X142.681 Y142.721 E.0025
G1 X135.092 Y135.132 E.38358
M73 P74 R2
G1 X135.092 Y135.779 E.02311
G1 X142.034 Y142.721 E.3509
G1 X141.387 Y142.721 E.02311
G1 X135.092 Y136.425 E.31821
G1 X135.092 Y137.072 E.02311
G1 X140.741 Y142.721 E.28552
G1 X140.094 Y142.721 E.02311
G1 X135.092 Y137.719 E.25284
G1 X135.092 Y138.365 E.02311
G1 X139.447 Y142.721 E.22015
G1 X138.8 Y142.721 E.02311
G1 X135.092 Y139.012 E.18746
G1 X135.092 Y139.659 E.02311
G1 X138.154 Y142.721 E.15478
G1 X137.507 Y142.721 E.02311
G1 X135.092 Y140.305 E.12209
G1 X135.092 Y140.952 E.02311
G1 X136.86 Y142.721 E.08941
G1 X136.214 Y142.721 E.02311
G1 X135.092 Y141.599 E.05672
G1 X135.092 Y142.245 E.02311
G1 X135.773 Y142.926 E.03443
; WIPE_START
G1 X135.092 Y142.245 E-.36605
G1 X135.092 Y142.209 E-.01395
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.211 J-.119 P1  F42000
G1 X134.18 Y132.926 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X131.149 Y129.896 E.15319
G1 X130.882 Y130.276 E.0166
G1 X133.327 Y132.721 E.12357
G1 X132.681 Y132.721 E.02311
G1 X130.567 Y130.607 E.10684
G3 X130.221 Y130.908 I-1.027 J-.829 E.01646
G1 X132.034 Y132.721 E.09162
G1 X131.75 Y132.721 E.01013
G1 X131.75 Y133.084 E.01298
G1 X129.841 Y131.175 E.09652
G1 X129.404 Y131.384 E.01732
G1 X131.75 Y133.731 E.1186
G1 X131.75 Y134.377 E.02311
G1 X128.925 Y131.552 E.1428
G1 X128.551 Y131.653 E.01385
G1 X128.393 Y131.666 E.00567
G1 X131.75 Y135.024 E.16971
M73 P75 R2
G1 X131.75 Y135.671 E.02311
G1 X127.783 Y131.703 E.20054
G1 X127.205 Y131.653 E.02072
G1 X127.042 Y131.609 E.00604
G1 X131.75 Y136.317 E.238
G1 X131.75 Y136.964 E.02311
G1 X124.156 Y129.37 E.38384
G1 X123.51 Y129.37 E.02311
G1 X131.75 Y137.611 E.41653
G1 X131.75 Y138.257 E.02311
G1 X122.863 Y129.37 E.44922
G1 X122.216 Y129.37 E.02311
G1 X131.75 Y138.904 E.4819
G1 X131.75 Y139.551 E.02311
G1 X121.57 Y129.37 E.51459
G1 X120.923 Y129.37 E.02311
G1 X131.75 Y140.198 E.54728
G1 X131.75 Y140.844 E.02311
G1 X124.092 Y133.185 E.38712
G1 X124.092 Y132.721 E.0166
G1 X123.627 Y132.721 E.0166
G1 X120.276 Y129.37 E.16936
G1 X119.63 Y129.37 E.02311
G1 X122.98 Y132.721 E.16936
G1 X122.334 Y132.721 E.02311
G1 X118.983 Y129.37 E.16936
G1 X118.336 Y129.37 E.02311
G1 X121.687 Y132.721 E.16936
G1 X121.04 Y132.721 E.02311
G1 X117.69 Y129.37 E.16936
G1 X117.043 Y129.37 E.02311
G1 X120.75 Y133.078 E.1874
G1 X120.75 Y133.724 E.02311
G1 X116.396 Y129.37 E.22008
G1 X115.75 Y129.37 E.02311
G1 X120.75 Y134.371 E.25277
G1 X120.75 Y135.018 E.02311
G1 X115.103 Y129.37 E.28546
G1 X114.456 Y129.37 E.02311
G1 X120.75 Y135.664 E.31814
G1 X120.75 Y136.311 E.02311
G1 X113.81 Y129.37 E.35083
G1 X113.163 Y129.37 E.02311
G1 X120.75 Y136.958 E.38352
G1 X120.75 Y137.604 E.02311
G1 X112.516 Y129.37 E.4162
G1 X111.87 Y129.37 E.02311
G1 X120.75 Y138.251 E.44889
G1 X120.75 Y138.898 E.02311
G1 X111.223 Y129.37 E.48157
G1 X110.576 Y129.37 E.02311
G1 X120.75 Y139.544 E.51426
G1 X120.75 Y140.191 E.02311
G1 X109.93 Y129.37 E.54695
G1 X109.283 Y129.37 E.02311
G1 X112.634 Y132.721 E.16936
G1 X113.092 Y132.721 E.01637
G1 X113.092 Y133.179 E.01637
G1 X120.75 Y140.838 E.38712
M73 P76 R2
G1 X120.75 Y141.484 E.02311
G1 X113.092 Y133.826 E.38712
G1 X113.092 Y134.472 E.02311
G1 X120.75 Y142.131 E.38712
G1 X120.75 Y142.721 E.02108
G1 X120.694 Y142.721 E.00203
G1 X113.092 Y135.119 E.38424
G1 X113.092 Y135.766 E.02311
G1 X120.047 Y142.721 E.35155
G1 X119.4 Y142.721 E.02311
G1 X113.092 Y136.412 E.31887
G1 X113.092 Y137.059 E.02311
G1 X118.753 Y142.721 E.28618
G1 X118.107 Y142.721 E.02311
G1 X113.092 Y137.706 E.25349
G1 X113.092 Y138.352 E.02311
G1 X117.46 Y142.721 E.22081
G1 X116.813 Y142.721 E.02311
G1 X113.092 Y138.999 E.18812
G1 X113.092 Y139.646 E.02311
G1 X116.167 Y142.721 E.15543
G1 X115.52 Y142.721 E.02311
G1 X113.092 Y140.292 E.12275
G1 X113.092 Y140.939 E.02311
G1 X114.873 Y142.721 E.09006
G1 X114.227 Y142.721 E.02311
G1 X113.092 Y141.586 E.05738
G1 X113.092 Y142.232 E.02311
G1 X113.786 Y142.926 E.03509
; WIPE_START
G1 X113.092 Y142.232 E-.37303
G1 X113.092 Y142.214 E-.00697
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.211 J-.117 P1  F42000
G1 X112.193 Y132.926 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X108.636 Y129.37 E.17976
G1 X107.989 Y129.37 E.02311
G1 X111.34 Y132.721 E.16936
G1 X110.694 Y132.721 E.02311
G1 X107.343 Y129.37 E.16936
M73 P77 R2
G1 X106.696 Y129.37 E.02311
G1 X110.047 Y132.721 E.16936
G1 X109.75 Y132.721 E.01059
G1 X109.75 Y133.071 E.01252
G1 X106.049 Y129.37 E.18707
G1 X105.403 Y129.37 E.02311
G1 X109.75 Y133.718 E.21976
G1 X109.75 Y134.364 E.02311
G1 X104.756 Y129.37 E.25244
G1 X104.109 Y129.37 E.02311
G1 X109.75 Y135.011 E.28513
G1 X109.75 Y135.658 E.02311
G1 X103.463 Y129.37 E.31781
G1 X102.816 Y129.37 E.02311
G1 X109.75 Y136.304 E.3505
G1 X109.75 Y136.951 E.02311
G1 X102.169 Y129.37 E.38319
G1 X101.523 Y129.37 E.02311
G1 X109.75 Y137.598 E.41587
G1 X109.75 Y138.244 E.02311
G1 X100.876 Y129.37 E.44856
G1 X100.229 Y129.37 E.02311
G1 X109.75 Y138.891 E.48125
G1 X109.75 Y139.538 E.02311
G1 X99.583 Y129.37 E.51393
G1 X98.936 Y129.37 E.02311
G1 X109.75 Y140.185 E.54662
G1 X109.75 Y140.831 E.02311
G1 X102.092 Y133.172 E.38712
G1 X102.092 Y132.721 E.01614
G1 X101.64 Y132.721 E.01614
G1 X98.289 Y129.37 E.16936
G1 X97.643 Y129.37 E.02311
G1 X100.993 Y132.721 E.16936
G1 X100.347 Y132.721 E.02311
G1 X96.996 Y129.37 E.16936
G1 X96.349 Y129.37 E.02311
G1 X99.7 Y132.721 E.16936
G1 X99.053 Y132.721 E.02311
G1 X95.703 Y129.37 E.16936
G1 X95.056 Y129.37 E.02311
G1 X98.75 Y133.065 E.18674
G1 X98.75 Y133.711 E.02311
G1 X94.409 Y129.37 E.21943
G1 X93.763 Y129.37 E.02311
G1 X98.75 Y134.358 E.25211
G1 X98.75 Y135.005 E.02311
G1 X93.116 Y129.37 E.2848
G1 X92.469 Y129.37 E.02311
G1 X98.75 Y135.651 E.31749
G1 X98.75 Y136.298 E.02311
G1 X91.823 Y129.37 E.35017
G1 X91.176 Y129.37 E.02311
G1 X98.75 Y136.945 E.38286
M73 P78 R2
G1 X98.75 Y137.591 E.02311
G1 X90.529 Y129.37 E.41555
G1 X89.883 Y129.37 E.02311
G1 X98.75 Y138.238 E.44823
G1 X98.75 Y138.885 E.02311
G1 X89.236 Y129.37 E.48092
G1 X88.589 Y129.37 E.02311
G1 X98.75 Y139.531 E.5136
G1 X98.75 Y140.178 E.02311
G1 X87.942 Y129.37 E.54629
G1 X87.296 Y129.37 E.02311
G1 X90.647 Y132.721 E.16936
G1 X91.092 Y132.721 E.01591
G1 X91.092 Y133.166 E.01591
G1 X98.75 Y140.825 E.38712
G1 X98.75 Y141.471 E.02311
G1 X91.092 Y133.813 E.38712
G1 X91.092 Y134.459 E.02311
G1 X98.75 Y142.118 E.38712
G1 X98.75 Y142.721 E.02154
G1 X98.707 Y142.721 E.00157
G1 X91.092 Y135.106 E.3849
G1 X91.092 Y135.753 E.02311
G1 X98.06 Y142.721 E.35221
G1 X97.413 Y142.721 E.02311
G1 X91.092 Y136.399 E.31952
G1 X91.092 Y137.046 E.02311
G1 X96.766 Y142.721 E.28684
G1 X96.12 Y142.721 E.02311
G1 X91.092 Y137.693 E.25415
G1 X91.092 Y138.339 E.02311
G1 X95.473 Y142.721 E.22146
G1 X94.826 Y142.721 E.02311
G1 X91.092 Y138.986 E.18878
G1 X91.092 Y139.633 E.02311
G1 X94.18 Y142.721 E.15609
G1 X93.533 Y142.721 E.02311
G1 X91.092 Y140.279 E.1234
G1 X91.092 Y140.926 E.02311
G1 X92.886 Y142.721 E.09072
G1 X92.24 Y142.721 E.02311
G1 X91.092 Y141.573 E.05803
G1 X91.092 Y142.219 E.02311
G1 X91.799 Y142.926 E.03574
; WIPE_START
M73 P79 R2
G1 X91.092 Y142.219 E-.38
G1 X91.092 Y142.219 E0
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I1.211 J-.116 P1  F42000
G1 X90.206 Y132.926 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X86.649 Y129.37 E.17976
G1 X86.002 Y129.37 E.02311
G1 X89.353 Y132.721 E.16936
G1 X88.707 Y132.721 E.02311
G1 X85.356 Y129.37 E.16936
G1 X84.709 Y129.37 E.02311
G1 X88.06 Y132.721 E.16936
G1 X87.75 Y132.721 E.01106
G1 X87.75 Y133.058 E.01206
G1 X84.062 Y129.37 E.18641
G1 X83.416 Y129.37 E.02311
G1 X87.75 Y133.705 E.2191
G1 X87.75 Y134.351 E.02311
G1 X82.769 Y129.37 E.25179
G1 X82.122 Y129.37 E.02311
G1 X87.75 Y134.998 E.28447
G1 X87.75 Y135.645 E.02311
G1 X81.476 Y129.37 E.31716
G1 X80.829 Y129.37 E.02311
G1 X87.75 Y136.291 E.34984
G1 X87.75 Y136.938 E.02311
G1 X80.182 Y129.37 E.38253
G1 X79.536 Y129.37 E.02311
G1 X87.75 Y137.585 E.41522
G1 X87.75 Y138.232 E.02311
G1 X78.889 Y129.37 E.4479
G1 X78.242 Y129.37 E.02311
G1 X87.75 Y138.878 E.48059
G1 X87.75 Y139.525 E.02311
G1 X77.596 Y129.37 E.51328
G1 X76.949 Y129.37 E.02311
G1 X87.75 Y140.172 E.54596
G1 X87.75 Y140.818 E.02311
G1 X80.092 Y133.159 E.38712
G1 X80.092 Y132.721 E.01568
M73 P79 R1
G1 X79.653 Y132.721 E.01568
G1 X76.302 Y129.37 E.16936
G1 X75.656 Y129.37 E.02311
G1 X79.006 Y132.721 E.16936
G1 X78.36 Y132.721 E.02311
G1 X75.009 Y129.37 E.16936
G1 X74.362 Y129.37 E.02311
G1 X77.713 Y132.721 E.16936
G1 X77.066 Y132.721 E.02311
G1 X73.716 Y129.37 E.16936
G1 X73.069 Y129.37 E.02311
G1 X76.75 Y133.052 E.18609
G1 X76.75 Y133.698 E.02311
G1 X72.422 Y129.37 E.21877
G1 X71.776 Y129.37 E.02311
M73 P80 R1
G1 X76.75 Y134.345 E.25146
G1 X76.75 Y134.992 E.02311
G1 X71.129 Y129.37 E.28414
G1 X70.482 Y129.37 E.02311
G1 X76.75 Y135.638 E.31683
G1 X76.75 Y136.285 E.02311
G1 X69.836 Y129.37 E.34952
G1 X69.189 Y129.37 E.02311
G1 X76.75 Y136.932 E.3822
G1 X76.75 Y137.578 E.02311
G1 X69.155 Y129.983 E.38393
G1 X69.152 Y130.626 E.023
G1 X76.75 Y138.225 E.38408
G1 X76.75 Y138.872 E.02311
G1 X69.149 Y131.27 E.38424
G1 X69.145 Y131.913 E.023
G1 X76.75 Y139.518 E.38439
G1 X76.75 Y140.165 E.02311
G1 X69.142 Y132.557 E.38455
G1 X69.139 Y133.201 E.023
G1 X76.75 Y140.812 E.3847
G1 X76.75 Y141.458 E.02311
G1 X69.136 Y133.844 E.38486
G1 X69.133 Y134.488 E.023
G1 X76.75 Y142.105 E.38501
G1 X76.75 Y142.721 E.02201
G1 X76.719 Y142.721 E.00111
G1 X69.13 Y135.132 E.3836
G1 X69.127 Y135.775 E.023
G1 X76.073 Y142.721 E.35107
G1 X75.426 Y142.721 E.02311
G1 X69.124 Y136.419 E.31854
G1 X69.121 Y137.062 E.023
G1 X74.779 Y142.721 E.28601
G1 X74.133 Y142.721 E.02311
G1 X69.118 Y137.706 E.25347
G1 X69.115 Y138.35 E.023
G1 X73.486 Y142.721 E.22094
G1 X72.839 Y142.721 E.02311
G1 X69.112 Y138.993 E.18841
G1 X69.109 Y139.637 E.023
G1 X72.193 Y142.721 E.15588
G1 X71.546 Y142.721 E.02311
G1 X69.106 Y140.28 E.12335
M73 P81 R1
G1 X69.103 Y140.924 E.023
G1 X70.899 Y142.721 E.09082
G1 X70.253 Y142.721 E.02311
G1 X69.1 Y141.568 E.05828
G1 X69.097 Y142.211 E.023
G1 X69.812 Y142.926 E.03615
; WIPE_START
G1 X69.105 Y142.219 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I.76 J.951 P1  F42000
G1 X79.886 Y133.6 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X87.75 Y141.465 E.39751
G1 X87.75 Y142.112 E.02311
G1 X80.092 Y134.453 E.38712
G1 X80.092 Y135.099 E.02311
G1 X87.713 Y142.721 E.38522
G1 X87.066 Y142.721 E.02311
G1 X80.092 Y135.746 E.35254
G1 X80.092 Y136.393 E.02311
G1 X86.42 Y142.721 E.31985
G1 X85.773 Y142.721 E.02311
G1 X80.092 Y137.039 E.28716
G1 X80.092 Y137.686 E.02311
G1 X85.126 Y142.721 E.25448
G1 X84.48 Y142.721 E.02311
G1 X80.092 Y138.333 E.22179
G1 X80.092 Y138.979 E.02311
G1 X83.833 Y142.721 E.1891
G1 X83.186 Y142.721 E.02311
G1 X80.092 Y139.626 E.15642
G1 X80.092 Y140.273 E.02311
G1 X82.54 Y142.721 E.12373
G1 X81.893 Y142.721 E.02311
G1 X80.092 Y140.92 E.09105
G1 X80.092 Y141.566 E.02311
G1 X81.246 Y142.721 E.05836
G1 X80.6 Y142.721 E.02311
G1 X79.886 Y142.007 E.03607
; WIPE_START
G1 X80.593 Y142.714 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I.478 J1.119 P1  F42000
G1 X101.886 Y133.613 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X109.75 Y141.478 E.39751
G1 X109.75 Y142.125 E.02311
G1 X102.092 Y134.466 E.38712
G1 X102.092 Y135.112 E.02311
M73 P82 R1
G1 X109.7 Y142.721 E.38457
G1 X109.053 Y142.721 E.02311
G1 X102.092 Y135.759 E.35188
G1 X102.092 Y136.406 E.02311
G1 X108.407 Y142.721 E.31919
G1 X107.76 Y142.721 E.02311
G1 X102.092 Y137.052 E.28651
G1 X102.092 Y137.699 E.02311
G1 X107.113 Y142.721 E.25382
G1 X106.467 Y142.721 E.02311
G1 X102.092 Y138.346 E.22114
G1 X102.092 Y138.992 E.02311
G1 X105.82 Y142.721 E.18845
G1 X105.173 Y142.721 E.02311
G1 X102.092 Y139.639 E.15576
G1 X102.092 Y140.286 E.02311
G1 X104.527 Y142.721 E.12308
G1 X103.88 Y142.721 E.02311
G1 X102.092 Y140.932 E.09039
G1 X102.092 Y141.579 E.02311
G1 X103.233 Y142.721 E.0577
G1 X102.587 Y142.721 E.02311
G1 X101.886 Y142.02 E.03541
; WIPE_START
G1 X102.587 Y142.721 E-.37651
G1 X102.596 Y142.721 E-.00349
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
G3 Z.6 I.478 J1.119 P1  F42000
G1 X123.886 Y133.626 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X131.75 Y141.491 E.39751
G1 X131.75 Y142.138 E.02311
G1 X124.092 Y134.479 E.38712
G1 X124.092 Y135.125 E.02311
G1 X131.687 Y142.721 E.38391
G1 X131.04 Y142.721 E.02311
G1 X124.092 Y135.772 E.35123
G1 X124.092 Y136.419 E.02311
G1 X130.394 Y142.721 E.31854
G1 X129.747 Y142.721 E.02311
G1 X124.092 Y137.065 E.28585
G1 X124.092 Y137.712 E.02311
G1 X129.1 Y142.721 E.25317
G1 X128.454 Y142.721 E.02311
G1 X124.092 Y138.359 E.22048
G1 X124.092 Y139.005 E.02311
G1 X127.807 Y142.721 E.18779
G1 X127.16 Y142.721 E.02311
G1 X124.092 Y139.652 E.15511
G1 X124.092 Y140.299 E.02311
G1 X126.514 Y142.721 E.12242
G1 X125.867 Y142.721 E.02311
G1 X124.092 Y140.945 E.08973
G1 X124.092 Y141.592 E.02311
G1 X125.22 Y142.721 E.05705
G1 X124.574 Y142.721 E.02311
G1 X123.886 Y142.033 E.03476
; WIPE_START
G1 X124.574 Y142.721 E-.36954
G1 X124.601 Y142.721 E-.01046
; WIPE_END
G1 E-.02 F1800
M204 S6000
G17
M73 P83 R1
G3 Z.6 I.478 J1.119 P1  F42000
G1 X145.886 Y133.639 Z.6
G1 Z.2
G1 E.4 F1800
G1 F6300
M204 S500
G1 X153.75 Y141.504 E.39751
G1 X153.75 Y142.151 E.02311
G1 X146.092 Y134.492 E.38712
G1 X146.092 Y135.138 E.02311
G1 X153.674 Y142.721 E.38326
G1 X153.027 Y142.721 E.02311
G1 X146.092 Y135.785 E.35057
G1 X146.092 Y136.432 E.02311
G1 X152.381 Y142.721 E.31788
G1 X151.734 Y142.721 E.02311
G1 X146.092 Y137.078 E.2852
G1 X146.092 Y137.725 E.02311
G1 X151.087 Y142.721 E.25251
G1 X150.441 Y142.721 E.02311
G1 X146.092 Y138.372 E.21982
G1 X146.092 Y139.018 E.02311
G1 X149.794 Y142.721 E.18714
G1 X149.147 Y142.721 E.02311
G1 X146.092 Y139.665 E.15445
G1 X146.092 Y140.312 E.02311
G1 X148.501 Y142.721 E.12176
G1 X147.854 Y142.721 E.02311
G1 X146.092 Y140.958 E.08908
G1 X146.092 Y141.605 E.02311
G1 X147.207 Y142.721 E.05639
G1 X146.561 Y142.721 E.02311
G1 X145.886 Y142.046 E.0341
; CHANGE_LAYER
; Z_HEIGHT: 0.4
; LAYER_HEIGHT: 0.2
; WIPE_START
G1 F6300
G1 X146.561 Y142.721 E-.36258
G1 X146.606 Y142.721 E-.01742
; WIPE_END
G1 E-.02 F1800
; layer num/total_layer_count: 2/2
; update layer progress
M73 L2
M991 S0 P1 ;notify layer change
; open powerlost recovery
M1003 S1
; OBJECT_ID: 121
M204 S10000
G17
G3 Z.6 I.664 J-1.02 P1  F42000
G1 X132.711 Y133.681 Z.6
G1 Z.4
G1 E.4 F1800
; FEATURE: Outer wall
; LINE_WIDTH: 0.42
G1 F10342.643
M204 S5000
G1 X132.711 Y143.23 E.28141
G3 X132.259 Y143.681 I-.453 J-.001 E.0209
G1 X123.583 Y143.681 E.25572
G3 X123.131 Y143.23 I-.002 J-.45 E.02094
G1 X123.131 Y133.681 E.28141
G1 X121.711 Y133.681 E.04185
G1 X121.711 Y143.23 E.28142
G3 X121.259 Y143.681 I-.45 J.002 E.02094
G1 X112.583 Y143.681 E.25572
G3 X112.131 Y143.23 I.02 J-.471 E.02069
G1 X112.131 Y133.681 E.28141
M73 P84 R1
G1 X110.711 Y133.681 E.04185
G1 X110.711 Y143.23 E.28142
G3 X110.259 Y143.681 I-.45 J.002 E.02094
G1 X101.583 Y143.681 E.25572
G3 X101.131 Y143.23 I.02 J-.471 E.02069
G1 X101.131 Y133.681 E.28141
G1 X99.711 Y133.681 E.04185
G1 X99.711 Y143.23 E.28141
G3 X99.259 Y143.681 I-.45 J.002 E.02094
G1 X90.583 Y143.681 E.25572
G3 X90.131 Y143.23 I.02 J-.471 E.02069
G1 X90.131 Y133.681 E.28141
G1 X88.711 Y133.681 E.04185
G1 X88.711 Y143.23 E.28141
G3 X88.259 Y143.681 I-.45 J.002 E.02094
G1 X79.583 Y143.681 E.25572
G3 X79.131 Y143.23 I.02 J-.471 E.02069
G1 X79.131 Y133.681 E.28141
G1 X77.711 Y133.681 E.04185
G1 X77.711 Y143.23 E.28141
G3 X77.259 Y143.681 I-.45 J.002 E.02094
G1 X68.583 Y143.681 E.25572
G3 X68.131 Y143.23 I-.002 J-.45 E.02092
G1 X68.2 Y128.761 E.42646
G3 X68.553 Y128.409 I.366 J.015 E.01613
G1 X125.03 Y128.409 E1.66455
G1 X125.143 Y128.832 E.01289
G2 X130.727 Y128.409 I2.732 J-1.007 E.22192
G1 X187.361 Y128.409 E1.66919
G3 X187.711 Y128.76 I-.013 J.363 E.01606
G1 X187.711 Y143.229 E.42647
G3 X187.259 Y143.681 I-.45 J.002 E.02094
G1 X178.583 Y143.681 E.25572
G3 X178.131 Y143.23 I-.002 J-.45 E.02094
G1 X178.131 Y133.681 E.28142
G1 X176.711 Y133.681 E.04185
G1 X176.711 Y143.229 E.28141
G3 X176.259 Y143.681 I-.45 J.002 E.02094
G1 X167.583 Y143.681 E.25572
G3 X167.131 Y143.23 I-.002 J-.45 E.02094
G1 X167.131 Y133.681 E.28142
G1 X165.711 Y133.681 E.04185
G1 X165.711 Y143.23 E.28141
G3 X165.259 Y143.681 I-.45 J.002 E.02094
G1 X156.583 Y143.681 E.25572
G3 X156.131 Y143.23 I.02 J-.471 E.02068
G1 X156.131 Y133.681 E.28142
G1 X154.711 Y133.681 E.04185
G1 X154.711 Y143.23 E.28141
G3 X154.259 Y143.681 I-.45 J.002 E.02094
G1 X145.583 Y143.681 E.25572
G3 X145.131 Y143.23 I-.002 J-.45 E.02094
G1 X145.131 Y133.681 E.28142
G1 X143.711 Y133.681 E.04185
G1 X143.711 Y143.23 E.28141
G3 X143.259 Y143.681 I-.45 J.002 E.02094
G1 X134.583 Y143.681 E.25572
G3 X134.131 Y143.23 I-.002 J-.45 E.02094
G1 X134.131 Y133.681 E.28142
G1 X132.771 Y133.681 E.04008
; WIPE_START
M204 S6000
G1 X132.765 Y134.681 E-.38
; WIPE_END
G1 E-.02 F1800
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
G1 X0 Y136.027 F18000 ; move to safe pos
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


G1 X176.504 Y142.687 F42000
G1 Z.4
G1 E.4 F1800
; FEATURE: Top surface
G1 F10342.643
M204 S2000
G1 X175.717 Y143.474 E.03279
G1 X175.184 Y143.474
G1 X176.504 Y142.154 E.05502
G1 X176.504 Y141.621
G1 X174.65 Y143.474 E.07725
G1 X174.117 Y143.474
G1 X176.504 Y141.088 E.09947
G1 X176.504 Y140.554
G1 X173.584 Y143.474 E.1217
G1 X173.051 Y143.474
G1 X176.504 Y140.021 E.14393
G1 X176.504 Y139.488
G1 X172.517 Y143.474 E.16615
G1 X171.984 Y143.474
G1 X176.504 Y138.954 E.18838
G1 X176.504 Y138.421
G1 X171.451 Y143.474 E.21061
G1 X170.918 Y143.474
G1 X176.504 Y137.888 E.23283
G1 X176.504 Y137.355
G1 X170.384 Y143.474 E.25506
G1 X169.851 Y143.474
M73 P85 R1
G1 X176.504 Y136.821 E.27729
G1 X176.504 Y136.288
G1 X169.318 Y143.474 E.29951
G1 X168.785 Y143.474
G1 X176.504 Y135.755 E.32174
G1 X176.504 Y135.222
G1 X168.251 Y143.474 E.34397
G1 X167.718 Y143.474
G1 X176.504 Y134.688 E.36619
G1 X176.504 Y134.155
G1 X167.356 Y143.303 E.38128
G1 X167.338 Y142.787
G1 X176.504 Y133.622 E.38202
; WIPE_START
M204 S6000
G1 X175.797 Y134.329 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.773 J.94 P1  F42000
G1 X186.915 Y143.474 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X187.504 Y142.886 E.02452
G1 X187.504 Y142.352
G1 X186.382 Y143.474 E.04675
G1 X185.849 Y143.474
G1 X187.504 Y141.819 E.06898
G1 X187.504 Y141.286
G1 X185.316 Y143.474 E.0912
G1 X184.782 Y143.474
G1 X187.504 Y140.753 E.11343
G1 X187.504 Y140.219
G1 X184.249 Y143.474 E.13566
G1 X183.716 Y143.474
G1 X187.504 Y139.686 E.15788
G1 X187.504 Y139.153
G1 X183.183 Y143.474 E.18011
G1 X182.649 Y143.474
G1 X187.504 Y138.62 E.20234
G1 X187.504 Y138.086
G1 X182.116 Y143.474 E.22456
G1 X181.583 Y143.474
G1 X187.504 Y137.553 E.24679
G1 X187.504 Y137.02
G1 X181.05 Y143.474 E.26902
G1 X180.516 Y143.474
G1 X187.504 Y136.487 E.29124
G1 X187.504 Y135.953
G1 X179.983 Y143.474 E.31347
G1 X179.45 Y143.474
G1 X187.504 Y135.42 E.3357
G1 X187.504 Y134.887
G1 X178.917 Y143.474 E.35792
G1 X178.439 Y143.418
G1 X187.504 Y134.354 E.37782
G1 X187.504 Y133.82
G1 X178.338 Y142.986 E.38202
G1 X178.338 Y142.452
G1 X187.504 Y133.287 E.38202
G1 X187.504 Y132.754
G1 X178.338 Y141.919 E.38202
G1 X178.338 Y141.386
G1 X187.504 Y132.221 E.38202
G1 X187.504 Y131.687
G1 X178.338 Y140.853 E.38202
G1 X178.338 Y140.319
G1 X187.504 Y131.154 E.38202
G1 X187.504 Y130.621
G1 X178.338 Y139.786 E.38202
G1 X178.338 Y139.253
G1 X187.504 Y130.088 E.38202
G1 X187.504 Y129.554
G1 X178.338 Y138.719 E.38202
G1 X178.338 Y138.186
G1 X187.504 Y129.021 E.38202
G1 X187.365 Y128.627
G1 X178.338 Y137.653 E.37623
G1 X178.338 Y137.12
G1 X186.832 Y128.627 E.35401
G1 X186.298 Y128.626
G1 X178.338 Y136.586 E.33178
G1 X178.338 Y136.053
G1 X185.765 Y128.626 E.30956
G1 X185.232 Y128.626
G1 X178.338 Y135.52 E.28734
G1 X178.338 Y134.987
G1 X184.699 Y128.626 E.26512
G1 X184.166 Y128.626
G1 X178.338 Y134.453 E.24289
G1 X178.338 Y133.92
M73 P86 R1
G1 X183.633 Y128.626 E.22067
G1 X183.099 Y128.626
G1 X178.251 Y133.474 E.20208
G1 X177.718 Y133.474
G1 X182.566 Y128.626 E.20208
G1 X182.033 Y128.626
G1 X177.185 Y133.474 E.20208
G1 X176.652 Y133.474
G1 X181.5 Y128.626 E.20209
G1 X180.967 Y128.626
G1 X167.338 Y142.254 E.56805
G1 X167.338 Y141.721
G1 X180.434 Y128.625 E.54582
G1 X179.9 Y128.625
G1 X167.338 Y141.187 E.5236
G1 X167.338 Y140.654
G1 X179.367 Y128.625 E.50138
G1 X178.834 Y128.625
G1 X167.338 Y140.121 E.47916
G1 X167.338 Y139.588
G1 X178.301 Y128.625 E.45693
G1 X177.768 Y128.625
G1 X167.338 Y139.054 E.43471
G1 X167.338 Y138.521
G1 X177.235 Y128.625 E.41249
G1 X176.701 Y128.625
G1 X167.338 Y137.988 E.39026
G1 X167.338 Y137.455
G1 X176.168 Y128.625 E.36804
G1 X175.635 Y128.625
G1 X167.338 Y136.921 E.34582
G1 X167.338 Y136.388
G1 X175.102 Y128.624 E.32359
G1 X174.569 Y128.624
G1 X167.338 Y135.855 E.30137
G1 X167.338 Y135.322
G1 X174.036 Y128.624 E.27915
G1 X173.502 Y128.624
G1 X167.338 Y134.788 E.25692
G1 X167.338 Y134.255
G1 X172.969 Y128.624 E.2347
G1 X172.436 Y128.624
G1 X167.338 Y133.722 E.21248
G1 X167.053 Y133.474
G1 X171.903 Y128.624 E.20216
G1 X171.37 Y128.624
G1 X166.52 Y133.474 E.20216
G1 X165.986 Y133.474
G1 X170.837 Y128.624 E.20216
; WIPE_START
M204 S6000
G1 X170.13 Y129.331 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-1.153 J-.39 P1  F42000
G1 X165.504 Y143.022 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X165.052 Y143.474 E.01883
G1 X164.519 Y143.474
G1 X165.504 Y142.489 E.04106
G1 X165.504 Y141.956
G1 X163.985 Y143.474 E.06329
G1 X163.452 Y143.474
G1 X165.504 Y141.422 E.08552
G1 X165.504 Y140.889
G1 X162.919 Y143.474 E.10774
G1 X162.386 Y143.474
G1 X165.504 Y140.356 E.12997
G1 X165.504 Y139.823
G1 X161.852 Y143.474 E.1522
M73 P87 R1
G1 X161.319 Y143.474
G1 X165.504 Y139.289 E.17442
G1 X165.504 Y138.756
G1 X160.786 Y143.474 E.19665
G1 X160.252 Y143.474
G1 X165.504 Y138.223 E.21888
G1 X165.504 Y137.69
G1 X159.719 Y143.474 E.2411
G1 X159.186 Y143.474
G1 X165.504 Y137.156 E.26333
G1 X165.504 Y136.623
G1 X158.653 Y143.474 E.28556
G1 X158.119 Y143.474
G1 X165.504 Y136.09 E.30778
G1 X165.504 Y135.557
G1 X157.586 Y143.474 E.33001
G1 X157.053 Y143.474
G1 X165.504 Y135.023 E.35224
G1 X165.504 Y134.49
G1 X156.535 Y143.458 E.37381
G1 X156.338 Y143.122
G1 X165.504 Y133.957 E.38202
; WIPE_START
M204 S6000
G1 X164.797 Y134.664 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.899 J.82 P1  F42000
G1 X170.303 Y128.624 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X156.338 Y142.589 E.58208
G1 X156.338 Y142.055
G1 X169.77 Y128.624 E.55986
G1 X169.237 Y128.623
G1 X156.338 Y141.522 E.53763
G1 X156.338 Y140.989
G1 X168.704 Y128.623 E.51541
G1 X168.171 Y128.623
G1 X156.338 Y140.456 E.49319
G1 X156.338 Y139.922
G1 X167.638 Y128.623 E.47096
G1 X167.104 Y128.623
G1 X156.338 Y139.389 E.44874
G1 X156.338 Y138.856
G1 X166.571 Y128.623 E.42652
G1 X166.038 Y128.623
G1 X156.338 Y138.323 E.4043
G1 X156.338 Y137.789
G1 X165.505 Y128.623 E.38207
G1 X164.972 Y128.623
G1 X156.338 Y137.256 E.35985
G1 X156.338 Y136.723
G1 X164.439 Y128.623 E.33763
G1 X163.906 Y128.623
G1 X156.338 Y136.19 E.3154
G1 X156.338 Y135.656
G1 X163.372 Y128.622 E.29318
G1 X162.839 Y128.622
G1 X156.338 Y135.123 E.27096
G1 X156.338 Y134.59
G1 X162.306 Y128.622 E.24874
G1 X161.773 Y128.622
G1 X156.338 Y134.057 E.22651
G1 X156.338 Y133.523
G1 X161.24 Y128.622 E.20429
G1 X160.707 Y128.622
G1 X155.855 Y133.474 E.20224
G1 X155.321 Y133.474
G1 X160.173 Y128.622 E.20224
G1 X159.64 Y128.622
G1 X154.788 Y133.474 E.20224
; WIPE_START
M204 S6000
G1 X155.495 Y132.767 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-1.211 J-.119 P1  F42000
G1 X154.504 Y142.824 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X153.853 Y143.474 E.0271
G1 X153.32 Y143.474
G1 X154.504 Y142.29 E.04933
G1 X154.504 Y141.757
G1 X152.787 Y143.474 E.07156
G1 X152.254 Y143.474
G1 X154.504 Y141.224 E.09379
G1 X154.504 Y140.691
M73 P88 R1
G1 X151.72 Y143.474 E.11601
G1 X151.187 Y143.474
G1 X154.504 Y140.157 E.13824
G1 X154.504 Y139.624
G1 X150.654 Y143.474 E.16047
G1 X150.121 Y143.474
G1 X154.504 Y139.091 E.18269
G1 X154.504 Y138.558
G1 X149.587 Y143.474 E.20492
G1 X149.054 Y143.474
G1 X154.504 Y138.024 E.22715
G1 X154.504 Y137.491
G1 X148.521 Y143.474 E.24937
G1 X147.988 Y143.474
G1 X154.504 Y136.958 E.2716
G1 X154.504 Y136.425
G1 X147.454 Y143.474 E.29383
G1 X146.921 Y143.474
G1 X154.504 Y135.891 E.31605
G1 X154.504 Y135.358
G1 X146.388 Y143.474 E.33828
G1 X145.855 Y143.474
G1 X154.504 Y134.825 E.36051
G1 X154.504 Y134.292
G1 X145.405 Y143.391 E.37925
G1 X145.338 Y142.924
G1 X154.504 Y133.758 E.38202
; WIPE_START
M204 S6000
G1 X153.797 Y134.465 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.901 J.818 P1  F42000
G1 X159.107 Y128.622 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X145.338 Y142.39 E.57389
G1 X145.338 Y141.857
G1 X158.574 Y128.622 E.55167
G1 X158.041 Y128.622
G1 X145.338 Y141.324 E.52944
G1 X145.338 Y140.791
G1 X157.508 Y128.621 E.50722
G1 X156.974 Y128.621
G1 X145.338 Y140.257 E.485
G1 X145.338 Y139.724
G1 X156.441 Y128.621 E.46278
G1 X155.908 Y128.621
G1 X145.338 Y139.191 E.44055
G1 X145.338 Y138.658
G1 X155.375 Y128.621 E.41833
G1 X154.842 Y128.621
G1 X145.338 Y138.124 E.39611
G1 X145.338 Y137.591
G1 X154.309 Y128.621 E.37388
G1 X153.775 Y128.621
G1 X145.338 Y137.058 E.35166
G1 X145.338 Y136.524
G1 X153.242 Y128.621 E.32944
G1 X152.709 Y128.621
G1 X145.338 Y135.991 E.30721
G1 X145.338 Y135.458
G1 X152.176 Y128.621 E.28499
G1 X151.643 Y128.62
G1 X145.338 Y134.925 E.26277
G1 X145.338 Y134.391
G1 X151.11 Y128.62 E.24055
G1 X150.576 Y128.62
G1 X145.338 Y133.858 E.21832
G1 X145.189 Y133.474
G1 X150.043 Y128.62 E.20231
G1 X149.51 Y128.62
G1 X144.656 Y133.474 E.20232
G1 X144.123 Y133.474
G1 X148.977 Y128.62 E.20232
G1 X148.444 Y128.62
G1 X143.59 Y133.474 E.20232
; WIPE_START
M204 S6000
G1 X144.297 Y132.767 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-1.213 J-.098 P1  F42000
G1 X143.504 Y142.625 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X142.655 Y143.474 E.03537
G1 X142.122 Y143.474
M73 P89 R1
G1 X143.504 Y142.092 E.0576
G1 X143.504 Y141.559
G1 X141.588 Y143.474 E.07983
G1 X141.055 Y143.474
G1 X143.504 Y141.026 E.10206
G1 X143.504 Y140.492
G1 X140.522 Y143.474 E.12428
G1 X139.989 Y143.474
G1 X143.504 Y139.959 E.14651
G1 X143.504 Y139.426
G1 X139.455 Y143.474 E.16874
G1 X138.922 Y143.474
G1 X143.504 Y138.892 E.19096
G1 X143.504 Y138.359
G1 X138.389 Y143.474 E.21319
G1 X137.856 Y143.474
G1 X143.504 Y137.826 E.23542
G1 X143.504 Y137.293
G1 X137.322 Y143.474 E.25764
G1 X136.789 Y143.474
G1 X143.504 Y136.759 E.27987
G1 X143.504 Y136.226
G1 X136.256 Y143.474 E.3021
G1 X135.723 Y143.474
G1 X143.504 Y135.693 E.32432
G1 X143.504 Y135.16
G1 X135.189 Y143.474 E.34655
G1 X134.656 Y143.474
G1 X143.504 Y134.626 E.36878
G1 X143.504 Y134.093
G1 X134.343 Y143.254 E.38185
G1 X134.338 Y142.725
G1 X143.504 Y133.56 E.38202
; WIPE_START
M204 S6000
G1 X142.797 Y134.267 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.902 J.817 P1  F42000
G1 X147.911 Y128.62 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X134.338 Y142.192 E.5657
G1 X134.338 Y141.659
G1 X147.377 Y128.62 E.54348
G1 X146.844 Y128.62
G1 X134.338 Y141.125 E.52125
G1 X134.338 Y140.592
G1 X146.311 Y128.62 E.49903
G1 X145.778 Y128.619
G1 X134.338 Y140.059 E.47681
G1 X134.338 Y139.526
G1 X145.245 Y128.619 E.45459
G1 X144.712 Y128.619
G1 X134.338 Y138.992 E.43236
G1 X134.338 Y138.459
G1 X144.178 Y128.619 E.41014
G1 X143.645 Y128.619
G1 X134.338 Y137.926 E.38792
G1 X134.338 Y137.393
G1 X143.112 Y128.619 E.36569
G1 X142.579 Y128.619
G1 X134.338 Y136.859 E.34347
G1 X134.338 Y136.326
G1 X142.046 Y128.619 E.32125
G1 X141.513 Y128.619
G1 X134.338 Y135.793 E.29903
G1 X134.338 Y135.26
G1 X140.979 Y128.619 E.2768
G1 X140.446 Y128.619
G1 X134.338 Y134.726 E.25458
G1 X134.338 Y134.193
G1 X139.913 Y128.618 E.23236
G1 X139.38 Y128.618
G1 X134.338 Y133.66 E.21013
G1 X133.991 Y133.474
G1 X138.847 Y128.618 E.20239
G1 X138.314 Y128.618
M73 P89 R0
G1 X133.458 Y133.474 E.2024
G1 X132.924 Y133.474
G1 X137.78 Y128.618 E.2024
; WIPE_START
M204 S6000
G1 X137.073 Y129.325 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-1.145 J-.411 P1  F42000
G1 X131.99 Y143.474 Z.8
G1 Z.4
G1 E.4 F1800
M73 P90 R0
G1 F10342.643
M204 S2000
G1 X132.504 Y142.96 E.02142
G1 X132.504 Y142.427
G1 X131.457 Y143.474 E.04364
G1 X130.923 Y143.474
G1 X132.504 Y141.894 E.06587
G1 X132.504 Y141.36
G1 X130.39 Y143.474 E.0881
G1 X129.857 Y143.474
G1 X132.504 Y140.827 E.11033
G1 X132.504 Y140.294
G1 X129.324 Y143.474 E.13255
G1 X128.79 Y143.474
G1 X132.504 Y139.761 E.15478
G1 X132.504 Y139.227
G1 X128.257 Y143.474 E.17701
G1 X127.724 Y143.474
G1 X132.504 Y138.694 E.19923
G1 X132.504 Y138.161
G1 X127.191 Y143.474 E.22146
G1 X126.657 Y143.474
G1 X132.504 Y137.628 E.24369
G1 X132.504 Y137.094
G1 X126.124 Y143.474 E.26591
G1 X125.591 Y143.474
G1 X132.504 Y136.561 E.28814
G1 X132.504 Y136.028
G1 X125.057 Y143.474 E.31037
G1 X124.524 Y143.474
G1 X132.504 Y135.495 E.33259
G1 X132.504 Y134.961
G1 X123.991 Y143.474 E.35482
G1 X123.485 Y143.447
G1 X132.504 Y134.428 E.37593
G1 X132.504 Y133.895
G1 X123.338 Y143.06 E.38202
G1 X123.338 Y142.527
G1 X137.247 Y128.618 E.57973
G1 X136.714 Y128.618
G1 X123.338 Y141.994 E.55751
G1 X123.338 Y141.46
G1 X136.181 Y128.618 E.53529
G1 X135.648 Y128.618
G1 X123.338 Y140.927 E.51307
G1 X123.338 Y140.394
G1 X135.115 Y128.618 E.49084
G1 X134.581 Y128.618
G1 X123.338 Y139.86 E.46862
G1 X123.338 Y139.327
G1 X134.048 Y128.617 E.4464
G1 X133.515 Y128.617
G1 X123.338 Y138.794 E.42417
G1 X123.338 Y138.261
G1 X132.982 Y128.617 E.40195
G1 X132.449 Y128.617
G1 X123.338 Y137.727 E.37973
G1 X123.338 Y137.194
G1 X131.916 Y128.617 E.3575
G1 X131.382 Y128.617
G1 X130.523 Y129.477 E.03584
; WIPE_START
M204 S6000
G1 X131.23 Y128.77 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.861 J-.861 P1  F42000
G1 X129.518 Y130.481 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X123.338 Y136.661 E.25759
G1 X123.338 Y136.128
G1 X128.609 Y130.857 E.21967
G1 X127.987 Y130.946
G1 X123.338 Y135.594 E.19376
G1 X123.338 Y135.061
G1 X127.479 Y130.921 E.17259
G1 X127.038 Y130.828
G1 X123.338 Y134.528 E.15421
G1 X123.338 Y133.995
G1 X126.644 Y130.689 E.13777
G1 X126.285 Y130.514
G1 X123.326 Y133.474 E.12336
G1 X122.793 Y133.474
M73 P91 R0
G1 X125.972 Y130.295 E.13252
G1 X125.69 Y130.043
G1 X122.259 Y133.474 E.14301
G1 X121.726 Y133.474
G1 X125.435 Y129.765 E.1546
; WIPE_START
M204 S6000
G1 X124.728 Y130.472 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-1.177 J-.309 P1  F42000
G1 X121.504 Y142.762 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X120.791 Y143.474 E.02969
G1 X120.258 Y143.474
G1 X121.504 Y142.228 E.05191
G1 X121.504 Y141.695
G1 X119.725 Y143.474 E.07414
G1 X119.192 Y143.474
G1 X121.504 Y141.162 E.09637
G1 X121.504 Y140.629
G1 X118.658 Y143.474 E.11859
G1 X118.125 Y143.474
G1 X121.504 Y140.095 E.14082
G1 X121.504 Y139.562
G1 X117.592 Y143.474 E.16305
G1 X117.059 Y143.474
G1 X121.504 Y139.029 E.18528
G1 X121.504 Y138.496
G1 X116.525 Y143.474 E.2075
G1 X115.992 Y143.474
G1 X121.504 Y137.962 E.22973
G1 X121.504 Y137.429
G1 X115.459 Y143.474 E.25196
G1 X114.926 Y143.474
G1 X121.504 Y136.896 E.27418
G1 X121.504 Y136.363
G1 X114.392 Y143.474 E.29641
G1 X113.859 Y143.474
G1 X121.504 Y135.829 E.31864
G1 X121.504 Y135.296
G1 X113.326 Y143.474 E.34086
G1 X112.793 Y143.474
G1 X121.504 Y134.763 E.36309
G1 X121.504 Y134.23
G1 X112.38 Y143.353 E.38027
G1 X112.338 Y142.862
G1 X121.504 Y133.696 E.38202
; WIPE_START
M204 S6000
G1 X120.797 Y134.403 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.908 J.81 P1  F42000
G1 X125.216 Y129.451 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X112.338 Y142.328 E.53674
G1 X112.338 Y141.795
G1 X125.037 Y129.096 E.5293
G1 X124.894 Y128.706
G1 X112.338 Y141.262 E.52335
G1 X112.338 Y140.729
G1 X124.45 Y128.617 E.50483
G1 X123.917 Y128.617
G1 X112.338 Y140.195 E.4826
G1 X112.338 Y139.662
G1 X123.383 Y128.617 E.46037
G1 X122.85 Y128.617
G1 X112.338 Y139.129 E.43813
G1 X112.338 Y138.596
G1 X122.317 Y128.617 E.4159
G1 X121.783 Y128.617
G1 X112.338 Y138.062 E.39367
G1 X112.338 Y137.529
G1 X121.25 Y128.618 E.37144
G1 X120.717 Y128.618
G1 X112.338 Y136.996 E.34921
G1 X112.338 Y136.463
G1 X120.183 Y128.618 E.32698
G1 X119.65 Y128.618
G1 X112.338 Y135.929 E.30475
G1 X112.338 Y135.396
G1 X119.117 Y128.618 E.28252
G1 X118.583 Y128.618
G1 X112.338 Y134.863 E.26029
G1 X112.338 Y134.329
G1 X118.05 Y128.618 E.23806
G1 X117.517 Y128.618
G1 X112.338 Y133.796 E.21583
G1 X112.127 Y133.474
G1 X116.983 Y128.618 E.20239
G1 X116.45 Y128.618
G1 X111.594 Y133.474 E.20239
G1 X111.061 Y133.474
G1 X115.916 Y128.618 E.20239
G1 X115.383 Y128.619
M73 P92 R0
G1 X110.528 Y133.474 E.20238
; WIPE_START
M204 S6000
G1 X111.235 Y132.767 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.799 J-.918 P1  F42000
G1 X98.928 Y143.474 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X99.504 Y142.898 E.024
G1 X99.504 Y142.365
G1 X98.395 Y143.474 E.04623
G1 X97.861 Y143.474
G1 X99.504 Y141.832 E.06845
G1 X99.504 Y141.298
G1 X97.328 Y143.474 E.09068
G1 X96.795 Y143.474
G1 X99.504 Y140.765 E.11291
G1 X99.504 Y140.232
G1 X96.262 Y143.474 E.13513
G1 X95.728 Y143.474
G1 X99.504 Y139.699 E.15736
G1 X99.504 Y139.165
G1 X95.195 Y143.474 E.17959
G1 X94.662 Y143.474
G1 X99.504 Y138.632 E.20181
G1 X99.504 Y138.099
G1 X94.129 Y143.474 E.22404
G1 X93.595 Y143.474
G1 X99.504 Y137.566 E.24627
G1 X99.504 Y137.032
G1 X93.062 Y143.474 E.2685
G1 X92.529 Y143.474
G1 X99.504 Y136.499 E.29072
G1 X99.504 Y135.966
G1 X91.996 Y143.474 E.31295
G1 X91.462 Y143.474
G1 X99.504 Y135.433 E.33518
G1 X99.504 Y134.899
G1 X90.929 Y143.474 E.3574
G1 X90.446 Y143.424
G1 X99.504 Y134.366 E.37753
G1 X99.504 Y133.833
G1 X90.338 Y142.998 E.38202
; WIPE_START
M204 S6000
G1 X91.046 Y142.291 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.193 J-1.202 P1  F42000
G1 X88.504 Y142.7 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X87.729 Y143.474 E.03227
G1 X87.196 Y143.474
G1 X88.504 Y142.167 E.0545
G1 X88.504 Y141.633
G1 X86.663 Y143.474 E.07672
G1 X86.13 Y143.474
G1 X88.504 Y141.1 E.09895
G1 X88.504 Y140.567
G1 X85.596 Y143.474 E.12118
G1 X85.063 Y143.474
G1 X88.504 Y140.033 E.1434
G1 X88.504 Y139.5
G1 X84.53 Y143.474 E.16563
G1 X83.997 Y143.474
G1 X88.504 Y138.967 E.18786
G1 X88.504 Y138.434
G1 X83.463 Y143.474 E.21008
G1 X82.93 Y143.474
G1 X88.504 Y137.9 E.23231
G1 X88.504 Y137.367
G1 X82.397 Y143.474 E.25454
G1 X81.864 Y143.474
G1 X88.504 Y136.834 E.27676
G1 X88.504 Y136.301
G1 X81.33 Y143.474 E.29899
G1 X80.797 Y143.474
G1 X88.504 Y135.767 E.32122
G1 X88.504 Y135.234
G1 X80.264 Y143.474 E.34345
G1 X79.731 Y143.474
G1 X88.504 Y134.701 E.36567
G1 X88.504 Y134.168
G1 X79.36 Y143.311 E.38111
G1 X79.338 Y142.8
G1 X88.504 Y133.634 E.38202
; WIPE_START
M204 S6000
G1 X87.797 Y134.342 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.785 J-.93 P1  F42000
G1 X77.504 Y143.035 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X77.064 Y143.474 E.01831
G1 X76.531 Y143.474
G1 X77.504 Y142.501 E.04054
G1 X77.504 Y141.968
M73 P93 R0
G1 X75.998 Y143.474 E.06277
G1 X75.465 Y143.474
G1 X77.504 Y141.435 E.08499
G1 X77.504 Y140.902
G1 X74.931 Y143.474 E.10722
G1 X74.398 Y143.474
G1 X77.504 Y140.368 E.12945
G1 X77.504 Y139.835
G1 X73.865 Y143.474 E.15167
G1 X73.331 Y143.474
G1 X77.504 Y139.302 E.1739
G1 X77.504 Y138.769
G1 X72.798 Y143.474 E.19613
G1 X72.265 Y143.474
G1 X77.504 Y138.235 E.21835
G1 X77.504 Y137.702
G1 X71.732 Y143.474 E.24058
G1 X71.198 Y143.474
G1 X77.504 Y137.169 E.26281
G1 X77.504 Y136.636
G1 X70.665 Y143.474 E.28503
G1 X70.132 Y143.474
G1 X77.504 Y136.102 E.30726
G1 X77.504 Y135.569
G1 X69.599 Y143.474 E.32949
G1 X69.065 Y143.474
G1 X77.504 Y135.036 E.35172
G1 X77.504 Y134.503
G1 X68.538 Y143.468 E.37368
G1 X68.339 Y143.134
G1 X77.504 Y133.969 E.382
; WIPE_START
M204 S6000
G1 X76.797 Y134.676 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.315 J1.175 P1  F42000
G1 X109.593 Y143.474 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X110.504 Y142.563 E.03796
G1 X110.504 Y142.03
G1 X109.06 Y143.474 E.06018
G1 X108.526 Y143.474
G1 X110.504 Y141.497 E.08241
G1 X110.504 Y140.964
G1 X107.993 Y143.474 E.10464
G1 X107.46 Y143.474
G1 X110.504 Y140.43 E.12686
G1 X110.504 Y139.897
G1 X106.927 Y143.474 E.14909
G1 X106.393 Y143.474
G1 X110.504 Y139.364 E.17132
G1 X110.504 Y138.831
G1 X105.86 Y143.474 E.19355
G1 X105.327 Y143.474
G1 X110.504 Y138.297 E.21577
G1 X110.504 Y137.764
G1 X104.794 Y143.474 E.238
G1 X104.26 Y143.474
G1 X110.504 Y137.231 E.26023
G1 X110.504 Y136.697
G1 X103.727 Y143.474 E.28245
G1 X103.194 Y143.474
G1 X110.504 Y136.164 E.30468
G1 X110.504 Y135.631
G1 X102.661 Y143.474 E.32691
G1 X102.127 Y143.474
G1 X110.504 Y135.098 E.34913
G1 X110.504 Y134.564
G1 X101.596 Y143.472 E.37127
G1 X101.338 Y143.196
G1 X110.504 Y134.031 E.38202
G1 X110.504 Y133.498
G1 X101.338 Y142.663 E.38202
G1 X101.338 Y142.13
G1 X114.85 Y128.619 E.56317
G1 X114.316 Y128.619
G1 X101.338 Y141.597 E.54094
G1 X101.338 Y141.063
G1 X113.783 Y128.619 E.51871
G1 X113.25 Y128.619
G1 X101.338 Y140.53 E.49648
G1 X101.338 Y139.997
G1 X112.716 Y128.619 E.47425
G1 X112.183 Y128.619
G1 X101.338 Y139.464 E.45201
G1 X101.338 Y138.93
G1 X111.65 Y128.619 E.42978
G1 X111.116 Y128.619
G1 X101.338 Y138.397 E.40755
G1 X101.338 Y137.864
G1 X110.583 Y128.619 E.38532
G1 X110.05 Y128.619
G1 X101.338 Y137.331 E.36309
M73 P94 R0
G1 X101.338 Y136.797
G1 X109.516 Y128.62 E.34086
G1 X108.983 Y128.62
G1 X101.338 Y136.264 E.31863
G1 X101.338 Y135.731
G1 X108.45 Y128.62 E.2964
G1 X107.916 Y128.62
G1 X101.338 Y135.198 E.27417
G1 X101.338 Y134.664
G1 X107.383 Y128.62 E.25194
G1 X106.85 Y128.62
G1 X101.338 Y134.131 E.22971
G1 X101.338 Y133.598
G1 X106.316 Y128.62 E.20748
G1 X105.783 Y128.62
G1 X100.929 Y133.474 E.20231
G1 X100.396 Y133.474
G1 X105.249 Y128.62 E.20231
G1 X104.716 Y128.62
G1 X99.862 Y133.474 E.2023
; WIPE_START
M204 S6000
G1 X100.57 Y132.767 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.917 J.8 P1  F42000
G1 X104.183 Y128.62 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X90.338 Y142.465 E.57705
G1 X90.338 Y141.932
G1 X103.649 Y128.621 E.55482
G1 X103.116 Y128.621
G1 X90.338 Y141.398 E.53259
G1 X90.338 Y140.865
G1 X102.583 Y128.621 E.51036
G1 X102.049 Y128.621
G1 X90.338 Y140.332 E.48813
G1 X90.338 Y139.799
G1 X101.516 Y128.621 E.4659
G1 X100.983 Y128.621
G1 X90.338 Y139.265 E.44366
G1 X90.338 Y138.732
G1 X100.449 Y128.621 E.42143
G1 X99.916 Y128.621
G1 X90.338 Y138.199 E.3992
G1 X90.338 Y137.665
G1 X99.383 Y128.621 E.37697
G1 X98.849 Y128.621
G1 X90.338 Y137.132 E.35474
G1 X90.338 Y136.599
G1 X98.316 Y128.621 E.33251
G1 X97.783 Y128.622
G1 X90.338 Y136.066 E.31028
G1 X90.338 Y135.532
G1 X97.249 Y128.622 E.28805
G1 X96.716 Y128.622
G1 X90.338 Y134.999 E.26582
G1 X90.338 Y134.466
G1 X96.183 Y128.622 E.24359
G1 X95.649 Y128.622
G1 X90.338 Y133.933 E.22136
G1 X90.264 Y133.474
G1 X95.116 Y128.622 E.20224
G1 X94.582 Y128.622
G1 X89.731 Y133.474 E.20223
G1 X89.197 Y133.474
G1 X94.049 Y128.622 E.20223
G1 X93.516 Y128.622
G1 X88.664 Y133.474 E.20222
; WIPE_START
M204 S6000
G1 X89.371 Y132.767 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
M73 P95 R0
G3 Z.8 I.918 J.799 P1  F42000
G1 X92.982 Y128.622 Z.8
G1 Z.4
G1 E.4 F1800
G1 F10342.643
M204 S2000
G1 X79.338 Y142.266 E.5687
G1 X79.338 Y141.733
G1 X92.449 Y128.622 E.54647
G1 X91.916 Y128.623
G1 X79.338 Y141.2 E.52424
G1 X79.338 Y140.667
G1 X91.382 Y128.623 E.50201
G1 X90.849 Y128.623
G1 X79.338 Y140.133 E.47977
G1 X79.338 Y139.6
G1 X90.316 Y128.623 E.45754
G1 X89.782 Y128.623
G1 X79.338 Y139.067 E.43531
G1 X79.338 Y138.534
G1 X89.249 Y128.623 E.41308
G1 X88.716 Y128.623
G1 X79.338 Y138 E.39085
G1 X79.338 Y137.467
G1 X88.182 Y128.623 E.36862
G1 X87.649 Y128.623
G1 X79.338 Y136.934 E.34639
G1 X79.338 Y136.401
G1 X87.116 Y128.623 E.32416
G1 X86.582 Y128.624
G1 X79.338 Y135.867 E.30193
G1 X79.338 Y135.334
G1 X86.049 Y128.624 E.2797
G1 X85.516 Y128.624
G1 X79.338 Y134.801 E.25747
G1 X79.338 Y134.268
G1 X84.982 Y128.624 E.23524
G1 X84.449 Y128.624
G1 X79.338 Y133.734 E.21301
G1 X79.065 Y133.474
G1 X83.915 Y128.624 E.20215
G1 X83.382 Y128.624
G1 X78.532 Y133.474 E.20215
G1 X77.999 Y133.474
G1 X82.849 Y128.624 E.20215
G1 X82.315 Y128.624
G1 X68.341 Y142.598 E.58245
G1 X68.344 Y142.062
G1 X81.782 Y128.624 E.56011
G1 X81.249 Y128.624
G1 X68.347 Y141.527 E.53778
G1 X68.349 Y140.991
G1 X80.715 Y128.625 E.51544
G1 X80.182 Y128.625
G1 X68.352 Y140.455 E.4931
G1 X68.354 Y139.919
G1 X79.649 Y128.625 E.47077
G1 X79.115 Y128.625
G1 X68.357 Y139.383 E.44843
G1 X68.359 Y138.848
G1 X78.582 Y128.625 E.42609
G1 X78.049 Y128.625
G1 X68.362 Y138.312 E.40375
G1 X68.364 Y137.776
G1 X77.515 Y128.625 E.38142
G1 X76.982 Y128.625
G1 X68.367 Y137.24 E.35908
G1 X68.37 Y136.704
G1 X76.449 Y128.625 E.33674
G1 X75.915 Y128.625
G1 X68.372 Y136.169 E.31441
G1 X68.375 Y135.633
G1 X75.382 Y128.625 E.29207
G1 X74.849 Y128.626
G1 X68.377 Y135.097 E.26973
G1 X68.38 Y134.561
M73 P96 R0
G1 X74.315 Y128.626 E.2474
G1 X73.782 Y128.626
G1 X68.382 Y134.025 E.22506
G1 X68.385 Y133.489
G1 X73.248 Y128.626 E.20272
G1 X72.715 Y128.626
G1 X68.387 Y132.954 E.18039
G1 X68.39 Y132.418
G1 X72.182 Y128.626 E.15805
G1 X71.648 Y128.626
G1 X68.392 Y131.882 E.13571
G1 X68.395 Y131.346
G1 X71.115 Y128.626 E.11338
G1 X70.582 Y128.626
G1 X68.398 Y130.81 E.09104
G1 X68.4 Y130.275
G1 X70.048 Y128.626 E.0687
G1 X69.515 Y128.626
G1 X68.403 Y129.739 E.04637
G1 X68.405 Y129.203
G1 X68.982 Y128.627 E.02403
M204 S10000
G1 X68.648 Y128.608 F42000
; FEATURE: Gap infill
; LINE_WIDTH: 0.208901
G1 F15000
M204 S6000
G2 X68.388 Y128.929 I.207 J.434 E.00555
; WIPE_START
G1 X68.502 Y128.711 E-.21983
G1 X68.648 Y128.608 E-.16017
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.52 J1.1 P1  F42000
G1 X99.523 Y143.193 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.161457
G1 F15000
M204 S6000
G3 X99.223 Y143.493 I-.59 J-.29 E.004
; WIPE_START
G1 X99.401 Y143.364 E-.19481
G1 X99.523 Y143.193 E-.18519
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.005 J1.217 P1  F42000
G1 X121.523 Y143.109 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.259137
G1 F15000
M204 S6000
G3 X121.123 Y143.493 I-.741 J-.372 E.00955
; WIPE_START
G1 X121.372 Y143.322 E-.20397
G1 X121.523 Y143.109 E-.17603
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I1.217 J.012 P1  F42000
G1 X121.617 Y133.365 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.110695
G1 F15000
M204 S6000
G1 X121.394 Y133.587 E.00167
; WIPE_START
G1 X121.617 Y133.365 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.135 J1.209 P1  F42000
G1 X123.418 Y133.566 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.102751
G1 F15000
M204 S6000
G1 X123.347 Y133.68 E.00063
; LINE_WIDTH: 0.0767504
G1 X123.32 Y133.666 E.00008
; WIPE_START
G1 X123.347 Y133.68 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.879 J.841 P1  F42000
G1 X132.504 Y143.248 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.112027
G1 F15000
M204 S6000
G3 X132.278 Y143.474 I-.39 J-.164 E.00177
; WIPE_START
G1 X132.42 Y143.377 E-.2005
G1 X132.504 Y143.248 E-.1795
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.089 J1.214 P1  F42000
G1 X134.583 Y143.401 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.12849
G1 F15000
M204 S6000
G3 X134.408 Y143.371 I-.07 J-.117 E.00129
; LINE_WIDTH: 0.101697
G1 X134.348 Y143.287 E.00047
; WIPE_START
G1 X134.408 Y143.371 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.06 J1.215 P1  F42000
G1 X143.523 Y142.92 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.348466
G1 F12764.046
M204 S6000
G1 X143.367 Y143.243 E.00856
G3 X143.291 Y143.329 I-.225 J-.124 E.00275
G1 X143.007 Y143.493 E.00783
; WIPE_START
G1 X143.291 Y143.329 E-.15565
G1 X143.367 Y143.243 E-.05416
G1 X143.523 Y142.92 E-.17019
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.025 J1.217 P1  F42000
G1 X154.523 Y143.147 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.217954
G1 F15000
M204 S6000
G3 X154.213 Y143.493 I-.912 J-.506 E.0064
; WIPE_START
G1 X154.389 Y143.335 E-.19225
G1 X154.523 Y143.147 E-.18775
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.01 J1.217 P1  F42000
G1 X176.523 Y142.974 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.286347
G1 F15000
M204 S6000
G1 X176.39 Y143.25 E.00583
; LINE_WIDTH: 0.318447
G1 F14154.723
G1 X176.351 Y143.305 E.00145
G1 X176.14 Y143.493 E.00608
; WIPE_START
G1 X176.351 Y143.305 E-.3068
G1 X176.39 Y143.25 E-.0732
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I1.216 J.034 P1  F42000
G1 X176.666 Y133.488 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.16761
G1 F15000
M204 S6000
G1 X176.518 Y133.43 E.00155
G1 X176.46 Y133.489 E.00081
G1 X176.518 Y133.636 E.00155
; WIPE_START
G1 X176.46 Y133.489 E-.15062
G1 X176.518 Y133.43 E-.07876
G1 X176.666 Y133.488 E-.15062
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.811 J.908 P1  F42000
G1 X187.523 Y143.186 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.170474
G1 F15000
M204 S6000
G3 X187.215 Y143.493 I-.603 J-.296 E.00441
; WIPE_START
G1 X187.391 Y143.368 E-.18604
G1 X187.523 Y143.186 E-.19396
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I.792 J-.924 P1  F42000
G1 X170.548 Y128.646 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.125275
G1 F15000
M204 S6000
G1 X170.326 Y128.646 E.00143
; WIPE_START
G1 X170.548 Y128.646 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I0 J-1.217 P1  F42000
G1 X161.479 Y128.649 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.134053
G1 F15000
M204 S6000
G1 X161.266 Y128.649 E.00152
; WIPE_START
G1 X161.479 Y128.649 E-.38
; WIPE_END
G1 E-.02 F1800
M204 S10000
G17
G3 Z.8 I-.035 J-1.216 P1  F42000
G1 X130.583 Y129.537 Z.8
G1 Z.4
G1 E.4 F1800
; LINE_WIDTH: 0.217737
G1 F15000
M204 S6000
G1 X130.445 Y129.702 E.00294
; LINE_WIDTH: 0.177249
G3 X129.908 Y130.265 I-7.375 J-6.491 E.00817
; LINE_WIDTH: 0.188513
G1 X129.744 Y130.403 E.00245
; LINE_WIDTH: 0.217719
G1 X129.579 Y130.541 E.00294
; close powerlost recovery
M1003 S0
; WIPE_START
G1 F15000
M73 P97 R0
G1 X129.744 Y130.403 E-.38
; WIPE_END
G1 E-.02 F1800
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
G1 X0 Y136.027 F18000 ; move to safe pos
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

