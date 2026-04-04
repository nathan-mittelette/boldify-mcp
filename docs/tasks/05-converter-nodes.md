# Tâche 05 — Converter : ToUnicode sur les nœuds AST

## Scope

Implémenter le trait `ToUnicode` et son dispatch sur les types AST du crate `converter`. Cette tâche suppose que les handlers (tâche 04) et l'AST (tâche 01) sont en place. Elle ne touche **ni le crate `parser` directement** (il est seulement importé en lecture), **ni `service`**.

**Référence** : [`04-converter.md`](../04-converter.md), [`02-ast-nodes.md`](../02-ast-nodes.md)

---

## Fichiers à créer

```
converter/src/
├── lib.rs          ← re-exports + fn convert()
├── traits.rs       ← trait ToUnicode
└── nodes/
    ├── mod.rs
    ├── container_node.rs
    ├── inline_node.rs
    ├── list_item_node.rs
    └── text_node.rs
```

---

## `converter/src/traits.rs`

```rust
/// Conversion d'un nœud AST en String Unicode formatée.
/// Le style est déterminé par le `ContainerType` du nœud porteur.
pub trait ToUnicode {
    fn to_unicode(&self) -> String;
}
```

---

## `converter/src/nodes/text_node.rs`

Un `TextNode` retourne son texte brut — c'est le `ContainerNode` parent qui applique le style.

```rust
use parser::TextNode;
use crate::traits::ToUnicode;

impl ToUnicode for TextNode {
    fn to_unicode(&self) -> String {
        self.text.clone()
    }
}
```

---

## `converter/src/nodes/inline_node.rs`

Dispatch sur les trois variantes d'`InlineNode`.

```rust
use parser::InlineNode;
use crate::traits::ToUnicode;

pub fn inline_to_unicode(node: &InlineNode) -> String {
    match node {
        InlineNode::Text(n)      => n.to_unicode(),
        InlineNode::Container(n) => n.to_unicode(),
        InlineNode::ListItem(n)  => n.to_unicode(),
    }
}
```

---

## `converter/src/nodes/list_item_node.rs`

```rust
use parser::ListItemNode;
use crate::traits::ToUnicode;
use crate::nodes::inline_node::inline_to_unicode;

impl ToUnicode for ListItemNode {
    fn to_unicode(&self) -> String {
        self.children.iter().map(inline_to_unicode).collect()
    }
}
```

---

## `converter/src/nodes/container_node.rs`

C'est ici que le style est appliqué en fonction du `ContainerType`.

```rust
use parser::{ContainerNode, ContainerType};
use crate::traits::ToUnicode;
use crate::nodes::inline_node::inline_to_unicode;
use crate::handlers::{bold_handler, italic_handler, underline_handler,
                       strikethrough_handler, surline_handler};

impl ToUnicode for ContainerNode {
    fn to_unicode(&self) -> String {
        // 1. Convertir récursivement tous les enfants en texte brut
        let raw: String = self.children.iter().map(inline_to_unicode).collect();

        // 2. Appliquer le style selon le ContainerType
        match &self.container_type {
            ContainerType::Text        => raw,
            ContainerType::Bold        => bold_handler().apply(&raw),
            ContainerType::Italic      => italic_handler().apply(&raw),
            ContainerType::Underline   => underline_handler().apply(&raw),
            ContainerType::Strikethrough => strikethrough_handler().apply(&raw),
            ContainerType::Surline     => surline_handler().apply(&raw),
            ContainerType::Blockquote  => format!("❝ {} ❞", raw),
            ContainerType::List => {
                self.children.iter()
                    .filter_map(|n| {
                        if let parser::InlineNode::ListItem(li) = n {
                            Some(format!("• {}", li.to_unicode()))
                        } else { None }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            ContainerType::OrderedList => {
                self.children.iter()
                    .enumerate()
                    .filter_map(|(i, n)| {
                        if let parser::InlineNode::ListItem(li) = n {
                            Some(format!("{}. {}", i + 1, li.to_unicode()))
                        } else { None }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}
```

> **Note** : `ContainerType::Heading` n'existe pas — rejeté par le parser. Pas de `_ => unreachable!()` : le match doit être exhaustif sur les variants réels.

---

## `converter/src/lib.rs`

Fonction publique d'entrée utilisée par `service`.

```rust
pub mod traits;
pub mod handlers;
pub mod nodes;

use parser::ContainerNode;
use traits::ToUnicode;

/// Convertit une liste de nœuds racines en String Unicode.
pub fn convert(nodes: &[ContainerNode]) -> String {
    nodes.iter()
        .map(|n| n.to_unicode())
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Tests à implémenter

Les tests de cette couche **ne font pas appel au parser**. Les nœuds AST sont construits manuellement.

Fichier : `converter/src/nodes/container_node.rs` (module `#[cfg(test)]`)

### Helper de construction

```rust
#[cfg(test)]
mod tests {
    use parser::ast::*;

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
```

### `ContainerType::Text`

```rust
    #[test]
    fn text_retourne_le_texte_brut() {
        let node = make_container(ContainerType::Text, vec![make_text("Bonjour")]);
        assert_eq!(node.to_unicode(), "Bonjour");
    }
```

### `ContainerType::Bold`

```rust
    #[test]
    fn bold_convertit_ascii_en_unicode_gras() {
        let node = make_container(ContainerType::Bold, vec![make_text("ABC")]);
        let result = node.to_unicode();
        assert!(!result.contains("ABC")); // pas d'ASCII
        assert_eq!(result.chars().count(), 3);
    }

    #[test]
    fn bold_preserve_espace() {
        let node = make_container(ContainerType::Bold, vec![make_text("A B")]);
        let result = node.to_unicode();
        assert!(result.contains(' '));
    }

    #[test]
    fn bold_convertit_accent() {
        let node = make_container(ContainerType::Bold, vec![make_text("é")]);
        let result = node.to_unicode();
        assert!(result.contains('\u{0301}'));
        assert!(!result.contains('é'));
    }
```

### `ContainerType::Italic`

```rust
    #[test]
    fn italic_convertit_ascii_en_unicode_italique() {
        let node = make_container(ContainerType::Italic, vec![make_text("Hello")]);
        let result = node.to_unicode();
        assert!(!result.contains("Hello"));
    }
```

### `ContainerType::Blockquote`

```rust
    #[test]
    fn blockquote_encadre_le_texte() {
        let node = make_container(ContainerType::Blockquote, vec![make_text("citation")]);
        let result = node.to_unicode();
        assert!(result.contains("citation"));
        assert!(result.contains('❝'));
        assert!(result.contains('❞'));
    }
```

### `ContainerType::List`

```rust
    #[test]
    fn list_prefixe_chaque_item_avec_puce() {
        let li_a = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![make_text("alpha")],
        });
        let li_b = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(1, Span::new(0, 0)),
            children: vec![make_text("beta")],
        });
        let node = make_container(ContainerType::List, vec![li_a, li_b]);
        let result = node.to_unicode();
        assert!(result.contains("• alpha"));
        assert!(result.contains("• beta"));
    }
```

### `ContainerType::OrderedList`

```rust
    #[test]
    fn ordered_list_numerote_les_items() {
        let li = InlineNode::ListItem(ListItemNode {
            base: NodeBase::new(0, Span::new(0, 0)),
            children: vec![make_text("premier")],
        });
        let node = make_container(ContainerType::OrderedList, vec![li]);
        let result = node.to_unicode();
        assert!(result.contains("1. premier"));
    }
```

### Imbrication

```rust
    #[test]
    fn bold_contenant_italic_applique_les_deux_styles() {
        // ContainerNode(Bold, [Container(Italic, [Text("Nathan")])])
        let italic_child = InlineNode::Container(
            make_container(ContainerType::Italic, vec![make_text("Nathan")])
        );
        let bold_node = make_container(ContainerType::Bold, vec![italic_child]);
        let result = bold_node.to_unicode();
        // Le texte doit être transformé (ni "Nathan" ASCII, ni italique pur)
        assert!(!result.contains("Nathan"));
    }
```

### `convert()` fonction publique

```rust
// converter/src/lib.rs tests
    #[test]
    fn convert_plusieurs_nodes_joint_avec_newline() {
        let n1 = make_container(ContainerType::Text, vec![make_text("ligne 1")]);
        let n2 = make_container(ContainerType::Text, vec![make_text("ligne 2")]);
        let result = convert(&[n1, n2]);
        assert!(result.contains("ligne 1\nligne 2"));
    }

    #[test]
    fn convert_vec_vide_retourne_chaine_vide() {
        let result = convert(&[]);
        assert_eq!(result, "");
    }
```

---

## Critère de succès

```bash
cargo test --package converter
```

Tous les tests passent. `cargo check --workspace` reste vert.
