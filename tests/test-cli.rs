
use std::process::{Command, Output};
use std::string::String;
use insta::assert_snapshot;

fn suggs_output(args: &[&str]) -> String {
    println!("---------------------------------");
    let stdout = suggs_run(args).stdout;
    String::from_utf8(stdout).unwrap()
}

fn suggs_run(args: &[&str]) -> Output {
    Command::new("target/debug/suggs")
        .args(args)
        .output().unwrap()
}

fn suggs_test_error(args: &[&str], error_str: &str) {
    let output = suggs_run(args);
    assert!(! output.status.success());
    let errmsg = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    assert!(errmsg.contains(error_str))    
}

#[test]
fn test_simple() {
    assert_snapshot!(suggs_output(&["old", "resources/suggestions-simple.txt"]));
    assert_snapshot!(suggs_output(&["new", "resources/suggestions-simple.txt"]));
}


#[test]
fn test_multiline() {
    assert_snapshot!(suggs_output(&["old", "resources/suggestions-multiline.txt"]));
    assert_snapshot!(suggs_output(&["new", "resources/suggestions-multiline.txt"]));
}


#[test]
fn test_nested() {
    assert_snapshot!(suggs_output(&["old", "resources/suggestions-nested.txt"]));
    assert_snapshot!(suggs_output(&["new", "resources/suggestions-nested.txt"]));
}


#[test]
fn test_colorize() {
    assert_snapshot!(suggs_output(&["colorize", "resources/suggestions-simple.txt"]));
}


#[test]
fn test_diff() {
    assert_snapshot!(suggs_output(&["diff", "resources/old.txt", "resources/new.txt"]));
    assert_snapshot!(suggs_output(&["diff", "--author", "author1", "resources/old.txt", "resources/new.txt"]));
}


#[test]
fn test_wrong() {
    suggs_test_error(&["old",
        "resources/suggestions-bad-nested-comment.txt"], 
        "comment");
    suggs_test_error(&["old", 
        "resources/suggestions-bad-unmatched-closer.txt"], 
        "unmatched");
    suggs_test_error(&["old", 
        "resources/suggestions-bad-unmatched-opener.txt"], 
        "unmatched");
}
