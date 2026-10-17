//! Adapter 协议：capability/discover/resolve/plan/parse/coverage/fix 编译期注册
//!
//! 验收标准：adapter 不绕开运行时启动进程或联网

use serde::{Deserialize, Serialize};

/// Adapter 能力声明
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterCapability {
    /// 适配器 ID
    pub id: String,
    /// 语言标识
    pub language: String,
    /// 支持的类别
    pub categories: Vec<String>,
    /// 协议版本
    pub protocol_version: String,
}

/// Adapter 协议接口
pub trait AdapterProtocol {
    /// 声明能力
    fn capability(&self) -> AdapterCapability;
    
    /// 发现项目配置
    fn discover(&self, root: &std::path::Path) -> Result<DiscoveryResult, String>;
    
    /// 解析工具身份
    fn resolve(&self, tool: &str) -> Result<ToolIdentity, String>;
    
    /// 生成执行计划
    fn plan(&self, input: &PlanInput) -> Result<ExecutionPlan, String>;
    
    /// 解析原生报告
    fn parse(&self, report: &[u8]) -> Result<ParsedFindings, String>;
    
    /// 核对覆盖范围
    fn coverage(&self, expected: &[String], actual: &[String]) -> CoverageResult;
    
    /// 生成修复建议
    fn fix(&self, finding: &Finding) -> Option<FixSuggestion>;
}

/// 发现结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// 配置状态
    pub status: String,
    /// 发现的配置文件
    pub config_files: Vec<String>,
    /// 工具版本
    pub tool_version: Option<String>,
}

/// 工具身份
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolIdentity {
    /// 工具名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 路径
    pub path: String,
    /// SHA-256
    pub sha256: Option<String>,
}

/// 计划输入
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanInput {
    /// 源文件列表
    pub sources: Vec<String>,
    /// 配置路径
    pub config: Option<String>,
    /// 超时（毫秒）
    pub timeout_ms: u64,
}

/// 执行计划
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 命令参数
    pub argv: Vec<String>,
    /// 工作目录
    pub cwd: String,
    /// 环境变量
    pub env: Vec<(String, String)>,
}

/// 解析结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedFindings {
    /// 发现列表
    pub findings: Vec<Finding>,
    /// 是否完整
    pub complete: bool,
    /// 未完成原因
    pub incomplete_reason: Option<String>,
}

/// 发现
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// 规则 ID
    pub rule: String,
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: u32,
    /// 列号
    pub column: Option<u32>,
    /// 严重度
    pub severity: String,
    /// 消息
    pub message: String,
}

/// 覆盖结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageResult {
    /// 是否完整覆盖
    pub complete: bool,
    /// 未覆盖列表
    pub uncovered: Vec<String>,
}

/// 修复建议
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixSuggestion {
    /// 修复类型
    pub kind: String,
    /// 修复描述
    pub description: String,
    /// 是否安全自动应用
    pub safe_auto_apply: bool,
}

/// 编译期注册表
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn AdapterProtocol>>,
}

impl AdapterRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }
    
    /// 注册适配器
    pub fn register(&mut self, adapter: Box<dyn AdapterProtocol>) {
        self.adapters.push(adapter);
    }
    
    /// 按语言查找适配器
    pub fn find_by_language(&self, language: &str) -> Option<&dyn AdapterProtocol> {
        self.adapters.iter()
            .find(|a| a.capability().language == language)
            .map(|a| a.as_ref())
    }
    
    /// 列出所有能力
    pub fn capabilities(&self) -> Vec<AdapterCapability> {
        self.adapters.iter().map(|a| a.capability()).collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
