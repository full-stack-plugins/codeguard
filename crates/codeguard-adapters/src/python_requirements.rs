//! Python requirements 输入的保守静态分类；不解析依赖图或漏洞数据。

use std::collections::BTreeSet;

/// 行级精确版本状态；即使全为精确行，也不证明传递依赖闭包完整。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PythonRequirementsPins {
    /// 每条非注释行均为简单的 `name==version`。
    PinnedLines,
    /// 没有可审计的依赖声明。
    Empty,
    /// 包含范围、引用、标记、URL、可编辑包或其它未解析语法。
    Unresolved,
}

/// 仅识别简单精确 pin；复杂语法必须交给原生工具和锁模型解析。
#[must_use]
pub fn inspect_python_requirements_pins(bytes: &[u8]) -> PythonRequirementsPins {
    let Ok(content) = std::str::from_utf8(bytes) else {
        return PythonRequirementsPins::Unresolved;
    };
    let mut count = 0_usize;
    let mut names = BTreeSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, version)) = line.split_once("==") else {
            return PythonRequirementsPins::Unresolved;
        };
        let name = name.trim();
        let version = version.trim();
        if name.is_empty()
            || !name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || version.is_empty()
            || !version
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_digit())
            || !version.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+' | b'!')
            })
            || !names.insert(normalize_package_name(name))
        {
            return PythonRequirementsPins::Unresolved;
        }
        count += 1;
    }
    if count == 0 {
        PythonRequirementsPins::Empty
    } else {
        PythonRequirementsPins::PinnedLines
    }
}

fn normalize_package_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut separator = false;
    for byte in name.bytes() {
        if matches!(byte, b'-' | b'_' | b'.') {
            if !separator {
                normalized.push('-');
            }
            separator = true;
        } else {
            normalized.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{PythonRequirementsPins, inspect_python_requirements_pins};

    #[test]
    fn simple_pins_are_distinct_from_dynamic_or_incomplete_inputs() {
        assert_eq!(
            inspect_python_requirements_pins(b"requests==2.31.0\nurllib3==1.26.18\n"),
            PythonRequirementsPins::PinnedLines
        );
        assert_eq!(
            inspect_python_requirements_pins(b"# no dependencies\n"),
            PythonRequirementsPins::Empty
        );
        for input in [
            "requests>=2.31\n",
            "-r other.txt\n",
            "requests==2.31.0 ; python_version > '3.9'\n",
            "requests @ https://example.invalid/pkg.whl\n",
            "-e .\n",
            "requests==2.31.0\nRequests==2.30.0\n",
            "my_pkg==1.0\nmy.pkg==2.0\n",
        ] {
            assert_eq!(
                inspect_python_requirements_pins(input.as_bytes()),
                PythonRequirementsPins::Unresolved,
                "{input}"
            );
        }
    }
}
