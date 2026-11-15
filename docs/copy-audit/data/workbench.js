/*
 * 工作台文案审查 · 数据入口
 *
 * 后面的 wb-*.js 往 items 里 push；顺序 = 工作台一级导航顺序（外壳 → 机型与版本 →
 * 参数台 → 参数说明 → 套餐 → 资产库 → 标注台 → 生成与发布 → 设置 → 公共组件）。
 *
 * 收录口径：
 *   · 只收界面上真的会显示的字（字符串字面量与 JSX 文本），不含代码注释；
 *   · 不含多语言（作者：这一轮不管多国语言）；
 *   · 不含开发台 / mock 演示数据（只给开发看）；
 *   · 参数名与参数说明来自 presets/registry/param_registry.toml，按参数台页签顺序排。
 */
window.COPY_AUDIT_DATA = {
  meta: {
    title: '工作台文案审查台',
    sub: 'SupportEase 工作台 · 按一级导航顺序 · 2026-11-14',
    dataset: 'workbench-2026-11-14',
    storageKey: 'mkp.copyaudit.workbench.2026-11-14',
    howto: `
      <p><b>这一轮的范围</b>：工作台六个业务页 + 外壳 + 公共组件，以及参数台里那 96 条参数名称与说明。
      不含多语言（繁 / 日 / 英），也不含开发台与 mock 演示。</p>
      <p><b>「建议留」</b>那些不是漏改 —— 现文案已经合格，列出来是让你看一眼有没有异议；
      一次可以整页「同意」把它们放过去。<b>「布局」</b>那几条不该靠改字解决，另开布局轮。</p>
      <p><b>落地</b>：批完导出 Markdown，我照着改。界面文案在 <code>src/workbench/**</code>；
      参数名称与说明在 <code>presets/registry/param_registry.toml</code>（只改中文，其余语言
      会被记成「待重写」，跟现在工作台的处理一致）。</p>
    `,
  },
  pages: [
    { id: 'shell', label: '外壳' },
    { id: 'machines', label: '机型与版本' },
    { id: 'params', label: '参数台' },
    { id: 'registry', label: '参数台 · 名称与说明' },
    { id: 'bundles', label: '套餐' },
    { id: 'assets', label: '资产库' },
    { id: 'annotate', label: '标注台' },
    { id: 'build', label: '生成与发布' },
    { id: 'settings', label: '设置' },
    { id: 'kit', label: '公共组件' },
  ],
  items: [],
}
