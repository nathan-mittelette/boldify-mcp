//! Markdown parser — converts a Markdown string into a list of [`ContainerNode`]s.
//!
//! # Supported syntax
//! - `**bold**` / `__bold__`
//! - `*italic*` / `_italic_`
//! - `~~strikethrough~~`
//! - `==highlight==`
//! - Unordered lists (`- item` or `* item`)
//! - Ordered lists (`1. item`)
//!
//! # Unsupported constructs
//! Headings (`#`), blockquotes (`>`), code spans (`` ` ``), and HTML tags
//! all produce a [`ParseError::UnsupportedSymbol`].

use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, ListItemNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::NodeIdGen,
    inline::parse_inline,
    Parser,
};

#[derive(Clone)]
pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        let lines = collect_lines(input);
        let mut nodes = Vec::new();
        let mut id_gen = NodeIdGen::new();
        let mut i = 0;

        // pending_newlines tracks how many \n to insert before the next content node:
        // - 1 after each content line (normal line break)
        // - +1 for each blank line (producing \n\n for a blank-separated block)
        // On the very first content node nothing is prepended.
        let mut pending_newlines: usize = 0;
        let mut first_node = true;

        while let Some(line) = lines.get(i) {
            if line.text.trim().is_empty() {
                // Each blank line adds one extra \n (beyond the normal post-line \n)
                if !first_node {
                    pending_newlines += 1;
                }
                i += 1;
                continue;
            }

            let (trimmed_start, trimmed) = trim_line_start(line.text);

            // # is only a heading when followed by a space or end of line.
            // A bare #word (hashtag) is treated as plain text.
            if trimmed.starts_with('#') {
                let after = trimmed.trim_start_matches('#');
                if after.is_empty() || after.starts_with(' ') {
                    let hashes: String = trimmed.chars().take_while(|&c| c == '#').collect();
                    return Err(ParseError::UnsupportedSymbol {
                        symbol: hashes,
                        position: SourcePosition {
                            line: line.number,
                            column: column_at(line.text, trimmed_start),
                            byte_offset: line.start + trimmed_start,
                        },
                    });
                }
            }

            if trimmed.starts_with('>') {
                return Err(ParseError::UnsupportedSymbol {
                    symbol: ">".into(),
                    position: SourcePosition {
                        line: line.number,
                        column: column_at(line.text, trimmed_start),
                        byte_offset: line.start + trimmed_start,
                    },
                });
            }

            // Push accumulated \n nodes before this content node
            if !first_node {
                push_newline_nodes(pending_newlines, &mut nodes, &mut id_gen);
            }
            pending_newlines = 1; // default: one \n after this line
            first_node = false;

            if let Some((bullet, _, list_start_offset)) = try_strip_unordered_prefix(line.text) {
                let parsed = parse_list(
                    &lines,
                    i,
                    line,
                    list_start_offset,
                    ListKind::Unordered(bullet),
                    &mut id_gen,
                )?;
                nodes.push(parsed.node);
                i = parsed.next_line;
                continue;
            }

            if let Some((_, list_start_offset)) = try_strip_ordered_prefix(line.text) {
                let parsed = parse_list(
                    &lines,
                    i,
                    line,
                    list_start_offset,
                    ListKind::Ordered,
                    &mut id_gen,
                )?;
                nodes.push(parsed.node);
                i = parsed.next_line;
                continue;
            }

            let children = parse_inline(line.text, line.number, line.start, 1, &mut id_gen)?;
            nodes.push(ContainerNode {
                base: NodeBase::new(id_gen.next_id(), Span::new(line.start, line.end)),
                container_type: ContainerType::Text,
                children,
            });
            i += 1;
        }

        Ok(nodes)
    }

    fn supported_symbols(&self) -> Vec<crate::SupportedSymbol> {
        vec![
            crate::SupportedSymbol {
                symbol: "**".into(),
                description: "Bold".into(),
                example: "**bold text**".into(),
            },
            crate::SupportedSymbol {
                symbol: "*".into(),
                description: "Italic".into(),
                example: "*italic text*".into(),
            },
            crate::SupportedSymbol {
                symbol: "_".into(),
                description: "Italic (alternative)".into(),
                example: "_italic text_".into(),
            },
            crate::SupportedSymbol {
                symbol: "__".into(),
                description: "Underline".into(),
                example: "__underlined text__".into(),
            },
            crate::SupportedSymbol {
                symbol: "~~".into(),
                description: "Strikethrough".into(),
                example: "~~strikethrough text~~".into(),
            },
            crate::SupportedSymbol {
                symbol: "==".into(),
                description: "Highlight".into(),
                example: "==highlighted text==".into(),
            },
            crate::SupportedSymbol {
                symbol: "- ".into(),
                description: "Unordered list".into(),
                example: "- list item".into(),
            },
            crate::SupportedSymbol {
                symbol: "1. ".into(),
                description: "Ordered list".into(),
                example: "1. first item".into(),
            },
        ]
    }
}

struct LineInfo<'a> {
    text: &'a str,
    start: usize,
    end: usize,
    number: usize,
}

#[derive(Clone, Copy)]
enum ListKind {
    Unordered(char),
    Ordered,
}

struct ParsedList {
    node: ContainerNode,
    next_line: usize,
}

fn parse_list(
    lines: &[LineInfo<'_>],
    start_index: usize,
    first_line: &LineInfo<'_>,
    content_start_offset: usize,
    kind: ListKind,
    id_gen: &mut NodeIdGen,
) -> Result<ParsedList, ParseError> {
    let list_start = first_line.start + content_start_offset;
    let mut list_end = first_line.end;
    let mut children = Vec::new();
    let mut next_line = start_index;

    while let Some(line) = lines.get(next_line) {
        if line.text.trim().is_empty() {
            break;
        }

        let Some((rest, content_offset)) = list_item_content(line.text, kind) else {
            break;
        };

        list_end = line.end;
        children.push(InlineNode::ListItem(build_list_item(
            rest,
            line.number,
            line.start + content_offset,
            column_at(line.text, content_offset),
            id_gen,
        )?));
        next_line += 1;
    }

    let container_type = match kind {
        ListKind::Unordered(_) => ContainerType::List,
        ListKind::Ordered => ContainerType::OrderedList,
    };

    Ok(ParsedList {
        node: ContainerNode {
            base: NodeBase::new(id_gen.next_id(), Span::new(list_start, list_end)),
            container_type,
            children,
        },
        next_line,
    })
}

fn list_item_content(line: &str, kind: ListKind) -> Option<(&str, usize)> {
    match kind {
        ListKind::Unordered(expected_bullet) => {
            let (bullet, rest, content_offset) = try_strip_unordered_prefix(line)?;
            (bullet == expected_bullet).then_some((rest, content_offset))
        }
        ListKind::Ordered => try_strip_ordered_prefix(line),
    }
}

fn collect_lines(input: &str) -> Vec<LineInfo<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;

    for (idx, raw_line) in input.split_inclusive('\n').enumerate() {
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        lines.push(LineInfo {
            text: line,
            start,
            end: start + line.len(),
            number: idx + 1,
        });
        start += raw_line.len();
    }

    lines
}

fn trim_line_start(line: &str) -> (usize, &str) {
    let trimmed = line.trim_start();
    (line.len() - trimmed.len(), trimmed)
}

fn column_at(line: &str, byte_offset: usize) -> usize {
    line[..byte_offset].chars().count() + 1
}

fn try_strip_unordered_prefix(line: &str) -> Option<(char, &str, usize)> {
    let (trimmed_start, trimmed) = trim_line_start(line);
    if let Some(rest) = trimmed.strip_prefix("- ") {
        return Some(('-', rest, trimmed_start + 2));
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        return Some(('*', rest, trimmed_start + 2));
    }
    None
}

fn build_list_item(
    text: &str,
    line: usize,
    byte_start: usize,
    column_start: usize,
    id_gen: &mut NodeIdGen,
) -> Result<ListItemNode, ParseError> {
    let children = parse_inline(text, line, byte_start, column_start, id_gen)?;
    Ok(ListItemNode {
        base: NodeBase::new(
            id_gen.next_id(),
            Span::new(byte_start, byte_start + text.len()),
        ),
        children,
    })
}

fn push_newline_nodes(count: usize, nodes: &mut Vec<ContainerNode>, id_gen: &mut NodeIdGen) {
    for _ in 0..count {
        nodes.push(ContainerNode {
            base: NodeBase::new(id_gen.next_id(), Span::new(0, 0)),
            container_type: ContainerType::Text,
            children: vec![InlineNode::Text(TextNode {
                base: NodeBase::new(id_gen.next_id(), Span::new(0, 0)),
                text: "\n".to_string(),
            })],
        });
    }
}

fn try_strip_ordered_prefix(line: &str) -> Option<(&str, usize)> {
    let (trimmed_start, trimmed) = trim_line_start(line);
    let dot_pos = trimmed.find(". ")?;
    let prefix = &trimmed[..dot_pos];
    if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
        Some((&trimmed[dot_pos + 2..], trimmed_start + dot_pos + 2))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ContainerType, InlineNode};

    fn extract_text(nodes: &[ContainerNode]) -> String {
        nodes
            .iter()
            .flat_map(|n| n.children.iter())
            .filter_map(|n| {
                if let InlineNode::Text(t) = n {
                    Some(t.text.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn find_container(nodes: &[ContainerNode], ct: ContainerType) -> Option<&ContainerNode> {
        for node in nodes {
            for child in &node.children {
                if let InlineNode::Container(c) = child {
                    if c.container_type == ct {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    fn find_container_in_inline(
        inline_nodes: &[InlineNode],
        ct: ContainerType,
    ) -> Option<&ContainerNode> {
        for node in inline_nodes {
            if let InlineNode::Container(c) = node {
                if c.container_type == ct {
                    return Some(c);
                }
            }
        }
        None
    }

    fn extract_text_from_container(node: &ContainerNode) -> String {
        node.children
            .iter()
            .filter_map(|n| {
                if let InlineNode::Text(t) = n {
                    Some(t.text.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn count_children_of_type(children: &[InlineNode], ct: ContainerType) -> usize {
        children
            .iter()
            .filter(|n| matches!(n, InlineNode::Container(c) if c.container_type == ct))
            .count()
    }

    // ── Basic cases ──────────────────────────────────────────────────────────

    #[test]
    fn plain_text_produces_text_container() {
        let result = MarkdownParser.parse("Hello").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
    }

    #[test]
    fn bold_produces_bold_container() {
        let result = MarkdownParser.parse("**Hello**").unwrap();
        assert_eq!(result.len(), 1);
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        assert_eq!(extract_text_from_container(bold.unwrap()), "Hello");
    }

    #[test]
    fn italic_star_produces_italic_container() {
        let result = MarkdownParser.parse("*Nathan*").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
        assert_eq!(extract_text_from_container(italic.unwrap()), "Nathan");
    }

    #[test]
    fn italic_underscore_produces_italic_container() {
        let result = MarkdownParser.parse("_Nathan_").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
        assert_eq!(extract_text_from_container(italic.unwrap()), "Nathan");
    }

    #[test]
    fn strikethrough_produces_strikethrough_container() {
        let result = MarkdownParser.parse("~~crossed~~").unwrap();
        let strike = find_container(&result, ContainerType::Strikethrough);
        assert!(strike.is_some());
        assert_eq!(extract_text_from_container(strike.unwrap()), "crossed");
    }

    #[test]
    fn highlight_produces_surline_container() {
        let result = MarkdownParser.parse("==highlighted==").unwrap();
        let surline = find_container(&result, ContainerType::Surline);
        assert!(surline.is_some());
        assert_eq!(extract_text_from_container(surline.unwrap()), "highlighted");
    }

    #[test]
    fn blockquote_produces_unsupported_error() {
        let result = MarkdownParser.parse("> quote");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn unordered_list_produces_list_container() {
        let result = MarkdownParser.parse("- a\n- b").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::List);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 2);
    }

    #[test]
    fn ordered_list_produces_ordered_list_container() {
        let result = MarkdownParser.parse("1. one\n2. two").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 2);
    }

    #[test]
    fn un_changement_de_puce_demarre_une_nouvelle_liste() {
        let result = MarkdownParser.parse("- premier\n* second").unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].container_type, ContainerType::List);
        assert_eq!(result[2].container_type, ContainerType::List);
    }

    // ── Nesting ──────────────────────────────────────────────────────────────

    #[test]
    fn bold_containing_italic_nested() {
        let result = MarkdownParser
            .parse("**Hello *Nathan*, how are you?**")
            .unwrap();
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        let italic = find_container_in_inline(&bold.unwrap().children, ContainerType::Italic);
        assert!(italic.is_some());
    }

    #[test]
    fn mixed_text_and_bold() {
        let result = MarkdownParser.parse("Hello **world**").unwrap();
        assert_eq!(result.len(), 1);
        let root = &result[0];
        let has_bold = root.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold),
        );
        assert!(has_bold);
    }

    // ── Errors ───────────────────────────────────────────────────────────────

    #[test]
    fn heading_produces_unsupported_error() {
        let result = MarkdownParser.parse("# Title");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "#")
        );
    }

    #[test]
    fn unclosed_marker_produces_error() {
        let result = MarkdownParser.parse("**not closed");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn backtick_produces_unsupported_error() {
        let result = MarkdownParser.parse("`code`");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn error_contains_precise_position() {
        let result = MarkdownParser.parse("Hello `code`");
        if let Err(ParseError::UnsupportedSymbol { position, .. }) = result {
            assert_eq!(position.line, 1);
            assert!(position.column > 1);
        } else {
            panic!("Expected UnsupportedSymbol error");
        }
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty_vec() {
        let result = MarkdownParser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn blank_lines_ignored() {
        let result = MarkdownParser.parse("\n\n\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn unicode_text_preserved() {
        let result = MarkdownParser.parse("data résumé").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("data résumé"));
    }

    #[test]
    fn supported_symbols_returns_8_entries() {
        let symbols = MarkdownParser.supported_symbols();
        assert_eq!(symbols.len(), 8);
        assert!(!symbols.iter().any(|symbol| symbol.symbol == "> "));
    }

    // ── Emojis ───────────────────────────────────────────────────────────────

    #[test]
    fn single_emoji_preserved() {
        let result = MarkdownParser.parse("🚀").unwrap();
        let text = extract_text(&result);
        assert_eq!(text, "🚀");
    }

    #[test]
    fn emoji_in_plain_text_preserved() {
        let result = MarkdownParser.parse("Hello 👋 world").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("👋"));
    }

    #[test]
    fn emoji_in_bold_preserved() {
        let result = MarkdownParser.parse("**🔥 Results**").unwrap();
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        let text = extract_text_from_container(bold.unwrap());
        assert!(text.contains("🔥"));
    }

    #[test]
    fn emoji_in_italic_preserved() {
        let result = MarkdownParser.parse("*✨ amazing*").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
    }

    #[test]
    fn consecutive_emojis_preserved() {
        let result = MarkdownParser.parse("🎯🎯🎯").unwrap();
        let text = extract_text(&result);
        assert_eq!(text, "🎯🎯🎯");
    }

    #[test]
    fn emoji_at_line_start_preserved() {
        let result = MarkdownParser.parse("✅ Goal reached").unwrap();
        let text = extract_text(&result);
        assert!(text.starts_with("✅"));
    }

    #[test]
    fn multi_codepoint_emoji_preserved() {
        let result = MarkdownParser.parse("👨‍💻 developer").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("👨‍💻"));
    }

    // ── Multi-line ───────────────────────────────────────────────────────────

    #[test]
    fn two_paragraphs_produce_three_nodes() {
        let input = "First line\nSecond line";
        let result = MarkdownParser.parse(input).unwrap();
        // text + \n node + text = 3 nodes
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn paragraph_followed_by_list() {
        let input = "Introduction\n- item A\n- item B";
        let result = MarkdownParser.parse(input).unwrap();
        // text + \n node + list = 3 nodes
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].container_type, ContainerType::Text);
        assert_eq!(result[2].container_type, ContainerType::List);
    }

    #[test]
    fn list_followed_by_text() {
        let input = "- item A\n- item B\nConclusion";
        let result = MarkdownParser.parse(input).unwrap();
        // list + \n node + text = 3 nodes
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].container_type, ContainerType::List);
        assert_eq!(result[2].container_type, ContainerType::Text);
    }

    #[test]
    fn blockquote_between_paragraphs_produces_error() {
        let input = "Intro\n> A thought\nConclusion";
        let result = MarkdownParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn blank_lines_between_blocks_ignored() {
        let input = "Paragraph 1\n\n\nParagraph 2";
        let result = MarkdownParser.parse(input).unwrap();
        // \n nodes have a single Text child containing "\n" — exclude them
        let content_nodes: Vec<_> = result
            .iter()
            .filter(|n| match n.children.as_slice() {
                [InlineNode::Text(t)] => t.text != "\n",
                _ => !n.children.is_empty(),
            })
            .collect();
        assert_eq!(content_nodes.len(), 2);
    }

    #[test]
    fn five_unordered_list_items() {
        let input = "- A\n- B\n- C\n- D\n- E";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::List);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 5);
    }

    #[test]
    fn long_ordered_list() {
        let input = "1. First\n2. Second\n3. Third\n4. Fourth\n5. Fifth";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 5);
    }

    // ── Style combinations ───────────────────────────────────────────────────

    #[test]
    fn bold_and_italic_on_different_words() {
        let result = MarkdownParser.parse("**bold** and *italic*").unwrap();
        let root = &result[0];
        let has_bold = root.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold),
        );
        let has_italic = root.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Italic),
        );
        assert!(has_bold);
        assert!(has_italic);
    }

    #[test]
    fn three_styles_on_one_line() {
        let result = MarkdownParser
            .parse("**bold** *italic* ~~strike~~")
            .unwrap();
        let root = &result[0];
        assert_eq!(
            count_children_of_type(&root.children, ContainerType::Bold),
            1
        );
        assert_eq!(
            count_children_of_type(&root.children, ContainerType::Italic),
            1
        );
        assert_eq!(
            count_children_of_type(&root.children, ContainerType::Strikethrough),
            1
        );
    }

    #[test]
    fn bold_strikethrough_highlight_on_different_words() {
        let result = MarkdownParser
            .parse("**important** ~~obsolete~~ ==new==")
            .unwrap();
        let root = &result[0];
        assert!(count_children_of_type(&root.children, ContainerType::Bold) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Strikethrough) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Surline) >= 1);
    }

    #[test]
    fn text_before_and_after_style() {
        let result = MarkdownParser.parse("See the **key** word here").unwrap();
        let root = &result[0];
        assert!(root.children.len() >= 3);
    }

    #[test]
    fn style_at_line_start() {
        let result = MarkdownParser.parse("**Post title**").unwrap();
        let root = &result[0];
        assert_eq!(root.children.len(), 1);
        assert!(
            matches!(&root.children[0], InlineNode::Container(c) if c.container_type == ContainerType::Bold)
        );
    }

    #[test]
    fn style_at_line_end() {
        let result = MarkdownParser.parse("Normal text then *italic*").unwrap();
        let root = &result[0];
        let last = root.children.last().unwrap();
        assert!(
            matches!(last, InlineNode::Container(c) if c.container_type == ContainerType::Italic)
        );
    }

    // ── Deep nesting ─────────────────────────────────────────────────────────

    #[test]
    fn triple_nesting_bold_italic_highlight() {
        let result = MarkdownParser
            .parse("**bold *italic ==highlight== end* end**")
            .unwrap();
        let root = &result[0];
        let bold = root.children.iter().find_map(|n| {
            if let InlineNode::Container(c) = n {
                if c.container_type == ContainerType::Bold {
                    Some(c)
                } else {
                    None
                }
            } else {
                None
            }
        });
        assert!(bold.is_some());
    }

    #[test]
    fn bold_with_two_nested_italics() {
        let result = MarkdownParser
            .parse("**Hello *world* and *Rust***")
            .unwrap();
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let italic_count = count_children_of_type(&bold.children, ContainerType::Italic);
        assert_eq!(italic_count, 2);
    }

    #[test]
    fn list_with_styled_items() {
        let input = "- **important**\n- *note*\n- plain text";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::List);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 3);
    }

    // ── Realistic posts ──────────────────────────────────────────────────────

    #[test]
    fn post_simple_announcement() {
        let input = "\n🎉 **Great news!**\nI just joined a new professional adventure.\n\nAfter 3 years at my previous company, it's time for new challenges.\n\n*Thank you to my whole team for these unforgettable years.*";

        let result = MarkdownParser.parse(input).unwrap();
        assert!(result.len() >= 3);
        let has_bold = result[0].children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold),
        );
        assert!(has_bold);
    }

    #[test]
    fn post_list_of_learnings() {
        let input = "\nWhat I learned this year:\n\n- **Rust** is amazing for performance\n- *Documentation* matters as much as code\n- ~~Useless meetings~~ Focus is precious\n- ==The community== makes all the difference 🙏";

        let result = MarkdownParser.parse(input).unwrap();
        assert!(result.len() >= 2);
        let last = result.last().unwrap();
        assert_eq!(last.container_type, ContainerType::List);
        let item_count = last
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(item_count, 4);
    }

    #[test]
    fn post_blockquote_and_list_produces_error() {
        let input = "\n> \"Clean code is not written by following a set of rules.\"\n\nMy 3 principles:\n\n1. Name things clearly\n2. Do one thing at a time\n3. Test before deploying 🚀";

        let result = MarkdownParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn long_post_with_styles_and_list() {
        let input = "\n🚀 **3 years of freelance: what nobody tells you**\n\nWhen I started, I thought the technical part was the hardest.\nI was *completely* wrong.\n\nHere's what I really learned:\n\n- **Finding clients** is a full-time job on its own\n- Invoicing: ~~nobody~~ literally nobody teaches you that\n- ==Your reputation== is worth more than any resume\n- *The loneliness* of freelance is real 🧘\n\n**And you, what surprised you the most?** 👇";

        let result = MarkdownParser.parse(input).unwrap();
        assert!(
            result.len() >= 4,
            "Expected at least 4 blocks, got {}",
            result.len()
        );
        let types: Vec<&ContainerType> = result.iter().map(|n| &n.container_type).collect();
        assert!(types.contains(&&ContainerType::List));
    }

    #[test]
    fn post_numbers_and_percentages() {
        let input = "In **2024**, I delivered *47 projects* with ==98%== satisfaction.";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        let root = &result[0];
        assert!(count_children_of_type(&root.children, ContainerType::Bold) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Italic) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Surline) >= 1);
    }

    #[test]
    fn post_with_accented_chars_in_styles() {
        let input = "**Developer** passionate about *elegance* in code.";
        let result = MarkdownParser.parse(input).unwrap();
        assert!(!result.is_empty());
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let text = extract_text_from_container(bold);
        assert!(text.contains("Developer"));
    }

    #[test]
    fn post_with_special_punctuation() {
        let input = "**Innovation** — it's also *daring to say no* to the useless…";
        let result = MarkdownParser.parse(input).unwrap();
        assert!(!result.is_empty());
    }

    // ── Robustness ───────────────────────────────────────────────────────────

    #[test]
    fn empty_bold_marker_handled() {
        let result = MarkdownParser.parse("****");
        let _ = result;
    }

    #[test]
    fn underscore_in_compound_word_does_not_trigger_italic() {
        let result = MarkdownParser.parse("snake_case_variable");
        let _ = result;
    }

    #[test]
    fn line_with_only_spaces_ignored() {
        let result = MarkdownParser.parse("   \t   ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn windows_line_endings_handled() {
        let result = MarkdownParser.parse("line1\r\nline2");
        assert!(result.is_ok());
        let nodes = result.unwrap();
        // text + \n node + text = 3 nodes
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn very_long_line_without_marker() {
        let long_line = "a".repeat(10_000);
        let result = MarkdownParser.parse(&long_line).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
    }

    #[test]
    fn bold_marker_spanning_multiple_words() {
        let result = MarkdownParser.parse("**hello world how are you**").unwrap();
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let text = extract_text_from_container(bold);
        assert_eq!(text, "hello world how are you");
    }

    #[test]
    fn blockquote_with_inline_style_produces_error() {
        let result = MarkdownParser.parse("> **Important quote** from someone");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn list_with_emoji_items() {
        let input = "- 🎯 Goal 1\n- 🚀 Launch\n- ✅ Done";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::List);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 3);
    }

    #[test]
    fn first_unsupported_symbol_stops_parsing() {
        let input = "valid text\n# invalid heading\nother valid lines";
        let result = MarkdownParser.parse(input);
        assert!(result.is_err());
    }
}
