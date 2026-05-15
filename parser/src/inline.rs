use crate::{
    ast::{ContainerNode, ContainerType, InlineNode, NodeBase, Span, TextNode},
    error::{ParseError, SourcePosition},
    id::NodeIdGen,
};

const MAX_DEPTH: usize = 64;

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
    let outcome = parse_inline_inner(input, line, byte_start, column_start, id_gen, None, 0)?;
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
    depth: usize,
) -> Result<ParseOutcome, ParseError> {
    if depth > MAX_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth,
            max: MAX_DEPTH,
        });
    }
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
                depth + 1,
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

        // Backslash escape: \X produces X literally (e.g. \# \* \~ \\ )
        if c == '\\' {
            if let Some(next) = input[i + 1..].chars().next() {
                current_text.push(next);
                i += 1 + next.len_utf8();
                char_offset += 2;
                continue;
            }
        }

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
    matches!(c, '`' | '[' | ']' | '<' | '>')
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

#[cfg(test)]
mod tests {
    use crate::id::NodeIdGen;

    use super::*;

    fn parse(input: &str) -> Vec<InlineNode> {
        parse_inline(input, 1, 0, 1, &mut NodeIdGen::new()).unwrap()
    }

    fn text_content(nodes: &[InlineNode]) -> String {
        fn collect(nodes: &[InlineNode], buf: &mut String) {
            for node in nodes {
                match node {
                    InlineNode::Text(t) => buf.push_str(&t.text),
                    InlineNode::Container(c) => collect(&c.children, buf),
                    InlineNode::ListItem(li) => collect(&li.children, buf),
                }
            }
        }
        let mut buf = String::new();
        collect(nodes, &mut buf);
        buf
    }

    // ── Échappement backslash ────────────────────────────────────────────────

    #[test]
    fn backslash_diese_produit_diese_litteral() {
        let nodes = parse(r"\#hashtag");
        assert_eq!(text_content(&nodes), "#hashtag");
    }

    #[test]
    fn backslash_etoile_produit_etoile_litterale() {
        let nodes = parse(r"\*pas italique\*");
        assert_eq!(text_content(&nodes), "*pas italique*");
    }

    #[test]
    fn backslash_double_etoile_premier_echappe_second_ouvre_gras() {
        // \** → '*' littéral puis '*' ouvre l'italique → pas fermé → erreur
        let result = parse_inline(r"\**gras", 1, 0, 1, &mut NodeIdGen::new());
        // Le premier * est échappé, le second ouvre un marqueur non fermé
        assert!(result.is_err());
    }

    #[test]
    fn backslash_tilde_produit_tilde_litteral() {
        let nodes = parse(r"\~~pas barré\~~");
        assert_eq!(text_content(&nodes), "~~pas barré~~");
    }

    #[test]
    fn backslash_backslash_produit_backslash_litteral() {
        let nodes = parse(r"\\");
        assert_eq!(text_content(&nodes), "\\");
    }

    #[test]
    fn backslash_en_fin_de_chaine_produit_backslash() {
        // Un \ seul en fin de chaîne : pas de caractère suivant → traité comme texte
        let nodes = parse("texte\\");
        assert_eq!(text_content(&nodes), "texte\\");
    }

    #[test]
    fn backslash_echappe_crochet_ouvrant() {
        let nodes = parse(r"\[pas un lien");
        assert_eq!(text_content(&nodes), "[pas un lien");
    }

    #[test]
    fn backslash_dans_gras_echappe_etoile() {
        // **avant \* après** → bold contient "avant * après"
        let nodes = parse(r"**avant \* après**");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], InlineNode::Container(c) if c.container_type == ContainerType::Bold)
        );
        assert_eq!(text_content(&nodes), "avant * après");
    }

    // ── Caractères désormais autorisés ───────────────────────────────────────

    #[test]
    fn parentheses_autorisees_dans_texte() {
        let nodes = parse("voir (ci-dessous)");
        assert_eq!(text_content(&nodes), "voir (ci-dessous)");
    }

    #[test]
    fn accolades_autorisees_dans_texte() {
        let nodes = parse("valeur={true}");
        assert_eq!(text_content(&nodes), "valeur={true}");
    }

    // ── Symboles toujours interdits ──────────────────────────────────────────

    #[test]
    fn backtick_toujours_interdit() {
        let result = parse_inline("`code`", 1, 0, 1, &mut NodeIdGen::new());
        assert!(result.is_err());
    }

    #[test]
    fn crochet_ouvrant_toujours_interdit() {
        let result = parse_inline("[lien]", 1, 0, 1, &mut NodeIdGen::new());
        assert!(result.is_err());
    }

    #[test]
    fn imbrication_trop_profonde_retourne_nesting_too_deep() {
        use crate::error::ParseError;
        // 65 niveaux d'imbrication : **__**__**__... (alternance bold/underline)
        let markers = ["**", "__"];
        let open: String = (0..65).map(|i| markers[i % 2]).collect();
        let close: String = (0..65).rev().map(|i| markers[i % 2]).collect();
        let input = format!("{}x{}", open, close);
        let result = parse_inline(&input, 1, 0, 1, &mut NodeIdGen::new());
        assert!(matches!(result, Err(ParseError::NestingTooDeep { .. })));
    }

    #[test]
    fn imbrication_a_la_limite_est_acceptee() {
        // 64 niveaux exactement : doit passer
        let markers = ["**", "__"];
        let open: String = (0..64).map(|i| markers[i % 2]).collect();
        let close: String = (0..64).rev().map(|i| markers[i % 2]).collect();
        let input = format!("{}x{}", open, close);
        let result = parse_inline(&input, 1, 0, 1, &mut NodeIdGen::new());
        assert!(result.is_ok());
    }
}
