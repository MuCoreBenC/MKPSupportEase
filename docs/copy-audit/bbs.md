# 工艺页（BBS）文案清单

> 判据 `docs/COPY-STYLE.md`；批注方式见 [`README.md`](README.md)。
> 这一页是技术视图，参数名与取值不译、不简化；只改界面自己说的话。

## 1. 左栏与预设抽屉

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `bbs.detailTitle` | 详情：统计 · 继承链 · 说明 | D | 详情 | 詳情 | 詳細 | Details | 三个子页签已在下方，不用预告；删行 |
| ☐ | `bbs.presetListTitle` | 预设列表：本机 BBS 里有哪些工艺预设 | D | 预设列表 | 預設清單 | プリセット一覧 | Presets | — |
| ☐ | `bbs.drawer.localTitle` | 你这台机器上原有的：本机 Bambu Studio 里的预设（你自己存的 + 软件自带的） | A D F | 本机 Bambu Studio 预设 | 本機 Bambu Studio 預設 | この端末の Bambu Studio プリセット | Local Bambu Studio presets | 括号里的分类列表已有分组标题 |
| ☐ | `bbs.drawer.mkpTitle` | 我们发的那几份工艺：预设页「切片器配置 → 云端」下载到本机的 | A D | 从「预设」下载的工艺 | 從「預設」下載的工藝 | 「プリセット」からダウンロードした工程 | Process presets downloaded from Presets | 路径说明放 tooltip |
| ☐ | `bbs.drawer.notDownloaded` | MKP 的工艺还没下载（资源区里有 {count} 份）—— 去「预设」页 → 切片器配置 → 云端，下载对应机型的那几份，回来就在这儿 | A D | 还没有下载。去「预设 → 切片器配置 → 云端」下载对应机型。 | 還沒下載。去「預設 → 切片器設定 → 雲端」下載對應機型。 | まだダウンロードしていません。「プリセット → スライサー設定 → クラウド」から該当機種を取得してください。 | Not downloaded yet. Get it from Presets → Slicer config → Cloud. | — |
| ☐ | `bbs.drawer.noneInAssets` | 产品资源区里没有 MKP 的工艺 | E | 还没有可下载的工艺 | 還沒有可下載的工藝 | ダウンロードできる工程がありません | No process presets to download | 「产品资源区」是内部名 |
| ☐ | `bbs.drawer.needDownloadHint` | —— 还没下载，点了去「预设」页 → 切片器配置 → 云端 取回来 | A D | 还没下载，点这里去取得 | 還沒下載，點這裡去取得 | 未ダウンロードです。クリックして取得 | Not downloaded; click to get it | 破折号开头来自拼接，结构问题 |
| ☐ | `bbs.drawer.readFailed` | 这几份工艺读不出正文（浏览器预览里没有下载区）—— 用桌面版能看到它们 | A D | 浏览器预览里读不到，请用桌面版 | 瀏覽器預覽讀不到，請用桌面版 | ブラウザプレビューでは読めません。デスクトップ版で表示してください | Not available in browser preview; use the desktop app | — |
| ☐ | `bbs.drawer.rescanTitle` | 重扫本机：重新读一遍本机 Bambu Studio 的预设目录（刚在 BBS 里新建的工艺预设会出现在这个列表里） | A D | 重新读取本机 Bambu Studio 预设 | 重新讀取本機 Bambu Studio 預設 | この端末の Bambu Studio プリセットを再読み込み | Rescan local Bambu Studio presets | 「重扫」是内部动作 |
| ☐ | `bbs.rail.localDesc` | 本机工艺 · 用户 {user} / 系统 {system} | D | 本机 · 用户 {user} / 系统 {system} | 本機 · 使用者 {user} / 系統 {system} | ローカル · ユーザー {user} / システム {system} | Local · user {user} / system {system} | — |
| ☐ | `bbs.rail.mkpDesc` | MKP 工艺 · 预设页「切片器配置」下载到本机的那几份 | D | MKP 工艺 · 从「预设」下载 | MKP 工藝 · 從「預設」下載 | MKP 工程 · 「プリセット」から取得 | MKP presets · downloaded from Presets | — |
| ☐ | `bbs.rail.mkpSourceTitle` | 这一份是我们发的：预设页 → 切片器配置 → 云端 下载，不在本机 Bambu Studio 的预设目录里 | A D | 来自「预设 → 切片器配置 → 云端」 | 來自「預設 → 切片器設定 → 雲端」 | 「プリセット → スライサー設定 → クラウド」由来 | From Presets → Slicer config → Cloud | — |
| ☐ | `bbs.rail.metaTitle` | 元数据（参数定义 / 版面 / 出厂默认）随本仓分发；清单才是实时读本机 BBS 目录的 | A E | 参数定义与出厂默认随程序分发，预设清单实时读取 | 參數定義與出廠預設隨程式分發，預設清單即時讀取 | パラメータ定義と既定値はアプリに同梱、一覧は端末から読み込みます | Definitions ship with the app; the list is read live | 「本仓」「清单才是」是开发者视角 |
| ☐ | `bbs.rail.allParamsTitle` | 把基准里有的行全铺出来（249 行那种） | A D | 显示全部参数 | 顯示全部參數 | すべてのパラメータを表示 | Show all parameters | 行数是实现细节 |
| ☐ | `bbs.rail.noParent` | —（没找到父预设，基准只有出厂默认） | A B | 没有父预设，只显示出厂默认 | 沒有父預設，只顯示出廠預設 | 親プリセットなし。出荷時の既定値のみ表示 | No parent preset; factory defaults only | — |
| ☐ | `bbs.rail.sameAsBbsTitle` | {note}；与 BBS 的界面看到的一致 | A | {note}；与 BBS 界面一致 | {note}；與 BBS 介面一致 | {note}；BBS の画面と同じ | {note}; matches what BBS shows | — |
| ☐ | `bbs.rail.synced` | · 元数据抄于 {date} | E | · 数据同步于 {date} | · 資料同步於 {date} | · データ同期 {date} | · Data synced {date} | 「抄」是内部动词 |
| ☐ | `bbs.rail.noMeta` | 没有元数据印章（public/bbs/_sync.json 缺失） | A E | 缺少参数定义数据 | 缺少參數定義資料 | パラメータ定義がありません | Parameter definitions are missing | 路径删 |
| ☐ | `bbs.systemBase` | —（系统预设就是基准） | B | 系统预设即基准 | 系統預設即基準 | システムプリセットが基準 | System preset is the baseline | — |
| ☐ | `bbs.group.other` | 其他机型 / 读不了的 | D | 其他机型或无法读取 | 其他機型或無法讀取 | 他の機種・読み込み不可 | Other machines or unreadable | — |
| ☐ | `bbs.noSource` | 没读到本机 BBS 预设目录：{note} | E | 读不到本机 BBS 预设目录：{note} | 讀不到本機 BBS 預設目錄：{note} | BBS プリセットフォルダを読めません：{note} | Can't read the local BBS preset folder: {note} | — |
| ☐ | `bbs.noSourceDesc` | 这一页列的是你本机 Bambu Studio 里的工艺预设 —— 本版不打包预设快照。 | A D | 这里列出本机 Bambu Studio 的工艺预设。 | 這裡列出本機 Bambu Studio 的工藝預設。 | この端末の Bambu Studio 工程プリセットを表示します。 | Lists process presets from local Bambu Studio. | 「不打包快照」是发布细节，删 |
| ☐ | `bbs.noSourceHow` | 装好 Bambu Studio（或它的 Beta），本页就能读到它的工艺预设。 | D | 装好 Bambu Studio 后即可读取。 | 裝好 Bambu Studio 後即可讀取。 | Bambu Studio をインストールすると読み込めます。 | Install Bambu Studio to read them. | — |

## 2. 行、搜索与面板

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `bbs.editedInMemory` | （已改 {count} 处，只在内存里） | A B | 已改 {count} 处（未保存） | 已改 {count} 處（未儲存） | {count} 件変更（未保存） | {count} changed (unsaved) | 「内存」是机制 |
| ☐ | `bbs.row.readOnlyHint` | · 这一页只能看，改了也不会保存 —— 不可点 | A D | · 只读 | · 唯讀 | · 読み取り専用 | · Read-only | 三句压成一个词 |
| ☐ | `bbs.row.unavailable` | · BBS 里此项当前不可用 | C | · BBS 中不可用 | · BBS 中不可用 | · BBS では使用不可 | · Unavailable in BBS | — |
| ☐ | `bbs.row.variantValue` | 按挤出头变体取第 {index} 个值；完整值 {value} | A E | 第 {index} 个挤出机的值：{value} | 第 {index} 個擠出機的值：{value} | {index} 番目の押出機の値：{value} | Extruder {index} value: {value} | 这是 tooltip，可留全值 |
| ☐ | `bbs.row.resetTitle` | 恢复默认值 {base} | C | 恢复默认值 | 還原預設值 | 既定値に戻す | Reset to default | 值已在格子里 |
| ☐ | `bbs.emptyScope` | 这一档下没有可显示的参数。切回「{all}」看看。 | D | 这一档没有参数，切回「{all}」 | 這一檔沒有參數，切回「{all}」 | この範囲にパラメータはありません。「{all}」に戻してください | No parameters in this scope. Switch back to "{all}". | — |
| ☐ | `bbs.noMatch` | 没有匹配「{query}」的参数 | C | 没有匹配「{query}」的参数 | 沒有符合「{query}」的參數 | 「{query}」に一致するパラメータはありません | No parameters match "{query}" | 可留 |
| ☐ | `bbs.searchPlaceholder` | 搜参数（名字 / key / 值）… | D | 搜索参数 | 搜尋參數 | パラメータを検索 | Search parameters | 「key」是内部字段 |
| ☐ | `bbs.hitsElsewhere` | 这一页没有命中，命中在： | D | 本页没有，命中在： | 本頁沒有，命中在： | このページにはありません。一致： | Not on this page. Matches: | — |
| ☐ | `bbs.hiddenHitsNote` | （有 {count} 个在「{same}」收起的行里） | B | {count} 个在「{same}」里 | {count} 個在「{same}」裡 | {count} 件は「{same}」内 | {count} more under "{same}" | — |
| ☐ | `bbs.rail.hiddenHitsLine` | 另有 {count} 个命中在「{same}」收起的行里，切到「{all}」能看到 | B D | 另有 {count} 个在「{same}」里 | 另有 {count} 個在「{same}」裡 | 他に {count} 件が「{same}」内にあります | {count} more under "{same}" | — |
| ☐ | `bbs.rail.searchLine` | 搜「{query}」· 命中 {hits} 项（这一页 {page}） | D | 「{query}」· {hits} 项 | 「{query}」· {hits} 項 | 「{query}」· {hits} 件 | "{query}" · {hits} matches | — |
| ☐ | `bbs.tabs.title` | {title} · {rows} 行 | D | {title} · {rows} 行 | {title} · {rows} 列 | {title} · {rows} 行 | {title} · {rows} rows | 可留 |
| ☐ | `bbs.tabs.titleHit` | {title} · {rows} 行 · 命中 {hits} | D | {title} · {rows} 行 · 命中 {hits} | {title} · {rows} 列 · 命中 {hits} | {title} · {rows} 行 · {hits} 件 | {title} · {rows} rows · {hits} matches | — |
| ☐ | `bbs.items` | {modified} / {stored} 项 | A | 已改 {modified} / 共 {stored} 项 | 已改 {modified} / 共 {stored} 項 | 変更 {modified} / 全 {stored} 件 | {modified} changed / {stored} total | 两个数并排看不出谁是谁 |
| ☐ | `bbs.panel.orphans` | 未在面板上的参数（{count}） | E | 未显示在面板的参数（{count}） | 未顯示在面板的參數（{count}） | パネル未表示のパラメータ（{count}） | Parameters not shown on the panel ({count}) | — |
| ☐ | `bbs.drawer.group` | {title}（{count}） | D | {title}（{count}） | {title}（{count}） | {title}（{count}） | {title} ({count}) | 全角括号改半角，四语统一 |
| ☐ | `bbs.drawer.target` | {label}（{count}） | D | {label}（{count}） | {label}（{count}） | {label}（{count}） | {label} ({count}) | 同上 |
| ☐ | `bbs.drawer.targetAll` | {label}（全部喷嘴，{count}） | D | {label}（全部喷嘴，{count}） | {label}（全部噴嘴，{count}） | {label}（全ノズル、{count}） | {label} (all nozzles, {count}) | — |

## 3. 备注、警告与错误

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `bbs.note.parentMissing` | 父预设「{name}」不在清单里：基准只剩出厂默认，改动判定会偏松 | A E | 父预设「{name}」不在，基准只剩出厂默认 | 父預設「{name}」不在，基準只剩出廠預設 | 親プリセット「{name}」がないため、基準は出荷時の既定値のみ | Parent preset "{name}" is missing; factory defaults only | 「改动判定会偏松」是实现后果 |
| ☐ | `bbs.note.unknownKeys` | 有 {count} 项参数表里没登记（新版 BBS 加的？） | A B | 有 {count} 项未登记的参数 | 有 {count} 項未登記的參數 | 未登録のパラメータが {count} 件 | {count} unregistered parameters | 猜测句删 |
| ☐ | `bbs.note.variantMissing` | 变体「{want}」没在系统预设里登记（登记的是 {list}），按第 1 个变体比对 | A D | 变体「{want}」未登记，按第 1 个变体比较 | 變體「{want}」未登記，按第 1 個變體比較 | バリアント「{want}」が未登録のため、1 番目で比較 | Variant "{want}" isn't registered; comparing with the first | — |
| ☐ | `bbs.note.fewParams` | 只识别到 {count} 个工艺参数，确认这是一份工艺预设？ | D | 只识别到 {count} 个工艺参数，确定这是工艺预设？ | 只識別到 {count} 個工藝參數，確定這是工藝預設？ | 工程パラメータが {count} 件のみです。このプリセットで正しいですか？ | Only {count} process parameters found. Is this a process preset? | — |
| ☐ | `bbs.note.chainLoop` | 继承链里有环，已截断 | A | 继承关系有循环，已停止 | 繼承關係有循環，已停止 | 継承に循環があるため停止しました | Inheritance loop detected; stopped | 「链」是内部词 |
| ☐ | `bbs.note.extruders` | 这份预设有 {count} 个挤出头，当前显示第 {index} 个 | D | {count} 个挤出机，当前显示第 {index} 个 | {count} 個擠出機，目前顯示第 {index} 個 | 押出機 {count} 台、現在 {index} 台目を表示 | {count} extruders; showing #{index} | 「挤出头」→「挤出机」 |
| ☐ | `bbs.note.droppedEdits` | 丢弃了上一份预设里 {count} 处未还原的改动 | D | 上一份有 {count} 处改动未还原，已丢弃 | 上一份有 {count} 處變更未還原，已捨棄 | 前のプリセットの未保存変更 {count} 件を破棄しました | Dropped {count} unrestored changes from the previous preset | — |
| ☐ | `bbs.support.badStyle` | 支撑样式「{bad}」在「{parent}」下不存在，已按 BBS 的做法改回「默认」 | A D | 「{bad}」在「{parent}」下不存在，已改为「默认」 | 「{bad}」在「{parent}」下不存在，已改為「預設」 | 「{bad}」は「{parent}」に存在しないため「既定」に変更 | "{bad}" doesn't exist under "{parent}"; reset to Default | — |
| ☐ | `bbs.reason.broken` | 文件解析失败：{error} | D | 文件解析失败：{error} | 檔案解析失敗：{error} | ファイルを解析できません：{error} | Couldn't parse the file: {error} | — |
| ☐ | `bbs.reason.otherTarget` | 属于 {printer} · {nozzle} | D | 属于 {printer} · {nozzle} | 屬於 {printer} · {nozzle} | {printer} · {nozzle} に属します | Belongs to {printer} · {nozzle} | — |
| ☐ | `bbs.noPreset` | —（还没选预设） | B | 还没选预设 | 還沒選預設 | プリセット未選択 | No preset selected | — |
| ☐ | `bbs.pending.missing` | 清单里没有「{name}」，已停在默认预设。 | D | 清单里没有「{name}」，已用默认预设。 | 清單裡沒有「{name}」，已用預設。 | 「{name}」が一覧にないため、既定のプリセットを使用します。 | "{name}" isn't in the list; using the default preset. | — |
| ☐ | `bbs.pending.notSelectable` | 清单里没有「{name}」（它不可选：{reason}），已停在默认预设。 | D | 「{name}」不可选（{reason}），已用默认预设。 | 「{name}」不可選（{reason}），已用預設。 | 「{name}」は選択不可（{reason}）。既定のプリセットを使用します。 | "{name}" can't be selected ({reason}); using the default. | — |
| ☐ | `bbs.preset.readFailed` | 读「{name}」失败 | D | 读不到「{name}」 | 讀不到「{name}」 | 「{name}」を読み込めません | Couldn't read "{name}" | — |
| ☐ | `bbs.data.liveFailed` | 读本机 BBS 目录失败：{error} | D | 读不到本机 BBS 目录：{error} | 讀不到本機 BBS 目錄：{error} | BBS フォルダを読めません：{error} | Can't read the local BBS folder: {error} | — |
| ☐ | `bbs.data.noDir` | 读不到本机 BBS 目录 | D | 读不到本机 BBS 目录 | 讀不到本機 BBS 目錄 | BBS フォルダを読み込めません | Can't read the local BBS folder | 可留 |
| ☐ | `bbs.data.notJson` | {path} 返回的不是 JSON（content-type: {type}），大概是路径对不上被回退成了页面 | A D | {path} 返回的不是 JSON（{type}） | {path} 回傳的不是 JSON（{type}） | {path} は JSON を返しません（{type}） | {path} didn't return JSON ({type}) | 「被回退成了页面」是调试推论 |
| ☐ | `bbs.data.missingMeta` | 没读到 public/bbs 下的 BBS 元数据（{error}）：那四份 json（registry / layout / defaults / icons）随本仓分发，少了说明产物不全，见 public/bbs/PROVENANCE.md。 | A D E | 缺少 BBS 参数数据（{error}）。请重新安装。 | 缺少 BBS 參數資料（{error}）。請重新安裝。 | BBS パラメータデータがありません（{error}）。再インストールしてください。 | BBS data is missing ({error}). Reinstall the app. | 面向开发者的长解释，删 |
| ☐ | `bbs.data.noBody` | 这一份不带正文：{scope} / {file} | E | 这份没有正文：{scope} / {file} | 這份沒有正文：{scope} / {file} | 本文がありません：{scope} / {file} | No body: {scope} / {file} | — |
| ☐ | `bbs.import.noProcessParams` | {file}：没有可识别的工艺参数（可能是机器或耗材预设），整份拒绝 | A B | {file}：没有工艺参数，未导入 | {file}：沒有工藝參數，未匯入 | {file}：工程パラメータがないため取り込みません | {file}: no process parameters, so it wasn't imported | 猜测原因删 |
| ☐ | `bbs.import.notObject` | {file}：顶层不是一个 JSON 对象，整份拒绝 | A E | {file}：不是有效的 JSON 对象，未导入 | {file}：不是有效的 JSON 物件，未匯入 | {file}：JSON オブジェクトではないため取り込みません | {file}: not a JSON object, so it wasn't imported | 「顶层」是解析术语 |
| ☐ | `bbs.delivered.failed` | 产品资源区读不出来 | E | 读不到可下载的工艺 | 讀不到可下載的工藝 | ダウンロード可能な工程を読めません | Can't read available presets | — |

## 4. 窗口胶囊与计数

| ☐ | 键 | 现文案（zh-CN） | 类型 | 建议 zh-CN | 建议 zh-TW | 建议 ja | 建议 en | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ☐ | `bbs.gadgets.pin.title` | 窗口置顶：{now} · 点一下改成「{next}」 | A D | 置顶：{now} → {next} | 置頂：{now} → {next} | 最前面：{now} → {next} | Always on top: {now} → {next} | tooltip 只写状态与下一档 |
| ☐ | `bbs.gadgets.compact.titleOn` | 小窗：600 宽、贴屏幕右侧、高度占屏幕可用区 90%，并自动置顶 | A D | 收成小窗并置顶 | 收成小視窗並置頂 | 小窓にして最前面に表示 | Shrink to a small window and pin it | 尺寸是实现细节 |
| ☐ | `bbs.gadgets.compact.titleOff` | 还原：回到上一次手动摆放的位置与大小，置顶也回到原来那一档 | A D | 还原窗口与置顶状态 | 還原視窗與置頂狀態 | ウィンドウと最前面設定を元に戻す | Restore the window and pin state | — |
| ☐ | `bbs.gadgets.compact.failed` | 窗口尺寸没改成 | D | 没能调整窗口大小 | 沒能調整視窗大小 | ウィンドウサイズを変更できませんでした | Couldn't resize the window | — |
| ☐ | `bbs.gadgets.pin.failed` | 置顶没改成 | D | 没能置顶 | 沒能置頂 | 最前面にできませんでした | Couldn't pin the window | — |
| ☐ | `bbs.detailTitleError` | 详情：统计 · 继承链 · 说明（有一条错误：{error}） | A D | 详情（{error}） | 詳情（{error}） | 詳細（{error}） | Details ({error}) | — |
