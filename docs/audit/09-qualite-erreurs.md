# Audit — Qualité et cohérence des erreurs

**Sévérité**: 🟡 Moyen  
**Crates concernées**: `service`, `mcp`, `api-convert`, `api-syntaxes`, `parser`  
**Fichiers**: `service/src/error.rs`, `mcp/src/server.rs`, `api-convert/src/main.rs`, `parser/src/error.rs`

---

## Problème 1 — `ServiceError::EmptyContent` défini mais jamais utilisé

```rust
// service/src/error.rs
pub enum ServiceError {
    Parse(ParseError),
    EmptyContent,  // ← Défini mais non utilisé
}
```

Dans `service/src/lib.rs`, la gestion du contenu vide ne produit pas cette erreur — elle retourne `Ok(String::new())`. La variante `EmptyContent` est donc morte. Soit elle doit être utilisée, soit supprimée.

**Recommandation** : Soit faire retourner `Err(ServiceError::EmptyContent)` pour un contenu vide (ce qui force l'appelant à gérer ce cas), soit supprimer la variante.

---

## Problème 2 — Erreurs converties en `String` dans le serveur MCP

```rust
// mcp/src/server.rs
Err(e) => {
    CallToolResult::error(vec![Content::text(format!("Erreur : {}", e))])
}
```

L'erreur est sérialisée en string libre. L'appelant MCP ne peut pas distinguer une erreur de parsing (input invalide) d'une erreur interne. Dans `api-convert`, la réponse JSON est structurée avec un code HTTP distinct.

**Recommandation** : Conserver le message lisible, mais ajouter un champ de type structuré si le protocole MCP le permet. Minima : distinguer erreur client (4xx-like) vs erreur serveur (5xx-like).

---

## Problème 3 — Messages d'erreur qui exposent la structure interne

```rust
// parser/src/error.rs
"Symbole non supporté : `{symbol}` à la position {position}.\n\
 Consultez la liste des syntaxes supportées via :\n\
 - API HTTP : GET /syntaxes\n\
 - MCP CLI  : mcp list"
```

Le message expose les endpoints internes de l'API (`GET /syntaxes`) et la commande CLI. Ce n'est pas un risque majeur ici (le service est censé être accessible), mais c'est à surveiller si le service devient plus privé.

**Recommandation** : Externaliser la liste des endpoints dans une constante, facilite leur mise à jour si les routes changent.

---

## Problème 4 — Pas de logging sur les erreurs (lié à l'audit #07)

Les erreurs sont retournées à l'appelant mais jamais loggées. En production Lambda, un `ParseError::UnclosedTag` disparaît dans le vide si l'appelant ne remonte pas l'erreur.

---

## Problème 5 — `serde_json::to_string` peut échouer silencieusement

```rust
// api-syntaxes/src/main.rs
let body = serde_json::to_string(&symbols)?;  // retourne 500 générique si échoue
```

La sérialisation de `Vec<String>` ne peut pas échouer en pratique, mais l'opérateur `?` propage une erreur vers le runtime Lambda avec un message peu clair. Pas critique, mais mieux vaut expliciter avec `.expect("serialization of Vec<String> cannot fail")` ou gérer l'erreur explicitement.

---

## Recommandation globale — Cohérence des réponses d'erreur

Définir un format d'erreur unique pour toutes les surfaces (Lambda et MCP) :

```json
{
  "error": {
    "code": "UNSUPPORTED_SYMBOL",
    "message": "Symbole non supporté : `#` à la position 1:1",
    "details": { "symbol": "#", "line": 1, "col": 1 }
  }
}
```

Cela facilite :
- Le parsing d'erreurs côté client
- L'ajout de monitoring sur des codes d'erreur spécifiques
- La traduction des messages sans casser les intégrations
