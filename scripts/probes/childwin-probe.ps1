# 验证：那两个 60x60 缩略图是不是真实子窗口（EnumChildWindows 能枚举到、GetWindowRect 能读到）
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class ChildEnum {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumProc cb, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr FindWindowW(string cls, string title);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    public static List<string> Rows = new List<string>();
    public static IntPtr Parent;
    public static int PL, PT;
    public static bool Cb(IntPtr h, IntPtr l) {
        var sb = new System.Text.StringBuilder(256);
        GetClassName(h, sb, 256);
        RECT r; GetWindowRect(h, out r);
        int w = r.R - r.L, hh = r.B - r.T;
        // 只记"方形且 40~120 一族"的候选 + 总数
        if (Math.Abs(w - hh) <= 2 && w >= 40 && w <= 120) {
            Rows.Add(string.Format("hwnd={0} class='{1}' rel=({2},{3}) size={4}x{5}",
                h, sb, r.L - PL, r.T - PT, w, hh));
        }
        return true;
    }
}
"@
[ChildEnum]::SetProcessDPIAware() | Out-Null

# 三个 pid 的主窗口都要；按标题找（FindWindowW 要精确标题，所以改用 EnumWindows 按 pid 找）
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class TopEnum {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    public static List<IntPtr> MainWindowsOf(uint target) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint pid; GetWindowThreadProcessId(h, out pid);
            if (pid == target && IsWindowVisible(h)) {
                var t = new StringBuilder(256); GetWindowText(h, t, 256);
                var c = new StringBuilder(256); GetClassName(h, c, 256);
                if (t.Length > 0) found.Add(h);
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

foreach ($p in 37056, 45052, 51096) {
    foreach ($h in [TopEnum]::MainWindowsOf([uint32]$p)) {
        $t = New-Object System.Text.StringBuilder 256
        [TopEnum]::GetWindowText($h, $t, 256) | Out-Null
        Write-Output ("pid {0} hwnd {1} '{2}'" -f $p, $h, $t)
        [ChildEnum]::Rows.Clear()
        # 读父窗口矩形（GetWindowRect on parent）
        $r = New-Object ChildEnum+RECT
        [ChildEnum]::GetWindowRect($h, [ref]$r) | Out-Null
        [ChildEnum]::PL = $r.L; [ChildEnum]::PT = $r.T
        [ChildEnum]::EnumChildWindows($h, [ChildEnum+EnumProc]{ param($a,$b) [ChildEnum]::Cb($a,$b) }, [IntPtr]::Zero) | Out-Null
        Write-Output ("  方形候选 {0} 个:" -f [ChildEnum]::Rows.Count)
        [ChildEnum]::Rows | ForEach-Object { Write-Output ("    {0}" -f $_) }
    }
}
