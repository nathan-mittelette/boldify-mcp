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

    // ── Tâche 05 — tests de base ────────────────────────────────────────────

    #[test]
    fn text_retourne_le_texte_brut() {
        let node = make_container(ContainerType::Text, vec![make_text("Bonjour")]);
        assert_eq!(node.to_unicode(), "Bonjour");
    }

    #[test]
    fn bold_convertit_ascii_en_unicode_gras() {
        let node = make_container(ContainerType::Bold, vec![make_text("ABC")]);
        let result = node.to_unicode();
        assert!(!result.contains("ABC"));
        assert_eq!(result.chars().count(), 3);
    }

    #[test]
    fn bold_preserve_espace() {
        let node = make_container(ContainerType::Bold, vec![make_text("A B")]);
        assert!(node.to_unicode().contains(' '));
    }

    #[test]
    fn bold_convertit_accent() {
        let node = make_container(ContainerType::Bold, vec![make_text("é")]);
        let result = node.to_unicode();
        assert!(result.contains('\u{0301}'));
        assert!(!result.contains('é'));
    }

    #[test]
    fn italic_convertit_ascii_en_unicode_italique() {
        let node = make_container(ContainerType::Italic, vec![make_text("Hello")]);
        assert!(!node.to_unicode().contains("Hello"));
    }

    #[test]
    fn blockquote_encadre_le_texte() {
        let node = make_container(ContainerType::Blockquote, vec![make_text("citation")]);
        let result = node.to_unicode();
        assert!(result.contains("citation"));
        assert!(result.contains('❝'));
        assert!(result.contains('❞'));
    }

    #[test]
    fn list_prefixe_chaque_item_avec_puce() {
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
    fn ordered_list_numerote_les_items() {
        let li = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![make_text("premier")],
        });
        let node = make_container(ContainerType::OrderedList, vec![li]);
        assert!(node.to_unicode().contains("1. premier"));
    }

    #[test]
    fn bold_contenant_italic_applique_les_deux_styles() {
        let italic_child = InlineNode::Container(make_container(
            ContainerType::Italic,
            vec![make_text("Nathan")],
        ));
        let bold_node = make_container(ContainerType::Bold, vec![italic_child]);
        assert!(!bold_node.to_unicode().contains("Nathan"));
    }

    // ── Tâche 05b — tests avancés ───────────────────────────────────────────

    #[test]
    fn bold_avec_emoji_dans_le_texte() {
        let node = make_container(ContainerType::Bold, vec![make_text("🔥 Résultats")]);
        let result = node.to_unicode();
        assert!(result.contains("🔥"));
        assert!(!result.contains('R'));
        assert!(!result.contains('é'));
    }

    #[test]
    fn bold_avec_ponctuation_et_chiffres() {
        let node = make_container(ContainerType::Bold, vec![make_text("Top 3 !")]);
        let result = node.to_unicode();
        assert!(result.contains(' '));
        assert!(result.contains('!'));
        assert!(!result.contains('T'));
    }

    #[test]
    fn text_avec_emoji_retourne_emoji_intact() {
        let node = make_container(ContainerType::Text, vec![make_text("Hello 👋")]);
        assert_eq!(node.to_unicode(), "Hello 👋");
    }

    #[test]
    fn italic_avec_accents_transforme_les_bases() {
        let node = make_container(ContainerType::Italic, vec![make_text("élégance")]);
        let result = node.to_unicode();
        assert!(!result.contains('é'));
        assert!(!result.contains('e'));
    }

    #[test]
    fn strikethrough_avec_plusieurs_mots() {
        let node = make_container(
            ContainerType::Strikethrough,
            vec![make_text("ancien contenu")],
        );
        let result = node.to_unicode();
        assert!(result.contains('\u{0336}'));
        assert!(result.contains(' '));
    }

    #[test]
    fn underline_avec_chiffres() {
        let node = make_container(ContainerType::Underline, vec![make_text("2024")]);
        assert!(node.to_unicode().contains('\u{0332}'));
    }

    #[test]
    fn blockquote_avec_emoji_dans_citation() {
        let node = make_container(
            ContainerType::Blockquote,
            vec![make_text("Sois le changement 🌱")],
        );
        let result = node.to_unicode();
        assert!(result.contains('❝'));
        assert!(result.contains("🌱"));
    }

    #[test]
    fn list_avec_items_contenant_des_emojis() {
        let items: Vec<InlineNode> = ["🎯 Objectif", "🚀 Lancement", "✅ Terminé"]
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
        assert!(result.contains("• 🎯 Objectif"));
        assert!(result.contains("• 🚀 Lancement"));
        assert!(result.contains("• ✅ Terminé"));
    }

    #[test]
    fn ordered_list_avec_bold_dans_items() {
        let bold_child = InlineNode::Container(make_container(
            ContainerType::Bold,
            vec![make_text("Important")],
        ));
        let li = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![bold_child, make_text(" à retenir")],
        });
        let node = make_container(ContainerType::OrderedList, vec![li]);
        let result = node.to_unicode();
        assert!(result.contains("1."));
        assert!(!result.contains("Important"));
        assert!(result.contains(" à retenir"));
    }

    #[test]
    fn tous_les_styles_produisent_des_resultats_differents() {
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
                    "Les styles {} et {} produisent le même résultat pour \"{}\"",
                    i, j, text
                );
            }
        }
    }

    #[test]
    fn convert_noeud_text_avec_newline_interne() {
        let node = make_container(ContainerType::Bold, vec![make_text("ligne1\nligne2")]);
        let result = node.to_unicode();
        assert!(result.contains('\n'));
        assert!(!result.contains("ligne"));
    }

    #[test]
    fn convert_container_text_avec_inline_containers_multiples() {
        let node = make_container(
            ContainerType::Text,
            vec![
                make_text("Texte "),
                InlineNode::Container(make_container(ContainerType::Bold, vec![make_text("gras")])),
                make_text(" et "),
                InlineNode::Container(make_container(
                    ContainerType::Italic,
                    vec![make_text("italique")],
                )),
                make_text(" et "),
                InlineNode::Container(make_container(
                    ContainerType::Surline,
                    vec![make_text("surligné")],
                )),
            ],
        );
        let result = node.to_unicode();
        assert!(result.contains("Texte "));
        assert!(result.contains(" et "));
        assert!(!result.contains("gras"));
        assert!(!result.contains("italique"));
        assert!(result.contains('\u{0305}'));
    }
}
