use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        ContentBlock, GetPromptRequestParams, GetPromptResult, ListPromptsResult,
        ListResourcesResult, PaginatedRequestParams, Prompt, PromptArgument, PromptMessage,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents, Role,
        ServerCapabilities, ServerInfo,
    },
    schemars,
    service::RequestContext,
    tool, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use service::ContentService;
use tracing::{info, warn};

const URI_SYNTAXES_MARKDOWN: &str = "boldify://syntaxes/markdown";
const URI_SYNTAXES_HTML: &str = "boldify://syntaxes/html";

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertParams {
    #[schemars(description = "Input syntax: 'markdown' or 'html'. \
        IMPORTANT: read the syntax resources first (boldify://syntaxes/markdown or \
        boldify://syntaxes/html) to know exactly which symbols are supported. \
        Using unsupported symbols will return an error. \
        See the 'convert-markdown' and 'convert-html' prompts for usage examples.")]
    pub syntax: String,
    #[schemars(description = "Content to convert to Unicode-formatted text")]
    pub content: String,
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

    fn syntax_resource_json(&self, syntax: &str) -> String {
        match self.svc.list_syntaxes(syntax) {
            Ok(symbols) => serde_json::to_string_pretty(&symbols)
                .expect("serialization of Vec<SupportedSymbol> cannot fail"),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    fn resources_list(&self) -> Vec<rmcp::model::Resource> {
        vec![
            syntax_resource(
                URI_SYNTAXES_MARKDOWN,
                "Markdown syntax reference",
                "All supported Markdown symbols with descriptions and examples. \
                Read this before calling convert with syntax='markdown'.",
            ),
            syntax_resource(
                URI_SYNTAXES_HTML,
                "HTML syntax reference",
                "All supported HTML tags with descriptions and examples. \
                Read this before calling convert with syntax='html'.",
            ),
        ]
    }

    fn read_resource_by_uri(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        let syntax = match uri {
            URI_SYNTAXES_MARKDOWN => "markdown",
            URI_SYNTAXES_HTML => "html",
            other => {
                return Err(McpError::invalid_params(
                    format!("unknown resource URI: {other}"),
                    None,
                ));
            }
        };
        let json = self.syntax_resource_json(syntax);
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            json, uri,
        )
        .with_mime_type("application/json")]))
    }

    fn prompts_list(&self) -> Vec<Prompt> {
        vec![
            Prompt::new(
                "convert-markdown",
                Some(
                    "Example showing how to convert Markdown to Unicode-formatted text. \
                    Demonstrates bold (**), italic (*), strikethrough (~~), \
                    highlight (==), and list (-) syntax.",
                ),
                Some(vec![PromptArgument::new("text")
                    .with_description(
                        "Markdown text to convert (optional — omit to see the built-in example)",
                    )
                    .with_required(false)]),
            ),
            Prompt::new(
                "convert-html",
                Some(
                    "Example showing how to convert HTML to Unicode-formatted text. \
                    Demonstrates <strong>, <em>, <s>, <mark>, <u>, <ul>/<ol>/<li> tags.",
                ),
                Some(vec![PromptArgument::new("text")
                    .with_description(
                        "HTML text to convert (optional — omit to see the built-in example)",
                    )
                    .with_required(false)]),
            ),
        ]
    }

    fn get_prompt_by_name(
        &self,
        name: &str,
        arguments: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<GetPromptResult, McpError> {
        let user_text = |key: &str| -> Option<&str> {
            arguments
                .and_then(|a| a.get(key))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
        };

        match name {
            "convert-markdown" => {
                let messages = if let Some(text) = user_text("text") {
                    vec![PromptMessage::new(
                        Role::User,
                        ContentBlock::text(format!(
                            "Convert the following Markdown text to Unicode-formatted text:\n\n{text}"
                        )),
                    )]
                } else {
                    vec![
                        PromptMessage::new(
                            Role::User,
                            ContentBlock::text(
                                "Convert the following Markdown text to Unicode-formatted text:\n\
                                \n\
                                **Product launch** — our new *intelligent* assistant is ~~delayed~~ ==available now==!\n\
                                \n\
                                Key highlights:\n\
                                - ==Outstanding== performance\n\
                                - **Seamless** integration\n\
                                - *Easy* to use",
                            ),
                        ),
                        PromptMessage::new(
                            Role::Assistant,
                            ContentBlock::text(
                                "I will call the `convert` tool with syntax `markdown` and the content above.\n\
                                \n\
                                Result: 𝐏𝐫𝐨𝐝𝐮𝐜𝐭 𝐥𝐚𝐮𝐧𝐜𝐡 — our new 𝘪𝘯𝘵𝘦𝘭𝘭𝘪𝘨𝘦𝘯𝘵 assistant is 𝐚𝐯𝐚𝐢𝐥𝐚𝐛𝐥𝐞 𝐧𝐨𝐰!\n\
                                \n\
                                Key highlights:\n\
                                • 𝙊𝙪𝙩𝙨𝙩𝙖𝙣𝙙𝙞𝙣𝙜 performance\n\
                                • 𝐒𝐞𝐚𝐦𝐥𝐞𝐬𝐬 integration\n\
                                • 𝘌𝘢𝘴𝘺 to use",
                            ),
                        ),
                    ]
                };
                Ok(GetPromptResult::new(messages))
            }
            "convert-html" => {
                let messages = if let Some(text) = user_text("text") {
                    vec![PromptMessage::new(
                        Role::User,
                        ContentBlock::text(format!(
                            "Convert the following HTML text to Unicode-formatted text:\n\n{text}"
                        )),
                    )]
                } else {
                    vec![
                        PromptMessage::new(
                            Role::User,
                            ContentBlock::text(
                                "Convert the following HTML text to Unicode-formatted text:\n\
                                \n\
                                <p><strong>Product launch</strong> — our new <em>intelligent</em> assistant \
                                is <s>delayed</s> <mark>available now</mark>!</p>\n\
                                <ul>\n\
                                  <li><mark>Outstanding</mark> performance</li>\n\
                                  <li><strong>Seamless</strong> integration</li>\n\
                                  <li><em>Easy</em> to use</li>\n\
                                </ul>",
                            ),
                        ),
                        PromptMessage::new(
                            Role::Assistant,
                            ContentBlock::text(
                                "I will call the `convert` tool with syntax `html` and the content above.\n\
                                \n\
                                Result: 𝐏𝐫𝐨𝐝𝐮𝐜𝐭 𝐥𝐚𝐮𝐧𝐜𝐡 — our new 𝘪𝘯𝘵𝘦𝘭𝘭𝘪𝘨𝘦𝘯𝘵 assistant is 𝐚𝐯𝐚𝐢𝐥𝐚𝐛𝐥𝐞 𝐧𝐨𝐰!\n\
                                \n\
                                • 𝙊𝙪𝙩𝙨𝙩𝙖𝙣𝙙𝙞𝙣𝙜 performance\n\
                                • 𝐒𝐞𝐚𝐦𝐥𝐞𝐬𝐬 integration\n\
                                • 𝘌𝘢𝘴𝘺 to use",
                            ),
                        ),
                    ]
                };
                Ok(GetPromptResult::new(messages))
            }
            other => Err(McpError::invalid_params(
                format!("unknown prompt: {other}"),
                None,
            )),
        }
    }
}

#[tool_router]
impl BoldifyServer {
    #[tool(
        description = "Converts HTML or Markdown content to Unicode-formatted text \
            (bold, italic, underline, strikethrough, highlight, lists, etc.). \
            IMPORTANT — before calling this tool: \
            (1) Read the syntax resource for your chosen format: \
                boldify://syntaxes/markdown or boldify://syntaxes/html. \
            (2) Only use symbols listed in that resource — unsupported symbols (headings, \
                code blocks, tables, etc.) will cause an error. \
            (3) Consult the 'convert-markdown' or 'convert-html' prompts to see \
                correctly composed examples."
    )]
    async fn convert(
        &self,
        Parameters(ConvertParams { syntax, content }): Parameters<ConvertParams>,
    ) -> rmcp::model::CallToolResult {
        info!(syntax, content_len = content.len(), "mcp convert called");
        match self.svc.convert(&syntax, &content) {
            Ok(result) => rmcp::model::CallToolResult::success(vec![ContentBlock::text(result)]),
            Err(e) => {
                warn!(error = %e, syntax, "mcp convert failed");
                rmcp::model::CallToolResult::error(vec![ContentBlock::text(e.to_string())])
            }
        }
    }
}

// ── Resources ────────────────────────────────────────────────────────────────

fn syntax_resource(uri: &str, name: &str, description: &str) -> rmcp::model::Resource {
    Resource::new(uri, name)
        .with_description(description)
        .with_mime_type("application/json")
}

// ── ServerHandler ─────────────────────────────────────────────────────────────

#[rmcp::tool_handler]
impl ServerHandler for BoldifyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(
            "Boldify MCP server: converts HTML/Markdown text to Unicode-formatted text. \
            Read the syntax resources (boldify://syntaxes/markdown, boldify://syntaxes/html) \
            before calling convert to avoid errors. \
            Use the convert-markdown or convert-html prompts to see usage examples.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(self.resources_list()))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        info!(uri = %request.uri, "mcp read_resource called");
        self.read_resource_by_uri(&request.uri)
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(self.prompts_list()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        info!(name = %request.name, "mcp get_prompt called");
        self.get_prompt_by_name(request.name.as_str(), request.arguments.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult};

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

    // ── convert tool ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn convert_valid_markdown_returns_text() {
        let s = server();
        let result = convert(&s, "markdown", "simple text").await;
        assert!(!is_error(&result));
        assert!(!text_of(&result).is_empty());
    }

    #[tokio::test]
    async fn convert_unknown_syntax_returns_error_message() {
        let s = server();
        let result = convert(&s, "xml", "content").await;
        assert!(is_error(&result));
        assert!(text_of(&result).contains("xml"));
    }

    #[tokio::test]
    async fn convert_markdown_with_unsupported_symbol_returns_error() {
        let s = server();
        let result = convert(&s, "markdown", "# title").await;
        assert!(is_error(&result));
        assert!(text_of(&result).contains("#"));
    }

    #[tokio::test]
    async fn convert_empty_content_returns_empty_string() {
        let s = server();
        let result = convert(&s, "markdown", "").await;
        assert!(!is_error(&result));
        assert_eq!(text_of(&result), "");
    }

    #[tokio::test]
    async fn convert_markdown_bold_returns_unicode() {
        let s = server();
        let result = convert(&s, "markdown", "**Hello**").await;
        assert!(!is_error(&result));
        assert!(!text_of(&result).contains("Hello"));
    }

    // ── resources ─────────────────────────────────────────────────────────────

    #[test]
    fn resources_list_returns_two_entries() {
        let s = server();
        let resources = s.resources_list();
        assert_eq!(resources.len(), 2);
        let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&URI_SYNTAXES_MARKDOWN));
        assert!(uris.contains(&URI_SYNTAXES_HTML));
    }

    #[test]
    fn read_resource_markdown_returns_json_array() {
        let s = server();
        let result = s.read_resource_by_uri(URI_SYNTAXES_MARKDOWN).unwrap();
        assert_eq!(result.contents.len(), 1);
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
            assert!(!parsed.as_array().unwrap().is_empty());
        } else {
            panic!("expected TextResourceContents");
        }
    }

    #[test]
    fn read_resource_html_returns_json_array() {
        let s = server();
        let result = s.read_resource_by_uri(URI_SYNTAXES_HTML).unwrap();
        assert_eq!(result.contents.len(), 1);
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        } else {
            panic!("expected TextResourceContents");
        }
    }

    #[test]
    fn read_resource_unknown_uri_returns_error() {
        let s = server();
        assert!(s.read_resource_by_uri("boldify://unknown").is_err());
    }

    #[test]
    fn read_resource_markdown_contains_required_fields() {
        let s = server();
        let result = s.read_resource_by_uri(URI_SYNTAXES_MARKDOWN).unwrap();
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            assert!(text.contains("symbol"));
            assert!(text.contains("description"));
            assert!(text.contains("example"));
        }
    }

    // ── prompts ────────────────────────────────────────────────────────────────

    #[test]
    fn prompts_list_returns_two_entries() {
        let s = server();
        let prompts = s.prompts_list();
        assert_eq!(prompts.len(), 2);
        let names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"convert-markdown"));
        assert!(names.contains(&"convert-html"));
    }

    #[test]
    fn get_prompt_markdown_no_args_returns_two_messages() {
        let s = server();
        let result = s.get_prompt_by_name("convert-markdown", None).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0].role, Role::User);
        assert_eq!(result.messages[1].role, Role::Assistant);
    }

    #[test]
    fn get_prompt_html_no_args_returns_two_messages() {
        let s = server();
        let result = s.get_prompt_by_name("convert-html", None).unwrap();
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0].role, Role::User);
        assert_eq!(result.messages[1].role, Role::Assistant);
    }

    #[test]
    fn get_prompt_with_custom_text_returns_single_user_message() {
        let s = server();
        let args = serde_json::json!({"text": "**Hello world**"})
            .as_object()
            .cloned()
            .unwrap();
        let result = s
            .get_prompt_by_name("convert-markdown", Some(&args))
            .unwrap();
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].role, Role::User);
    }

    #[test]
    fn get_prompt_unknown_returns_error() {
        let s = server();
        assert!(s.get_prompt_by_name("unknown-prompt", None).is_err());
    }
}
