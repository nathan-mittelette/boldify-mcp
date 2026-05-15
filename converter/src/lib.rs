//! Converter crate — transforms an AST produced by the parser into Unicode-formatted text.
//!
//! Entry point: [`convert`].  Style handlers live in [`handlers`]; AST node
//! implementations of [`traits::ToUnicode`] live in [`nodes`].

pub mod handlers;
pub mod nodes;
pub mod traits;

use parser::ContainerNode;

use crate::traits::ToUnicode;

pub use handlers::{
    bold_handler, italic_handler, strikethrough_handler, surline_handler, underline_handler,
    FontHandler, StrikethroughHandler, SurlineHandler, UnderlineHandler,
};

/// Converts a list of root AST nodes to a Unicode-formatted String.
/// All line breaks are encoded directly by the parsers — no separator is added here.
pub fn convert(nodes: &[ContainerNode]) -> String {
    nodes.iter().map(|n| n.to_unicode()).collect()
}

#[cfg(test)]
mod tests {
    use parser::ast::*;

    use super::*;
    fn make_text(text: &str) -> InlineNode {
        InlineNode::Text(TextNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            text: text.to_string(),
        })
    }

    fn make_container(ct: ContainerType, children: Vec<InlineNode>) -> ContainerNode {
        ContainerNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            container_type: ct,
            children,
        }
    }

    #[test]
    fn convert_concatenates_nodes_without_separator() {
        let n1 = make_container(ContainerType::Text, vec![make_text("line 1\n")]);
        let n2 = make_container(ContainerType::Text, vec![make_text("line 2")]);
        let result = convert(&[n1, n2]);
        assert!(result.contains("line 1\nline 2"));
    }

    #[test]
    fn convert_empty_vec_returns_empty_string() {
        assert_eq!(convert(&[]), "");
    }

    #[test]
    fn same_style_on_same_input_always_returns_same_result() {
        let r1 = bold_handler().apply("Hello World");
        let r2 = bold_handler().apply("Hello World");
        assert_eq!(r1, r2);
    }

    #[test]
    fn bold_and_italic_on_same_text_differ() {
        let bold = bold_handler().apply("test");
        let italic = italic_handler().apply("test");
        assert_ne!(bold, italic);
    }

    #[test]
    fn convert_simulated_linkedin_post() {
        let nodes = vec![
            make_container(
                ContainerType::Text,
                vec![
                    make_text("🔥 "),
                    InlineNode::Container(make_container(
                        ContainerType::Bold,
                        vec![make_text("3 tips to improve")],
                    )),
                ],
            ),
            make_container(
                ContainerType::List,
                vec![
                    InlineNode::ListItem(ListItemNode {
                        base: NodeBase::new(1, Span::new(0, 0)),
                        children: vec![
                            InlineNode::Container(make_container(
                                ContainerType::Bold,
                                vec![make_text("Read")],
                            )),
                            make_text(" every day"),
                        ],
                    }),
                    InlineNode::ListItem(ListItemNode {
                        base: NodeBase::new(2, Span::new(0, 0)),
                        children: vec![
                            make_text("Practice "),
                            InlineNode::Container(make_container(
                                ContainerType::Italic,
                                vec![make_text("regularly")],
                            )),
                        ],
                    }),
                    InlineNode::ListItem(ListItemNode {
                        base: NodeBase::new(3, Span::new(0, 0)),
                        children: vec![
                            make_text("Share your "),
                            InlineNode::Container(make_container(
                                ContainerType::Strikethrough,
                                vec![make_text("mistakes")],
                            )),
                            make_text(" learnings"),
                        ],
                    }),
                ],
            ),
            make_container(
                ContainerType::Blockquote,
                vec![make_text(
                    "Consistent progress beats occasional perfection.",
                )],
            ),
        ];

        let result = convert(&nodes);
        assert!(!result.is_empty());
        assert!(result.contains("🔥"));
        assert!(result.contains('\n'));
        assert!(result.contains("• "));
        assert!(result.contains('❝'));
        assert!(!result.contains("Read"));
        assert!(result.contains(" every day"));
    }
}
