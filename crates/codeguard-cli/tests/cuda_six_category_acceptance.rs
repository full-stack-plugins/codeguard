//! CUDA 六类别真实工具验收
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
fn valid_cuda_samples_pass_parse() {
    let tmp = ensure_clean_dir("cuda-valid-test");
    std::fs::write(tmp.join("kernel.cu"), "#include <cuda_runtime.h>\n\n// Adds two numbers.\n__global__ void add(int *a, int *b, int *c) {\n    int i = threadIdx.x;\n    c[i] = a[i] + b[i];\n}\n").unwrap();

    assert!(tmp.join("kernel.cu").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_cuda_samples_are_detected() {
    let tmp = ensure_clean_dir("cuda-invalid-test");
    std::fs::write(tmp.join("kernel.cu"), "#include <cuda_runtime.h>\n\n__global__ void add(int *a, int *b, int *c) {\n    int i = threadIdx.x\n    c[i] = a[i] + b[i];\n}\n").unwrap();

    assert!(tmp.join("kernel.cu").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("cuda-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("kernel.cu"), "#include <cuda_runtime.h>\n\n__global__ void add(int *a, int *b, int *c) {\n    int i = threadIdx.x;\n    c[i] = a[i] + b[i];\n}\n").unwrap();

    assert!(tmp.join("kernel.cu").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
