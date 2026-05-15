use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_router, ServerHandler,
};
use service::ContentService;
use tracing::{info, warn};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertParams {
    #[schemars(description = "La syntaxe du contenu : 'markdown' ou 'html'")]
    pub syntax: String,
    #[schemars(description = "Le contenu à convertir")]
    pub content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListSyntaxesParams {
    #[schemars(description = "La syntaxe à inspecter : 'markdown' ou 'html'")]
    pub syntax: String,
}

#[derive(Clone)]
pub struct BoldifyServer {
    svc: ContentService,
    #[allow(dead_code)]
    tool_router: ToolRouter<BoldifyServer>,
}

impl BoldifyServer {
    pub fn new() -> Self {
        Self {
            svc: ContentService::new(),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl BoldifyServer {
    #[tool(
        description = "Convertit du contenu HTML ou Markdown en texte Unicode formaté (gras, italique, souligné, barré, etc.)"
    )]
    async fn convert(
        &self,
        Parameters(ConvertParams { syntax, content }): Parameters<ConvertParams>,
    ) -> String {
        info!(syntax, content_len = content.len(), "mcp convert called");
        match self.svc.convert(&syntax, &content) {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, syntax, "mcp convert failed");
                format!("Erreur : {}", e)
            }
        }
    }

    #[tool(
        description = "Liste les symboles/tags supportés pour la syntaxe donnée (markdown ou html)"
    )]
    async fn list_syntaxes(
        &self,
        Parameters(ListSyntaxesParams { syntax }): Parameters<ListSyntaxesParams>,
    ) -> String {
        info!(syntax, "mcp list_syntaxes called");
        match self.svc.list_syntaxes(&syntax) {
            Ok(symbols) => serde_json::to_string_pretty(&symbols)
                .unwrap_or_else(|_| "Erreur de sérialisation".to_string()),
            Err(e) => {
                warn!(error = %e, syntax, "mcp list_syntaxes failed");
                format!("Erreur : {}", e)
            }
        }
    }
}

#[rmcp::tool_handler]
impl ServerHandler for BoldifyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Serveur MCP Boldify : convertit du texte HTML/Markdown en Unicode formaté.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> BoldifyServer {
        BoldifyServer::new()
    }

    async fn convert(s: &BoldifyServer, syntax: &str, content: &str) -> String {
        s.convert(Parameters(ConvertParams {
            syntax: syntax.to_string(),
            content: content.to_string(),
        }))
        .await
    }

    async fn list_syntaxes(s: &BoldifyServer, syntax: &str) -> String {
        s.list_syntaxes(Parameters(ListSyntaxesParams {
            syntax: syntax.to_string(),
        }))
        .await
    }

    #[tokio::test]
    async fn convert_markdown_valide_retourne_texte() {
        let s = server();
        let result = convert(&s, "markdown", "texte simple").await;
        assert!(!result.is_empty());
        assert!(!result.starts_with("Erreur"));
    }

    #[tokio::test]
    async fn convert_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = convert(&s, "xml", "contenu").await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("xml"));
    }

    #[tokio::test]
    async fn convert_markdown_avec_symbole_non_supporte_retourne_erreur() {
        let s = server();
        let result = convert(&s, "markdown", "# titre").await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("#"));
    }

    #[tokio::test]
    async fn convert_contenu_vide_retourne_chaine_vide() {
        let s = server();
        let result = convert(&s, "markdown", "").await;
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn convert_markdown_gras_retourne_unicode() {
        let s = server();
        let result = convert(&s, "markdown", "**Bonjour**").await;
        assert!(!result.starts_with("Erreur"));
        assert!(!result.contains("Bonjour"));
    }

    #[tokio::test]
    async fn list_syntaxes_markdown_retourne_json_valide() {
        let s = server();
        let result = list_syntaxes(&s, "markdown").await;
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("La réponse doit être du JSON valide");
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn list_syntaxes_html_retourne_json_valide() {
        let s = server();
        let result = list_syntaxes(&s, "html").await;
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert!(!parsed.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_syntaxes_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = list_syntaxes(&s, "toml").await;
        assert!(result.starts_with("Erreur"));
        assert!(result.contains("toml"));
    }

    #[tokio::test]
    async fn list_syntaxes_markdown_contient_champs_requis() {
        let s = server();
        let result = list_syntaxes(&s, "markdown").await;
        assert!(result.contains("symbol"));
        assert!(result.contains("description"));
        assert!(result.contains("example"));
    }
}
