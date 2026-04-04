# Parser Markdown — Algorithme détaillé

## Vue d'ensemble

Le parser Markdown fonctionne en **deux passes** :

1. **Passe bloc** (`MarkdownParser::parse`) : lit le texte ligne par ligne et identifie le type de bloc de chaque ligne (`Heading`, `Blockquote`, `List`, `OrderedList`, `Text`).
2. **Passe inline** (`parse_inline`) : pour le contenu de chaque ligne, parcourt les caractères et construit l'arbre de `InlineNode` en détectant les marqueurs de style (`**`, `*`, `~~`, `==`).

Les deux passes sont récursives uniquement au niveau inline (styles imbriqués).

---

## Passe 1 : détection des blocs

### Flux général

```
Pour chaque ligne du texte source :
  │
  ├─ Ligne vide ?          → ignorer, passer à la suivante
  │
  ├─ Commence par `#` ?
  │   ├─ Compte les `#` → level (1–6)
  │   ├─ level <= 6      → ContainerNode(Heading(level))
  │   │                    contenu = reste de la ligne après les `#`
  │   │                    children = parse_inline(contenu)
  │   └─ level > 6       → Err(UnsupportedSymbol)
  │
  ├─ Commence par `> ` ?  → ContainerNode(Blockquote)
  │                          contenu = reste après `> `
  │                          children = parse_inline(contenu)
  │
  ├─ Commence par `- ` ou `* ` ?
  │                        → ContainerNode(List)
  │                          children = [ ListItem { children = parse_inline(contenu) } ]
  │
  ├─ Commence par `N. ` (N = chiffres) ?
  │                        → ContainerNode(OrderedList)
  │                          children = [ ListItem { children = parse_inline(contenu) } ]
  │
  └─ Sinon               → ContainerNode(Text)
                            children = parse_inline(ligne entière)
```

### Gestion des listes multi-lignes

En Markdown, plusieurs lignes consécutives `- item` sont des items distincts, pas un seul bloc liste. Chaque ligne produit son propre `ContainerNode(List)`. Le converter est responsable de les afficher de manière cohérente.

```
Entrée :
  - item A
  - item B
  - item C

Sortie :
  ContainerNode(List) > [ListItem > [Text("item A")]]
  ContainerNode(List) > [ListItem > [Text("item B")]]
  ContainerNode(List) > [ListItem > [Text("item C")]]
```

> Cette approche simplifie le parser (pas de fusion de lignes) au prix d'une légère perte de sémantique. Si la fusion devient nécessaire, elle sera gérée dans le converter.

### Détection du préfixe de liste ordonnée

Un préfixe `N. ` est valide si :
- Il y a un `.` suivi d'un espace dans la ligne.
- Tout ce qui précède le `.` est un entier (`chars().all(is_ascii_digit())`).
- Le préfixe n'est pas vide.

```
"1. item"   → valide  (prefix="1", content="item")
"42. item"  → valide  (prefix="42", content="item")
"1.item"    → invalide (pas d'espace après le point)
". item"    → invalide (prefix vide)
"a. item"   → invalide (prefix non numérique)
```

---

## Passe 2 : parsing inline

### Principe

`parse_inline` reçoit une chaîne (le contenu d'une ligne ou d'un fragment imbriqué) et produit `Vec<InlineNode>`. Elle maintient un **tampon de texte courant** (`current_text`) qui est vidé ("flush") à chaque fois qu'un marqueur est détecté.

### État interne

```
current_text : String       → accumule les caractères sans marqueur
current_text_start : usize  → position de début du tampon dans la source
chars : Peekable<CharIndices> → itérateur sur les caractères
nodes : Vec<InlineNode>     → résultat en construction
```

### Flux caractère par caractère

```
Pour chaque caractère (i, c) :
  │
  ├─ Le reste de la chaîne commence par un marqueur connu ?
  │   (testé du plus long au plus court : "**" avant "*")
  │   │
  │   ├─ OUI :
  │   │   1. Flush current_text → InlineNode::Text si non vide
  │   │   2. Cherche le marqueur FERMANT dans le reste de la chaîne
  │   │   │
  │   │   ├─ Trouvé à position close_pos :
  │   │   │   - inner = chaîne entre marqueur ouvrant et fermant
  │   │   │   - children = parse_inline(inner)  ← récursion
  │   │   │   - Pousse InlineNode::Container(ContainerNode { type, children })
  │   │   │   - Avance l'itérateur jusqu'après le marqueur fermant
  │   │   │
  │   │   └─ Pas trouvé :
  │   │       → Err(UnsupportedSymbol { symbol: marqueur, position })
  │   │
  └─ NON :
       ├─ c == '`' ou c == '<' → Err(UnsupportedSymbol)
       ├─ c == '\n'            → current_text.push('\n')
       └─ sinon                → current_text.push(c)

Fin de boucle :
  Flush current_text restant → InlineNode::Text si non vide
```

### Ordre de détection des marqueurs

Les marqueurs sont testés **du plus long au plus court** pour éviter les ambiguïtés :

```rust
const MARKDOWN_MARKERS: &[(&str, ContainerType)] = &[
    ("**", ContainerType::Bold),         // testé avant "*"
    ("*",  ContainerType::Italic),
    ("_",  ContainerType::Italic),
    ("~~", ContainerType::Strikethrough),
    ("==", ContainerType::Surline),
];
```

Si on testait `*` avant `**`, le texte `**gras**` serait interprété comme
`Italic("") + texte "gras" + Italic("")` au lieu de `Bold("gras")`.

### Exemple pas à pas : `**Bonjour *Nathan*, vas-tu ?**`

```
Position 0 : reste = "**Bonjour *Nathan*, vas-tu ?**"
  → détecte "**" (Bold)
  → cherche "**" fermant → trouvé à position 28 (après "?")
  → inner = "Bonjour *Nathan*, vas-tu ?"
  → récursion parse_inline("Bonjour *Nathan*, vas-tu ?")

    Récursion :
    Position 0–7 : "Bonjour " → aucun marqueur → current_text = "Bonjour "
    Position 8 : reste = "*Nathan*, vas-tu ?"
      → détecte "*" (Italic)
      → flush "Bonjour " → Text("Bonjour ")
      → cherche "*" fermant → trouvé à position 7 ("Nathan,")
                              Attention : find("*") cherche dans "*Nathan*, vas-tu ?"
                              → position 7 = le "*" après "Nathan"
      → inner = "Nathan"
      → récursion parse_inline("Nathan") → [Text("Nathan")]
      → pousse Container(Italic) > [Text("Nathan")]
      → avance après le "*" fermant
    Position 16–27 : ", vas-tu ?" → current_text = ", vas-tu ?"
    Fin : flush → Text(", vas-tu ?")

    Résultat récursion : [Text("Bonjour "), Container(Italic)[Text("Nathan")], Text(", vas-tu ?")]

→ pousse Container(Bold) > [Text("Bonjour "), Container(Italic)[Text("Nathan")], Text(", vas-tu ?")]

Résultat final :
ContainerNode(Bold) {
  children: [
    Text("Bonjour "),
    ContainerNode(Italic) {
      children: [Text("Nathan")]
    },
    Text(", vas-tu ?")
  ]
}
```

### Avancement de l'itérateur après un marqueur fermant

Après avoir consommé `marker_ouvrant + inner + marker_fermant`, l'itérateur `chars` doit sauter exactement :

```
skip = (marker_len - 1)   // -1 car le char courant (premier char du marqueur) est déjà consommé
     + close_rel           // longueur de inner
     + marker_len          // marqueur fermant
```

Exemple pour `**gras**` avec `marker_len = 2`, `close_rel = 4` ("gras") :
```
skip = (2 - 1) + 4 + 2 = 7
chars consommés : *, g, r, a, s, *, *  → 7 chars ✓
```

### Flush du tampon de texte

```rust
fn flush_text(
    nodes: &mut Vec<InlineNode>,
    text: &mut String,
    start: usize,
    end: usize,
    id_gen: &mut IdGenerator,
) {
    if !text.is_empty() {
        nodes.push(InlineNode::Text(TextNode {
            base: NodeBase::new(id_gen.next(), Span::new(start, end)),
            text: std::mem::take(text),  // vide le tampon en une opération
        }));
    }
}
```

`std::mem::take` est préféré à `text.clone(); text.clear()` car il ne fait pas d'allocation supplémentaire.

---

## Cas limites et comportements définis

### Marqueur non fermé

```
"**non fermé"
  → détecte "**" à position 0
  → cherche "**" fermant → non trouvé
  → Err(UnsupportedSymbol { symbol: "**", line: N, column: 1 })
```

Le parser ne fait **pas** de fallback texte brut : un marqueur ouvert non fermé est toujours une erreur.

### Marqueurs imbriqués du même type

```
"**a **b** c**"
  → détecte "**" ouvrant à position 0
  → cherche "**" fermant → trouvé à position 5 (avant "b")
  → inner = "a "
  → Container(Bold) > [Text("a ")]
  → reste = "b** c**"
  → pas de marqueur ouvrant au début → current_text = "b"
  → ...
```

Le `find` simple retourne la **première** occurrence du marqueur fermant.
L'imbrication `**…**…**` n'est donc pas supportée pour le même type — c'est cohérent
avec le comportement de la majorité des parsers Markdown.

### Ligne de titre sans espace après les `#`

```
"##Titre sans espace"
  → level = 2
  → content = trimmed[2..].trim() = "Titre sans espace"  ← le trim() absorbe l'absence d'espace
```

Le trim() sur le contenu rend l'espace optionnel.

### Ligne `>` sans espace (citation vide)

```
">"
  → strip_prefix("> ") → None  (l'espace est requis)
  → traité comme ContainerNode(Text) > [Text(">")]
```

Une citation valide requiert `"> "` (chevron + espace).

---

## Invariants garantis par le parser Markdown

1. Tout `ContainerNode(List)` ou `ContainerNode(OrderedList)` a exactement un `InlineNode::ListItem` enfant.
2. Tout `InlineNode::ListItem` a au moins un `InlineNode` enfant (vide si l'item est `"- "`).
3. Tout `InlineNode::Container` a un `ContainerType` parmi `{Bold, Italic, Strikethrough, Surline}` — jamais `Text`, `Heading`, `Blockquote`, `List`, `OrderedList`.
4. Les `TextNode` ne contiennent jamais de marqueurs de style (`**`, `*`, etc.) — ils ont été consommés par le parser.
5. Un `\n` dans un `TextNode` provient uniquement d'un saut de ligne explicite dans la source (pas d'un `<br>`).
