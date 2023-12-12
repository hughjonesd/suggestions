

mod changetxt;

use changetxt::*;

use similar::{Algorithm, ChangeTag};
use similar::utils::diff_words;

use clap::{Parser, Subcommand};

use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::io::Read;

use regex::Regex;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand)]
enum Commands {
    /// Output a changetxt file showing the difference from old to new
    Diff { old: String, new: String },
    /// Output the old file from changetxt, with all changes rejected
    Old {changetxt: String},
    /// Output the new file from changetxt, with all changes accepted
    New {changetxt: String},
}


fn main() {
    let cli = Cli::parse();
    
    let result = match &cli.command {
        Commands::Diff{old, new} => {
            println!("old: {:?} new: {:?}", old, new);
            command_diff(old, new)
        },
        Commands::Old{changetxt} => {
            command_old(changetxt)
        },
        Commands::New{changetxt} => {
            command_new(changetxt)
        }
    };
}

// TODO allow optional author signing
fn command_diff(old: &String, new: &String) -> io::Result<()> {
    let result = make_changetxt_from_diff(old, new)?;
    Ok(println!("{}", result))
}

// TODO make stripping comments optional
fn command_old(changetxt: &String) -> io::Result<()> {
    let mut changetxt = make_changetxt_from_file(changetxt)?;
    let result = changetxt.reject_to_string();
    Ok(println!("{}", result))
}

// TODO make stripping comments optional
fn command_new(changetxt: &String) -> io::Result<()> {
    let mut changetxt = make_changetxt_from_file(changetxt)?;
    let result = changetxt.accept_to_string();
    Ok(println!("{}", result))
}


fn make_changetxt_from_file(path: &String) -> io::Result<Node> {
    let mut file = File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;


    let mut root = Node {
        author: None,
        contents: Vec::new(),
        kind: NodeKind::Root
    };
    // The vector of nodes that we are "in".
    let mut context = vec![root];

    let re_string = format!(
        // (?sv) is the s flag, which makes "." match even a newline;
        //       and the v flag, which means you can put whitespace.
        r"(?sv)
        (?<chunk_text>.*?)          # everything up to the author
        (?<author_text>@\S+\s*)?    # optionally, an author tag (plus trailing whitespace)
        (?<tag>{}|{}|{}|{}|{}|{}|$) # either an opener, closer or EOF
        (?<remainder>.*)            # everything that's left
        ",
        OPENERS[0], OPENERS[1], OPENERS[2], CLOSERS[0], CLOSERS[1], CLOSERS[2]
    );
    let re = Regex::new(re_string.as_str()).unwrap();
    
    while text.len() > 0 {
        // read chunks up to the next marker (or EOF)
        text = {
            let Some(caps) = re.captures(&text) else {
                panic!("Regular expression failed to match")
            }; 
            let mut chunk_text = caps["chunk_text"].to_string();
            let author_text = &caps["author_text"];
            let tag = &caps["tag"];

            if author_text.len() > 0 {
                if CLOSERS.contains(&tag) {
                    let author = author_text.trim().to_string();
                    context.last_mut().unwrap().author = Some(author);
                } else {
                    chunk_text.push_str(author_text);
                    eprintln!("Found possible handle {author_text} before opening tag or EOF");
                    eprintln!("Author handles should only be before a closing tag, like:");
                    eprintln!("  ++[Addition. @author ]++");
                }
            }

            if chunk_text != "" {
                let tc = Chunk::TextChunk(chunk_text);
                context.last_mut().unwrap().contents.push(tc);
            }

            if OPENERS.contains(&tag) {
                // - create a node of the opener's type and add it to the context vector.
                let nn_kind = match tag {
                    "++[" => NodeKind::Insertion,
                    "--[" => NodeKind::Deletion,
                    "%%[" => NodeKind::Comment,
                    _     => panic!("Weird opening tag {:?}", tag)
                };
                let new_node = Node {
                    author : None,
                    contents: Vec::new(),
                    kind : nn_kind
                };
                context.push(new_node);
                
            } else { // tag is a closer or EOF
                let mut finished_node = context.pop().unwrap();
                if let Some(cur_node) = context.last_mut() {
                    let cur_closer = closer(&cur_node.kind);
                    if tag != cur_closer {
                        panic!("Unmatched closing tag '{}', I was expecting '{}'", tag, cur_closer);
                    }
                    cur_node.contents.push(Chunk::NodeChunk(finished_node));
                } else {
                    // we're done, tag was EOF
                    return Ok(finished_node);
                }
            }

            // assign to 'text'
            (&caps["remainder"]).to_string()
        }; 
    }

    panic!("Couldn't parse changetxt file {}, was it empty?", path)
}


fn make_changetxt_from_diff(path_old: &String, path_new: &String) -> io::Result<String> {
    let mut file_old = File::open(path_old)?;
    let mut file_new = File::open(path_new)?;
    let mut contents_old = String::new();
    let mut contents_new = String::new();
    file_old.read_to_string(&mut contents_old)?;
    file_new.read_to_string(&mut contents_new)?;

    let changes = diff_words(Algorithm::Myers, &contents_old, &contents_new);

    Ok(changes_to_changetxt(changes)) 
}


fn changes_to_changetxt(changes: Vec<(ChangeTag, &str)>) -> String {
    let mut output = String::new();
    for change in changes {
        let change_str = match &change {
            (ChangeTag::Equal, text) => {
                text.to_string()       
            },
            (ChangeTag::Insert, text) => {
                TAGS_INSERTION[0].to_string() + text + TAGS_INSERTION[1]
            },
            (ChangeTag::Delete, text) => {
                TAGS_DELETION[0].to_string() + text + TAGS_DELETION[1]
            }
        };
        output.extend(change_str.chars());
    }

    output
}


#[test]
fn can_diff_files() {
    let path_old = "old.txt".to_string();
    let path_new = "new.txt".to_string();

    let test_output = make_changetxt_from_diff(&path_old, &path_new).unwrap();

    let expected_output = "A ++new ++sentence.\n";
    assert_eq!(test_output, expected_output);
}
