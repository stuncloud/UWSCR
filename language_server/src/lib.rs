mod completion;
// mod semantic_token;

// use semantic_token::SemanticTokenParser;
use completion::get_snippets;

use tower_lsp::{
    jsonrpc::{self, Result},
    lsp_types::*,
    Client, LanguageServer, LspService, Server,
};
use serde_json::json;
use tokio::task::block_in_place;
// use tokio::sync::RwLock;

// use std::collections::HashMap;

use evaluator::error::UError;
use evaluator::builtins::{get_builtin_names, BuiltinName, BuiltinNameDesc};
use parser::Parser;
use parser::ast::*;
use parser::error::ParseError;
use parser::lexer::{self, Lexer};

pub struct UwscrLanguageServer;

impl UwscrLanguageServer {
    pub fn run() -> std::result::Result<(), UError> {
        let rt = tokio::runtime::Runtime::new()?;
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend::new(client));
        let future = Server::new(stdin, stdout, socket).serve(service);
        rt.block_on(future);
        Ok(())
    }
}

type BackendResult<T> = std::result::Result<T, BackendError>;
enum BackendError {
    ScriptPath,
    IO(std::io::Error),
}
impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::ScriptPath => write!(f, "Unable to get script path."),
            BackendError::IO(err) => write!(f, "{err}"),
        }
    }
}
impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}
impl From<BackendError> for jsonrpc::Error {
    fn from(err: BackendError) -> Self {
        let mut internal = jsonrpc::Error::internal_error();
        internal.data = Some(json!(err.to_string()));
        internal
    }
}

// impl std::fmt::Display for Lexer {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:#?}", self.lines)
//     }
// }

// #[derive(Debug)]
// struct FileCache {
//     contents: HashMap<Url, String>,
// }
// impl FileCache {
//     fn new() -> Self {
//         Self {
//             contents: HashMap::new()
//         }
//     }
//     fn insert(&mut self, uri: Url, script: String) {
//         self.contents.insert(uri, script);
//     }
//     fn get(&self, uri: &Url) -> Option<String> {
//         self.contents.get(uri).map(|s| s.clone())
//     }
//     // fn get_mut(&self, uri: &Url) -> Option<&mut String> {
//     //     self.contents.get_mut(uri)
//     // }
//     // fn update(&mut self, uri: &Url, )
// }

#[allow(unused)]
struct ProgramAndDiagnostics {
    program: Program,
    diagnostics: Vec<Diagnostic>,
}

fn new_completion_item(kind: CompletionItemKind, detail: String, insert_text: String, label: String, label_detail: Option<String>, label_desc: Option<String>, document: Option<String>) -> CompletionItem {
    CompletionItem {
        label: label,
        label_details: Some(CompletionItemLabelDetails {
            detail: label_detail,
            description: label_desc,
        }),
        kind: Some(kind),
        detail: Some(detail),
        documentation: document.map(|doc| Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc,
        })),
        deprecated: None,
        preselect: None,
        sort_text: None,
        filter_text: None,
        insert_text: Some(insert_text),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
    }
}
struct BuiltinNameWrapper<'a>(&'a BuiltinName);

impl<'a> From<BuiltinNameWrapper<'_>> for Vec<CompletionItem> {
    fn from(wrapper: BuiltinNameWrapper) -> Self {
        let name = wrapper.0.name();
        match wrapper.0.desc() {
            Some(desc) => match desc {
                BuiltinNameDesc::Function(desc) => {
                    let label = name.to_ascii_lowercase();
                    // let pos = label.len() as u32 + 1;
                    match &desc.args {
                        Some(args) => {
                            args.as_params_and_document().into_iter()
                                .map(|pd| {
                                    new_completion_item(
                                        CompletionItemKind::FUNCTION,
                                        desc.desc.clone(),
                                        format!("{label}({})", pd.snippet_params),
                                        format!("{label}"),
                                        Some(format!("({})", pd.params)),
                                        pd.label_desc,
                                        Some(pd.document)
                                    )
                                })
                                .collect()
                        },
                        None => {
                            let item = new_completion_item(
                                CompletionItemKind::FUNCTION,
                                desc.desc.clone(),
                                format!("{label}()"),
                                format!("{label}"),
                                Some("()".into()),
                                None,
                                None
                            );
                            vec![item]
                        },
                    }
                },
                BuiltinNameDesc::Const(desc) => {
                    let label = name.to_ascii_uppercase();
                    let item = new_completion_item(
                        CompletionItemKind::CONSTANT,
                        desc.to_string(),
                        format!("{}", &label),
                        label,
                        None,
                        None,
                        None
                    );
                    vec![item]
                },
            },
            None => {
                let label = name.to_ascii_uppercase();
                let detail = label.clone();
                let item = new_completion_item(
                    CompletionItemKind::CONSTANT,
                    detail,
                    format!("{}", &label),
                    label,
                    None,
                    None,
                    None
                );
                vec![item]
            },
        }
    }
}

// #[derive(Debug)]
struct Backend {
    client: Client,
    // cache: RwLock<FileCache>,
    builtins: Vec<BuiltinName>,
    completion_items: Vec<CompletionItem>,
}
impl Backend {
    fn new(client: Client) -> Self {
        let builtins = get_builtin_names();
        let mut completion_items: Vec<CompletionItem> = builtins.iter()
            .filter(|name| name.is_visible())
            .map(|name| Vec::<CompletionItem>::from(BuiltinNameWrapper(name)))
            .flatten()
            .collect();
        let mut snippets = get_snippets();
        completion_items.append(&mut snippets);
        Self {
            client,
            // cache: RwLock::new(FileCache::new()),
            builtins,
            completion_items,
        }
    }
    fn get_script(&self, uri: &Url) -> BackendResult<String> {
        let path = uri.to_file_path().map_err(|_| BackendError::ScriptPath)?;
        let script = std::fs::read_to_string(path)?;
        Ok(script)
    }
    // pub async fn insert_script(&self, uri: Url, script: String) {
    //     let mut cache = self.cache.write().await;
    //     cache.insert(uri, script);
    // }
    // pub async fn get_cache(&self, uri: &Url) -> Option<String> {
    //     let cache = self.cache.read().await;
    //     cache.get(uri)
    // }
    fn get_builtin_names(&self) -> Vec<String> {
        self.builtins.iter().map(|name| name.name().clone()).collect()
    }
    async fn parse(&self, uri: &Url) -> BackendResult<ProgramAndDiagnostics> {
        let script_path = uri.to_file_path().unwrap_or_default();
        let file_name = script_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let script = self.get_script(uri)?;

        let lexer = Lexer::new(&script);
        let builtin_names = self.get_builtin_names();
        let parser = Parser::new_diagnostics_parser(lexer, script_path, builtin_names);

        // self.insert_script(uri.clone(), script).await;

        let (program, errors) = block_in_place(move || {
            parser.parse_to_program_and_errors()
        });
        let diagnostics = errors.into_iter()
            .filter_map(|e| {
                // ファイル名が一致した場合のみDiagnosticを返す
                (e.script_name == file_name).then_some(e.into_lsp_type())
            })
            .collect();
        let result = ProgramAndDiagnostics { program, diagnostics };
        Ok(result)
    }
    async fn get_diagnostics(&self, uri: &Url) -> BackendResult<Vec<Diagnostic>> {
        let diagnostics = self.parse(uri).await?.diagnostics;
        Ok(diagnostics)
    }
    async fn send_diagnostics(&self, uri: Url) {
        match self.get_diagnostics(&uri).await {
            Ok(diags) => {
                self.client.publish_diagnostics(uri, diags, None).await;
            },
            Err(err) => {
                self.client.log_message(MessageType::ERROR, err.to_string()).await;
            },
        }
    }
    async fn log_info<M: std::fmt::Display>(&self, message: M) {
        self.client.log_message(MessageType::INFO, message).await;
    }
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let initialize_result = InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: None,
                text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
                    open_close: Some(true),
                    // INCREMENTALの方が良い？
                    // change: Some(TextDocumentSyncKind::INCREMENTAL),
                    // change: Some(TextDocumentSyncKind::FULL),
                    change: None,
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(TextDocumentSyncSaveOptions::Supported(true))
                })),
                selection_range_provider: None,
                hover_provider: None,
                completion_provider: Some(CompletionOptions {
                    resolve_provider: None,
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
                    completion_item: Some(CompletionOptionsCompletionItem {
                        label_details_support: None
                    }),
                }),
                signature_help_provider: None,
                definition_provider: None,
                type_definition_provider: None,
                implementation_provider: None,
                references_provider: None,
                document_highlight_provider: None,
                document_symbol_provider: None,
                workspace_symbol_provider: None,
                code_action_provider: None,
                code_lens_provider: None,
                document_formatting_provider: None,
                document_range_formatting_provider: None,
                document_on_type_formatting_provider: None,
                rename_provider: None,
                document_link_provider: None,
                color_provider: None,
                folding_range_provider: None,
                declaration_provider: None,
                execute_command_provider: None,
                workspace: None,
                call_hierarchy_provider: None,
                semantic_tokens_provider: None,
                // semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                //     work_done_progress_options: WorkDoneProgressOptions { work_done_progress: None },
                //     legend: SemanticTokenParser::legend(),
                //     range: None,
                //     full: Some(SemanticTokensFullOptions::Bool(true)),
                //     // full: Some(SemanticTokensFullOptions::Delta{delta:Some(true)}),
                // })),
                moniker_provider: None,
                linked_editing_range_provider: None,
                inline_value_provider: None,
                inlay_hint_provider: None,
                diagnostic_provider: None,
                experimental: None
            },
            server_info: Some(ServerInfo {
                name: "UwscrLanguageServer".into(),
                version: Some(env!("CARGO_PKG_VERSION").into())
            }),
        };
        Ok(initialize_result)
    }
    async fn initialized(&self, _: InitializedParams) {
        let path = std::env::current_exe().unwrap_or_default();
        let msg = format!("UWSCR Language Server is running. [{}]", path.to_string_lossy());
        self.log_info(msg).await;
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.send_diagnostics(params.text_document.uri).await;
    }
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.send_diagnostics(params.text_document.uri).await;
    }
    // async fn did_change(&self, params: DidChangeTextDocumentParams) {

    //     /* 常にエディタ上の状態をキャッシュしておく */
    //     let uri = params.text_document.uri;
    //     let script = params.content_changes
    //         .first()
    //         .map(|event| event.text.to_owned())
    //         .unwrap_or_default();

    //     self.insert_script(uri, script).await;

    // }
    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let response = CompletionResponse::Array(self.completion_items.clone());
        Ok(Some(response))
    }
    // async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>> {
    //     let uri = &params.text_document.uri;
    //     if let Some(script) = self.get_cache(uri).await {
    //         let lexer = Lexer::new(&script);
    //         let parser = SemanticTokenParser::new(lexer);
    //         let data = parser.parse(&self.builtins);
    //         // self.log_info(format!("{:?}", data)).await;
    //         let result = SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data });
    //         Ok(Some(result))
    //     } else {
    //         Ok(None)
    //     }
    // }
    // async fn semantic_tokens_full_delta(
    //     &self,
    //     params: SemanticTokensDeltaParams,
    // ) -> Result<Option<SemanticTokensFullDeltaResult>> {
    //     let message = format!("{:#?}", params);
    //     self.log_info(message).await;
    //     let result = SemanticTokensFullDeltaResult::Tokens(SemanticTokens { result_id: None, data: vec![] });
    //     Ok(Some(result))
    // }
    // async fn semantic_tokens_range(
    //     &self,
    //     params: SemanticTokensRangeParams,
    // ) -> Result<Option<SemanticTokensRangeResult>> {
    //     // let message = format!("{:#?}", params);
    //     // self.log_info(message).await;
    //     let lexer = self.lexer(&params.text_document.uri)?;
    //     let parser = SemanticTokenParser::new(lexer);
    //     let data = parser.parse();
    //     let result = SemanticTokensRangeResult::Tokens(SemanticTokens { result_id: None, data });
    //     Ok(Some(result))
    // }
}

trait IntoLspType<T> {
    fn into_lsp_type(self) -> T;
}

impl IntoLspType<Diagnostic> for ParseError {
    fn into_lsp_type(self) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: self.start.into_lsp_type(),
                end: self.end.into_lsp_type(),
            },
            message: self.kind.to_string(),
            source: Some("uwscr".into()),
            ..Default::default()
        }
    }
}

impl IntoLspType<Position> for lexer::Position {
    fn into_lsp_type(self) -> Position {
        let line = (self.row - 1) as u32;
        let character = (self.column -1) as u32;
        Position { line, character }
    }
}

// impl Program {
//     fn into_rows(self) -> Vec<StatementWithRow> {
//         let mut global = self.global;
//         let mut script = self.script;
//         global.append(&mut script);
//         global
//     }
// }
