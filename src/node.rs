
use colored::*;
use regex::Regex;

pub struct Node {
    /// `author_string` includes spaces, so it can be included directly
    /// without changing the output. See `author_clean()` below.
    pub author_string: Option<String>,
    pub contents: Vec<Chunk>,
    pub kind: NodeKind
}

#[derive(PartialEq)]
pub enum NodeKind {
    Root,
    Insertion,
    Deletion,
    Comment
}

pub enum Chunk {
    TextChunk(String),
    NodeChunk(Node)
}

impl Node {
    pub fn root() -> Node {
        Node {
            kind: NodeKind::Root,
            author_string: None,
            contents: Vec::new()
        }
    }

    pub fn author_clean(&self) -> Option<String> {
        self.author_string
            .clone()
            .map(|a| a.trim().to_string())
    }

    pub fn accept_to_string (&self) -> String {
        // if type is Insertion or Root, send contents
        // if type is Deletion or Comment return nothing
        if let NodeKind::Deletion | NodeKind::Comment = self.kind {
            "".to_string()
        } else {
            let content_strings: Vec<String> = self.contents.iter().map(
                |chunk| chunk.accept_to_string()
            ).collect();
            let mut output = "".to_string();
            for s in content_strings {
                output.push_str(s.as_str());
            }
            output
        }
    }

    pub fn reject_to_string(&self) -> String {
        // if type is Deletion or Root, send contents
        // if type is Insertion or Comment return nothing without delegating
        if let NodeKind::Insertion | NodeKind::Comment = self.kind {
            "".to_string()
        } else {
            let content_strings: Vec<String> = self.contents.iter().map(
                |chunk| chunk.reject_to_string()
            ).collect();
            let mut output = "".to_string();
            for s in content_strings {
                output.push_str(s.as_str());
            }
            output
        }
    }

    pub fn leave_to_string(&self) -> String {
        let op = opener(&self.kind);
        let cl = closer(&self.kind);
        
        let content_strings: Vec<String> = self.contents.iter().map(
            |ch| ch.leave_to_string()
        ).collect();

        let mut output = "".to_string();

        output.push_str(op);
        for s in content_strings {
            output.push_str(s.as_str());
        }
        let auth_str = self
            .author_string
            .clone()
            .unwrap_or("".to_string());
        output.push_str(auth_str.as_str());
        output.push_str(cl);

        output
    }


    pub fn leave_to_colorized(&self) -> ColoredString {
        let my_color = match self.kind {
            NodeKind::Comment => "cyan",
            NodeKind::Insertion => "green",
            NodeKind::Deletion  => "red",
            NodeKind::Root => "black" // we'll just clear it later
        };

        // let op = opener(&self.kind).color(my_color);
        // let cl = closer(&self.kind).color(my_color);

        let mut content_strings: Vec<ColoredString> = self.contents.iter().map(
            |ch| {
                let cs = match ch {
                    Chunk::NodeChunk(nd) => nd.leave_to_colorized(),
                    Chunk::TextChunk(text) => text.color(my_color)
                };
                match self.kind {
                    NodeKind::Root => cs.clear(),
                    NodeKind::Deletion => cs.strikethrough(),
                    _ => cs
                }
              
            }
        ).collect();

        if self.kind == NodeKind::Comment {
            let auth_str = self
                .author_string
                .clone()
                .unwrap_or("".to_string())
                .bright_cyan();
            let mut cs2 = vec!["[".color(my_color)];
            cs2.append(&mut content_strings);
            cs2.push(auth_str); 
            cs2.push("]".color(my_color));
            content_strings = cs2;
        }

        let mut output = "".to_string();

        for s in content_strings {
            output.push_str(format!("{}", s).as_str());
        }

        output.into()
    }
} 


pub fn opener(nk: &NodeKind) -> &str {
    match nk {
        NodeKind::Root => "",
        NodeKind::Insertion => "++[",
        NodeKind::Deletion => "--[",
        NodeKind::Comment => "%%["
    }
}


pub fn closer(nk: &NodeKind) -> &str {
    match nk {
        NodeKind::Root => "",
        NodeKind::Insertion => "]++",
        NodeKind::Deletion => "]--",
        NodeKind::Comment => "]%%"
    }
}


pub const OPENERS: [&str; 3] = ["++[", "--[", "%%["];
pub const CLOSERS: [&str; 3]  = ["]++", "]--", "]%%"];


pub fn ensure_canonical_author(author: &mut String) {
   if ! author.starts_with('@') {
     author.insert(0, '@')
   }
   let re = Regex::new(r"^\S+$").unwrap();
   if ! re.is_match(author) {
        panic!("Author '{}' contained space characters", author);
   }
}


impl Chunk {
    fn leave_to_string(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.leave_to_string()
        }
    }

    fn accept_to_string(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.accept_to_string()
        }
    }

    // text chunk is still being rejected
    fn reject_to_string(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.reject_to_string()
        }
    }
}


#[test]
fn test_can_use_structure() {
    let cch = Chunk::TextChunk("This is a comment. ".to_string());
    let insch = Chunk::TextChunk("This is an insertion. ".to_string());
    let startch = Chunk::TextChunk("Main text. ".to_string());
    let endch = Chunk::TextChunk("More main text.".to_string());

    let n = Node {
        contents: vec![cch],
        kind: NodeKind::Comment,
        author_string: Some("@DHJ".to_string())
    };
    let n2 = Node {
        contents: vec![insch],
        kind: NodeKind::Insertion,
        author_string: None
    };
    let root_node = Node {
        contents: vec![startch, Chunk::NodeChunk(n2), Chunk::NodeChunk(n), endch],
        kind: NodeKind::Root,
        author_string: None
    };

    let s = root_node.leave_to_string();
    println!("{:?}", s);
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