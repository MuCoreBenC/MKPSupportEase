import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

export default tseslint.config(
  { ignores: ['dist', 'node_modules'] },
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
