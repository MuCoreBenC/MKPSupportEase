# 首页文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。

## 1. 首屏与机型选择

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `home.hero.lead` | 选择机型，完成基础配置与校准。 | D | 选机型，配置，校准。 | 選機型，設定，校正。 | 機種を選び、設定と校正を行います。 | Pick a machine, configure, calibrate. | 三个动词对三步，短句 |
| ☐ | `home.entry.changeTitle` | 点击更换品牌 / 机型 / 版本 | A | 更换品牌、机型或版本 | 更換品牌、機型或版本 | ブランド・機種・バージョンを変更 | Change brand, machine, or version | 「点击」是操作说明，用户已会点 |
| ☐ | `home.entry.noMachine` | 未选择机型 | D | 未选机型 | 未選機型 | 機種未選択 | No machine selected | — |
| ☐ | `home.picker.modelHint` | 决定校准板尺寸 | C | （删） | （刪） | （削除） | (remove) | 下拉标签已说明；删行 |
| ☐ | `home.picker.variantHint` | 对应不同预设文件 | C | （删） | （刪） | （削除） | (remove) | 同上 |
| ☐ | `home.picker.deselect` | 再点一次取消选择 | A | 再点一次取消 | 再點一次取消 | もう一度押すと解除 | Click again to clear | — |
| ☐ | `home.reason.pickFirst` | 先选齐机型与版本，才有可指的预设 | D | 先选机型和版本 | 先選機型和版本 | 先に機種とバージョンを選んでください | Choose a machine and version first | 「可指的预设」是内部逻辑 |
| ☐ | `home.action.planMissing` | 套餐未配置 | E | 预设未配置 | 預設未設定 | プリセット未設定 | Preset not configured | 「套餐」是内部说法 |
| ☐ | `home.note.partial` | 套餐没下全，这次不使用：{detail} | E | 预设没下全，这次不使用：{detail} | 預設沒下載完整，這次不使用：{detail} | プリセットが揃っていないため使用しません：{detail} | Preset incomplete; not used this time: {detail} | — |

## 2. 使用中的回执

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `home.note.used` | 已使用：这份现在就是正在使用的那一份 | C D | 已设为使用中 | 已設為使用中 | 使用中に設定しました | Set as in use | 同一句在三种回执里重复 |
| ☐ | `home.note.downloaded` | 已下载并使用：这份现在就是正在使用的那一份 | C D | 已下载，并设为使用中 | 已下載，並設為使用中 | ダウンロードして使用中に設定しました | Downloaded and set as in use | — |
| ☐ | `home.note.updated` | 已更新并使用：这份现在就是正在使用的那一份 | C D | 已更新，并设为使用中 | 已更新，並設為使用中 | 更新して使用中に設定しました | Updated and set as in use | — |
| ☐ | `home.note.ready` | 这几份本机都有了 —— 直接设成正在使用的那一份 | A D | 本机已有，可直接设为使用中 | 本機已有，可直接設為使用中 | この端末にあります。使用中に設定できます。 | Already here; can be set as in use | — |
| ☐ | `home.note.missing` | 这份本机还没有 —— 先下载，再设成正在使用的那一份 | A D | 本机没有，先下载 | 本機沒有，先下載 | この端末にないため、先にダウンロードしてください | Not here yet; download first | — |
| ☐ | `home.note.drift` | 本机这份和云端对不上 —— 重新下载，再设成正在使用的那一份 | A D | 和云端对不上，需要重新下载 | 和雲端對不上，需要重新下載 | クラウドと一致しません。再ダウンロードが必要です | Doesn't match the cloud; re-download | — |
| ☐ | `home.unapply.failed` | 撤不掉「正在使用」，底账没动 | E | 没能取消使用 | 沒能取消使用 | 使用中の解除に失敗しました | Couldn't clear the in-use state | 「底账」是内部词 |

## 3. 引导四步

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `home.guide.step1Hint` | 品牌 / 机型 / 打印件版本 | D | 品牌、机型、版本 | 品牌、機型、版本 | ブランド・機種・バージョン | Brand, machine, version | 斜杠改顿号 |
| ☐ | `home.guide.step2Hint` | 核对群里的最新参考值 | A | 核对最新参考值 | 核對最新參考值 | 最新の基準値を確認 | Check the latest reference values | 「群里」是外部语境 |
| ☐ | `home.guide.step3Hint` | 打 Z 板判断哪格涂胶均匀 | D | 打 Z 板，看哪格涂胶均匀 | 印 Z 板，看哪格塗膠均勻 | Z プレートを印刷し、塗布の均一性を確認 | Print the Z plate and check glue evenness | — |
| ☐ | `home.guide.step4Hint` | 打 XY 板判断哪条涂胶均匀 | D | 打 XY 板，看哪条涂胶均匀 | 印 XY 板，看哪條塗膠均勻 | XY プレートを印刷し、塗布の均一性を確認 | Print the XY plate and check glue evenness | — |
| ☐ | `home.guide.step5Hint` | 打一件确认最终效果 | D | 打一件，确认效果 | 印一件，確認效果 | 1 点印刷して最終確認 | Print one part to confirm | — |

## 4. 校准卡与模型

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `home.calib.modelsFeature` | 鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景 | D | 一套 3mf 覆盖全部校准场景 | 一套 3mf 覆蓋全部校正場景 | 1 つの 3mf ですべての校正をカバー | One 3mf covers every calibration case | 前半段是零件名，图上有 |
| ☐ | `home.calib.modelsLead` | 打印这套模型，检查校准结果。 | D | 打印后检查校准结果。 | 列印後檢查校正結果。 | 印刷して校正結果を確認します。 | Print it, then check the result. | — |
| ☐ | `home.openModel.checking` | {name} · 正在看本机有没有缓存… | A D | {name} · 正在检查缓存… | {name} · 正在檢查快取… | {name} · キャッシュを確認中… | {name} · Checking cache… | — |
| ☐ | `home.openModel.hasCache` | {name} · 本机已有缓存（{size}），可以直接打开 | A D | {name} · 已有缓存（{size}） | {name} · 快取あり（{size}） | {name} · キャッシュあり（{size}） | {name} · Cached ({size}) | 「可以直接打开」由按钮说 |
| ☐ | `home.openModel.noCache` | {name} · 本机还没有缓存文件，「从云端获取」会先取回再打开（请耐心等待 3mf 打开） | A D F | {name} · 没有缓存，需要先从云端获取 | {name} · 沒有快取，需要先從雲端取得 | {name} · キャッシュなし。先にクラウドから取得します | {name} · Not cached; fetch from cloud first | 「请耐心等待」是安抚 |
| ☐ | `home.openModel.working` | {name} · 正在处理，请稍候… | D | {name} · 正在处理… | {name} · 處理中… | {name} · 処理中… | {name} · Working… | — |
| ☐ | `home.script.copyTitle` | 把这一份的后处理脚本复制到剪贴板 | D | 复制后处理脚本 | 複製後處理腳本 | 後処理スクリプトをコピー | Copy post-processing script | 与 `tabs.copyScript` 同义，二选一 |
| ☐ | `home.script.notReady` | 这条命令现在还抄不了 | F | 还没有可复制的脚本 | 還沒有可複製的腳本 | コピーできるスクリプトがありません | No script to copy yet | 「抄」是口语 |

## 5. 离开与切换的确认

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `home.modal.pendingNote` | 偏移改动还没保存： | D | 偏移改动还没保存 | 偏移變更還沒儲存 | オフセットの変更が未保存です | Offset changes aren't saved | 句尾冒号靠拼接，删 |
| ☐ | `home.modal.pendingAsk` | 。离开前要保存吗？ | D | 离开前要保存吗？ | 離開前要儲存嗎？ | 終了前に保存しますか？ | Save before leaving? | 句首句号来自拼接（`home.modal.axisSeparator`），结构问题 |
| ☐ | `home.modal.pendingLose` | 。现在离开这些改动会丢掉。 | D | 现在离开会丢掉这些改动。 | 現在離開會捨棄這些變更。 | 今終了すると変更は失われます。 | Leaving now discards these changes. | 同上 |
| ☐ | `home.modal.presetAskNote` | 换成 {name} 之后，这一页点出来的偏移改动会作废 —— 它是相对上一份预设算出来的增减，换了基准就没有意义了。 | A D | 换成 {name} 后，这些偏移改动会作废。 | 換成 {name} 後，這些偏移變更會作廢。 | {name} に変更すると、これらのオフセット変更は破棄されます。 | Switching to {name} discards these offset changes. | 原因解释删 |
| ☐ | `home.calib.unsaved` | 未保存 | C | 未保存 | 未儲存 | 未保存 | Unsaved | 可留 |
