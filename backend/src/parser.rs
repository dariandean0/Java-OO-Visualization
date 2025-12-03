use anyhow::{Context, Result};
use tree_sitter::{Node, Parser, Tree};

pub struct JavaParser {
    parser: Parser,
}

impl JavaParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_java::LANGUAGE.into();
        // Note: tree_sitter::LanguageError doesn't implement std::error::Error,
        // so we can't use .context() and must use map_err instead
        parser
            .set_language(&language)
            .map_err(|e| anyhow::anyhow!("Failed to set language for parser: {e:?}"))?;

        Ok(JavaParser { parser })
    }

    pub fn parse(&mut self, source_code: &str) -> Result<Tree> {
        self.parser
            .parse(source_code, None)
            .context("Failed to parse Java code")
    }

    pub fn get_root_node<'a>(&self, tree: &'a Tree) -> Node<'a> {
        tree.root_node()
    }
}

pub fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

pub fn walk_tree<F>(node: &Node, source: &str, depth: usize, callback: &mut F)
where
    F: FnMut(&Node, &str, usize),
{
    callback(node, source, depth);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(&child, source, depth + 1, callback);
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parser_creation() {
        assert!(JavaParser::new().is_ok());
    }

    #[test]
    fn simple_java_parsing() {
        let mut parser = JavaParser::new().unwrap();
        let code = "public class Test {}";
        let tree = parser.parse(code).unwrap();
        let root = parser.get_root_node(&tree);
        assert_eq!(root.kind(), "program");
    }
}
