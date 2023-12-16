
//! # Suggestions
//!
//! 
//! `suggestions` provides functions to handle the 
//! [suggestions](https://hughjonesd.github.io/suggestions)
//! format.
//! 
//! Suggestions is a simple, human-readable diff 
//! and comment format.
//! 
//! The crate also builds the `suggs` 
//! binary to work with suggestions
//! files on the command line.


mod node;

pub use node::{Node, NodeKind, Chunk};
use node::*;

use similar::{Algorithm, ChangeTag};
use similar::utils::diff_words;

use anyhow::{Result, bail};

use std::fs::File;
use std::io;
use std::io::Read;

use regex::Regex;


pub fn make_node_from_file(path: &String) -> Result<Node> {
    let mut file = File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    make_node_from_string(text)
}

/// Make a Node from a string representing suggestions.
/// 
/// # Examples
/// 
/// ```
/// # use suggestions::make_node_from_string;
/// let text = String::from(
///     "Original text. ++[An addition.]++--[A deletion]-- More original text.");
/// let node = make_node_from_string(text);
/// ```
/// 
/// # Errors
/// 
/// Returns an error if the string was not a valid suggestions file.
pub fn make_node_from_string(mut text: String) -> Result<Node> {
    let root = Node::root();
    // The vector of nodes that we are "in".
    let mut context = vec![root];

    let re_string = 
        // (?sx) is the s flag, which makes "." match even a newline;
        //       and the x "verbose" flag, to use whitespace.
        r"(?sx)
        (?<chunk_text> .*?)              # everything up to the author
        (?<author_string> \ *@\S+?\s*)?  # optionally, an author tag (plus whitespace)
                                         # note '\ ' matches a single literal space
        (?<tag>                     
            \+\+\[   |                   # either an opener...
            --\[     |
            %%\[     |
            ]\+\+    |                   # ... a closer ...
            ]--      |
            ]%%      |
            $                            # ... or EOF
        )                      
        (?<remainder> .*)                # everything that's left
        ";
    let re = Regex::new(re_string).unwrap();
    
    while text.len() > 0 {
        // read chunks up to the next marker (or EOF)
        text = {
            let caps = re.captures(&text).unwrap(); 
            let mut chunk_text = caps["chunk_text"].to_string();
            let author_string = &caps.name("author_string").map_or("", |m| m.as_str());
            let tag = &caps["tag"];
            let mut remainder = caps["remainder"].to_string();

            if chunk_text.ends_with('\n') || author_string.ends_with('\n') {
                remainder = fix_newlines(remainder);
            }

            if author_string.len() > 0 {
                if CLOSERS.contains(&tag) {
                    context.last_mut().unwrap().author_string = 
                        Some(author_string.to_string());
                } else {
                    chunk_text.push_str(author_string);
                    eprintln!("Found possible handle {author_string} before opening tag or EOF");
                    eprintln!("Author handles should only be before a closing tag, like:");
                    eprintln!("  ++[Addition. @author ]++");
                }
            }

            if chunk_text != "" {
                let tc = Chunk::TextChunk(chunk_text);
                context.last_mut().unwrap().contents.push(tc);
            }

            if CLOSERS.contains(&tag) {
                let finished_node = context.pop().unwrap();
                if let Some(cur_node) = context.last_mut() {
                    let correct_closer = closer(&finished_node.kind);
                    if tag != correct_closer && tag != "//" {
                        bail!("Unmatched closing tag '{}', I was expecting '{}'.", tag, correct_closer);
                    }
                    if cur_node.kind == NodeKind::Comment {
                        bail!("Comments cannot contain other tags.");
                    }
                    cur_node.contents.push(Chunk::NodeChunk(finished_node));
                }
            }

            if OPENERS.contains(&tag) {
                // Create a node of the opener's type, add it to the context
                let nn_kind = match tag {
                    "++[" => NodeKind::Addition,
                    "--[" => NodeKind::Deletion,
                    "%%[" => NodeKind::Comment,
                    "//"  => match context
                                    .last_mut()
                                    .map_or(NodeKind::Root, |n| n.kind) {
                                NodeKind::Deletion => NodeKind::Addition,
                                NodeKind::Addition => NodeKind::Deletion,
                                _ => panic!("'//' can only be used within additions or deletions")
                            },
                    _     => panic!("Weird opening tag {:?}", tag)
                };
                let new_node = Node {
                    author_string : None,
                    contents: Vec::new(),
                    kind : nn_kind
                };
                context.push(new_node);  
            } 
                
            if tag.len() == 0 {
                let finished_node = context.pop().unwrap();
                return Ok(finished_node);
            }

            // assign to 'text'
            remainder.to_string()
        }; 
    }

    bail!("Couldn't parse text, was it empty?")
}


fn fix_newlines(mut remainder: String) -> String {
    // if tag is on its own on a line:
    let re_opening_ws = Regex::new(r"^\s*?\n").unwrap();
    if re_opening_ws.is_match(remainder.as_str()) {
        remainder = re_opening_ws.replace(remainder.as_str(), "").to_string();
    } 

    remainder
}

/// Return the difference between two files in suggestions format.
/// 
/// # Examples
/// 
/// ```rust
/// # use suggestions::make_suggestions_from_diff;
/// # let old_file = "resources/old.txt";
/// # let new_file = "resources/new.txt";
/// let suggestions = make_suggestions_from_diff(old_file, new_file, None);
/// ```
/// 
pub fn make_suggestions_from_diff(
    path_old: &str, 
    path_new: &str, 
    author: Option<String>
) -> io::Result<String> {
    let mut file_old = File::open(path_old)?;
    let mut file_new = File::open(path_new)?;
    let mut contents_old = String::new();
    let mut contents_new = String::new();
    file_old.read_to_string(&mut contents_old)?;
    file_new.read_to_string(&mut contents_new)?;

    let diffs = diff_words(Algorithm::Myers, &contents_old, &contents_new);

    let nd = make_node_from_diffs(diffs, author);
    let output = nd.to_string_suggestion();
    Ok(output) 
}


fn make_node_from_diffs(changes: Vec<(ChangeTag, &str)>, author: Option<String>) -> Node {
    let author_string = if let Some(author) = author {
        Some(format!(" {} ", author))
    } else {
        None
    };

    let mut root = Node::root();
    
    for change in changes {
        match change {
            (ChangeTag::Equal, text) => {
                root.contents.push(Chunk::TextChunk(text.to_string()));    
            },
            (ChangeTag::Insert, text) => {
                let nd = Node {
                    kind: NodeKind::Addition,
                    contents: vec![Chunk::TextChunk(text.to_string())],
                    author_string: author_string.clone()
                };
                root.contents.push(Chunk::NodeChunk(nd));
            },
            (ChangeTag::Delete, text) => {
                let nd = Node {
                    kind: NodeKind::Deletion,
                    contents: vec![Chunk::TextChunk(text.to_string())],
                    author_string: author_string.clone()
                };
                root.contents.push(Chunk::NodeChunk(nd));
            }
        };
        
    }

    root
}