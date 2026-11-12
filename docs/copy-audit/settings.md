# 设置页文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。
> 备注写「删行」的，是整条小字都可以不要——那要动布局，先批再改。

## 1. 页头、外观与语言

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.subtitle` | 软件更新与应用设置。 | D G | 外观、行为、数据与更新 | 外觀、行為、資料與更新 | 外観・動作・データ・更新 | Appearance, behavior, data, updates | 现在这句只说了两件事，还带句号 |
| ☐ | `settings.appearance.desc` | 主题与界面语言 | C | （删） | （刪） | （削除） | (remove) | 分组标题下的小字，内容就是这两行本身；删行 |
| ☐ | `settings.locale.note` | 换完立刻生效，记在应用状态里 —— 下次打开还是这一档。 | A F | 立即生效，下次打开保持不变。 | 立即生效，下次開啟保持不變。 | すぐに反映され、次回も維持されます。 | Applies immediately and is remembered. | 「记在应用状态里」是机制 |
| ☐ | `settings.locale.zhHint` | 界面全部中文 | C | （删） | （刪） | （削除） | (remove) | 标签已经写了「简体中文」 |
| ☐ | `settings.locale.zhTWHint` | 台灣用字（預設 / 設定 / 檔案） | C | （删） | （刪） | （削除） | (remove) | 同上；举例反而占位 |
| ☐ | `settings.locale.jaHint` | 界面全部日文 | C | （删） | （刪） | （削除） | (remove) | 同上 |
| ☐ | `settings.locale.enHint` | Interface in English | C | （删） | （刪） | （削除） | (remove) | 同上 |
| ☐ | `settings.locale.systemHint` | 系统是英文就用英文，否则中文 | D | 跟随系统语言 | 跟隨系統語言 | システムに合わせる | Follow system language | 现在只描述两种语言，实际有四档 |
| ☐ | `settings.theme.note` | 换完立刻生效，记在应用状态里 —— 下次打开还是这一档。 | A F | 立即生效，下次打开保持不变。 | 立即生效，下次開啟保持不變。 | すぐに反映され、次回も維持されます。 | Applies immediately and is remembered. | 与语言那句一字不差，可共用一条键 |
| ☐ | `settings.theme.systemHint` | 系统切深色时就跟着深色（没选过时就是这个） | B F | 跟随系统深浅色 | 跟隨系統深淺色 | システムの表示に合わせる | Follow system appearance | 括号删 |
| ☐ | `settings.theme.lightHint` | 一直用浅色 | C | （删） | （刪） | （削除） | (remove) | 「浅色」已说明 |
| ☐ | `settings.theme.darkHint` | 一直用深色 | C | （删） | （刪） | （削除） | (remove) | 同上 |

## 2. 后处理

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.postproc.desc` | 完成后的通知与显示 | C | （删） | （刪） | （削除） | (remove) | 分组小字 |
| ☐ | `settings.postproc.notifyNote` | 处理完成后发送系统通知，点通知可打开处理结果 | C D | 点通知打开处理结果。 | 點通知開啟處理結果。 | 通知をクリックすると結果が開きます。 | Click the notification to open the result. | 前半句与上方标签重复 |
| ☐ | `settings.postproc.mode` | 这一趟怎么显示 | D | 显示方式 | 顯示方式 | 表示方法 | Display | 「这一趟」是口语 |
| ☐ | `settings.postproc.modeNote` | 静默模式照样跑完、照样改 G-code，只是不弹窗给你看进度。 | A F | 静默模式不显示进度，但仍会处理。 | 靜默模式不顯示進度，但仍會處理。 | サイレントでも処理は実行されます（進捗は表示しません）。 | Silent mode still processes; it just hides progress. | — |
| ☐ | `settings.postproc.silentHint` | 后台静默运行，不显示任何窗口 | C | （删） | （刪） | （削除） | (remove) | 「静默」已经说明 |
| ☐ | `settings.postproc.guiHint` | 在独立窗口中显示处理进度和详细信息 | C | （删） | （刪） | （削除） | (remove) | — |
| ☐ | `settings.postproc.glueBottomNote` | 在支撑面下方一层也执行涂胶（比主涂胶低一个层高） | B | 支撑面下方一层也涂胶。 | 支撐面下方一層也塗膠。 | サポート面の 1 層下にも塗布します。 | Also glue one layer below the support surface. | 括号是机制 |
| ☐ | `settings.postproc.towerNote` | 模型与擦料塔相撞时怎么办。三种都只改这一趟的塔位置，预设文件不动。 | D | 撞塔时的处理方式，只影响这一趟。 | 撞塔時的處理方式，只影響這一趟。 | タワー衝突時の処理方法。今回の実行のみに適用されます。 | What to do on a tower collision; applies to this run only. | — |
| ☐ | `settings.postproc.resetNote` | 钩子卡在等答复、界面这一屏还是上次的时点这里。已经跑完的处理不会被改。 | A D | 界面卡在上次状态时点这里。已完成的处理不受影响。 | 介面卡在上次狀態時點這裡。已完成的處理不受影響。 | 画面が前回のままのときはここを押してください。完了済みの処理は変わりません。 | Use this if the panel is stuck on the last run. Finished runs aren't affected. | 「钩子」是内部词 |

## 3. 校准与补偿

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.calib.desc` | 会改 G-code 的几项 | D | 会改写 G-code | 會改寫 G-code | G-code を書き換えます | Modifies G-code | 分组小字，可删 |
| ☐ | `settings.calib.skipVibrationNote` | 跳过 G-code 中的震动补偿校准检测步骤（M970/M974），使用打印机已保存的校准数据，减少打印准备时间 | A B D | 跳过 M970/M974 检测，用打印机已存的校准数据。 | 跳過 M970/M974 偵測，用印表機已存的校正資料。 | M970/M974 の検出を省略し、プリンタ保存の校正データを使います。 | Skips M970/M974 checks and uses the printer's saved calibration. | 一句说完，不解释收益 |
| ☐ | `settings.calib.liftNote` | 在起始段抬 Z 之后**再补抬**的相对高度（{min}-{max} mm）。用的是带堵转保护的抬升，碰到行程顶端会自己停，不会硬顶 | A B D | 抬 Z 后再补抬的高度（{min}-{max} mm），到顶会自动停。 | 抬 Z 後再補抬的高度（{min}-{max} mm），到頂會自動停。 | Z 上昇後に追加で上げる高さ（{min}-{max} mm）。上端で自動停止します。 | Extra lift after Z homing ({min}–{max} mm); stops at the top. | 加粗与「堵转保护」删 |
| ☐ | `settings.calib.beginRetractNote` | 在起始段把 Z 抬起来之后，按下面的设定再补抬一截、把 X 归零，然后到右端收笔位把笔尖收起 | A D | 抬 Z 后按下面设定补抬、X 归零，再到右端收笔。 | 抬 Z 後按下方設定補抬、X 歸零，再到右端收筆。 | Z 上昇後、下の設定で追加上昇し、X を原点へ戻して右端でペンを収めます。 | After Z lift: extra lift, home X, then park the pen at the right end. | — |
| ☐ | `settings.calib.zDwellNote` | Z 校准每一层测量后的暂停时间（{min}-{max} 秒） | B | 每层测量后暂停（{min}-{max} 秒）。 | 每層測量後暫停（{min}-{max} 秒）。 | 各層の測定後の待ち時間（{min}-{max} 秒）。 | Pause after each layer ({min}–{max} s). | 「Z 校准」已由标签给出 |
| ☐ | `settings.calib.xyDwellNote` | XY 校准每一条线测量后的暂停时间（{min}-{max} 秒） | B | 每条线测量后暂停（{min}-{max} 秒）。 | 每條線測量後暫停（{min}-{max} 秒）。 | 各ラインの測定後の待ち時間（{min}-{max} 秒）。 | Pause after each line ({min}–{max} s). | — |

## 4. 涂胶

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.glue.risk` | 此功能在打印前于热床执行涂胶操作，可能导致笔尖划伤打印板。建议不要开启，除非您清楚了解风险。 | D G | 可能划伤打印板。 | 可能刮傷列印板。 | 印刷プレートを傷つける場合があります。 | May scratch the build plate. | 危险提示只留后果，客套删 |
| ☐ | `settings.glue.enableNote` | 在自动调平后、检测条前执行涂胶（降温到 30°C → 换笔 → 涂胶 → 收笔）。校准件那一趟不涂 —— 那边没有擦料塔，涂胶路径会跑到校准区域外面。 | A B D | 调平后、检测条前涂胶。校准件不涂。 | 調平後、檢測條前塗膠。校正件不塗。 | レベリング後、検査ライン前に塗布。校正パーツでは塗布しません。 | Glue after leveling, before the test line. Skipped for calibration parts. | 流程箭头与原因删 |
| ☐ | `settings.glue.allModelsNote` | 关着只涂擦料塔那一块；打开会把每个模型的首层轮廓也走一遍（更耗时间）。 | A D | 关闭只涂擦料塔；开启会涂每个模型的首层。 | 關閉只塗擦料塔；開啟會塗每個模型的首層。 | オフはタワーのみ、オンは各モデルの 1 層目にも塗布します。 | Off: tower only. On: also each model's first layer. | — |

## 5. 预设默认行为与执行记录

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.presets.desc` | 复制与脚本的默认行为 | C | （删） | （刪） | （削除） | (remove) | 分组小字 |
| ☐ | `settings.presets.copies` | 复制出来的也跟着云端更新 | D | 副本跟随云端更新 | 副本跟隨雲端更新 | コピーもクラウド更新に追従 | Copies follow cloud updates | — |
| ☐ | `settings.presets.copiesNote` | 默认关：你复制出来的那一份是独立的一份 —— 它不算「云端有新版」的那一份，点「更新」也不会动它。开着时它与原件一样，一样显示云端更新、一样会被更新到。 | A C D F | 开启后，副本和原文件一样显示并接受云端更新。 | 開啟後，副本和原檔案一樣顯示並接受雲端更新。 | オンにすると、コピーも元ファイルと同じくクラウド更新を表示・適用します。 | When on, copies show and take cloud updates like the original. | 同一件事说了两遍（关着怎样/开着怎样） |
| ☐ | `settings.presets.autoCopy` | 使用后自动复制后处理脚本 | D | 使用预设后复制后处理脚本 | 使用預設後複製後處理腳本 | プリセット使用後に後処理スクリプトをコピー | Copy the post-processing script after use | — |
| ☐ | `settings.presets.autoCopyNote` | 默认关：每次「使用中」换成另一份文件时，自动把那一份的后处理命令抄进剪贴板并发一条通知 —— 回切片器直接粘贴。同一份再点一次不重复抄；撤销使用也不抄。 | A D F | 使用中每换一份，自动复制它的后处理脚本并通知。 | 使用中每換一份，自動複製它的後處理腳本並通知。 | 使用中に切り替えるたび、そのスクリプトをコピーして通知します。 | Copies the script and notifies each time the in-use preset changes. | 末尾两个例外是机制 |
| ☐ | `settings.history.note` | 每跑一次后处理都会存下原件、输出与执行记录，攒久了很占地方。开着时只保留最近 20 条，更旧的会被自动清掉 —— 清理不打扰你（执行记录页里直接少几条）；最新那条永远留着，停用多久回来都还在。 | A D F | 开启后只保留最近 20 条，更旧的自动清理。 | 開啟後只保留最近 20 條，更舊的自動清理。 | オンにすると直近 20 件のみ残し、それ以前は自動削除します。 | When on, keeps the last 20 runs and clears older ones. | 三句压成一句 |
| ☐ | `settings.history.on` | 自动清理：开 —— 只保留最近 20 条，更旧的静默清掉 | D | 自动清理：开 | 自動清理：開 | 自動整理：オン | Auto-clean: on | 规则已在上一句 |
| ☐ | `settings.history.off` | 自动清理：关 —— 记录一直留着 | D | 自动清理：关 | 自動清理：關 | 自動整理：オフ | Auto-clean: off | — |
| ☐ | `settings.history.unknown` | 自动清理（还没读到这一档） | A | 自动清理 | 自動清理 | 自動整理 | Auto-clean | 读不到就显示未知，不解释 |
| ☐ | `settings.guides.note` | 参数保存成功后，会弹一句「切片器不会自己知道参数变了」，并带你去看重新切片那一步。关掉之后保存就不再弹（提示里那个「不再显示」就是它，两边是同一个开关）。 | A C F | 保存后提示回切片器重新切片。关闭后不再提示。 | 儲存後提示回切片器重新切片。關閉後不再提示。 | 保存後にスライサーでの再スライスを案内します。オフにすると表示しません。 | Prompts you to re-slice after saving. Turn off to stop the prompt. | 「两边是同一个开关」是机制 |
| ☐ | `settings.guides.on` | 显示 —— 保存后会提示回切片器重切 | D | 显示 | 顯示 | 表示する | Show | — |
| ☐ | `settings.guides.off` | 不显示 —— 保存后不再打扰 | D | 不显示 | 不顯示 | 表示しない | Don't show | 「不再打扰」是安抚 |

## 6. 切片器与打开方式

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.slicer.note` | 立刻生效。「打开模型」的框里还可以只对那一次换一个，不动这里的默认。 | A D | 立即生效。「打开模型」时可只换这一次。 | 立即生效。「開啟模型」時可只換這一次。 | すぐに反映されます。「モデルを開く」では今回のみ変更できます。 | Applies immediately. You can change it for a single open. | — |
| ☐ | `settings.slicer.customHandoff` | 打开模型时交给「{name}」：{path} | A | 打开模型时用「{name}」 | 開啟模型時用「{name}」 | モデルを開くときは「{name}」 | Opens models with "{name}" | 路径删 |
| ☐ | `settings.slicer.label` | 默认用哪个切片软件打开模型 | D | 默认打开模型的应用 | 預設開啟模型的應用程式 | モデルを開く既定のアプリ | Default app for opening models | — |
| ☐ | `settings.guide.step1Note` | 选好之后，下面那颗「全部更改…」才会亮 | A D | 选好后「全部更改…」才会亮。 | 選好後「全部更改…」才會亮。 | 選択すると「すべて変更…」が押せるようになります。 | "Change All…" becomes available after you choose. | — |
| ☐ | `settings.guide.grantNote` | 勾上「访达」后，回来再点一次「去系统里改…」就行 | D F | 勾选「访达」后，回来再点一次「去系统里改…」。 | 勾選「Finder」後，回來再點一次「去系統裡改…」。 | 「Finder」にチェックを入れ、戻ってもう一度「システムで変更…」を押してください。 | Check Finder, then click Change in System Settings again. | 中文机翻腔 |
| ☐ | `settings.guide.step2Note` | 对所有 .3mf 生效 | C | 对所有 .3mf 生效 | 對所有 .3mf 生效 | すべての .3mf に適用されます | Applies to all .3mf files | 可留 |

## 7. 数据、备份与卸载

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.data.desc` | 预设目录与执行记录 | C | （删） | （刪） | （削除） | (remove) | 分组小字 |
| ☐ | `settings.data.privacy` | MKP SupportEase 不会读取、保存、导出或修改您的 3MF 文件 | F | 不会读取、保存或修改你的 3MF 文件。 | 不會讀取、儲存或修改你的 3MF 檔案。 | 3MF ファイルの読み取り・保存・変更は行いません。 | Your 3MF files are never read, saved, or modified. | 「您」全站统一成「你」 |
| ☐ | `settings.data.uninstallRiskPre` | 「卸载应用」会退出程序并删除程序本体， | D | 会删除程序本体， | 會刪除程式本體， | アプリ本体を削除します。 | Removes the app itself; | 两个键拼一句，拆得别扭 |
| ☐ | `settings.data.uninstallRiskPost` | —— 预设、执行记录、备份都留在原来的位置，重装回来还在。 | A B | 预设、记录与备份会保留。 | 預設、記錄與備份會保留。 | プリセット・記録・バックアップは残ります。 | presets, runs, and backups stay. | — |
| ☐ | `settings.data.uninstallRiskWipePost` | —— 预设、执行记录、备份全部清空，重装回来是一张白纸。 | A B | 预设、记录与备份一并删除。 | 預設、記錄與備份一併刪除。 | プリセット・記録・バックアップも削除されます。 | presets, runs, and backups are removed too. | — |
| ☐ | `settings.data.uninstallNote` | 删掉程序本体之后就打不开了。切片器里那条后处理命令也要自己清掉，否则下次切片会报「脚本不存在」。 | A D | 卸载后无法打开。切片器里的后处理命令要自己清掉，否则切片会报错。 | 解除安裝後無法開啟。切片器裡的後處理指令要自己清掉，否則切片會報錯。 | アンインストール後は開けません。スライサーの後処理コマンドは手動で消してください。 | After uninstalling you can't reopen it. Remove the slicer's post-processing command yourself. | — |
| ☐ | `settings.data.clearNote` | 删掉本机的预设、云端原文、执行记录、回收站与全部偏好，程序本体保留。清理完会退出程序，重新打开就是出厂状态。 | D | 删除本机的预设、云端原文、执行记录、回收站与偏好。完成后退出程序。 | 刪除本機的預設、雲端原文、執行記錄、回收站與偏好。完成後結束程式。 | 端末のプリセット・クラウド原文・実行記録・ゴミ箱・設定を削除します。完了後に終了します。 | Deletes local presets, cloud copies, runs, trash, and preferences. The app then quits. | — |
| ☐ | `settings.data.clearRisk` | 「清理数据」只删数据、不删程序本体，但那一步不可撤销。 | D | 只删数据，不删程序。不可撤销。 | 只刪資料，不刪程式。不可復原。 | データのみ削除し、アプリは残ります。元に戻せません。 | Deletes data only, not the app. This can't be undone. | — |
| ☐ | `settings.data.dangerNote` | 「清理数据」只删数据、「卸载应用」删程序本体；两件都不可撤销，点开后在框里再确认一次。 | D | 两项都不可撤销，会再确认一次。 | 兩項都不可復原，會再確認一次。 | どちらも元に戻せません。実行前に再確認します。 | Both are permanent and ask for confirmation. | 前半段与上面重复 |
| ☐ | `settings.data.cleanOnUninstallNote` | 打开后，卸载时会连预设、执行记录、备份、偏好一起删掉 —— 重装回来是一张白纸。关着则只删程序本体，数据原样留着、重装回来还在。 | A D C | 开启后卸载会一并删除数据。 | 開啟後解除安裝會一併刪除資料。 | オンにするとアンインストール時にデータも削除します。 | When on, uninstalling also deletes your data. | 与卸载风险那两句重复 |
| ☐ | `settings.backup.desc` | 打包带走，删了能找回 | D F | 备份与恢复数据 | 備份與還原資料 | バックアップと復元 | Back up and restore | 「带走」是口号 |
| ☐ | `settings.backup.tip` | 打包设置与执行归档；恢复会覆盖当前数据。 | D | 备份设置与执行记录。恢复会覆盖当前数据。 | 備份設定與執行記錄。還原會覆蓋目前資料。 | 設定と実行記録をバックアップします。復元は現在のデータを上書きします。 | Backs up settings and runs. Restoring overwrites current data. | — |
| ☐ | `settings.backupModal.done` | 备份好了 · {files} 个文件 · 原始 {source} → 压缩后 {size} | D | 备份完成 · {files} 个文件 · {size} | 備份完成 · {files} 個檔案 · {size} | バックアップ完了 · {files} 件 · {size} | Backup done · {files} files · {size} | 压缩前后对比是内部细节 |
| ☐ | `settings.backupModal.scopeHint` | 只动选中的那一部分，覆盖后无法撤销 | D | 只覆盖选中的部分，不可撤销。 | 只覆蓋選取的部分，不可復原。 | 選択した範囲のみ上書きします。元に戻せません。 | Only the selected scope is overwritten; this can't be undone. | — |
| ☐ | `settings.recovery.subtitle` | 30 天或 20 份后自动清理 | D | 保留 30 天或 20 份 | 保留 30 天或 20 份 | 30 日または 20 件まで保持 | Kept for 30 days or 20 items | — |
| ☐ | `settings.recovery.restored` | 已恢复「{name}」，未设为使用中 | D | 已恢复「{name}」（未设为使用中） | 已還原「{name}」（未設為使用中） | 「{name}」を復元しました（使用中にはしていません） | Restored "{name}" (not set as in use) | — |
| ☐ | `settings.recovery.failed` | 回收站操作失败：{detail} | E | 操作失败：{detail} | 操作失敗：{detail} | 操作に失敗しました：{detail} | Action failed: {detail} | 「回收站」是内部模型名 |
| ☐ | `settings.data.folderNote` | 打开预设配置文件所在目录 | D | 打开预设目录 | 開啟預設資料夾 | プリセットフォルダを開く | Open the presets folder | — |

## 8. 软件更新

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.update.desc` | 检查并安装新版本 | C | （删） | （刪） | （削除） | (remove) | 分组小字 |
| ☐ | `settings.update.hint` | 更新检查需要能连上发布地址（见下方「高级设置」）。连不上时如实说没查到，不冒充「已是最新」。 | A B G | 连不上发布地址时，会显示没查到。 | 連不上發布位址時，會顯示沒查到。 | 公開先に接続できない場合は「確認できません」と表示します。 | When the release server is unreachable, it says so. | 元话术删 |
| ☐ | `settings.update.versionTo` | 当前版本 {version} → 最新版本 {latest} | D | 当前 {version} → 最新 {latest} | 目前 {version} → 最新 {latest} | 現在 {version} → 最新 {latest} | Current {version} → latest {latest} | — |
| ☐ | `settings.update.previewNote` | 仅供尝鲜，可能存在尚未修复的问题。 | D G | 测试版可能有未修复的问题。 | 測試版可能有未修正的問題。 | ベータ版には未修正の問題がある場合があります。 | Beta builds may have unfixed issues. | — |
| ☐ | `settings.update.channelNote` | 选择更新的发布渠道 | C | （删） | （刪） | （削除） | (remove) | 与标签重复 |
| ☐ | `settings.update.lastFailed` | 上次更新没成功：{detail}（安装日志：{log}） | B | 上次更新没成功：{detail} | 上次更新沒成功：{detail} | 前回の更新に失敗しました：{detail} | Last update failed: {detail} | 日志路径进折叠/tooltip |
| ☐ | `settings.update.unknown` | 这次没能查到更新 | F | 没查到更新 | 沒查到更新 | 更新を確認できませんでした | Couldn't check for updates | — |
| ☐ | `settings.update.latest` | 已是最新版本 | F | 已是最新 | 已是最新 | 最新です | Up to date | 与预设页统一 |
| ☐ | `settings.update.checking` | 正在检查…… | D | 正在检查… | 正在檢查… | 確認中… | Checking… | 六个点 |
| ☐ | `settings.update.autoCheckNote` | 同步目录时顺路查一次正式版 | A | 同步时检查正式版更新 | 同步時檢查正式版更新 | 同期時に正式版の更新を確認 | Checks for stable updates on sync | 「顺路」口语 |

## 9. 高级设置

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `settings.advanced.desc` | 预设数据源地址 | E | 云端数据地址 | 雲端資料位址 | クラウドデータの URL | Cloud data address | 「数据源」是内部说法 |
| ☐ | `settings.advanced.hint` | 支持数据源根、source.json、分支页；先探通再生效。 | A E | 支持根地址、source.json 或分支页。 | 支援根位址、source.json 或分支頁。 | ルート URL、source.json、ブランチページに対応。 | Accepts a root URL, source.json, or branch page. | 「先探通再生效」删 |
| ☐ | `settings.advanced.sourceNote` | 国内选 Gitee 更稳；自定义地址仅供开发排查。 | G | 国内用 Gitee 更稳。 | 中國境內用 Gitee 較穩定。 | 中国国内では Gitee が安定します。 | In mainland China, Gitee is more reliable. | 「仅供开发排查」是对另一群人说的 |
| ☐ | `settings.advanced.syncFailed` | 目录未同步：{detail} | E | 云端数据未同步：{detail} | 雲端資料未同步：{detail} | クラウドデータを同期できません：{detail} | Cloud data didn't sync: {detail} | — |
| ☐ | `settings.advanced.applied` | 已应用。 | D | 已应用 | 已套用 | 適用しました | Applied | 去掉句号 |
| ☐ | `settings.advanced.synced` | 目录已同步。 | E | 已同步 | 已同步 | 同期しました | Synced | — |
| ☐ | `settings.advanced.defaultTag` | （默认） | B | 默认 | 預設 | 既定 | Default | 括号删 |
