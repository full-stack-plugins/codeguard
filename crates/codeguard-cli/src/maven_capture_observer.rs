use crate::maven_native_capture::MavenNativeCapture;
/// 显式、无全局状态的私有捕获回调；Err只传播采集缺口，不改旧产品报告。无Java对应类型。
pub type MavenCaptureObserver<'a> = dyn Fn(&MavenNativeCapture) -> Result<(), String> + Sync + 'a;
