//! **开发台 · 故障档位表**（作者 2026-10-19）。
//!
//! # 它解决什么
//!
//! 作者：「希望所有的模态框都能触发，而且要真实的 mock 触发」。
//!
//! 「开发台」原来只会**打开**一格界面（`devsim`）—— 打开的那一屏没有错可看，
//! 想看"出错时长什么样"只能靠真机撞上（比如目录过期要等云端发新版）。这一族反过来：
//! **不动界面，动后端** —— 给某一个 API 方法装一条错，客户端里**所有走这个方法的界面**
//! 从此真的 reject 一个 [`AppError`]。
//!
//! # ★★ 为什么挂在「方法」上，不挂在「模态框」上
//!
//! 挂在方法上：一个开关覆盖**所有**走它的界面，产品代码一行不改
//! （`src/api/bridge.ts` 那一层查一次表），加一档只改 [`faults`] 这一处。
//! 挂在模态框上：每个框都要在产品逻辑里留一个钩子 —— 那是"演示用代码住进产品"，
//! 而且每加一个框就多一处。作者选了前者。
//!
//! # ★★ 真值从哪来（不许抄第二份文案）
//!
//! 每一档的错都调**产品代码自己那句话**（[`crate::runtime::delivery::sha_mismatch_sentence`]
//! 那一族），不在这里重写一遍。抄一份的话改一处漂一处，**演示出来的反而是错的那句** ——
//! 那比不演示更糟。判据见本文件底部那条测试。
//!
//! # 为什么这一份**无条件编译**（与 `devsim` 同一条路）
//!
//! 造错发生在**客户端进程**里（工作台那条 loopback 只过一个 id），而客户端是
//! 普通构建（不带 `workbench` feature）。放 `workbench/` 下的话客户端根本编不出这条错。
//! 正式包里的处境与 `devsim` 一样：**没有命令发得进来**（那三条命令只挂在工作台
//! 那一份清单里），于是这个表恒不被问到 —— 它不是"藏起来"，是没有入口。

use serde::Serialize;

use crate::error::AppError;

/// 开发台上一档**可装的故障**。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevFault {
    /// 稳定 id（`delivery-stale` 之类）—— 命令按它分发
    pub id: String,
    /// **装在哪个 API 方法上**（前端 `src/api/contract.ts` 的 `MkpApiMethod` 名，
    /// camelCase，**逐字一样**）。客户端里所有走这个方法的界面都会中。
    pub method: String,
    /// 行上的名字
    pub label: String,
    /// 一句话说清它什么时候真出现 / 装了之后会看到什么
    pub note: String,
    /// 分组（界面按它分类摆）
    pub group: String,
    /// 组名（人话）
    pub group_label: String,
}

/// 装上之后的那一档：**方法 + 那一条错**（emit 给前端的 payload 就是这个形状）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArmedFault {
    /// 装在哪个方法上
    pub method: String,
    /// 那一条错（**真机同一份代码造出来的**）
    pub error: AppError,
}

/// 演示用的两个 revision —— 界面那句会原样摆出来（形状真、值是演示值）。
///
/// 为什么编一对：故障注入本来就是"造一个假故障"，而那一句的**骨架**
/// （"本机目录停在 X，云端已是 Y"）要看得见才验得了版面。
const DEMO_REMOTE: &str = "638456a9";
const DEMO_LOCAL: &str = "6374b7ce";

/// 演示用的文件名 —— 校准件那一支里最常被取回的那一件
const DEMO_FILE: &str = "ZOffset_Calibration.3mf";

/// 可装的故障清单。**唯一一份**：界面照它摆，命令照它分发。加一档只改这里。
pub fn faults() -> Vec<DevFault> {
    vec![
        DevFault {
            id: "delivery-stale".to_owned(),
            method: "downloadCatalogFile".to_owned(),
            label: "目录过期（要重启）".to_owned(),
            note: "云端出了新版、本机目录还是旧的。装了之后点「从云端获取」就会中 —— \
                   界面给「怎么解决」那块（关掉软件再打开一次）"
                .to_owned(),
            group: "delivery".to_owned(),
            group_label: "交付下载".to_owned(),
        },
        DevFault {
            id: "delivery-sha".to_owned(),
            method: "downloadCatalogFile".to_owned(),
            label: "文件真坏（重启没用）".to_owned(),
            note: "取不到远端、判不了目录过不过期时的保守说法。重启解决不了它 —— \
                   这一类**不该**出现「怎么解决」那块（指错路比不指更糟）"
                .to_owned(),
            group: "delivery".to_owned(),
            group_label: "交付下载".to_owned(),
        },
        DevFault {
            id: "delivery-size".to_owned(),
            method: "downloadCatalogFile".to_owned(),
            label: "大小对不上".to_owned(),
            note: "字节数与目录登记的不一致 —— 不是同一版。同样是「重下一遍」那一档".to_owned(),
            group: "delivery".to_owned(),
            group_label: "交付下载".to_owned(),
        },
        DevFault {
            id: "open-model-stale".to_owned(),
            method: "openModel".to_owned(),
            label: "打开模型 · 目录过期".to_owned(),
            note: "装在「打开模型」这一步上：能正常打开那个框、看到正常状态，\
                   点「从缓存打开」才中 —— 正是作者要看的那一屏"
                .to_owned(),
            group: "delivery".to_owned(),
            group_label: "交付下载".to_owned(),
        },
    ]
}

/// 按 id 造**那一条错** —— **在客户端进程里调**，于是文案 / 码与真机同一份代码。
///
/// `None` = 不认识这个 id（命令那一侧已经先拦过一次，这里是第二道）。
pub fn fault_error(id: &str) -> Option<AppError> {
    armed(id).map(|a| a.error)
}

/// 装一档要交给前端的那一份（方法 + 错）。`None` = 不认识。
pub fn armed(id: &str) -> Option<ArmedFault> {
    use crate::runtime::delivery as d;
    let row = faults().into_iter().find(|r| r.id == id)?;
    let error = match id {
        "delivery-stale" | "open-model-stale" => {
            let pair = (DEMO_REMOTE.to_owned(), DEMO_LOCAL.to_owned());
            AppError::sha_mismatch(d::sha_mismatch_sentence(DEMO_FILE, Some(&pair)))
                .with_message_code(d::sha_mismatch_code(true))
        }
        "delivery-sha" => AppError::sha_mismatch(d::sha_mismatch_sentence(DEMO_FILE, None))
            .with_message_code(d::sha_mismatch_code(false)),
        "delivery-size" => AppError::corrupted(d::size_mismatch_sentence(DEMO_FILE))
            .with_message_code(d::SIZE_MISMATCH_CODE),
        _ => return None,
    };
    Some(ArmedFault {
        method: row.method,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每一档都造得出一条错，且**都带稳定码**（没码的句子用户贴给 AI 时无从检索）
    #[test]
    fn every_fault_produces_a_coded_error() {
        for row in faults() {
            let e = fault_error(&row.id).unwrap_or_else(|| panic!("{} 造不出错", row.id));
            assert!(
                e.message_code.is_some(),
                "{} 这条错没盖码 —— 界面上就只是人话",
                row.id
            );
            assert!(!e.message.is_empty(), "{} 那句是空的", row.id);
        }
    }

    /// 故障 id **必须唯一**，而且都是 kebab-case ASCII（按 id 分发，改个汉字就静默失配）
    #[test]
    fn the_fault_ids_are_unique_stable_slugs() {
        let rows = faults();
        let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "故障 id 必须唯一");
        for r in &rows {
            assert!(
                r.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "故障 id 只能是 kebab-case ASCII：{}",
                r.id
            );
        }
    }

    /// `method` 是**前端**那边的名字（camelCase）—— 写成 snake_case 会**静永不命中**
    /// （拦截表查的是前端 `MkpApiMethod`），所以在这里钉住形状。
    #[test]
    fn the_methods_are_frontend_camel_case_names() {
        for r in faults() {
            assert!(
                r.method
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "method 只许是 ASCII 标识符：{}",
                r.method
            );
            assert!(
                r.method
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase()),
                "method 得是小驼峰（前端那套命名）：{}",
                r.method
            );
        }
    }

    /// ★★ **演示的错与真机的错一字不差** —— 这是"真实的 mock"那条要求的判据。
    ///
    /// 不认识的 id 必须给 `None`（不许兜一条"看起来像"的错出来）。
    #[test]
    fn the_injected_error_matches_the_real_one_and_unknown_ids_give_none() {
        use crate::runtime::delivery as d;
        let pair = (DEMO_REMOTE.to_owned(), DEMO_LOCAL.to_owned());
        let real = d::sha_mismatch_sentence(DEMO_FILE, Some(&pair));
        let demo = fault_error("delivery-stale").expect("这一档在");
        assert_eq!(demo.message, real, "演示那条错与真机那条必须一字不差");
        assert_eq!(
            demo.message_code.as_deref(),
            Some(d::sha_mismatch_code(true))
        );

        assert!(fault_error("不存在的故障").is_none(), "认不出就给 None");
    }

    /// 「文件真坏」那一档**不许**与「目录过期」同码 —— 界面按码决定给不给"怎么办"，
    /// 码串了就会对"重启没用"的故障指一条"重启"的路
    #[test]
    fn the_two_delivery_causes_keep_different_codes() {
        let stale = fault_error("delivery-stale").expect("在");
        let bad = fault_error("delivery-sha").expect("在");
        assert_ne!(stale.message_code, bad.message_code);
        assert_ne!(stale.message, bad.message);
    }
}
