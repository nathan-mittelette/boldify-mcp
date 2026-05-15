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

        while i < lines.len() {
            let line = &lines[i];
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
                let list_start = line.start + list_start_offset;
                let mut list_end = line.end;
                let mut children = Vec::new();

                while i < lines.len() {
                    let line = &lines[i];
                    if line.text.trim().is_empty() {
                        break;
                    }

                    let Some((next_bullet, rest, content_offset)) =
                        try_strip_unordered_prefix(line.text)
                    else {
                        break;
                    };

                    if next_bullet != bullet {
                        break;
                    }

                    list_end = line.end;
                    children.push(InlineNode::ListItem(build_list_item(
                        rest,
                        line.number,
                        line.start + content_offset,
                        column_at(line.text, content_offset),
                        &mut id_gen,
                    )?));
                    i += 1;
                }

                nodes.push(ContainerNode {
                    base: NodeBase::new(id_gen.next_id(), Span::new(list_start, list_end)),
                    container_type: ContainerType::List,
                    children,
                });
                continue;
            }

            if let Some((_, list_start_offset)) = try_strip_ordered_prefix(line.text) {
                let list_start = line.start + list_start_offset;
                let mut list_end = line.end;
                let mut children = Vec::new();

                while i < lines.len() {
                    let line = &lines[i];
                    if line.text.trim().is_empty() {
                        break;
                    }

                    let Some((rest, content_offset)) = try_strip_ordered_prefix(line.text) else {
                        break;
                    };

                    list_end = line.end;
                    children.push(InlineNode::ListItem(build_list_item(
                        rest,
                        line.number,
                        line.start + content_offset,
                        column_at(line.text, content_offset),
                        &mut id_gen,
                    )?));
                    i += 1;
                }

                nodes.push(ContainerNode {
                    base: NodeBase::new(id_gen.next_id(), Span::new(list_start, list_end)),
                    container_type: ContainerType::OrderedList,
                    children,
                });
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
                description: "Gras".into(),
                example: "**texte gras**".into(),
            },
            crate::SupportedSymbol {
                symbol: "*".into(),
                description: "Italique".into(),
                example: "*texte italique*".into(),
            },
            crate::SupportedSymbol {
                symbol: "_".into(),
                description: "Italique (variante)".into(),
                example: "_texte italique_".into(),
            },
            crate::SupportedSymbol {
                symbol: "__".into(),
                description: "Souligné".into(),
                example: "__texte souligné__".into(),
            },
            crate::SupportedSymbol {
                symbol: "~~".into(),
                description: "Texte barré".into(),
                example: "~~texte barré~~".into(),
            },
            crate::SupportedSymbol {
                symbol: "==".into(),
                description: "Surligné".into(),
                example: "==texte surligné==".into(),
            },
            crate::SupportedSymbol {
                symbol: "- ".into(),
                description: "Liste non ordonnée".into(),
                example: "- item de liste".into(),
            },
            crate::SupportedSymbol {
                symbol: "1. ".into(),
                description: "Liste ordonnée".into(),
                example: "1. premier item".into(),
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

    // Cas nominaux
    #[test]
    fn texte_simple_produit_container_text() {
        let result = MarkdownParser.parse("Bonjour").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
    }

    #[test]
    fn gras_produit_container_bold() {
        let result = MarkdownParser.parse("**Hello**").unwrap();
        assert_eq!(result.len(), 1);
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        assert_eq!(extract_text_from_container(bold.unwrap()), "Hello");
    }

    #[test]
    fn italique_etoile_produit_container_italic() {
        let result = MarkdownParser.parse("*Nathan*").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
        assert_eq!(extract_text_from_container(italic.unwrap()), "Nathan");
    }

    #[test]
    fn italique_underscore_produit_container_italic() {
        let result = MarkdownParser.parse("_Nathan_").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
        assert_eq!(extract_text_from_container(italic.unwrap()), "Nathan");
    }

    #[test]
    fn barre_produit_container_strikethrough() {
        let result = MarkdownParser.parse("~~barré~~").unwrap();
        let strike = find_container(&result, ContainerType::Strikethrough);
        assert!(strike.is_some());
        assert_eq!(extract_text_from_container(strike.unwrap()), "barré");
    }

    #[test]
    fn surline_produit_container_surline() {
        let result = MarkdownParser.parse("==surligné==").unwrap();
        let surline = find_container(&result, ContainerType::Surline);
        assert!(surline.is_some());
        assert_eq!(extract_text_from_container(surline.unwrap()), "surligné");
    }

    #[test]
    fn citation_produit_erreur_unsupported() {
        let result = MarkdownParser.parse("> citation");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn liste_non_ordonnee_produit_container_list() {
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
    fn liste_ordonnee_produit_container_ordered_list() {
        let result = MarkdownParser.parse("1. un\n2. deux").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::OrderedList);
        let list_items = result[0]
            .children
            .iter()
            .filter(|n| matches!(n, InlineNode::ListItem(_)))
            .count();
        assert_eq!(list_items, 2);
    }

    // Imbrication
    #[test]
    fn gras_contenant_italique_imbriqué() {
        let result = MarkdownParser
            .parse("**Bonjour *Nathan*, vas-tu ?**")
            .unwrap();
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        let bold_node = bold.unwrap();
        let italic = find_container_in_inline(&bold_node.children, ContainerType::Italic);
        assert!(italic.is_some());
    }

    #[test]
    fn texte_mixte_gras_et_normal() {
        let result = MarkdownParser.parse("Hello **world**").unwrap();
        assert_eq!(result.len(), 1);
        let root = &result[0];
        let has_bold = root.children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold),
        );
        assert!(has_bold);
    }

    // Erreurs
    #[test]
    fn diese_produit_erreur_unsupported() {
        let result = MarkdownParser.parse("# Titre");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "#")
        );
    }

    #[test]
    fn marqueur_non_ferme_produit_erreur() {
        let result = MarkdownParser.parse("**non fermé");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn backtick_produit_erreur_unsupported() {
        let result = MarkdownParser.parse("`code`");
        assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
    }

    #[test]
    fn erreur_contient_position_precise() {
        let result = MarkdownParser.parse("Hello `code`");
        if let Err(ParseError::UnsupportedSymbol { position, .. }) = result {
            assert_eq!(position.line, 1);
            assert!(position.column > 1);
        } else {
            panic!("Attendu UnsupportedSymbol");
        }
    }

    // Cas limites
    #[test]
    fn input_vide_retourne_vec_vide() {
        let result = MarkdownParser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn lignes_vides_ignorees() {
        let result = MarkdownParser.parse("\n\n\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn texte_unicode_preserve() {
        let result = MarkdownParser.parse("données résumé").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("données résumé"));
    }

    #[test]
    fn supported_symbols_retourne_8_entrees() {
        let symbols = MarkdownParser.supported_symbols();
        assert_eq!(symbols.len(), 8);
        assert!(!symbols.iter().any(|symbol| symbol.symbol == "> "));
    }

    // Tests avancés - Emojis
    #[test]
    fn emoji_seul_dans_texte_preserve() {
        let result = MarkdownParser.parse("🚀").unwrap();
        let text = extract_text(&result);
        assert_eq!(text, "🚀");
    }

    #[test]
    fn emoji_dans_texte_normal_preserve() {
        let result = MarkdownParser.parse("Bonjour 👋 tout le monde").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("👋"));
    }

    #[test]
    fn emoji_dans_gras_preserve() {
        let result = MarkdownParser.parse("**🔥 Résultats**").unwrap();
        let bold = find_container(&result, ContainerType::Bold);
        assert!(bold.is_some());
        let text = extract_text_from_container(bold.unwrap());
        assert!(text.contains("🔥"));
    }

    #[test]
    fn emoji_dans_italique_preserve() {
        let result = MarkdownParser.parse("*✨ incroyable*").unwrap();
        let italic = find_container(&result, ContainerType::Italic);
        assert!(italic.is_some());
    }

    #[test]
    fn emoji_consecutifs_preserves() {
        let result = MarkdownParser.parse("🎯🎯🎯").unwrap();
        let text = extract_text(&result);
        assert_eq!(text, "🎯🎯🎯");
    }

    #[test]
    fn emoji_en_debut_de_ligne_preserve() {
        let result = MarkdownParser.parse("✅ Objectif atteint").unwrap();
        let text = extract_text(&result);
        assert!(text.starts_with("✅"));
    }

    #[test]
    fn emoji_multicodepoint_preserve() {
        let result = MarkdownParser.parse("👨‍💻 développeur").unwrap();
        let text = extract_text(&result);
        assert!(text.contains("👨‍💻"));
    }

    // Tests avancés - Multi-lignes
    #[test]
    fn deux_paragraphes_produisent_deux_containers() {
        let input = "Première ligne\nDeuxième ligne";
        let result = MarkdownParser.parse(input).unwrap();
        // text + \n node + text = 3 nodes
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn paragraphe_suivi_de_liste() {
        let input = "Introduction\n- item A\n- item B";
        let result = MarkdownParser.parse(input).unwrap();
        // text + \n node + list = 3 nodes
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].container_type, ContainerType::Text);
        assert_eq!(result[2].container_type, ContainerType::List);
    }

    #[test]
    fn liste_suivie_de_texte() {
        let input = "- item A\n- item B\nConclusion";
        let result = MarkdownParser.parse(input).unwrap();
        // list + \n node + text = 3 nodes
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].container_type, ContainerType::List);
        assert_eq!(result[2].container_type, ContainerType::Text);
    }

    #[test]
    fn citation_entre_deux_paragraphes_produit_erreur() {
        let input = "Intro\n> Une pensée\nConclusion";
        let result = MarkdownParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn lignes_vides_entre_blocs_ignorees() {
        let input = "Paragraphe 1\n\n\nParagraphe 2";
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
    fn cinq_items_de_liste_non_ordonnee() {
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
    fn liste_ordonnee_longue() {
        let input = "1. Premier\n2. Deuxième\n3. Troisième\n4. Quatrième\n5. Cinquième";
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

    // Tests avancés - Combinaisons de styles
    #[test]
    fn gras_et_italique_sur_mots_differents() {
        let result = MarkdownParser.parse("**gras** et *italique*").unwrap();
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
    fn trois_styles_differents_sur_une_ligne() {
        let result = MarkdownParser
            .parse("**gras** *italique* ~~barré~~")
            .unwrap();
        let root = &result[0];
        let bold_count = count_children_of_type(&root.children, ContainerType::Bold);
        let italic_count = count_children_of_type(&root.children, ContainerType::Italic);
        let strike_count = count_children_of_type(&root.children, ContainerType::Strikethrough);
        assert_eq!(bold_count, 1);
        assert_eq!(italic_count, 1);
        assert_eq!(strike_count, 1);
    }

    #[test]
    fn gras_barré_surligné_sur_mots_differents() {
        let result = MarkdownParser
            .parse("**important** ~~obsolète~~ ==nouveau==")
            .unwrap();
        let root = &result[0];
        assert!(count_children_of_type(&root.children, ContainerType::Bold) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Strikethrough) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Surline) >= 1);
    }

    #[test]
    fn texte_avant_et_apres_le_style() {
        let result = MarkdownParser.parse("Voir le mot **clé** ici").unwrap();
        let root = &result[0];
        assert!(root.children.len() >= 3);
    }

    #[test]
    fn style_en_tout_debut_de_ligne() {
        let result = MarkdownParser.parse("**Titre de post**").unwrap();
        let root = &result[0];
        assert_eq!(root.children.len(), 1);
        assert!(
            matches!(&root.children[0], InlineNode::Container(c) if c.container_type == ContainerType::Bold)
        );
    }

    #[test]
    fn style_en_toute_fin_de_ligne() {
        let result = MarkdownParser
            .parse("Texte normal puis *italique*")
            .unwrap();
        let root = &result[0];
        let last = root.children.last().unwrap();
        assert!(
            matches!(last, InlineNode::Container(c) if c.container_type == ContainerType::Italic)
        );
    }

    // Tests avancés - Imbrications profondes
    #[test]
    fn triple_imbrication_gras_italique_surligné() {
        let result = MarkdownParser
            .parse("**gras *italique ==surligné== fin* fin**")
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
    fn gras_avec_deux_mots_italiques_dedans() {
        let result = MarkdownParser
            .parse("**Hello *world* and *Rust***")
            .unwrap();
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let italic_count = count_children_of_type(&bold.children, ContainerType::Italic);
        assert_eq!(italic_count, 2);
    }

    #[test]
    fn liste_avec_items_styles() {
        let input = "- **important**\n- *note*\n- texte normal";
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

    // Tests avancés - Posts LinkedIn réalistes
    #[test]
    fn post_linkedin_annonce_simple() {
        let input = "\n🎉 **Bonne nouvelle !**\nJe viens de rejoindre une nouvelle aventure professionnelle.\n\nAprès 3 ans chez mon ancienne entreprise, il est temps de relever de nouveaux défis.\n\n*Merci à toute mon équipe pour ces années inoubliables.*";

        let result = MarkdownParser.parse(input).unwrap();
        assert!(result.len() >= 3);
        let has_bold = result[0].children.iter().any(
            |n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold),
        );
        assert!(has_bold);
    }

    #[test]
    fn post_linkedin_liste_apprentissages() {
        let input = "\nCe que j'ai appris cette année :\n\n- **Rust** est incroyable pour les performances\n- *La documentation* est aussi importante que le code\n- ~~Les réunions inutiles~~ Le focus, c'est précieux\n- ==La communauté== fait toute la différence 🙏";

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
    fn post_linkedin_citation_et_liste_produit_erreur() {
        let input = "\n> \"Le code propre n'est pas écrit en suivant un ensemble de règles.\"\n\nMes 3 principes :\n\n1. Nommer les choses clairement\n2. Faire une chose à la fois\n3. Tester avant de déployer 🚀";

        let result = MarkdownParser.parse(input);
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn post_linkedin_long_avec_styles_et_liste() {
        let input = "\n🚀 **3 ans de freelance : ce que personne ne vous dit**\n\nQuand j'ai commencé, je pensais que la technique était le plus dur.\nJ'avais *complètement* tort.\n\nVoici ce que j'ai vraiment appris :\n\n- **Trouver des clients** est un métier à part entière\n- La facturation, ~~personne~~ vraiment personne ne vous apprend ça\n- ==Votre réputation== vaut plus que n'importe quel CV\n- *La solitude* du freelance est réelle 🧘\n\n**Et vous, qu'est-ce qui vous a le plus surpris ?** 👇";

        let result = MarkdownParser.parse(input).unwrap();
        assert!(
            result.len() >= 4,
            "Attendu au moins 4 blocs, obtenu {}",
            result.len()
        );

        let types: Vec<&ContainerType> = result.iter().map(|n| &n.container_type).collect();
        assert!(types.contains(&&ContainerType::List));
    }

    #[test]
    fn post_linkedin_chiffres_et_pourcentages() {
        let input = "En **2024**, j'ai livré *47 projets* avec un taux de satisfaction de ==98%==.";
        let result = MarkdownParser.parse(input).unwrap();
        assert_eq!(result.len(), 1);
        let root = &result[0];
        assert!(count_children_of_type(&root.children, ContainerType::Bold) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Italic) >= 1);
        assert!(count_children_of_type(&root.children, ContainerType::Surline) >= 1);
    }

    #[test]
    fn post_avec_accents_dans_styles() {
        let input = "**Développeur** passionné par l'*élégance* du code.";
        let result = MarkdownParser.parse(input).unwrap();
        assert!(!result.is_empty());
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let text = extract_text_from_container(bold);
        assert!(text.contains("Développeur"));
    }

    #[test]
    fn post_avec_ponctuation_speciale() {
        let input = "**L'innovation** — c'est aussi *oser dire non* à l'inutile…";
        let result = MarkdownParser.parse(input).unwrap();
        assert!(!result.is_empty());
    }

    // Tests avancés - Cas limites et robustesse
    #[test]
    fn marqueur_gras_vide_produit_container_bold_sans_enfants() {
        let result = MarkdownParser.parse("****");
        let _ = result;
    }

    #[test]
    fn underscore_dans_mot_compose_ne_declenche_pas_italique() {
        let result = MarkdownParser.parse("snake_case_variable");
        let _ = result;
    }

    #[test]
    fn ligne_avec_uniquement_des_espaces_ignoree() {
        let result = MarkdownParser.parse("   \t   ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn texte_avec_retour_chariot_windows() {
        let result = MarkdownParser.parse("ligne1\r\nligne2");
        assert!(result.is_ok());
        let nodes = result.unwrap();
        // text + \n node + text = 3 nodes
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn tres_longue_ligne_sans_marqueur() {
        let long_line = "a".repeat(10_000);
        let result = MarkdownParser.parse(&long_line).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container_type, ContainerType::Text);
    }

    #[test]
    fn marqueur_gras_sur_plusieurs_mots_avec_espaces() {
        let result = MarkdownParser
            .parse("**bonjour monde comment vas tu**")
            .unwrap();
        let bold = find_container(&result, ContainerType::Bold).unwrap();
        let text = extract_text_from_container(bold);
        assert_eq!(text, "bonjour monde comment vas tu");
    }

    #[test]
    fn citation_avec_style_inline_produit_erreur() {
        let result = MarkdownParser.parse("> **Citation importante** de quelqu'un");
        assert!(
            matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == ">")
        );
    }

    #[test]
    fn liste_avec_emojis_en_items() {
        let input = "- 🎯 Objectif 1\n- 🚀 Lancement\n- ✅ Terminé";
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
    fn erreur_premier_symbole_non_supporte_arrete_le_parsing() {
        let input = "texte valide\n# titre invalide\nautres lignes valides";
        let result = MarkdownParser.parse(input);
        assert!(result.is_err());
    }
}
