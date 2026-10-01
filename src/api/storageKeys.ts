/**
 * 客户端与工作台用到的 localStorage 键，**全部收在这一处**。
 *
 * 作者还没定这套名字（方案 §6-12），所以先集中：将来改名是把这里改掉，
 * 不是去页面里翻。
 *
 * C4 收口后的现状：**三格底账（说明书 / 本机预设 / 使用中）已经退役** ——
 * 它们的对应物住在新世界（catalog / `mkp/` / `run/active-preset.json`，
 * 全部走 Rust 侧 `atomic_write`），localStorage 不再承载任何底账。
 * 留在这里的全是**纯前端偏好**（排序 / 搜索词 / 抽屉宽度 / 显示模式），
 * 加上 `cloud` 那格工作台演示管道（C7 退役时随它走）。
 */
export const STORAGE = {
  /** 模拟云端：工作台「上传」写它，客户端「同步」页读它（演示管道，C7 退役） */
  cloud: 'mkp.cloud.presets',

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
   * BBS 预设页的四格偏好：看全部还是跟 BBS 一样、皮肤深浅、抽屉是并排还是浮层、
   * 抽屉宽度。**全是纯前端的显示偏好** —— 这一页不写盘（本机目录那个端点是只读的），
   * 所以四格都不承载底账。
   */
  clientBbsView: 'mkp.A40.bbs.view',
  clientBbsTheme: 'mkp.A40.bbs.light',
  clientBbsDrawerMode: 'mkp.A40.bbs.drawerMode',
  clientBbsDrawerW: 'mkp.A40.bbs.drawerW',
} as const

export type StorageKey = (typeof STORAGE)[keyof typeof STORAGE]
