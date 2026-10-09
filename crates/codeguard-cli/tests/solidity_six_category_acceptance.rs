//! Solidity 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn solc_bin() -> String {
    std::env::var("CODEGUARD_SOLC_BIN").unwrap_or_else(|_| "solc".to_string())
}

/// 正例通过
#[test]
fn valid_solidity_samples_pass_compile() {
    let tmp = ensure_clean_dir("solidity-valid-test");
    std::fs::write(tmp.join("main.sol"), "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.0;\n\n/// Adds two numbers.\ncontract Calculator {\n    function add(uint256 a, uint256 b) public pure returns (uint256) {\n        return a + b;\n    }\n}\n").unwrap();

    let output = Command::new(solc_bin())
        .args(["--bin", "main.sol"])
        .current_dir(&tmp)
        .output()
        .expect("solc 不可用");

    assert!(output.status.success(), "有效 Solidity 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_solidity_samples_are_detected() {
    let tmp = ensure_clean_dir("solidity-invalid-test");
    std::fs::write(tmp.join("main.sol"), "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.0;\n\ncontract Broken {\n    function broken(uint256 a, uint256 b) public pure returns (uint256) {\n        return a + b + notDefinedAnywhere;\n    }\n}\n").unwrap();

    let output = Command::new(solc_bin())
        .args(["--bin", "main.sol"])
        .current_dir(&tmp)
        .output()
        .expect("solc 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("solidity-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.sol"), "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.0;\n\ncontract Undocumented {\n    function undocumented(uint256 a) public pure returns (uint256) {\n        return a * 2;\n    }\n}\n\n/// Has docs.\ncontract Documented {\n    function documented(uint256 a) public pure returns (uint256) {\n        return a + 1;\n    }\n}\n").unwrap();

    let output = Command::new(solc_bin())
        .args(["--bin", "main.sol"])
        .current_dir(&tmp)
        .output()
        .expect("solc 不可用");

    assert!(output.status.success(), "Solidity 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
