# 发布内核依赖的外部接口事实

> 只记**错了会让发布不正确**的那几条。第三方 SDK 的通用行为不在这里 —— 它们不是我们的产品。

## 1. Gitee：没有 Release 时回 200 + `null`

`GET /repos/{owner}/{repo}/releases/tags/<tag>` 在「这个 tag 上没有 Release」时**不是 404**，
而是 HTTP 200 + 空 body（读成 `null`）。

- 适配器：`platform/gitee.rs::find_release` 把 `null` 判成 `None`，并保留 404 分支；
- 错了的后果：状态派生把「没有暂存面」读成「有但没 id」，第一趟就挂在建 Release 那一步；
- 由谁盯：`ship --probe` 的第 1 条（打真接口，只读）。

## 2. GitHub：草稿不创建 tag，按 tag 查不到

`create_release` 带 `draft: true` 时 GitHub **不会创建 tag**；因此
`GET /releases/tags/<tag>` 对草稿返回 404，只能从 `GET /releases?per_page=100` 里按
`tag_name` 找回来（认证请求里草稿会出现在列表）。

- 适配器：`platform/github.rs::release_json` 先试 tag 端点，404 再退回列表；
- 错了的后果：暂存面永远读成「GitHub 上不全」，封版拦项永久为真；重复跑还会多建草稿；
- 由谁盯：`ship --probe` 的第 2 条。

## 3. 两边的资产身份字段不一样

| | 大小 | 摘要 | 附件 id |
|---|---|---|---|
| Gitee（`/attach_files`） | 给 | **不给** | 给 |
| GitHub（release 对象里的 `assets[]`） | 给 | 给 `digest: sha256:…` | 给 |

- 适配器：统一成 `RemoteAsset { name, size, sha256, id, url }`；
- **「读不到大小」不等于「一样大」**：`staging::upload_missing` 只在拿到大小且相等时才判
  「已在远端、跳过上传」，否则先删再传（`Gitee` 没有摘要，所以摘要那一格只能宽松）；
- 错了的后果：远端留着旧包、清单写着新 SHA，客户端校验必然失败；
- 由谁盯：`ship --probe` 的第 3 条 + 演练 `an_unknown_remote_size_is_not_the_same_bytes`。

## 4. CI 产物是临时路径

`upload-artifact` 打出来的 zip 解包在 `target/release-artifacts/run-<id>/` 下，**不是长期住址**。

- 适配器：`ArtifactFact::from_path` 保留本地路径；封版重传前用
  `staging::ensure_local` 补齐（同机找解包目录，换机器就从暂存面下载并核大小 + SHA）；
- 错了的后果：封版时「只有身份没有本地文件」，永远发不出去；
- 由谁盯：演练 `another_machine_resumes_by_downloading_from_staging`。

## 5. 暂存 tag 不能带 `/`

Gitee 的 `/releases/tags/<tag>` 会被斜杠切成另一个路径，读回来是 `null`。

- 适配器/常量：`platforms::STAGING_PREFIX = "staging-"`；
- 由谁盯：`platforms::tests`（命名约束）+ `ship --probe` 的第 2 条（真实 tag 名能读回来）。

## 6. 封版 tag 指向冻结源码

`workflow_dispatch` 的 `ref` 只能是分支/标签，所以 `workflow_run.head_sha` 是**触发时 main 的 tip**，
不是 `inputs.source_sha`。

- 适配器：`ci::wait` 按「触发之后新建的那次运行」认，不按 `head_sha`；被构建的是哪一笔由
  `build-receipt.json` 的 `source_sha` 与内核自算的 SHA 一起证明；
- 封版那一步用 `Git::tag_at(source)`，不是 `git tag`（后者打在当前 HEAD 上）；
- 由谁盯：演练 `another_machine_...` 里「正式 tag 必须指向冻结源码」那条断言 + `ship --probe`。

## 不在这份清单里的东西

上传/下载/删除/列表的通用行为不单独测 —— 由真实发布那一次覆盖。这里只钉住「错了会让
发布**静默地**发错东西」的那几条。

## 已知不覆盖（代价与理由）

| 项 | 现状 | 为什么现在不补 |
|---|---|---|
| 暂存 Release 的 `target_commitish` 与冻结源码不一致 | 只认 tag 指向的那一笔，不比对 Release 上的 `commitish` | 需要平台返回该字段；当前两个平台都没暴露，人造判据只会误报 |
| GitHub 草稿被谁提前发布过 | 不判（草稿 → 正式只在内核封版那一步做） | 人为操作；出错时封版的「两源一致」那条会拦住 |
| 清单 PR 的 head 是否就是刚推的那一笔 | 按分支名复用，不比对 head sha | 分支名带版本号，撞上别人的概率极低；比对需要额外一次 API |
| 暂存清理与核验的先后 | 先核验、后清理；清理失败会在下次重跑时重试 | 两步都是幂等操作，失败留下的现场不影响已封版那一份 |
| push tag 失败但本地 tag 已成 | 本地 tag 留着，重跑会判「同一笔，沿用」并重推 | 本地 tag 不参与「发没发」的判断（判据看远端与正式面） |
