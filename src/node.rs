
use colored::*;


/// A Node represents a particular addition, deletion or comment in 
/// a suggestions file. A whole file is a tree of Nodes.
/// 
///  
pub struct Node {
    /// `author_string` includes spaces, so it can be included directly
    /// without changing the output. See [`Self::author_clean()`] below.
    pub author_string: Option<String>,
    /// A vector of [Chunk] objects representing the Node's contents.
    pub contents: Vec<Chunk>,
    /// The Node's [NodeKind]
    pub kind: NodeKind
}

#[derive(PartialEq, Clone, Copy)]
pub enum NodeKind {
    Root,
    Addition,
    Deletion,
    Comment
}

/// Chunks are pieces of contents within a Node. They can either be 
/// TextChunks containing text, or NodeChunks containing another
/// Node.
pub enum Chunk {
    TextChunk(String),
    NodeChunk(Node)
}

impl Node {
    /// Returns an empty root Node representing an entire document
    pub fn root() -> Node {
        Node {
            kind: NodeKind::Root,
            author_string: None,
            contents: Vec::new()
        }
    }

    /// Returns `Some<author>` if the node has an author,
    /// with author trimmed of whitespace. Returns `None`
    /// if there is no author.
    pub fn author_clean(&self) -> Option<String> {
        self.author_string
            .clone()
            .map(|a| a.trim().to_string())
    }

    /// Return a string representing the Node with all changes accepted.
    pub fn to_string_accept (&self) -> String {
        // if type is Addition or Root, send contents
        // if type is Deletion or Comment return nothing
        if let NodeKind::Deletion | NodeKind::Comment = self.kind {
            "".to_string()
        } else {
            let content_strings: Vec<String> = self.contents.iter().map(
                |chunk| chunk.to_string_accept()
            ).collect();
            let mut output = "".to_string();
            for s in content_strings {
                output.push_str(s.as_str());
            }
            output
        }
    }

    /// Return a string representing the Node with all changes rejected.
    pub fn to_string_reject(&self) -> String {
        // if type is Deletion or Root, send contents
        // if type is Addition or Comment return nothing without delegating
        if let NodeKind::Addition | NodeKind::Comment = self.kind {
            "".to_string()
        } else {
            let content_strings: Vec<String> = self.contents.iter().map(
                |chunk| chunk.to_string_reject()
            ).collect();
            let mut output = "".to_string();
            for s in content_strings {
                output.push_str(s.as_str());
            }
            output
        }
    }

    /// Return a String representing the Node in suggestions format
    pub fn to_string_suggestion(&self) -> String {
        let op = opener(&self.kind);
        let cl = closer(&self.kind);
        
        let content_strings: Vec<String> = self.contents.iter().map(
            |ch| ch.to_string_suggestion()
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


    pub fn to_colored_string(&self) -> ColoredString {
        let my_color = match self.kind {
            NodeKind::Comment => "cyan",
            NodeKind::Addition => "green",
            NodeKind::Deletion  => "red",
            NodeKind::Root => "black" // we'll just clear it later
        };

        // let op = opener(&self.kind).color(my_color);
        // let cl = closer(&self.kind).color(my_color);

        let mut content_strings: Vec<ColoredString> = self.contents.iter().map(
            |ch| {
                let cs = match ch {
                    Chunk::NodeChunk(nd) => nd.to_colored_string(),
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


/// Returns the opening tags for a given kind of Node
pub fn opener(nk: &NodeKind) -> &str {
    match nk {
        NodeKind::Root => "",
        NodeKind::Addition => "++[",
        NodeKind::Deletion => "--[",
        NodeKind::Comment => "%%["
    }
}


/// Returns the closing tags for a given kind of Node
pub fn closer(nk: &NodeKind) -> &str {
    match nk {
        NodeKind::Root => "",
        NodeKind::Addition => "]++",
        NodeKind::Deletion => "]--",
        NodeKind::Comment => "]%%"
    }
}


pub const OPENERS: [&str; 4] = ["++[", "--[", "%%[", "//"];
pub const CLOSERS: [&str; 4]  = ["]++", "]--", "]%%", "//"];


impl Chunk {
    fn to_string_suggestion(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.to_string_suggestion()
        }
    }

    fn to_string_accept(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.to_string_accept()
        }
    }

    // text chunk is still being rejected if this is called
    fn to_string_reject(&self) -> String {
        match self {
            Chunk::TextChunk(text) => text.clone(),
            Chunk::NodeChunk(node) => node.to_string_reject()
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
        kind: NodeKind::Addition,
        author_string: None
    };
    let root_node = Node {
        contents: vec![startch, Chunk::NodeChunk(n2), Chunk::NodeChunk(n), endch],
        kind: NodeKind::Root,
        author_string: None
    };

    let s = root_node.to_string_suggestion();
    println!("{:?}", s);
}
