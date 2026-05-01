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
pub fn convert(nodes: &[ContainerNode]) -> String {
    nodes
        .iter()
        .map(|n| n.to_unicode())
        .collect::<Vec<_>>()
        .join("\n")
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
    fn convert_plusieurs_nodes_joint_avec_newline() {
        let n1 = make_container(ContainerType::Text, vec![make_text("ligne 1")]);
        let n2 = make_container(ContainerType::Text, vec![make_text("ligne 2")]);
        let result = convert(&[n1, n2]);
        assert!(result.contains("ligne 1\nligne 2"));
    }

    #[test]
    fn convert_vec_vide_retourne_chaine_vide() {
        assert_eq!(convert(&[]), "");
    }

    // ── Tâche 05b ────────────────────────────────────────────────────────────

    #[test]
    fn style_sur_chaine_identique_retourne_toujours_le_meme_resultat() {
        let r1 = bold_handler().apply("Hello World");
        let r2 = bold_handler().apply("Hello World");
        assert_eq!(r1, r2);
    }

    #[test]
    fn bold_puis_italic_sur_meme_texte_pas_idem() {
        let bold = bold_handler().apply("test");
        let italic = italic_handler().apply("test");
        assert_ne!(bold, italic);
    }

    #[test]
    fn convert_post_linkedin_simule() {
        let nodes = vec![
            make_container(
                ContainerType::Text,
                vec![
                    make_text("🔥 "),
                    InlineNode::Container(make_container(
                        ContainerType::Bold,
                        vec![make_text("3 conseils pour progresser")],
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
                                vec![make_text("Lire")],
                            )),
                            make_text(" tous les jours"),
                        ],
                    }),
                    InlineNode::ListItem(ListItemNode {
                        base: NodeBase::new(2, Span::new(0, 0)),
                        children: vec![
                            make_text("Pratiquer "),
                            InlineNode::Container(make_container(
                                ContainerType::Italic,
                                vec![make_text("régulièrement")],
                            )),
                        ],
                    }),
                    InlineNode::ListItem(ListItemNode {
                        base: NodeBase::new(3, Span::new(0, 0)),
                        children: vec![
                            make_text("Partager ses "),
                            InlineNode::Container(make_container(
                                ContainerType::Strikethrough,
                                vec![make_text("erreurs")],
                            )),
                            make_text(" apprentissages"),
                        ],
                    }),
                ],
            ),
            make_container(
                ContainerType::Blockquote,
                vec![make_text(
                    "La progression constante bat la perfection occasionnelle.",
                )],
            ),
        ];

        let result = convert(&nodes);
        assert!(!result.is_empty());
        assert!(result.contains("🔥"));
        assert!(result.contains('\n'));
        assert!(result.contains("• "));
        assert!(result.contains('❝'));
        assert!(!result.contains("Lire"));
        assert!(result.contains(" tous les jours"));
    }
}
