/*
 * 浏览器演示用的**目录登记**（`catalog.files[]`）—— 一处事实两个消费者：
 *
 *   1. `mock.ts::getRuntimeCatalog()` 的 `files` 栏（预设页的云端表按它列 MKP / 切片器配置）
 *   2. `files.ts::resolveVersionFiles()` 里那一支 MKP —— **真机就是从目录查的**
 *      （`ipc/presets.rs::version_files_dto` 调 `catalog.file_of(machine, version)`）
 *
 * 2026-10-06 把这一份从 `mock.ts` 挪出来：以前只喂 ①，「这个 combo 的套餐」那一支
 * 用的是上游老命名（`machine_catalog.json` 的 `presetFile` = `A1.toml` / `A1F.toml`）。
 * 两个后果：**同一份文件在一处叫 `A1.toml`、在另一处叫 `A1-standard.toml`**；
 * 而消费端按 `{机型}-{版本小写}.toml` 找文件（`workbench/app/build.rs` §命名），
 * 老命名永远对不上 —— 首页套餐会一律显示「缺」。所以这里收成一份，两支共用。
 */

import type { RuntimeCatalogFile } from '../contract'

/**
 * 一条切片器配置（`bbs_config`）的登记。
 *
 * 9 条只差「文件名 / 落点目录 / 机型 / 字节数」这四列，写成一个函数 —— 手抄九遍
 * 抄错一格比少一条难查得多（那四列就是真机那份 catalog 里的原值）。
 */
function bbsConfig(
  fileName: string,
  dir: string,
  machineId: string,
  size: number,
): RuntimeCatalogFile[] {
  let h = 0
  for (const ch of fileName) h = (h * 31 + ch.charCodeAt(0)) >>> 0
  return [
    {
      kind: 'bbs_config',
      fileName,
      path: `${dir}${fileName}`,
      machineId,
      versionId: '',
      /* 假 sha：由文件名推出来的（同一份恒定、不同份不同）—— 演示数据也要自洽 */
      sha256: h.toString(16).padStart(8, '0').repeat(8),
      size,
    },
  ]
}

export function mockCatalogFiles(): RuntimeCatalogFile[] {
  return [
    {
      kind: 'mkp_preset',
      fileName: 'A1-standard.toml',
      path: 'dist/mkp/presets/A1-standard.toml',
      machineId: 'A1',
      versionId: 'STANDARD',
      sha256: '0'.repeat(64),
      size: 2048,
    },
    {
      kind: 'mkp_preset',
      fileName: 'A1-fast.toml',
      path: 'dist/mkp/presets/A1-fast.toml',
      machineId: 'A1',
      versionId: 'FAST',
      sha256: '1'.repeat(64),
      size: 2048,
    },
    {
      /* 「内容异常」那一档的演示：盘上有它、但与目录对不上，而且哪儿都查不出它是哪一版 */
      kind: 'mkp_preset',
      fileName: 'A1mini-standard.toml',
      path: 'dist/mkp/presets/A1mini-standard.toml',
      machineId: 'A1_MINI',
      versionId: 'STANDARD',
      sha256: '2'.repeat(64),
      size: 2048,
    },
    /*
     * 切片器那一类的交付文件（`bbs_config`）：预设页按 `kind` 把它分流进
     * 「切片器配置 → 云端」——MKP 档**不列它**（2026-10-02 作者截图里混进 MKP 表的就有它）。
     *
     * ★ 2026-10-13 补齐到 **9 条**（文件名 / 落点 / 机型 / 字节数照真机那份
     *   `presets/delivery/catalog.json` 逐条抄）—— 原来是 1 条，只为让预设页那张表有行。
     *   现在它多了一个消费者：**工艺页的「MKP 工艺」档**（那 9 份减去已下载的，
     *   就是"还没下载"那几份灰着列出来的那一批）。条数不实，那一档在预览里就看不到东西。
     */
    ...bbsConfig('MKPProcess A1 0.2 0.10.json', 'assets/bbs/Process/0.2mm/', 'A1', 1308),
    ...bbsConfig('MKPProcess A1 0.4 0.20.json', 'assets/bbs/Process/0.4mm/', 'A1', 1332),
    ...bbsConfig('MKPProcess A1 mini 0.2 0.10.json', 'assets/bbs/Process/0.2mm/', 'A1_MINI', 1315),
    ...bbsConfig('MKPProcess A1 mini 0.4 0.20.json', 'assets/bbs/Process/0.4mm/', 'A1_MINI', 1301),
    ...bbsConfig('MKPProcess P1S 0.2 0.10.json', 'assets/bbs/Process/0.2mm/', 'P1S', 1906),
    ...bbsConfig('MKPProcess P1S 0.4 0.24.json', 'assets/bbs/Process/0.4mm/', 'P1S', 1872),
    ...bbsConfig('MKPProcess P2S 0.4 0.24.json', 'assets/bbs/Process/0.4mm/', 'P2S', 1264),
    ...bbsConfig('MKPProcess X1 0.2 0.10.json', 'assets/bbs/Process/0.2mm/', 'X1C', 1904),
    ...bbsConfig('MKPProcess X1 0.4 0.24.json', 'assets/bbs/Process/0.4mm/', 'X1C', 1925),
    {
      /*
       * 图标（`icon`）：**不归预设页** —— 它是资源，由自己的资源体系消费。
       * 登记在 catalog 里只为钉住一条判据：「登记了」不等于「预设页要显示」
       * （同截图的 `a1.svg`）。
       */
      kind: 'icon',
      fileName: 'a1.svg',
      path: 'assets/icons/a1.svg',
      machineId: 'A1',
      versionId: '',
      sha256: '4'.repeat(64),
      size: 2400,
    },
  ]
}
