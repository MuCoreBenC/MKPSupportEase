; HEADER_BLOCK_START
; BambuStudio 02.08.02.60
; model printing time: 0m 30s; total estimated time: 5m 0s
; total layer number: 2
; max_z_height: 0.40
; filament: 1
; HEADER_BLOCK_END

; CONFIG_BLOCK_START
; printable_area = 0x0,256x0,256x256,0x256
; printer_model = Bambu Lab A1
; CONFIG_BLOCK_END

; 开发台「挪塔碰撞」档：**必须撞**的那一份。
;
; 两个条件缺一不可：
;   ① `; FEATURE: Support` —— 让擦料决策判「这一趟需要擦料」（走 tower 档）。
;      少了这一行 ⇒ `wiping.needed=false` ⇒ `tower.use_towers=false`
;      ⇒ `detect_tower_collision` 第一行就 return，整趟安静跑完（实测 54ms 成功，不问）。
;      这就是 `rib-tall-min.gcode` 单独跑不出碰撞的原因。
;   ② 模型点落在**擦料塔当前框里**：A1 的 wiper(20,20) + brim local [2.96,37.04]²
;      ⇒ machine 22.96..57.04。所以下面是 X30 Y30 一带。
;
; 期望：第 6 步（Collision）弹「自动挪塔」那一问，`_meta.json.relocation` 非 null。
; 与 `crates/postprocess/tests/pipeline_tower_ask.rs` 是同一套几何（那边手搭 IR，这边走真入口）。
; FEATURE: Support
G90
G1 Z0.2
G1 X30 Y30 E0.5
G1 X120 Y120 E0.6
G1 X140 Y60 E0.7
