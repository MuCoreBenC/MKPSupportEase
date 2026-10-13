/**
 * 客户端与工作台用到的 localStorage 键，**全部收在这一处**。
 *
 * 作者还没定这套名字（方案 §6-12），所以先集中：将来改名是把这里改掉，
 * 不是去页面里翻。
 *
 * C4 收口后的现状：**三格底账（说明书 / 本机预设 / 使用中）已经退役** ——
 * 它们的对应物住在新世界（catalog / `mkp/` / `run/active-preset.json`，
 * 全部走 Rust 侧 `atomic_write`），localStorage 不再承载任何底账。
 * 留在这里的**全是纯前端偏好**（排序 / 搜索词 / 抽屉宽度 / 显示模式）。
 *
 * `cloud` 那格（模拟云端，只服务工作台那个 `ClientDataPackage` 演示管道）
 * 2026-10-04 随 `ClientDataPackage` 一并退役 —— 真实发布契约是 `catalog.path` + Git，
 * 不再有「假的云端数据结构」这一格。
 */
export const STORAGE = {
  /**
   * 预设页的「置顶」集合（一串 pinKey，JSON 数组）。**纯前端的排序偏好**，
   * 不是底账 —— 它只影响那一张表的先后。
   */
  clientPresetsPinned: 'mkp.A40.presets.pinned',

  /**
   * 参数页的「最近搜索」词表（一串搜索词，JSON 数组）。**纯前端的输入偏好**，
   * 不是底账 —— 它只影响搜索框那个下拉里的几条。
   *
   * 与上面几格一样：值沿用原来的、一个字不改，名字还没定，所以先收到这里。
   */
  clientParamsSearchHistory: 'mkp.A40.params.searchHistory',

  /** 参数页「修改历史」悬浮抽屉的宽度（px，JSON 数字）。同样是纯前端偏好 */
  clientParamsHistoryDrawerW: 'mkp.A40.params.historyDrawerW',

  /**
   * 工艺页（BBS 预设查看器）的偏好：看全部还是跟 BBS 一样、抽屉宽度。
   * （「并排还是浮层」2026-10-13 改成按窗口宽度自动决定，不再落盘。）
   * **全是纯前端的显示偏好** —— 这一页不写盘（本机目录那个端点是只读的），
   * 所以都不承载底账。
   *
   * ★ `clientBbsTheme`（`mkp.A40.bbs.light`）**2026-10-12 退役**：那一页的深浅改成
   * 跟随应用主题（`useIsDark()`），不再自己记一格。用户盘上可能还留着那个旧值 ——
   * 留着不用即可（这是"换落点当成全新应用"那条口径，不写迁移逻辑）。
   */
  clientBbsView: 'mkp.A40.bbs.view',
  clientBbsDrawerW: 'mkp.A40.bbs.drawerW',
  /**
   * 右侧「详情」栏的宽度（px，JSON 数字）。2026-10-13 加：底条整条退场、
   * 那一栏搬到了右边，宽度与左抽屉走**同一套**拖宽手感（`shared/useDrawerWidth`：
   * 拖宽 + 三态吸附 + 键盘 + 记住宽度）。同样是纯前端偏好。
   */
  clientBbsRailW: 'mkp.A40.bbs.railW',
  /* ★ `clientBbsDrawerMode`（`mkp.A40.bbs.drawerMode`）2026-10-13 退役：并排 / 浮层
     改成按窗口宽度自动决定（与报告页同一个判据），旧值留着不用即可（同上口径）。 */

  /** 默认用哪个切片软件打开模型（`system` / `bambu` / `orca`，见 shared/slicer）。纯前端偏好 */
  clientSlicer: 'mkp.A40.slicer',

  /**
   * 报告页导出框里那个「保存位置」（**上一次选过的父目录**）。
   *
   * 纯前端偏好，不是底账：它只决定框打开时路径胶囊里先写哪一串。真正往哪儿写由
   * 后端说了算（目录不在了会照实报"建不出保存位置"），所以记住一个失效的路径不会骗人。
   * 空串 = 还没选过 ⇒ 用后端给的默认值（`<用户根>/exports`，见 `fsx::paths::EXPORTS_DIR`）。
   */
  clientReportExportDir: 'mkp.A40.report.exportDir',
} as const

export type StorageKey = (typeof STORAGE)[keyof typeof STORAGE]
