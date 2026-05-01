use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::NodeIdGen,
};

/// Symboles Markdown reconnus, du plus long au plus court.
const MARKDOWN_MARKERS: &[(&str, ContainerType)] = &[
    ("**", ContainerType::Bold),
    ("~~", ContainerType::Strikethrough),
    ("==", ContainerType::Surline),
    ("__", ContainerType::Underline),
    ("*", ContainerType::Italic),
    ("_", ContainerType::Italic),
];

pub fn parse_inline(
    input: &str,
    line: usize,
    byte_start: usize,
    column_start: usize,
    id_gen: &mut NodeIdGen,
) -> Result<Vec<InlineNode>, ParseError> {
    let outcome = parse_inline_inner(input, line, byte_start, column_start, id_gen, None)?;
    Ok(outcome.nodes)
}

struct ParseOutcome {
    nodes: Vec<InlineNode>,
    consumed_bytes: usize,
    consumed_chars: usize,
    closed: bool,
}

fn parse_inline_inner(
    input: &str,
    line: usize,
    byte_start: usize,
    column_start: usize,
    id_gen: &mut NodeIdGen,
    closing_marker: Option<&str>,
) -> Result<ParseOutcome, ParseError> {
    let mut nodes = Vec::new();
    let mut current_text = String::new();
    let mut current_text_start = byte_start;
    let mut i = 0;
    let mut char_offset = 0;

    while i < input.len() {
        if let Some(marker) = closing_marker {
            if input[i..].starts_with(marker) {
                flush_text(
                    &mut nodes,
                    &mut current_text,
                    current_text_start,
                    byte_start + i,
                    id_gen,
                );
                return Ok(ParseOutcome {
                    nodes,
                    consumed_bytes: i + marker.len(),
                    consumed_chars: char_offset + marker.chars().count(),
                    closed: true,
                });
            }
        }

        if let Some((marker, container_type)) = detect_marker(&input[i..]) {
            flush_text(
                &mut nodes,
                &mut current_text,
                current_text_start,
                byte_start + i,
                id_gen,
            );

            let inner_start = i + marker.len();
            let marker_chars = marker.chars().count();
            let inner = parse_inline_inner(
                &input[inner_start..],
                line,
                byte_start + inner_start,
                column_start + char_offset + marker_chars,
                id_gen,
                Some(marker),
            )?;

            if !inner.closed {
                return Err(ParseError::UnsupportedSymbol {
                    symbol: marker.to_string(),
                    position: SourcePosition {
                        line,
                        column: column_start + char_offset,
                        byte_offset: byte_start + i,
                    },
                });
            }

            let consumed_bytes = marker.len() + inner.consumed_bytes;
            let consumed_chars = marker_chars + inner.consumed_chars;

            nodes.push(InlineNode::Container(ContainerNode {
                base: NodeBase::new(
                    id_gen.next_id(),
                    Span::new(byte_start + i, byte_start + i + consumed_bytes),
                ),
                container_type,
                children: inner.nodes,
            }));

            i += consumed_bytes;
            char_offset += consumed_chars;
            current_text_start = byte_start + i;
            continue;
        }

        let c = input[i..]
            .chars()
            .next()
            .expect("char boundary must point to a character");

        if is_unsupported_symbol(c) {
            return Err(ParseError::UnsupportedSymbol {
                symbol: extract_symbol(input, i),
                position: SourcePosition {
                    line,
                    column: column_start + char_offset,
                    byte_offset: byte_start + i,
                },
            });
        }

        current_text.push(c);
        i += c.len_utf8();
        char_offset += 1;
    }

    flush_text(
        &mut nodes,
        &mut current_text,
        current_text_start,
        byte_start + input.len(),
        id_gen,
    );

    Ok(ParseOutcome {
        nodes,
        consumed_bytes: input.len(),
        consumed_chars: char_offset,
        closed: closing_marker.is_none(),
    })
}

fn flush_text(
    nodes: &mut Vec<InlineNode>,
    current_text: &mut String,
    start: usize,
    end: usize,
    id_gen: &mut NodeIdGen,
) {
    if current_text.is_empty() {
        return;
    }

    nodes.push(make_text(&std::mem::take(current_text), start, end, id_gen));
}

fn make_text(text: &str, start: usize, end: usize, id_gen: &mut NodeIdGen) -> InlineNode {
    InlineNode::Text(TextNode {
        base: NodeBase::new(id_gen.next_id(), Span::new(start, end)),
        text: text.to_string(),
    })
}

fn detect_marker(s: &str) -> Option<(&'static str, ContainerType)> {
    for &(marker, ref ct) in MARKDOWN_MARKERS {
        if s.starts_with(marker) {
            return Some((marker, ct.clone()));
        }
    }
    None
}

fn is_unsupported_symbol(c: char) -> bool {
    matches!(c, '#' | '`' | '[' | ']' | '<' | '>' | '(' | ')' | '{' | '}')
}

fn extract_symbol(s: &str, from: usize) -> String {
    let rest = &s[from..];
    if rest.starts_with('<') {
        let end = rest.find('>').map(|i| i + 1).unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        rest.chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_default()
    }
}
