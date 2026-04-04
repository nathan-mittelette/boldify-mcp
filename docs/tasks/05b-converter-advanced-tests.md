# Tâche 05b — Tests avancés : Converter (handlers + nœuds)

## Scope

Compléments de tests à ajouter dans le crate `converter`, en plus des tests de base (tâches 04 et 05). Couvre les handlers sur des textes réalistes (emojis, accents complets, phrases longues), les cas de conversion sur des arbres AST complexes, et la cohérence entre styles.

> Les nœuds AST sont **toujours construits manuellement** — ces tests ne font pas appel au parser.

---

## Tests sur `BoldHandler` — textes réalistes

```rust
// converter/src/handlers/bold.rs — module #[cfg(test)]

#[test]
fn bold_phrase_complete_avec_espaces() {
    let h = bold_handler();
    let result = h.apply("Bonjour tout le monde");
    // Aucune lettre ASCII ne doit survivre
    assert!(!result.chars().any(|c| c.is_ascii_alphabetic()));
    // Les espaces doivent être préservés
    assert!(result.contains(' '));
}

#[test]
fn bold_phrase_avec_ponctuation() {
    let h = bold_handler();
    let result = h.apply("Hello, world!");
    // La virgule et le point d'exclamation ne sont pas dans la map — préservés tels quels
    assert!(result.contains(','));
    assert!(result.contains('!'));
    // Les lettres sont transformées
    assert!(!result.contains('H'));
    assert!(!result.contains('e'));
}

#[test]
fn bold_chiffres_et_lettres_mixtes() {
    let h = bold_handler();
    let result = h.apply("Top 3 en 2024");
    assert!(!result.contains('T'));
    assert!(!result.contains('3'));
    assert!(!result.contains('2'));
    assert!(result.contains(' '));
}

#[test]
fn bold_tous_les_accents_francais() {
    let h = bold_handler();
    let accents = "éèêëàáâäùúûüôöîïçÉÈÊËÀÂÄÙÛÜÔÖÎÏÇ";
    let result = h.apply(accents);
    // Aucun accent original ne doit survivre tel quel
    for c in accents.chars() {
        assert!(!result.contains(c), "Le caractère '{}' n'a pas été transformé", c);
    }
    // Le résultat doit contenir des combining characters
    assert!(result.chars().any(|c| matches!(c as u32, 0x0300..=0x036F)));
}

#[test]
fn bold_emoji_preserve_sans_transformation() {
    let h = bold_handler();
    // Les emojis ne sont pas dans la map → préservés tels quels
    assert_eq!(h.apply("🚀"), "🚀");
    assert_eq!(h.apply("🎉"), "🎉");
    assert_eq!(h.apply("👨‍💻"), "👨‍💻");
}

#[test]
fn bold_texte_avec_emoji_milieu() {
    let h = bold_handler();
    let result = h.apply("Top 🚀 performer");
    assert!(result.contains("🚀")); // emoji préservé
    assert!(!result.contains('T')); // lettres transformées
    assert!(!result.contains('p'));
}

#[test]
fn bold_newline_preserve() {
    let h = bold_handler();
    let result = h.apply("ligne1\nligne2");
    assert!(result.contains('\n'));
}

#[test]
fn bold_chaine_vide_retourne_chaine_vide() {
    let h = bold_handler();
    assert_eq!(h.apply(""), "");
}

#[test]
fn bold_uniquement_espaces_retourne_espaces() {
    let h = bold_handler();
    assert_eq!(h.apply("   "), "   ");
}

#[test]
fn bold_texte_unicode_non_latin_preserve() {
    // Les caractères chinois, arabes, etc. ne sont pas dans la map → préservés
    let h = bold_handler();
    assert_eq!(h.apply("你好"), "你好");
    assert_eq!(h.apply("مرحبا"), "مرحبا");
}

#[test]
fn bold_tous_les_chiffres() {
    let h = bold_handler();
    let result = h.apply("0123456789");
    assert!(!result.contains('0'));
    assert!(!result.contains('9'));
    assert_eq!(result.chars().count(), 10);
}
```

---

## Tests sur `ItalicHandler`

```rust
// converter/src/handlers/italic.rs — module #[cfg(test)]

#[test]
fn italic_different_du_bold() {
    let bold_result   = bold_handler().apply("Hello");
    let italic_result = italic_handler().apply("Hello");
    assert_ne!(bold_result, italic_result);
}

#[test]
fn italic_phrase_avec_accents() {
    let result = italic_handler().apply("élégance");
    assert!(!result.contains('é'));
    assert!(!result.contains('e')); // 'e' de base transformé
}

#[test]
fn italic_emoji_preserve() {
    assert_eq!(italic_handler().apply("✨"), "✨");
}

#[test]
fn italic_chiffres_pas_de_variante_unicode() {
    // Les chiffres n'ont pas de variante italique en Unicode — préservés
    let result = italic_handler().apply("123");
    assert_eq!(result, "123");
}
```

---

## Tests sur `UnderlineHandler`

```rust
// converter/src/handlers/underline.rs — module #[cfg(test)]

#[test]
fn underline_chaque_char_a_son_combining() {
    let h = UnderlineHandler;
    let result = h.apply("ABC");
    let chars: Vec<char> = result.chars().collect();
    // Structure attendue : A U+0332 B U+0332 C U+0332
    assert_eq!(chars.len(), 6);
    assert_eq!(chars[1], '\u{0332}');
    assert_eq!(chars[3], '\u{0332}');
    assert_eq!(chars[5], '\u{0332}');
}

#[test]
fn underline_emoji_preserve_sans_combining() {
    // Les emojis ne reçoivent pas le combining — comportement à documenter
    let h = UnderlineHandler;
    let result = h.apply("🚀");
    // L'emoji doit être présent
    assert!(result.contains('🚀') || result.chars().next().unwrap() as u32 > 0xFFFF);
}

#[test]
fn underline_espace_preserve_sans_combining() {
    let h = UnderlineHandler;
    let result = h.apply("A B");
    let chars: Vec<char> = result.chars().collect();
    // A + U+0332, espace (sans combining), B + U+0332
    let space_index = chars.iter().position(|&c| c == ' ').unwrap();
    // Le char précédant l'espace doit être U+0332
    assert_eq!(chars[space_index - 1], '\u{0332}');
    // Le char suivant l'espace doit être 'B', pas U+0332
    assert_eq!(chars[space_index + 1], 'B');
}
```

---

## Tests sur `StrikethroughHandler`

```rust
// converter/src/handlers/strikethrough.rs — module #[cfg(test)]

#[test]
fn strikethrough_chaque_char_a_son_combining_stroke() {
    let h = StrikethroughHandler;
    let result = h.apply("AB");
    let chars: Vec<char> = result.chars().collect();
    assert_eq!(chars[1], '\u{0336}');
    assert_eq!(chars[3], '\u{0336}');
}

#[test]
fn strikethrough_different_du_underline() {
    let u = UnderlineHandler.apply("X");
    let s = StrikethroughHandler.apply("X");
    assert_ne!(u, s);
}

#[test]
fn strikethrough_preserve_espace() {
    let h = StrikethroughHandler;
    let result = h.apply("A B");
    assert!(result.contains(' '));
}
```

---

## Tests sur `SurlineHandler`

```rust
// converter/src/handlers/surline.rs — module #[cfg(test)]

#[test]
fn surline_encadre_avec_guillemets_speciaux() {
    let h = SurlineHandler;
    let result = h.apply("texte");
    assert!(result.starts_with('〚'));
    assert!(result.ends_with('〛'));
    assert!(result.contains("texte"));
}

#[test]
fn surline_chaine_vide() {
    let h = SurlineHandler;
    let result = h.apply("");
    assert_eq!(result, "〚〛");
}

#[test]
fn surline_avec_emoji() {
    let h = SurlineHandler;
    let result = h.apply("🚀 go");
    assert!(result.contains("🚀 go"));
}
```

---

## Tests sur `ContainerNode::to_unicode` — arbres complexes

```rust
// converter/src/nodes/container_node.rs — module #[cfg(test)]

#[test]
fn bold_avec_emoji_dans_le_texte() {
    let node = make_container(ContainerType::Bold, vec![make_text("🔥 Résultats")]);
    let result = node.to_unicode();
    // L'emoji est préservé, les lettres sont transformées
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
    let node = make_container(ContainerType::Strikethrough, vec![make_text("ancien contenu")]);
    let result = node.to_unicode();
    // Chaque lettre doit être suivie de U+0336
    assert!(result.contains('\u{0336}'));
    assert!(result.contains(' ')); // espaces préservés
}

#[test]
fn underline_avec_chiffres() {
    let node = make_container(ContainerType::Underline, vec![make_text("2024")]);
    let result = node.to_unicode();
    assert!(result.contains('\u{0332}'));
}

#[test]
fn blockquote_avec_emoji_dans_citation() {
    let node = make_container(ContainerType::Blockquote, vec![make_text("Sois le changement 🌱")]);
    let result = node.to_unicode();
    assert!(result.contains('❝'));
    assert!(result.contains("🌱"));
}

#[test]
fn list_avec_items_contenant_des_emojis() {
    let items: Vec<InlineNode> = vec!["🎯 Objectif", "🚀 Lancement", "✅ Terminé"]
        .into_iter()
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
    // Un item contient lui-même un Container(Bold)
    let bold_child = InlineNode::Container(
        make_container(ContainerType::Bold, vec![make_text("Important")])
    );
    let li = InlineNode::ListItem(ListItemNode {
        base: NodeBase::new(0, Span::new(0, 0)),
        children: vec![bold_child, make_text(" à retenir")],
    });
    let node = make_container(ContainerType::OrderedList, vec![li]);
    let result = node.to_unicode();
    // "1. " présent
    assert!(result.contains("1."));
    // Le mot "Important" est transformé (bold)
    assert!(!result.contains("Important"));
    // " à retenir" est en texte brut
    assert!(result.contains(" à retenir"));
}
```

---

## Tests de cohérence entre styles

```rust
#[test]
fn tous_les_styles_produisent_des_resultats_differents() {
    let text = "ABC";
    let bold          = make_container(ContainerType::Bold,          vec![make_text(text)]).to_unicode();
    let italic        = make_container(ContainerType::Italic,        vec![make_text(text)]).to_unicode();
    let underline     = make_container(ContainerType::Underline,     vec![make_text(text)]).to_unicode();
    let strikethrough = make_container(ContainerType::Strikethrough, vec![make_text(text)]).to_unicode();
    let surline       = make_container(ContainerType::Surline,       vec![make_text(text)]).to_unicode();

    let styles = [&bold, &italic, &underline, &strikethrough, &surline];
    for i in 0..styles.len() {
        for j in (i + 1)..styles.len() {
            assert_ne!(styles[i], styles[j],
                "Les styles {} et {} produisent le même résultat pour \"{}\"", i, j, text);
        }
    }
}

#[test]
fn style_sur_chaine_identique_retourne_toujours_le_meme_resultat() {
    // Déterminisme : appeler apply() deux fois retourne le même résultat
    let h = bold_handler();
    let r1 = h.apply("Hello World");
    let r2 = h.apply("Hello World");
    assert_eq!(r1, r2);
}

#[test]
fn bold_puis_italic_sur_meme_texte_pas_idem() {
    let bold   = bold_handler().apply("test");
    let italic = italic_handler().apply("test");
    assert_ne!(bold, italic);
}
```

---

## Tests sur `convert()` — fonction publique du crate

```rust
// converter/src/lib.rs — module #[cfg(test)]

#[test]
fn convert_post_linkedin_simule() {
    // Simulation d'un arbre AST produit par le parser Markdown pour un post type
    let nodes = vec![
        // "🔥 **3 conseils pour progresser**"
        make_container(ContainerType::Text, vec![
            make_text("🔥 "),
            InlineNode::Container(make_container(ContainerType::Bold, vec![
                make_text("3 conseils pour progresser"),
            ])),
        ]),
        // Liste avec 3 items
        make_container(ContainerType::List, vec![
            InlineNode::ListItem(ListItemNode {
                base: NodeBase::new(1, Span::new(0, 0)),
                children: vec![
                    InlineNode::Container(make_container(ContainerType::Bold, vec![make_text("Lire")])),
                    make_text(" tous les jours"),
                ],
            }),
            InlineNode::ListItem(ListItemNode {
                base: NodeBase::new(2, Span::new(0, 0)),
                children: vec![make_text("Pratiquer "), InlineNode::Container(
                    make_container(ContainerType::Italic, vec![make_text("régulièrement")])
                )],
            }),
            InlineNode::ListItem(ListItemNode {
                base: NodeBase::new(3, Span::new(0, 0)),
                children: vec![make_text("Partager ses "), InlineNode::Container(
                    make_container(ContainerType::Strikethrough, vec![make_text("erreurs")])
                ), make_text(" apprentissages")],
            }),
        ]),
        // Blockquote final
        make_container(ContainerType::Blockquote, vec![
            make_text("La progression constante bat la perfection occasionnelle."),
        ]),
    ];

    let result = convert(&nodes);

    // Structure globale
    assert!(!result.is_empty());
    // Emoji préservé dans le premier bloc
    assert!(result.contains("🔥"));
    // Séparateur entre blocs
    assert!(result.contains('\n'));
    // Puces de liste
    assert!(result.contains("• "));
    // Citation
    assert!(result.contains('❝'));
    // "Lire" en bold → ne contient pas l'ASCII
    assert!(!result.contains("Lire"));
    // Texte normal préservé
    assert!(result.contains(" tous les jours"));
}

#[test]
fn convert_noeud_text_avec_newline_interne() {
    // Un TextNode peut contenir des \n (issus de <br> ou </p>)
    let node = make_container(ContainerType::Bold, vec![
        make_text("ligne1\nligne2"),
    ]);
    let result = node.to_unicode();
    assert!(result.contains('\n'));
    // Les lettres sont transformées, les \n préservés
    assert!(!result.contains("ligne"));
}

#[test]
fn convert_container_text_avec_inline_containers_multiples() {
    // "Texte **gras** et *italique* et ==surligné=="
    let node = make_container(ContainerType::Text, vec![
        make_text("Texte "),
        InlineNode::Container(make_container(ContainerType::Bold,   vec![make_text("gras")])),
        make_text(" et "),
        InlineNode::Container(make_container(ContainerType::Italic, vec![make_text("italique")])),
        make_text(" et "),
        InlineNode::Container(make_container(ContainerType::Surline, vec![make_text("surligné")])),
    ]);
    let result = node.to_unicode();

    assert!(result.contains("Texte "));
    assert!(result.contains(" et "));
    assert!(!result.contains("gras"));      // bold transformé
    assert!(!result.contains("italique"));  // italic transformé
    assert!(result.contains("surligné"));   // surline encadre mais le texte brut est dedans
    assert!(result.contains('〚'));
}
```
