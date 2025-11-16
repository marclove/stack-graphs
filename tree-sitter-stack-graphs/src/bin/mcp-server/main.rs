// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2024, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! MCP server for stack graphs definition lookup.
//!
//! This server implements the Model Context Protocol (MCP) to provide definition lookup
//! capabilities using stack graphs. It accepts requests to find all symbol definitions
//! referenced within a specific line range of a source file.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stack_graphs::arena::Handle;
use stack_graphs::graph::{Node, StackGraph};
use stack_graphs::stitching::{DatabaseCandidates, ForwardPartialPathStitcher, StitcherConfig};
use stack_graphs::storage::SQLiteReader;
use stack_graphs::NoCancellation;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use tree_sitter_stack_graphs::loader::FileReader;

/// MCP protocol message types
const JSONRPC_VERSION: &str = "2.0";

/// Returns the default database path in the current user's local data directory for the
/// given crate name. Distinct crate names will have distinct database paths.
fn default_user_database_path_for_crate(crate_name: &str) -> Result<PathBuf> {
    match dirs::data_local_dir() {
        Some(dir) => Ok(dir.join(format!("{}.sqlite", crate_name))),
        None => Err(anyhow!(
            "unable to determine data local directory for database"
        )),
    }
}

/// Request from MCP client
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// Response to MCP client
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// Error response structure
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Parameters for the lookup_definitions tool
#[derive(Debug, Deserialize)]
struct LookupDefinitionsParams {
    /// Path to the source file
    file_path: String,
    /// Starting line (1-indexed, inclusive)
    line_start: usize,
    /// Ending line (1-indexed, inclusive)
    line_end: usize,
}

/// A single definition result
#[derive(Debug, Serialize)]
struct DefinitionResult {
    /// Symbol name
    symbol: String,
    /// File where the definition is located
    file: String,
    /// Line number where definition starts (1-indexed)
    line: usize,
    /// Column number where definition starts (1-indexed)
    column: usize,
    /// Source code of the definition
    source: String,
}

/// Response from lookup_definitions
#[derive(Debug, Serialize)]
struct LookupDefinitionsResult {
    /// All definitions found
    definitions: Vec<DefinitionResult>,
    /// Summary statistics
    summary: LookupSummary,
}

#[derive(Debug, Serialize)]
struct LookupSummary {
    /// Number of references found in the range
    references_found: usize,
    /// Number of definitions found
    definitions_found: usize,
    /// Number of references with no definition
    unresolved_references: usize,
}

struct McpServer {
    db_path: PathBuf,
    file_reader: FileReader,
}

impl McpServer {
    fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            file_reader: FileReader::new(),
        }
    }

    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        // Handle different methods
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(request.params),
            method => Err(anyhow!("Unknown method: {}", method)),
        };

        match result {
            Ok(result) => JsonRpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: Some(result),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "1.0",
            "serverInfo": {
                "name": "stack-graphs-mcp-server",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {}
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value> {
        Ok(json!({
            "tools": [{
                "name": "lookup_definitions",
                "description": "Find definitions for all symbols referenced in a line range of a source file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the source file"
                        },
                        "line_start": {
                            "type": "integer",
                            "description": "Starting line number (1-indexed, inclusive)",
                            "minimum": 1
                        },
                        "line_end": {
                            "type": "integer",
                            "description": "Ending line number (1-indexed, inclusive)",
                            "minimum": 1
                        }
                    },
                    "required": ["file_path", "line_start", "line_end"]
                }
            }]
        }))
    }

    fn handle_tools_call(&mut self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| anyhow!("Missing params"))?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing tool name"))?;

        let arguments = params
            .get("arguments")
            .ok_or_else(|| anyhow!("Missing arguments"))?;

        match tool_name {
            "lookup_definitions" => {
                let args: LookupDefinitionsParams = serde_json::from_value(arguments.clone())?;
                let result = self.lookup_definitions(args)?;
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result)?
                    }]
                }))
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    fn lookup_definitions(&mut self, params: LookupDefinitionsParams) -> Result<LookupDefinitionsResult> {
        // Validate line range
        if params.line_start > params.line_end {
            return Err(anyhow!(
                "Invalid line range: start ({}) > end ({})",
                params.line_start,
                params.line_end
            ));
        }

        // Canonicalize the file path
        let file_path = std::fs::canonicalize(&params.file_path)
            .map_err(|e| anyhow!("Failed to resolve file path '{}': {}", params.file_path, e))?;

        // Open the database
        let mut db_reader = SQLiteReader::open(&self.db_path)
            .map_err(|e| anyhow!("Failed to open database: {}", e))?;

        // Load the graph for this file
        let file_path_str = file_path.to_string_lossy();
        db_reader.load_graph_for_file(&file_path_str)
            .map_err(|e| anyhow!("Failed to load graph for file: {}", e))?;

        // Get mutable references to graph, partials, and database
        let (graph, partials, db) = db_reader.get();

        // Find the file handle
        let file_handle = graph
            .iter_files()
            .find(|f| graph[*f].name() == file_path_str.as_ref())
            .ok_or_else(|| anyhow!("File not found in graph: {}", file_path_str))?;

        // Find all reference nodes in the line range (convert to 0-indexed)
        let line_start_0 = params.line_start.saturating_sub(1);
        let line_end_0 = params.line_end.saturating_sub(1);

        let references = self.find_references_in_range(
            &graph,
            file_handle,
            line_start_0,
            line_end_0,
        );

        eprintln!("Found {} references in range", references.len());

        // Find definitions for each reference
        let mut definitions = Vec::new();
        let mut unresolved_count = 0;
        let mut seen_definitions = HashSet::new();

        for reference in &references {
            let mut found_definition = false;

            // Use path stitching to find the definition
            let result = ForwardPartialPathStitcher::find_all_complete_partial_paths(
                &mut DatabaseCandidates::new(graph, partials, db),
                vec![*reference],
                StitcherConfig::default(),
                &NoCancellation,
                |g, _p, path| {
                    // path.end_node is the definition
                    let definition_node = path.end_node;

                    // Get source info for the definition
                    if let Some(source_info) = g.source_info(definition_node) {
                        // Get the file from the node ID
                        let def_file_handle = match g[definition_node].id().file() {
                            Some(f) => f,
                            None => return, // Skip nodes without file info
                        };
                        let def_file = &g[def_file_handle];
                        let def_file_path = def_file.name();

                        // Create a unique key for this definition
                        let def_key = (
                            def_file_path.to_string(),
                            source_info.span.start.line,
                            source_info.span.start.column.grapheme_offset,
                        );

                        // Skip if we've already seen this definition
                        if seen_definitions.contains(&def_key) {
                            return;
                        }
                        seen_definitions.insert(def_key);

                        // Get the symbol name
                        let symbol_name = g[definition_node]
                            .symbol()
                            .map(|s| g[s].to_string())
                            .unwrap_or_else(|| "<unknown>".to_string());

                        // Read the definition source code
                        let def_source = self.extract_definition_source(
                            Path::new(def_file_path),
                            &source_info.span,
                        ).unwrap_or_else(|e| {
                            format!("// Error reading source: {}", e)
                        });

                        definitions.push(DefinitionResult {
                            symbol: symbol_name,
                            file: def_file_path.to_string(),
                            line: source_info.span.start.line + 1, // Convert to 1-indexed
                            column: source_info.span.start.column.grapheme_offset + 1,
                            source: def_source,
                        });

                        found_definition = true;
                    }
                },
            );

            if let Err(e) = result {
                eprintln!("Error finding definition for reference: {}", e);
            }

            if !found_definition {
                unresolved_count += 1;
            }
        }

        Ok(LookupDefinitionsResult {
            definitions,
            summary: LookupSummary {
                references_found: references.len(),
                definitions_found: seen_definitions.len(),
                unresolved_references: unresolved_count,
            },
        })
    }

    fn find_references_in_range(
        &self,
        graph: &StackGraph,
        file_handle: Handle<stack_graphs::graph::File>,
        line_start: usize,
        line_end: usize,
    ) -> Vec<Handle<Node>> {
        graph
            .nodes_for_file(file_handle)
            .filter(|node_handle| {
                let node = &graph[*node_handle];

                // Check if it's a reference
                if !node.is_reference() {
                    return false;
                }

                // Check if its source span overlaps with our range
                if let Some(source_info) = graph.source_info(*node_handle) {
                    let span_start = source_info.span.start.line;
                    let span_end = source_info.span.end.line;

                    // Check for overlap with the target range
                    span_start <= line_end && span_end >= line_start
                } else {
                    false
                }
            })
            .collect()
    }

    fn extract_definition_source(
        &mut self,
        file_path: &Path,
        span: &lsp_positions::Span,
    ) -> Result<String> {
        let content = self.file_reader.get(file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        let start_line = span.start.line;
        let end_line = span.end.line;

        if start_line >= lines.len() {
            return Err(anyhow!("Start line {} out of range", start_line));
        }

        let end_line = end_line.min(lines.len().saturating_sub(1));

        // Extract lines from start to end
        let extracted_lines: Vec<&str> = lines[start_line..=end_line].to_vec();

        Ok(extracted_lines.join("\n"))
    }

    fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut stderr = io::stderr();

        writeln!(stderr, "Stack Graphs MCP Server starting...")?;
        writeln!(stderr, "Database: {}", self.db_path.display())?;

        for line in stdin.lock().lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            writeln!(stderr, "Received: {}", line)?;

            // Parse the request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = JsonRpcResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };
                    let response_json = serde_json::to_string(&error_response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                    continue;
                }
            };

            // Handle the request
            let response = self.handle_request(request);

            // Send the response
            let response_json = serde_json::to_string(&response)?;
            writeln!(stderr, "Sending: {}", response_json)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let db_path = default_user_database_path_for_crate(env!("CARGO_PKG_NAME"))?;

    let mut server = McpServer::new(db_path);
    server.run()
}
