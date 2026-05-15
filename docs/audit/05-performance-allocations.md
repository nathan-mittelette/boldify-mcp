# Audit — Allocations inutiles dans les handlers

**Sévérité**: 🟡 Moyen  
**Crates concernées**: `converter`  
**Fichiers**: `converter/src/handlers/bold.rs` (et italic, strikethrough, underline, surline)

---

## Problème 1 — HashMap initialisée avec des `String` au lieu de `&'static str`

Dans chaque handler qui gère les accents (bold, italic…), la `HashMap` de correspondance est construite avec des clés et valeurs allouées dynamiquement :

```rust
// Exemple dans bold.rs
let mut map = HashMap::new();
map.insert("a".to_string(), "𝐚".to_string());  // 2 allocations par entrée
map.insert("b".to_string(), "𝐛".to_string());
// ... ~99 entrées
```

Avec ~99 entrées (26 min + 26 maj + 10 chiffres + ~37 accents), cela représente **~198 allocations String** au premier accès, pour stocker des littéraux qui pourraient être statiques.

---

## Recommandation 1 — Utiliser `&'static str` comme clés et valeurs

```rust
static BOLD_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn bold_map() -> &'static HashMap<&'static str, &'static str> {
    BOLD_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("a", "𝐚");
        m.insert("b", "𝐛");
        // ...
        m
    })
}
```

Aucune allocation heap pour les clés/valeurs — les littéraux sont dans le segment data du binaire.

---

## Problème 2 — Itérations multiples sur les listes

Dans `converter/src/nodes/container_node.rs` :

```rust
ContainerType::List => self
    .children
    .iter()
    .filter_map(|n| {
        if let InlineNode::ListItem(li) = n {
            Some(format!("• {}", li.to_unicode()))
        } else {
            None
        }
    })
    .collect::<Vec<_>>()  // allocation + 1ère itération complète
    .join("\n"),           // 2ème itération sur le Vec
```

Ce pattern fait **deux passes** sur les données et alloue un `Vec<String>` intermédiaire.

---

## Recommandation 2 — Utiliser `itertools::join` ou une accumulation manuelle

Option A — avec `itertools` (pas de dépendance supplémentaire si déjà présente) :

```rust
use itertools::Itertools;

self.children
    .iter()
    .filter_map(|n| {
        if let InlineNode::ListItem(li) = n {
            Some(format!("• {}", li.to_unicode()))
        } else {
            None
        }
    })
    .join("\n")  // pas de Vec intermédiaire
```

Option B — sans dépendance supplémentaire :

```rust
let mut result = String::new();
let mut first = true;
for child in &self.children {
    if let InlineNode::ListItem(li) = child {
        if !first { result.push('\n'); }
        result.push_str("• ");
        result.push_str(&li.to_unicode());
        first = false;
    }
}
result
```

---

## Problème 3 — Clone inutile sur `SourcePosition`

Dans `parser/src/html.rs` :

```rust
position: unclosed.opened_at.clone(),
```

`SourcePosition` contient uniquement des `usize` (ligne, colonne, offset) et devrait dériver `Copy` :

```rust
// parser/src/ast.rs (ou error.rs)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePosition {
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}
```

Une fois `Copy` dérivé, `.clone()` devient redondant et peut être supprimé partout.

---

## Impact attendu

- Réduction des allocations heap au démarrage (~200 String évitées par handler)
- Suppression d'un `Vec<String>` intermédiaire par rendu de liste
- Code plus lisible et plus proche des bonnes pratiques Rust
