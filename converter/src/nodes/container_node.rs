use parser::{ContainerNode, ContainerType, InlineNode};

use crate::handlers::{
    bold_handler, italic_handler, strikethrough_handler, surline_handler, underline_handler,
    FontHandler,
};
use crate::nodes::inline_node::inline_to_unicode;
use crate::traits::ToUnicode;

impl ToUnicode for ContainerNode {
    fn to_unicode(&self) -> String {
        let raw: String = self.children.iter().map(inline_to_unicode).collect();

        match &self.container_type {
            ContainerType::Text => raw,
            ContainerType::Bold => bold_handler().apply(&raw),
            ContainerType::Italic => italic_handler().apply(&raw),
            ContainerType::Underline => underline_handler().apply(&raw),
            ContainerType::Strikethrough => strikethrough_handler().apply(&raw),
            ContainerType::Surline => surline_handler().apply(&raw),
            ContainerType::Blockquote => format!("❝ {} ❞", raw),
            ContainerType::List => {
                let mut out = String::new();
                for n in &self.children {
                    if let InlineNode::ListItem(li) = n {
                        if !out.is_empty() {
                            out.push('\n');
                        }
                        out.push_str("• ");
                        out.push_str(&li.to_unicode());
                    }
                }
                out
            }
            ContainerType::OrderedList => {
                let mut out = String::new();
                let mut counter = 0usize;
                for n in &self.children {
                    if let InlineNode::ListItem(li) = n {
                        counter += 1;
                        if counter > 1 {
                            out.push('\n');
                        }
                        out.push_str(&format!("{}. {}", counter, li.to_unicode()));
                    }
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use parser::ast::*;

    use crate::traits::ToUnicode;

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
    fn text_returns_raw_text() {
        let node = make_container(ContainerType::Text, vec![make_text("Hello")]);
        assert_eq!(node.to_unicode(), "Hello");
    }

    #[test]
    fn bold_converts_ascii_to_unicode_bold() {
        let node = make_container(ContainerType::Bold, vec![make_text("ABC")]);
        let result = node.to_unicode();
        assert!(!result.contains("ABC"));
        assert_eq!(result.chars().count(), 3);
    }

    #[test]
    fn bold_preserves_space() {
        let node = make_container(ContainerType::Bold, vec![make_text("A B")]);
        assert!(node.to_unicode().contains(' '));
    }

    #[test]
    fn bold_converts_accented_char() {
        let node = make_container(ContainerType::Bold, vec![make_text("é")]);
        let result = node.to_unicode();
        assert!(result.contains('\u{0301}'));
        assert!(!result.contains('é'));
    }

    #[test]
    fn italic_converts_ascii_to_unicode_italic() {
        let node = make_container(ContainerType::Italic, vec![make_text("Hello")]);
        assert!(!node.to_unicode().contains("Hello"));
    }

    #[test]
    fn blockquote_wraps_text_with_quotes() {
        let node = make_container(ContainerType::Blockquote, vec![make_text("quote")]);
        let result = node.to_unicode();
        assert!(result.contains("quote"));
        assert!(result.contains('❝'));
        assert!(result.contains('❞'));
    }

    #[test]
    fn list_prefixes_each_item_with_bullet() {
        let li_a = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![make_text("alpha")],
        });
        let li_b = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(1, Span::new(0, 0)),
            children: vec![make_text("beta")],
        });
        let node = make_container(ContainerType::List, vec![li_a, li_b]);
        let result = node.to_unicode();
        assert!(result.contains("• alpha"));
        assert!(result.contains("• beta"));
    }

    #[test]
    fn ordered_list_numbers_items() {
        let li = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![make_text("first")],
        });
        let node = make_container(ContainerType::OrderedList, vec![li]);
        assert!(node.to_unicode().contains("1. first"));
    }

    #[test]
    fn bold_containing_italic_applies_both_styles() {
        let italic_child = InlineNode::Container(make_container(
            ContainerType::Italic,
            vec![make_text("Nathan")],
        ));
        let bold_node = make_container(ContainerType::Bold, vec![italic_child]);
        assert!(!bold_node.to_unicode().contains("Nathan"));
    }

    #[test]
    fn bold_with_emoji_in_text() {
        let node = make_container(ContainerType::Bold, vec![make_text("🔥 Results")]);
        let result = node.to_unicode();
        assert!(result.contains("🔥"));
        assert!(!result.contains('R'));
    }

    #[test]
    fn bold_with_punctuation_and_digits() {
        let node = make_container(ContainerType::Bold, vec![make_text("Top 3 !")]);
        let result = node.to_unicode();
        assert!(result.contains(' '));
        assert!(result.contains('!'));
        assert!(!result.contains('T'));
    }

    #[test]
    fn text_with_emoji_returns_emoji_intact() {
        let node = make_container(ContainerType::Text, vec![make_text("Hello 👋")]);
        assert_eq!(node.to_unicode(), "Hello 👋");
    }

    #[test]
    fn italic_with_accents_transforms_base_chars() {
        let node = make_container(ContainerType::Italic, vec![make_text("élégance")]);
        let result = node.to_unicode();
        assert!(!result.contains('é'));
        assert!(!result.contains('e'));
    }

    #[test]
    fn strikethrough_with_multiple_words() {
        let node = make_container(ContainerType::Strikethrough, vec![make_text("old content")]);
        let result = node.to_unicode();
        assert!(result.contains('\u{0336}'));
        assert!(result.contains(' '));
    }

    #[test]
    fn underline_with_digits() {
        let node = make_container(ContainerType::Underline, vec![make_text("2024")]);
        assert!(node.to_unicode().contains('\u{0332}'));
    }

    #[test]
    fn blockquote_with_emoji_in_quote() {
        let node = make_container(
            ContainerType::Blockquote,
            vec![make_text("Be the change 🌱")],
        );
        let result = node.to_unicode();
        assert!(result.contains('❝'));
        assert!(result.contains("🌱"));
    }

    #[test]
    fn list_with_emoji_items() {
        let items: Vec<InlineNode> = ["🎯 Goal", "🚀 Launch", "✅ Done"]
            .iter()
            .enumerate()
            .map(|(i, text)| {
                InlineNode::ListItem(ListItemNode {
                    base: NodeBase::new(i as u64, Span::new(0, 0)),
                    children: vec![make_text(text)],
                })
            })
            .collect();
        let node = make_container(ContainerType::List, items);
        let result = node.to_unicode();
        assert!(result.contains("• 🎯 Goal"));
        assert!(result.contains("• 🚀 Launch"));
        assert!(result.contains("• ✅ Done"));
    }

    #[test]
    fn ordered_list_with_bold_in_items() {
        let bold_child = InlineNode::Container(make_container(
            ContainerType::Bold,
            vec![make_text("Important")],
        ));
        let li = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![bold_child, make_text(" to remember")],
        });
        let node = make_container(ContainerType::OrderedList, vec![li]);
        let result = node.to_unicode();
        assert!(result.contains("1."));
        assert!(!result.contains("Important"));
        assert!(result.contains(" to remember"));
    }

    #[test]
    fn all_styles_produce_different_results() {
        let text = "ABC";
        let bold = make_container(ContainerType::Bold, vec![make_text(text)]).to_unicode();
        let italic = make_container(ContainerType::Italic, vec![make_text(text)]).to_unicode();
        let underline =
            make_container(ContainerType::Underline, vec![make_text(text)]).to_unicode();
        let strikethrough =
            make_container(ContainerType::Strikethrough, vec![make_text(text)]).to_unicode();
        let surline = make_container(ContainerType::Surline, vec![make_text(text)]).to_unicode();

        let styles = [&bold, &italic, &underline, &strikethrough, &surline];
        for i in 0..styles.len() {
            for j in (i + 1)..styles.len() {
                assert_ne!(
                    styles[i], styles[j],
                    "Styles {} and {} produce the same result for \"{}\"",
                    i, j, text
                );
            }
        }
    }

    #[test]
    fn bold_node_with_internal_newline() {
        let node = make_container(ContainerType::Bold, vec![make_text("line1\nline2")]);
        let result = node.to_unicode();
        assert!(result.contains('\n'));
        assert!(!result.contains("line"));
    }

    #[test]
    fn text_container_with_multiple_inline_containers() {
        let node = make_container(
            ContainerType::Text,
            vec![
                make_text("Text "),
                InlineNode::Container(make_container(ContainerType::Bold, vec![make_text("bold")])),
                make_text(" and "),
                InlineNode::Container(make_container(
                    ContainerType::Italic,
                    vec![make_text("italic")],
                )),
                make_text(" and "),
                InlineNode::Container(make_container(
                    ContainerType::Surline,
                    vec![make_text("highlighted")],
                )),
            ],
        );
        let result = node.to_unicode();
        assert!(result.contains("Text "));
        assert!(result.contains(" and "));
        assert!(!result.contains("bold"));
        assert!(!result.contains("italic"));
        assert!(result.contains('\u{0305}'));
    }
}
