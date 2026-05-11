use std::fs;
use std::path::Path;

use yuna::aliases::{
    inject_alias, rc_file_for_shell, suggestion_for_command, AliasSuggestion,
};
use tempfile::tempdir;

#[test]
fn suggests_alias_for_known_transposed_git_typo() {
    let suggestion = suggestion_for_command("gti status").unwrap();

    assert_eq!(suggestion.alias, "gti");
    assert_eq!(suggestion.target, "git");
    assert_eq!(suggestion.line(), "alias gti='git'");
}

#[test]
fn suggests_alias_for_one_edit_common_command_typo() {
    let suggestion = suggestion_for_command("gir status").unwrap();

    assert_eq!(suggestion.alias, "gir");
    assert_eq!(suggestion.target, "git");
}

#[test]
fn ignores_commands_that_are_not_known_typos() {
    assert!(suggestion_for_command("git status").is_none());
    assert!(suggestion_for_command("deploy-prod now").is_none());
}

#[test]
fn selects_rc_file_from_shell_name() {
    let home = Path::new("/tmp/yuna-home");

    assert_eq!(rc_file_for_shell(home, "/bin/zsh"), home.join(".zshrc"));
    assert_eq!(
        rc_file_for_shell(home, "/usr/bin/bash"),
        home.join(".bashrc")
    );
    assert_eq!(rc_file_for_shell(home, "/bin/fish"), home.join(".profile"));
}

#[test]
fn inject_alias_appends_managed_line_once() {
    let dir = tempdir().unwrap();
    let rc_file = dir.path().join(".zshrc");
    fs::write(&rc_file, "export PATH=\"$HOME/bin:$PATH\"\n").unwrap();
    let suggestion = AliasSuggestion {
        alias: "gti".to_string(),
        target: "git".to_string(),
    };

    assert!(inject_alias(&rc_file, &suggestion).unwrap());
    assert!(!inject_alias(&rc_file, &suggestion).unwrap());

    let content = fs::read_to_string(&rc_file).unwrap();
    assert_eq!(content.matches("alias gti='git'").count(), 1);
    assert!(content.contains("# Added by Yuna"));
}
