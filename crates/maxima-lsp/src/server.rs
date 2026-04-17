use std::path::Path;
use std::sync::{Arc, RwLock};

use aximar_core::catalog::doc_index;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use serde_json::Value;
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
    catalog: RwLock<Arc<Catalog>>,
    packages: Arc<PackageCatalog>,
    documents: DashMap<Url, DocumentState>,
}

impl MaximaLsp {
    pub fn new(client: Client) -> Self {
        let catalog = RwLock::new(Arc::new(Catalog::load()));
        let packages = Arc::new(PackageCatalog::load());
        tracing::info!("Loaded function catalog and packages");
        Self {
            client,
            catalog,
            packages,
            documents: DashMap::new(),
        }
    }

    /// Snapshot the current catalog (cheap Arc clone).
    fn catalog(&self) -> Arc<Catalog> {
        self.catalog.read().unwrap().clone()
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
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "maxima.searchFunctions".into(),
                        "maxima.getFunctionDocs".into(),
                    ],
                    ..Default::default()
                }),
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

        // Watch for mxpm doc-index changes so completions/hover update
        // automatically when packages are installed or removed.
        if let Some(userdir) = doc_index::maxima_userdir() {
            let glob = format!("{}/*/doc/*-doc-index.json", userdir.display());
            let registration = Registration {
                id: "mxpm-doc-indexes".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                        watchers: vec![FileSystemWatcher {
                            glob_pattern: GlobPattern::String(glob),
                            kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
                        }],
                    })
                    .unwrap(),
                ),
            };
            if let Err(e) = self.client.register_capability(vec![registration]).await {
                tracing::warn!("Failed to register doc-index watcher: {e}");
            }
        }
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

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        tracing::info!("[doc-index] Reloading catalog (re-ingests runtime doc-indexes)...");
        match tokio::task::spawn_blocking(Catalog::load).await {
            Ok(catalog) => {
                *self.catalog.write().unwrap() = Arc::new(catalog);
            }
            Err(e) => {
                tracing::warn!("[doc-index] Reload failed: {e}");
            }
        }
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

        let catalog = self.catalog();
        let items = completion::completions(
            &prefix,
            &catalog,
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

        let catalog = self.catalog();
        Ok(hover::hover_info(
            &word,
            &catalog,
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

        let catalog = self.catalog();
        Ok(signature::signature_help(
            &func_name,
            active_param,
            &catalog,
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

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<Value>> {
        match params.command.as_str() {
            "maxima.searchFunctions" => {
                let query = params
                    .arguments
                    .first()
                    .and_then(|v| v.get("query"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let catalog = self.catalog();
                let mut results = Vec::new();

                // Search catalog (wraps doc-index: includes core + installed packages)
                for dr in catalog.search(query) {
                    results.push(serde_json::json!({
                        "name": dr.name,
                        "signature": dr.signature,
                        "description": dr.summary,
                        "category": null,
                        "score": dr.score,
                        "package": dr.package,
                    }));
                }

                // Search package functions (deduped)
                let existing: std::collections::HashSet<String> =
                    results.iter().filter_map(|r| r.get("name")?.as_str().map(|n| n.to_lowercase())).collect();
                for pfr in self.packages.search_functions(query) {
                    let name_lower = pfr.function_name.to_lowercase();
                    if !existing.contains(&name_lower) {
                        results.push(serde_json::json!({
                            "name": pfr.function_name,
                            "signature": pfr.signature,
                            "description": pfr.package_description,
                            "category": null,
                            "score": pfr.score,
                            "package": pfr.package_name,
                        }));
                    }
                }

                results.truncate(50);
                Ok(Some(Value::Array(results)))
            }

            "maxima.getFunctionDocs" => {
                let name = params
                    .arguments
                    .first()
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if name.is_empty() {
                    return Err(jsonrpc::Error::invalid_params("missing 'name'"));
                }

                let catalog = self.catalog();

                if let Some((pkg, entry)) = catalog.get(name) {
                    let mut all_sigs = vec![entry.signature.clone()];
                    for alt in &entry.signatures {
                        if !all_sigs.contains(alt) {
                            all_sigs.push(alt.clone());
                        }
                    }
                    let full_docs = if entry.body_md.is_empty() {
                        None
                    } else {
                        let md = entry.body_md.clone();
                        // Inline relative image references as base64 data URIs
                        let doc_dir = doc_index::maxima_userdir()
                            .map(|d| d.join(pkg).join("doc"));
                        Some(inline_images(&md, doc_dir.as_deref()))
                    };
                    return Ok(Some(serde_json::json!({
                        "name": name,
                        "signatures": all_sigs,
                        "description": entry.summary,
                        "category": entry.category,
                        "examples": entry.examples,
                        "see_also": entry.see_also,
                        "full_docs": full_docs,
                        "package": pkg,
                    })));
                }

                Err(jsonrpc::Error::new(jsonrpc::ErrorCode::InvalidRequest))
            }

            _ => Err(jsonrpc::Error::method_not_found()),
        }
    }
}

/// Replace relative markdown image references with inline base64 data URIs.
///
/// Matches `![alt](path)` where path is a relative path, reads the file from
/// `doc_dir/path`, and replaces it with `![alt](data:image/<ext>;base64,...)`.
fn inline_images(md: &str, doc_dir: Option<&Path>) -> String {
    use base64::Engine;

    let doc_dir = match doc_dir {
        Some(d) if d.is_dir() => d,
        _ => return md.to_string(),
    };

    let re = regex::Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();
    re.replace_all(md, |caps: &regex::Captures| {
        let alt = &caps[1];
        let src = &caps[2];

        // Skip absolute URLs and data URIs
        if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:") {
            return caps[0].to_string();
        }

        let file_path = doc_dir.join(src);
        match std::fs::read(&file_path) {
            Ok(bytes) => {
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png");
                let mime = match ext {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    "webp" => "image/webp",
                    _ => "image/png",
                };
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                format!("![{alt}](data:{mime};base64,{b64})")
            }
            Err(_) => caps[0].to_string(),
        }
    })
    .to_string()
}
