# 校准页文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。

## 1. 点选与偏移

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `calib.needPreset.title` | 点选还没有基准 | E | 还不能点选 | 還不能點選 | まだ選択できません | Can't pick yet | 「基准」是内部词 |
| ☐ | `calib.needPreset.how` | 先选一份预设，取到它之后板上的点选才有依据 | A D | 先选一份预设 | 先選一份預設 | 先にプリセットを選んでください | Choose a preset first | 后半句是机制 |
| ☐ | `calib.originalTitle` | 原值 {value} —— 改过之后点这里退回 | A D | 原值 {value}，点此恢复 | 原值 {value}，點此還原 | 元の値 {value}。クリックで戻す | Original {value}; click to restore | — |
| ☐ | `calib.savedWritten` | 改动已经写进预设文件 | D | 已写入预设文件 | 已寫入預設檔案 | プリセットに保存しました | Saved to the preset | — |
| ☐ | `calib.unsavedTitle` | 有未保存的改动 | C | 有未保存的改动 | 有未儲存的變更 | 未保存の変更があります | Unsaved changes | 可留 |
| ☐ | `calib.leaveBody` | 偏移改动还没保存：{changes}。离开前要保存吗？ | D | 偏移改动还没保存：{changes}。要先保存吗？ | 偏移變更還沒儲存：{changes}。要先儲存嗎？ | オフセットの変更が未保存です：{changes}。保存しますか？ | Offset changes aren't saved: {changes}. Save first? | — |
| ☐ | `calib.switchBody` | 换成 {file} 之后，点出来的偏移改动会作废 —— 它是相对上一份预设算出来的增减，换了基准就没有意义了。 | A D | 换成 {file} 后，这些偏移改动会作废。 | 換成 {file} 後，這些偏移變更會作廢。 | {file} に変更すると、これらのオフセット変更は破棄されます。 | Switching to {file} discards these offset changes. | 原因删 |
| ☐ | `calib.z.legend` | ← 抬高笔尖 ｜ 降低笔尖 → | D | ← 抬高　降低 → | ← 抬高　降低 → | ← 上げる　下げる → | ← Raise　Lower → | 箭头与文字语义重复 |
| ☐ | `calib.z.tip` | 校准的时候感觉都没涂上胶水 → 点最右边的格子（-0.5，降低笔尖），或者手动把上面的 Z 改小一点 | A D | 都没涂上胶水就点最右格（-0.5），或把 Z 调小。 | 都沒塗上膠水就點最右格（-0.5），或把 Z 調小。 | 塗布されない場合は右端のマス（-0.5）か Z を小さくしてください。 | Nothing glued? Pick the rightmost cell (-0.5) or lower Z. | 口语「校准的时候感觉」 |
| ☐ | `calib.testCaption` | 鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景 | D | 一套 3mf 覆盖全部校准场景 | 一套 3mf 覆蓋全部校正場景 | 1 つの 3mf ですべての校正をカバー | One 3mf covers every calibration case | 与首页同句，共用一条键 |

## 2. 模型缓存与打开

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `calib.cache.checking` | {label} · 正在看本机有没有缓存… | A D | {label} · 正在检查缓存… | {label} · 正在檢查快取… | {label} · キャッシュを確認中… | {label} · Checking cache… | — |
| ☐ | `calib.cache.ready` | {label} · 本机已有缓存（{size}），可以直接打开 | A D | {label} · 已有缓存（{size}） | {label} · 快取あり（{size}） | {label} · キャッシュあり（{size}） | {label} · Cached ({size}) | — |
| ☐ | `calib.cache.none` | {label} · 本机还没有缓存文件，「{fetch}」会先取回再打开（请耐心等待 3mf 打开） | A D F | {label} · 没有缓存，需要先从云端获取 | {label} · 沒有快取，需要先從雲端取得 | {label} · キャッシュなし。先にクラウドから取得します | {label} · Not cached; fetch from cloud first | 「请耐心等待」是安抚 |
| ☐ | `calib.cache.busy` | {label} · 正在处理，请稍候… | D | {label} · 正在处理… | {label} · 處理中… | {label} · 処理中… | {label} · Working… | — |
