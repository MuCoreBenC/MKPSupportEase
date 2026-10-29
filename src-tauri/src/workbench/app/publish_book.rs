//! **发布账本** —— `presets/delivery/publish-serial.json`：这条发布线发到第几号了。
//!
//! ```text
//! { "serialSchema": 1, "catalogId": "support-ease", "serial": 43,
//!   "revision": "48a1517241b6da83", "catalogHash": "<catalog.json 全文 sha256>",
//!   "commitSha": "<这一版提交进 git 的那一笔，没有就省掉>" }
//! ```
//!
//! # 它解决的是哪一件事（作者 2026-10-28）
//!
//! `revision` 是**内容指纹**（SHA 前 16 位，不可比较大小），`publishedAt` 是**展示用时刻**，
//! `release_time` 是配方的写作日期 —— 这三样都回答不了「发布有没有往前走」。
//! 所以另立一个**只增不减的序号**，与**发布线身份**绑在一起：
//!
//! ```text
//! catalogId  这条线是谁（本程序自己的交付线 `support-ease`；GitHub / Gitee 镜像
//!            共享同一个值 —— 同一个仓库同一笔提交。**与厂商官方无关**）
//! serial     这条线的第几次发布（只有正式发布事务能让它 +1）
//! revision   那一次发布的内容指纹（回答"内容一样吗"，不回答"谁新"）
//! ```
//!
//! # 谁写、谁只读
//!
//! | 入口 | 能做什么 |
//! |---|---|
//! | `delivery::publish_into`（正式发布事务） | **唯一**分配下一个序号并写这本账 |
//! | `bin/gen-catalog` / 工作台「生成」 | **只读**，把上一次发布的值**沿用**进新目录 |
//! | `publish_tx` / `audit` | 只读 + 校验 |
//!
//! ★ 生成即使把目录内容改了个遍，`serial` 也一分不动 —— 有没有正式发出去，是两件事。
//!
//! # 坏档 / 缺档
//!
//! 缺文件 = 这条线还**没发布过**（合法，`serial` 从 1 开始）。
//! 坏 JSON / 代次认不出 = `CORRUPTED`，**不静默当 0** —— 当 0 的后果是下一次发布
//! 复用已经发出去过的号，客户端就再也分不出新旧了。
//!
//! # 与 catalog 的关系：**catalog 是权威**
//!
//! 一次发布要写四份东西（manifest / catalog / source / 本账），它们不是一次原子提交。
//! 所以规则只能是**单向**的：
//!
//! - 本账的 `serial` 落后于 `catalog.publishSerial` ⇒ 上次写到一半断了，按 catalog 校正
//!   （[`Book::next_after`] 取两者最大值再加一）；
//! - 本账的 `serial` **超前** catalog ⇒ 有人手工改过（或并行发了两次），`publish/identity`
//!   闸直接拦下，不猜。

use std::path::{Path, PathBuf};

use postprocess::diag::messages;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 账本文件名（在**交付根**里：`presets/delivery/publish-serial.json`）。
pub const BOOK_FILE: &str = "publish-serial.json";

/// 账本格式代次。加字段不升号、改语义才升（与 source / catalog / manifest 同一条）。
pub const BOOK_SCHEMA: u32 = 1;

/// **本仓这条线的身份**。GitHub / Gitee 镜像与随包 bootstrap 都是它 ——
/// 因为它们来自**同一个仓库的同一笔提交**，不是一个"推出来的相同"。
///
/// # 它是什么、不是什么（作者 2026-10-28 纠正）
///
/// 它是**本程序自己的交付线名字**，取自产品名（`productName: SupportEase`），
/// 只用来回答"这两份目录是不是同一条线的产品"。
///
/// ★ **它不代表任何官方身份**。这套预设是**第三方**整理与分发的，不是切片器厂商
///   （MKP / Bambu 等）的官方目录 —— 所以这一格**绝不许**写成 `official` 之类，
///   那是在替作者认领一个他没有的身份。往后要换名字只改这一行。
///
/// # 什么时候改、什么时候不许改
///
/// - **第一次正式发布之前**：随便改（还没有任何客户端读到过）；
/// - **那之后**：一格都不能动 —— 客户端按它判"同线 / 不同线"，改了等于凭空换线，
///   所有用户手上的副本会一次性变成 `elsewhere`（来源不同），不再提示更新。
///
/// 想做一条独立的测试线 / 实验线：**复制整个交付目录时把账本与 catalog 的 ID 一起改掉**，
/// 让它是另一条线；共用同一个 ID 又发不同内容，会被 `publish/identity` 判成矛盾的目录。
pub const DEFAULT_LINE: &str = "support-ease";

fn default_schema() -> u32 {
    BOOK_SCHEMA
}

/// 账本本体。**写在交付根里 = 进 git = 两个镜像天然同号**（同一个 commit 推两个远端）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    #[serde(default = "default_schema")]
    pub serial_schema: u32,
    /// 发布线身份。第一次发布时写定，此后只读（换线 = 手工改，闸会拦）。
    pub catalog_id: String,
    /// 这条线最近一次**正式发布**的序号。0 = 还没发布过。
    pub serial: u64,
    /// 那一次发布的内容指纹（`catalog.revision`）；还没发布过就是空串。
    #[serde(default)]
    pub revision: String,
    /// 那一次发布的 `catalog.json` 全文 sha256 —— 回答"账本说的那一版是不是盘上这一份"。
    #[serde(default)]
    pub catalog_hash: String,
    /// **上一代**那一代目录落进 git 的那一笔提交（`git log -1 -- presets/delivery/catalog.json`）。
    ///
    /// # 为什么是"上一代"，为什么不是"本次"
    ///
    /// 账本与 catalog 必须是**同一笔 commit**（否则"目录说 43、账本说 42"那种半截状态
    /// 就没人拦得住）。而记账本需要知道"这一笔的 sha"—— 那个 sha 只有 commit 之后才存在，
    /// commit 之后再写账本就得再提交一次（amend 是仓库明令避免的）。
    ///
    /// 所以口径定为：**下一次**发布落盘时，把**上一次**那一代的提交号写进来。
    /// 于是它永远滞后一代 —— 这不是缺陷，是"账本要跟目录同笔提交"的必然代价。
    ///
    /// # 它答什么、不答什么
    ///
    /// 答：**这一代目录是在哪一笔提交里落库的**（排障、追凶、跨机器对账）。
    /// 不答：**推没推出去** —— 本地提交也可能没 push 成功。想要"推了没有"去问 git，
    /// 不要拿这一格当"已发布到远端"的证据。
    ///
    /// 幂等恢复**不读它**（读的是 `head_touches` + [`State::InSync`]），所以它写错或
    /// 缺失都不会让恢复逻辑误判 —— 这也正是它敢"滞后一代"的底气。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

impl Book {
    /// 一条新线（还没发布过）。
    pub fn new_line(catalog_id: impl Into<String>) -> Self {
        Self {
            serial_schema: BOOK_SCHEMA,
            catalog_id: catalog_id.into(),
            serial: 0,
            revision: String::new(),
            catalog_hash: String::new(),
            commit_sha: None,
        }
    }

    /// **下一次发布该用几号**：账本与盘上 catalog 两者的最大值 + 1。
    ///
    /// 为什么取 max 而不是账本 +1（作者 2026-10-28 的恢复规则）：
    /// 上次可能写到 catalog 就断了（账本还是旧的），那一号**已经发出去了**
    /// （目录已经带着它躺在盘上），复用就是重号。所以只认"谁更大"。
    ///
    /// `on_disk` = 盘上 catalog 的 `publishSerial`（读不出 / 没有就是 `None`）。
    pub fn next_after(&self, on_disk: Option<u64>) -> u64 {
        self.serial.max(on_disk.unwrap_or(0)) + 1
    }

    /// 账本与盘上目录**是不是同一代**（`catalogHash` 对得上）。
    pub fn matches_catalog(&self, catalog_bytes: &[u8]) -> bool {
        !self.catalog_hash.is_empty() && self.catalog_hash == sha256_hex(catalog_bytes)
    }
}

/// catalog.json 全文的 sha256（小写 hex）。账本用它回答"盘上这一份是不是我说的那一版"。
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 账本落点：`<交付根>/publish-serial.json`。
pub fn book_path(delivery_root: &Path) -> PathBuf {
    delivery_root.join(BOOK_FILE)
}

/// 读账本。
///
/// 缺文件 = `Ok(None)`（这条线还没发布过，合法）；
/// 坏 JSON / 代次认不出 = `CORRUPTED`（**不静默当 0**：那等于允许重号）。
pub fn load(delivery_root: &Path) -> Result<Option<Book>, AppError> {
    let path = book_path(delivery_root);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io(messages::text("ui.workbench.book.500", &[]))
                .with_detail(format!("{}：{e}", path.display())))
        }
    };
    let book: Book = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::corrupted(messages::text("ui.workbench.book.501", &[]))
            .with_detail(format!("{}：{e}", path.display()))
    })?;
    if book.serial_schema != BOOK_SCHEMA {
        return Err(AppError::corrupted(messages::text(
            "ui.workbench.book.502",
            &[
                ("v1", &book.serial_schema.to_string()),
                ("v2", &BOOK_SCHEMA.to_string()),
            ],
        )));
    }
    if book.catalog_id.trim().is_empty() {
        /* 没有身份的账本比没有账本更糟：它会让"哪条线"这件事看起来已经定了 */
        return Err(AppError::corrupted(messages::text(
            "ui.workbench.book.503",
            &[],
        )));
    }
    Ok(Some(book))
}

/// 写账本（原子写）。**调用方负责顺序**：它必须和 catalog 在同一次发布里落地，
/// 且在 `git stage` 之前（这样两样东西进的是**同一笔** commit）。
pub fn save(delivery_root: &Path, book: &Book) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(book).map_err(|e| {
        AppError::internal(messages::text("ui.workbench.book.504", &[])).with_detail(e.to_string())
    })?;
    let body = format!("{text}\n");
    atomic_write(&book_path(delivery_root), body.as_bytes())
}

/// 读出来的账本 + 盘上那份 `catalog.json` 的字节，一起回答「这一代到底算不算发布完成」。
///
/// | 情形 | 结论 |
/// |---|---|
/// | 没账本 | `NoBook`（这条线从没正式发布过 —— 生成 / 预演走到这里都正常） |
/// | 账本 `catalogHash` == 盘上 catalog 的 sha | `InSync`（上一代发布完整落地了） |
/// | 对不上 | `Stale`（写到一半断了；下一次发布用 [`Book::next_after`] 继续往上） |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    NoBook,
    InSync(Book),
    Stale(Book),
}

/// 只读地看一眼交付根的状态。**不写盘、不修盘**。
pub fn inspect(delivery_root: &Path, catalog_file: &str) -> Result<State, AppError> {
    let Some(book) = load(delivery_root)? else {
        return Ok(State::NoBook);
    };
    let bytes = std::fs::read(delivery_root.join(catalog_file)).unwrap_or_default();
    Ok(if book.matches_catalog(&bytes) {
        State::InSync(book)
    } else {
        State::Stale(book)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("临时目录")
    }

    /// 缺文件 = 还没发布过（合法）；坏档 = `CORRUPTED`（不许当 0 —— 那会重号）。
    #[test]
    fn a_missing_book_is_a_new_line_but_a_broken_one_is_never_zero() {
        let dir = tmp();
        assert!(load(dir.path()).unwrap().is_none(), "没发过就是没账本");

        crate::fsx::atomic::atomic_write(&book_path(dir.path()), b"{ not json").unwrap();
        assert_eq!(
            load(dir.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );

        /* 身份空着也是坏的：它让"哪条线"看起来已经定了 */
        crate::fsx::atomic::atomic_write(
            &book_path(dir.path()),
            br#"{ "serialSchema": 1, "catalogId": "  ", "serial": 3 }"#,
        )
        .unwrap();
        assert_eq!(
            load(dir.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// **下一次序号取两者最大值 +1**：上次写到 catalog 就断了，那一号已经发出去了。
    #[test]
    fn the_next_serial_never_reuses_a_number_that_reached_the_catalog() {
        let mut book = Book::new_line("support-ease");
        book.serial = 42;

        assert_eq!(book.next_after(None), 43, "盘上没有数字 ⇒ 账本 +1");
        assert_eq!(book.next_after(Some(42)), 43, "两处一致 ⇒ 照样 +1");
        assert_eq!(
            book.next_after(Some(43)),
            44,
            "catalog 已经写了 43（上次写到一半）⇒ 44，绝不复用 43"
        );
        assert_eq!(
            book.next_after(Some(10)),
            43,
            "盘上更旧 ⇒ 以账本为准（不倒退）"
        );
    }

    /// 账本与盘上目录是不是同一代，只认 `catalogHash`；空 hash 一律不算匹配。
    #[test]
    fn only_the_catalog_hash_proves_the_same_generation() {
        let dir = tmp();
        let catalog = br#"{"catalogSchema":1,"revision":"abc","files":[]}"#;
        crate::fsx::atomic::atomic_write(&dir.path().join("catalog.json"), catalog).unwrap();

        let mut book = Book::new_line("support-ease");
        book.serial = 1;
        book.revision = "abc".to_owned();
        save(dir.path(), &book).unwrap();

        assert_eq!(
            inspect(dir.path(), "catalog.json").unwrap(),
            State::Stale(book.clone()),
            "空 catalogHash 不算匹配 —— 没写就说明上一代没写完"
        );

        book.catalog_hash = sha256_hex(catalog);
        save(dir.path(), &book).unwrap();
        assert_eq!(
            inspect(dir.path(), "catalog.json").unwrap(),
            State::InSync(book)
        );
    }

    /// **回填 `commit_sha` 不能把 `InSync` 弄坏**（2026-10-28）。
    ///
    /// `commit_sha` 是"上一代那一笔" —— 它永远是下一次发布才写进去的，也就是说
    /// 账本**每次发布都会变一格**。如果 `matches_catalog` 把账本自身也算进指纹，
    /// 那么"账本变了、catalog 没变"就会被判成 `Stale`，幂等恢复（只补 push）立刻失效，
    /// 还会连带让 `verify` 报假警。
    ///
    /// 所以它只算 `catalog.json` 的字节。这一条把它钉死。
    #[test]
    fn refilling_the_commit_sha_does_not_break_in_sync() {
        let dir = tmp();
        let catalog = br#"{"catalogSchema":1,"revision":"abc","files":[]}"#;
        atomic_write(&dir.path().join("catalog.json"), catalog).unwrap();

        let mut book = Book::new_line("support-ease");
        book.serial = 3;
        book.revision = "abc".to_owned();
        book.catalog_hash = sha256_hex(catalog);
        save(dir.path(), &book).unwrap();
        let before = inspect(dir.path(), "catalog.json").unwrap();
        assert!(matches!(before, State::InSync(_)), "第一代：同代");

        /* 下一次发布回填上一代的提交 */
        book.commit_sha = Some("4f56320".to_owned());
        save(dir.path(), &book).unwrap();
        let after = inspect(dir.path(), "catalog.json").unwrap();
        assert_eq!(
            after,
            State::InSync(book.clone()),
            "只改 commit_sha ⇒ 仍是同一代（幂等恢复的判断不受影响）"
        );
    }

    /// **内容没变就不改盘**：同一份账本存两次，字节完全一致（`atomic_write` 不比内容时会白写）。
    ///
    /// 这条咬的是"每次发布都必然产生账本 diff"那个隐患：如果序列化不稳定（键序、缩进、
    /// 末尾换行），即使值一样也会让 `git has_staged` 非空，于是"产物无变化"那一支永远
    /// 走不到 —— `serial` 就会被白白推进。
    #[test]
    fn saving_the_same_book_twice_writes_the_same_bytes() {
        let dir = tmp();
        let mut book = Book::new_line("support-ease");
        book.serial = 9;
        book.revision = "abc".to_owned();
        book.catalog_hash = "0".repeat(64);
        book.commit_sha = Some("4f56320".to_owned());

        save(dir.path(), &book).unwrap();
        let first = std::fs::read(book_path(dir.path())).unwrap();
        save(dir.path(), &book).unwrap();
        let second = std::fs::read(book_path(dir.path())).unwrap();
        assert_eq!(first, second, "同一份账本两次写盘必须逐字节相同");
    }
}
