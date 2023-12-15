

mod node;

use node::*;

use similar::{Algorithm, ChangeTag};
use similar::utils::diff_words;

use anyhow::{Result, bail};

use std::fs::File;
use std::io;
use std::io::Read;

use regex::Regex;


pub fn print_suggestions_to_file(string: String, path: &String) -> Result<()> {
    Ok(std::fs::write(path.as_str(), string.as_str())?)
}


pub fn make_node_from_file(path: &String) -> Result<Node> {
    let mut file = File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    make_node_from_string(text)
}


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

            if OPENERS.contains(&tag) {
                // Create a node of the opener's type, add it to the context
                let nn_kind = match tag {
                    "++[" => NodeKind::Insertion,
                    "--[" => NodeKind::Deletion,
                    "%%[" => NodeKind::Comment,
                    _     => panic!("Weird opening tag {:?}", tag)
                };
                let new_node = Node {
                    author_string : None,
                    contents: Vec::new(),
                    kind : nn_kind
                };
                context.push(new_node);
                
            } else { // tag is a closer or EOF
                let finished_node = context.pop().unwrap();
                if let Some(cur_node) = context.last_mut() {
                    let correct_closer = closer(&finished_node.kind);
                    if tag != correct_closer {
                        bail!("Unmatched closing tag '{}', I was expecting '{}'.", tag, correct_closer);
                    }
                    if cur_node.kind == NodeKind::Comment {
                        bail!("Comments cannot contain other tags.");
                    }
                    cur_node.contents.push(Chunk::NodeChunk(finished_node));
                } else {
                    // we're done, tag was EOF
                    return Ok(finished_node);
                }
            }

            // assign to 'text'
            remainder.to_string()
        }; 
    }

    panic!("Couldn't parse changetxt, was it empty?")
}


pub fn fix_newlines(mut remainder: String) -> String {
    // if tag is on its own on a line:
    let re_opening_ws = Regex::new(r"^\s*?\n").unwrap();
    if re_opening_ws.is_match(remainder.as_str()) {
            remainder = re_opening_ws.replace(remainder.as_str(), "").to_string();
    } 

    remainder
}

pub fn make_suggestions_from_diff(
    path_old: &String, 
    path_new: &String, 
    author: &Option<String>
) -> io::Result<String> {
    let mut file_old = File::open(path_old)?;
    let mut file_new = File::open(path_new)?;
    let mut contents_old = String::new();
    let mut contents_new = String::new();
    file_old.read_to_string(&mut contents_old)?;
    file_new.read_to_string(&mut contents_new)?;

    let diffs = diff_words(Algorithm::Myers, &contents_old, &contents_new);

    let nd = make_node_from_diffs(diffs, author);
    let output = nd.leave_to_string();
    Ok(output) 
}


pub fn make_node_from_diffs(changes: Vec<(ChangeTag, &str)>, author: &Option<String>) -> Node {
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
                    kind: NodeKind::Insertion,
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


pub fn ensure_canonical_author(author: &mut String) {
    if ! author.starts_with('@') {
      author.insert(0, '@')
    }
    let re = Regex::new(r"^\S+$").unwrap();
    if ! re.is_match(author) {
         panic!("Author '{}' contained space characters", author);
    }
 }




#[test]
fn test_ensure_canonical_author() {
    let mut x = "author".to_string();
    ensure_canonical_author(&mut x);
    assert_eq!(x, "@author");

    let mut y = "@author".to_string();
    ensure_canonical_author(&mut y);
    assert_eq!(y, "@author");
}

#[test]
#[should_panic]
fn test_ensure_canonical_author_2() {
    let mut problematic = "@author with spaces".to_string();
    ensure_canonical_author(&mut problematic);
}
 