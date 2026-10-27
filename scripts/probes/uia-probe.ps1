# 屏幕引导 Windows 可行性探针（一次性，跑完看结果）：
# 1) 找 Bambu Studio 的主窗口（句柄/矩形/标题）
# 2) 走 UIA 控件树，找「约 50x50 且相对窗口 y 在 100~140」的缩略图候选
# 3) 顺带统计整棵树有多少元素、都什么类型 —— 判断 Qt 的树暴露到什么程度
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32P {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

# 让本进程 DPI 感知，GetWindowRect 才给物理像素（与 UIA 的 BoundingRectangle 同一套）
Add-Type @"
using System.Runtime.InteropServices;
public class DpiAware { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[DpiAware]::SetProcessDPIAware() | Out-Null

$root = [System.Windows.Automation.AutomationElement]::RootElement
$pids = 37056, 45052, 51096
foreach ($p in $pids) {
    Write-Output "=== pid $p ==="
    $cond = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p)
    $wins = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
    foreach ($w in $wins) {
        $h = [IntPtr]$w.Current.NativeWindowHandle
        $r = New-Object Win32P+RECT
        [Win32P]::GetWindowRect($h, [ref]$r) | Out-Null
        $vis = [Win32P]::IsWindowVisible($h)
        $mini = [Win32P]::IsIconic($h)
        Write-Output ("窗口: name='{0}' class='{1}' hwnd={2} visible={3} iconic={4} rect=({5},{6})-({7},{8})" -f `
            $w.Current.Name, $w.Current.ClassName, $h, $vis, $mini, $r.L, $r.T, $r.R, $r.B)
        if (-not $vis -or $mini) { continue }

        # 遍历整棵树
        $all = $w.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        Write-Output ("后代元素总数: {0}" -f $all.Count)
        $ww = $r.R - $r.L; $wh = $r.B - $r.T
        $cands = 0
        foreach ($e in $all) {
            $b = $e.Current.BoundingRectangle
            if ($b.Width -le 0 -and $b.Height -le 0) { continue }
            $rw = $b.X - $r.L; $ry = $b.Y - $r.T
            $side = [Math]::Min($b.Width, $b.Height)
            $sq = [Math]::Abs($b.Width - $b.Height) -le 2
            # 候选①：50x50 一族（macOS 判据，物理像素下可能是 100x100 等，按比例也搜）
            $cand1 = $sq -and ($side -ge 40 -and $side -le 120) -and ($ry -ge 60 -and $ry -le 260)
            if ($cand1 -and $cands -lt 25) {
                $cands++
                Write-Output ("  候选: type={0} name='{1}' class='{2}' rel=({3},{4}) size={5}x{6}" -f `
                    $e.Current.ControlType.ProgrammaticName, $e.Current.Name, $e.Current.ClassName, `
                    [int]$rw, [int]$ry, [int]$b.Width, [int]$b.Height)
            }
        }
        # 类型分布（前 15）
        $all | Group-Object { $_.Current.ControlType.ProgrammaticName } |
            Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object {
                Write-Output ("  类型分布: {0} x{1}" -f $_.Name, $_.Count)
            }
    }
}
