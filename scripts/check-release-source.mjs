/**
 * 资源寻址 M6 判据（`docs/RESOURCE-ADDRESSING-ROADMAP.md` §5-M6）：
 *
 *   从 0.0.6 起，`presets/delivery/release.json` 里的下载地址一律来自 **Gitee Release**
 *   （发布仓库），禁止出现 `github.com/.../releases`。
 *
 * v≤0.0.5 是**断代前的历史记录**（GitHub 发的，`asset.url` 指 GitHub 是事实）——
 * 照旧合法，不改写（总纲 §1③：测试版断代，不是抹历史）。
 *
 * ★ 断代线常量与 `src-tauri/src/workbench/app/release_tx.rs` 的
 *   `FIRST_GITEE_RELEASE` 同值 —— 那边拦"正在生成的"，这边查"已入库的"。
 *   改断代线时两边一起改（CI 会在文件里 grep 不到时依旧各自为政……所以写在这条注释里提醒）。
 */

import { readFileSync } from 'node:fs';

const FIRST_GITEE_RELEASE = [0, 0, 6];

/* 两份清单（正式 / 预览）走同一条断代线与同一条判定，不另立一套。
 * 预览清单是可选能力：没这份文件不算错（老交付面本来就没有）。 */
const FILES = [
  ['release.json', new URL('../presets/delivery/release.json', import.meta.url)],
  ['release-preview.json', new URL('../presets/delivery/release-preview.json', import.meta.url)],
];

const versionAtLeastFirstGitee = (v) => {
  const parse = (s) => s.split(/[-+]/)[0].split('.').map((n) => Number.parseInt(n, 10) || 0);
  const [a, b, c] = parse(String(v ?? ''));
  return (
    a > FIRST_GITEE_RELEASE[0] ||
    (a === FIRST_GITEE_RELEASE[0] &&
      (b > FIRST_GITEE_RELEASE[1] ||
        (b === FIRST_GITEE_RELEASE[1] && c >= FIRST_GITEE_RELEASE[2])))
  );
};

let failed = false;
for (const [label, path] of FILES) {
  let text;
  try {
    text = readFileSync(path, 'utf8');
  } catch (e) {
    if (label === 'release-preview.json') {
      console.log(`${label} 不存在（可选预览通道），跳过`);
      continue;
    }
    console.error(`::error::读不了 ${path.pathname}：${e.message}`);
    process.exit(1);
  }
  let info;
  try {
    info = JSON.parse(text);
  } catch (e) {
    console.error(`::error::${label} 不是合法 JSON：${e.message}`);
    process.exit(1);
  }

  const version = String(info.version ?? '');
  if (!versionAtLeastFirstGitee(version)) {
    // 断代前：GitHub 地址是历史事实，放行
    console.log(`${label} ${version}：断代线（0.0.6）之前，历史记录放行`);
    continue;
  }

  const urls = [
    ['release 页', info.url],
    ...(info.asset ? [['安装包', info.asset.url]] : []),
    ...(Array.isArray(info.assets) ? info.assets.map((a, i) => [`assets[${i}]`, a.url]) : []),
  ];
  const bad = urls.filter(([, u]) => typeof u === 'string' && u.includes('github.com'));
  if (bad.length > 0) {
    console.error(`::error::${label} ${version} 的下载地址必须是 Gitee（总纲 §5-M6 断代）—— 还指着 GitHub：`);
    for (const [what, u] of bad) console.error(`  ${what}: ${u}`);
    failed = true;
  } else {
    console.log(`${label} ${version}：下载地址来自 Gitee ✓`);
  }
}
if (failed) process.exit(1);
