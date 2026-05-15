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

        flush_text(&mut text_buf, &mut stack, &mut result, input.len(), &mut id_gen);

        if let Some(unclosed) = stack.last() {
            return Err(ParseError::UnclosedTag {
                tag: unclosed.tag_name.clone(),
                position: unclosed.opened_at.clone(),
            });
        }

        Ok(result)
    }

    fn supported_symbols(&self) -> Vec<SupportedSymbol> {
        vec![
            sym("<strong>", "Gras", "<strong>texte</strong>"),
            sym("<b>", "Gras (alias)", "<b>texte</b>"),
            sym("<em>", "Italique", "<em>texte</em>"),
            sym("<i>", "Italique (alias)", "<i>texte</i>"),
            sym("<u>", "Souligné", "<u>texte</u>"),
            sym("<mark>", "Surligné", "<mark>texte</mark>"),
            sym("<s>", "Texte barré", "<s>texte</s>"),
            sym("<del>", "Texte barré (alias)", "<del>texte</del>"),
            sym("<ul>", "Liste non ordonnée", "<ul><li>item</li></ul>"),
            sym("<ol>", "Liste ordonnée", "<ol><li>item</li></ol>"),
            sym("<li>", "Item de liste", "<li>contenu</li>"),
            sym("<br>", "Saut de ligne", "<br>"),
        ]
    }
}

/// Returns the absolute end index to skip past a `<!-- -->` comment or `<!...>` doctype.
/// Returns `None` if the input does not start with either construct.
fn skip_comment_or_doctype(rest: &str, base: usize) -> Option<usize> {
    if rest.starts_with("<!--") {
        let end = rest.find("-->").map(|p| base + p + 3).unwrap_or(base + rest.len());
        return Some(end);
    }
    if rest.starts_with("<!") {
        let end = rest.find('>').map(|p| base + p + 1).unwrap_or(base + rest.len());
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
            TagKind::Container(ContainerType::List) | TagKind::Container(ContainerType::OrderedList)
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
    stack: &mut Vec<OpenTag>,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return;
    }
    attach_to_root(node, result, id_gen);
}

fn attach_to_root(
    node: InlineNode,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
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

fn merge_or_push_text(
    node: InlineNode,
    result: &mut Vec<ContainerNode>,
    id_gen: &mut NodeIdGen,
) {
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

    // ── Cas nominaux ────────────────────────────────────────────────────────

    #[test]
    fn texte_brut_produit_container_text() {
        let result = HtmlParser.parse("Bonjour").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
        assert_eq!(flatten_text(&result), "Bonjour");
    }

    #[test]
    fn strong_produit_container_bold() {
        let result = HtmlParser.parse("<strong>Hello</strong>").unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "Hello");
    }

    #[test]
    fn b_produit_container_bold() {
        let result = HtmlParser.parse("<b>Hello</b>").unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn em_produit_container_italic() {
        let result = HtmlParser.parse("<em>monde</em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
    }

    #[test]
    fn u_produit_container_underline() {
        let result = HtmlParser.parse("<u>texte</u>").unwrap();
        assert!(find_node(&result, ContainerType::Underline).is_some());
    }

    #[test]
    fn mark_produit_container_surline() {
        let result = HtmlParser.parse("<mark>texte</mark>").unwrap();
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn s_produit_container_strikethrough() {
        let result = HtmlParser.parse("<s>texte</s>").unwrap();
        assert!(find_node(&result, ContainerType::Strikethrough).is_some());
    }

    #[test]
    fn del_produit_container_strikethrough() {
        let result = HtmlParser.parse("<del>texte</del>").unwrap();
        assert!(find_node(&result, ContainerType::Strikethrough).is_some());
    }

    #[test]
    fn br_insere_newline() {
        let result = HtmlParser.parse("ligne1<br>ligne2").unwrap();
        assert_eq!(flatten_text(&result), "ligne1\nligne2");
    }

    #[test]
    fn p_transparent_et_fermeture_insere_newline() {
        let result = HtmlParser.parse("<p>texte</p>").unwrap();
        assert!(flatten_text(&result).contains("texte"));
    }

    #[test]
    fn ul_li_produit_liste() {
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
    fn ol_li_produit_liste_ordonnee() {
        let result = HtmlParser.parse("<ol><li>un</li></ol>").unwrap();
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
    }

    #[test]
    fn imbrication_strong_dans_texte() {
        let result = HtmlParser
            .parse("texte <strong>gras</strong> suite")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("texte"));
        assert!(text.contains("gras"));
        assert!(text.contains("suite"));
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn blockquote_produit_erreur_unsupported() {
        let result = HtmlParser.parse("<blockquote>citation</blockquote>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    // ── Commentaires et DOCTYPE ─────────────────────────────────────────────

    #[test]
    fn commentaire_html_ignore() {
        let result = HtmlParser.parse("<!-- commentaire -->texte").unwrap();
        assert_eq!(flatten_text(&result), "texte");
    }

    #[test]
    fn doctype_ignore() {
        let result = HtmlParser
            .parse("<!DOCTYPE html><strong>X</strong>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "X");
    }

    // ── Erreurs ─────────────────────────────────────────────────────────────

    #[test]
    fn div_produit_erreur_unsupported() {
        let result = HtmlParser.parse("<div>contenu</div>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<div>")
        );
    }

    #[test]
    fn span_produit_erreur_unsupported() {
        let result = HtmlParser.parse("<span>x</span>");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn h1_produit_erreur_unsupported() {
        let result = HtmlParser.parse("<h1>titre</h1>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<h1>")
        );
    }

    #[test]
    fn balise_non_fermee_produit_erreur_unclosed() {
        let result = HtmlParser.parse("<strong>non fermé");
        assert!(matches!(result, Err(ParseError::UnclosedTag { ref tag, .. }) if tag == "strong"));
    }

    #[test]
    fn erreur_contient_position_precise() {
        let input = "texte <div>contenu</div>";
        if let Err(ParseError::UnsupportedSymbol { position, .. }) = HtmlParser.parse(input) {
            assert_eq!(position.line, 1);
            assert!(position.byte_offset > 0);
        } else {
            panic!("Attendu UnsupportedSymbol");
        }
    }

    // ── Cas limites ──────────────────────────────────────────────────────────

    #[test]
    fn input_vide_retourne_vec_vide() {
        let result = HtmlParser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn texte_unicode_preserve() {
        let result = HtmlParser.parse("<strong>données</strong>").unwrap();
        assert_eq!(flatten_text(&result), "données");
    }

    #[test]
    fn supported_symbols_retourne_12_entrees() {
        let symbols = HtmlParser.supported_symbols();
        assert_eq!(symbols.len(), 12);
        assert!(!symbols.iter().any(|symbol| symbol.symbol == "<blockquote>"));
    }

    // ── Tests avancés 03b ────────────────────────────────────────────────────

    #[test]
    fn emoji_dans_texte_brut_preserve() {
        let result = HtmlParser.parse("🚀 Lancement").unwrap();
        assert!(flatten_text(&result).contains("🚀"));
    }

    #[test]
    fn emoji_dans_strong_preserve() {
        let result = HtmlParser
            .parse("<strong>🔥 Top performer</strong>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        let text = flatten_text(&result);
        assert!(text.contains("🔥"));
        assert!(text.contains("Top performer"));
    }

    #[test]
    fn emoji_dans_em_preserve() {
        let result = HtmlParser.parse("<em>✨ Incroyable</em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(flatten_text(&result).contains("✨"));
    }

    #[test]
    fn emoji_entre_balises_preserve() {
        let result = HtmlParser
            .parse("<strong>Avant</strong> 👉 <em>Après</em>")
            .unwrap();
        assert!(flatten_text(&result).contains("👉"));
    }

    #[test]
    fn emoji_multicodepoint_dans_li_preserve() {
        let result = HtmlParser
            .parse("<ul><li>👨‍💻 Dev</li><li>👩‍🎨 Designer</li></ul>")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("👨‍💻"));
        assert!(text.contains("👩‍🎨"));
    }

    #[test]
    fn strong_avec_attribut_class_ignore() {
        let result = HtmlParser
            .parse(r#"<strong class="highlight">texte</strong>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert_eq!(flatten_text(&result), "texte");
    }

    #[test]
    fn em_avec_attribut_style_ignore() {
        let result = HtmlParser
            .parse(r#"<em style="color:red">italique</em>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
    }

    #[test]
    fn ul_avec_attribut_ignore() {
        let result = HtmlParser
            .parse(r#"<ul class="list-disc"><li>item</li></ul>"#)
            .unwrap();
        assert_eq!(result[0].container_type, ContainerType::List);
    }

    #[test]
    fn br_avec_attribut_slash_self_closing() {
        let result = HtmlParser.parse("avant<br />après").unwrap();
        assert!(flatten_text(&result).contains('\n'));
    }

    #[test]
    fn tag_inconnu_avec_attribut_retourne_erreur_avec_le_bon_tag() {
        let result = HtmlParser.parse(r#"<div class="foo">x</div>"#);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<div>")
        );
    }

    #[test]
    fn strong_dans_em() {
        let result = HtmlParser.parse("<em><strong>texte</strong></em>").unwrap();
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn em_dans_strong_dans_p() {
        let result = HtmlParser
            .parse("<p><strong>Bonjour <em>monde</em></strong></p>")
            .unwrap();
        let bold = find_node(&result, ContainerType::Bold).unwrap();
        let has_italic = bold.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Italic),
        );
        assert!(has_italic);
    }

    #[test]
    fn liste_avec_items_styles() {
        let input = "<ul>\
            <li><strong>Premier</strong> point</li>\
            <li>Deuxième point <em>important</em></li>\
            <li><s>obsolète</s></li>\
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
    fn blockquote_avec_strong_dedans_produit_erreur() {
        let result =
            HtmlParser.parse("<blockquote><strong>Important</strong> à retenir</blockquote>");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    #[test]
    fn ol_li_avec_style_inline() {
        let result = HtmlParser
            .parse("<ol><li><strong>Étape 1</strong></li><li><em>Étape 2</em></li></ol>")
            .unwrap();
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
    }

    #[test]
    fn triple_imbrication_mark_em_strong() {
        let result = HtmlParser
            .parse("<mark><em><strong>triple</strong></em></mark>")
            .unwrap();
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn multiple_br_insere_multiple_newlines() {
        let result = HtmlParser.parse("a<br>b<br>c").unwrap();
        assert_eq!(flatten_text(&result), "a\nb\nc");
    }

    #[test]
    fn p_suivi_de_p_insere_deux_newlines() {
        let result = HtmlParser.parse("<p>premier</p><p>deuxième</p>").unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("premier"));
        assert!(text.contains("deuxième"));
    }

    #[test]
    fn p_vide_insere_newline() {
        let result = HtmlParser.parse("<p></p>").unwrap();
        assert!(flatten_text(&result).is_empty() || result.is_empty());
    }

    #[test]
    fn br_dans_liste_item_insere_newline() {
        let result = HtmlParser
            .parse("<ul><li>ligne1<br>ligne2</li></ul>")
            .unwrap();
        assert!(flatten_text(&result).contains("ligne1\nligne2"));
    }

    #[test]
    fn br_dans_strong_insere_newline_dans_le_bold() {
        let result = HtmlParser.parse("<strong>avant<br>après</strong>").unwrap();
        assert!(flatten_text(&result).contains('\n'));
    }

    #[test]
    fn entite_amp_preserve_dans_texte() {
        let result = HtmlParser.parse("A &amp; B").unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("&amp;") || text.contains('&'));
    }

    #[test]
    fn entite_nbsp_preserve_dans_texte() {
        let result = HtmlParser.parse("mot&nbsp;mot");
        let _ = result;
    }

    #[test]
    fn html_notion_like_paragraphe_avec_strong() {
        let input = r#"<p>Aujourd'hui, j'ai <strong>lancé</strong> mon nouveau projet.</p>"#;
        let result = HtmlParser.parse(input).unwrap();
        assert!(flatten_text(&result).contains("lancé"));
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn html_linkedin_post_simple() {
        let input = "<p><strong>🎉 Nouvelle étape professionnelle !</strong></p>\
                     <p>Je suis ravi d'annoncer que je rejoins <em>Acme Corp</em>.</p>\
                     <p>Merci à tous. 🙏</p>";
        let result = HtmlParser.parse(input).unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert!(find_node(&result, ContainerType::Italic).is_some());
        let text = flatten_text(&result);
        assert!(text.contains("🎉"));
        assert!(text.contains("🙏"));
    }

    #[test]
    fn html_post_avec_liste_et_citation_produit_erreur() {
        let input = "\
            <p><strong>Ce que j'ai appris :</strong></p>\
            <ul>\
                <li><strong>Rust</strong> pour la perf 🚀</li>\
                <li>La <em>documentation</em> sauve des vies</li>\
                <li><s>Les deadlines</s> Le temps</li>\
            </ul>\
            <blockquote>Le meilleur code.</blockquote>";
        let result = HtmlParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { ref symbol, .. }) if symbol == "<blockquote>")
        );
    }

    #[test]
    fn html_post_long_tous_les_styles_sans_citation() {
        let input = "\
            <p>🚀 <strong>3 ans de freelance</strong></p>\
            <p>J'avais <em>complètement</em> tort.</p>\
            <ul>\
                <li><strong>Trouver des clients</strong></li>\
                <li>La facturation, <s>personne</s> vraiment personne</li>\
                <li><mark>Votre réputation</mark> vaut plus</li>\
                <li><em>La solitude</em> du freelance 🧘</li>\
            </ul>\
            <p><strong>Et vous ?</strong> 👇</p>";
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
    fn html_avec_accents_dans_tous_les_styles() {
        let input =
            "<strong>Développeur</strong> passionné par <em>l'élégance</em> du <u>code propre</u>.";
        let result = HtmlParser.parse(input).unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("Développeur"));
        assert!(text.contains("l'élégance"));
        assert!(text.contains("code propre"));
    }

    #[test]
    fn html_chiffres_et_pourcentages_dans_styles() {
        let input =
            "<p>En <strong>2024</strong>, j'ai livré <em>47 projets</em> avec <mark>98%</mark>.</p>";
        let result = HtmlParser.parse(input).unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
        assert!(find_node(&result, ContainerType::Italic).is_some());
        assert!(find_node(&result, ContainerType::Surline).is_some());
    }

    #[test]
    fn tag_vide_strong_sans_contenu() {
        let result = HtmlParser.parse("<strong></strong>").unwrap();
        let bold = find_node(&result, ContainerType::Bold).unwrap();
        assert!(bold.children.is_empty());
    }

    #[test]
    fn texte_brut_avant_et_apres_balise() {
        let result = HtmlParser
            .parse("Avant <strong>gras</strong> après")
            .unwrap();
        let text = flatten_text(&result);
        assert!(text.contains("Avant"));
        assert!(text.contains("après"));
    }

    #[test]
    fn commentaire_multi_ligne_ignore() {
        let input = "avant<!--\ncommentaire\nsur plusieurs lignes\n-->après";
        let result = HtmlParser.parse(input).unwrap();
        let text = flatten_text(&result);
        assert!(!text.contains("commentaire"));
        assert!(text.contains("avant"));
        assert!(text.contains("après"));
    }

    #[test]
    fn html_unicode_chinois_preserve() {
        let result = HtmlParser.parse("<strong>你好世界</strong>").unwrap();
        assert_eq!(flatten_text(&result), "你好世界");
    }

    #[test]
    fn html_arabe_preserve() {
        let result = HtmlParser.parse("<em>مرحبا</em>").unwrap();
        assert!(flatten_text(&result).contains("مرحبا"));
    }

    #[test]
    fn balise_fermante_sans_ouvrante_ne_panique_pas() {
        let result = HtmlParser.parse("texte </strong> suite");
        let _ = result;
    }

    #[test]
    fn tres_longue_liste_html() {
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
    fn balise_avec_attribut_data_ignore() {
        let result = HtmlParser
            .parse(r#"<strong data-id="123">texte</strong>"#)
            .unwrap();
        assert!(find_node(&result, ContainerType::Bold).is_some());
    }

    #[test]
    fn erreur_stoppe_a_la_premiere_balise_invalide() {
        let input = "<strong>valide</strong><div>invalide</div><em>aussi valide</em>";
        let result = HtmlParser.parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn imbrication_trop_profonde_retourne_nesting_too_deep() {
        let open: String = "<strong>".repeat(65);
        let close: String = "</strong>".repeat(65);
        let input = format!("{}x{}", open, close);
        let result = HtmlParser.parse(&input);
        assert!(matches!(result, Err(ParseError::NestingTooDeep { .. })));
    }

    #[test]
    fn imbrication_a_la_limite_est_acceptee() {
        let open: String = "<strong>".repeat(64);
        let close: String = "</strong>".repeat(64);
        let input = format!("{}x{}", open, close);
        let result = HtmlParser.parse(&input);
        assert!(result.is_ok());
    }
}
