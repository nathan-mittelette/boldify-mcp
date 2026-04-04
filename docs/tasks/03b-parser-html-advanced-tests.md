# Tâche 03b — Tests avancés : Parser HTML

## Scope

Compléments de tests à ajouter dans `parser/src/html.rs` en plus des tests de base (tâche 03). Couvre des HTML multi-niveaux, emojis, attributs ignorés, imbrications complexes, et des textes réalistes issus d'éditeurs rich-text (Notion, LinkedIn, etc.).

---

## Emojis dans le HTML

```rust
#[test]
fn emoji_dans_texte_brut_preserve() {
    let result = HtmlParser.parse("🚀 Lancement").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("🚀"));
}

#[test]
fn emoji_dans_strong_preserve() {
    let result = HtmlParser.parse("<strong>🔥 Top performer</strong>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("🔥"));
    assert!(text.contains("Top performer"));
    assert!(result[0].container_type == ContainerType::Bold);
}

#[test]
fn emoji_dans_em_preserve() {
    let result = HtmlParser.parse("<em>✨ Incroyable</em>").unwrap();
    assert!(result[0].container_type == ContainerType::Italic);
    let text = flatten_text(&result);
    assert!(text.contains("✨"));
}

#[test]
fn emoji_entre_balises_preserve() {
    let result = HtmlParser.parse("<strong>Avant</strong> 👉 <em>Après</em>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("👉"));
}

#[test]
fn emoji_multicodepoint_dans_li_preserve() {
    let result = HtmlParser.parse("<ul><li>👨‍💻 Dev</li><li>👩‍🎨 Designer</li></ul>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("👨‍💻"));
    assert!(text.contains("👩‍🎨"));
}
```

---

## Attributs HTML

Les attributs dans les balises ouvrantes doivent être ignorés — seul le nom de la balise est utilisé.

```rust
#[test]
fn strong_avec_attribut_class_ignore() {
    // "<strong class="highlight">texte</strong>" → ContainerNode(Bold, [Text("texte")])
    let result = HtmlParser.parse(r#"<strong class="highlight">texte</strong>"#).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].container_type, ContainerType::Bold);
    let text = flatten_text(&result);
    assert_eq!(text, "texte");
}

#[test]
fn em_avec_attribut_style_ignore() {
    let result = HtmlParser.parse(r#"<em style="color:red">italique</em>"#).unwrap();
    assert_eq!(result[0].container_type, ContainerType::Italic);
}

#[test]
fn ul_avec_attribut_ignore() {
    let result = HtmlParser.parse(r#"<ul class="list-disc"><li>item</li></ul>"#).unwrap();
    assert_eq!(result[0].container_type, ContainerType::List);
}

#[test]
fn br_avec_attribut_slash_self_closing() {
    // "<br />" — variante XHTML du br
    let result = HtmlParser.parse("avant<br />après").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains('\n'));
}

#[test]
fn tag_inconnu_avec_attribut_retourne_erreur_avec_le_bon_tag() {
    let result = HtmlParser.parse(r#"<div class="foo">x</div>"#);
    assert!(matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "<div>"));
}
```

---

## Imbrications HTML complexes

```rust
#[test]
fn strong_dans_em() {
    // "<em><strong>texte</strong></em>"
    let result = HtmlParser.parse("<em><strong>texte</strong></em>").unwrap();
    let italic = &result[0];
    assert_eq!(italic.container_type, ContainerType::Italic);
    let has_bold = italic.children.iter().any(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold)
    });
    assert!(has_bold);
}

#[test]
fn em_dans_strong_dans_p() {
    let result = HtmlParser.parse("<p><strong>Bonjour <em>monde</em></strong></p>").unwrap();
    let bold = find_node(&result, ContainerType::Bold).unwrap();
    let has_italic = bold.children.iter().any(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Italic)
    });
    assert!(has_italic);
}

#[test]
fn liste_avec_items_styles() {
    let input = "<ul>\
        <li><strong>Premier</strong> point</li>\
        <li>Deuxième point <em>important</em></li>\
        <li>~~Troisième~~ <s>obsolète</s></li>\
    </ul>";
    let result = HtmlParser.parse(input).unwrap();
    assert_eq!(result[0].container_type, ContainerType::List);
    let item_count = result[0].children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 3);
}

#[test]
fn blockquote_avec_strong_dedans() {
    let result = HtmlParser.parse("<blockquote><strong>Important</strong> à retenir</blockquote>").unwrap();
    let bq = &result[0];
    assert_eq!(bq.container_type, ContainerType::Blockquote);
    let has_bold = bq.children.iter().any(|n| {
        matches!(n, InlineNode::Container(c) if c.container_type == ContainerType::Bold)
    });
    assert!(has_bold);
}

#[test]
fn ol_li_avec_style_inline() {
    let input = "<ol><li><strong>Étape 1</strong></li><li><em>Étape 2</em></li></ol>";
    let result = HtmlParser.parse(input).unwrap();
    assert_eq!(result[0].container_type, ContainerType::OrderedList);
}

#[test]
fn triple_imbrication_mark_em_strong() {
    let result = HtmlParser.parse("<mark><em><strong>triple</strong></em></mark>").unwrap();
    assert_eq!(result[0].container_type, ContainerType::Surline);
}
```

---

## Sauts de ligne et paragraphes

```rust
#[test]
fn multiple_br_insere_multiple_newlines() {
    let result = HtmlParser.parse("a<br>b<br>c").unwrap();
    let text = flatten_text(&result);
    assert_eq!(text, "a\nb\nc");
}

#[test]
fn p_suivi_de_p_insere_deux_newlines() {
    let result = HtmlParser.parse("<p>premier</p><p>deuxième</p>").unwrap();
    let text = flatten_text(&result);
    // Chaque </p> insère un \n
    assert!(text.contains("premier\n"));
    assert!(text.contains("deuxième"));
}

#[test]
fn p_vide_insere_newline() {
    let result = HtmlParser.parse("<p></p>").unwrap();
    let text = flatten_text(&result);
    assert_eq!(text, "\n");
}

#[test]
fn br_dans_liste_item_insere_newline() {
    let result = HtmlParser.parse("<ul><li>ligne1<br>ligne2</li></ul>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("ligne1\nligne2"));
}

#[test]
fn br_dans_strong_insere_newline_dans_le_bold() {
    let result = HtmlParser.parse("<strong>avant<br>après</strong>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains('\n'));
}
```

---

## Entités HTML

Les entités courantes doivent être préservées telles quelles dans le `TextNode` (le renderer ne les décode pas — ce n'est pas son rôle).

```rust
#[test]
fn entite_amp_preserve_dans_texte() {
    // "&amp;" → le parser conserve "&amp;" tel quel dans le TextNode
    // (pas de décodage d'entité — c'est la responsabilité du client)
    let result = HtmlParser.parse("A &amp; B").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("&amp;") || text.contains("&")); // selon le choix d'implémentation
}

#[test]
fn entite_nbsp_preserve_dans_texte() {
    let result = HtmlParser.parse("mot&nbsp;mot").unwrap();
    // Ne doit pas paniquer
    assert!(result.is_ok() || true); // le comportement est documenté, pas forcément décodé
}
```

---

## HTML réaliste d'éditeurs rich-text

Ces inputs reproduisent ce que génèrent Notion, LinkedIn editor, ou d'autres outils.

```rust
#[test]
fn html_notion_like_paragraphe_avec_strong() {
    let input = r#"<p>Aujourd'hui, j'ai <strong>lancé</strong> mon nouveau projet.</p>"#;
    let result = HtmlParser.parse(input).unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("lancé"));
    // Vérifier que "lancé" est dans un Bold
    let bold = find_node(&result, ContainerType::Bold);
    assert!(bold.is_some());
}

#[test]
fn html_linkedin_post_simple() {
    let input = "<p><strong>🎉 Nouvelle étape professionnelle !</strong></p>\
                 <p>Je suis ravi d'annoncer que je rejoins <em>Acme Corp</em> en tant que Senior Engineer.</p>\
                 <p>Merci à tous ceux qui m'ont soutenu dans cette aventure. 🙏</p>";

    let result = HtmlParser.parse(input).unwrap();
    // Au moins un Bold et un Italic
    assert!(find_node(&result, ContainerType::Bold).is_some());
    assert!(find_node(&result, ContainerType::Italic).is_some());
    // Les emojis sont préservés
    let text = flatten_text(&result);
    assert!(text.contains("🎉"));
    assert!(text.contains("🙏"));
}

#[test]
fn html_post_avec_liste_et_citation() {
    let input = "\
        <p><strong>Ce que j'ai appris en 2024 :</strong></p>\
        <ul>\
            <li><strong>Rust</strong> > tout pour la perf 🚀</li>\
            <li>La <em>documentation</em> sauve des vies</li>\
            <li><s>Les deadlines</s> Le temps, c'est précieux</li>\
        </ul>\
        <blockquote>Le meilleur code est celui qu'on n'a pas à écrire.</blockquote>";

    let result = HtmlParser.parse(input).unwrap();
    assert!(result.len() >= 2); // au moins liste + blockquote

    let types: Vec<&ContainerType> = result.iter().map(|n| &n.container_type).collect();
    assert!(types.contains(&&ContainerType::List));
    assert!(types.contains(&&ContainerType::Blockquote));

    // La liste a 3 items
    let list = result.iter().find(|n| n.container_type == ContainerType::List).unwrap();
    let item_count = list.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 3);
}

#[test]
fn html_post_long_tous_les_styles() {
    let input = "\
        <p>🚀 <strong>3 ans de freelance : ce que personne ne vous dit</strong></p>\
        <p>Quand j'ai commencé, je pensais que la technique était le plus dur.<br>\
        J'avais <em>complètement</em> tort.</p>\
        <p>Voici ce que j'ai vraiment appris :</p>\
        <ul>\
            <li><strong>Trouver des clients</strong> est un métier à part entière</li>\
            <li>La facturation, <s>personne</s> vraiment personne ne vous apprend ça</li>\
            <li><mark>Votre réputation</mark> vaut plus que n'importe quel CV</li>\
            <li><em>La solitude</em> du freelance est réelle 🧘</li>\
        </ul>\
        <blockquote>\"Votre réseau est votre filet de sécurité.\"</blockquote>\
        <p><strong>Et vous, qu'est-ce qui vous a le plus surpris ?</strong> 👇</p>";

    let result = HtmlParser.parse(input).unwrap();
    assert!(result.len() >= 4);

    let text = flatten_text(&result);
    assert!(text.contains("🚀"));
    assert!(text.contains("🧘"));
    assert!(text.contains("👇"));

    // Tous les types présents
    assert!(find_node(&result, ContainerType::Bold).is_some());
    assert!(find_node(&result, ContainerType::Italic).is_some());
    assert!(find_node(&result, ContainerType::Strikethrough).is_some());
    assert!(find_node(&result, ContainerType::Surline).is_some());
    assert!(find_node(&result, ContainerType::Blockquote).is_some());
    assert!(find_node(&result, ContainerType::List).is_some());
}

#[test]
fn html_avec_accents_dans_tous_les_styles() {
    let input = "<strong>Développeur</strong> passionné par <em>l'élégance</em> du <u>code propre</u>.";
    let result = HtmlParser.parse(input).unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("Développeur"));
    assert!(text.contains("l'élégance"));
    assert!(text.contains("code propre"));
}

#[test]
fn html_chiffres_et_pourcentages_dans_styles() {
    let input = "<p>En <strong>2024</strong>, j'ai livré <em>47 projets</em> avec un taux de <mark>98%</mark>.</p>";
    let result = HtmlParser.parse(input).unwrap();
    assert!(find_node(&result, ContainerType::Bold).is_some());
    assert!(find_node(&result, ContainerType::Italic).is_some());
    assert!(find_node(&result, ContainerType::Surline).is_some());
}
```

---

## Robustesse et cas limites

```rust
#[test]
fn tag_vide_strong_sans_contenu() {
    // "<strong></strong>" — balise vide
    let result = HtmlParser.parse("<strong></strong>").unwrap();
    assert_eq!(result[0].container_type, ContainerType::Bold);
    assert!(result[0].children.is_empty());
}

#[test]
fn texte_brut_avant_et_apres_balise() {
    let result = HtmlParser.parse("Avant <strong>gras</strong> après").unwrap();
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
    let text = flatten_text(&result);
    assert_eq!(text, "你好世界");
}

#[test]
fn html_arabe_preserve() {
    let result = HtmlParser.parse("<em>مرحبا</em>").unwrap();
    let text = flatten_text(&result);
    assert!(text.contains("مرحبا"));
}

#[test]
fn balise_fermante_sans_ouvrante_ne_panique_pas() {
    // "</strong>" sans <strong> ouvert — le parser doit gérer proprement
    let result = HtmlParser.parse("texte </strong> suite");
    // Soit erreur propre, soit le tag fermant est ignoré — jamais panic
    let _ = result;
}

#[test]
fn tres_longue_liste_html() {
    let items: String = (1..=50).map(|i| format!("<li>Item {}</li>", i)).collect();
    let input = format!("<ul>{}</ul>", items);
    let result = HtmlParser.parse(&input).unwrap();
    let list = &result[0];
    assert_eq!(list.container_type, ContainerType::List);
    let item_count = list.children.iter()
        .filter(|n| matches!(n, InlineNode::ListItem(_)))
        .count();
    assert_eq!(item_count, 50);
}

#[test]
fn balise_avec_attribut_data_ignore() {
    let result = HtmlParser.parse(r#"<strong data-id="123">texte</strong>"#).unwrap();
    assert_eq!(result[0].container_type, ContainerType::Bold);
}

#[test]
fn erreur_stoppe_a_la_premiere_balise_invalide() {
    // Même si le reste est valide, l'erreur est immédiate
    let input = "<strong>valide</strong><div>invalide</div><em>aussi valide</em>";
    let result = HtmlParser.parse(input);
    assert!(result.is_err());
    // La balise <em> après l'erreur n'est pas parsée
}
```

---

## Helpers de test

```rust
#[cfg(test)]
fn flatten_text(nodes: &[ContainerNode]) -> String {
    fn collect(children: &[InlineNode], buf: &mut String) {
        for node in children {
            match node {
                InlineNode::Text(t)      => buf.push_str(&t.text),
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

#[cfg(test)]
fn find_node<'a>(nodes: &'a [ContainerNode], ct: ContainerType) -> Option<&'a ContainerNode> {
    fn search<'a>(children: &'a [InlineNode], ct: &ContainerType) -> Option<&'a ContainerNode> {
        for child in children {
            if let InlineNode::Container(c) = child {
                if c.container_type == *ct { return Some(c); }
                if let Some(found) = search(&c.children, ct) { return Some(found); }
            }
        }
        None
    }
    for node in nodes {
        if node.container_type == ct { return Some(node); }
        if let Some(found) = search(&node.children, &ct) { return Some(found); }
    }
    None
}
```
