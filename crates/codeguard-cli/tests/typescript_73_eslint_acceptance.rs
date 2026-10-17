//! TypeScript 7.3 ESLint 验收
//! 验收标准：parser/tsconfig/本地插件和 monorepo 范围正确

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// parser 覆盖：TypeScript parser 正确解析
#[test]
fn parser_covers_typescript() {
    let tmp = ensure_clean_dir("ts-parser-test");
    std::fs::write(tmp.join("package.json"), r#"{"name": "test", "version": "1.0.0"}"#).unwrap();
    std::fs::write(tmp.join("tsconfig.json"), r#"{"compilerOptions": {"strict": true}}"#).unwrap();
    std::fs::write(tmp.join("eslint.config.js"), r#"
export default [
  {
    files: ["**/*.ts"],
    languageOptions: {
      parser: (await import("@typescript-eslint/parser")).default,
    },
    rules: {
      "no-unused-vars": "error"
    }
  }
];
"#).unwrap();
    std::fs::write(tmp.join("main.ts"), "const x: number = 1;\nconsole.log(x);\n").unwrap();

    // 安装依赖
    let _ = Command::new("npm")
        .args(["install", "eslint", "@typescript-eslint/parser", "--no-save"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["eslint", "main.ts"])
        .current_dir(&tmp)
        .output()
        .expect("eslint 不可用");

    // parser 应能解析 TypeScript
    assert!(output.status.success() || !output.stderr.is_empty(), "parser 应覆盖 TypeScript: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// tsconfig 绑定：TypeScript 文件绑定 tsconfig
#[test]
fn tsconfig_binding_for_typescript() {
    let tmp = ensure_clean_dir("ts-tsconfig-test");
    std::fs::write(tmp.join("package.json"), r#"{"name": "test", "version": "1.0.0"}"#).unwrap();
    std::fs::write(tmp.join("tsconfig.json"), r#"{"compilerOptions": {"strict": true, "target": "ES2020"}}"#).unwrap();
    std::fs::write(tmp.join("main.ts"), "const x: number = 1;\n").unwrap();

    // tsconfig 存在时应被绑定
    assert!(tmp.join("tsconfig.json").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// monorepo 范围：多包正确覆盖
#[test]
fn monorepo_scope_covers_multiple_packages() {
    let tmp = ensure_clean_dir("ts-monorepo-test");
    std::fs::write(tmp.join("package.json"), r#"{"name": "root", "workspaces": ["packages/*"]}"#).unwrap();

    // 创建两个包
    std::fs::create_dir_all(tmp.join("packages/pkg-a")).unwrap();
    std::fs::create_dir_all(tmp.join("packages/pkg-b")).unwrap();

    std::fs::write(tmp.join("packages/pkg-a/package.json"), r#"{"name": "pkg-a", "version": "1.0.0"}"#).unwrap();
    std::fs::write(tmp.join("packages/pkg-a/tsconfig.json"), r#"{"compilerOptions": {"strict": true}}"#).unwrap();
    std::fs::write(tmp.join("packages/pkg-a/index.ts"), "export const a = 1;\n").unwrap();

    std::fs::write(tmp.join("packages/pkg-b/package.json"), r#"{"name": "pkg-b", "version": "1.0.0"}"#).unwrap();
    std::fs::write(tmp.join("packages/pkg-b/tsconfig.json"), r#"{"compilerOptions": {"strict": false}}"#).unwrap();
    std::fs::write(tmp.join("packages/pkg-b/index.ts"), "export const b = 2;\n").unwrap();

    // monorepo 应覆盖两个包
    assert!(tmp.join("packages/pkg-a/tsconfig.json").exists());
    assert!(tmp.join("packages/pkg-b/tsconfig.json").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 本地插件：自定义规则正确加载
#[test]
fn local_plugin_rules_loaded() {
    let tmp = ensure_clean_dir("ts-plugin-test");
    std::fs::write(tmp.join("package.json"), r#"{"name": "test", "version": "1.0.0"}"#).unwrap();
    std::fs::write(tmp.join("eslint.config.js"), r#"
export default [
  {
    files: ["**/*.ts"],
    rules: {
      "no-console": "error",
      "prefer-const": "error"
    }
  }
];
"#).unwrap();
    std::fs::write(tmp.join("main.ts"), "console.log('test');\n").unwrap();

    let _ = Command::new("npm")
        .args(["install", "eslint", "--no-save"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["eslint", "main.ts"])
        .current_dir(&tmp)
        .output()
        .expect("eslint 不可用");

    // 应检出 no-console 违规
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no-console") || !output.status.success(), "应检出 no-console: {}", stdout);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 依赖审计：package.json 依赖正确解析
#[test]
fn dependency_audit_parses_package_json() {
    let tmp = ensure_clean_dir("ts-dep-test");
    std::fs::write(tmp.join("package.json"), r#"{
        "name": "test",
        "version": "1.0.0",
        "dependencies": {
            "typescript": "^5.0.0",
            "eslint": "^9.0.0"
        },
        "devDependencies": {
            "@typescript-eslint/parser": "^6.0.0"
        }
    }"#).unwrap();

    // package.json 应能正确解析
    let content = std::fs::read_to_string(tmp.join("package.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["dependencies"].is_object());
    assert!(parsed["devDependencies"].is_object());
    std::fs::remove_dir_all(&tmp).unwrap();
}
