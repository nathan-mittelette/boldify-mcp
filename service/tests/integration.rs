use service::ContentService;

fn svc() -> ContentService {
    ContentService::new()
}

// ── Pipeline Markdown → Unicode ───────────────────────────────────────────────

#[test]
fn markdown_texte_simple_convertit_sans_erreur() {
    let result = svc().convert("markdown", "texte simple");
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn markdown_gras_produit_unicode_different_de_ascii() {
    let result = svc().convert("markdown", "**Bonjour**").unwrap();
    assert!(!result.contains("Bonjour"));
}

#[test]
fn markdown_italique_produit_unicode_different_du_gras() {
    let bold = svc().convert("markdown", "**A**").unwrap();
    let italic = svc().convert("markdown", "*A*").unwrap();
    assert_ne!(bold, italic);
}

#[test]
fn markdown_texte_barre_produit_combining_stroke() {
    let result = svc().convert("markdown", "~~barré~~").unwrap();
    assert!(result.contains('\u{0336}'));
}

#[test]
fn markdown_texte_surline_encadre_avec_guillemets() {
    let result = svc().convert("markdown", "==surligné==").unwrap();
    assert!(result.contains('〚'));
    assert!(result.contains('〛'));
}

#[test]
fn markdown_blockquote_encadre_avec_guillemets_doubles() {
    let result = svc().convert("markdown", "> citation").unwrap();
    assert!(result.contains('❝'));
    assert!(result.contains("citation"));
}

#[test]
fn markdown_liste_produit_puces() {
    let result = svc().convert("markdown", "- alpha\n- beta").unwrap();
    assert!(result.contains("• alpha"));
    assert!(result.contains("• beta"));
}

#[test]
fn markdown_liste_ordonnee_produit_numerotation() {
    let result = svc().convert("markdown", "1. un\n2. deux").unwrap();
    assert!(result.contains("1. un"));
    assert!(result.contains("2. deux"));
}

#[test]
fn markdown_imbrication_gras_italique() {
    let result = svc()
        .convert("markdown", "**Bonjour *Nathan*, vas-tu ?**")
        .unwrap();
    assert!(!result.contains("Bonjour"));
    assert!(!result.contains("Nathan"));
}

#[test]
fn markdown_contenu_vide_retourne_chaine_vide() {
    let result = svc().convert("markdown", "").unwrap();
    assert_eq!(result, "");
}

#[test]
fn markdown_double_underscore_produit_combining_underline() {
    let result = svc().convert("markdown", "__souligné__").unwrap();
    assert!(result.contains('\u{0332}'));
}

#[test]
fn markdown_symbole_non_supporte_retourne_erreur() {
    use service::ServiceError;
    let result = svc().convert("markdown", "# titre");
    assert!(matches!(result, Err(ServiceError::Parse(_))));
}

// ── Pipeline HTML → Unicode ───────────────────────────────────────────────────

#[test]
fn html_strong_produit_unicode_gras() {
    let result = svc().convert("html", "<strong>ABC</strong>").unwrap();
    assert!(!result.contains("ABC"));
}

#[test]
fn html_em_produit_unicode_italique() {
    let result = svc().convert("html", "<em>ABC</em>").unwrap();
    assert!(!result.contains("ABC"));
}

#[test]
fn html_u_produit_combining_underline() {
    let result = svc().convert("html", "<u>A</u>").unwrap();
    assert!(result.contains('\u{0332}'));
}

#[test]
fn html_br_insere_newline() {
    let result = svc().convert("html", "ligne1<br>ligne2").unwrap();
    assert!(result.contains('\n'));
}

#[test]
fn html_p_fermeture_insere_newline() {
    let result = svc().convert("html", "<p>texte</p>").unwrap();
    assert!(result.ends_with('\n'));
}

#[test]
fn html_ul_li_produit_liste_avec_puces() {
    let result = svc()
        .convert("html", "<ul><li>a</li><li>b</li></ul>")
        .unwrap();
    assert!(result.contains("• a"));
    assert!(result.contains("• b"));
}

#[test]
fn html_ol_li_produit_liste_numerotee() {
    let result = svc()
        .convert("html", "<ol><li>un</li><li>deux</li></ol>")
        .unwrap();
    assert!(result.contains("1. un"));
    assert!(result.contains("2. deux"));
}

#[test]
fn html_blockquote_encadre() {
    let result = svc()
        .convert("html", "<blockquote>citation</blockquote>")
        .unwrap();
    assert!(result.contains('❝'));
}

#[test]
fn html_div_retourne_erreur() {
    use service::ServiceError;
    let result = svc().convert("html", "<div>contenu</div>");
    assert!(matches!(result, Err(ServiceError::Parse(_))));
}

#[test]
fn html_h1_retourne_erreur() {
    use service::ServiceError;
    let result = svc().convert("html", "<h1>titre</h1>");
    assert!(matches!(result, Err(ServiceError::Parse(_))));
}

// ── list_syntaxes — pipeline complet ─────────────────────────────────────────

#[test]
fn list_syntaxes_markdown_retourne_symboles_du_parser() {
    let symbols = svc().list_syntaxes("markdown").unwrap();
    assert_eq!(symbols.len(), 9);
    assert!(symbols.iter().any(|s| s.symbol == "**"));
}

#[test]
fn list_syntaxes_html_retourne_symboles_du_parser() {
    let symbols = svc().list_syntaxes("html").unwrap();
    assert_eq!(symbols.len(), 13);
    assert!(symbols.iter().any(|s| s.symbol == "<strong>"));
}

// ── Cohérence entre Markdown et HTML ─────────────────────────────────────────

#[test]
fn markdown_et_html_gras_produisent_meme_unicode() {
    let md = svc().convert("markdown", "**A**").unwrap();
    let html = svc().convert("html", "<strong>A</strong>").unwrap();
    assert_eq!(md.trim(), html.trim());
}

#[test]
fn markdown_et_html_italique_produisent_meme_unicode() {
    let md = svc().convert("markdown", "*A*").unwrap();
    let html = svc().convert("html", "<em>A</em>").unwrap();
    assert_eq!(md.trim(), html.trim());
}
