# 预设页文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。
> **改词不删字段**：展开详情的分组、字段、折叠状态都按现状，只换说法。

## 0. 截图两处（建议先批）

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.tree.cloudLocal.unchanged` | 已是最新（云端发布前进过，这份内容一致；可重新下载补齐字段） | B C E | 已是最新 · 可重新下载 | 已是最新 · 可重新下載 | 最新です · 再ダウンロードできます | Up to date · Re-download | 同一句在行状态、详情「更新状态」、详情顶部各出现一次，只留行状态；动作键现在是「更新」，要显示「重新下载」得新增一条键（下一轮代码） |
| ☐ | `presets.page.removeAsk.presetDetail` | 它回到「未下载」，随时可以从云端重新下载（云端那份一直留着，不会丢什么）。它正在被使用的话，会一并撤下。 | A B F | 删除后回到「未下载」，可以重新下载。正在使用的会停用。 | 刪除後回到「未下載」，可以重新下載。正在使用的會停用。 | 削除すると「未ダウンロード」に戻り、再ダウンロードできます。使用中のものは停止します。 | Deleting puts it back to Not downloaded; you can download it again. Any copy in use will stop. | 正文只讲后果；「云端一直留着」是机制，删 |
| ☐ | `presets.page.removeAsk.alsoSlicer` | 同时删掉切片器目录里那份 | A F | 同时删除切片器里的同名文件 | 同時刪除切片器裡的同名檔案 | スライサー内の同名ファイルも削除 | Also delete the same file in the slicer | 复选框：说对象，不说目录 |
| ☐ | `presets.page.removeAsk.alsoSlicerNote` | 删的是本机那几个账号目录里的同名文件。BBS 已经把它同步到你的账号云端的话，下次打开 BBS 会再拉回来 —— 那种只能进 BBS 里删。 | A B D | 会删掉本机各账号目录里的同名文件。若已同步到 BBS 云端，下次打开 BBS 会再拉回来。 | 會刪掉本機各帳號目錄裡的同名檔案。若已同步到 BBS 雲端，下次開啟 BBS 會再拉回來。 | この端末の各アカウントの同名ファイルを削除します。BBS のクラウドに同期済みなら、次回 BBS を開いたときに戻ります。 | Deletes the same file in each local account folder. If it's synced to BBS cloud, BBS will pull it back next time. | 两句话说完范围与例外；「只能进 BBS 里删」是推论，删 |
| ☐ | `presets.page.removeAsk.liveDetail` | 这份正在使用中，删除会一并撤下使用；有没保存的改动也一并丢弃。删掉后进「设置 → 最近删除」，可找回。 | D F | 正在使用的会停用，没保存的改动会丢弃。可在「设置 → 最近删除」找回。 | 正在使用的會停用，未儲存的變更會捨棄。可在「設定 → 最近刪除」找回。 | 使用中のものは停止し、未保存の変更は破棄されます。「設定 → 最近削除」から復元できます。 | Anything in use will stop, and unsaved changes are discarded. You can restore it from Settings → Recently deleted. | 「这份…中」多余，状态已由行给出 |
| ☐ | `presets.page.removeAsk.localDetail` | 这是你自己的文件，删掉后进「设置 → 最近删除」，可找回。 | F | 删除后可在「设置 → 最近删除」找回。 | 刪除後可在「設定 → 最近刪除」找回。 | 削除後は「設定 → 最近削除」から復元できます。 | You can restore it from Settings → Recently deleted. | 抬头已经写了是自己的文件 |
| ☐ | `presets.page.removeAsk.manyPreset` | 其中 {count} 份是云端交付 —— 删了回到「未下载」，随时能重新下载（云端那份一直留着，不会丢什么）；正在被使用的话，会一并撤下。 | A B F | 其中 {count} 份来自云端：删除后回到「未下载」，可以重新下载；正在使用的会停用。 | 其中 {count} 份來自雲端：刪除後回到「未下載」，可以重新下載；正在使用的會停用。 | うち {count} 件はクラウド由来：削除すると「未ダウンロード」に戻り、再ダウンロードできます。使用中のものは停止します。 | {count} of them come from the cloud: deleting puts them back to Not downloaded. Any copy in use will stop. | 批量版同上一句口径 |
| ☐ | `presets.page.removeAsk.manyLocal` | 其中 {count} 份是你自己的文件 —— 删掉后进「设置 → 最近删除」，可找回。 | F | 其中 {count} 份是你自己的文件：删除后可在「设置 → 最近删除」找回。 | 其中 {count} 份是你自己的檔案：刪除後可在「設定 → 最近刪除」找回。 | うち {count} 件は自分のファイルです：削除後は「設定 → 最近削除」から復元できます。 | {count} of them are your own files: you can restore them from Settings → Recently deleted. | — |
| ☐ | `presets.page.removeAsk.manyLive` | 其中正在使用的那一份会一并撤下使用；有没保存的改动也一并丢弃。 | D F | 其中正在使用的会停用，没保存的改动会丢弃。 | 其中正在使用的會停用，未儲存的變更會捨棄。 | 使用中のものは停止し、未保存の変更は破棄されます。 | The one in use will stop, and its unsaved changes are discarded. | — |

## 1. 列表与状态词

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.table.col.cloudTime` | 目录发布 | E | 云端发布 | 雲端發布 | クラウド公開 | Published | 「目录」是内部说法 |
| ☐ | `presets.tree.cloudLocal.none` | 未获取 | E | 未下载 | 未下載 | 未ダウンロード | Not downloaded | 全站统一成「未下载」 |
| ☐ | `presets.tree.cloudLocal.detached` | 本地副本（不跟云端更新） | B | 本地副本 | 本機副本 | ローカルコピー | Local copy | 括号里的规则进 tooltip |
| ☐ | `presets.tree.cloudLocal.elsewhere` | 跨分支，不比版本 | D | 不同分支，不比较 | 不同分支，不比對 | 別ブランチのため比較しません | Different branch, not compared | — |
| ☐ | `presets.tree.cloudLocal.newer` | 云端这一份比你手上的旧 | F | 云端比本机旧 | 雲端比本機舊 | クラウドの方が古いです | Cloud is older than this device | — |
| ☐ | `presets.tree.cloudLocal.inconsistent` | 目录异常 | E | 云端数据异常 | 雲端資料異常 | クラウドデータの異常 | Cloud data error | — |
| ☐ | `presets.tree.cloudLocalWhy.has` | 你机器上已经有一份跟上云端当前版的了 —— 这一行没有要你做的事：要改、要用，去「本地」那张表那一行 | A F | 本机已有这一版。要改、要用，去「本地」。 | 本機已有這一版。要改、要用，去「本機」。 | この端末に最新版があります。編集や使用は「ローカル」から。 | You already have this version. To edit or use it, go to Local. | 「没有要你做的事」是元话术 |
| ☐ | `presets.tree.cloudLocalWhy.none` | 云端有这一份，你机器上还没有它 —— 点「下载」把它取到本机（会落一份本地文件，能改能用） | A D | 云端有，本机没有。点「下载」取到本机，之后可改可用。 | 雲端有，本機沒有。點「下載」取到本機，之後可改可用。 | クラウドにあり、この端末にはありません。「ダウンロード」で取得すると編集・使用できます。 | On the cloud but not here. Click Download to get a copy you can edit and use. | — |
| ☐ | `presets.tree.cloudLocalWhy.stale` | 你机器上那份当初基于的**不是**云端现在这一版（云端换过版了）。点「更新」会逐项比较：你没改过的按新版更新，你也改过的要你选（使用新版值 / 保留本地值）—— **确认之前不会动你这份文件** | C D | 云端换过版。点「更新」逐项比较：你没改过的用新版，你也改过的由你选。确认前不会动这份文件。 | 雲端換過版。點「更新」逐項比對：你沒改過的用新版，你也改過的由你選。確認前不會動這份檔案。 | クラウドが更新されました。「更新」で項目ごとに比較します。変更していない項目は新版を適用し、変更した項目は選択できます。確認するまでファイルは変更されません。 | The cloud has a newer release. Update compares item by item: untouched values take the new version, values you changed are your call. Nothing changes until you confirm. | 去掉加粗与「当初基于的」 |
| ☐ | `presets.tree.localDrawer.note` | 这是**你自己的文件**（住 ~/Documents/SupportEase/presets-local）：云端没有它，所以没有 SHA、不属于任何版本、也不参与套餐。云端换版本不会动它；改它（同一格里的「改这份」）写回的是它自己，永远不回写云端原件。 | A D E | 这是你自己的文件，不在云端：没有版本号，也不参与套餐。改动只写回它自己。 | 這是你自己的檔案，不在雲端：沒有版本號，也不參與套餐。變更只寫回它自己。 | これは自分のファイルで、クラウドにはありません。バージョン番号もパッケージもありません。変更はこのファイルにのみ書き戻されます。 | This is your own file, not on the cloud. It has no version and isn't part of a package. Edits are written back only to it. | 路径进 tooltip；SHA 属折叠区 |
| ☐ | `presets.tree.localNotPresetWhy` | 这一份认不出来是 MKP 预设（切片器那两类都是 .json，光看扩展名分不出是 bbs 还是 orca）—— 能被使用的只有 TOML 预设 | A D | 认不出是 MKP 预设（.json 分不出 BBS 还是 Orca）。只有 TOML 预设能使用。 | 認不出是 MKP 預設（.json 分不出 BBS 還是 Orca）。只有 TOML 預設能使用。 | MKP プリセットとして認識できません（.json は BBS か Orca かを判別できません）。使えるのは TOML プリセットだけです。 | Not recognized as an MKP preset. Only TOML presets can be used. | — |
| ☐ | `presets.tree.localUnreadableWhy` | {text} —— 这一份现在不能使用、也不能改（先把它改回一份能读的 TOML，或者删掉它）。它还是你自己的文件：程序只如实说读不出来，不动它 | A F | {text}。这份现在不能使用或修改。把它改回可读的 TOML，或者删除。 | {text}。這份現在不能使用或修改。把它改回可讀的 TOML，或刪除。 | {text}。現在は使用も編集もできません。読み込める TOML に直すか、削除してください。 | {text}. It can't be used or edited. Fix the TOML or delete it. | 末句「程序只如实说」是元话术 |
| ☐ | `presets.tree.noCombo` | 后端没有 {at} 这个组合 | E | 没有 {at} 这个组合 | 沒有 {at} 這個組合 | {at} の組み合わせはありません | No {at} combination | 「后端」是内部说法 |
| ☐ | `presets.tree.origin.cloudNoRefTitle` | 从云端下载；当时没有记下来源分支 | A | 从云端下载（没有记录分支） | 從雲端下載（沒有記錄分支） | クラウドから取得（ブランチの記録なし） | Downloaded from cloud (branch not recorded) | — |
| ☐ | `presets.tree.origin.cloudTitle` | 从云端下载：这条发布线的 {ref} 分支 | D | 云端下载 · {ref} 分支 | 雲端下載 · {ref} 分支 | クラウドから取得 · {ref} ブランチ | Downloaded from cloud · {ref} branch | — |
| ☐ | `presets.tree.origin.importedTitle` | 导入：这个文件是你从外面导进来的（拖拽 / 文件选择器）—— 云端没有它，删了没有地方能找回来 | A B | 外部导入，云端没有这份。删除后无法找回。 | 外部匯入，雲端沒有這份。刪除後無法找回。 | 外部からインポート。クラウドにはありません。削除すると復元できません。 | Imported from outside; not on the cloud. Deleting is permanent. | — |
| ☐ | `presets.tree.origin.localCopyTitle` | 本地副本：从 {name} 复制出来的（点击定位到母本那一份） | F | 从 {name} 复制（点击定位原件） | 從 {name} 複製（點擊定位原件） | {name} から複製（クリックで元ファイルへ） | Copied from {name} (click to locate the original) | 「母本」改「原件」 |
| ☐ | `presets.tree.origin.presetTitle` | 云端：发布方写进目录、从 {source} 的预设数据源下载（目录指纹 {version}） | A E | 云端发布 · 来源 {source} · 指纹 {version} | 雲端發布 · 來源 {source} · 指紋 {version} | クラウド公開 · 提供元 {source} · ハッシュ {version} | Published · from {source} · fingerprint {version} | 这一句是 tooltip，允许带指纹 |
| ☐ | `presets.tree.origin.unknownCloudTitle` | 这份文件落在盘上时还没有来源记录 —— 只能确认它基于云端那一份，说不出当时从哪个分支下载的 | A D | 下载时没有记录来源，只知它来自云端。 | 下載時沒有記錄來源，只知它來自雲端。 | 取得時の記録がなく、クラウド由来であることだけ分かります。 | No origin recorded at download; only that it came from the cloud. | — |
| ☐ | `presets.tree.originWhy.asset` | 云端：仓库里那份文件下载到本机的副本。不能改名（复制一份再改），删了可以从云端重新下载 | A D | 云端文件的本机副本。不能改名，删除后可以重新下载。 | 雲端檔案的本機副本。不能改名，刪除後可以重新下載。 | クラウドファイルのローカルコピー。名前は変更できません。削除後は再ダウンロードできます。 | Local copy of a cloud file. Can't be renamed; delete and download again. | — |
| ☐ | `presets.tree.originWhy.local` | 本地：你自己放进预设目录的文件，云端没有它。可以改名，但删了没有任何地方能找回来 | A D | 你自己放进来的文件，云端没有。可以改名，删除后无法找回。 | 你自己放進來的檔案，雲端沒有。可以改名，刪除後無法找回。 | 自分で追加したファイルで、クラウドにはありません。名前は変更できますが、削除すると復元できません。 | Your own file, not on the cloud. Renamable; deleting is permanent. | — |
| ☐ | `presets.tree.originWhy.preset` | 云端：发布方发布的那一版，每次发布会换成新的，下载后存在本机。不能改名（复制一份再改），删了可以从云端重新下载 | A D | 云端发布的版本，每次发布会换成新的。不能改名，删除后可以重新下载。 | 雲端發布的版本，每次發布會換成新的。不能改名，刪除後可以重新下載。 | クラウド公開版。公開ごとに新しくなります。名前は変更できません。削除後は再ダウンロードできます。 | A published version, replaced on each release. Can't be renamed; delete and download again. | — |
| ☐ | `presets.tree.suspectWhy` | 这一份的内容和云端登记的对不上 —— 内容存疑，所以不许使用、不许改、不许复制。先「下载」换一份干净的回来 | A D | 内容和云端对不上，不能使用、修改或复制。点「下载」换回正确版本。 | 內容和雲端對不上，不能使用、修改或複製。點「下載」換回正確版本。 | 内容がクラウドと一致しません。使用・編集・複製はできません。「ダウンロード」で正しい版に置き換えてください。 | Content doesn't match the cloud. It can't be used, edited, or copied. Download a clean copy. | — |
| ☐ | `presets.tree.untrustedNote` | 本机这份认不出是云端哪一版 —— 云端连发几版时，更早的版本指纹在客户端留不全，所以认不出不等于被改过（没人动过它的话，多半就是旧版）。点「下载」换一份当前版即可 | A D | 认不出是云端哪一版。点「下载」换一份当前版即可。 | 認不出是雲端哪一版。點「下載」換一份目前版本即可。 | クラウドのどの版か判別できません。「ダウンロード」で現在の版に置き換えてください。 | Can't tell which cloud version this is. Download the current one. | 机制解释删掉 |
| ☐ | `presets.tree.unverified.noSourceRecord` | 无法确认：这份没有来源记录 | A | 无法确认：没有来源记录 | 無法確認：沒有來源記錄 | 確認できません：入手元の記録がありません | Can't confirm: no origin record | — |
| ☐ | `presets.tree.unverified.noSourceIdentity` | 无法确认：下载时没有记录来源分支 | A | 无法确认：没有记录分支 | 無法確認：沒有記錄分支 | 確認できません：ブランチの記録がありません | Can't confirm: no branch recorded | — |
| ☐ | `presets.tree.unverified.noReleaseLine` | 无法确认：下载时没有记录发布线 | A | 无法确认：没有记录发布线 | 無法確認：沒有記錄發布線 | 確認できません：公開ラインの記録がありません | Can't confirm: no release line recorded | — |
| ☐ | `presets.tree.unverified.noSha` | 无法确认：没有记下这是哪一版 | E | 无法确认：没有版本记录 | 無法確認：沒有版本記錄 | 確認できません：バージョンの記録がありません | Can't confirm: no version recorded | — |
| ☐ | `presets.tree.unverified.notInCatalog` | 无法确认：当前云端目录里已经没有这一份 | A | 无法确认：云端已没有这一份 | 無法確認：雲端已沒有這一份 | 確認できません：クラウドにこのファイルがありません | Can't confirm: no longer on the cloud | — |
| ☐ | `presets.tree.unverified.catalogWithoutIdentity` | 无法确认：本机这份云端目录没有发布身份，先更新目录 | A E | 无法确认：云端数据没有发布编号，请先更新 | 無法確認：雲端資料沒有發布編號，請先更新 | 確認できません：クラウドにリリース ID がありません。先に更新してください | Can't confirm: the cloud has no release ID. Update first. | — |
| ☐ | `presets.tree.unverified.noReason` | 无法确认：没有记下原因 | A | 无法确认：没有记录原因 | 無法確認：沒有記錄原因 | 確認できません：理由の記録がありません | Can't confirm: no reason recorded | — |
| ☐ | `presets.tree.usageHint` | 后处理成功跑完才算一次（机型不符、中途失败的都不算） | A | 后处理跑完才算一次 | 後處理跑完才算一次 | 後処理が完了した時点で 1 回と数えます | Counted once post-processing finishes | 例外不用列，用户不需要 |
| ☐ | `presets.tree.versions.noneWhy` | 这个文件不属于任何打印件版本 —— 它是你自己放进预设目录的，仓库里没有它，也没有交付身份 | A E | 这个文件不属于任何版本，是你自己放进来的。 | 這個檔案不屬於任何版本，是你自己放進來的。 | どのバージョンにも属しません。自分で追加したファイルです。 | Belongs to no version; it's your own file. | — |
| ☐ | `presets.tree.versions.oneWhy` | 只有「{text}」用到这个文件 | D | 仅「{text}」使用 | 僅「{text}」使用 | 「{text}」のみが使用 | Used only by "{text}" | — |
| ☐ | `presets.tree.versions.manyWhy` | {count} 个版本共用这个文件：{text} | D | {count} 个版本共用：{text} | {count} 個版本共用：{text} | {count} 版が共用：{text} | Shared by {count} versions: {text} | — |
| ☐ | `presets.table.olderVersions` | 旧版 ×{count} | E | 旧版 {count} 份 | 舊版 {count} 份 | 旧版 {count} 件 | {count} older versions | — |
| ☐ | `presets.table.stale` / `presets.table.stale.title` | 有新版 / 云端这一份出了新版 —— 右键这一行 →「选择要更新的参数」 | A D | 有新版 / 右键选择要更新的参数 | 有新版本 / 右鍵選擇要更新的參數 | 新バージョンあり / 右クリックで更新する項目を選択 | New version / Right-click to choose what to update | 标题句砍半 |
| ☐ | `presets.table.unsaved.title` | 有未保存的参数改动 —— 打开「修改参数」接着改 | A | 有未保存的改动 | 有未儲存的變更 | 未保存の変更があります | Unsaved changes | — |

## 2. 展开详情（结构与字段都不动，只换说法）

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.table.detail.key.sourceMode` | 源配置 | E | 来源 | 來源 | 入手元 | Origin | 与「来源」重复，二者取一 |
| ☐ | `presets.table.detail.key.versions` | 版本来源 | E | 来源 | 來源 | 入手元 | Origin | 同上 |
| ☐ | `presets.table.detail.key.cloudRelease` | 云端新版发布 | E | 云端新版 | 雲端新版 | クラウド新版 | New on cloud | — |
| ☐ | `presets.table.detail.more.version` | 版本判定 | E | 版本 | 版本 | バージョン | Version | 「判定」不上屏 |
| ☐ | `presets.table.detail.more.verdict` | 判定 | E | 版本 | 版本 | バージョン | Version | — |
| ☐ | `presets.table.detail.verdict.none` | 本机暂无副本，下载后建立基线再判定 | E F | 本机没有，下载后才能比较 | 本機沒有，下載後才能比對 | この端末にないため、ダウンロード後に比較できます | Not here yet; download to compare | 「基线」删 |
| ☐ | `presets.table.detail.more.baseline` | 本机留档 | E | 本机记录 | 本機記錄 | この端末の記録 | This device | — |
| ☐ | `presets.table.detail.more.localLine` | 来源发布线 | E | 发布线 | 發布線 | 公開ライン | Release line | 「来源」多余 |
| ☐ | `presets.table.detail.more.sourceSha` | 来源原文摘要 | E | 来源指纹 | 來源指紋 | 提供元ハッシュ | Origin fingerprint | 「摘要」→「指纹」 |
| ☐ | `presets.table.detail.more.localSha` | 本地 SHA-256 | E | 本地指纹 | 本機指紋 | ローカルハッシュ | Local fingerprint | — |
| ☐ | `presets.table.detail.more.cloudSha` | 云端 SHA-256 | E | 云端指纹 | 雲端指紋 | クラウドハッシュ | Cloud fingerprint | — |
| ☐ | `presets.table.detail.more.releaseIdentity` | 发布身份 | E | 发布编号 | 發布編號 | リリース ID | Release ID | — |
| ☐ | `presets.table.detail.more.provenance` | 溯源记录 | E | 来源记录 | 來源記錄 | 入手元の記録 | Origin | — |
| ☐ | `presets.table.detail.more.fullOrigin` | 完整来源 | E | 来源 | 來源 | 入手元 | Origin | — |
| ☐ | `presets.table.detail.more.sourcePresetPath` | 来源预设路径 | E | 来源路径 | 來源路徑 | 提供元のパス | Origin path | — |
| ☐ | `presets.table.detail.moreTech.hint` | （本机留档 / 溯源记录 / SHA） | B E | 本机记录 · 来源 · 指纹 | 本機記錄 · 來源 · 指紋 | 端末の記録 · 入手元 · ハッシュ | Device · origin · fingerprint | 括号去掉 |
| ☐ | `presets.table.detail.lineValue` | {catalog} · 第 {serial} 号 | E | {catalog} · 第 {serial} 版 | {catalog} · 第 {serial} 版 | {catalog} · 第 {serial} 版 | {catalog} · version {serial} | 「号」→「版」 |
| ☐ | `presets.table.detail.publishedHere` | 本机这份发布于 {time} | D | 本机：{time} | 本機：{time} | この端末：{time} | This device: {time} | 标签已说明含义 |
| ☐ | `presets.table.detail.publishedCloud` | 云端最新版发布于 {time} | D | 云端：{time} | 雲端：{time} | クラウド：{time} | Cloud: {time} | — |
| ☐ | `presets.table.detail.cloudVersion` | 云端最新版本 {version} | D | 云端 {version} | 雲端 {version} | クラウド {version} | Cloud {version} | — |
| ☐ | `presets.table.detail.editRemark.title` | 编辑备注 —— 列表里文件名下面那行小字 | A D | 编辑备注（列表里文件名下那行） | 編輯備註（列表檔名下方那行） | メモを編集（一覧のファイル名の下の行） | Edit note (the line under the file name) | — |
| ☐ | `presets.table.detail.editAttr.title` | 编辑机型 / 版本 —— 写进文件头的 # machine: / # variant: 两行 | A D E | 编辑机型和版本 | 編輯機型和版本 | 機種とバージョンを編集 | Edit machine and version | 文件头字段是机制，删 |
| ☐ | `presets.table.detail.update.title` | 把云端新版收进来 —— 你改过的那几项会先问过你 | A D | 更新：你改过的参数会先问你 | 更新：你改過的參數會先問你 | 更新：変更した項目は先に確認します | Update: you'll be asked about values you changed | — |
| ☐ | `presets.table.detail.remarkEmpty` | 空着 = 副标题留空 | D | 留空则不显示副标题 | 留空則不顯示副標題 | 空欄なら副題を表示しません | Leave empty for no subtitle | — |
| ☐ | `presets.table.collapse.title` | 收起详情（与点这一行同一件事） | A F | 收起 | 收合 | 閉じる | Collapse | 括号删 |
| ☐ | `presets.table.remark.restoreDefault.title` | 拿掉覆盖，退回工作台写的那句（没有就空着） | A D | 恢复默认备注 | 還原預設備註 | 既定のメモに戻す | Restore default note | — |
| ☐ | `presets.table.empty.mkpLocal` | 本地还没有这一档的预设 —— 你自己那份本地文件（presets-local）会出现在这里；从云端「下载」一份就会有 | A D | 本地还没有这一档。你自己放的文件会出现在这里，从云端下载一份也会有。 | 本機還沒有這一檔。你自己放的檔案會出現在這裡，從雲端下載一份也會有。 | ローカルにはまだありません。自分で追加したファイルや、クラウドからダウンロードしたファイルがここに表示されます。 | Nothing here yet. Your own files and cloud downloads will appear here. | 目录名删 |
| ☐ | `presets.table.empty.mkpCloud` | 云端没有这一档的云端预设 —— 云端目录里没有登记的，就下不到 | A F | 云端没有这一档。 | 雲端沒有這一檔。 | クラウドにはこの組み合わせがありません。 | Nothing on the cloud for this combination. | 后半句是同义重复 |
| ☐ | `presets.table.empty.slicerLocal` | 本机还没有这个组合的「{kind}」—— 云端文件下载之后会出现在这里，你自己放进预设目录的也会 | A D | 本机还没有这个组合的「{kind}」。下载后会出现在这里。 | 本機還沒有這個組合的「{kind}」。下載後會出現在這裡。 | この組み合わせの「{kind}」はまだありません。ダウンロード後に表示されます。 | No "{kind}" for this combination yet. Downloads will show up here. | — |
| ☐ | `presets.table.empty.slicerCloud` | 云端没有这个组合的「{kind}」—— 菜单上没有，就下不到 | D | 云端没有这个组合的「{kind}」。 | 雲端沒有這個組合的「{kind}」。 | クラウドにこの組み合わせの「{kind}」はありません。 | No "{kind}" on the cloud for this combination. | — |
| ☐ | `presets.table.empty.filtered` | 这一档的 {total} 条都被搜索词「{query}」筛掉了 | D | 「{query}」筛掉了 {total} 条 | 「{query}」篩掉了 {total} 條 | 「{query}」で {total} 件が除外されました | "{query}" filtered out {total} items | — |
| ☐ | `presets.page.headNote.missing` | 少了「{label}」{at}{more}，切片会用默认值 | A | 少了「{label}」，切片会用默认值 | 少了「{label}」，切片會用預設值 | 「{label}」がないため、スライス時に既定値を使います | Missing "{label}"; the slicer will use defaults | — |
| ☐ | `presets.page.headNote.problem` | 这份文件的写法读不懂，切不了 | F | 文件格式无法解析，不能切片 | 檔案格式無法解析，不能切片 | ファイル形式を解析できず、スライスできません | Can't parse this file, so it can't be sliced | — |
| ☐ | `presets.page.viewer.archiveSubtitle` | {name} · 归档 {count} 份 | E | {name} · 旧版 {count} 份 | {name} · 舊版 {count} 份 | {name} · 旧版 {count} 件 | {name} · {count} older versions | 「归档」→「旧版」 |
| ☐ | `presets.tree.path.delivery` | 下载区 mkp · {uid} | E | 下载文件 · {uid} | 下載檔案 · {uid} | ダウンロード · {uid} | Download · {uid} | — |

## 3. 右键菜单与页脚

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.page.menu.revealOurs` | 显示我们那份的文件夹 | F | 显示文件夹 | 顯示資料夾 | フォルダを表示 | Show in folder | — |
| ☐ | `presets.page.menu.revealSlicer` | 显示切片器里的那份 | F | 显示切片器文件 | 顯示切片器檔案 | スライサーのファイルを表示 | Show slicer file | — |
| ☐ | `presets.page.menu.redownload` | 重新从云端下载 | D | 重新下载 | 重新下載 | 再ダウンロード | Re-download | — |
| ☐ | `presets.page.menu.revealSlicerDisabled` | 还没放入 —— 先在行上点「放入」 | A D | 先「放入」才能打开 | 先「放入」才能開啟 | 先に「入れる」を押してください | Add to slicer first | — |
| ☐ | `presets.page.why.copy` | 只有「本地文件」能复制出一份新的 —— 云端那份不归你复制 | A D F | 云端文件不能复制。先「使用」取一份本地的。 | 雲端檔案不能複製。先「使用」取一份本機的。 | クラウドのファイルは複製できません。先に「使用」でローカル版を取得してください。 | Cloud files can't be copied. Use it first to get a local copy. | — |
| ☐ | `presets.page.why.rename` | 只有「本地文件」能改名（只改名字，内容一个字不动）—— 云端那份不归你改名 | A D F | 云端文件不能改名。 | 雲端檔案不能改名。 | クラウドのファイルは名前を変更できません。 | Cloud files can't be renamed. | — |
| ☐ | `presets.page.why.params` | 只有「本地」那一份能改参数 —— 云端那一份是模板，先用「使用」取一份自己的 | A D | 云端文件是模板，先「使用」取一份才能改。 | 雲端檔案是範本，先「使用」取一份才能改。 | クラウドのファイルはテンプレートです。先に「使用」でローカル版を取得してください。 | Cloud files are templates. Use one to get an editable copy. | — |
| ☐ | `presets.page.why.reveal` | 住在程序自己管理的区域 —— 能这样打开的是「本地文件」（你自己的目录里的那份） | A D F | 只有本地文件能这样打开。 | 只有本機檔案能這樣開啟。 | ローカルファイルのみ開けます。 | Only local files can be opened this way. | — |
| ☐ | `presets.page.why.remove` | 云端仓库文件不在这里删 —— 能删的是你自己那份与目录登记的交付文件 | A D E | 云端文件不能在这里删除。 | 雲端檔案不能在這裡刪除。 | クラウドのファイルはここでは削除できません。 | Cloud files can't be deleted here. | — |
| ☐ | `presets.page.why.noMachine` | 这一份没标机型 —— 先在详情里「改归属」，参数表才知道该画哪些项 | A D | 这份没标机型。先在详情里「改归属」。 | 這份沒標機型。先在詳情裡「改歸屬」。 | 機種が未設定です。詳細の「所属を変更」で設定してください。 | No machine set. Set it in Details → Change assignment. | — |
| ☐ | `presets.page.why.noVersion` | 这一份没标版本 —— 先在详情里「改归属」 | A D | 这份没标版本。先在详情里「改归属」。 | 這份沒標版本。先在詳情裡「改歸屬」。 | バージョンが未設定です。詳細の「所属を変更」で設定してください。 | No version set. Set it in Details → Change assignment. | — |
| ☐ | `presets.page.why.paramsNotLocal` | 这一行不在本机 | F | 这一行不在本机 | 這一列不在本機 | この行はこの端末にありません | This row isn't on this device | 文案本身可，保留备查 |
| ☐ | `presets.page.foot.mkpCloud` | 右键任意一行还有下载 / 更新 / 复制链接（没有鼠标就 {menu}） | H | 右键看更多（没有鼠标就 {menu}） | 右鍵看更多（沒有滑鼠就 {menu}） | 右クリックでその他（マウスなしは {menu}） | Right-click for more (no mouse: {menu}) | **布局问题**：菜单项不该在页脚枚举 |
| ☐ | `presets.page.foot.mkpLocal` | 右键任意一行还有使用 / 复制后处理脚本 / 修改参数 / 编辑文件 / 参数对比 / 复制 / 重命名 / 显示所在文件夹 / 置顶（没有鼠标就 {menu}） | H | 右键看更多（没有鼠标就 {menu}） | 右鍵看更多（沒有滑鼠就 {menu}） | 右クリックでその他（マウスなしは {menu}） | Right-click for more (no mouse: {menu}) | 同上；真要教，只教第一项 |
| ☐ | `presets.page.foot.slicer` | 右键任意一行还有置顶 / 查看放入情况 / 显示文件夹 / 删除（没有鼠标就 {menu}） | H | 右键看更多（没有鼠标就 {menu}） | 右鍵看更多（沒有滑鼠就 {menu}） | 右クリックでその他（マウスなしは {menu}） | Right-click for more (no mouse: {menu}) | 同上 |
| ☐ | `presets.page.compare.picked` | 已选中「{name}」作为对比的第一份 —— 再右键另一份，选「与…对比」 | D | 已选「{name}」。再右键另一份，选「与…对比」。 | 已選「{name}」。再右鍵另一份，選「與…對比」。 | 「{name}」を選択しました。別の項目を右クリックして「比較」を選んでください。 | Selected "{name}". Right-click another item and choose Compare. | — |
| ☐ | `presets.page.select.hint` | 点一行勾选或取消 · Esc 退出 | D | 点一行选择 · Esc 退出 | 點一列選擇 · Esc 退出 | 行をクリックで選択 · Esc で終了 | Click a row to select · Esc to exit | — |

## 4. 编辑器与对比台

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.page.editor.hint` | 文件头几行锁定不可改 · 两指左右划 / ⇧滚轮看长行 · {wrapKey} 切换自动换行 | D | 文件头锁定 · {wrapKey} 切换折行 | 檔頭鎖定 · {wrapKey} 切換折行 | ヘッダーは固定 · {wrapKey} で折り返し | Header locked · {wrapKey} to wrap | 手势说明进 tooltip（H） |
| ☐ | `presets.page.editor.saveBlocked` | 先把红条里那几处改好 —— 现在的正文切不了，或者会丢设置 | A D | 先改好红条里的问题 | 先改好紅條裡的問題 | 赤いバーの問題を先に直してください | Fix the errors in the red bar first | — |
| ☐ | `presets.page.editor.undoTitle` | 撤销（一次打字停顿回退一格） | A | 撤销 | 復原 | 元に戻す | Undo | 括号是机制 |
| ☐ | `presets.page.editor.diffHint` | 打开左右对比：左边是它最初保存时的样子，右边是现在 | D | 左边是保存时，右边是现在 | 左邊是儲存時，右邊是現在 | 左が保存時、右が現在 | Left is saved, right is current | — |
| ☐ | `presets.page.editor.diffNoOrigin` | 这份还没有原始备份 —— 没法对比 | A | 这份没有原始备份，无法对比 | 這份沒有原始備份，無法比對 | 元のバックアップがないため比較できません | No original backup to compare | — |
| ☐ | `presets.edit.headLocked.text` | 这几行是程序写的（机型 / 版本 / 出处）—— 要换机型或版本，用详情里那颗「改归属」 | A D | 这几行由程序写入。改机型或版本，用详情里的「改归属」。 | 這幾行由程式寫入。改機型或版本，用詳情裡的「改歸屬」。 | この行はプログラムが書き込みます。機種やバージョンは詳細の「所属を変更」で。 | These lines are written by the app. Change machine or version in Details. | — |
| ☐ | `presets.edit.params.why` | 改这一份的参数（打开参数表，就是参数页本身）；要直接改文件内容，用右键菜单里的「编辑文件」 | A D | 改参数用参数表；改文件内容用右键「编辑文件」。 | 改參數用參數表；改檔案內容用右鍵「編輯檔案」。 | パラメータはパラメータ表、ファイル内容は右クリックの「ファイルを編集」で。 | Use the parameter table for values; use Edit file for raw text. | — |
| ☐ | `presets.edit.taken.note` | 本机已经有一份叫「{name}」的文件了。继续导入会**自动改一个名字**（{suggested}），两份都留着 —— 原来那份一个字都不会动。 | A D | 已有同名文件。继续导入会改名为「{suggested}」，两份都保留。 | 已有同名檔案。繼續匯入會改名為「{suggested}」，兩份都保留。 | 同名ファイルがあります。続行すると「{suggested}」に改名され、両方残ります。 | A file with this name exists. Continuing renames it to "{suggested}", keeping both. | — |
| ☐ | `presets.cmp.readonlyNote` | 只读对照 —— 要改值，点右下「进入编辑模式」 | A D | 只读。改值点「进入编辑模式」。 | 唯讀。改值點「進入編輯模式」。 | 読み取り専用。編集するには「編集モード」へ。 | Read-only. Enter edit mode to change values. | — |
| ☐ | `presets.cmp.subtitleReadonly` | 并排看 2~3 份（只读）；要改值点右下「进入编辑模式」 | A D | 并排比较 2~3 份。改值点「进入编辑模式」。 | 並排比較 2~3 份。改值點「進入編輯模式」。 | 2〜3 件を並べて比較。編集するには「編集モード」へ。 | Compare 2–3 presets. Enter edit mode to change values. | — |
| ☐ | `presets.cmp.subtitleEditing` | 正在编辑 —— 改完按「保存」写回各自那份文件 | D | 正在编辑。改完按「保存」。 | 正在編輯。改完按「儲存」。 | 編集中。終わったら「保存」を押してください。 | Editing. Press Save when done. | — |
| ☐ | `presets.cmp.closeDirty` | 关掉会丢掉未保存的改动 —— 先保存 | A | 有未保存的改动，先保存 | 有未儲存的變更，先儲存 | 未保存の変更があります。先に保存してください | Unsaved changes; save first | — |
| ☐ | `presets.cmp.enterTitle` | 进编辑模式：这一台的值就可以改了 | A | 进入编辑模式 | 進入編輯模式 | 編集モードに入る | Enter edit mode | — |
| ☐ | `presets.cmp.undoTitle` | 撤销（一次手势一条） | A | 撤销 | 復原 | 元に戻す | Undo | — |
| ☐ | `presets.cmp.registryFailed` | 读不到参数注册表 —— 对比台起不来 | E | 读不到参数数据，无法对比 | 讀不到參數資料，無法比對 | パラメータデータを読めないため比較できません | Can't read parameter data, so comparison is unavailable | 「注册表」是内部说法 |
| ☐ | `presets.cmp.savedReloadFailed` | 保存后重读失败 —— 关掉重开一次看盘上的真实值 | A D | 已保存，但重读失败。关掉重开看实际值。 | 已儲存，但重讀失敗。關掉重開看實際值。 | 保存しましたが再読み込みに失敗しました。開き直して確認してください。 | Saved, but reload failed. Reopen to check. | — |
| ☐ | `presets.cmp.searchPlaceholder` | 搜索参数名或 toolhead.* | E | 搜索参数名 | 搜尋參數名稱 | パラメータ名を検索 | Search parameter names | 内部通配符不给用户看 |

## 5. 回执、错误与进度

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `presets.page.copy.done` | 已复制出一份：{name} —— 原文件一个字都没动 | B F | 已复制 {name} | 已複製 {name} | {name} を複製しました | Copied {name} | 安抚尾巴删 |
| ☐ | `presets.page.copy.doneClipped` | 已复制出一份：{name}（名字到了上限，截短了一点）—— 原文件一个字都没动 | B D F | 已复制 {name}（名字过长，已截短） | 已複製 {name}（名稱過長，已截短） | {name} を複製しました（名前が長いため短縮） | Copied {name} (name shortened) | — |
| ☐ | `presets.page.rename.done` | 已改名：{from} → {to} —— 只换了名字，内容一个字都没动 | B F | 已改名：{from} → {to} | 已改名：{from} → {to} | 名前を変更：{from} → {to} | Renamed: {from} → {to} | — |
| ☐ | `presets.page.rename.noDefault` | {name} 没有默认名可恢复（它本来就不叫这个名字） | B F | {name} 没有默认名 | {name} 沒有預設名稱 | {name} に既定の名前はありません | {name} has no default name | — |
| ☐ | `presets.page.rename.restoredShifted` | 已恢复默认名：{from} → {to}（默认名已被占用，让开了编号） | B D | 已恢复默认名：{from} → {to}（原名被占用，编号已顺延） | 已還原預設名稱：{from} → {to}（原名被佔用，編號已順延） | 既定の名前に戻しました：{from} → {to}（使用中のため番号を繰り下げ） | Restored default name: {from} → {to} (number shifted) | — |
| ☐ | `presets.page.download.done` | 已下载 {name} —— 本地表里现在有它了 | B | 已下载 {name} | 已下載 {name} | {name} をダウンロードしました | Downloaded {name} | — |
| ☐ | `presets.page.download.doneSlicer` | 已下载 {name} —— 本地表里现在有它了；要进切片器再点「放入」 | B D | 已下载 {name}。要进切片器，点「放入」。 | 已下載 {name}。要進切片器，點「放入」。 | {name} をダウンロードしました。スライサーへは「入れる」で。 | Downloaded {name}. Click Add to slicer to use it. | — |
| ☐ | `presets.page.download.archivedDone` | 已更新 {name} —— 旧的那一份进了归档（archive/），没有删 | A B E | 已更新 {name}，旧版已移到「旧版本」 | 已更新 {name}，舊版已移到「舊版本」 | {name} を更新しました。旧版は「旧バージョン」へ移動 | Updated {name}; the old version is kept under Older versions | 路径删 |
| ☐ | `presets.page.download.tamperedDone` | 已更新 {name} —— 盘上那份认不出的，现在换成了目录登记的当前版本 | A E | 已更新 {name}，换成了云端当前版本 | 已更新 {name}，換成雲端目前版本 | {name} を更新しました。クラウドの現在版に置き換えました | Updated {name} to the current cloud version | — |
| ☐ | `presets.page.download.notDeliverable` | {name}：这一份不走「下载」—— 只有登记过的交付文件能取 | A E | {name}：这份不能下载 | {name}：這份不能下載 | {name}：このファイルはダウンロードできません | {name}: this file can't be downloaded | — |
| ☐ | `presets.page.remove.donePreset` | 已删除 {name} —— 它回到「未下载」，随时可以从云端重新下载 | B F | 已删除 {name}，可重新下载 | 已刪除 {name}，可重新下載 | {name} を削除しました。再ダウンロードできます | Deleted {name}. You can download it again. | — |
| ☐ | `presets.page.remove.doneLocal` | 已删除 {name} —— 已放进回收站，可在「设置 → 最近删除」找回 | B D | 已删除 {name}，可在「设置 → 最近删除」找回 | 已刪除 {name}，可在「設定 → 最近刪除」找回 | {name} を削除しました。「設定 → 最近削除」から復元できます | Deleted {name}. Restore it from Settings → Recently deleted. | — |
| ☐ | `presets.page.remove.archivedDone` | 已删除 {name} 的这份旧版本 —— 删了就找不回（云端只有最新版） | A B D | 已删除 {name} 的旧版本，无法找回 | 已刪除 {name} 的舊版本，無法復原 | {name} の旧版を削除しました。復元できません | Deleted an older version of {name}. This can't be undone. | — |
| ☐ | `presets.page.import.takenDone` | 已导入 {name} —— 原来那份没动，两份都留着（本地表里现在有它） | A B F | 已导入 {name}，原文件保留 | 已匯入 {name}，原檔案保留 | {name} をインポートしました。元のファイルは残ります | Imported {name}; the original is kept. | — |
| ☐ | `presets.page.pin.on` | 已把 {name} 排到最前（只影响这张表的顺序，不是「设为当前」） | A B F | 已置顶 {name} | 已置頂 {name} | {name} を先頭に固定しました | Pinned {name} to the top | — |
| ☐ | `presets.page.pin.off` | 已取消置顶 {name}（只影响这张表的顺序） | B | 已取消置顶 {name} | 已取消置頂 {name} | {name} の固定を解除しました | Unpinned {name} | — |
| ☐ | `presets.page.remark.saved` | 备注已保存 —— 以后更新不会覆盖它 | A B | 备注已保存，更新时会保留 | 備註已儲存，更新時會保留 | メモを保存しました。更新しても保持されます | Note saved; it survives updates | — |
| ☐ | `presets.page.remark.savedEmpty` | 备注已保存 —— 副标题留空（写什么就是什么，不回退） | A B F | 备注已清空，副标题不显示 | 備註已清空，副標題不顯示 | メモを空にしました。副題は表示されません | Note cleared; no subtitle shown | — |
| ☐ | `presets.page.remark.restored` | 已恢复默认备注 —— 回到工作台写的那句（没写就空着） | A B D | 已恢复默认备注 | 已還原預設備註 | 既定のメモに戻しました | Restored the default note | — |
| ☐ | `presets.page.edit.discarded` | 已放弃上次那些改动 —— 那份文件还是保存过的样子 | B D | 已放弃未保存的改动 | 已捨棄未儲存的變更 | 未保存の変更を破棄しました | Discarded unsaved changes | — |
| ☐ | `presets.page.edit.restored` | 已恢复成默认值 —— 这一步按「撤销」能退回来 | B | 已恢复默认值，可撤销 | 已還原預設值，可復原 | 既定値に戻しました。元に戻せます | Reset to defaults; undo is available | — |
| ☐ | `presets.page.edit.savedReopened` | 这份刚在「修改参数」里保存过 —— 编辑区已按最新内容重开 | B D | 编辑区已按最新内容重开 | 編輯區已按最新內容重新開啟 | 最新の内容で編集画面を開き直しました | Editor reopened with the latest content | — |
| ☐ | `presets.page.update.askPick` | {name} 有 {count} 项云端改过的参数等你挑 | D | {name}：{count} 项云端改动待选择 | {name}：{count} 項雲端變更待選擇 | {name}：クラウドの変更 {count} 件を選択してください | {name}: {count} cloud changes to review | — |
| ☐ | `presets.page.update.zeroPending` | 已更新 {name} —— 已经是云端最新版了 | B | 已更新 {name} | 已更新 {name} | {name} を更新しました | Updated {name} | — |
| ☐ | `presets.page.update.nothingToDo` | 云端 {name} 这一版没有要处理的（你机器上那几份都跟得上） | A D | {name} 这一版没有需要更新的 | {name} 這一版沒有需要更新的 | {name} のこの版に更新はありません | Nothing to update for this version of {name} | — |
| ☐ | `presets.page.update.gone` | {name} 已经不在本地了 —— 列表刚变过，再看一眼 | A F | {name} 已不在本机，列表可能刚变过 | {name} 已不在本機，清單可能剛變過 | {name} はこの端末にありません。一覧が更新された可能性があります | {name} is no longer here. The list may have changed. | — |
| ☐ | `presets.page.update.noLocalMatch` | 更新 {name} 没成：你机器上没有跟这个云端条目对得上的那一份 | A D F | 没更新 {name}：本机找不到对应文件 | 沒更新 {name}：本機找不到對應檔案 | {name} を更新できません：この端末に対応ファイルがありません | Couldn't update {name}: no matching file on this device | — |
| ☐ | `presets.page.update.unknownAttribution` | 更新 {name} 没成：认不出这一份属于哪台机型的哪一版 | A E | 没更新 {name}：认不出机型和版本 | 沒更新 {name}：認不出機型和版本 | {name} を更新できません：機種とバージョンを判別できません | Couldn't update {name}: machine or version unknown | — |
| ☐ | `presets.page.update.cloudNotReady` | 更新 {name} 没成：云端新版还没取回来（网络或数据源的问题） | A B | 没更新 {name}：云端新版还没取到 | 沒更新 {name}：雲端新版還沒取到 | {name} を更新できません：クラウドの新版を取得できていません | Couldn't update {name}: the new cloud version isn't available yet | — |
| ☐ | `presets.page.notify.unsavedJust` | 刚才那些改动还没保存、还没生效 —— 要生效得先保存 | D F | 改动还没保存 | 變更還沒儲存 | 変更はまだ保存されていません | Changes aren't saved yet | — |
| ☐ | `presets.page.notify.unsavedLast` | 上次那些改动还没保存、还没生效 —— 要生效得先保存（底栏「放弃并退出」= 作废这次改动） | A D F | 改动还没保存。底栏「放弃并退出」会作废这次改动。 | 變更還沒儲存。底欄「放棄並退出」會作廢這次變更。 | 変更はまだ保存されていません。下部の「破棄して終了」で破棄されます。 | Changes aren't saved. Discard and exit will throw them away. | — |
| ☐ | `presets.page.banner.needsNewer` | 此预设需要更新版 SupportEase —— 当前客户端版本过旧，暂不支持此预设文件 | A D | 这份预设需要新版客户端，请先更新 SupportEase | 這份預設需要新版用戶端，請先更新 SupportEase | このプリセットには新しいクライアントが必要です。SupportEase を更新してください | This preset needs a newer app. Update SupportEase first. | — |
| ☐ | `presets.page.unsupported.detail` | {machine} 在机型目录里，但后端一行资源都没写（它的 {count} 个版本一个都没配）。这不是你这台机器的问题，也不是加载失败。 | A B G | {machine} 还没有可用版本（{count} 个版本都没配置）。 | {machine} 還沒有可用版本（{count} 個版本都沒設定）。 | {machine} には利用できるバージョンがありません（{count} 件とも未設定）。 | {machine} has no available versions yet. | 「后端」「不是…也不是…」都删 |
| ☐ | `presets.page.bbs.mkpRecipe` | 这是 MKP 配方（toml），不是 BBS 工艺预设 | E | 这是 MKP 预设，不是 BBS 工艺预设 | 這是 MKP 預設，不是 BBS 工藝預設 | MKP プリセットであり、BBS の工程プリセットではありません | This is an MKP preset, not a BBS process preset | 「配方」改「预设」 |
| ☐ | `presets.page.bbs.notProfile` | 只认 BBS 的工艺预设，Orca 的字段表是另一套 | A E | 只支持 BBS 工艺预设 | 只支援 BBS 工藝預設 | BBS の工程プリセットのみ対応 | BBS process presets only | — |
| ☐ | `presets.page.bbs.notJson` | 不是 .json，BBS 查看器读不了 | A | 不是 .json，无法查看 | 不是 .json，無法檢視 | .json ではないため表示できません | Not a .json file, so it can't be viewed | — |
| ☐ | `presets.page.bbs.noOutlet` | 这一版的外壳没给跳转出口 | E | 当前版本无法跳转 | 目前版本無法跳轉 | この版では移動できません | Can't navigate in this version | — |
| ☐ | `presets.placements.putAllNote` | 把这一档里还没放的那些账号目录都补上 | D | 补齐还没放入的账号 | 補齊還沒放入的帳號 | 未設定のアカウントをすべて追加 | Add to all remaining accounts | — |
| ☐ | `presets.placements.otherSlicerNote` | 另一个切片器 · 字段表不一样，放进去不一定读得出来 | A D E | 另一个切片器的字段不同，可能读不出来 | 另一個切片器的欄位不同，可能讀不出來 | 別のスライサーは項目が異なるため、読み込めない場合があります | The other slicer uses different fields and may not read it | — |
| ☐ | `presets.page.loading` | 正在读取预设仓库… | E | 正在读取预设… | 正在讀取預設… | プリセットを読み込み中… | Loading presets… | 「仓库」是内部说法 |
| ☐ | `presets.page.count.mkp` | 云端 {cloud} · 本地 {local} | D | 云端 {cloud} · 本机 {local} | 雲端 {cloud} · 本機 {local} | クラウド {cloud} · ローカル {local} | Cloud {cloud} · local {local} | — |
