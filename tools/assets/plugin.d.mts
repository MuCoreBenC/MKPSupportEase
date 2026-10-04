/** 随包资产流水线的 vite 入口：实现在同名 `.mjs` 里（Node 侧代码不进 tsc 那棵树） */
export declare function mkpAssets(options: { workbench: boolean }): import('vite').Plugin

/** 工作台预览的 URL 契约前缀（与后端同值） */
export declare const ASSETS_URL_PREFIX: string

/** 这一刀认的三个位置（台账 / 源根 / 交付根，都相对仓库根） */
export declare const paths: { ledger: string; source: string; output: string }
