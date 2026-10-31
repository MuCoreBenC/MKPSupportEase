/*
 * 前端单元测试（2026-10-31 起）。
 *
 * 只测**纯函数**：多语言挑选、格式化、解析这类"进一个值出一个值"的东西。
 * 渲染层不在这里 —— 工作台的验收靠真机（作者跑 `npm run tauri dev`）。
 *
 * 环境用 node 而不是 jsdom：这些函数不碰 DOM，装了 jsdom 只会让"这个测试跑得慢"
 * 变成习惯。真要测组件再单独加一个 environment 为 jsdom 的项目。
 */
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
    environment: 'node',
    /*
     * 与产品代码同一条路径别名（`vite.config.ts` 里只声明了根目录的那些）。
     * 测试里写相对路径也能跑，但一旦用例挪位置就得改一堆 `../..`。
     */
    alias: [{ find: /^@\/(.*)$/, replacement: '/src/$1' }],
  },
})
