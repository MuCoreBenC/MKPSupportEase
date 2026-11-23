//! **匿名使用统计**（`run/usage-ping.json`）：「今天有没有人用过」这一个事实。
//!
//! # 计数怎么分桶
//!
//! 计数服务（不蒜子）每被请求一次 +1，按 **Referer 的 origin** 分组返回三样：
//! `site_pv`（这个 origin 累计多少次）、`page_pv`（这个 origin + path 累计多少次）、
//! `site_uv`（它自己那套按 cookie 的去重，桌面端没有 cookie，**不用它**）。
//!
//! 于是请求这样造：
//!
//! ```text
//! Referer: https://mkp-usage-v2.test/2026-11-23/0.0.18-preview.2/macos
//!                                   └── 日期 ──┘└──── 版本 ────┘└ 平台 ┘
//! ```
//!
//! ★ **日期必须住在 path 里，不能住子域**：不蒜子的桶按**主域名 + path** 记，
//! 子域不参与（`d11-23.xxx.test` 与 `d11-24.xxx.test` 是同一个桶，实测）。
//! 所以一台机器一天发两次：`/<日期>/` 记"当天总数"，`/<日期>/<版本>/<平台>` 记分布。
//!
//! # 三条纪律
//!
//! 1. **一天最多一次**：`lastDay` 记在本地，当天已经报过就不再发 ——
//!    于是"当天请求数"就是"当天用过的机器数"，免费额度也永远吃得下。
//! 2. **不进启动路径**（铁律 2：启动零网络）：入口挂在「客户端成功拉到云端目录」
//!    之后，与自动检查更新同一趟。
//! 3. **失败一律静默**：计数服务挂了、断网、被挡，都不许影响用户 —— 这一层不是功能。
//!
//! 上报内容只有**日期 / 版本 / 平台**三样聚合值，不带设备标识，服务端也不认人；
//! 想看"这几天有没有人用"用 `node scripts/usage.mjs`（同一个分桶规则，见那边注释）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::AppError;
use crate::fsx;

use super::net;
use super::structure::APP_VERSION;

/// 状态文件代次。加字段不升号、改语义才升（与 `release.json` / `catalog.json` 同一条）。
pub const USAGE_SCHEMA: u32 = 1;

/// 计数接口。`jsonpCallback` 是它要求的固定形状；响应体这一层不解析。
const ENDPOINT: &str = "https://busuanzi.ibruce.info/busuanzi?jsonpCallback=cb";

/// 造 Referer 用的域名。**不需要能解析** —— 它只当分桶用的名字：
/// origin 决定"哪一天"，path 决定"哪个版本 + 哪个平台"。
const REFERER_HOST: &str = "mkp-usage-v2.test";

const STATE_FILE: &str = "run/usage-ping.json";

/// 本地记账：哪天已经报过。
///
/// 它不是业务数据，坏了就当"没记过"（[`load`] 的注释说了为什么）——
/// 这一层唯一的代价是当天可能多报一次，而"再也不报"会让统计直接归零。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageState {
    pub schema: u32,
    /// 最近一次成功上报的本地日期（`YYYY-MM-DD`）。`None` = 还没报过。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_day: Option<String>,
}

impl Default for UsageState {
    fn default() -> Self {
        Self {
            schema: USAGE_SCHEMA,
            last_day: None,
        }
    }
}

pub fn state_path(root: &Path) -> PathBuf {
    root.join(STATE_FILE)
}

/// 读本地记账。**读不到 / 坏了都当"没记过"**：这不是用户的数据，
/// 报错只会让统计永远停摆，而多报一次的代价只是当天数字多一位。
pub fn load(root: &Path) -> UsageState {
    let Ok(bytes) = std::fs::read(state_path(root)) else {
        return UsageState::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save(root: &Path, day: &str) -> Result<(), AppError> {
    let state = UsageState {
        schema: USAGE_SCHEMA,
        last_day: Some(day.to_owned()),
    };
    fsx::atomic::atomic_write_json(&state_path(root), &state)
}

/// 本地日期（`YYYY-MM-DD`）。取不到本地时区就按 UTC 走 —— 差一天不影响趋势。
pub fn today() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

/// 本机平台，口径与发布清单里 `platform` 那一列相同（`macos` / `windows`）。
pub fn platform() -> &'static str {
    std::env::consts::OS
}

/// 这条心跳的 Referer（纯函数，判据在测试里）。
///
/// 版本里的点与连字符在 path 上都是合法字符，不需要转义。
pub fn referer_for(day: &str, version: &str, platform: &str) -> String {
    format!("https://{REFERER_HOST}/{day}/{version}/{platform}")
}

/// 「当天总数」那一条：只带日期。脚本读它拿当天的台次，不必逐个版本去加。
pub fn referer_total_for(day: &str) -> String {
    format!("https://{REFERER_HOST}/{day}/")
}

/// 当天还没报过就报一次。返回"这一趟真的发了没有"。
pub fn ping_today(root: &Path) -> Result<bool, AppError> {
    /* 联网功能关着 ⇒ 连统计都不报（那是"出网"的一部分，见 runtime::online） */
    if !super::online::is_on(root)? {
        return Ok(false);
    }
    let day = today();
    if load(root).last_day.as_deref() == Some(day.as_str()) {
        return Ok(false);
    }
    /* 两条：总数与分布。**先总数后分布** —— 前一条失败就不记这一天，
     * 免得出现"有分布、没总数"那种读起来缺一块的日子。 */
    net::usage_ping(ENDPOINT, &referer_total_for(&day))?;
    net::usage_ping(ENDPOINT, &referer_for(&day, APP_VERSION, platform()))?;
    save(root, &day)?;
    Ok(true)
}

/// 顺路那一趟：**任何失败都只留一条 debug 日志**。
pub fn maybe_ping(root: &Path) {
    match ping_today(root) {
        Ok(true) => tracing::debug!("今天这一次使用已记账"),
        Ok(false) => {}
        Err(e) => tracing::debug!("使用统计这次没报成（不影响任何功能）：{}", e.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referer_carries_day_version_and_platform() {
        let r = referer_for("2026-11-23", "0.0.18-preview.2", "macos");
        assert_eq!(r, "https://mkp-usage-v2.test/2026-11-23/0.0.18-preview.2/macos");
    }

    /// 一天一个 path 段：换一天必须换桶（不蒜子只认主域名 + path，子域不算）
    #[test]
    fn a_new_day_is_a_new_path() {
        let a = referer_total_for("2026-11-23");
        let b = referer_total_for("2026-11-24");
        assert_ne!(a, b);
        assert!(a.contains("/2026-11-23/") && b.contains("/2026-11-24/"));
    }

    /// 同一天里版本/平台各占一个更深的段，总数那条是它们的前缀
    #[test]
    fn same_day_shares_the_day_segment() {
        let total = referer_total_for("2026-11-23");
        let m = referer_for("2026-11-23", "0.0.18-preview.2", "macos");
        let w = referer_for("2026-11-23", "0.0.18-preview.2", "windows");
        assert!(m.starts_with(&total), "{m} 该以 {total} 开头");
        assert!(w.starts_with(&total));
        assert_ne!(m, w);
    }

    /// 坏档 / 缺档都当"没记过"：宁可多报一次，也不能让统计从此停摆
    #[test]
    fn broken_or_missing_state_means_not_reported_yet() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load(d.path()).last_day, None);

        std::fs::create_dir_all(d.path().join("run")).unwrap();
        fsx::atomic::atomic_write(&state_path(d.path()), b"{ not json").unwrap();
        assert_eq!(load(d.path()).last_day, None);
    }

    /// 记过之后落盘、再读回来还是那天（写入形状的唯一判据）
    #[test]
    fn saved_day_round_trips() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        save(d.path(), "2026-11-23").unwrap();
        let got = load(d.path());
        assert_eq!(got.last_day.as_deref(), Some("2026-11-23"));
        assert_eq!(got.schema, USAGE_SCHEMA);
    }

    /// 日期就是 `YYYY-MM-DD`，不多不少（脚本按它拼 origin）
    #[test]
    fn today_is_a_plain_iso_day() {
        let t = today();
        assert_eq!(t.len(), 10, "不该带时间：{t}");
        assert_eq!(t.as_bytes()[4], b'-');
        assert_eq!(t.as_bytes()[7], b'-');
    }
}
