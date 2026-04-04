# Parser HTML — Algorithme détaillé

## Vue d'ensemble

Le parser HTML fonctionne **caractère par caractère**, exactement comme le parser Markdown. Pas de dépendance externe : un seul itérateur sur les caractères, une stack explicite pour gérer l'imbrication des tags.

La stack remplace la récursion : quand on ouvre un tag, on empile un contexte. Quand on ferme un tag, on dépile et on rattache les enfants collectés au niveau parent.

---

## Structures internes

### `OpenTag` — contexte d'un tag ouvert

```rust
/// Contexte empilé lors de l'ouverture d'un tag.
struct OpenTag {
    /// Type de conteneur correspondant au tag.
    container_type: ContainerType,
    /// Enfants collectés depuis l'ouverture du tag.
    children: Vec<InlineNode>,
    /// Position dans la source (pour les erreurs).
    position: SourcePosition,
}
```

### La stack

```rust
// Contenu de la stack au fil du parsing de :
// "<strong>Bonjour <em>Nathan</em> !</strong>"
//
// Début :
//   stack = []
//   current_text = ""
//
// Après "<strong>" :
//   stack = [ OpenTag { type: Bold, children: [] } ]
//   current_text = ""
//
// Après "Bonjour " :
//   stack = [ OpenTag { type: Bold, children: [] } ]
//   current_text = "Bonjour "
//
// Après "<em>" :
//   stack = [ OpenTag { type: Bold, children: [Text("Bonjour ")] },
//             OpenTag { type: Italic, children: [] } ]
//   current_text = ""
//
// Après "Nathan" :
//   stack = [ OpenTag { type: Bold, children: [Text("Bonjour ")] },
//             OpenTag { type: Italic, children: [] } ]
//   current_text = "Nathan"
//
// Après "</em>" :
//   dépile Italic, flush "Nathan", children = [Text("Nathan")]
//   → InlineNode::Container(Italic)[Text("Nathan")]
//   → rattaché au Bold du dessus
//   stack = [ OpenTag { type: Bold, children: [Text("Bonjour "), Container(Italic)[Text("Nathan")]] } ]
//   current_text = ""
//
// Après " !" :
//   stack = [ OpenTag { type: Bold, children: [...] } ]
//   current_text = " !"
//
// Après "</strong>" :
//   dépile Bold, flush " !", children = [Text("Bonjour "), Container(Italic)[Text("Nathan")], Text(" !")]
//   → InlineNode::Container(Bold)[...]
//   stack = []
```

---

## Algorithme principal

### Flux général

```
stack        : Vec<OpenTag>   → contextes ouverts imbriqués
current_text : String         → tampon de texte courant
current_text_start : usize    → position de début du tampon
nodes        : Vec<InlineNode> → résultat de niveau racine
line         : usize          → numéro de ligne courant (pour les erreurs)
line_start   : usize          → offset du début de la ligne courante

Pour chaque caractère (i, c) :
  │
  ├─ c == '\n'
  │   → current_text.push('\n')
  │   → line++, line_start = i + 1
  │
  ├─ c == '<'
  │   → extrait le tag depuis input[i..]
  │   │
  │   ├─ Tag fermant (commence par '/') ?
  │   │   → flush current_text dans le niveau courant (stack.last ou nodes)
  │   │   → tag_name = nom extrait
  │   │   │
  │   │   ├─ tag_name ∉ SUPPORTED_TAGS
  │   │   │   → Err(UnsupportedSymbol { symbol: "</tag>", position })
  │   │   │
  │   │   ├─ "br", "p" → pas de dépilage (void ou transparent, géré à l'ouverture)
  │   │   │   "br" → déjà traité à l'ouverture (void tag)
  │   │   │   "p"  → pousse Text("\n") dans le niveau courant
  │   │   │
  │   │   ├─ stack vide → Err(UnsupportedSymbol) ← tag fermant sans ouvrant
  │   │   │
  │   │   └─ tag correspond au sommet de la stack ?
  │   │       ├─ OUI → dépile, crée ContainerNode ou ListItem selon le type
  │   │       │         → pousse dans le niveau parent (stack.last ou nodes)
  │   │       └─ NON → Err(UnsupportedSymbol) ← fermeture mal imbriquée
  │   │
  │   └─ Tag ouvrant :
  │       → flush current_text dans le niveau courant
  │       → tag_name = nom extrait
  │       │
  │       ├─ tag_name ∉ SUPPORTED_TAGS
  │       │   → Err(UnsupportedSymbol { symbol: "<tag>", position })
  │       │
  │       ├─ "br" → void tag, pas d'empilement
  │       │         pousse Text("\n") dans le niveau courant
  │       │
  │       ├─ "p"  → transparent, pas d'empilement
  │       │         (le </p> ajoutera le \n)
  │       │
  │       └─ sinon → empile OpenTag { container_type, children: [], position }
  │
  └─ sinon → current_text.push(c)

Fin de boucle :
  flush current_text restant dans nodes
  stack non vide → Err(UnsupportedSymbol) ← tag ouvert non fermé
```

### Détermination du "niveau courant"

À tout moment, le niveau courant est soit :
- `stack.last_mut().children` si la stack est non vide.
- `nodes` (le résultat racine) si la stack est vide.

```rust
fn current_children<'a>(
    stack: &'a mut Vec<OpenTag>,
    nodes: &'a mut Vec<InlineNode>,
) -> &'a mut Vec<InlineNode> {
    if let Some(top) = stack.last_mut() {
        &mut top.children
    } else {
        nodes
    }
}
```

### Flush du tampon de texte

```rust
fn flush_text(
    text: &mut String,
    start: &mut usize,
    end: usize,
    target: &mut Vec<InlineNode>,
    id_gen: &mut IdGenerator,
) {
    if !text.trim().is_empty() {  // ignore le whitespace inter-balises
        target.push(InlineNode::Text(TextNode {
            base: NodeBase::new(id_gen.next(), Span::new(*start, end)),
            text: std::mem::take(text),
        }));
    } else {
        text.clear();
    }
    *start = end;
}
```

> **Note** : contrairement au Markdown, le whitespace inter-balises (`"\n  "`, `"\t"`) est filtré via `trim().is_empty()`. Un espace significatif comme `"mot "` avant une balise ouvrante n'est **pas** filtré car `"mot ".trim()` = `"mot"` ≠ `""`.

---

## Extraction d'un tag depuis la source brute

```rust
/// Extrait le nom et la nature (ouvrant/fermant/void) d'un tag HTML
/// à partir de la position du '<' dans la source.
///
/// Retourne (is_closing, tag_name, longueur_totale_du_tag).
fn extract_tag(input: &str, from: usize) -> Option<(bool, String, usize)> {
    let rest = &input[from + 1..]; // skip '<'

    // Commentaire <!-- --> ou DOCTYPE → ignorer
    if rest.starts_with('!') || rest.starts_with('?') {
        let end = input[from..].find('>').map(|i| from + i + 1)?;
        return None; // signale "à ignorer", l'appelant avance jusqu'à end
    }

    let is_closing = rest.starts_with('/');
    let name_start = if is_closing { 1 } else { 0 };

    let tag_name: String = rest[name_start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();

    if tag_name.is_empty() {
        return None;
    }

    // Avance jusqu'au '>' pour connaître la longueur totale du tag
    let tag_end = input[from..].find('>')?;
    let total_len = tag_end + 1; // inclut le '>'

    Some((is_closing, tag_name.to_lowercase(), total_len))
}
```

---

## Dépilage et création du nœud

```rust
fn pop_tag(
    stack: &mut Vec<OpenTag>,
    tag_name: &str,
    nodes: &mut Vec<InlineNode>,
    id_gen: &mut IdGenerator,
    position: &SourcePosition,
) -> Result<(), ParseError> {
    let top = stack.last().ok_or_else(|| ParseError::UnsupportedSymbol {
        symbol: format!("</{}>", tag_name),
        position: position.clone(),
    })?;

    // Vérifie que le tag fermant correspond bien au sommet
    let top_name = container_type_to_tag_name(&top.container_type);
    if top_name != tag_name {
        return Err(ParseError::UnsupportedSymbol {
            symbol: format!("</{}>", tag_name),
            position: position.clone(),
        });
    }

    let open_tag = stack.pop().unwrap();
    let node = build_node(open_tag, id_gen);

    // Rattache au niveau parent
    current_children(stack, nodes).push(node);
    Ok(())
}

fn build_node(open_tag: OpenTag, id_gen: &mut IdGenerator) -> InlineNode {
    match open_tag.container_type {
        ContainerType::List | ContainerType::OrderedList => {
            // Les enfants sont des ListItem → ContainerNode bloc
            InlineNode::Container(ContainerNode {
                base: NodeBase::new(id_gen.next(), Span::new(0, 0)),
                container_type: open_tag.container_type,
                children: open_tag.children,
            })
        }
        _ => {
            InlineNode::Container(ContainerNode {
                base: NodeBase::new(id_gen.next(), Span::new(0, 0)),
                container_type: open_tag.container_type,
                children: open_tag.children,
            })
        }
    }
}
```

---

## Correspondance tag HTML → `ContainerType`

```rust
fn tag_to_container_type(tag: &str) -> Option<ContainerType> {
    match tag {
        "strong" | "b"     => Some(ContainerType::Bold),
        "em" | "i"         => Some(ContainerType::Italic),
        "u"                => Some(ContainerType::Underline),
        "s" | "del"        => Some(ContainerType::Strikethrough),
        "mark"             => Some(ContainerType::Surline),
        "h1"               => Some(ContainerType::Heading(1)),
        "h2"               => Some(ContainerType::Heading(2)),
        "h3"               => Some(ContainerType::Heading(3)),
        "h4"               => Some(ContainerType::Heading(4)),
        "h5"               => Some(ContainerType::Heading(5)),
        "h6"               => Some(ContainerType::Heading(6)),
        "blockquote"       => Some(ContainerType::Blockquote),
        "ul"               => Some(ContainerType::List),
        "ol"               => Some(ContainerType::OrderedList),
        "li"               => Some(ContainerType::ListItem),
        "br" | "p"         => None, // gérés spécifiquement, pas empilés
        _                  => None, // ne devrait pas arriver (validé en amont)
    }
}

fn container_type_to_tag_name(ct: &ContainerType) -> &'static str {
    match ct {
        ContainerType::Bold            => "strong", // ou "b", on choisit le canonique
        ContainerType::Italic          => "em",
        ContainerType::Underline       => "u",
        ContainerType::Strikethrough   => "s",
        ContainerType::Surline         => "mark",
        ContainerType::Heading(1)      => "h1",
        ContainerType::Heading(2)      => "h2",
        ContainerType::Heading(3)      => "h3",
        ContainerType::Heading(4)      => "h4",
        ContainerType::Heading(5)      => "h5",
        ContainerType::Heading(6)      => "h6",
        ContainerType::Blockquote      => "blockquote",
        ContainerType::List            => "ul",
        ContainerType::OrderedList     => "ol",
        ContainerType::ListItem        => "li",
        ContainerType::Text            => "p",
        _                              => "",
    }
}
```

> **Problème des alias** : `<b>` et `<strong>` produisent tous les deux `ContainerType::Bold`. Lors du dépilage, `container_type_to_tag_name` retourne `"strong"`. Si l'utilisateur a écrit `<b>texte</b>`, le tag fermant sera `"b"` ≠ `"strong"` → erreur de fermeture mal imbriquée.
>
> **Solution** : stocker le tag_name original dans `OpenTag` pour la vérification de fermeture.

```rust
struct OpenTag {
    container_type: ContainerType,
    /// Nom du tag tel qu'écrit dans la source (ex: "b", "strong", "em").
    /// Utilisé pour vérifier la cohérence du tag fermant.
    tag_name: String,
    children: Vec<InlineNode>,
    position: SourcePosition,
}

// Vérification lors du dépilage :
if top.tag_name != tag_name {
    return Err(ParseError::UnsupportedSymbol { ... });
}
```

---

## Exemple complet pas à pas

### Source : `<ul><li>item <strong>A</strong></li><li>item B</li></ul>`

```
i=0  : '<' → extract_tag → ("ul", ouvrant)
           → tag_to_container_type("ul") = List
           → empile OpenTag { type: List, tag: "ul", children: [] }
           stack = [List[]]

i=4  : '<' → extract_tag → ("li", ouvrant)
           → empile OpenTag { type: ListItem, tag: "li", children: [] }
           stack = [List[], ListItem[]]

i=8  : 'i' → current_text = "i"
i=9  : 't' → current_text = "it"
i=10 : 'e' → current_text = "ite"
i=11 : 'm' → current_text = "item"
i=12 : ' ' → current_text = "item "

i=13 : '<' → extract_tag → ("strong", ouvrant)
           → flush "item " → ListItem.children = [Text("item ")]
           → empile OpenTag { type: Bold, tag: "strong", children: [] }
           stack = [List[], ListItem[Text("item ")], Bold[]]

i=21 : 'A' → current_text = "A"

i=22 : '<' → extract_tag → ("/strong", fermant)
           → flush "A" → Bold.children = [Text("A")]
           → pop "strong" → top.tag_name == "strong" ✓
           → build_node → Container(Bold)[Text("A")]
           → rattache à ListItem : ListItem.children = [Text("item "), Container(Bold)[Text("A")]]
           stack = [List[], ListItem[Text("item "), Container(Bold)[Text("A")]]]

i=31 : '<' → extract_tag → ("/li", fermant)
           → flush "" (rien) → pas de Text
           → pop "li" → top.tag_name == "li" ✓
           → build_node → Container(ListItem)[Text("item "), Container(Bold)[Text("A")]]
           → rattache à List : List.children = [Container(ListItem)[...]]
           stack = [List[ListItem[Text("item "), Container(Bold)[Text("A")]]]]

i=35 : '<' → extract_tag → ("li", ouvrant)
           → empile OpenTag { type: ListItem, tag: "li", children: [] }
           stack = [List[...], ListItem[]]

i=39..45 : "item B" → current_text = "item B"

i=45 : '<' → extract_tag → ("/li", fermant)
           → flush "item B" → ListItem.children = [Text("item B")]
           → pop "li" → Container(ListItem)[Text("item B")]
           → rattache à List
           stack = [List[ListItem[...], ListItem[Text("item B")]]]

i=50 : '<' → extract_tag → ("/ul", fermant)
           → flush "" → rien
           → pop "ul" → top.tag_name == "ul" ✓
           → build_node → Container(List)[ListItem[...], ListItem[...]]
           → rattache à nodes (stack vide)
           stack = []

Fin :
  stack vide ✓
  nodes = [
    ContainerNode {
      type: List,
      children: [
        ListItem { children: [Text("item "), Container(Bold)[Text("A")]] },
        ListItem { children: [Text("item B")] },
      ]
    }
  ]
```

---

## Cas limites et comportements définis

### Tag inconnu

```
"<div>texte</div>"
  i=0 : '<' → extract_tag → ("div", ouvrant)
  → tag_to_container_type("div") = None
  → "div" ∉ SUPPORTED_TAGS
  → Err(UnsupportedSymbol { symbol: "<div>", line: 1, column: 1 })
```

### Tag ouvert non fermé

```
"<strong>texte"
  Fin de boucle : stack = [Bold[Text("texte")]] non vide
  → Err(UnsupportedSymbol { symbol: "<strong>", position: position_d_ouverture })
```

### Fermeture mal imbriquée

```
"<strong><em>texte</strong></em>"
  Après "texte" : stack = [Bold[], Italic[Text("texte")]]
  '</strong>' → top.tag_name = "em" ≠ "strong"
  → Err(UnsupportedSymbol { symbol: "</strong>", position })
```

### `<br>` void tag

```
"texte<br>suite"
  "texte" → current_text = "texte"
  '<br>' → flush "texte", pousse Text("\n"), avance après '>'
  "suite" → current_text = "suite"
  Fin : flush "suite"
  → [Text("texte"), Text("\n"), Text("suite")]
```

### `<p>` transparent

```
"<p>para <em>italique</em></p>"
  '<p>'  → flush, pas d'empilement (transparent)
  "para " → current_text = "para "
  '<em>' → flush "para ", empile Italic
  "italique" → current_text = "italique"
  '</em>' → flush, pop Italic → Container(Italic)[Text("italique")]
             rattache à nodes
  '</p>' → pousse Text("\n") dans nodes
  → nodes = [Text("para "), Container(Italic)[Text("italique")], Text("\n")]
```

### Commentaire HTML

```
"<!-- commentaire --><strong>texte</strong>"
  '<' → extract_tag → starts_with('!') → None (ignoré)
  → avance l'itérateur jusqu'après '>'
  '<strong>' → traitement normal
```

---

## Invariants garantis par le parser HTML

1. Tout tag rencontré est dans `SUPPORTED_TAGS` — garanti par le rejet immédiat.
2. La stack est vide en fin de parsing — garanti par la vérification finale.
3. Les tags fermants correspondent toujours à leur ouvrant (`tag_name` stocké dans `OpenTag`).
4. `<br>` produit toujours `Text("\n")`, jamais un `ContainerNode`.
5. `<p>` ne produit jamais de `ContainerNode` — son contenu est injecté directement dans le niveau courant.
6. Le whitespace inter-balises (`"\n  "`, `"\t"`) est filtré lors du flush.
7. Les attributs HTML sont ignorés — seul le nom du tag est extrait et vérifié.

---

## `Cargo.toml` du crate `parser`

```toml
[dependencies]
thiserror = "1"

# Pas de dépendance externe pour le parsing :
# HTML et Markdown sont parsés caractère par caractère.
```
