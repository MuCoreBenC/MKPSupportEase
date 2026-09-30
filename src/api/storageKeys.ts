/**
 * 客户端与工作台用到的 localStorage 键，**全部收在这一处**。
 *
 * 作者还没定这套名字（方案 §6-12），所以先集中：将来改名是把这里改掉，
 * 不是去页面里翻。
 *
 * 两条要注意的：
 *
 * 1. **`cloud` 那一格必须是同一个键、而且不带端名** —— 工作台「上传」写它、
 *    客户端「同步」读它，联动就靠这一格。名字本身就是那条管道的名字，
 *    不是某一稿或某一端的偏好（C15 的 `store/cloud.ts` 里写着这层道理）。
 * 2. 客户端自己的三格沿用试验场 A40 的原名（`mkp.a40.*`）——
 *    这一轮先做到「行为与试验场一致」，改名等作者定下来。
 */
export const STORAGE = {
  /** 模拟云端：工作台「上传」写它，客户端「同步」页读它 */
  cloud: 'mkp.cloud.presets',

  /** 客户端同步下来的说明书（`ClientDataPackage`）—— 自动同步，界面只说「上次同步 / 已是最新」 */
  clientPackage: 'mkp.a40.package',

  /** 客户端「获取预设」拿到的那些 TOML（本机预设） */
  clientPresets: 'mkp.a40.presets',

  /** 客户端「使用这一份」记的那一条（**全局唯一**的当前使用） */
  clientActive: 'mkp.a40.active',

  /**
   * 预设页的「置顶」集合（一串 pinKey，JSON 数组）。**纯前端的排序偏好**，
   * 不是底账 —— 它只影响那一张表的先后。
   *
   * 与上面三格一样：值沿用原来的、一个字不改，名字还没定，所以先收到这里。
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
