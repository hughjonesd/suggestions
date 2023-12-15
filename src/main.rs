

/* 
TODO: 

* means priority

- allow variable opening/closing tags
  - automatically recognize numbers of +/-/% *
  - maybe also arbitrary strings embedded as "suggs add ++{ }++" or such
- writing a README and justification *
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

- split binary from library
- vim syntax?
- allow stdin as input to old/new
- tex output
- options: handling comments 
  - options to strip or keep; maybe separate command*
- testing: 
  - more wrongitude

- make author a &str, understand this stuff better
- visitor pattern?
    - something like "visit each node and replace tags with the following
      (either string or closure)"

- optionally sign output of diff DONE
- rename changetxt DONE
- accept/reject commands work on file in place DONE
- colorized output DONE
- ban nesting inside comments DONE
- strip whitespace if @handle, opener, or closer is only thing on a line. DONE
- integration tests DONE
- trousers DONE
*/




use suggestions::*;

use clap::{Parser, Subcommand, Args};

use anyhow::Result;


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


fn command_diff(old: &String, new: &String, author: &Option<String>) -> Result<()> {
    let author_canon = author.clone().map(|mut a| {
        ensure_canonical_author(&mut a);
        a
    });
    let result = make_suggestions_from_diff(old, new, &author_canon)?;
    Ok(println!("{}", result))
}


fn command_old(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.reject_to_string();
    Ok(println!("{}", suggs))
}


fn command_new(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.accept_to_string();
    Ok(println!("{}", suggs))
}


fn command_colorize(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.leave_to_colorized();
    Ok(println!("{}", suggs))
}


fn command_reject(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.reject_to_string();
    Ok(print_suggestions_to_file(suggs, path)?)
}


fn command_accept(path: &String) -> Result<()> {
    let node = make_node_from_file(path)?;
    let suggs = node.accept_to_string();
    Ok(print_suggestions_to_file(suggs, path)?)
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
        let path_old = "resources/old.txt".to_string();
        let path_new = "resources/new.txt".to_string();

        let test_output = make_suggestions_from_diff(&path_old, &path_new, &None).unwrap();
        let expected_output = "A ++[new ]++sentence.\n";
        assert_eq!(test_output, expected_output);

        let author = Some("@author1".to_string());
        let test_output = make_suggestions_from_diff(&path_old, &path_new, &author).unwrap();
        let expected_output = "A ++[new  @author1 ]++sentence.\n";
        assert_eq!(test_output, expected_output);
    }
}
