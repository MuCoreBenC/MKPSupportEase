# 报告页文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。
> 这一页是执行记录的诊断视图，**字段不删**；只把「判定 / 档位 / 命中 / 复检」这类内部词
> 换成读得懂的说法，并砍掉解释机制的长句。

## 1. 页头与空态

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `report.head.sub` | 后处理每跑一次（切片器导出时按「复制后处理脚本」那条命令跑），这里多一条记录。 | A D | 每跑一次后处理，这里多一条记录。 | 每跑一次後處理，這裡多一條記錄。 | 後処理を実行するたびに記録が増えます。 | One record per post-processing run. | 括号里是机制 |
| ☐ | `report.empty.none` | 把首页「复制后处理脚本」贴进切片器的后处理脚本栏，导出打印文件时会自动生成第一条。 | A D | 把「复制后处理脚本」贴进切片器，导出后这里就会有一条记录。 | 把「複製後處理腳本」貼進切片器，匯出後這裡就會有一條記錄。 | 「後処理スクリプトをコピー」をスライサーに貼り、書き出すと記録が残ります。 | Paste the script into your slicer; the first run shows up after export. | — |
| ☐ | `report.empty.loading` | 正在读执行记录… | E | 正在读取记录… | 正在讀取記錄… | 記録を読み込み中… | Loading runs… | 「执行记录」是内部名 |
| ☐ | `report.detail.pick` | 左边选一条记录看详情。 | D | 选一条记录看详情 | 選一條記錄看詳情 | 記録を選ぶと詳細が表示されます | Pick a run to see details | — |
| ☐ | `report.detail.loading` | 正在读这一条的详情… | D | 正在读取… | 正在讀取… | 読み込み中… | Loading… | — |

## 2. 记录列表

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `report.side.unfold` | 展开执行记录（{n} 条） | E | 展开记录（{n}） | 展開記錄（{n}） | 記録を展開（{n}） | Expand runs ({n}) | — |
| ☐ | `report.row.noName` | （记录里没写文件名） | A B | 未记录文件名 | 未記錄檔名 | ファイル名の記録なし | No file name recorded | — |
| ☐ | `report.cell.printTimeEst` | 打印时间（估算） | D | 打印时间（估） | 列印時間（估） | 印刷時間（推定） | Print time (est.) | — |
| ☐ | `report.note.reading` | 正在重读执行记录… | E | 正在重新读取… | 正在重新讀取… | 再読み込み中… | Reloading… | — |
| ☐ | `report.detail.err` | 详情读不出来：{err} | D | 读不出来：{err} | 讀不出來：{err} | 読み込めません：{err} | Couldn't load: {err} | — |
| ☐ | `report.err.list` | 执行记录读不出来：{err} | D | 读不出来：{err} | 讀不出來：{err} | 読み込めません：{err} | Couldn't load: {err} | — |
| ☐ | `report.replay.needDetail` | 等这条记录读出来才能回放 | A D | 记录读取后才能回放 | 記錄讀取後才能回放 | 記録の読み込み後に再生できます | Available after the run loads | — |

## 3. 详情字段

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `report.calib.verdict` | 判定 | E | 版本 | 版本 | バージョン | Version | 字段名当句子 |
| ☐ | `report.calib.verdict3mf` | 模型里带校准标记 | E | 校准件 | 校正件 | 校正パーツ | Calibration part | — |
| ☐ | `report.line.calibClassic` | 档位: 按机型固定坐标（经典分支，未认档） | A E | 档位：按机型固定坐标 | 檔位：按機型固定座標 | 位置：機種ごとの固定座標 | Position: fixed per machine | 「经典分支 / 未认档」是实现细节 |
| ☐ | `report.line.calibHitSpec` | 命中档位: {id}（{w}×{h}，容差 {tol}） | E | 档位：{id}（{w}×{h}，容差 {tol}） | 檔位：{id}（{w}×{h}，容差 {tol}） | 位置：{id}（{w}×{h}、許容差 {tol}） | Position: {id} ({w}×{h}, tolerance {tol}) | 「命中」→「档位」 |
| ☐ | `report.line.calibBox` | 实测包围盒: {w} × {h} mm / X[{x0}, {x1}]  Y[{y0}, {y1}] | E | 实测范围：{w} × {h} mm / X[{x0}, {x1}] Y[{y0}, {y1}] | 實測範圍：{w} × {h} mm / X[{x0}, {x1}] Y[{y0}, {y1}] | 実測範囲：{w} × {h} mm / X[{x0}, {x1}] Y[{y0}, {y1}] | Measured: {w} × {h} mm / X[{x0}, {x1}] Y[{y0}, {y1}] | 「包围盒」是图形学术语 |
| ☐ | `report.line.bed` | 机型维度: {w}×{h}mm | E | 底板尺寸：{w}×{h} mm | 底板尺寸：{w}×{h} mm | プレート寸法：{w}×{h} mm | Bed size: {w}×{h} mm | — |
| ☐ | `report.line.execDirect` | 执行方式: 直接校准（跳过模型打印） | B | 执行方式：直接校准 | 執行方式：直接校正 | 実行方法：校正のみ | Mode: calibration only | — |
| ☐ | `report.line.execPrintThen` | 执行方式: 打印后校准 | D | 执行方式：打印后校准 | 執行方式：列印後校正 | 実行方法：印刷後に校正 | Mode: print, then calibrate | — |
| ☐ | `report.line.towerMoved` | 已挪塔 ({fromX}, {fromY}) → ({toX}, {toY})，挪了 {moved} mm | D | 挪塔 ({fromX}, {fromY}) → ({toX}, {toY})，{moved} mm | 挪塔 ({fromX}, {fromY}) → ({toX}, {toY})，{moved} mm | タワー移動 ({fromX}, {fromY}) → ({toX}, {toY})、{moved} mm | Tower moved ({fromX}, {fromY}) → ({toX}, {toY}), {moved} mm | — |
| ☐ | `report.line.towerMovedStill` | 复检仍有 {n} 个冲突点 | E | 仍有 {n} 处冲突 | 仍有 {n} 處衝突 | まだ {n} 件の衝突があります | Still {n} collisions | 「复检」是内部步骤 |
| ☐ | `report.line.towerLayers` | 擦料塔层数: {count} | D | 擦料塔层数：{count} | 擦料塔層數：{count} | タワー層数：{count} | Tower layers: {count} | 全角冒号统一 |
| ☐ | `report.line.towerPos` | 擦料塔位置: ({x}, {y}) | D | 擦料塔位置：({x}, {y}) | 擦料塔位置：({x}, {y}) | タワー位置：({x}, {y}) | Tower position: ({x}, {y}) | — |
| ☐ | `report.line.layersIndexed` | 收集 {count} 层高索引 | E | 层高：{count} 个 | 層高：{count} 個 | 層高：{count} 件 | Layer heights: {count} | 「索引」是内部词 |
| ☐ | `report.line.ironingThreshold` | 熨烫覆盖率阈值: {pct}% | E | 熨烫覆盖率阈值：{pct}% | 熨燙覆蓋率門檻：{pct}% | アイロン覆盖率のしきい値：{pct}% | Ironing coverage threshold: {pct}% | — |
| ☐ | `report.line.primeOn` | 润笔: 是（每模型: {perModel}, 每层: {everyLayer}） | D | 润笔：是（每模型 {perModel}，每层 {everyLayer}） | 潤筆：是（每模型 {perModel}，每層 {everyLayer}） | プライム：あり（モデルごと {perModel}、層ごと {everyLayer}） | Prime: yes ({perModel} per model, {everyLayer} per layer) | 全角/半角统一 |
| ☐ | `report.line.supportNormal` | 支撑类型: 普通支撑 | D | 支撑类型：普通 | 支撐類型：一般 | サポート種類：通常 | Support: normal | — |
| ☐ | `report.line.supportTree` | 支撑类型: 树状支撑 | D | 支撑类型：树状 | 支撐類型：樹狀 | サポート種類：ツリー | Support: tree | — |
| ☐ | `report.line.lift` | 从擦料塔到模型的抬笔高度: {v}mm | D | 抬笔高度：{v} mm | 抬筆高度：{v} mm | ペン上昇高さ：{v} mm | Pen lift: {v} mm | — |
| ☐ | `report.line.collisionOk` | 无碰撞，擦料塔位置安全 | C | 无碰撞 | 無碰撞 | 衝突なし | No collisions | 后半句同义重复 |
| ☐ | `report.line.inRange` | 模型坐标均在打印范围内 | D | 都在打印范围内 | 都在列印範圍內 | すべて印刷範囲内 | All within the print area | — |
| ☐ | `report.line.write` | {how} · 输出 {size} MB | E | {how} · {size} MB | {how} · {size} MB | {how} · {size} MB | {how} · {size} MB | 「输出」可删 |
| ☐ | `report.reloc.choiceTimeout` | 没等到回答（超时） | D | 超时未选择 | 逾時未選擇 | 時間切れで未選択 | Timed out | — |
| ☐ | `report.reloc.posFromTo` | X {fx} / Y {fy} → X {tx} / Y {ty} | D | X {fx} / Y {fy} → X {tx} / Y {ty} | X {fx} / Y {fy} → X {tx} / Y {ty} | X {fx} / Y {fy} → X {tx} / Y {ty} | X {fx} / Y {fy} → X {tx} / Y {ty} | 可留 |
| ☐ | `report.reloc.posNoMove` | X {fx} / Y {fy}（没有挪） | B | X {fx} / Y {fy}（未移动） | X {fx} / Y {fy}（未移動） | X {fx} / Y {fy}（移動なし） | X {fx} / Y {fy} (not moved) | — |
| ☐ | `report.reloc.recheckBad` | 仍然有 {n} 个冲突点（挪前 {m} 个） | D | 仍有 {n} 处冲突（挪前 {m}） | 仍有 {n} 處衝突（挪前 {m}） | まだ {n} 件の衝突（移動前 {m}） | Still {n} collisions (was {m}) | — |
| ☐ | `report.reloc.recheckOk` | 没有冲突（挪前 {n} 个冲突点） | D | 无冲突（挪前 {n}） | 無衝突（挪前 {n}） | 衝突なし（移動前 {n}） | No collisions (was {n}) | — |

## 4. 快照、预览与导出

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `report.snapshot.irTitle` | IR 终态（程序实际认到的，原样 JSON） | A E | 参数终态（原始 JSON） | 參數終態（原始 JSON） | パラメータ最終値（生 JSON） | Final parameters (raw JSON) | 「IR」是内部缩写 |
| ☐ | `report.snapshot.missing` | 这一盘有 {n} 项设置没读到，用的是默认值： | D | {n} 项没读到，用了默认值： | {n} 項沒讀到，用了預設值： | {n} 件を読み取れず、既定値を使用： | {n} items unread; defaults used: | 「这一盘」是口语 |
| ☐ | `report.snapshot.sum` | {n} 项 · 当时那一份 | D F | {n} 项 | {n} 項 | {n} 件 | {n} items | — |
| ☐ | `report.snapshot.sumEmpty` | 这一份读不出参数 | F | 读不出参数 | 讀不出參數 | パラメータを読めません | No parameters readable | — |
| ☐ | `report.snapshot.noHit` | 没有命中「{q}」的参数。 | D | 没有匹配「{q}」的参数 | 沒有符合「{q}」的參數 | 「{q}」に一致するパラメータはありません | No parameters match "{q}" | — |
| ☐ | `report.map.label` | 放大查看碰撞俯视图 | D | 查看碰撞俯视图 | 檢視碰撞俯視圖 | 衝突の上面図を表示 | View collision map | — |
| ☐ | `report.towerPlate.label` | 放大查看擦料塔位置 | D | 查看擦料塔位置 | 檢視擦料塔位置 | タワー位置を表示 | View tower position | — |
| ☐ | `report.note.devExport` | 已确认导出：3 份文件写到 {path} | A | 已导出 3 份文件到 {path} | 已匯出 3 份檔案到 {path} | 3 件を {path} に書き出しました | Exported 3 files to {path} | 「已确认」是内部状态 |
| ☐ | `report.export.doneZip` | 已导出压缩包（含 {n} 个文件） | D | 已导出 {n} 个文件 | 已匯出 {n} 個檔案 | {n} 件を書き出しました | Exported {n} files | — |
| ☐ | `report.export.kindOriginal` | 原始文件（改动前的备份） | B | 原始文件 | 原始檔案 | 元のファイル | Original file | — |
| ☐ | `report.export.zipHint` | 把这些文件打成一个 zip 包再保存 | A | 打包成 zip 保存 | 打包成 zip 儲存 | zip にまとめて保存 | Save as a zip | — |
| ☐ | `report.export.planErr` | 导出清单读不出来：{err} | D | 读不出来：{err} | 讀不出來：{err} | 読み込めません：{err} | Couldn't load: {err} | — |
| ☐ | `report.export.loading` | 正在读导出清单… | D | 正在读取… | 正在讀取… | 読み込み中… | Loading… | — |
| ☐ | `report.calib.gone` | （已不在原路径） | A B | 文件已不在 | 檔案已不在 | ファイルがありません | File is gone | — |
