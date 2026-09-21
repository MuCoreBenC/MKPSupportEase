import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

export default tseslint.config(
  /* 构建产物一律不扫。
     src-tauri/target 是 `tauri build` 之后才出现的 —— 里面有 tauri-codegen 生成的资产 .js
     （压缩过的二进制字节），eslint 会在那上面报 Parsing error。本地一跑 build 就红，
     CI 上因为 target 不入库反而看不见，是个只在本机出现的假故障。 */
  { ignores: ['dist', 'node_modules', 'src-tauri/target', 'src-tauri/gen'] },
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
      // 内联 style 是 Stylelint 的盲区，overflow 一律写进 CSS Module
      'no-restricted-syntax': [
        'error',
        {
          selector: "JSXAttribute[name.name='style'] Property[key.name=/^overflow/]",
          message: 'overflow 请写在 CSS Module 里，内联样式会绕过 Stylelint 的滚动条检查',
        },
        {
          selector: "JSXAttribute[name.name='style'] Property[key.value=/^overflow/]",
          message: 'overflow 请写在 CSS Module 里，内联样式会绕过 Stylelint 的滚动条检查',
        },
      ],
    },
  }
)
