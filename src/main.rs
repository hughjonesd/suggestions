

/* 
TODO: 

* means priority

- build binaries *
- document library *
  - clean it up, e.g. have an add() method
- allow --[ Deleted text // Added text ]++.
  - Note, this isn't gonna happen when people do addition first.
    It's most useful for deletions. 
    - Should you worry whether the ending tag is ]++ or ]--?
      People will probably forget if you make it be ]++.
      So I think --[ Deletion // Addition ]-- is the most natural format.
      Maybe you should warn if they put ]++ at the end. 
- allow variable opening/closing tags
  - automatically recognize numbers of +/-/% *
  - maybe also arbitrary strings embedded as "suggs add ++{ }++" or such
- bug: author followed by newline removes newline *
    Problem is that this:

    xxx
    ++[
    blah @foo
    ]++ 
    xxx
    
    becomes

    xxx
    blah xxx

    But note that it's ok if you put this:

    xxx
    ++[
    blah 
    @foo
    ]++ 
    xxx

    So I think maybe people can learn? Make it an issue.
    OTOH it's also true that the newline _before_ author stays in place.

- BUG: 
    suggs diff old.txt new.txt > diff.txt
    suggs old diff.txt
  adds newlines at the end. The diff adds one line. old (or new) adds a second line.

- vim syntax?
- allow stdin as input to old/new
- tex output
- options: handling comments 
  - options to strip or keep; maybe separate command*

- make author a &str, understand this stuff better
- visitor pattern?
    - something like "visit each node and replace tags with the following
      (either string or closure)"

*/


use suggestions::*;
use clap::{Parser, Subcommand, Args};
use anyhow::Result;
use regex::Regex;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand)]
enum Commands {
    /// Output diff from OLD to NEW in suggestions format
    Diff(DiffArgs),
    /// Output result of rejecting all changes in FILE
    Old {file: String},
    /// Output result of accepting all changes in FILE
    New {file: String},
    /// Overwrite FILE, rejecting all changes
    Reject {file: String},
    /// Overwrite FILE, accepting all changes
    Accept {file: String},
    /// Print suggestions FILE, highlight changes and comments
    Colorize {file: String},
    /// Print suggestions FILE with TeX highlighting
    Tex {file: String},

    #[command(hide = true)]
    Trousers {},
}

#[derive(Args)]
struct DiffArgs {
    /// Add AUTHOR to diff
    #[arg(short, long)]
    author: Option<String>,
    old: String, 
    new: String 
}


fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Diff(DiffArgs{author, old, new}) => {
            Ok(command_diff(old, new, author)?)
        },
        Commands::Old{file} => {
            Ok(command_old(file)?)
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
        Commands::Tex{file} => {
            command_tex(file)
        },
        Commands::Trousers{} => {
            command_trousers()
        }
    }
}


fn command_trousers() -> Result<()> {
    println!("Oh what fun we had!");
    println!("But at the time it seemed so bad");
    Ok(())
}


fn command_diff(old: &str, new: &str, author: &Option<String>) -> Result<()> {
    let author_canon = author.clone().map(|mut a| {
        ensure_canonical_author(&mut a);
        a
    });
    let result = make_suggestions_from_diff(old, new, author_canon)?;
    Ok(println!("{}", result))
}


fn command_old(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.to_string_reject();
    Ok(println!("{}", suggs))
}


fn command_new(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.to_string_accept();
    Ok(println!("{}", suggs))
}


fn command_colorize(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.to_colored_string();
    Ok(println!("{}", suggs))
}


fn command_reject(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.to_string_reject();
    print_suggestions_to_file(suggs, path)
}


fn command_accept(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.to_string_accept();
    print_suggestions_to_file(suggs, path)
}


fn command_tex(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let tex = node.to_string_tex()?;

    let tex = add_tex_dependencies(tex);
    Ok(println!("{}", tex))
}


fn add_tex_dependencies(tex: String) -> String {
    let begin_doc_re = Regex::new(r"\\begin\{document\}").unwrap();
    let begin_with_uses = 
    r"
\usepackage{color}
\usepackage{ulem}
\begin{document}";
    begin_doc_re.replace(tex.as_str(), begin_with_uses).to_string()
}

fn ensure_canonical_author(author: &mut String) {
    if ! author.starts_with('@') {
      author.insert(0, '@')
    }
    let re = Regex::new(r"^\S+$").unwrap();
    if ! re.is_match(author) {
         panic!("Author '{}' contained space characters", author);
    }
 } 


fn print_suggestions_to_file(string: String, path: &str) -> Result<()> {
    Ok(std::fs::write(path, string.as_str())?)
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
}

