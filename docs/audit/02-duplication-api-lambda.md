# Audit — Duplication entre les crates API Lambda

**Sévérité**: 🟠 Haute  
**Crates concernées**: `api-convert`, `api-syntaxes`  
**Fichiers**: `api-convert/src/main.rs`, `api-syntaxes/src/main.rs`

---

## Problème

Les deux crates Lambda partagent du code quasi-identique jamais factorisé :

### 1. Fonction `bad_request` — dupliquée à l'identique

```rust
// api-convert/src/main.rs
fn bad_request(msg: &str) -> Response<String> {
    Response::builder()
        .status(400)
        .body(msg.to_string())
        .unwrap()
}

// api-syntaxes/src/main.rs — copie exacte
fn bad_request(msg: &str) -> Response<String> {
    Response::builder()
        .status(400)
        .body(msg.to_string())
        .unwrap()
}
```

### 2. Pattern `handler_generic` / `handler` — logique d'extraction similaire

Les deux fichiers ont un handler principal et un wrapper générique qui ne sert qu'à l'injection de dépendance pour les tests, avec la même structure boilerplate.

### 3. Traits de service inutiles

`api-convert` définit `ConvertService` et `api-syntaxes` définit `SyntaxesService` — des traits à une seule méthode qui wrappent directement `ContentService`. Ces traits n'apportent aucune valeur réelle dans le contexte Lambda.

---

## Recommandation

### Option A — Crate partagée `api-shared`

Créer une crate `api-shared` (ou `lambda-utils`) qui expose :

```rust
// api-shared/src/lib.rs
use lambda_http::{Response, Body};

pub fn bad_request(msg: &str) -> Response<String> {
    Response::builder()
        .status(400)
        .body(msg.to_string())
        .unwrap()
}

pub fn internal_error(msg: &str) -> Response<String> {
    Response::builder()
        .status(500)
        .body(msg.to_string())
        .unwrap()
}

pub fn ok_json(body: String) -> Response<String> {
    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(body)
        .unwrap()
}
```

### Option B — Module dans `service`

Ajouter un feature flag `lambda` à la crate `service` qui expose ces helpers. Plus simple, évite une crate supplémentaire.

### Suppression des traits inutiles

Remplacer :

```rust
// Avant
trait ConvertService {
    fn convert(&self, content: &str, syntax: &str) -> Result<String, ServiceError>;
}
impl ConvertService for ContentService { ... }
async fn handler_generic<S: ConvertService>(event: Request, svc: &S) -> Response<String> { ... }
```

Par un appel direct :

```rust
// Après
async fn handler(event: Request, svc: &ContentService) -> Response<String> { ... }
```

---

## Impact attendu

- Suppression de ~40 lignes dupliquées
- Comportement HTTP cohérent entre les deux Lambdas
- Moins de surface à maintenir lors d'un changement de format de réponse
