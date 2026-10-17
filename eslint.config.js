import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

/* 两条"语法禁区"各自成组，下面按目录拼装：
 *   客户端（`src/app` + 共用件 `src/components`）**两条都要**；
 *   工作台（`src/workbench`，内部工具）只要内联 overflow 那条。 */

/* 内联 style 是 Stylelint 的盲区，overflow 一律写进 CSS Module */
const INLINE_OVERFLOW = [
  {
    selector: "JSXAttribute[name.name='style'] Property[key.name=/^overflow/]",
    message: 'overflow 请写在 CSS Module 里，内联样式会绕过 Stylelint 的滚动条检查',
  },
  {
    selector: "JSXAttribute[name.name='style'] Property[key.value=/^overflow/]",
    message: 'overflow 请写在 CSS Module 里，内联样式会绕过 Stylelint 的滚动条检查',
  },
]

/*
 * 勾选 / 单选一律走共用件 `components/check`（2026-10-17 作者：「我发现我好多地方的
 * 按钮样子不一样…主要是**勾和框统一，圆角，勾的位置**之类的」之后拍的口径）。
 *
 * 为什么用 eslint 硬拦（而不是只靠注释约定）：原生 `<input type=checkbox|radio>` 会被
 * 系统画成"另一套样子"（框、圆角、勾的画法全不由我们定，还会吃 `tokens.css` 里那条
 * 全站 `accent-color` 的印刷机绿）—— 正是这一刀要收掉的那批。写注释挡不住下一个人手写。
 *
 * ★ 只认**字面量** `type="checkbox" / type="radio"`：共用件自己那只是
 *   `type={isRadio ? 'radio' : 'checkbox'}`（表达式），天然不被命中，不用给自己开后门。
 * ★ 预设页那个分段控件（`PresetScopeBar`）的 `type="radio"` 是**刻意藏圆点**的
 *   例外，就地 `eslint-disable` 并写明原因 —— 例外要看得见，不是悄悄放行。
 */
const NATIVE_CHOICE = [
  {
    selector:
      "JSXOpeningElement[name.name='input'] JSXAttribute[name.name='type'][value.value='checkbox']",
    message:
      '勾选 / 单选一律用共用件 `components/check` 的 `Check`，不要写原生 input[type=checkbox]（见该件 check.module.css 头注释）',
  },
  {
    selector:
      "JSXOpeningElement[name.name='input'] JSXAttribute[name.name='type'][value.value='radio']",
    message:
      '勾选 / 单选一律用共用件 `components/check` 的 `Check`，不要写原生 input[type=radio]（见该件 check.module.css 头注释）',
  },
]

export default tseslint.config(
  /* 构建产物一律不扫。
     target 是 `tauri build` 之后才出现的 —— 里面有 tauri-codegen 生成的资产 .js
     （压缩过的二进制字节），eslint 会在那上面报 Parsing error。本地一跑 build 就红，
     CI 上因为 target 不入库反而看不见，是个只在本机出现的假故障。

     建了 workspace（b04 Task 13）之后产物目录搬到了仓库根，所以几个都要列：
     根上的 `target` 是新的，`src-tauri/target` 是切换之前留下的，
     `target-workbench` 是后厨工作台那一份（CARGO_TARGET_DIR，与客户端 target 分开）。 */
  {
    ignores: [
      'dist',
      'node_modules',
      'target',
      'target-workbench',
      'src-tauri/target',
      'src-tauri/gen',
    ],
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      'no-restricted-syntax': ['error', ...INLINE_OVERFLOW],
    },
  },
  /*
   * 客户端这一侧**多一条**：勾选 / 单选必须走共用件（见上面 `NATIVE_CHOICE`）。
   * 工作台（`src/workbench`）不在这块里 —— 它是内部工具，有自己那套控件语言。
   *
   * ★ 这里必须把 `INLINE_OVERFLOW` 一起带上：flat config 里后一块会把同一条规则
   *   **整条覆盖**掉，漏带就等于把上面那条 overflow 检查从客户端删了。
   */
  {
    files: ['src/app/**/*.{ts,tsx}', 'src/components/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-syntax': ['error', ...INLINE_OVERFLOW, ...NATIVE_CHOICE],
    },
  }
)
