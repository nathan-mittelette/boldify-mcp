# Stratégie de Tests

## Principes généraux

Les tests suivent la même architecture en couches que le code. Chaque crate est testé de façon autonome, sans dépendance sur les couches supérieures. Les tests d'intégration traversent plusieurs couches mais restent dans le workspace Rust — pas de réseau réel.

| Niveau              | Portée                              | Outil principal        | Localisation                        |
|---------------------|-------------------------------------|------------------------|-------------------------------------|
| Unitaire (parser)   | `MarkdownParser`, `HtmlParser`      | `#[test]` natif        | `parser/src/markdown.rs`, etc.      |
| Unitaire (converter)| `ToUnicode` par type de nœud        | `#[test]` natif        | `converter/src/nodes/*.rs`          |
| Unitaire (service)  | `ContentService`, `ServiceError`    | `#[test]` natif        | `service/src/lib.rs`                |
| Intégration         | Parser → Converter → Service        | `#[test]` natif        | `service/tests/integration.rs`      |
| API (simulation)    | Handlers HTTP sans réseau           | `lambda_http::mock`    | `api/tests/`                        |

---

## Tests unitaires — crate `parser`

```rust
// parser/src/markdown.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, ast::TextNode};

    fn parse(input: &str) -> Vec<TextNode> {
        MarkdownParser.parse(input).expect("Parsing réussi")
    }

    // --- Titres ---

    #[test]
    fn titre_niveau_1() {
        let nodes = parse("# Hello");
        assert_eq!(nodes.len(), 1);
        if let TextNode::Heading(h) = &nodes[0] {
            assert_eq!(h.level, 1);
            assert_eq!(h.content, "Hello");
        } else {
            panic!("Attendu un Heading");
        }
    }

    #[test]
    fn titre_sans_contenu() {
        let nodes = parse("## ");
        assert_eq!(nodes.len(), 1);
        if let TextNode::Heading(h) = &nodes[0] {
            assert_eq!(h.level, 2);
            assert_eq!(h.content, "");
        } else {
            panic!("Attendu un Heading vide");
        }
    }

    #[test]
    fn titre_niveau_6_maximum() {
        let nodes = parse("###### Profond");
        if let TextNode::Heading(h) = &nodes[0] {
            assert_eq!(h.level, 6);
        } else {
            panic!("Attendu un Heading");
        }
    }

    #[test]
    fn titre_niveau_7_traite_comme_paragraphe() {
        let nodes = parse("####### Trop profond");
        // Level > 6 : traité comme paragraphe par le parser
        assert!(matches!(nodes[0], TextNode::Paragraph(_)));
    }

    // --- Listes ---

    #[test]
    fn liste_non_ordonnee() {
        let nodes = parse("- item A");
        if let TextNode::List(l) = &nodes[0] {
            assert!(!l.ordered);
            assert_eq!(l.items, vec!["item A"]);
        } else {
            panic!("Attendu une List");
        }
    }

    #[test]
    fn liste_ordonnee() {
        let nodes = parse("1. premier");
        if let TextNode::List(l) = &nodes[0] {
            assert!(l.ordered);
            assert_eq!(l.items, vec!["premier"]);
        } else {
            panic!("Attendu une List ordonnée");
        }
    }

    #[test]
    fn liste_item_vide() {
        let nodes = parse("- ");
        if let TextNode::List(l) = &nodes[0] {
            assert_eq!(l.items, vec![""]);
        } else {
            panic!("Attendu une List");
        }
    }

    // --- Lignes vides ---

    #[test]
    fn ligne_vide_ignoree() {
        let nodes = parse("");
        assert!(nodes.is_empty());
    }

    #[test]
    fn plusieurs_lignes_vides_ignorees() {
        let nodes = parse("\n\n\n");
        assert!(nodes.is_empty());
    }

    // --- Paragraphes ---

    #[test]
    fn paragraphe_simple() {
        let nodes = parse("Du texte simple.");
        if let TextNode::Paragraph(p) = &nodes[0] {
            assert_eq!(p.content, "Du texte simple.");
        } else {
            panic!("Attendu un Paragraph");
        }
    }

    #[test]
    fn paragraphe_unicode() {
        let nodes = parse("Données 数据 résumé");
        if let TextNode::Paragraph(p) = &nodes[0] {
            assert_eq!(p.content, "Données 数据 résumé");
        } else {
            panic!("Attendu un Paragraph Unicode");
        }
    }

    // --- Blocs spéciaux ---

    #[test]
    fn citation() {
        let nodes = parse("> Une citation.");
        if let TextNode::Blockquote(b) = &nodes[0] {
            assert_eq!(b.content, "Une citation.");
        } else {
            panic!("Attendu un Blockquote");
        }
    }

    #[test]
    fn separateur_horizontal() {
        let nodes = parse("---");
        assert!(matches!(nodes[0], TextNode::HorizontalRule(_)));
    }

    #[test]
    fn bloc_de_code_indente() {
        let nodes = parse("    fn main() {}");
        if let TextNode::CodeBlock(c) = &nodes[0] {
            assert_eq!(c.content, "fn main() {}");
            assert!(c.language.is_none());
        } else {
            panic!("Attendu un CodeBlock");
        }
    }

    // --- Span ---

    #[test]
    fn span_pointe_vers_la_source() {
        let input = "# Titre";
        let nodes = MarkdownParser.parse(input).unwrap();
        let span = &nodes[0].base().span;
        assert_eq!(&input[span.start..span.end.min(input.len())], "# Titre");
    }

    // --- Limite de taille ---

    #[test]
    fn entree_trop_grande_retourne_erreur() {
        use crate::ParseError;
        let big = "a".repeat(11 * 1024 * 1024);
        let result = MarkdownParser.parse(&big);
        assert!(matches!(result, Err(ParseError::InputTooLarge { .. })));
    }

    // --- Nœuds multiples ---

    #[test]
    fn document_mixte() {
        let input = "# Titre\n\nParagraphe.\n\n- item";
        let nodes = parse(input);
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], TextNode::Heading(_)));
        assert!(matches!(nodes[1], TextNode::Paragraph(_)));
        assert!(matches!(nodes[2], TextNode::List(_)));
    }
}
```

---

## Tests unitaires — crate `converter`

```rust
// converter/src/nodes/heading.rs

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ast::{HeadingNode, NodeBase, Span};
    use crate::{font::UnicodeFont, traits::ToUnicode};

    fn heading(level: u8, content: &str) -> HeadingNode {
        HeadingNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            level,
            content: content.to_string(),
        }
    }

    #[test]
    fn titre_bold_produit_symboles_unicode() {
        let h = heading(1, "ABC");
        let result = h.to_unicode(UnicodeFont::Bold);
        // Doit contenir des caractères mathématiques gras, pas "ABC" ASCII
        assert!(!result.contains("ABC"));
        assert!(result.starts_with('━'));
    }

    #[test]
    fn titre_plain_preserve_ascii() {
        let h = heading(2, "Hello");
        let result = h.to_unicode(UnicodeFont::Plain);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn titre_vide_ne_plante_pas() {
        let h = heading(1, "");
        let result = h.to_unicode(UnicodeFont::Bold);
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn niveau_prefixe_correspond_au_level() {
        for level in 1u8..=6 {
            let h = heading(level, "X");
            let result = h.to_unicode(UnicodeFont::Plain);
            let prefix_count = result.chars().take_while(|&c| c == '━').count();
            assert_eq!(prefix_count as u8, level);
        }
    }
}
```

```rust
// converter/src/nodes/list.rs

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ast::{ListNode, NodeBase, Span};
    use crate::{font::UnicodeFont, traits::ToUnicode};

    fn list(items: &[&str], ordered: bool) -> ListNode {
        ListNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            items: items.iter().map(|s| s.to_string()).collect(),
            ordered,
        }
    }

    #[test]
    fn liste_non_ordonnee_utilise_puce() {
        let l = list(&["alpha", "beta"], false);
        let result = l.to_unicode(UnicodeFont::Plain);
        assert!(result.contains("• alpha"));
        assert!(result.contains("• beta"));
    }

    #[test]
    fn liste_ordonnee_numerote_correctement() {
        let l = list(&["un", "deux", "trois"], true);
        let result = l.to_unicode(UnicodeFont::Plain);
        assert!(result.contains("1. un"));
        assert!(result.contains("2. deux"));
        assert!(result.contains("3. trois"));
    }

    #[test]
    fn liste_vide_produit_chaine_vide() {
        let l = list(&[], false);
        let result = l.to_unicode(UnicodeFont::Plain);
        assert!(result.is_empty());
    }
}
```

---

## Tests unitaires — crate `service`

```rust
// service/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> ContentService {
        ContentService::new()
    }

    #[test]
    fn conversion_markdown_retourne_ok() {
        let result = svc().convert("markdown", "# Titre", "plain");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Titre"));
    }

    #[test]
    fn conversion_html_retourne_ok() {
        let result = svc().convert("html", "<h1>Bonjour</h1>", "plain");
        assert!(result.is_ok());
    }

    #[test]
    fn syntaxe_inconnue_retourne_erreur() {
        let result = svc().convert("xml", "contenu", "bold");
        assert!(matches!(result, Err(ServiceError::UnsupportedSyntax(_, _))));
    }

    #[test]
    fn contenu_vide_retourne_chaine_vide() {
        let result = svc().convert("markdown", "", "bold");
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn contenu_espaces_seuls_retourne_chaine_vide() {
        let result = svc().convert("markdown", "   \n  ", "bold");
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn font_inconnu_retourne_erreur() {
        let result = svc().convert("markdown", "# Titre", "cursive");
        assert!(matches!(result, Err(ServiceError::UnknownFont(_))));
    }

    #[test]
    fn list_syntaxes_retourne_markdown_et_html() {
        let syntaxes = ContentService::list_syntaxes();
        assert!(syntaxes.contains(&"markdown"));
        assert!(syntaxes.contains(&"html"));
    }

    #[test]
    fn alias_md_accepte() {
        let result = svc().convert("md", "# Titre", "plain");
        assert!(result.is_ok());
    }
}
```

---

## Tests d'intégration — `service/tests/integration.rs`

```rust
// service/tests/integration.rs

use service::ContentService;

/// Vérifie qu'un pipeline complet (parse + convert) produit une sortie cohérente.
#[test]
fn pipeline_markdown_vers_unicode_bold() {
    let service = ContentService::new();
    let input = "# Bienvenue\n\nCeci est un test.\n\n- item A\n- item B";
    let output = service.convert("markdown", input, "bold").unwrap();

    // La sortie doit contenir des caractères Unicode (pas l'ASCII original)
    assert!(!output.contains("Bienvenue"), "Le contenu ASCII ne doit pas apparaître en mode bold");
    // Elle doit contenir des sauts de ligne
    assert!(output.contains('\n'));
}

#[test]
fn pipeline_markdown_vers_plain_preserve_contenu() {
    let service = ContentService::new();
    let input = "# Titre\n\nParagraphe normal.";
    let output = service.convert("markdown", input, "plain").unwrap();

    assert!(output.contains("Titre"));
    assert!(output.contains("Paragraphe normal."));
}

#[test]
fn pipeline_html_vers_unicode() {
    let service = ContentService::new();
    let input = "<h2>Section</h2><p>Contenu de la section.</p>";
    let output = service.convert("html", input, "plain").unwrap();

    assert!(output.contains("Section"));
    assert!(output.contains("Contenu de la section."));
}

#[test]
fn plusieurs_styles_produisent_des_sorties_differentes() {
    let service = ContentService::new();
    let input = "# Test";

    let bold      = service.convert("markdown", input, "bold").unwrap();
    let italic    = service.convert("markdown", input, "italic").unwrap();
    let monospace = service.convert("markdown", input, "monospace").unwrap();

    // Les styles Unicode produisent des codepoints différents
    assert_ne!(bold, italic);
    assert_ne!(bold, monospace);
    assert_ne!(italic, monospace);
}

#[test]
fn bloc_de_code_utilise_toujours_monospace() {
    let service = ContentService::new();
    let input = "    fn main() {}";
    // Même avec font=plain, les blocs de code sont en monospace
    let output_bold  = service.convert("markdown", input, "bold").unwrap();
    let output_plain = service.convert("markdown", input, "plain").unwrap();

    // Le contenu brut ASCII ne doit pas apparaître avec bold (monospace s'applique)
    // Les deux sorties sont différentes si le renderer fait son travail
    assert_ne!(output_bold, output_plain);
}
```

---

## Edge cases couverts — récapitulatif

| Couche      | Edge case                                      | Test correspondant                                     |
|-------------|------------------------------------------------|--------------------------------------------------------|
| Parser      | Titre sans contenu (`## `)                     | `titre_sans_contenu`                                   |
| Parser      | Titre de niveau > 6                            | `titre_niveau_7_traite_comme_paragraphe`               |
| Parser      | Entrée vide                                    | `ligne_vide_ignoree`                                   |
| Parser      | Entrée > 10 Mo                                 | `entree_trop_grande_retourne_erreur`                   |
| Parser      | Caractères Unicode dans le contenu             | `paragraphe_unicode`                                   |
| Parser      | Span pointe vers la source                     | `span_pointe_vers_la_source`                           |
| Parser      | Document avec plusieurs types de nœuds         | `document_mixte`                                       |
| Converter   | Titre vide sans panique                        | `titre_vide_ne_plante_pas`                             |
| Converter   | Niveau de titre dans le préfixe                | `niveau_prefixe_correspond_au_level`                   |
| Converter   | Liste vide                                     | `liste_vide_produit_chaine_vide`                       |
| Converter   | Numérotation liste ordonnée                    | `liste_ordonnee_numerote_correctement`                 |
| Service     | Syntaxe inconnue                               | `syntaxe_inconnue_retourne_erreur`                     |
| Service     | Contenu vide ou espaces                        | `contenu_vide_retourne_chaine_vide`, `contenu_espaces_seuls_retourne_chaine_vide` |
| Service     | Style inconnu                                  | `font_inconnu_retourne_erreur`                         |
| Service     | Alias `md` pour Markdown                       | `alias_md_accepte`                                     |
| Intégration | Les styles produisent des sorties différentes  | `plusieurs_styles_produisent_des_sorties_differentes`  |
| Intégration | Plain préserve le contenu ASCII                | `pipeline_markdown_vers_plain_preserve_contenu`        |
| Intégration | HTML pipeline complet                          | `pipeline_html_vers_unicode`                           |

---

## Commandes de test

```bash
# Tous les tests du workspace
cargo test --workspace

# Tests d'un crate spécifique
cargo test --package parser
cargo test --package converter
cargo test --package service

# Tests avec affichage des println! (utile pour déboguer les sorties Unicode)
cargo test --package service -- --nocapture

# Tests d'intégration uniquement
cargo test --package service --test integration

# Vérification des types sans exécution
cargo check --workspace
```
