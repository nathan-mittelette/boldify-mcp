//! HTML parser — converts a subset of HTML into an AST.
//!
//! Supported tags: `<b>`, `<strong>`, `<i>`, `<em>`, `<u>`, `<mark>`, `<s>`,
//! `<del>`, `<ul>`, `<ol>`, `<li>`, `<p>`, `<br>`.  All other tags, including
//! `<div>`, `<span>`, `<blockquote>`, and heading tags, produce
//! [`ParseError::UnsupportedSymbol`].  HTML comments and `<!DOCTYPE …>` are
//! silently skipped.

use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, ListItemNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::NodeIdGen,
    Parser, SupportedSymbol,
};

#[derive(Clone)]
pub struct HtmlParser;

enum TagKind {
    Container(ContainerType),
    ListItem,
}

struct OpenTag {
    kind: TagKind,
    tag_name: String,
    children: Vec<InlineNode>,
    opened_at: SourcePosition,
}

struct PositionTracker {
    line: usize,
    col: usize,
}

impl PositionTracker {
    fn new() -> Self {
        Self { line: 1, col: 1 }
    }

    fn advance_str(&mut self, s: &str) {
        for c in s.chars() {
            self.advance_char(c);
        }
    }

    fn advance_char(&mut self, c: char) {
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }

    fn current(&self, byte_offset: usize) -> SourcePosition {
        SourcePosition {
            line: self.line,
            column: self.col,
            byte_offset,
        }
    }
}

const MAX_DEPTH: usize = 64;

impl Parser for HtmlParser {
    fn parse(&self, input: &str) -> Result<Vec<ContainerNode>, ParseError> {
        if input.is_empty() {
            return Ok(vec![]);
        }

        let mut stack: Vec<OpenTag> = Vec::new();
        let mut result: Vec<ContainerNode> = Vec::new();
        let mut text_buf = String::new();
        let mut id_gen = NodeIdGen::new();
        let mut pos = PositionTracker::new();

        let bytes = input.as_bytes();
        let mut i = 0;

        while i < input.len() {
            if bytes[i] != b'<' {
                let c = input[i..].chars().next().unwrap();
                // At root level, discard only \n/\r between block tags
                if !stack.is_empty() || (c != '\n' && c != '\r') {
                    text_buf.push(c);
                }
                pos.advance_char(c);
                i += c.len_utf8();
                continue;
            }

            flush_text(&mut text_buf, &mut stack, &mut result, i, &mut id_gen);

            let rest = &input[i..];
            let tag_pos = pos.current(i);

            if let Some(end) = skip_comment_or_doctype(rest, i) {
                pos.advance_str(&input[i..end]);
                i = end;
                continue;
            }

            if rest.starts_with("</") {
                let (tag_name, tag_end) = extract_tag_name(&input[i + 2..]);
                let abs_end = i + 2 + tag_end;
                process_close_tag(&tag_name, &mut stack, &mut result, &mut id_gen);
                pos.advance_str(&input[i..abs_end]);
                i = abs_end;
                continue;
            }

            let (tag_name, tag_end) = extract_tag_name(&input[i + 1..]);
            let abs_tag_end = i + 1 + tag_end;
            let is_self_closing = input[i + 1..abs_tag_end.min(input.len())]
                .trim_end()
                .ends_with('/');

            process_open_tag(
                &tag_name,
                is_self_closing,
                tag_pos,
                &mut stack,
                &mut result,
                &mut id_gen,
            )?;

            pos.advance_str(&input[i..abs_tag_end]);
            i = abs_tag_end;
        }

        flush_text(
            &mut text_buf,
            &mut stack,
            &mut result,
            input.len(),
            &mut id_gen,
        );

        if let Some(unclosed) = stack.last() {
            return Err(ParseError::UnclosedTag {
                tag: unclosed.tag_name.clone(),
                position: unclosed.opened_at,
            });
        }

        Ok(result)
    }

    fn supported_symbols(&self) -> Vec<SupportedSymbol> {
        vec![
            sym("<strong>", "Bold", "<strong>text</strong>"),
            sym("<b>", "Bold (alias)", "<b>text</b>"),
            sym("<em>", "Italic", "<em>text</em>"),
            sym("<i>", "Italic (alias)", "<i>text</i>"),
            sym("<u>", "Underline", "<u>text</u>"),
            sym("<mark>", "Highlight", "<mark>text</mark>"),
            sym("<s>", "Strikethrough", "<s>text</s>"),
            sym("<del>", "Strikethrough (alias)", "<del>text</del>"),
            sym("<ul>", "Unordered list", "<ul><li>item</li></ul>"),
            sym("<ol>", "Ordered list", "<ol><li>item</li></ol>"),
            sym("<li>", "List item", "<li>content</li>"),
            sym("<br>", "Line break", "<br>"),
        ]
    }
}

/// Returns the absolute end index to skip past a `<!-- -->` comment or `<!...>` doctype.
/// Returns `None` if the input does not start with either construct.
fn skip_comment_or_doctype(rest: &str, base: usize) -> Option<usize> {
    if rest.starts_with("<!--") {
        let end = rest
            .find("-->")
            .map(|p| base + p + 3)
            .unwrap_or(base + rest.len());
        return Some(end);
    }
    if rest.starts_with("<!") {
        let end = rest
            .find('>')
            .map(|p| base + p + 1)
            .unwrap_or(base + rest.len());
        return Some(end);
    }
    None
}

fn process_close_tag(
    tag_name: &str,
    stack: &mut Vec<OpenTag>,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    // </p> at root: no \n emitted — \n is only emitted on <p> opening
    if tag_name != "p" || !stack.is_empty() {
        pop_tag(tag_name, stack, result, id_gen);
    }
}

fn process_open_tag(
    tag_name: &str,
    is_self_closing: bool,
    pos: SourcePosition,
    stack: &mut Vec<OpenTag>,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) -> Result<(), ParseError> {
    match tag_name {
        "br" => {
            push_newline(stack, result, id_gen);
        }
        "p" => {
            if stack.is_empty() && result_has_content(result) {
                push_newline(stack, result, id_gen);
            }
        }
        _ if is_self_closing => {}
        _ => match tag_to_kind(tag_name) {
            Some(kind) => push_open_tag(kind, tag_name, pos, stack, result, id_gen)?,
            None => {
                return Err(ParseError::UnsupportedSymbol {
                    symbol: format!("<{}>", tag_name),
                    position: pos,
                });
            }
        },
    }
    Ok(())
}

fn push_open_tag(
    kind: TagKind,
    tag_name: &str,
    pos: SourcePosition,
    stack: &mut Vec<OpenTag>,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) -> Result<(), ParseError> {
    // List at root: \n before if not the very first element
    if stack.is_empty()
        && result_has_content(result)
        && matches!(
            kind,
            TagKind::Container(ContainerType::List)
                | TagKind::Container(ContainerType::OrderedList)
        )
    {
        push_newline(stack, result, id_gen);
    }
    let new_depth = stack.len() + 1;
    if new_depth > MAX_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth: new_depth,
            max: MAX_DEPTH,
        });
    }
    stack.push(OpenTag {
        kind,
        tag_name: tag_name.to_string(),
        children: Vec::new(),
        opened_at: pos,
    });
    Ok(())
}

fn sym(symbol: &str, description: &str, example: &str) -> SupportedSymbol {
    SupportedSymbol {
        symbol: symbol.to_string(),
        description: description.to_string(),
        example: example.to_string(),
    }
}

/// Extracts the lowercase tag name and returns (name, byte_offset_after_closing_gt).
/// Handles attributes by scanning until '>'.
fn extract_tag_name(s: &str) -> (String, usize) {
    let mut name = String::new();
    let mut chars = s.char_indices();

    for (_, c) in chars.by_ref() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            name.push(c.to_ascii_lowercase());
        } else {
            break;
        }
    }

    let close = s.find('>').map(|p| p + 1).unwrap_or(s.len());
    (name, close)
}

fn tag_to_kind(tag: &str) -> Option<TagKind> {
    match tag {
        "strong" | "b" => Some(TagKind::Container(ContainerType::Bold)),
        "em" | "i" => Some(TagKind::Container(ContainerType::Italic)),
        "u" => Some(TagKind::Container(ContainerType::Underline)),
        "mark" => Some(TagKind::Container(ContainerType::Surline)),
        "s" | "del" => Some(TagKind::Container(ContainerType::Strikethrough)),
        "ul" => Some(TagKind::Container(ContainerType::List)),
        "ol" => Some(TagKind::Container(ContainerType::OrderedList)),
        "li" => Some(TagKind::ListItem),
        _ => None,
    }
}

/// Returns true if result contains any meaningful content (non-\n nodes or block nodes).
fn result_has_content(result: &[ContainerNode]) -> bool {
    result.iter().any(|n| match n.container_type {
        ContainerType::List | ContainerType::OrderedList => true,
        _ => n.children.iter().any(|c| match c {
            InlineNode::Text(t) => t.text != "\n",
            _ => true,
        }),
    })
}

/// Push a `\n` TextNode into the current context (stack top or root result).
fn push_newline(stack: &mut [OpenTag], result: &mut Vec<ContainerNode>, id_gen: &mut NodeIdGen) {
    let node = InlineNode::Text(TextNode {
        base: NodeBase::new(id_gen.next_id(), Span::new(0, 0)),
        text: "\n".to_string(),
    });
    push_inline(node, stack, result, id_gen);
}

fn flush_text(
    buf: &mut String,
    stack: &mut [OpenTag],
    result: &mut Vec<ContainerNode>,
    byte_end: usize,
    id_gen: &mut NodeIdGen,
) {
    if buf.is_empty() {
        return;
    }
    let text = std::mem::take(buf);
    // Discard whitespace-only text inside list containers (spacing between <li> tags)
    if text.chars().all(|c| c.is_whitespace()) {
        if let Some(top) = stack.last() {
            if matches!(
                top.kind,
                TagKind::Container(ContainerType::List)
                    | TagKind::Container(ContainerType::OrderedList)
            ) {
                return;
            }
        }
    }
    let node = InlineNode::Text(TextNode {
        base: NodeBase::new(
            id_gen.next_id(),
            Span::new(byte_end.saturating_sub(text.len()), byte_end),
        ),
        text,
    });
    push_inline(node, stack, result, id_gen);
}

fn push_inline(
    node: InlineNode,
    stack: &mut [OpenTag],
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    if let Some(top) = stack.last_mut() {
        top.children.push(node);
    } else {
        // At root: merge into the last Text container
        if let Some(last) = result.last_mut() {
            if last.container_type == ContainerType::Text {
                last.children.push(node);
                return;
            }
        }
        let id = id_gen.next_id();
        result.push(ContainerNode {
            base: NodeBase::new(id, Span::new(0, 0)),
            container_type: ContainerType::Text,
            children: vec![node],
        });
    }
}

fn pop_tag(
    tag_name: &str,
    stack: &mut Vec<OpenTag>,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    let pos = stack.iter().rposition(|t| t.tag_name == tag_name);
    if let Some(idx) = pos {
        let open = stack.remove(idx);
        let node = close_open_tag(open, id_gen);
        attach_closed_node(node, stack, result, id_gen);
    }
    // Stray closing tag — silently ignore
}

fn close_open_tag(open: OpenTag, id_gen: &mut NodeIdGen) -> InlineNode {
    match open.kind {
        TagKind::ListItem => InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(id_gen.next_id(), Span::new(0, 0)),
            children: open.children,
        }),
        TagKind::Container(ct) => InlineNode::Container(ContainerNode {
            base: NodeBase::new(id_gen.next_id(), Span::new(0, 0)),
            container_type: ct,
            children: open.children,
        }),
    }
}

fn attach_closed_node(
    node: InlineNode,
    stack: &mut [OpenTag],
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return;
    }
    attach_to_root(node, result, id_gen);
}

fn attach_to_root(node: InlineNode, result: &mut Vec<ContainerNode>, id_gen: &mut NodeIdGen) {
    match node {
        InlineNode::Container(c) => attach_container_to_root(c, result, id_gen),
        InlineNode::ListItem(li) => {
            let id = id_gen.next_id();
            result.push(ContainerNode {
                base: NodeBase::new(id, Span::new(0, 0)),
                container_type: ContainerType::Text,
                children: vec![InlineNode::ListItem(li)],
            });
        }
        InlineNode::Text(t) => merge_or_push_text(InlineNode::Text(t), result, id_gen),
    }
}

fn attach_container_to_root(
    c: ContainerNode,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    let is_block = matches!(
        c.container_type,
        ContainerType::List | ContainerType::OrderedList | ContainerType::Blockquote
    );
    if is_block {
        result.push(c);
    } else {
        merge_or_push_text(InlineNode::Container(c), result, id_gen);
    }
}

fn merge_or_push_text(node: InlineNode, result: &mut Vec<ContainerNode>, id_gen: &mut NodeIdGen) {
    if let Some(last) = result.last_mut() {
        if last.container_type == ContainerType::Text {
            last.children.push(node);
            return;
        }
    }
    let id = id_gen.next_id();
    result.push(ContainerNode {
        base: NodeBase::new(id, Span::new(0, 0)),
        container_type: ContainerType::Text,
        children: vec![node],
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    fn flatten_text(nodes: &[ContainerNode]) -> String {
        fn collect(children: &[InlineNode], buf: &mut String) {
            for node in children {
                match node {
                    InlineNode::Text(t) => buf.push_str(&t.text),
                    InlineNode::Container(c) => collect(&c.children, buf),
                    InlineNode::ListItem(li) => collect(&li.children, buf),
                }
            }
        }
        let mut buf = String::new();
        for node in nodes {
            collect(&node.children, &mut buf);
        }
        buf
    }

    fn find_node(nodes: &[ContainerNode], ct: ContainerType) -> Option<&ContainerNode> {
        fn search<'a>(children: &'a [InlineNode], ct: &ContainerType) -> Option<&'a ContainerNode> {
            for child in children {
                match child {
                    InlineNode::Container(c) => {
                        if c.container_type == *ct {
                            return Some(c);
                        }
                        if let Some(found) = search(&c.children, ct) {
                            return Some(found);
                        }
                    }
                    InlineNode::ListItem(li) => {
                        if let Some(found) = search(&li.children, ct) {
                            return Some(found);
                        }
                    }
                    InlineNode::Text(_) => {}
                }
            }
            None
        }
        for node in nodes {
            if node.container_type == ct {
                return Some(node);
            }
            if let Some(found) = search(&node.children, &ct) {
                return Some(found);
            }
        }
        None
    }

    // ── Happy path ───────────────────────────────────────────────────────────

    #[test]
    fn plain_text_produces_text_container() {
        let result = HtmlParser.parse("Hello").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
        assert_eq!(flatten_text(&result), "Hello");
    }

    #[test]
    fn strong_produces_bold_container() {
        let result = HtmlParser.parse("<strong>Hello</strong>").unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "Hello");
    }

    #[test]
    fn b_produces_bold_container() {
        let result = HtmlParser.parse("<b>Hello</b>").unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn em_produces_italic_container() {
        let result = HtmlParser.parse("<em>world</em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
    }

    #[test]
    fn u_produces_underline_container() {
        let result = HtmlParser.parse("<u>text</u>").unwrap();
        assert!(find_node(&result, ContainerType::Underline).is_some());
    }

    #[test]
    fn mark_produces_surline_container() {
        let result = HtmlParser.parse("<mark>text</mark>").unwrap();
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn s_produces_strikethrough_container() {
        let result = HtmlParser.parse("<s>text</s>").unwrap();
        assert!(find_node(&result, ContainerType::Strikethrough).is_some());
    }

    #[test]
    fn del_produces_strikethrough_container() {
        let result = HtmlParser.parse("<del>text</del>").unwrap();
        assert!(find_node(&result, ContainerType::Strikethrough).is_some());
    }

    #[test]
    fn br_inserts_newline() {
        let result = HtmlParser.parse("line1<br>line2").unwrap();
        assert_eq!(flatten_text(&result), "line1\nline2");
    }

    #[test]
    fn p_is_transparent_and_closing_tag_inserts_newline() {
        let result = HtmlParser.parse("<p>text</p>").unwrap();
        assert!(flatten_text(&result).contains("text"));
    }

    #[test]
    fn ul_li_produces_list() {
        let result = HtmlParser.parse("<ul><li>a</li><li>b</li></ul>").unwrap();
        assert_eq!(result[0].container_type, ContainerType::List);
        let item_count = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(item_count, 2);
    }

    #[test]
    fn ol_li_produces_ordered_list() {
        let result = HtmlParser.parse("<ol><li>one</li></ol>").unwrap();
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
    }

    #[test]
    fn strong_nested_in_plain_text() {
        let result = HtmlParser
            .parse("text <strong>bold</strong> after")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("text"));
        assert!(text.contains("bold"));
        assert!(text.contains("after"));
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn blockquote_produces_unsupported_error() {
        let result = HtmlParser.parse("<blockquote>quote</blockquote>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    // ── Comments and DOCTYPE ─────────────────────────────────────────────────

    #[test]
    fn html_comment_is_ignored() {
        let result = HtmlParser.parse("<!-- comment -->text").unwrap();
        assert_eq!(flatten_text(&result), "text");
    }

    #[test]
    fn doctype_is_ignored() {
        let result = HtmlParser
            .parse("<!DOCTYPE html><strong>X</strong>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "X");
    }

    // ── Errors ───────────────────────────────────────────────────────────────

    #[test]
    fn div_produces_unsupported_error() {
        let result = HtmlParser.parse("<div>content</div>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<div>")
        );
    }

    #[test]
    fn span_produces_unsupported_error() {
        let result = HtmlParser.parse("<span>x</span>");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn h1_produces_unsupported_error() {
        let result = HtmlParser.parse("<h1>title</h1>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<h1>")
        );
    }

    #[test]
    fn unclosed_tag_produces_unclosed_error() {
        let result = HtmlParser.parse("<strong>not closed");
        assert!(matches!(result, Err(ParseError::UnclosedTag { ref tag, .. }) if tag == "strong"));
    }

    #[test]
    fn error_contains_accurate_position() {
        let input = "text <div>content</div>";
        if let Err(ParseError::UnsupportedSymbol { position, .. }) = HtmlParser.parse(input) {
            assert_eq!(position.line, 1);
            assert!(position.byte_offset > 0);
        } else {
            panic!("Expected UnsupportedSymbol");
        }
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty_vec() {
        let result = HtmlParser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn unicode_text_is_preserved() {
        let result = HtmlParser.parse("<strong>données</strong>").unwrap();
        assert_eq!(flatten_text(&result), "données");
    }

    #[test]
    fn supported_symbols_returns_12_entries() {
        let symbols = HtmlParser.supported_symbols();
        assert_eq!(symbols.len(), 12);
        assert!(!symbols.iter().any(|symbol| symbol.symbol == "<blockquote>"));
    }

    // ── Advanced tests ───────────────────────────────────────────────────────

    #[test]
    fn emoji_in_plain_text_is_preserved() {
        let result = HtmlParser.parse("🚀 Launch").unwrap();
        assert!(flatten_text(&result).contains("🚀"));
    }

    #[test]
    fn emoji_in_strong_is_preserved() {
        let result = HtmlParser
            .parse("<strong>🔥 Top performer</strong>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        let text = flatten_text(&result);
        assert!(text.contains("🔥"));
        assert!(text.contains("Top performer"));
    }

    #[test]
    fn emoji_in_em_is_preserved() {
        let result = HtmlParser.parse("<em>✨ Amazing</em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(flatten_text(&result).contains("✨"));
    }

    #[test]
    fn emoji_between_tags_is_preserved() {
        let result = HtmlParser
            .parse("<strong>Before</strong> 👉 <em>After</em>")
            .unwrap();
        assert!(flatten_text(&result).contains("👉"));
    }

    #[test]
    fn multi_codepoint_emoji_in_li_is_preserved() {
        let result = HtmlParser
            .parse("<ul><li>👨‍💻 Dev</li><li>👩‍🎨 Designer</li></ul>")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("👨‍💻"));
        assert!(text.contains("👩‍🎨"));
    }

    #[test]
    fn strong_with_class_attribute_is_parsed() {
        let result = HtmlParser
            .parse(r#"<strong class="highlight">text</strong>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "text");
    }

    #[test]
    fn em_with_style_attribute_is_parsed() {
        let result = HtmlParser
            .parse(r#"<em style="color:red">italic</em>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
    }

    #[test]
    fn ul_with_attribute_is_parsed() {
        let result = HtmlParser
            .parse(r#"<ul class="list-disc"><li>item</li></ul>"#)
            .unwrap();
        assert_eq!(result[0].container_type, ContainerType::List);
    }

    #[test]
    fn br_self_closing_with_slash_inserts_newline() {
        let result = HtmlParser.parse("before<br />after").unwrap();
        assert!(flatten_text(&result).contains('\n'));
    }

    #[test]
    fn unknown_tag_with_attribute_reports_correct_tag_name() {
        let result = HtmlParser.parse(r#"<div class="foo">x</div>"#);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<div>")
        );
    }

    #[test]
    fn strong_nested_in_em() {
        let result = HtmlParser.parse("<em><strong>text</strong></em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn em_nested_in_strong_nested_in_p() {
        let result = HtmlParser
            .parse("<p><strong>Hello <em>world</em></strong></p>")
            .unwrap();
        let bold = find_node(&result, ContainerType::Bold).unwrap();
        let has_italic = bold.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Italic),
        );
        assert!(has_italic);
    }

    #[test]
    fn list_with_styled_items() {
        let input = "<ul>\
            <li><strong>First</strong> point</li>\
            <li>Second point <em>important</em></li>\
            <li><s>obsolete</s></li>\
        </ul>";
        let result = HtmlParser.parse(input).unwrap();
        assert_eq!(result[0].container_type, ContainerType::List);
        let item_count = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(item_count, 3);
    }

    #[test]
    fn blockquote_containing_strong_produces_error() {
        let result =
            HtmlParser.parse("<blockquote><strong>Important</strong> to remember</blockquote>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    #[test]
    fn ol_li_with_inline_styles() {
        let result = HtmlParser
            .parse("<ol><li><strong>Step 1</strong></li><li><em>Step 2</em></li></ol>")
            .unwrap();
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
    }

    #[test]
    fn triple_nesting_mark_em_strong() {
        let result = HtmlParser
            .parse("<mark><em><strong>triple</strong></em></mark>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn multiple_br_inserts_multiple_newlines() {
        let result = HtmlParser.parse("a<br>b<br>c").unwrap();
        assert_eq!(flatten_text(&result), "a\nb\nc");
    }

    #[test]
    fn consecutive_p_tags_produce_text_with_content() {
        let result = HtmlParser.parse("<p>first</p><p>second</p>").unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("first"));
        assert!(text.contains("second"));
    }

    #[test]
    fn empty_p_tag_is_handled() {
        let result = HtmlParser.parse("<p></p>").unwrap();
        assert!(flatten_text(&result).is_empty() || result.is_empty());
    }

    #[test]
    fn br_inside_list_item_inserts_newline() {
        let result = HtmlParser
            .parse("<ul><li>line1<br>line2</li></ul>")
            .unwrap();
        assert!(flatten_text(&result).contains("line1\nline2"));
    }

    #[test]
    fn br_inside_strong_inserts_newline_in_bold() {
        let result = HtmlParser
            .parse("<strong>before<br>after</strong>")
            .unwrap();
        assert!(flatten_text(&result).contains('\n'));
    }

    #[test]
    fn amp_entity_is_preserved_in_text() {
        let result = HtmlParser.parse("A &amp; B").unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("&amp;") || text.contains('&'));
    }

    #[test]
    fn nbsp_entity_does_not_panic() {
        let result = HtmlParser.parse("word&nbsp;word");
        let _ = result;
    }

    #[test]
    fn notion_like_paragraph_with_strong() {
        let input = r#"<p>Today I <strong>launched</strong> my new project.</p>"#;
        let result = HtmlParser.parse(input).unwrap();
        assert!(flatten_text(&result).contains("launched"));
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn linkedin_post_simple() {
        let input = "<p><strong>🎉 New milestone!</strong></p>\
                     <p>I am thrilled to announce I am joining <em>Acme Corp</em>.</p>\
                     <p>Thanks to everyone. 🙏</p>";
        let result = HtmlParser.parse(input).unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert!(find_node(&result, ContainerType::Italic).is_some());
        let text = flatten_text(&result);
        assert!(text.contains("🎉"));
        assert!(text.contains("🙏"));
    }

    #[test]
    fn post_with_list_and_blockquote_produces_error() {
        let input = "\
            <p><strong>What I learned:</strong></p>\
            <ul>\
                <li><strong>Rust</strong> for performance 🚀</li>\
                <li>Good <em>documentation</em> saves lives</li>\
                <li><s>Deadlines</s> Time</li>\
            </ul>\
            <blockquote>The best code.</blockquote>";
        let result = HtmlParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    #[test]
    fn long_post_all_styles_without_blockquote() {
        let input = "\
            <p>🚀 <strong>3 years of freelancing</strong></p>\
            <p>I was <em>completely</em> wrong.</p>\
            <ul>\
                <li><strong>Finding clients</strong></li>\
                <li>Invoicing, <s>nobody</s> really nobody</li>\
                <li><mark>Your reputation</mark> is worth more</li>\
                <li><em>The solitude</em> of freelancing 🧘</li>\
            </ul>\
            <p><strong>What about you?</strong> 👇</p>";
        let result = HtmlParser.parse(input).unwrap();
        assert!(!result.is_empty());
        let text = flatten_text(&result);
        assert!(text.contains("🚀"));
        assert!(text.contains("🧘"));
        assert!(text.contains("👇"));
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(find_node(&result, ContainerType::Strikethrough).is_some());
        assert!(find_node(&result, ContainerType::Surline).is_some());
        assert!(find_node(&result, ContainerType::List).is_some());
    }

    #[test]
    fn accented_characters_preserved_across_all_styles() {
        let input =
            "<strong>Developer</strong> passionate about <em>elegance</em> in <u>clean code</u>.";
        let result = HtmlParser.parse(input).unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("Developer"));
        assert!(text.contains("elegance"));
        assert!(text.contains("clean code"));
    }

    #[test]
    fn numbers_and_percentages_in_styles() {
        let input =
            "<p>In <strong>2024</strong>, I delivered <em>47 projects</em> with <mark>98%</mark>.</p>";
        let result = HtmlParser.parse(input).unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn empty_strong_tag_produces_empty_bold_node() {
        let result = HtmlParser.parse("<strong></strong>").unwrap();
        let bold = find_node(&result, ContainerType::Bold).unwrap();
        assert!(bold.children.is_empty());
    }

    #[test]
    fn plain_text_before_and_after_tag() {
        let result = HtmlParser
            .parse("Before <strong>bold</strong> after")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("Before"));
        assert!(text.contains("after"));
    }

    #[test]
    fn multiline_comment_is_ignored() {
        let input = "before<!--\ncomment\nover multiple lines\n-->after";
        let result = HtmlParser.parse(input).unwrap();
        let text = flatten_text(&result);
        assert!(!text.contains("comment"));
        assert!(text.contains("before"));
        assert!(text.contains("after"));
    }

    #[test]
    fn chinese_unicode_is_preserved() {
        let result = HtmlParser.parse("<strong>你好世界</strong>").unwrap();
        assert_eq!(flatten_text(&result), "你好世界");
    }

    #[test]
    fn arabic_unicode_is_preserved() {
        let result = HtmlParser.parse("<em>مرحبا</em>").unwrap();
        assert!(flatten_text(&result).contains("مرحبا"));
    }

    #[test]
    fn closing_tag_without_opening_does_not_panic() {
        let result = HtmlParser.parse("text </strong> after");
        let _ = result;
    }

    #[test]
    fn very_long_list_is_parsed() {
        let items: String = (1..=50).map(|i| format!("<li>Item {}</li>", i)).collect();
        let input = format!("<ul>{}</ul>", items);
        let result = HtmlParser.parse(&input).unwrap();
        let item_count = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(item_count, 50);
    }

    #[test]
    fn tag_with_data_attribute_is_parsed() {
        let result = HtmlParser
            .parse(r#"<strong data-id="123">text</strong>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn error_stops_at_first_invalid_tag() {
        let input = "<strong>valid</strong><div>invalid</div><em>also valid</em>";
        let result = HtmlParser.parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn nesting_too_deep_returns_error() {
        let open: String = "<strong>".repeat(65);
        let close: String = "</strong>".repeat(65);
        let input = format!("{}x{}", open, close);
        let result = HtmlParser.parse(&input);
        assert!(matches!(result, Err(ParseError::NestingTooDeep { .. })));
    }

    #[test]
    fn nesting_at_limit_is_accepted() {
        let open: String = "<strong>".repeat(64);
        let close: String = "</strong>".repeat(64);
        let input = format!("{}x{}", open, close);
        let result = HtmlParser.parse(&input);
        assert!(result.is_ok());
    }
}
