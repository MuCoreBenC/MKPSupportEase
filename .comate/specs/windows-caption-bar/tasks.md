# Windows 标题栏窗口键：接上真实动作 + 改成撑满式

- [x] Task 1: 切分支，脱离 main
    - 1.1: `git switch -c fix/win-caption-buttons`，闸⑥要求合法前缀

- [x] Task 2: 补齐 Tauri ACL 权限，让三颗键真正能动
    - 2.1: 编辑 `src-tauri/capabilities/default.json`，`permissions` 数组增加 `core:window:allow-minimize`、`allow-maximize`、`allow-unmaximize`、`allow-toggle-maximize`、`allow-close` 五条
    - 2.2: 更新 `description` 字段，补充窗口动作权限的说明

- [x] Task 3: Icon 组件增加 `restore`（还原）图标
    - 3.1: `src/components/Icon.tsx` 的 `IconName` 联合类型增加 `'restore'`
    - 3.2: `paths` 对象增加 `restore` 条目（双叠框路径）

- [x] Task 4: `window.ts` 导出 `inTauri` 常量
    - 4.1: 把 `const inTauri` 改为 `export const inTauri`，供新 hook 复用

- [x] Task 5: 新增 `useWindowMaximized` hook
    - 5.1: 创建 `src/hooks/useWindowMaximized.ts`
    - 5.2: 实现订阅 `onResized` → 查询 `isMaximized` → 返回 `boolean` 的逻辑
    - 5.3: 浏览器路径下（`inTauri` 为 false）直接返回 false，不走 IPC

- [x] Task 6: 改写 `TopTabs.tsx` 窗口键区域
    - 6.1: 导入 `useWindowMaximized`，在组件顶部无条件调用
    - 6.2: 去掉 `.controls` 容器上的 `data-tauri-drag-region="deep"`
    - 6.3: 最大化键按 `maximized` 状态切换图标（`max` / `restore`）与 `aria-label`
    - 6.4: 三颗键图标统一收到 `size={11} strokeWidth={1.5}`

- [x] Task 7: 重写 `TopTabs.module.css` 窗口键样式为撑满式
    - 7.1: 新增 `.bar[data-platform='windows'] { padding-right: 0 }`
    - 7.2: 重写 `.controls`：去掉 `gap`/`align-items`，改为 `align-self: stretch`
    - 7.3: 重写 `.ctrl`：宽 `var(--winctrl-w, 46px)`、高 `100%`、去掉 `border-radius`
    - 7.4: 新增 `.bar[data-density='mini'] .controls { --winctrl-w: 40px }`
    - 7.5: `.ctrl:focus-visible { outline-offset: -2px }` 覆盖全局外扩焦点环
    - 7.6: 更新 hover/active 底色（普通键 `rgb(16 24 32 / 6%)` / `11%`，关闭键 `#e5484d` / `#cf3c41`）

- [x] Task 8: 验证——lint + build
    - 8.1: 运行 `npm run lint`，确认 eslint + stylelint 全绿
    - 8.2: 运行 `npm run build`，确认 `tsc -b` + vite build 通过

- [x] Task 9: 更新 `docs/ARCHITECTURE.md` 过期描述
    - 9.1: §9 删掉「标题栏那三颗窗口按钮还是装饰」那两句过期文字，替换为当前真实状态
