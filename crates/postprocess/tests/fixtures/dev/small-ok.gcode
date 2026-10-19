; HEADER_BLOCK_START
; BambuStudio 02.08.02.60
; model printing time: 0m 12s; total estimated time: 5m 0s
; total layer number: 2
; max_z_height: 0.40
; filament: 1
; HEADER_BLOCK_END

; CONFIG_BLOCK_START
; printable_area = 0x0,256x0,256x256,0x256
; printer_model = Bambu Lab A1
; CONFIG_BLOCK_END

; 开发台「小文件成功」档：只有一小片模型，几十毫秒跑完。
; 用途 = 快速回归（管线能跑通、记录能落、客户端能收到），不是看进度。
; ★ 机型头（printer_model）必须在：缺了它钩子会硬失败，那一档是 no-machine.gcode。
G90
G1 Z0.2
G1 X100 Y100 E0.5
G1 X110 Y100 E0.6
G1 X110 Y110 E0.7
G1 Z0.4
G1 X100 Y110 E0.8
G1 X100 Y100 E0.9
