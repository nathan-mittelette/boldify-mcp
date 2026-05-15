use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
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
    ) -> CallToolResult {
        info!(syntax, content_len = content.len(), "mcp convert called");
        match self.svc.convert(&syntax, &content) {
            Ok(result) => CallToolResult::success(vec![Content::text(result)]),
            Err(e) => {
                warn!(error = %e, syntax, "mcp convert failed");
                CallToolResult::error(vec![Content::text(e.to_string())])
            }
        }
    }

    #[tool(
        description = "Liste les symboles/tags supportés pour la syntaxe donnée (markdown ou html)"
    )]
    async fn list_syntaxes(
        &self,
        Parameters(ListSyntaxesParams { syntax }): Parameters<ListSyntaxesParams>,
    ) -> CallToolResult {
        info!(syntax, "mcp list_syntaxes called");
        match self.svc.list_syntaxes(&syntax) {
            Ok(symbols) => {
                let text = serde_json::to_string_pretty(&symbols)
                    .expect("serialization of Vec<SupportedSymbol> cannot fail");
                CallToolResult::success(vec![Content::text(text)])
            }
            Err(e) => {
                warn!(error = %e, syntax, "mcp list_syntaxes failed");
                CallToolResult::error(vec![Content::text(e.to_string())])
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

    fn text_of(result: &CallToolResult) -> &str {
        result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.as_str())
            .unwrap_or("")
    }

    fn is_error(result: &CallToolResult) -> bool {
        result.is_error == Some(true)
    }

    async fn convert(s: &BoldifyServer, syntax: &str, content: &str) -> CallToolResult {
        s.convert(Parameters(ConvertParams {
            syntax: syntax.to_string(),
            content: content.to_string(),
        }))
        .await
    }

    async fn list_syntaxes(s: &BoldifyServer, syntax: &str) -> CallToolResult {
        s.list_syntaxes(Parameters(ListSyntaxesParams {
            syntax: syntax.to_string(),
        }))
        .await
    }

    #[tokio::test]
    async fn convert_markdown_valide_retourne_texte() {
        let s = server();
        let result = convert(&s, "markdown", "texte simple").await;
        assert!(!is_error(&result));
        assert!(!text_of(&result).is_empty());
    }

    #[tokio::test]
    async fn convert_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = convert(&s, "xml", "contenu").await;
        assert!(is_error(&result));
        assert!(text_of(&result).contains("xml"));
    }

    #[tokio::test]
    async fn convert_markdown_avec_symbole_non_supporte_retourne_erreur() {
        let s = server();
        let result = convert(&s, "markdown", "# titre").await;
        assert!(is_error(&result));
        assert!(text_of(&result).contains("#"));
    }

    #[tokio::test]
    async fn convert_contenu_vide_retourne_chaine_vide() {
        let s = server();
        let result = convert(&s, "markdown", "").await;
        assert!(!is_error(&result));
        assert_eq!(text_of(&result), "");
    }

    #[tokio::test]
    async fn convert_markdown_gras_retourne_unicode() {
        let s = server();
        let result = convert(&s, "markdown", "**Bonjour**").await;
        assert!(!is_error(&result));
        assert!(!text_of(&result).contains("Bonjour"));
    }

    #[tokio::test]
    async fn list_syntaxes_markdown_retourne_json_valide() {
        let s = server();
        let result = list_syntaxes(&s, "markdown").await;
        assert!(!is_error(&result));
        let parsed: serde_json::Value =
            serde_json::from_str(text_of(&result)).expect("La réponse doit être du JSON valide");
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn list_syntaxes_html_retourne_json_valide() {
        let s = server();
        let result = list_syntaxes(&s, "html").await;
        assert!(!is_error(&result));
        let parsed: serde_json::Value = serde_json::from_str(text_of(&result)).unwrap();
        assert!(parsed.is_array());
        assert!(!parsed.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_syntaxes_syntaxe_inconnue_retourne_message_erreur() {
        let s = server();
        let result = list_syntaxes(&s, "toml").await;
        assert!(is_error(&result));
        assert!(text_of(&result).contains("toml"));
    }

    #[tokio::test]
    async fn list_syntaxes_markdown_contient_champs_requis() {
        let s = server();
        let result = list_syntaxes(&s, "markdown").await;
        assert!(!is_error(&result));
        let text = text_of(&result);
        assert!(text.contains("symbol"));
        assert!(text.contains("description"));
        assert!(text.contains("example"));
    }
}
