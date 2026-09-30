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
} as const

export type StorageKey = (typeof STORAGE)[keyof typeof STORAGE]
