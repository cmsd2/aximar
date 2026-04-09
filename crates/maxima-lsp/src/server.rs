use std::sync::Arc;

use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completion;
use crate::convert::parser_span_to_lsp;
use crate::definition;
use crate::document::DocumentState;
use crate::folding;
use crate::helpers;
use crate::hover;
use crate::signature;
use crate::symbols;

pub struct MaximaLsp {
    client: Client,
    catalog: Arc<Catalog>,
    docs: Arc<Docs>,
    packages: Arc<PackageCatalog>,
    documents: DashMap<Url, DocumentState>,
}

impl MaximaLsp {
    pub fn new(client: Client) -> Self {
        let catalog = Arc::new(Catalog::load());
        let docs = Arc::new(Docs::load());
        let packages = Arc::new(PackageCatalog::load());
        tracing::info!("Loaded function catalog, documentation, and packages");
        Self {
            client,
            catalog,
            docs,
            packages,
            documents: DashMap::new(),
        }
    }

    async fn on_change(&self, uri: Url, content: String, version: i32) {
        let state = DocumentState::new(content, version);
        let diagnostics = state.diagnostics();
        self.documents.insert(uri.clone(), state);
        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for MaximaLsp {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "(".into(),
                        ",".into(),
                        "_".into(),
                        "%".into(),
                    ]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(
                    true,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "maxima-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        tracing::info!("maxima-lsp initialized");
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        tracing::info!("maxima-lsp shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.on_change(doc.uri, doc.text, doc.version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        // Full sync — take the last content change
        if let Some(change) = params.content_changes.into_iter().last() {
            self.on_change(uri, change.text, version).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        // Clear diagnostics for closed file
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let prefix = self
            .documents
            .get(&uri)
            .and_then(|doc| {
                helpers::word_at_position(&doc.content, pos.line, pos.character)
            })
            .unwrap_or_default();

        if prefix.is_empty() {
            return Ok(None);
        }

        let items = completion::completions(
            &prefix,
            &self.catalog,
            &self.packages,
            &self.documents,
            &uri,
        );
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(
        &self,
        params: HoverParams,
    ) -> jsonrpc::Result<Option<Hover>> {
        let pos = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let word = match self.documents.get(&uri) {
            Some(doc) => {
                helpers::word_at_position(&doc.content, pos.line, pos.character)
            }
            None => None,
        };

        let word = match word {
            Some(w) => w,
            None => return Ok(None),
        };

        Ok(hover::hover_info(
            &word,
            &self.catalog,
            &self.docs,
            &self.packages,
            &self.documents,
        ))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        let pos = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let call = match self.documents.get(&uri) {
            Some(doc) => {
                helpers::find_enclosing_call(&doc.content, pos.line, pos.character)
            }
            None => None,
        };

        let (func_name, active_param) = match call {
            Some(c) => c,
            None => return Ok(None),
        };

        Ok(signature::signature_help(
            &func_name,
            active_param,
            &self.catalog,
            &self.packages,
            &self.documents,
        ))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let syms = match self.documents.get(&uri) {
            Some(doc) => symbols::document_symbols(&doc.parsed.items),
            None => return Ok(None),
        };
        Ok(Some(DocumentSymbolResponse::Nested(syms)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let pos = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let word = match self.documents.get(&uri) {
            Some(doc) => {
                helpers::word_at_position(&doc.content, pos.line, pos.character)
            }
            None => None,
        };

        let word = match word {
            Some(w) => w,
            None => return Ok(None),
        };

        let loc = definition::goto_definition(&word, &self.documents, &uri);
        Ok(loc.map(GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<Location>>> {
        let pos = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;

        let word = match self.documents.get(&uri) {
            Some(doc) => {
                helpers::word_at_position(&doc.content, pos.line, pos.character)
            }
            None => None,
        };

        let word = match word {
            Some(w) => w,
            None => return Ok(None),
        };

        let locs = definition::find_references(&word, &self.documents);
        if locs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locs))
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        let mut result = Vec::new();

        for entry in self.documents.iter() {
            let uri = entry.key();
            for item in &entry.value().parsed.items {
                let name = item.name();
                if query.is_empty() || name.to_lowercase().contains(&query) {
                    #[allow(deprecated)] // SymbolInformation::deprecated field
                    result.push(SymbolInformation {
                        name: name.to_string(),
                        kind: match item {
                            maxima_mac_parser::MacItem::FunctionDef(_)
                            | maxima_mac_parser::MacItem::MacroDef(_) => {
                                SymbolKind::FUNCTION
                            }
                            maxima_mac_parser::MacItem::VariableAssign(_) => {
                                SymbolKind::VARIABLE
                            }
                        },
                        location: Location {
                            uri: uri.clone(),
                            range: parser_span_to_lsp(item.name_span()),
                        },
                        tags: None,
                        deprecated: None,
                        container_name: None,
                    });
                }
            }
        }

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        let ranges = match self.documents.get(&uri) {
            Some(doc) => folding::folding_ranges(&doc.content, &doc.parsed),
            None => return Ok(None),
        };
        if ranges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ranges))
        }
    }
}
