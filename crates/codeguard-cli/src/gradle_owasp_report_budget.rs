use serde_json::Value;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ADVISORIES: usize = 1000;
const MAX_FEEDBACK_BYTES: usize = 2 * 1024 * 1024;
const EXCEEDED: &str = "gradle_owasp_report_budget_exceeded";

/// 单次Gradle执行的累计报告预算；防止多任务及重复包标识扩大内存和反馈。
#[derive(Default)]
pub(crate) struct GradleOwaspReportBudget {
    input_bytes: usize,
    advisories: usize,
    feedback_bytes: usize,
}
impl GradleOwaspReportBudget {
    /// 返回剩余输入字节预算，调用方仍须限制单份报告为8MiB。
    pub(crate) fn remaining_input(&self) -> usize {
        MAX_INPUT_BYTES - self.input_bytes
    }
    /// 登记实际读取字节与原生漏洞数；溢出时拒绝整轮观察，不静默截断。
    pub(crate) fn reserve_input(
        &mut self,
        bytes: usize,
        advisories: usize,
    ) -> Result<(), &'static str> {
        let bytes = self
            .input_bytes
            .checked_add(bytes)
            .filter(|value| *value <= MAX_INPUT_BYTES)
            .ok_or(EXCEEDED)?;
        let advisories = self
            .advisories
            .checked_add(advisories)
            .filter(|value| *value <= MAX_ADVISORIES)
            .ok_or(EXCEEDED)?;
        self.input_bytes = bytes;
        self.advisories = advisories;
        Ok(())
    }
    /// 在保留单项反馈前登记序列化字节，计入数组分隔符的保守开销。
    pub(crate) fn reserve_feedback(&mut self, value: &Value) -> Result<(), &'static str> {
        let length = serde_json::to_vec(value).map_err(|_| EXCEEDED)?.len();
        self.feedback_bytes = self
            .feedback_bytes
            .checked_add(length)
            .and_then(|value| value.checked_add(2))
            .filter(|value| *value <= MAX_FEEDBACK_BYTES)
            .ok_or(EXCEEDED)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{GradleOwaspReportBudget, MAX_ADVISORIES, MAX_INPUT_BYTES};
    #[test]
    fn exact_input_boundary_and_overflow_do_not_wrap_or_erase_budget() {
        let mut budget = GradleOwaspReportBudget::default();
        budget
            .reserve_input(MAX_INPUT_BYTES, MAX_ADVISORIES)
            .unwrap();
        assert_eq!(budget.remaining_input(), 0);
        assert!(budget.reserve_input(1, 0).is_err());
        assert!(budget.reserve_input(0, 1).is_err());
        assert!(budget.reserve_input(usize::MAX, 0).is_err());
        assert_eq!(budget.remaining_input(), 0);
    }
}
