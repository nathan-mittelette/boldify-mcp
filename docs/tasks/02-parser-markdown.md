# Tâche 02 — Implémentation du parser Markdown

## Scope

Implémenter `MarkdownParser` dans le crate `parser`. Cette tâche suppose que l'AST (tâche 01) est déjà en place. Elle ne touche **ni `converter`, ni `service`**.

**Référence** : [`03-parser.md`](../03-parser.md), [`03a-parser-markdown.md`](../03a-parser-markdown.md), [`02-ast-nodes.md`](../02-ast-nodes.md)

---

## Fichiers à créer / modifier

```
parser/src/
├── lib.rs           ← ajouter : pub mod markdown; pub mod id; pub mod inline;
├── id.rs            ← générateur d'ID auto-incrémenté
├── inline.rs        ← parse_inline() partagé Markdown/HTML
└── markdown.rs      ← MarkdownParser + impl Parser
```

---

## `parser/src/id.rs`

Générateur d'identifiants uniques pour les nœuds.

```rust
pub struct NodeIdGen(u64);

impl NodeIdGen {
    pub fn new() -> Self { Self(0) }
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}
```

---

## Algorithme `MarkdownParser`

### Passe 1 — détection de bloc (ligne par ligne)

Pour chaque ligne de l'input :

| Condition | Action |
|---|---|
| Ligne vide ou espaces seuls | Skip |
| Commence par `# ` (1–6 `#`) | → `ParseError::UnsupportedSymbol` immédiat |
| Commence par `> ` | → bloc `Blockquote`, `parse_inline` sur le reste |
| Commence par `- ` ou `* ` | → accumuler items dans un bloc `List` |
| Commence par `N. ` (digit suivi de `.`) | → accumuler items dans un `OrderedList` |
| Sinon | → bloc `Text`, `parse_inline` sur la ligne entière |

Les items de liste (`- item`) sont accumulés jusqu'à ce qu'une ligne ne commence plus par le même préfixe — alors le bloc `List`/`OrderedList` est finalisé.

### Passe 2 — `parse_inline(line, id_gen) -> Result<Vec<InlineNode>, ParseError>`

Détection des marqueurs dans l'ordre : **du plus long au plus court**.

Ordre de priorité :
1. `**` → `ContainerType::Bold`
2. `~~` → `ContainerType::Strikethrough`
3. `==` → `ContainerType::Surline`
4. `*` ou `_` → `ContainerType::Italic`

Algorithme état par état :

```
current_text = ""
i = 0

tant que i < len(chars):
    si chars[i..] commence par un marqueur connu M :
        si current_text non vide → flush en TextNode(Text)
        chercher marqueur fermant M à partir de i + len(M)
        si non trouvé → ParseError::UnsupportedSymbol (marqueur non fermé)
        extraire contenu entre les deux marqueurs
        appeler parse_inline() récursivement sur ce contenu
        créer ContainerNode(type de M, enfants = résultat récursif)
        avancer i après le marqueur fermant
    sinon si chars[i] est un symbole non-ASCII non reconnu :
        → ParseError::UnsupportedSymbol
    sinon :
        current_text += chars[i]
        i += 1

si current_text non vide → flush en TextNode(Text)
```

`flush_text()` utilise `std::mem::take(&mut current_text)` pour éviter le clone.

### Calcul du `skip` après fermeture d'un marqueur

```
skip = (marqueur_len - 1) + position_fermeture + marqueur_len
```

### Symboles non supportés en Markdown

Tout caractère déclencheur non listé ci-dessus (`#`, backtick, `[`, `<`, etc.) génère `ParseError::UnsupportedSymbol` avec la position exacte.

---

## `supported_symbols()` pour `MarkdownParser`

Retourner les 8 entrées suivantes :

| symbol | description | example |
|---|---|---|
| `**` | Gras | `**texte gras**` |
| `*` | Italique | `*texte italique*` |
| `_` | Italique (variante) | `_texte italique_` |
| `~~` | Texte barré | `~~texte barré~~` |
| `==` | Surligné | `==texte surligné==` |
| `> ` | Citation | `> une citation` |
| `- ` | Liste non ordonnée | `- item de liste` |
| `1. ` | Liste ordonnée | `1. premier item` |

---

## Tests à implémenter

Fichier : `parser/src/markdown.rs` (module `#[cfg(test)]`)

### Cas nominaux

```rust
#[test]
fn texte_simple_produit_container_text() {
    // "Bonjour" → [ContainerNode(Text, [TextNode("Bonjour")])]
}

#[test]
fn gras_produit_container_bold() {
    // "**Hello**" → [ContainerNode(Text, [Container(Bold, [Text("Hello")])])]
}

#[test]
fn italique_etoile_produit_container_italic() {
    // "*Nathan*" → Container(Italic, [Text("Nathan")])
}

#[test]
fn italique_underscore_produit_container_italic() {
    // "_Nathan_" → Container(Italic, [Text("Nathan")])
}

#[test]
fn barre_produit_container_strikethrough() {
    // "~~barré~~" → Container(Strikethrough, ...)
}

#[test]
fn surline_produit_container_surline() {
    // "==surligné==" → Container(Surline, ...)
}

#[test]
fn blockquote_produit_container_blockquote() {
    // "> citation" → [ContainerNode(Blockquote, [Text("citation")])]
}

#[test]
fn liste_non_ordonnee_produit_container_list() {
    // "- a\n- b" → [ContainerNode(List, [ListItem([Text("a")]), ListItem([Text("b")])])]
}

#[test]
fn liste_ordonnee_produit_container_ordered_list() {
    // "1. un\n2. deux" → ContainerNode(OrderedList, ...)
}
```

### Imbrication

```rust
#[test]
fn gras_contenant_italique_imbriqué() {
    // "**Bonjour *Nathan*, vas-tu ?**"
    // → ContainerNode(Bold, [
    //     Text("Bonjour "),
    //     Container(Italic, [Text("Nathan")]),
    //     Text(", vas-tu ?")
    //   ])
}

#[test]
fn texte_mixte_gras_et_normal() {
    // "Hello **world**"
    // → ContainerNode(Text, [Text("Hello "), Container(Bold, [Text("world")])])
}
```

### Erreurs

```rust
#[test]
fn diese_produit_erreur_unsupported() {
    let result = MarkdownParser.parse("# Titre");
    assert!(matches!(result, Err(ParseError::UnsupportedSymbol { symbol, .. }) if symbol == "#"));
}

#[test]
fn marqueur_non_ferme_produit_erreur() {
    // "**non fermé" → ParseError::UnsupportedSymbol
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
    let result = MarkdownParser.parse("Hello # monde");
    if let Err(ParseError::UnsupportedSymbol { position, .. }) = result {
        assert_eq!(position.line, 1);
        assert!(position.column > 1);
    } else {
        panic!("Attendu UnsupportedSymbol");
    }
}
```

### Cas limites

```rust
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
    // "données résumé" → Text contenant les caractères accentués tels quels
    let result = MarkdownParser.parse("données résumé").unwrap();
    // le TextNode doit contenir "données résumé" sans modification
}

#[test]
fn supported_symbols_retourne_8_entrees() {
    let symbols = MarkdownParser.supported_symbols();
    assert_eq!(symbols.len(), 8);
}
```

---

## Critère de succès

```bash
cargo test --package parser --lib markdown
```

Tous les tests passent. `cargo check --workspace` reste vert.
