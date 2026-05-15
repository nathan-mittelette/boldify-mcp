# Audit — boldify-mcp

Date : 2026-05-12  
Version auditée : branche `main` (commit `55c7d88`)

---

## Vue d'ensemble

L'architecture est **solide** : séparation parser/converter/service propre, tests bien couverts, CI fonctionnelle. Les améliorations identifiées portent principalement sur la réduction de duplication, la performance des allocations, la sécurité des inputs et l'observabilité en production.

---

## Points positifs

- Architecture modulaire avec dépendances unidirectionnelles
- Tests unitaires et d'intégration couvrant les cas Unicode, accents, emoji
- CI multi-target (cli + http) avec fmt, clippy, tests, build
- Gestion des erreurs contextualisée avec position (ligne, colonne, offset)
- Build de release optimisé (LTO, strip, codegen-units=1)
- Support Unicode étendu (60+ caractères accentués, 12+ langues)

---

## Fichiers d'audit

| # | Fichier | Sévérité | Sujet |
|---|---------|----------|-------|
| 01 | [Duplication handlers](01-duplication-handlers.md) | 🔴 Critique | 30+ lignes dupliquées dans strikethrough/underline/surline |
| 02 | [Duplication API Lambda](02-duplication-api-lambda.md) | 🟠 Haute | `bad_request`, traits inutiles, handlers dupliqués |
| 03 | [Sécurité — limite HTML](03-securite-limite-taille-html.md) | 🔴 Critique | Parser HTML sans protection contre les inputs surdimensionnés |
| 04 | [Complexité parse HTML](04-complexite-parse-html.md) | 🟠 Haute | Fonction `parse()` de 140 lignes, complexité cyclomatique >15 |
| 05 | [Performance — allocations](05-performance-allocations.md) | 🟡 Moyen | HashMap avec String keys, itérations multiples sur les listes |
| 06 | [once_cell obsolète](06-once-cell-obsolete.md) | 🟠 Haute | Remplacer par `std::sync::OnceLock` (Rust 1.80+) |
| 07 | [Logging absent](07-logging-absent.md) | 🟡 Moyen | Aucun log applicatif, impossible de diagnostiquer en production |
| 08 | [Couverture de tests](08-couverture-tests.md) | 🟡 Moyen | Cas manquants : profondeur, concurrence, API malformée |
| 09 | [Qualité des erreurs](09-qualite-erreurs.md) | 🟡 Moyen | `EmptyContent` inutilisé, erreurs inconsistantes MCP vs Lambda |
| 10 | [Dépendances](10-dependances.md) | 🟡 Moyen | `tokio/full` surdimensionné, `rust-version` absent |
| 11 | [Documentation](11-documentation.md) | 🟢 Faible | Modules et traits publics sans doc comments |

---

## Ordre de priorité suggéré

1. **🔴 Sécurité** — Ajouter la limite de taille au parser HTML (audit #03)
2. **🔴 Qualité** — Factoriser les handlers combining (audit #01)
3. **🟠 Modernisation** — Migrer `once_cell` → `OnceLock` (audit #06)
4. **🟠 Architecture** — Refactoriser `parse()` HTML en sous-fonctions (audit #04)
5. **🟠 Maintenance** — Factoriser le code Lambda partagé (audit #02)
6. **🟡 Performance** — Optimiser les allocations HashMap (audit #05)
7. **🟡 Observabilité** — Ajouter `tracing` (audit #07)
8. **🟡 Robustesse** — Compléter la couverture de tests (audit #08)
9. **🟡 Cohérence** — Harmoniser les erreurs (audit #09)
10. **🟡 Deps** — Nettoyer les dépendances (audit #10)
11. **🟢 Docs** — Documenter les interfaces publiques (audit #11)
