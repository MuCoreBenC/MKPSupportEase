/**
 * 后处理脚本 —— 贴进切片器「后处理脚本」那一栏的命令行。
 *
 * 这一版是写死的示例值（取自试验场的实测截图），路径里带着当时那台机器的安装位置。
 * 真实情况下它由三部分拼出来：可执行文件的实际路径 + 当前预设的 toml 路径 + `--Gcode`，
 * 所以 Rust 侧起来之后这条会变成后端算出来的值（它才知道自己装在哪、预设在哪）。
 * 放在 constants 里是因为界面现在需要一个能复制的字符串，不是因为它该长期写死。
 */
export const postProcessScript =
  '"G:\\project\\mkp-ssr\\target\\debug\\mkp-ssr.exe" --Toml "C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF_260628.toml" --Gcode'
