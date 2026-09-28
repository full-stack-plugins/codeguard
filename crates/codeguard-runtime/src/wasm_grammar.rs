use ring::digest::{SHA256, digest};
use tree_sitter::{
    Parser, Tree, WasmStore,
    wasmtime::{Config, Engine},
};

/// 经调用方固定清单校验的 Tree-sitter WASM grammar 的 Rust 加载与解析器。
/// 仅提供语法树；是否属于源码违规由上层规则和原生检查结果决定。
pub struct WasmGrammar {
    // 先释放解析器及其 WASM store，再释放其依赖的 Engine。
    parser: Parser,
    _engine: Engine,
    abi_version: usize,
}

impl WasmGrammar {
    /// 校验名称、大小、散列及真实 ABI 后离线加载 grammar。
    /// 参数为语种名、WASM 字节、预期 SHA-256 和预期 ABI；预期值必须来自可信的固定清单。
    pub fn load(
        name: &str,
        wasm: &[u8],
        expected_sha256: &str,
        expected_abi: usize,
    ) -> Result<Self, String> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-' || byte == b'_')
        {
            return Err("无效的 grammar 名称".into());
        }
        if wasm.len() < 8 || wasm.len() > 8 * 1024 * 1024 || !wasm.starts_with(b"\0asm") {
            return Err("WASM 字节无效或超过大小上限".into());
        }
        if expected_sha256.len() != 64
            || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("预期 SHA-256 格式无效".into());
        }
        let actual_sha256 = digest(&SHA256, wasm)
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual_sha256 != expected_sha256.to_ascii_lowercase() {
            return Err("WASM SHA-256 与固定资产不符".into());
        }

        // 默认 4 GiB 虚拟内存预留会先于 Linux worker 的地址空间上限失败。
        // 这仅缩小预留量；真正的进程硬限制由父进程设置并按平台验收。
        let mut config = Config::new();
        config
            .memory_reservation(64 * 1024 * 1024)
            .memory_guard_size(16 * 1024 * 1024)
            .max_wasm_stack(2 * 1024 * 1024);
        let engine = Engine::new(&config).map_err(|error| error.to_string())?;
        let mut store = WasmStore::new(&engine).map_err(|error| error.to_string())?;
        let language = store
            .load_language(name, wasm)
            .map_err(|error| error.to_string())?;
        let abi_version = language.abi_version();
        if abi_version != expected_abi {
            return Err(format!(
                "WASM ABI 不符：预期 {expected_abi}，实际 {abi_version}"
            ));
        }
        let mut parser = Parser::new();
        parser
            .set_wasm_store(store)
            .map_err(|error| error.to_string())?;
        parser
            .set_language(&language)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            parser,
            _engine: engine,
            abi_version,
        })
    }

    /// 返回已加载 grammar 的真实 ABI 版本。
    #[must_use]
    pub fn abi_version(&self) -> usize {
        self.abi_version
    }

    /// 解析源码字节并返回原始语法树；编码与语法错误需由上层检查。
    /// 当前尚未提供跨进程资源隔离，因此调用方不得传入不受控的大文件。
    pub fn parse(&mut self, source: &[u8]) -> Result<Tree, String> {
        if source.len() > 1024 * 1024 {
            return Err("语法初检源码超过大小上限".into());
        }
        self.parser
            .parse(source, None)
            .ok_or_else(|| "WASM 解析被中断或未能生成语法树".into())
    }
}
