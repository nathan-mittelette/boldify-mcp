/// Tests d'exploration — lance avec :
///   cargo test --package service --test explore_outputs -- --nocapture
///
/// Ces tests ne vérifient rien pour l'instant : ils affichent l'output réel
/// pour qu'on puisse décider ensemble ce que chaque post doit produire.

use service::ContentService;

fn svc() -> ContentService {
    ContentService::new()
}

fn show(label: &str, input: &str, syntax: &str) {
    println!("\n╔══ {} ══╗", label);
    println!("── INPUT ──────────────────────────────");
    println!("{}", input);
    println!("── OUTPUT ─────────────────────────────");
    match svc().convert(syntax, input) {
        Ok(out) => println!("{}", out),
        Err(e) => println!("ERROR: {}", e),
    }
    println!("───────────────────────────────────────");
}

// ── Posts Markdown réalistes ──────────────────────────────────────────────────

#[test]
fn explore_md_post_annonce_recrutement() {
    show(
        "MD — Annonce recrutement",
        "\
**🚀 Nouveau défi tech**
On recrute !
__DM ouverts__",
        "markdown",
    );
}

#[test]
fn explore_md_post_gras_simple_multilignes() {
    show(
        "MD — Gras simple multi-lignes",
        "\
**Titre du post**
Voici une ligne normale.
Et une autre ligne normale.",
        "markdown",
    );
}

#[test]
fn explore_md_post_mixte_styles() {
    show(
        "MD — Styles mixtes",
        "\
**Bonjour le monde !**
Je suis *développeur* passionné.
~~Ancien poste~~ Nouveau poste.
==Disponible dès maintenant==",
        "markdown",
    );
}

#[test]
fn explore_md_post_avec_liste() {
    show(
        "MD — Post avec liste",
        "\
**Ce que j'ai appris cette année :**

- **Rust** est incroyable
- *La doc* compte autant que le code
- ~~Les réunions~~ Le focus, c'est précieux",
        "markdown",
    );
}

#[test]
fn explore_md_post_avec_citation() {
    show(
        "MD — Post avec citation",
        "\
**Une pensée du jour :**

> Le code propre est une lettre d'amour aux futurs développeurs.

*Tu en penses quoi ?*",
        "markdown",
    );
}

#[test]
fn explore_md_post_complet_linkedin() {
    show(
        "MD — Post LinkedIn complet",
        "\
**🚀 3 ans de freelance : ce que personne ne vous dit**

Quand j'ai commencé, je pensais que la technique était le plus dur.
J'avais *complètement* tort.

Voici ce que j'ai vraiment appris :

- **Trouver des clients** est un métier à part entière
- La facturation, ~~personne~~ vraiment personne ne vous apprend ça
- ==Votre réputation== vaut plus que n'importe quel CV
- *La solitude* du freelance est réelle

> Votre réseau est votre filet de sécurité.

**Et vous, qu'est-ce qui vous a le plus surpris ?** 👇",
        "markdown",
    );
}

#[test]
fn explore_md_post_liste_ordonnee() {
    show(
        "MD — Liste ordonnée",
        "\
**Mes 3 principes de code :**

1. Nommer les choses clairement
2. Faire une chose à la fois
3. Tester avant de déployer 🚀",
        "markdown",
    );
}

#[test]
fn explore_md_post_avec_underscore_souligné() {
    show(
        "MD — Souligné avec __",
        "\
**Disponibilité :**
__Ouvert aux opportunités__ dès janvier 2025.
Contactez-moi en DM.",
        "markdown",
    );
}

#[test]
fn explore_md_post_emojis_et_styles() {
    show(
        "MD — Emojis + styles",
        "\
🎯 **Objectif atteint !**
✅ *Sprint terminé à temps*
🔥 ==Record personnel==
~~ancien objectif~~ nouveau cap 🚀",
        "markdown",
    );
}

#[test]
fn explore_md_post_imbrication() {
    show(
        "MD — Imbrication gras + italique",
        "\
**Bonjour *Nathan*, comment vas-tu ?**
C'est un test d'*imbrication* de styles.",
        "markdown",
    );
}

// ── Posts HTML réalistes ──────────────────────────────────────────────────────

#[test]
fn explore_html_post_annonce_simple() {
    show(
        "HTML — Annonce simple",
        "<strong>🚀 Nouveau défi tech</strong><br>On recrute !<br><u>DM ouverts</u>",
        "html",
    );
}

#[test]
fn explore_html_post_paragraphes() {
    show(
        "HTML — Paragraphes",
        "\
<p><strong>Titre du post</strong></p>
<p>Voici une ligne normale.</p>
<p>Et une autre ligne.</p>",
        "html",
    );
}

#[test]
fn explore_html_post_mixte_styles() {
    show(
        "HTML — Styles mixtes",
        "\
<p><strong>Bonjour le monde !</strong></p>
<p>Je suis <em>développeur</em> passionné.</p>
<p><del>Ancien poste</del> Nouveau poste.</p>",
        "html",
    );
}

#[test]
fn explore_html_post_avec_liste() {
    show(
        "HTML — Post avec liste",
        "\
<p><strong>Ce que j'ai appris :</strong></p>
<ul>
<li><strong>Rust</strong> est incroyable</li>
<li><em>La doc</em> compte autant que le code</li>
<li><del>Les réunions</del> Le focus</li>
</ul>",
        "html",
    );
}

#[test]
fn explore_html_post_avec_citation() {
    show(
        "HTML — Post avec citation",
        "\
<p><strong>Une pensée du jour :</strong></p>
<blockquote>Le code propre est une lettre d'amour aux futurs développeurs.</blockquote>
<p><em>Tu en penses quoi ?</em></p>",
        "html",
    );
}

#[test]
fn explore_html_post_complet() {
    show(
        "HTML — Post complet",
        "\
<p><strong>🚀 3 ans de freelance</strong></p>
<p>J'avais <em>complètement</em> tort.</p>
<ul>
<li><strong>Trouver des clients</strong> est un métier</li>
<li>La facturation, <del>personne</del> vraiment personne</li>
<li><u>Votre réputation</u> vaut plus que tout</li>
</ul>
<blockquote>Votre réseau est votre filet de sécurité.</blockquote>",
        "html",
    );
}

#[test]
fn explore_html_post_liste_ordonnee() {
    show(
        "HTML — Liste ordonnée",
        "\
<p><strong>Mes 3 principes :</strong></p>
<ol>
<li>Nommer les choses clairement</li>
<li>Faire une chose à la fois</li>
<li>Tester avant de déployer</li>
</ol>",
        "html",
    );
}

#[test]
fn explore_html_br_et_styles() {
    show(
        "HTML — <br> et styles inline",
        "<strong>Disponibilité :</strong><br><u>Ouvert aux opportunités</u> dès janvier 2025.<br>Contactez-moi en DM.",
        "html",
    );
}
