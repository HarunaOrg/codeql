use super::nodes_types::{Entry, Field, Storage, TypeName};

use std::collections::BTreeMap as Map;
use std::collections::BTreeSet as Set;
use std::fmt;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

pub struct Extractor {
    pub parser: Parser,
    pub schema: Vec<Entry>,
}

pub fn create(language: Language, schema: Vec<Entry>) -> Extractor {
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    Extractor { parser, schema }
}
impl Extractor {
    pub fn extract<'a>(&'a mut self, path: &Path) -> std::io::Result<Program> {
        let source = std::fs::read(&path)?;
        let tree = &self
            .parser
            .parse(&source, None)
            .expect("Failed to parse file");
        let mut visitor = Visitor {
            source: &source,
            trap_output: vec![TrapEntry::Comment(format!(
                "Auto-generated TRAP file for {}",
                path.display()
            ))],
            counter: -1,
            // TODO: should we handle path strings that are not valid UTF8 better?
            path: format!("{}", path.display()),
            stack: Vec::new(),
            tables: build_schema_lookup(&self.schema),
            union_types: build_union_type_lookup(&self.schema),
        };
        traverse(&tree, &mut visitor);

        &self.parser.reset();
        Ok(Program(visitor.trap_output))
    }
}

fn build_schema_lookup<'a>(schema: &'a Vec<Entry>) -> Map<&'a TypeName, &'a Entry> {
    let mut map = std::collections::BTreeMap::new();
    for entry in schema {
        if let Entry::Table { type_name, .. } = entry {
            map.insert(type_name, entry);
        }
    }
    map
}

fn build_union_type_lookup<'a>(schema: &'a Vec<Entry>) -> Map<&'a TypeName, &'a Set<TypeName>> {
    let mut union_types = std::collections::BTreeMap::new();
    for entry in schema {
        if let Entry::Union { type_name, members } = entry {
            union_types.insert(type_name, members);
        }
    }
    union_types
}

struct Visitor<'a> {
    /// The file path of the source code (as string)
    path: String,
    /// The source code as a UTF-8 byte array
    source: &'a Vec<u8>,
    /// The accumulated trap entries
    trap_output: Vec<TrapEntry>,
    /// A counter for generating fresh labels
    counter: i32,
    /// A lookup table from type name to dbscheme table entries
    tables: Map<&'a TypeName, &'a Entry>,
    /// A lookup table for union types mapping a type name to its direct members
    union_types: Map<&'a TypeName, &'a Set<TypeName>>,
    /// A stack for gathering information from hild nodes. Whenever a node is entered
    /// an empty list is pushed. All children append their data (field name, label, type) to
    /// the the list. When the visitor leaves a node the list containing the child data is popped
    /// from the stack and matched against the dbscheme for the node. If the expectations are met
    /// the corresponding row definitions are added to the trap_output.
    stack: Vec<Vec<(Option<&'static str>, Label, TypeName)>>,
}

impl Visitor<'_> {
    fn enter_node(&mut self, node: Node) -> bool {
        if node.is_error() {
            println!(
                "error: {}:{}: parse error",
                &self.path,
                node.start_position().row,
            );
            return false;
        }
        if node.is_missing() {
            println!(
                "error: {}:{}: parse error: expecting '{}'",
                &self.path,
                node.start_position().row,
                node.kind()
            );
            return false;
        }

        if node.is_extra() {
            return false;
        }

        self.stack.push(Vec::new());
        return true;
    }

    fn leave_node(&mut self, field_name: Option<&'static str>, node: Node) {
        if node.is_extra() || node.is_error() || node.is_missing() {
            return;
        }
        let child_nodes = self.stack.pop().expect("Vistor: empty stack");
        let table = self.tables.get(&TypeName {
            kind: node.kind().to_owned(),
            named: node.is_named(),
        });
        if let Some(Entry::Table { fields, .. }) = table {
            self.counter += 1;
            let id = Label::Normal(self.counter);
            let loc = Label::Location(self.counter);
            self.trap_output.push(TrapEntry::New(id));
            self.trap_output.push(TrapEntry::New(loc));
            self.trap_output.push(location_for(&self.path, loc, node));
            let table_name = node_type_name(node.kind(), node.is_named());
            let args: Option<Vec<Arg>>;
            if fields.is_empty() {
                args = Some(vec![sliced_source_arg(self.source, node)]);
            } else {
                args = self.complex_node(&node, fields, child_nodes, id);
            }
            if let Some(args) = args {
                self.trap_output
                    .push(TrapEntry::Definition(table_name, id, args, loc));
            }
            if let Some(parent) = self.stack.last_mut() {
                parent.push((
                    field_name,
                    id,
                    TypeName {
                        kind: node.kind().to_owned(),
                        named: node.is_named(),
                    },
                ))
            };
        } else {
            println!(
                "error: {}:{}: unknown table type: '{}'",
                &self.path,
                node.start_position().row,
                node.kind()
            );
        }
    }
    fn complex_node(
        &mut self,
        node: &Node,
        fields: &Vec<Field>,
        child_nodes: Vec<(Option<&str>, Label, TypeName)>,
        parent_id: Label,
    ) -> Option<Vec<Arg>> {
        let mut map: Map<&Option<String>, (&Field, Vec<Label>)> = std::collections::BTreeMap::new();
        for field in fields {
            map.insert(&field.name, (field, Vec::new()));
        }
        for (child_field, child_id, child_type) in child_nodes {
            if let Some((field, values)) = map.get_mut(&child_field.map(|x| x.to_owned())) {
                //TODO: handle error and missing nodes
                if self.type_matches(&child_type, &field.types) {
                    values.push(child_id);
                } else if field.name.is_some() {
                    println!(
                        "error: {}:{}: type mismatch for field {}::{} with type {:?} != {:?}",
                        &self.path,
                        node.start_position().row,
                        node.kind(),
                        child_field.unwrap_or("child"),
                        child_type,
                        field.types
                    )
                }
            } else {
                if child_field.is_some() || child_type.named {
                    println!(
                        "error: {}:{}: value for unknown field: {}::{} and type {:?}",
                        &self.path,
                        node.start_position().row,
                        node.kind(),
                        &child_field.unwrap_or("child"),
                        &child_type
                    );
                }
            }
        }
        let mut args = Vec::new();
        let mut is_valid = true;
        for field in fields {
            let child_ids = &map.get(&field.name).unwrap().1;
            match &field.storage {
                Storage::Column => {
                    if child_ids.len() == 1 {
                        args.push(Arg::Label(*child_ids.first().unwrap()));
                    } else {
                        is_valid = false;
                        println!(
                            "error: {}:{}: {} for field: {}::{}",
                            &self.path,
                            node.start_position().row,
                            if child_ids.is_empty() {
                                "missing value"
                            } else {
                                "too many values"
                            },
                            node.kind(),
                            match field.name.as_ref() {
                                Some(x) => x,
                                None => "child",
                            }
                        )
                    }
                }
                Storage::Table { parent, index } => {
                    for child_id in child_ids {
                        self.trap_output.push(TrapEntry::ChildOf(
                            node_type_name(&parent.kind, parent.named),
                            parent_id,
                            match &field.name {
                                Some(name) => name.to_owned(),
                                None => "child".to_owned(),
                            },
                            Index(*index),
                            *child_id,
                        ));
                    }
                }
            }
        }
        if is_valid {
            Some(args)
        } else {
            None
        }
    }
    fn type_matches(&self, tp: &TypeName, types: &Set<TypeName>) -> bool {
        if types.contains(tp) {
            return true;
        }
        for other in types.iter() {
            if let Some(x) = self.union_types.get(other) {
                if self.type_matches(tp, x) {
                    return true;
                }
            }
        }
        return false;
    }
}

// Emit a slice of a source file as an Arg.
fn sliced_source_arg(source: &Vec<u8>, n: Node) -> Arg {
    let range = n.byte_range();
    Arg::String(String::from(
        std::str::from_utf8(&source[range.start..range.end]).expect("Failed to decode string"),
    ))
}

// Emit a 'Located' TrapEntry for the provided node, appropriately calibrated.
fn location_for<'a>(fp: &String, label: Label, n: Node) -> TrapEntry {
    let start_line = n.start_position().row;
    let start_col = n.start_position().column;
    let end_line = n.end_position().row;
    let end_col = n.end_position().column;
    TrapEntry::Located(vec![
        Arg::Label(label),
        Arg::String(fp.to_owned()),
        Arg::Int(start_line),
        Arg::Int(start_col),
        Arg::Int(end_line),
        Arg::Int(end_col),
    ])
}

fn traverse(tree: &Tree, visitor: &mut Visitor) {
    let cursor = &mut tree.walk();
    visitor.enter_node(cursor.node());
    let mut recurse = true;
    loop {
        if recurse && cursor.goto_first_child() {
            recurse = visitor.enter_node(cursor.node());
        } else {
            visitor.leave_node(cursor.field_name(), cursor.node());

            if cursor.goto_next_sibling() {
                recurse = visitor.enter_node(cursor.node());
            } else if cursor.goto_parent() {
                recurse = false;
            } else {
                break;
            }
        }
    }
}

pub struct Program(Vec<TrapEntry>);

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut text = String::new();
        for trap_entry in &self.0 {
            text.push_str(&format!("{}\n", trap_entry));
        }
        write!(f, "{}", text)
    }
}

enum TrapEntry {
    // @id = *@
    New(Label),
    // @node_def(self, arg?, location)@
    Definition(String, Label, Vec<Arg>, Label),
    // @node_child(self, index, parent)@
    ChildOf(String, Label, String, Index, Label),
    // @location(loc, path, r1, c1, r2, c2)
    Located(Vec<Arg>),
    Comment(String),
}
impl fmt::Display for TrapEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrapEntry::New(id) => write!(f, "{} = *", id),
            TrapEntry::Definition(n, id, args, loc) => {
                let mut args_str = String::new();
                for arg in args {
                    args_str.push_str(&format!("{}, ", arg));
                }
                write!(
                    f,
                    "{}({}, {}{})",
                    escape_name(&format!("{}_def", &n)),
                    id,
                    args_str,
                    loc
                )
            }
            TrapEntry::ChildOf(pname, id, fname, idx, p) => write!(
                f,
                "{}({}, {}, {})",
                escape_name(&format!("{}_{}", &pname, &fname)),
                id,
                idx,
                p
            ),
            TrapEntry::Located(args) => write!(
                f,
                "location({}, {}, {}, {}, {}, {})",
                args.get(0).unwrap(),
                args.get(1).unwrap(),
                args.get(2).unwrap(),
                args.get(3).unwrap(),
                args.get(4).unwrap(),
                args.get(5).unwrap(),
            ),
            TrapEntry::Comment(line) => write!(f, "// {}", line),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Label {
    // Identifiers of the form #0, #1...
    Normal(i32), // #0, #1, etc.
    // Location identifiers of the form #0_loc, #1_loc...
    Location(i32), // #0_loc, #1_loc, etc.
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Label::Normal(x) => write!(f, "#{}", x),
            Label::Location(x) => write!(f, "#{}_loc", x),
        }
    }
}

// Numeric indices.
#[derive(Debug, Copy, Clone)]
struct Index(usize);

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Some untyped argument to a TrapEntry.
#[derive(Debug)]
enum Arg {
    Label(Label),
    Int(usize),
    String(String),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Arg::Label(x) => write!(f, "{}", x),
            Arg::Int(x) => write!(f, "{}", x),
            Arg::String(x) => write!(f, "\"{}\"", x.replace("\"", "\"\"")),
        }
    }
}

const RESERVED_KEYWORDS: [&'static str; 14] = [
    "boolean", "case", "date", "float", "int", "key", "of", "order", "ref", "string", "subtype",
    "type", "unique", "varchar",
];

/// Returns a string that's a copy of `name` but suitably escaped to be a valid
/// QL identifier.
pub fn escape_name(name: &str) -> String {
    let mut result = String::new();

    // If there's a leading underscore, replace it with 'underscore_'.
    if let Some(c) = name.chars().next() {
        if c == '_' {
            result.push_str("underscore");
        }
    }
    for c in name.chars() {
        match c {
            '{' => result.push_str("lbrace"),
            '}' => result.push_str("rbrace"),
            '<' => result.push_str("langle"),
            '>' => result.push_str("rangle"),
            '[' => result.push_str("lbracket"),
            ']' => result.push_str("rbracket"),
            '(' => result.push_str("lparen"),
            ')' => result.push_str("rparen"),
            '|' => result.push_str("pipe"),
            '=' => result.push_str("equal"),
            '~' => result.push_str("tilde"),
            '?' => result.push_str("question"),
            '`' => result.push_str("backtick"),
            '^' => result.push_str("caret"),
            '!' => result.push_str("bang"),
            '#' => result.push_str("hash"),
            '%' => result.push_str("percent"),
            '&' => result.push_str("ampersand"),
            '.' => result.push_str("dot"),
            ',' => result.push_str("comma"),
            '/' => result.push_str("slash"),
            ':' => result.push_str("colon"),
            ';' => result.push_str("semicolon"),
            '"' => result.push_str("dquote"),
            '*' => result.push_str("star"),
            '+' => result.push_str("plus"),
            '-' => result.push_str("minus"),
            '@' => result.push_str("at"),
            _ => result.push_str(&c.to_lowercase().to_string()),
        }
    }

    for &keyword in &RESERVED_KEYWORDS {
        if result == keyword {
            result.push_str("__");
            break;
        }
    }

    result
}

/// Given a tree-sitter node type's (kind, named) pair, returns a single string
/// representing the (unescaped) name we'll use to refer to corresponding QL
/// type.
fn node_type_name(kind: &str, named: bool) -> String {
    if named {
        kind.to_string()
    } else {
        format!("{}_unnamed", kind)
    }
}