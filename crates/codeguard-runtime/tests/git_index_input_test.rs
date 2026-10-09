//! Git index 输入测试

use codeguard_runtime::git_index_input::*;

#[test]
fn handle_git_index_file() {
    let input = GitInputHandler::handle(GitInputType::GitIndexFile, "/project/.git/index");
    assert_eq!(input.input_type, GitInputType::GitIndexFile);
    assert!(input.index_unchanged);
}

#[test]
fn handle_initial_commit() {
    let input = GitInputHandler::handle(GitInputType::InitialCommit, "/project");
    assert_eq!(input.input_type, GitInputType::InitialCommit);
}

#[test]
fn handle_worktree() {
    let input = GitInputHandler::handle(GitInputType::Worktree, "/project/worktree");
    assert_eq!(input.input_type, GitInputType::Worktree);
}

#[test]
fn handle_multi_ref() {
    let input = GitInputHandler::handle(GitInputType::MultiRef, "/project");
    assert_eq!(input.input_type, GitInputType::MultiRef);
}

#[test]
fn validate_index_unchanged() {
    let input = GitInputHandler::handle(GitInputType::GitIndexFile, "/project/.git/index");
    assert!(GitInputHandler::validate_index_unchanged(&input));
}

#[test]
fn validate_accurate_check() {
    let input = GitInputHandler::handle(GitInputType::InitialCommit, "/project");
    assert!(GitInputHandler::validate_accurate_check(&input));
}
