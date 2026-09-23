//! 步骤词汇与进度事件 —— tracing 与 progress 是**两条独立通道**。
//!
//! - 步骤 `id` 字符串与旧 Go 侧 `pipeline_spec.go:19-35` 的常量逐字一致
//!   （11 个继承 + Rust 侧新增的 `printtime`）。
//!   这是前后端/新旧实现互认的稳定标识，不是显示名。
//! - **显示名刻意不在这里**：中文步骤名（「输入 G-code」等）是 UI 边界的事，
//!   全局百分比/名称映射表只允许存在于 UI 一处（spec design.md §九.2）。
//! - `ProgressEvent` **不带全局百分比**：`EndProgress` 在旧侧就只用于失败时的
//!   前端进度估算（`pipeline_spec.go:41`），把它放进事件会造出第二个真相源。

/// 管线步骤。12 项，顺序即 [`STEPS`] 中的编排顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    Input,
    Config,
    MachineDetect,
    Parse,
    Support,
    Collision,
    CalibCheck,
    Pass1,
    Pass2,
    Calibration,
    Write,
    PrintTime,
}

/// 全部 12 步，按编排顺序。Task 14.3 的「有序子序列」断言以此为被测对象。
pub const STEPS: [Step; 12] = [
    Step::Input,
    Step::Config,
    Step::MachineDetect,
    Step::Parse,
    Step::Support,
    Step::Collision,
    Step::CalibCheck,
    Step::Pass1,
    Step::Pass2,
    Step::Calibration,
    Step::Write,
    Step::PrintTime,
];

impl Step {
    /// 稳定步骤 ID（与旧 Go 侧 `PipelineStepIDs` 常量集逐字一致，`printtime` 除外）。
    ///
    /// 无通配臂：新增步骤忘了给 ID 是编译错误。
    pub fn id(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Config => "config",
            Self::MachineDetect => "machine-detect",
            Self::Parse => "parse",
            Self::Support => "support",
            Self::Collision => "collision",
            Self::CalibCheck => "calib-check",
            Self::Pass1 => "pass1",
            Self::Pass2 => "pass2",
            Self::Calibration => "calibration",
            Self::Write => "write",
            Self::PrintTime => "printtime",
        }
    }

    /// `id()` 的逆映射（进度状态文件的 wire 值 → 步骤）。
    ///
    /// 无通配臂的**正向**枚举遍历不成立（入参是任意字符串），故这里以
    /// [`STEPS`] 为唯一来源做线性查找 —— 新增步骤只要在 `STEPS` 里登记就自动生效，
    /// 忘了登记会被 `step_ids_round_trip` 与 `Step::id` 的无通配臂 match 一起抓到。
    pub fn from_id(id: &str) -> Option<Step> {
        STEPS.iter().copied().find(|s| s.id() == id)
    }
}

/// 进度事件：只有步骤、步内比例、人类可读消息。
///
/// 刻意**没有**全局百分比字段 —— 旧侧 `EndProgress` 只用于失败进度估算，
/// 全局映射是 UI 边界的职责。谁往这里加 `overall_percent`，谁就在造第二个真相源。
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressEvent {
    pub step: Step,
    /// 步内完成比例 `0.0..=1.0`；`None` = 该步当前无法给出比例（如 support）。
    pub fraction_in_step: Option<f32>,
    pub message: String,
}

/// 进度出口。engine 只往 sink 推事件，**不写日志**（通道独立性见模块级测试）。
pub trait ProgressSink {
    fn emit(&mut self, event: ProgressEvent);
}

/// 任何 `FnMut(ProgressEvent)` 都是 sink（测试与 CLI 转发用）。
impl<F: FnMut(ProgressEvent)> ProgressSink for F {
    fn emit(&mut self, event: ProgressEvent) {
        self(event)
    }
}

/// 空 sink（`ProgressEvent` 必须被消费的静音场景，如库内自测）。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn emit(&mut self, _event: ProgressEvent) {}
}

/// 构造事件的唯一入口：clamp 到 0..=1，防止上层把 1.2 当 120% 发出去。
pub fn event(
    step: Step,
    fraction_in_step: Option<f32>,
    message: impl Into<String>,
) -> ProgressEvent {
    ProgressEvent {
        step,
        fraction_in_step: fraction_in_step.map(|f| f.clamp(0.0, 1.0)),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 步骤 ID 与旧 Go 侧常量集逐字一致（`printtime` 为 Rust 侧新增，登记在册）。
    #[test]
    fn step_ids_match_go_vocabulary() {
        let expected = [
            "input",
            "config",
            "machine-detect",
            "parse",
            "support",
            "collision",
            "calib-check",
            "pass1",
            "pass2",
            "calibration",
            "write",
            "printtime", // Rust 侧新增（旧侧 printtime 在 backup 步内，非独立步骤）
        ];
        assert_eq!(STEPS.len(), expected.len());
        for (step, id) in STEPS.iter().zip(expected) {
            assert_eq!(step.id(), id);
        }
        // id 两两不同（步骤身份不可碰撞）
        let mut ids: Vec<_> = STEPS.iter().map(|s| s.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), STEPS.len(), "步骤 ID 有碰撞");
    }

    /// Task 31.2.4：`id` ↔ `from_id` 对全部 12 步往返相等；未知 id 返回 None。
    ///
    /// 这条是进度状态文件（跨进程 wire）的身份判据：wire 上传的是 `id()` 字符串，
    /// 读侧必须能还原成同一个 `Step`，否则 GUI 的步骤高亮会静默错位。
    #[test]
    fn step_ids_round_trip() {
        for step in STEPS {
            assert_eq!(
                Step::from_id(step.id()),
                Some(step),
                "步骤 {step:?} 的 id 往返不相等"
            );
        }
        assert_eq!(Step::from_id("pass3"), None);
        assert_eq!(Step::from_id(""), None);
        assert_eq!(Step::from_id("Pass1"), None, "大小写不做宽松匹配");
    }

    /// tasks.md 2.5：progress 与 tracing 是两条独立通道 ——
    /// 行为判据：装一个捕获所有 tracing 事件的 layer，然后走一遍进度发射路径，
    /// **必须零日志事件**。engine 的发射路径若有人塞进 info!/warn!，这条立即红。
    #[test]
    fn emitting_progress_writes_zero_log_events() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<String>>>);
        impl<S> Layer<S> for Capture
        where
            S: tracing::Subscriber,
        {
            fn on_event(
                &self,
                event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let mut fields = String::new();
                let mut visitor = FieldStringer(&mut fields);
                event.record(&mut visitor);
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("{}: {fields}", event.metadata().level()));
            }
        }

        struct FieldStringer<'a>(&'a mut String);
        impl tracing::field::Visit for FieldStringer<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.0, "{}={value:?} ", field.name());
            }
        }

        let captured = Arc::new(Mutex::new(Vec::new()));
        let guard = {
            let layer = Capture(Arc::clone(&captured));
            // 用 set_default 而不是全局 set：不影响同进程其他测试的 subscriber
            tracing_subscriber::registry().with(layer).set_default()
        };

        // —— 进度发射路径（engine 唯一的 emit 形态）——
        let mut seen = Vec::new();
        {
            let mut sink = |e: ProgressEvent| seen.push(e);
            sink.emit(event(Step::Input, Some(0.0), "读取输入"));
            sink.emit(event(Step::Pass1, Some(0.5), "第一遍扫描"));
            sink.emit(event(Step::Pass2, None, "第二遍扫描"));
        }
        assert_eq!(seen.len(), 3, "sink 必须收到全部 3 个事件");
        assert_eq!(seen[1].step, Step::Pass1);
        assert_eq!(seen[2].fraction_in_step, None);

        drop(guard);
        assert!(
            captured.lock().unwrap().is_empty(),
            "进度发射路径产生了日志事件（通道被混用）：{:?}",
            captured.lock().unwrap()
        );
    }

    /// fraction 会被 clamp 进 0..=1；越界值不许原样透传。
    #[test]
    fn fraction_is_clamped() {
        let e = event(Step::Write, Some(1.5), "x");
        assert_eq!(e.fraction_in_step, Some(1.0));
        let e = event(Step::Write, Some(-0.3), "x");
        assert_eq!(e.fraction_in_step, Some(0.0));
    }
}
