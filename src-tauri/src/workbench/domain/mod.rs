//! Domain / Service 层 —— 规则与派生的唯一算处。
//!
//! doc §1 的第二、四条铁律落在这里：
//!
//! - **前端不实现任何业务规则**，所以三层怎么合、状态怎么算、哪条校验不过，全在这一层
//! - **状态只有一份**，由这一层派生给前端；不允许前端、快照文件、Rust 各存一套
//!
//! 七个子模块（Task 4–8 逐个落地）：
//!
//! | 子模块 | 管什么 |
//! |---|---|
//! | `layer` | 三层取值（出厂 → 机型基底 → 版本覆盖）与来源层 |
//! | `variants` | `machineVariants` 的三步归并（doc §3.3：纯机型键进基底 / 版本键全体一致则上提） |
//! | `visibility` | `showWhen` 链式求值 + 环检测 + `blockedBy` |
//! | `patch` | `Patch` 定义、应用、反向计算（撤销要的 inverse 由后端算） |
//! | `derive` | 状态派生（BookView / Matrix / BuildRow） |
//! | `preview` | 移动预览四组 + 批量影响范围 |
//! | `wording` | 状态词与解释句的唯一出处（doc §13） |
//!
//! Task 4 落地 `layer` / `variants`，Task 5 落地 `visibility`，Task 6 落地 `patch`，
//! Task 8 的 `wording` 提前到 Task 7 之前落地（`derive` 要用它的词，
//! 先写好才不用把同一批文案写两遍再搬一次）。

pub mod derive;
pub mod issues;
pub mod layer;
pub mod patch;
pub mod preview;
pub mod variants;
pub mod visibility;
pub mod wording;

#[cfg(test)]
pub mod testkit;


pub use derive::{Book, BookView, Col, ColRef, Matrix};
pub use layer::{Layers, Level, Origin, Overrides, ValueOrigin};
pub use preview::{BulkPreview, MovePreview};
pub use patch::{Committed, Draft, Patch, Visibility};
pub use variants::{digest, Digested};
pub use visibility::{BlockScope, BlockedBy, Gate};
pub use wording::{ArtifactState, BbsAssign, BbsSource, BuildState, SaveState};
