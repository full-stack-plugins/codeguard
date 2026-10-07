use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_sql_samples_pass_syntax_check() {
    let tmp = ensure_clean_dir("sql-valid-test");
    std::fs::write(tmp.join("schema.sql"), "CREATE TABLE users (\n    id INTEGER PRIMARY KEY,\n    name TEXT NOT NULL,\n    email TEXT UNIQUE\n);\n\n-- Insert a user\nINSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');\n").unwrap();

    let output = Command::new("sqlite3")
        .args([":memory:", ".read schema.sql"])
        .current_dir(&tmp)
        .output()
        .expect("sqlite3 不可用");

    assert!(output.status.success(), "有效 SQL 应通过语法检查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_sql_samples_are_detected() {
    let tmp = ensure_clean_dir("sql-invalid-test");
    std::fs::write(tmp.join("schema.sql"), "CREATE TABLE users (\n    id INTEGER PRIMARY KEY,\n    name TEXT NOT NULL,\n    email TEXT UNIQUE\n;\n").unwrap();

    let output = Command::new("sqlite3")
        .args([":memory:", ".read schema.sql"])
        .current_dir(&tmp)
        .output()
        .expect("sqlite3 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("sql-comment-test");
    // 格式正确但缺少注释——SQL 编译器不检查注释
    std::fs::write(tmp.join("schema.sql"), "CREATE TABLE products (\n    id INTEGER PRIMARY KEY,\n    name TEXT NOT NULL,\n    price REAL\n);\n\nCREATE TABLE orders (\n    id INTEGER PRIMARY KEY,\n    product_id INTEGER,\n    FOREIGN KEY (product_id) REFERENCES products(id)\n);\n").unwrap();

    let output = Command::new("sqlite3")
        .args([":memory:", ".read schema.sql"])
        .current_dir(&tmp)
        .output()
        .expect("sqlite3 不可用");

    assert!(output.status.success(), "SQL 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
