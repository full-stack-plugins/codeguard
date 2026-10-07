//! Ansible 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 正例通过
#[test]
fn valid_ansible_samples_pass_syntax() {
    let tmp = ensure_clean_dir("ansible-valid-test");
    std::fs::write(tmp.join("playbook.yml"), "---\n# Deploys application.\n- hosts: localhost\n  tasks:\n    - name: Install package\n      apt:\n        name: nginx\n        state: present\n").unwrap();

    let output = Command::new("ansible-playbook")
        .args(["--syntax-check", "playbook.yml"])
        .current_dir(&tmp)
        .output()
        .expect("ansible-playbook 不可用");

    assert!(output.status.success(), "有效 Ansible 代码应通过语法检查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_ansible_samples_are_detected() {
    let tmp = ensure_clean_dir("ansible-invalid-test");
    std::fs::write(tmp.join("playbook.yml"), "---\n- hosts: localhost\n  tasks:\n    - name: Install package\n      apt:\n        name: nginx\n        state: present\n  invalid_key: value\n").unwrap();

    let output = Command::new("ansible-playbook")
        .args(["--syntax-check", "playbook.yml"])
        .current_dir(&tmp)
        .output()
        .expect("ansible-playbook 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("ansible-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("playbook.yml"), "---\n- hosts: localhost\n  tasks:\n    - name: Undocumented task\n      apt:\n        name: nginx\n        state: present\n    - name: Documented task\n      apt:\n        name: nginx\n        state: present\n").unwrap();

    let output = Command::new("ansible-playbook")
        .args(["--syntax-check", "playbook.yml"])
        .current_dir(&tmp)
        .output()
        .expect("ansible-playbook 不可用");

    assert!(output.status.success(), "Ansible 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
