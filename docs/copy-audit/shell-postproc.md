# 外壳、通知与零散文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。
> 这一份收：后处理进度与结果、通知、导入弹框、引导卡、更新指示、下载进度、复制脚本、字段控件、缩放与诊断。

## 1. 后处理进度与结果

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `postproc.chip.okTitle` | 后处理完成：{message}（点击看详情） | B C | 后处理完成：{message} | 後處理完成：{message} | 後処理完了：{message} | Done: {message} | 括号里的操作由可点性表达 |
| ☐ | `postproc.chip.notOkTitle` | 后处理没成：{message}（点击看详情） | B C | 后处理没成：{message} | 後處理失敗：{message} | 後処理に失敗：{message} | Failed: {message} | — |
| ☐ | `postproc.chip.runningTitle` | 后处理在后台跑：{phase} · {pct}%（点击看进度） | B C | 后处理中：{phase} · {pct}% | 後處理中：{phase} · {pct}% | 後処理中：{phase} · {pct}% | Running: {phase} · {pct}% | — |
| ☐ | `postproc.chip.okAria` | 后处理完成：{message}，点击看详情 | B C | 后处理完成：{message} | 後處理完成：{message} | 後処理完了：{message} | Done: {message} | 读屏文案不用教点击 |
| ☐ | `postproc.chip.notOkAria` | 后处理没成：{message}，点击看详情 | B C | 后处理没成：{message} | 後處理失敗：{message} | 後処理に失敗：{message} | Failed: {message} | — |
| ☐ | `postproc.chip.runningAria` | 后处理在后台跑：{phase}，{pct}%，点击看进度 | B C | 后处理中：{phase} · {pct}% | 後處理中：{phase} · {pct}% | 後処理中：{phase} · {pct}% | Running: {phase} · {pct}% | — |
| ☐ | `postproc.collision.title` | 这次模型与擦料塔撞上了 | F | 模型和擦料塔相撞 | 模型和擦料塔相撞 | モデルとタワーが衝突しました | The model hit the tower | 「这次」可删 |
| ☐ | `postproc.collision.body` | 这一趟没有做后处理，原文件一个字节都没改。回切片器把模型挪开一点、再切一次就好。 | A D F | 没有做后处理，原文件未改动。把模型挪开一点再切一次。 | 沒有做後處理，原始檔案未變更。把模型挪開一點再切一次。 | 後処理は行っていません。ファイルは未変更です。モデルを少し動かして再スライスしてください。 | Post-processing was skipped; the file is unchanged. Move the model and slice again. | 「一个字节都没改」是安抚尾巴 |
| ☐ | `postproc.collision.triedTitle` | 塔已经挪过，但挪完还是撞 | D | 挪塔后仍然相撞 | 挪塔後仍然相撞 | タワーを移動しても衝突します | Still colliding after moving the tower | — |
| ☐ | `postproc.collision.triedBody` | 这一趟挪过塔之后复检仍然有冲突，所以停下了，原文件一个字节都没改。把模型挪开一点再切一次。 | A D F | 挪塔后仍有冲突，已停止，原文件未改动。把模型挪开一点再切一次。 | 挪塔後仍有衝突，已停止，原始檔案未變更。把模型挪開一點再切一次。 | タワー移動後も衝突するため停止しました。ファイルは未変更です。モデルを動かして再スライスしてください。 | Stopped: still colliding after the move; the file is unchanged. Move the model and slice again. | — |
| ☐ | `postproc.collision.tip` | 不想每次自己挪：在「设置 → 后处理模式 → 智能塔位置」选「自动挪塔」，下次会直接替你把塔挪到不撞的位置。 | A D | 想自动处理，去「设置 → 智能塔位置」选「自动挪塔」。 | 想自動處理，去「設定 → 智慧塔位置」選「自動挪塔」。 | 自動化するには「設定 → タワー位置」で「自動移動」を選んでください。 | To automate this, choose Auto-move in Settings → Tower position. | — |
| ☐ | `postproc.relocate.doneTitle` | 这一趟的塔位置已经帮你挪好了 | A D | 已自动挪塔 | 已自動挪塔 | タワーを移動しました | Tower moved | — |
| ☐ | `postproc.relocate.doneBody` | 从 {from} 挪到 {to}，挪了 {moved} mm；挪完复检没有冲突，于是接着跑完了这一趟。只改这一趟，预设文件没动。 | A D | 从 {from} 挪到 {to}，{moved} mm，无冲突，已继续处理。只改这一趟。 | 從 {from} 挪到 {to}，{moved} mm，無衝突，已繼續處理。只改這一趟。 | {from} から {to} へ {moved} mm 移動。衝突なしで処理を続行しました。今回のみの変更です。 | Moved {from} → {to} ({moved} mm); no collisions, run continued. This run only. | — |
| ☐ | `postproc.relocate.doneBodyNoMove` | 这一趟没有挪塔（板子上摆不下，或塔本来就不撞）。 | A D | 没有挪塔（摆不下，或本来不冲突）。 | 沒有挪塔（擺不下，或本來不衝突）。 | タワーは移動していません（置けなし、または衝突なし）。 | Tower not moved (no room, or no collision). | — |
| ☐ | `postproc.relocate.doneTip` | 想每次都这样不弹问地自动挪，就在「设置 → 后处理模式 → 智能塔位置」选「自动挪塔」。 | A D | 想每次都自动挪，去「设置 → 智能塔位置」选「自动挪塔」。 | 想每次都自動挪，去「設定 → 智慧塔位置」選「自動挪塔」。 | 毎回自動で移動するには「設定 → タワー位置」で「自動移動」を選択。 | To always do this, choose Auto-move in Settings → Tower position. | — |
| ☐ | `postproc.relocate.noRoom` | 板子上摆不下 | D | 板上放不下 | 板上放不下 | プレートに収まりません | No room on the plate | — |
| ☐ | `postproc.relocate.newLabel` | 自动摆放位置 | E | 自动位置 | 自動位置 | 自動配置 | Auto position | — |
| ☐ | `postproc.calibDone.direct` | 已去掉打印件，只保留校准路径 —— 处理完成 | A B | 已移除打印件，只保留校准路径 | 已移除列印件，只保留校正路徑 | 印刷パーツを除き、校正パスのみ残しました | Removed the part; kept the calibration path | — |
| ☐ | `postproc.calibDone.print` | 打印件保留，只加了涂胶与擦料 —— 处理完成 | A B | 已加涂胶与擦料，打印件保留 | 已加塗膠與擦料，列印件保留 | 塗布と拭き取りを追加。印刷パーツは残しました | Added glue and wipe; part kept | — |
| ☐ | `postproc.ask.calibTitle` | 这是校准件 · 选怎么跑 | D | 校准件：选择运行方式 | 校正件：選擇執行方式 | 校正パーツ：実行方法を選択 | Calibration part: choose how to run | — |
| ☐ | `postproc.ask.machineTitle` | 预设与机型不匹配 · 等你决定 | D | 预设与机型不匹配 | 預設與機型不相符 | プリセットと機種が一致しません | Preset and machine don't match | — |
| ☐ | `postproc.mismatch.tag` | 这一趟对不上了 | F | 不匹配 | 不相符 | 不一致 | Mismatch | — |
| ☐ | `postproc.preview.tag` | 预览模式 · 不接真实后处理 | A | 预览 | 預覽 | プレビュー | Preview | — |
| ☐ | `postproc.replay.tag` | 回放 · 当时的那一趟，不接真实后处理 | A | 回放 | 回放 | リプレイ | Replay | — |
| ☐ | `postproc.preview.subtitle` | 预览 · {name} | D | 预览 · {name} | 預覽 · {name} | プレビュー · {name} | Preview · {name} | — |
| ☐ | `postproc.replay.subtitle` | 回放 · {name} | D | 回放 · {name} | 回放 · {name} | リプレイ · {name} | Replay · {name} | — |
| ☐ | `postproc.relocate.toFacts` | 切换：显示原始信息 | A | 显示原始信息 | 顯示原始資訊 | 元の情報を表示 | Show raw info | 「切换：」是操作说明 |
| ☐ | `postproc.relocate.toNote` | 切换：显示挪塔说明与坐标 | A | 显示挪塔说明 | 顯示挪塔說明 | 移動の説明を表示 | Show move details | — |
| ☐ | `postproc.warn.title` | 警告 · {n} 条 | D | 警告 · {n} | 警告 · {n} | 警告 · {n} | Warnings · {n} | — |
| ☐ | `postproc.err.code` | 错误码 {code} | D | 错误码 {code} | 錯誤碼 {code} | エラーコード {code} | Error code {code} | 可留 |
| ☐ | `postproc.err.exit` | 退出码 {code} | D | 退出码 {code} | 結束碼 {code} | 終了コード {code} | Exit code {code} | 可留 |
| ☐ | `postproc.eta.sec` | 预计还需 {n} 秒 | D | 还需 {n} 秒 | 還需 {n} 秒 | 残り {n} 秒 | {n}s left | — |
| ☐ | `postproc.eta.minSec` | 预计还需 {m} 分 {s} 秒 | D | 还需 {m} 分 {s} 秒 | 還需 {m} 分 {s} 秒 | 残り {m} 分 {s} 秒 | {m}m {s}s left | — |
| ☐ | `postproc.autoStop.sec` | {n} 秒后自动终止 | D | {n} 秒后终止 | {n} 秒後に終了 | {n} 秒後に終了します | Stops in {n}s | — |
| ☐ | `postproc.autoStop.minSec` | {m} 分 {s} 秒后自动终止 | D | {m} 分 {s} 秒后终止 | {m} 分 {s} 秒後終了 | {m} 分 {s} 秒後に終了します | Stops in {m}m {s}s | — |

## 2. 导入

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `import.dropHint` | 松开：把文件导进来（现在收 .toml 预设） | A B | 松开即可导入（支持 .toml） | 放開即可匯入（支援 .toml） | ドロップで取り込み（.toml 対応） | Drop to import (.toml) | — |
| ☐ | `import.nameRule` | 名字的规矩与重命名一致：不许空、不许带路径、后缀保持原样；改完点「继续导入」。 | A D | 名字不能为空，不能带路径，后缀保持不变。 | 名稱不能為空，不能帶路徑，副檔名保持不變。 | 名前は空にできず、パスを含められず、拡張子は固定です。 | Name can't be empty or contain a path; the extension stays. | 「规矩与重命名一致」是引用另一处 |
| ☐ | `import.allRejected` | 这 {count} 份收不了 —— 明细在下面 | D F | {count} 份无法导入，明细在下方 | {count} 份無法匯入，詳情如下 | {count} 件を取り込めません。詳細は下に表示 | {count} files can't be imported; details below | — |
| ☐ | `import.cancelled` | 已取消：{count} 份重名的没有导入 —— 可以先改个名字再试 | A D | 已取消，{count} 份重名未导入。改名后可重试。 | 已取消，{count} 份重名未匯入。改名後可重試。 | キャンセルしました。同名 {count} 件は取り込まれていません。名前を変えて再試行してください。 | Cancelled; {count} same-name files weren't imported. Rename and retry. | — |
| ☐ | `import.dupSubtitle` | {count} 份重名的，改好名字才能进来 | D | {count} 份重名，改名后才能导入 | {count} 份重名，改名後才能匯入 | 同名 {count} 件。名前を変えると取り込めます | {count} same-name files need a new name | — |
| ☐ | `import.existsPre` | 已存在同名文件 | C | 已存在同名文件 | 已存在同名檔案 | 同名ファイルがあります | A file with this name exists | 与 existsPost 拼一句，结构问题 |
| ☐ | `import.existsPost` | —— 换个名字（原来那份不会被覆盖） | A B | 换个名字（原文件不会被覆盖） | 換個名字（原檔案不會被覆蓋） | 別の名前を付けてください（元のファイルは残ります） | Choose another name; the original is kept | — |
| ☐ | `import.partial` | 导入：{done} 份成了、{miss} 份没成 | D | 导入：{done} 份成功，{miss} 份失败 | 匯入：{done} 份成功，{miss} 份失敗 | 取り込み：{done} 件成功、{miss} 件失敗 | Imported {done}; {miss} failed | — |
| ☐ | `import.someDone` | 其中 {done} 份已经进来了；还有 {rest} 份没成 —— 在右边那一格里改 | A D | {done} 份已导入，还有 {rest} 份待改名。 | {done} 份已匯入，還有 {rest} 份待改名。 | {done} 件を取り込みました。残り {rest} 件は名前の変更が必要です。 | {done} imported; {rest} need renaming. | 「右边那一格」是布局指代（H） |
| ☐ | `import.noneDone` | 这 {count} 份还没进来 —— 换个名字再试 | D | {count} 份未导入，改名后重试 | {count} 份未匯入，改名後重試 | {count} 件は未取り込み。名前を変えて再試行 | {count} not imported; rename and retry | — |
| ☐ | `import.renamed` | 导入完成：{count} 份重名的都改好名字进来了 | D | 导入完成：{count} 份已改名并导入 | 匯入完成：{count} 份已改名並匯入 | 取り込み完了：{count} 件を改名して取り込みました | Imported {count} files with new names | — |
| ☐ | `import.line.done` | 已导入 {name} → {path} | A | 已导入 {name} | 已匯入 {name} | {name} を取り込みました | Imported {name} | 路径删 |
| ☐ | `import.line.rejected` | {name}：{reason} | D | {name}：{reason} | {name}：{reason} | {name}：{reason} | {name}: {reason} | 可留 |
| ☐ | `import.failed` | 导入没走成：{detail} | D | 导入失败：{detail} | 匯入失敗：{detail} | 取り込みに失敗：{detail} | Import failed: {detail} | — |
| ☐ | `import.noOutcome` | 没走成（没有回结局） | A | 导入没有返回结果 | 匯入沒有回傳結果 | 結果が返りませんでした | No result returned | — |
| ☐ | `import.count` | 导入了 {count} 份文件 | D | 已导入 {count} 份 | 已匯入 {count} 份 | {count} 件を取り込みました | Imported {count} files | — |

## 3. 引导卡（重新切片）

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `guide.savedSubtitle` | 切片器不会自己知道参数变了 —— 重新切片后才生效 | A D | 重新切片后才会生效 | 重新切片後才會生效 | 再スライスすると反映されます | Re-slice to apply the changes | 「切片器不会自己知道」是拟人化解释 |
| ☐ | `guide.card.lead` | MKP 预设参数已修改，重新切片后生效 | D | 参数已修改，重新切片后生效 | 參數已修改，重新切片後生效 | パラメータを変更しました。再スライスで反映されます。 | Parameters changed. Re-slice to apply. | — |
| ☐ | `guide.body1.pre` / `guide.body1.post` | 请在 Bambu Studio 中 + ，以生成应用最新参数的 G-code。 | D G | 在 Bambu Studio 中重新切片，以应用新参数。 | 在 Bambu Studio 中重新切片，以套用新參數。 | Bambu Studio で再スライスして新しいパラメータを適用してください。 | Re-slice in Bambu Studio to apply the new parameters. | 两个键拼一句，结构问题 |
| ☐ | `guide.body2.pre` / `guide.body2.strong` / `guide.body2.post` | 如果点击「切片」后没有重新切片，或「切片」按钮呈灰色无法点击，请先 + 切换到另一块打印板，再选回原来的打印板 + ，然后重新切片。 | A D | 「切片」没反应或变灰时，先切到另一块打印板再切回来。 | 「切片」沒反應或變灰時，先切到另一塊列印板再切回來。 | 「スライス」が反応しない、または灰色のときは、別のプレートに切り替えて戻してください。 | If Slice does nothing or is greyed out, switch to another plate and back. | 三个键拼一句 |
| ☐ | `guide.dontShow` | 以后不再显示（可在「设置 → 引导」里重新打开） | B | 以后不再显示 | 以後不再顯示 | 今後表示しない | Don't show again | 恢复路径进设置页即可，不必写在这 |
| ☐ | `guide.closeTitle` | 关闭（以后是否再提示，看下面的「不再显示」或设置页） | A D | 关闭 | 關閉 | 閉じる | Close | — |
| ☐ | `guide.shown` | 引导卡片已摆到切片器那边（它这会儿在最前）。 | A D F | 引导卡已显示在切片器上 | 引導卡已顯示在切片器上 | ガイドをスライサー上に表示しました | Guide shown over the slicer | — |
| ☐ | `guide.cannotShow` | 这一台上指不了引导 | F | 无法显示引导 | 無法顯示引導 | ガイドを表示できません | Can't show the guide | — |
| ☐ | `guide.zoomAria` | 放大看：{alt} | D | 放大查看：{alt} | 放大檢視：{alt} | 拡大表示：{alt} | Zoom in: {alt} | — |
| ☐ | `guide.openWith.alt` | 系统「显示简介」窗口：展开的「打开方式」，里面有应用下拉与「全部更改…」 | D | 系统「显示简介」窗口，展开「打开方式」 | 系統「显示简介」視窗，展開「打開方式」 | システムの「情報」ウィンドウで「このアプリケーションで開く」を展開 | Finder Get Info, then expand Open With | 中文机里混了英文系统名，见下 |

## 4. 更新、下载与复制脚本

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `update.done` | 下载完成，可以重启安装。 | D | 下载完成，重启后安装 | 下載完成，重新啟動後安裝 | ダウンロード完了。再起動してインストールしてください | Downloaded. Restart to install. | — |
| ☐ | `update.download` | 下载 {version}（{size}） | D | 下载 {version}（{size}） | 下載 {version}（{size}） | {version} をダウンロード（{size}） | Download {version} ({size}) | — |
| ☐ | `update.downloading` | 正在下载 {received} | D | 正在下载 {received} | 正在下載 {received} | ダウンロード中 {received} | Downloading {received} | — |
| ☐ | `update.paused` | 已暂停（{received}） | B | 已暂停 · {received} | 已暫停 · {received} | 一時停止 · {received} | Paused · {received} | — |
| ☐ | `update.tip` | 软件更新：{label}（点击查看详情） | A B | 软件更新：{label} | 軟體更新：{label} | アップデート：{label} | Update: {label} | — |
| ☐ | `download.tick.connecting` | {name}：正在连接数据源 | E | {name}：正在连接 | {name}：正在連線 | {name}：接続中 | {name}: connecting | 「数据源」是内部词 |
| ☐ | `download.tick.done` | {name}：已落进下载区 | E | {name}：已下载 | {name}：已下載 | {name}：ダウンロード完了 | {name}: downloaded | 「落进下载区」是内部动作 |
| ☐ | `download.tick.failedWhy` | {name}：没成 —— {message} | D | {name}：失败 —— {message} | {name}：失敗 —— {message} | {name}：失敗 —— {message} | {name}: failed — {message} | — |
| ☐ | `download.tick.transferringPct` | {name}：已收 {pct}%（{received} / {total} 字节） | A | {name}：{pct}%（{received} / {total}） | {name}：{pct}%（{received} / {total}） | {name}：{pct}%（{received} / {total}） | {name}: {pct}% ({received} / {total}) | 「字节」可删 |
| ☐ | `download.tick.transferringBytes` | {name}：已收 {received} 字节 | A | {name}：{received} | {name}：{received} | {name}：{received} | {name}: {received} | — |
| ☐ | `download.outcome.failed` | {name}：没成，原因没说清 | D G | {name}：失败，原因未知 | {name}：失敗，原因不明 | {name}：失敗、原因不明 | {name}: failed (reason unknown) | — |
| ☐ | `download.outcome.failedWhy` | {name}：{message} | D | {name}：{message} | {name}：{message} | {name}：{message} | {name}: {message} | 可留 |
| ☐ | `copyScript.ok.title` | 已复制后处理脚本 | C | 已复制后处理脚本 | 已複製後處理腳本 | 後処理スクリプトをコピーしました | Post-processing script copied | 可留 |
| ☐ | `copyScript.ok.text` | 去切片器的后处理栏粘贴 | A | 去切片器的后处理栏粘贴 | 去切片器的後處理欄貼上 | スライサーの後処理欄に貼り付けてください | Paste it into your slicer's post-processing field | — |
| ☐ | `copyScript.ok.textNamed` | {name} —— 去切片器的后处理栏粘贴 | A B | {name} · 去切片器的后处理栏粘贴 | {name} · 去切片器的後處理欄貼上 | {name} · スライサーの後処理欄に貼り付け | {name} · paste it into your slicer | — |
| ☐ | `copyScript.noTarget.text` | 先下载并应用一份预设，才有可复制的脚本 | D | 先下载并使用一份预设 | 先下載並使用一份預設 | 先にプリセットをダウンロードして使用してください | Download and use a preset first | — |
| ☐ | `copyScript.notReady` | 这条命令现在还抄不了 | F | 还没有可复制的脚本 | 還沒有可複製的腳本 | コピーできるスクリプトがありません | No script to copy yet | — |

## 5. 外壳与控件

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `tabs.copyScript` | 把这一份的后处理脚本复制到剪贴板 | D | 复制后处理脚本 | 複製後處理腳本 | 後処理スクリプトをコピー | Copy post-processing script | 与 `home.script.copyTitle` 重复，共用一键 |
| ☐ | `app.boot.failed` | 启动数据没能读出来。 | D | 启动数据读取失败 | 啟動資料讀取失敗 | 起動データを読み込めません | Couldn't load startup data | — |
| ☐ | `app.loading` | 页面正在加载 | D | 正在加载 | 正在載入 | 読み込み中 | Loading | — |
| ☐ | `common.dismiss` | 关闭这条提示 | D | 关闭 | 關閉 | 閉じる | Dismiss | — |
| ☐ | `menu.noActions` | 这一行没有可用的操作 | D | 没有可用的操作 | 沒有可用的操作 | 操作はありません | No actions available | — |
| ☐ | `field.hint.empty` | 数值不能为空 | D | 不能为空 | 不能為空 | 空にはできません | Can't be empty | — |
| ☐ | `field.hint.notNumber` | 不是有效的数字 | D | 请输入数字 | 請輸入數字 | 数字を入力してください | Enter a number | — |
| ☐ | `field.hint.outOfRange` | 超出范围 {min} ~ {max} | D | 范围 {min} ~ {max} | 範圍 {min} ~ {max} | 範囲 {min} ~ {max} | Range {min} ~ {max} | — |
| ☐ | `slicer.choices.system.label` | 跟随系统默认 | D | 跟随系统默认 | 跟隨系統預設 | システム既定に従う | System default | — |
| ☐ | `slicer.choices.system.hint` | 用 .3mf 关联的程序打开（没选过时就是这个） | B F | 用 .3mf 关联的应用打开 | 用 .3mf 關聯的應用程式開啟 | .3mf に関連付けられたアプリで開く | Opens with the .3mf handler | — |
| ☐ | `slicer.choices.bambu.hint` | 总是交给 Bambu Studio | D | 总是用 Bambu Studio | 總是用 Bambu Studio | 常に Bambu Studio | Always Bambu Studio | — |
| ☐ | `slicer.choices.orca.hint` | 总是交给 OrcaSlicer | D | 总是用 OrcaSlicer | 總是用 OrcaSlicer | 常に OrcaSlicer | Always OrcaSlicer | — |
| ☐ | `slicer.onceNote` | 仅这一次用选定的（默认不写回设置） | A B | 仅这一次 | 僅這一次 | 今回のみ | This time only | 括号是机制 |
| ☐ | `slicer.systemLine` | 用 .3mf 关联的程序打开：{name} | D | 用 .3mf 关联的应用：{name} | 用 .3mf 關聯的應用：{name} | .3mf 関連のアプリ：{name} | .3mf handler: {name} | — |
| ☐ | `zoom.hintDefault` | 拖动挪位置 · 滚轮 / 捏合缩放 · 点任意处或 Esc 收起 | D | 拖动移动 · 滚轮缩放 · Esc 收起 | 拖曳移動 · 滾輪縮放 · Esc 收起 | ドラッグで移動・ホイールで拡大縮小・Esc で閉じる | Drag to move · scroll to zoom · Esc to close | 手势说明可进 tooltip |
| ☐ | `zoom.closeDefault` | 收起放大视图 | D | 关闭 | 關閉 | 閉じる | Close | — |
| ☐ | `trace.copy` | 点击复制 traceId（查日志用） | A | 复制 traceId | 複製 traceId | traceId をコピー | Copy traceId | 诊断入口，开发者向可留 |
