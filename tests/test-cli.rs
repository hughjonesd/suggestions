
use std::process::Command;
use std::string::String;

fn suggs_output(args: &[&str]) -> String {
    println!("---------------------------------");
    let stdout = Command::new("target/debug/suggs")
        .args(args)
        .output().unwrap().stdout;
    String::from_utf8(stdout).unwrap()
}

#[test]
fn test_simple() {
    println!("{}", suggs_output(&["old", "resources/suggestions-simple.txt"]));
    println!("{}", suggs_output(&["new", "resources/suggestions-simple.txt"]));
}


#[test]
fn test_multiline() {
    println!("{}", suggs_output(&["old", "resources/suggestions-multiline.txt"]));
    println!("{}", suggs_output(&["new", "resources/suggestions-multiline.txt"]));
}


#[test]
fn test_nested() {
    println!("{}", suggs_output(&["old", "resources/suggestions-nested.txt"]));
    println!("{}", suggs_output(&["new", "resources/suggestions-nested.txt"]));
}

