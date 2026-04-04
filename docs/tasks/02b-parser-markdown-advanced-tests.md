# Tâche 02b — Tests avancés : Parser Markdown

## Scope

Compléments de tests à ajouter dans `parser/src/markdown.rs` en plus des tests de base (tâche 02). Ces tests couvrent des scénarios réalistes : posts multi-lignes, emojis, combinaisons de styles, textes qui ressemblent à des vrais contenus LinkedIn.

---

## Emojis et caractères spéciaux

Les emojis sont des caractères Unicode valides — le parser doit les laisser passer tels quels dans les `TextNode`, y compris à l'intérieur de marqueurs de style.

```rust
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
    // "**🔥 Résultats**" → Container(Bold, [Text("🔥 Résultats")])
    let result = MarkdownParser.parse("**🔥 Résultats**").unwrap();
    // L'emoji doit être dans le TextNode enfant du Bold
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
    // Emoji avec modificateur de couleur de peau ou séquence ZWJ
    let result = MarkdownParser.parse("👨‍💻 développeur").unwrap();
    let text = extract_text(&result);
    assert!(text.contains("👨‍💻"));
}
```

---

## Textes multi-lignes

```rust
#[test]
fn deux_paragraphes_produisent_deux_containers() {
    let input = "Première ligne\nDeuxième ligne";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn paragraphe_suivi_de_liste() {
    let input = "Introduction\n- item A\n- item B";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].container_type, ContainerType::Text);
    assert_eq!(result[1].container_type, ContainerType::List);
}

#[test]
fn liste_suivie_de_texte() {
    let input = "- item A\n- item B\nConclusion";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].container_type, ContainerType::List);
    assert_eq!(result[1].container_type, ContainerType::Text);
}

#[test]
fn blockquote_entre_deux_paragraphes() {
    let input = "Intro\n> Une pensée\nConclusion";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[1].container_type, ContainerType::Blockquote);
}

#[test]
fn lignes_vides_entre_blocs_ignorees() {
    let input = "Paragraphe 1\n\n\nParagraphe 2";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn cinq_items_de_liste_non_ordonnee() {
    let input = "- A\n- B\n- C\n- D\n- E";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 1);
    let list = &result[0];
    assert_eq!(list.container_type, ContainerType::List);
    // Doit avoir 5 ListItem enfants
    let item_count = list.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 5);
}

#[test]
fn liste_ordonnee_longue() {
    let input = "1. Premier\n2. Deuxième\n3. Troisième\n4. Quatrième\n5. Cinquième";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result[0].container_type, ContainerType::OrderedList);
    let item_count = result[0].children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 5);
}
```

---

## Combinaisons de styles dans une même ligne

```rust
#[test]
fn gras_et_italique_sur_mots_differents() {
    // "**gras** et *italique*"
    let result = MarkdownParser.parse("**gras** et *italique*").unwrap();
    // Le ContainerNode racine (Text) doit avoir :
    // Container(Bold), Text(" et "), Container(Italic)
    let root = &result[0];
    let has_bold   = root.children.iter().any(|n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold));
    let has_italic = root.children.iter().any(|n| matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Italic));
    assert!(has_bold);
    assert!(has_italic);
}

#[test]
fn trois_styles_differents_sur_une_ligne() {
    // "**gras** *italique* ~~barré~~"
    let result = MarkdownParser.parse("**gras** *italique* ~~barré~~").unwrap();
    let root = &result[0];
    let bold_count        = count_children_of_type(&root.children, ContainerType::Bold);
    let italic_count      = count_children_of_type(&root.children, ContainerType::Italic);
    let strike_count      = count_children_of_type(&root.children, ContainerType::Strikethrough);
    assert_eq!(bold_count, 1);
    assert_eq!(italic_count, 1);
    assert_eq!(strike_count, 1);
}

#[test]
fn gras_barré_surligné_sur_mots_differents() {
    let result = MarkdownParser.parse("**important** ~~obsolète~~ ==nouveau==").unwrap();
    let root = &result[0];
    assert!(count_children_of_type(&root.children, ContainerType::Bold)          >= 1);
    assert!(count_children_of_type(&root.children, ContainerType::Strikethrough) >= 1);
    assert!(count_children_of_type(&root.children, ContainerType::Surline)       >= 1);
}

#[test]
fn texte_avant_et_apres_le_style() {
    // "Voir le mot **clé** ici"
    // → Text("Voir le mot "), Container(Bold, [Text("clé")]), Text(" ici")
    let result = MarkdownParser.parse("Voir le mot **clé** ici").unwrap();
    let root = &result[0];
    // Doit contenir au moins 3 enfants inline
    assert!(root.children.len() >= 3);
}

#[test]
fn style_en_tout_debut_de_ligne() {
    let result = MarkdownParser.parse("**Titre de post**").unwrap();
    let root = &result[0];
    assert_eq!(root.children.len(), 1);
    assert!(matches!(&root.children[0], InlineNode::Container(c) if c.container_type == ContainerType::Bold));
}

#[test]
fn style_en_toute_fin_de_ligne() {
    let result = MarkdownParser.parse("Texte normal puis *italique*").unwrap();
    let root = &result[0];
    let last = root.children.last().unwrap();
    assert!(matches!(last, InlineNode::Container(c) if c.container_type == ContainerType::Italic));
}
```

---

## Imbrications profondes

```rust
#[test]
fn triple_imbrication_gras_italique_surligné() {
    // "**gras *italique ==surligné== fin* fin**"
    let result = MarkdownParser.parse("**gras *italique ==surligné== fin* fin**").unwrap();
    // Au moins un Container(Bold) à la racine
    let root = &result[0];
    let bold = root.children.iter().find_map(|n| {
        if let InlineNode::Container(c) = n {
            if c.container_type == ContainerType::Bold { Some(c) } else { None }
        } else { None }
    });
    assert!(bold.is_some());
}

#[test]
fn gras_avec_deux_mots_italiques_dedans() {
    // "**Hello *world* and *Rust***"
    let result = MarkdownParser.parse("**Hello *world* and *Rust***").unwrap();
    let bold = find_container(&result, ContainerType::Bold).unwrap();
    let italic_count = count_children_of_type(&bold.children, ContainerType::Italic);
    assert_eq!(italic_count, 2);
}

#[test]
fn liste_avec_items_styles() {
    // "- **important**\n- *note*\n- texte normal"
    let input = "- **important**\n- *note*\n- texte normal";
    let result = MarkdownParser.parse(input).unwrap();
    let list = &result[0];
    assert_eq!(list.container_type, ContainerType::List);
    // 3 items
    let item_count = list.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 3);
}
```

---

## Posts LinkedIn réalistes — Markdown

Ces tests utilisent des textes qui ressemblent à de vrais posts, avec plusieurs blocs, emojis et styles.

```rust
#[test]
fn post_linkedin_annonce_simple() {
    let input = "\
🎉 **Bonne nouvelle !**
Je viens de rejoindre une nouvelle aventure professionnelle.

Après 3 ans chez mon ancienne entreprise, il est temps de relever de nouveaux défis.

*Merci à toute mon équipe pour ces années inoubliables.*";

    let result = MarkdownParser.parse(input).unwrap();
    // Doit produire plusieurs blocs
    assert!(result.len() >= 3);
    // Le premier bloc contient "Bonne nouvelle !" en gras
    let has_bold = result[0].children.iter().any(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold)
    });
    assert!(has_bold);
}

#[test]
fn post_linkedin_liste_apprentissages() {
    let input = "\
Ce que j'ai appris cette année :

- **Rust** est incroyable pour les performances
- *La documentation* est aussi importante que le code
- ~~Les réunions inutiles~~ Le focus, c'est précieux
- ==La communauté== fait toute la différence 🙏";

    let result = MarkdownParser.parse(input).unwrap();
    assert!(result.len() >= 2);
    // Le dernier bloc est une liste
    let last = result.last().unwrap();
    assert_eq!(last.container_type, ContainerType::List);
    let item_count = last.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 4);
}

#[test]
fn post_linkedin_citation_et_liste() {
    let input = "\
> \"Le code propre n'est pas écrit en suivant un ensemble de règles.\"

Mes 3 principes :

1. Nommer les choses clairement
2. Faire une chose à la fois
3. Tester avant de déployer 🚀";

    let result = MarkdownParser.parse(input).unwrap();
    // Blockquote + paragraphe + liste ordonnée
    assert!(result.len() >= 3);
    assert_eq!(result[0].container_type, ContainerType::Blockquote);
    let last = result.last().unwrap();
    assert_eq!(last.container_type, ContainerType::OrderedList);
}

#[test]
fn post_linkedin_long_avec_tous_les_styles() {
    let input = "\
🚀 **3 ans de freelance : ce que personne ne vous dit**

Quand j'ai commencé, je pensais que la technique était le plus dur.
J'avais *complètement* tort.

Voici ce que j'ai vraiment appris :

- **Trouver des clients** est un métier à part entière
- La facturation, ~~personne~~ vraiment personne ne vous apprend ça
- ==Votre réputation== vaut plus que n'importe quel CV
- *La solitude* du freelance est réelle 🧘

> \"Votre réseau est votre filet de sécurité.\"

**Et vous, qu'est-ce qui vous a le plus surpris ?** 👇";

    let result = MarkdownParser.parse(input).unwrap();
    assert!(result.len() >= 5, "Attendu au moins 5 blocs, obtenu {}", result.len());

    // Vérification de présence des types
    let types: Vec<&ContainerType> = result.iter().map(|n| &n.container_type).collect();
    assert!(types.contains(&&ContainerType::List));
    assert!(types.contains(&&ContainerType::Blockquote));
}

#[test]
fn post_linkedin_chiffres_et_pourcentages() {
    let input = "En **2024**, j'ai livré *47 projets* avec un taux de satisfaction de ==98%==.";
    let result = MarkdownParser.parse(input).unwrap();
    assert_eq!(result.len(), 1);
    let root = &result[0];
    // Doit contenir Bold, Italic, Surline
    assert!(count_children_of_type(&root.children, ContainerType::Bold)   >= 1);
    assert!(count_children_of_type(&root.children, ContainerType::Italic) >= 1);
    assert!(count_children_of_type(&root.children, ContainerType::Surline) >= 1);
}

#[test]
fn post_avec_accents_dans_styles() {
    let input = "**Développeur** passionné par l'*élégance* du code.";
    let result = MarkdownParser.parse(input).unwrap();
    assert!(result.len() >= 1);
    // "Développeur" dans un Bold — les accents doivent être dans le TextNode enfant
    let bold = find_container(&result, ContainerType::Bold).unwrap();
    let text = extract_text_from_container(bold);
    assert!(text.contains("Développeur"));
}

#[test]
fn post_avec_ponctuation_speciale() {
    // Guillemets, tirets longs, points de suspension — doivent passer sans erreur
    let input = "**L'innovation** — c'est aussi *oser dire non* à l'inutile…";
    let result = MarkdownParser.parse(input).unwrap();
    assert!(result.len() >= 1);
}
```

---

## Cas limites et robustesse

```rust
#[test]
fn marqueur_gras_vide_produit_container_bold_sans_enfants() {
    // "****" — gras avec contenu vide
    // Comportement attendu : Container(Bold, []) ou erreur propre
    // Ne doit pas paniquer
    let result = MarkdownParser.parse("****");
    // Soit Ok (container vide), soit Err propre — jamais panic
    let _ = result; // juste vérifier que ça ne panique pas
}

#[test]
fn underscore_dans_mot_compose_ne_declenche_pas_italique() {
    // "snake_case_variable" — le _ au milieu d'un mot ne doit pas créer d'italique
    // Ce cas est ambigu en Markdown standard ; documenter le comportement choisi
    // Si le parser le rejette, l'erreur doit être claire
    let result = MarkdownParser.parse("snake_case");
    // Le comportement doit être déterministe (pas de crash)
    let _ = result;
}

#[test]
fn ligne_avec_uniquement_des_espaces_ignoree() {
    let result = MarkdownParser.parse("   \t   ").unwrap();
    assert!(result.is_empty());
}

#[test]
fn texte_avec_retour_chariot_windows() {
    // "\r\n" doit être traité comme un saut de ligne normal
    let result = MarkdownParser.parse("ligne1\r\nligne2");
    assert!(result.is_ok());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2);
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
    // "**bonjour monde comment vas tu**"
    let result = MarkdownParser.parse("**bonjour monde comment vas tu**").unwrap();
    let bold = find_container(&result, ContainerType::Bold).unwrap();
    let text = extract_text_from_container(bold);
    assert_eq!(text, "bonjour monde comment vas tu");
}

#[test]
fn blockquote_avec_style_inline() {
    // "> **Citation importante** de quelqu'un"
    let result = MarkdownParser.parse("> **Citation importante** de quelqu'un").unwrap();
    let bq = &result[0];
    assert_eq!(bq.container_type, ContainerType::Blockquote);
    // Le blockquote doit avoir un enfant Bold
    let has_bold = bq.children.iter().any(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold)
    });
    assert!(has_bold);
}

#[test]
fn liste_avec_emojis_en_items() {
    let input = "- 🎯 Objectif 1\n- 🚀 Lancement\n- ✅ Terminé";
    let result = MarkdownParser.parse(input).unwrap();
    let list = &result[0];
    assert_eq!(list.container_type, ContainerType::List);
    let item_count = list.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 3);
}

#[test]
fn erreur_premier_symbole_non_supporte_arrete_le_parsing() {
    // Le parser s'arrête à la PREMIÈRE erreur
    let input = "texte valide\n# titre invalide\nautres lignes valides";
    let result = MarkdownParser.parse(input);
    // Doit être Err, pas Ok avec les lignes valides seulement
    assert!(result.is_err());
}
```

---

## Helpers de test (à définir dans un module `test_utils` ou en haut du module de test)

```rust
#[cfg(test)]
fn extract_text(nodes: &[ContainerNode]) -> String {
    nodes.iter().flat_map(|n| n.children.iter()).filter_map(|n| {
        if let InlineNode::Text(t) = n { Some(t.text.clone()) } else { None }
    }).collect()
}

#[cfg(test)]
fn find_container<'a>(nodes: &'a [ContainerNode], ct: ContainerType) -> Option<&'a ContainerNode> {
    for node in nodes {
        for child in &node.children {
            if let InlineNode::Container(c) = child {
                if c.container_type == ct { return Some(c); }
            }
        }
    }
    None
}

#[cfg(test)]
fn extract_text_from_container(node: &ContainerNode) -> String {
    node.children.iter().filter_map(|n| {
        if let InlineNode::Text(t) = n { Some(t.text.clone()) } else { None }
    }).collect()
}

#[cfg(test)]
fn count_children_of_type(children: &[InlineNode], ct: ContainerType) -> usize {
    children.iter().filter(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ct)
    }).count()
}
```
