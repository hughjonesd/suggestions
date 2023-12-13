

/* 
TODO: 
- allow variable opening/closing tags
  - automatically recognize numbers of +/-/%
  - maybe also arbitrary strings embedded as "suggs add ++{ }++" or such
- colorized output (but also a vim plugin?)
- allow stdin as input to old/new

- options: handling comments (optional to strip in old/new; maybe separate command)
- testing: 
  - multiple
  - accept/reject
- writing a README and justification
- make author a &str, understand this stuff better
- visitor pattern?

- optionally sign output of diff DONE
- rename changetxt DONE
- accept/reject commands work on file in place DONE
*/

mod node;

use node::*;

use similar::{Algorithm, ChangeTag};
use similar::utils::diff_words;

use clap::{Parser, Subcommand, Args};

use std::fs::File;
use std::io;
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
    /// Output a suggestions file showing the difference from old to new
    Diff(DiffArgs),
    /// Output `file` with all changes rejected
    Old {file: String},
    /// Output `file` with all changes accepted
    New {file: String},
    /// Reject all changes in `file` in-place
    Reject {file: String},
    /// Accept all changes in `file` in-place
    Accept {file: String},
    /// Print `file` with changes highlighted in color
    Colorize {file: String},
}

#[derive(Args)]
struct DiffArgs {
    #[arg(short, long)]
    author: Option<String>,
    old: String, 
    new: String 
}


fn main() {
    let cli = Cli::parse();
    
    let _ = match &cli.command {
        Commands::Diff(DiffArgs{author, old, new}) => {
            command_diff(old, new, author)
        },
        Commands::Old{file} => {
            command_old(file)
        },
        Commands::New{file} => {
            command_new(file)
        },
        Commands::Reject{file} => {
            command_reject(file)
        },
        Commands::Accept{file} => {
            command_accept(file)
        },
        Commands::Colorize{file} => {
            command_colorize(file)
        },
    };
}


fn command_diff(old: &String, new: &String, author: &Option<String>) -> io::Result<()> {
    let result = make_suggestions_from_diff(old, new, author)?;
    Ok(println!("{}", result))
}


fn command_old(path: &String) -> io::Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.reject_to_string();
    Ok(println!("{}", suggs))
}


fn command_new(path: &String) -> io::Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.accept_to_string();
    Ok(println!("{}", suggs))
}


fn command_colorize(path: &String) -> io::Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.leave_to_colorized();
    Ok(println!("{}", suggs))
}


fn command_reject(path: &String) -> io::Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.reject_to_string();
    print_suggestions_to_file(suggs, path)
}


fn command_accept(path: &String) -> io::Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.accept_to_string();
    print_suggestions_to_file(suggs, path)
}


fn print_suggestions_to_file(string: String, path: &String) -> io::Result<()> {
    std::fs::write(path.as_str(), string.as_str())
}


fn make_node_from_file(path: &String) -> io::Result<Node> {
    let mut file = File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    make_node_from_string(text)
}


fn make_node_from_string(mut text: String) -> io::Result<Node> {
    let root = Node::root();
    // The vector of nodes that we are "in".
    let mut context = vec![root];

    let re_string = 
        // (?sx) is the s flag, which makes "." match even a newline;
        //       and the x "verbose" flag, to use whitespace.
        r"(?sx)
        (?<chunk_text>.*?)          # everything up to the author
        (?<author_text>@\S+?\s*)?   # optionally, an author tag (plus trailing whitespace)
        (?<tag>                     
            \+\+\[   |              # either an opener...
            --\[     |
            %%\[     |
            ]\+\+    |              # ... a closer ...
            ]--      |
            ]%%      |
            $                       # ... or EOF
        )                      
        (?<remainder>.*)            # everything that's left
        ";
    let re = Regex::new(re_string).unwrap();
    
    while text.len() > 0 {
        // read chunks up to the next marker (or EOF)
        text = {
            let Some(caps) = re.captures(&text) else {
                panic!("Regular expression failed to match")
            }; 
            let mut chunk_text = caps["chunk_text"].to_string();
            let author_text = &caps.name("author_text").map_or("", |m| m.as_str());
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
                // Create a node of the opener's type, add it to the context
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
                let finished_node = context.pop().unwrap();
                if let Some(cur_node) = context.last_mut() {
                    let correct_closer = closer(&finished_node.kind);
                    if tag != correct_closer {
                        panic!("Unmatched closing tag '{}', I was expecting '{}'", tag, correct_closer);
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

    panic!("Couldn't parse changetxt, was it empty?")
}


fn make_suggestions_from_diff(
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


fn make_node_from_diffs(changes: Vec<(ChangeTag, &str)>, author: &Option<String>) -> Node {
    // let author_string = if let Some(author) = author {
    //     format!(" {} ", author)
    // } else {
    //     "".to_string()
    // };

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
                    author: author.clone()
                };
                root.contents.push(Chunk::NodeChunk(nd));
            },
            (ChangeTag::Delete, text) => {
                let nd = Node {
                    kind: NodeKind::Deletion,
                    contents: vec![Chunk::TextChunk(text.to_string())],
                    author: author.clone()
                };
                root.contents.push(Chunk::NodeChunk(nd));
            }
        };
        
    }

    root
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_node_basic() {
        let txt = r"
        Original text.
        ++[An addition.]++
        More text. 
        --[Deleted text.]--
        %%[A comment.]%%
        ".to_string();
        make_node_from_string(txt).unwrap();
    }

    #[test]
    fn test_make_node_signed() {
        let txt = r"
        Original text.
        ++[An addition. @author1]++
        More text. 
        --[Deleted text. @author2 ]--
        %%[A comment. @author3]%%
        ".to_string();
        make_node_from_string(txt).unwrap();
    }

    #[test]
    fn test_make_node_nested() {
        let txt = r"
        Original text.
        ++[An addition. ++[A nested addition.]++ More of that addition.]++
        More text. 
        ++[An addition. --[Nested deletion.]-- More of that addition.]++
        ".to_string();
        make_node_from_string(txt).unwrap();
    }

    #[test]
    fn test_can_diff_files() {
        let path_old = "old.txt".to_string();
        let path_new = "new.txt".to_string();

        let test_output = make_suggestions_from_diff(&path_old, &path_new, &None).unwrap();
        let expected_output = "A ++[new ]++sentence.\n";
        assert_eq!(test_output, expected_output);

        let author = Some("@author1".to_string());
        let test_output = make_suggestions_from_diff(&path_old, &path_new, &author).unwrap();
        let expected_output = "A ++[new  @author1 ]++sentence.\n";
        assert_eq!(test_output, expected_output);
    }
}
