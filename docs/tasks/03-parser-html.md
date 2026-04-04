# Tâche 03 — Implémentation du parser HTML

## Scope

Implémenter `HtmlParser` dans le crate `parser`. Cette tâche suppose que l'AST (tâche 01) et `parse_inline` (tâche 02) sont déjà en place. Elle ne touche **ni `converter`, ni `service`**. Aucune dépendance externe (pas de `scraper`, pas de `html5ever`).

**Référence** : [`03-parser.md`](../03-parser.md), [`03b-parser-html.md`](../03b-parser-html.md), [`02-ast-nodes.md`](../02-ast-nodes.md)

---

## Fichiers à créer / modifier

```
parser/src/
├── lib.rs    ← ajouter : pub mod html;
└── html.rs   ← HtmlParser + impl Parser
```

---

## Structures internes de `HtmlParser`

```rust
// Entrée de la pile lors du traitement d'une balise ouvrante
struct OpenTag {
    container_type: ContainerType,
    tag_name: String,          // ex: "b", "strong", "ul" — pour matcher la fermeture
    children: Vec<InlineNode>,
    opened_at: SourcePosition,
}
```

La pile (`Vec<OpenTag>`) représente l'état courant de l'arbre en cours de construction.

---

## Algorithme — parsing caractère par caractère

```
stack  = []           // pile des balises ouvertes
result = []           // ContainerNode racines finalisés
text_buf = ""         // texte courant accumulé

i = 0
tant que i < len(input) :

    si input[i] == '<' :
        flush_text(text_buf, stack)  // TextNode dans le parent courant

        si input[i+1..] commence par '!--' :
            avancer jusqu'à '-->'      // commentaire HTML, skip
        sinon si input[i+1..] commence par '!' :
            avancer jusqu'à '>'        // DOCTYPE, skip
        sinon si input[i+1] == '/' :
            tag = extract_tag(input, i)   // ex: "/strong"
            pop_tag(tag[1..], stack, result)
            avancer après '>'
        sinon :
            tag = extract_tag(input, i)   // ex: "strong", "br", "ul"
            si tag est void ('br') :
                push TextNode("\n") dans current_children(stack, result)
            sinon si tag est transparent ('p') :
                // ne rien empiler
            sinon si tag est supporté :
                empiler OpenTag { container_type, tag_name: tag, children: [], opened_at }
            sinon :
                → ParseError::UnsupportedSymbol { symbol: "<tag>", position }
            avancer après '>'
    sinon :
        text_buf += input[i]
        i += 1

flush_text(text_buf, stack)

si input est '</p>' (fin de </p>) :
    push TextNode("\n") dans current_children

si stack non vide :
    → ParseError::UnclosedTag { tag: stack.last().tag_name, position: stack.last().opened_at }

retourner result
```

### Fonctions auxiliaires

**`flush_text(buf, stack)`** : si `buf` non vide, crée un `TextNode` et l'ajoute à `current_children(stack)`, puis vide `buf`.

**`current_children(stack, result)`** : retourne `&mut stack.last().children` si la pile est non vide, sinon prépare un nouveau `ContainerNode(Text)` dans `result`.

**`pop_tag(tag_name, stack, result)`** :
- Cherche dans la pile le dernier `OpenTag` avec `tag_name` correspondant.
- Crée un `ContainerNode { container_type, children }` depuis l'`OpenTag`.
- Si la pile est vide après pop → ajouter le nœud dans `result`.
- Sinon → ajouter le nœud dans `stack.last().children`.

**`tag_to_container_type(tag) -> Option<ContainerType>`** :

| Tags | ContainerType |
|---|---|
| `strong`, `b` | `Bold` |
| `em`, `i` | `Italic` |
| `u` | `Underline` |
| `mark` | `Surline` |
| `s`, `del` | `Strikethrough` |
| `blockquote` | `Blockquote` |
| `ul` | `List` |
| `ol` | `OrderedList` |
| `li` | `ListItem` |
| `p` | transparent (pas de push) |
| `br` | void (TextNode `\n`) |
| tout autre | `None` → erreur |

**Tags interdits explicitement** : `h1`–`h6`, `div`, `span`, `code`, `pre`, `table`, `a`, `img`, etc. → `ParseError::UnsupportedSymbol`.

---

## Gestion du `</p>`

`<p>` ne crée pas d'entrée dans la pile. À la rencontre de `</p>`, on insère un `TextNode("\n")` dans les enfants courants.

---

## `supported_symbols()` pour `HtmlParser`

Retourner les 13 entrées suivantes :

| symbol | description | example |
|---|---|---|
| `<strong>` | Gras | `<strong>texte</strong>` |
| `<b>` | Gras (alias) | `<b>texte</b>` |
| `<em>` | Italique | `<em>texte</em>` |
| `<i>` | Italique (alias) | `<i>texte</i>` |
| `<u>` | Souligné | `<u>texte</u>` |
| `<mark>` | Surligné | `<mark>texte</mark>` |
| `<s>` | Texte barré | `<s>texte</s>` |
| `<del>` | Texte barré (alias) | `<del>texte</del>` |
| `<blockquote>` | Citation | `<blockquote>texte</blockquote>` |
| `<ul>` | Liste non ordonnée | `<ul><li>item</li></ul>` |
| `<ol>` | Liste ordonnée | `<ol><li>item</li></ol>` |
| `<li>` | Item de liste | `<li>contenu</li>` |
| `<br>` | Saut de ligne | `<br>` |

---

## Tests à implémenter

Fichier : `parser/src/html.rs` (module `#[cfg(test)]`)

### Cas nominaux

```rust
#[test]
fn texte_brut_produit_container_text() {
    // "Bonjour" → [ContainerNode(Text, [Text("Bonjour")])]
}

#[test]
fn strong_produit_container_bold() {
    // "<strong>Hello</strong>" → [ContainerNode(Bold, [Text("Hello")])]
}

#[test]
fn b_produit_container_bold() {
    // "<b>Hello</b>" → ContainerNode(Bold, ...)
}

#[test]
fn em_produit_container_italic() {
    // "<em>monde</em>" → ContainerNode(Italic, ...)
}

#[test]
fn u_produit_container_underline() {
    // "<u>texte</u>" → ContainerNode(Underline, ...)
}

#[test]
fn mark_produit_container_surline() {
    // "<mark>texte</mark>" → ContainerNode(Surline, ...)
}

#[test]
fn s_produit_container_strikethrough() {
    // "<s>texte</s>" → ContainerNode(Strikethrough, ...)
}

#[test]
fn del_produit_container_strikethrough() {
    // "<del>texte</del>" → ContainerNode(Strikethrough, ...)
}

#[test]
fn br_insere_newline() {
    // "ligne1<br>ligne2" → Text("ligne1\nligne2")
}

#[test]
fn p_transparent_et_fermeture_insere_newline() {
    // "<p>texte</p>" → Text("texte\n")
}

#[test]
fn ul_li_produit_liste() {
    // "<ul><li>a</li><li>b</li></ul>"
    // → ContainerNode(List, [ListItem([Text("a")]), ListItem([Text("b")])])
}

#[test]
fn ol_li_produit_liste_ordonnee() {
    // "<ol><li>un</li></ol>"
    // → ContainerNode(OrderedList, [ListItem([Text("un")])])
}

#[test]
fn imbrication_strong_dans_texte() {
    // "texte <strong>gras</strong> suite"
    // → ContainerNode(Text, [Text("texte "), Container(Bold, [Text("gras")]), Text(" suite")])
}

#[test]
fn blockquote_produit_container_blockquote() {
    // "<blockquote>citation</blockquote>" → ContainerNode(Blockquote, [Text("citation")])
}
```

### Commentaires et DOCTYPE

```rust
#[test]
fn commentaire_html_ignore() {
    // "<!-- commentaire -->texte" → [ContainerNode(Text, [Text("texte")])]
}

#[test]
fn doctype_ignore() {
    // "<!DOCTYPE html><strong>X</strong>" → ContainerNode(Bold, [Text("X")])
}
```

### Erreurs

```rust
#[test]
fn div_produit_erreur_unsupported() {
    let result = HtmlParser.parse("<div>contenu</div>");
    assert!(matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "<div>"));
}

#[test]
fn span_produit_erreur_unsupported() {
    let result = HtmlParser.parse("<span>x</span>");
    assert!(matches!(result, Err(ParseError::UnsupportedSymbol { .. })));
}

#[test]
fn h1_produit_erreur_unsupported() {
    let result = HtmlParser.parse("<h1>titre</h1>");
    assert!(matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "<h1>"));
}

#[test]
fn balise_non_fermee_produit_erreur_unclosed() {
    let result = HtmlParser.parse("<strong>non fermé");
    assert!(matches!(result, Err(ParseError::UnclosedTag { tag, .. }) if tag == "strong"));
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
```

### Cas limites

```rust
#[test]
fn input_vide_retourne_vec_vide() {
    let result = HtmlParser.parse("").unwrap();
    assert!(result.is_empty());
}

#[test]
fn texte_unicode_preserve() {
    // "<strong>données</strong>" → Bold contenant "données"
}

#[test]
fn supported_symbols_retourne_13_entrees() {
    let symbols = HtmlParser.supported_symbols();
    assert_eq!(symbols.len(), 13);
}
```

---

## Critère de succès

```bash
cargo test --package parser --lib html
```

Tous les tests passent. `cargo check --workspace` reste vert.
