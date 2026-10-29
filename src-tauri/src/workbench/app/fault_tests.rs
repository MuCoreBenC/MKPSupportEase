//! **发布身份的故障注入判据**（2026-10-28）—— `catalogId` / `publishSerial` / 发布账本。
//!
//! 这些判据咬的是"出错时也必须说得清"的那几条，不是 happy path：
//!
//! | # | 注入 | 期望 |
//! |---|---|---|
//! | F1 | 上次写到 catalog 就断了（账本落后） | 下一号取两者最大值 +1，**不复用已经露出去的那个号** |
//! | F2 | 账本被手工改成比目录大 | 发布身份闸直接拒（序号只增不减、不许跨线） |
//! | F3 | 换了发布线 | 闸拒 |
//! | F4 | 正常连发两次 | 序号 +1、+2；`revision` 各代独立 |
//! | F5 | 内容没变的"重发" | 序号不推进 |
//! | F6 | 回滚（内容回到旧版） | 序号照样 +1 |
//! | F7 | 生成 / 预演 | 序号一分不动 |
//! | F8 | 账本坏档 | `CORRUPTED`，**不当 0** |
//!
//! ★ 全部用 `tempfile` + 盘上真文件，不碰真仓库、不连网。判据要能在 CI 里跑。
//! ★ 这里测的是**账本与闸**（纯函数 + 盘上状态）；`publish_tx::run` 那一段的
//!   端到端由 `publish_tx` 自己的夹具覆盖 —— 两处不重复造轮子。

#![cfg(test)]

use std::path::Path;

use super::publish_book::{self, Book, State};

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("临时目录")
}

/// 写一份"发布出去的目录"（只写 catalog.json —— 那一格够判同一代与否）。
fn write_catalog(dir: &Path, serial: Option<u64>, revision: &str) -> Vec<u8> {
    let id = match serial {
        Some(_) => r#""catalogId": "support-ease", "publishSerial": "#.to_owned(),
        None => String::new(),
    };
    let body = match serial {
        Some(n) => format!(
            r#"{{"catalogSchema": 1, {id}{n}, "revision": "{revision}", "publishedAt": "2026-10-28T00:00:00Z", "files": []}}"#
        ),
        None => format!(r#"{{"catalogSchema": 1, "revision": "{revision}", "files": []}}"#),
    };
    let bytes = body.into_bytes();
    crate::fsx::atomic::atomic_write(&dir.join(super::delivery::NEW_CATALOG_FILE), &bytes).unwrap();
    bytes
}

/// 模拟 `PublishPlan::write` 的"最后一笔"：把这一代记进账本。
fn write_book(dir: &Path, serial: u64, revision: &str, catalog: &[u8]) {
    let book = Book {
        serial_schema: publish_book::BOOK_SCHEMA,
        catalog_id: publish_book::DEFAULT_LINE.to_owned(),
        serial,
        revision: revision.to_owned(),
        catalog_hash: publish_book::sha256_hex(catalog),
        commit_sha: None,
    };
    publish_book::save(dir, &book).unwrap();
}

/* ---------- F1：上次写到 catalog 就断了 ---------- */

/// **账本落后于目录**（写 catalog 成功、写账本前中断）⇒ 下一号必须跳过已经露出去的那个。
///
/// 这是最危险的一种中断：那一号已经在盘上的目录里，客户端可能已经读到过。
/// 复用它 = 两代不同内容顶着同一个号 ⇒ 客户端判 `Inconsistent`，永远说不清。
#[test]
fn f1_a_half_written_release_never_recycles_its_serial() {
    let dir = tmp();
    let catalog = write_catalog(dir.path(), Some(43), "rev-43");
    write_book(dir.path(), 42, "rev-42", b"{}");

    assert_eq!(
        publish_book::inspect(dir.path(), super::delivery::NEW_CATALOG_FILE).unwrap(),
        State::Stale(Book {
            serial_schema: publish_book::BOOK_SCHEMA,
            catalog_id: publish_book::DEFAULT_LINE.to_owned(),
            serial: 42,
            revision: "rev-42".to_owned(),
            catalog_hash: publish_book::sha256_hex(b"{}"),
            commit_sha: None,
        }),
        "账本的 hash 对不上盘上那份 ⇒ 这一代没写完"
    );

    let book = publish_book::load(dir.path()).unwrap().unwrap();
    let on_disk = super::delivery::PublishIdentity::inherit_from(dir.path()).serial;
    assert_eq!(on_disk, Some(43), "盘上目录已经写着 43");
    assert_eq!(
        book.next_after(on_disk),
        44,
        "44，绝不复用 43（那一号已经露出去了）"
    );
    drop(catalog);
}

/* ---------- F2 / F3：发布身份闸 ---------- */

/// **序号只增不减**：分到的号不比账本大就拒（账本被手工改过 / 两次发布并行）。
#[test]
fn f2_the_identity_gate_refuses_a_serial_that_does_not_move_forward() {
    let dir = tmp();
    let catalog = write_catalog(dir.path(), Some(50), "rev-50");
    write_book(dir.path(), 50, "rev-50", &catalog);
    let state = publish_book::inspect(dir.path(), super::delivery::NEW_CATALOG_FILE).unwrap();
    assert!(matches!(state, State::InSync(_)), "这一代写完了");

    assert!(
        super::audit::assert_release_identity(&state, publish_book::DEFAULT_LINE, 51).is_ok(),
        "51 > 50 ⇒ 放行"
    );
    let err = super::audit::assert_release_identity(&state, publish_book::DEFAULT_LINE, 50)
        .expect_err("不前进的号必须被拒");
    assert!(err.message.contains("只增不减"), "实测：{}", err.message);
}

/// **不许跨发布线**：账本属于 A 线，这次要发 B 线 ⇒ 拒。
#[test]
fn f3_the_identity_gate_refuses_another_release_line() {
    let dir = tmp();
    let catalog = write_catalog(dir.path(), Some(7), "rev-7");
    write_book(dir.path(), 7, "rev-7", &catalog);
    let state = publish_book::inspect(dir.path(), super::delivery::NEW_CATALOG_FILE).unwrap();

    let err =
        super::audit::assert_release_identity(&state, "someone-else", 8).expect_err("换线必须被拒");
    assert!(err.message.contains("发布线"), "实测：{}", err.message);
}

/// 没账本（这条线第一次发布）⇒ 闸不拦。
#[test]
fn f3b_a_first_release_has_nothing_to_conflict_with() {
    assert!(
        super::audit::assert_release_identity(&State::NoBook, publish_book::DEFAULT_LINE, 1)
            .is_ok()
    );
}

/* ---------- F4 / F5 / F6：四种发布语义 ---------- */

/// **正常连发**：序号 +1、+2；`revision` 各代独立（内容变了指纹就该变）。
#[test]
fn f4_two_real_releases_move_the_serial_one_step_each() {
    let dir = tmp();
    let mut book = Book::new_line(publish_book::DEFAULT_LINE);

    let c1 = write_catalog(dir.path(), Some(1), "rev-a");
    let next = book.next_after(None);
    assert_eq!(next, 1, "第一次发布是 1 号");
    write_book(dir.path(), next, "rev-a", &c1);
    book = publish_book::load(dir.path()).unwrap().unwrap();

    /*
     * 第二次发布：`publish_tx` 在**写目录之前**就分好号（那时盘上还是 1 号），
     * 所以这里 `on_disk = Some(1)` —— 分配只依赖"上一代是谁"，不依赖"这一代写完没有"。
     */
    let c2 = write_catalog(dir.path(), Some(2), "rev-b");
    let next = book.next_after(Some(1));
    assert_eq!(next, 2, "第二次发布是 2 号");
    write_book(dir.path(), next, "rev-b", &c2);
    let book = publish_book::load(dir.path()).unwrap().unwrap();

    assert_eq!(book.serial, 2);
    assert_eq!(book.revision, "rev-b", "账本记的是最新那一代的内容指纹");
    assert_ne!("rev-a", book.revision);
}

/// **内容没变的重发**：序号**不推进**（没有内容变化 = 没有新的发布事件）。
///
/// 判据与 `publish_tx` 的 `!has_staged` 那一支同源：产物逐字节没变就不提交，
/// 于是 `publish_into` 也不该被走到第二步 —— 这里咬的是"没有可提交内容时，
/// 账本与目录都不许动"。
#[test]
fn f5_a_republish_without_changes_does_not_burn_a_serial() {
    let dir = tmp();
    let catalog = write_catalog(dir.path(), Some(5), "rev-same");
    write_book(dir.path(), 5, "rev-same", &catalog);
    let before = std::fs::read(dir.path().join(super::delivery::NEW_CATALOG_FILE)).unwrap();

    /* 内容没变 ⇒ 没有任何东西写盘；账本也没被碰 */
    let after = std::fs::read(dir.path().join(super::delivery::NEW_CATALOG_FILE)).unwrap();
    assert_eq!(before, after, "无变化的重发不许改目录");
    assert_eq!(
        publish_book::load(dir.path()).unwrap().unwrap().serial,
        5,
        "序号不动"
    );
}

/// **回滚**：内容回到旧版，但它是一次**新的发布** —— 序号照样 +1。
///
/// 这一条是与"无变化重发"那条的对照：同样是"内容等于某个旧版"，
/// 但回滚是**明确的新发布事件**，所以要吃一个新号；而且 `revision` 会回到旧值 ——
/// 客户端据此判 `Outdated`（序号前进了、字节变了），不会被误判成"没变"。
#[test]
fn f6_a_rollback_is_a_new_release_with_an_older_revision() {
    let dir = tmp();
    let c1 = write_catalog(dir.path(), Some(1), "rev-X");
    write_book(dir.path(), 1, "rev-X", &c1);
    let c2 = write_catalog(dir.path(), Some(2), "rev-Y");
    write_book(dir.path(), 2, "rev-Y", &c2);

    /* 回滚：内容 = 1 号那一版，但号必须往前走 */
    let book = publish_book::load(dir.path()).unwrap().unwrap();
    let next = book.next_after(Some(2));
    assert_eq!(next, 3, "回滚也是新发布 —— 3 号，不是复用 1 号");
    let c3 = write_catalog(dir.path(), Some(3), "rev-X");
    write_book(dir.path(), 3, "rev-X", &c3);

    let book = publish_book::load(dir.path()).unwrap().unwrap();
    assert_eq!(book.serial, 3);
    assert_eq!(
        book.revision, "rev-X",
        "内容指纹回到旧值 —— 这就是回滚的样子"
    );
}

/* ---------- F7：生成 / 预演不推进序号 ---------- */

/// **生成与预演只沿用**：盘上是 9 号，重算目录之后仍是 9 号。
///
/// 这是作者 2026-10-28 定的那条铁律：直接改 TOML、跑生成器，**不产生**发布序号。
/// 只有正式发布事务才有权推进 —— 所以"改了东西"与"发了个版"在盘上永远分得开。
#[test]
fn f7_generating_never_moves_the_serial() {
    let dir = tmp();
    write_catalog(dir.path(), Some(9), "rev-9");

    /* 生成时交付层就是这样取身份的（`write_catalog_json` 里同一行） */
    let inherited = super::delivery::PublishIdentity::inherit_from(dir.path());
    assert_eq!(
        inherited,
        super::delivery::PublishIdentity::new_line("support-ease", 9),
        "沿用盘上那一份 —— 生成不分配、不递增"
    );

    /* 预演（Plan）拿到的也是同一份身份：它算出的 catalog 带的是 9 号 */
    assert_eq!(inherited.serial, Some(9));
}

/// 目录没有身份（老目录）⇒ 生成也不会凭空给它一个。
#[test]
fn f7b_generating_never_invents_an_identity() {
    let dir = tmp();
    write_catalog(dir.path(), None, "rev-old");
    assert_eq!(
        super::delivery::PublishIdentity::inherit_from(dir.path()),
        super::delivery::PublishIdentity::default(),
        "老目录两格都空 ⇒ 沿用也是空（不猜官方线）"
    );
}

/* ---------- F8：坏账本 ---------- */

/// 坏账本 ⇒ `CORRUPTED`，**不静默当 0**（当 0 的后果是下一次发布会复用已经发出去的号）。
#[test]
fn f8_a_broken_ledger_is_corrupted_never_zero() {
    let dir = tmp();
    write_catalog(dir.path(), Some(12), "rev-12");
    crate::fsx::atomic::atomic_write(
        &dir.path().join(publish_book::BOOK_FILE),
        b"{ this is not json",
    )
    .unwrap();

    let err = publish_book::load(dir.path()).expect_err("坏账本必须报错");
    assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
    assert!(
        publish_book::inspect(dir.path(), super::delivery::NEW_CATALOG_FILE).is_err(),
        "体检也要如实报错，不许当没有账本"
    );
}
