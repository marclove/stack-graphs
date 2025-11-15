// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Interactive HTML visualization for stack graphs.
//!
//! This module generates standalone HTML pages that visualize stack graphs and partial paths
//! using D3.js. The visualizations are interactive, allowing you to explore the graph structure,
//! inspect nodes and edges, and understand path-finding results.
//!
//! ## Overview
//!
//! The visualization module helps with:
//! - **Understanding stack graphs**: See the structure of your graph visually
//! - **Debugging**: Identify issues in graph construction or path finding
//! - **Documentation**: Create visual documentation of how name resolution works
//! - **Teaching**: Explain stack graphs concepts interactively
//!
//! ## Features
//!
//! The generated visualizations include:
//!
//! ### Interactive Graph Display
//! - **Nodes**: All stack graph nodes with labels and types
//! - **Edges**: Directed edges showing connections
//! - **Layout**: Automatic graph layout using D3-DAG
//! - **Zoom/Pan**: Navigate large graphs interactively
//!
//! ### Path Highlighting
//! - **Partial paths**: Visualize precomputed partial paths
//! - **Complete paths**: See how paths are stitched together
//! - **Symbol/scope stacks**: Inspect stack states at each path step
//!
//! ### Node Details
//! - **Node type**: Scope, push/pop symbol, etc.
//! - **Symbol information**: What symbols are pushed/popped
//! - **Source location**: Where in the source code this node represents
//! - **Metadata**: Debug attributes and other annotations
//!
//! ## Basic Usage
//!
//! ### Generate HTML for a Stack Graph
//!
//! ```rust,ignore
//! use stack_graphs::graph::StackGraph;
//! use stack_graphs::partial::PartialPaths;
//! use stack_graphs::stitching::Database;
//! use stack_graphs::serde::NoFilter;
//!
//! let graph = /* ... your stack graph ... */;
//! let mut partials = PartialPaths::new();
//! let mut db = Database::new();
//!
//! // Generate HTML visualization
//! let html = graph.to_html_string(
//!     "My Stack Graph",  // Title
//!     &mut partials,
//!     &mut db,
//!     &NoFilter,  // Include all nodes
//! )?;
//!
//! // Save to file
//! std::fs::write("graph.html", html)?;
//! // Open graph.html in a web browser
//! ```
//!
//! ### Filter What's Visualized
//!
//! For large graphs, filter to show only relevant parts:
//!
//! ```rust,ignore
//! use stack_graphs::serde::FileFilter;
//!
//! // Only show nodes from specific files
//! let filter = FileFilter::new(vec!["src/main.rs", "src/lib.rs"]);
//! let html = graph.to_html_string(
//!     "Filtered View",
//!     &mut partials,
//!     &mut db,
//!     &filter,
//! )?;
//! ```
//!
//! ## Visualization Output
//!
//! The generated HTML is a single self-contained file that includes:
//! - All necessary JavaScript libraries (D3.js, D3-DAG)
//! - CSS styling
//! - The graph data as embedded JSON
//! - Interactive visualization code
//!
//! No external dependencies or internet connection needed - just open the HTML file
//! in any modern web browser.
//!
//! ## Use Cases
//!
//! ### Debugging TSG Rules
//!
//! When implementing stack graph rules for a new language, visualize the result
//! to ensure nodes and edges are created correctly:
//!
//! ```rust,ignore
//! // Build graph for a test file
//! let graph = build_stack_graph_for_file("test.py")?;
//! let mut partials = PartialPaths::new();
//! let mut db = Database::new();
//!
//! // Visualize it
//! let html = graph.to_html_string("Test Graph", &mut partials, &mut db, &NoFilter)?;
//! std::fs::write("debug.html", html)?;
//!
//! // Open in browser to inspect
//! ```
//!
//! ### Inspecting Path Finding
//!
//! After computing partial paths, visualize them to understand how name
//! resolution works:
//!
//! ```rust,ignore
//! // Compute partial paths for a file
//! partials.find_all_partial_paths_in_file(&graph, file, &NoCancellation, |_, _, _| {})?;
//! db.add_partial_paths(&graph, &mut partials, file);
//!
//! // Visualize graph with paths
//! let html = graph.to_html_string("With Paths", &mut partials, &mut db, &NoFilter)?;
//! std::fs::write("paths.html", html)?;
//! ```
//!
//! ### Creating Documentation
//!
//! Generate visualizations to document how your language's name resolution works:
//!
//! ```rust,ignore
//! // Create example showing function scope
//! let graph = create_function_scope_example()?;
//! let html = graph.to_html_string(
//!     "Function Scoping Example",
//!     &mut partials,
//!     &mut db,
//!     &NoFilter
//! )?;
//! std::fs::write("docs/function-scoping.html", html)?;
//! ```
//!
//! ## Performance Considerations
//!
//! ### Large Graphs
//!
//! For very large graphs (thousands of nodes):
//! - Use filters to show subsets
//! - Consider splitting into multiple visualizations (one per file)
//! - Browser performance may degrade with >10,000 nodes
//!
//! ### File Size
//!
//! The HTML includes:
//! - D3.js library (~250 KB)
//! - D3-DAG library (~100 KB)
//! - Visualization code (~50 KB)
//! - Your graph data (varies)
//!
//! Total file size is typically 400 KB + graph size.
//!
//! ## Browser Compatibility
//!
//! The visualization works in modern browsers:
//! - Chrome/Edge 90+
//! - Firefox 88+
//! - Safari 14+
//!
//! Older browsers may not support all features.
//!
//! ## Cargo Features
//!
//! This module requires the `visualization` cargo feature:
//!
//! ```toml
//! [dependencies]
//! stack-graphs = { version = "...", features = ["visualization"] }
//! ```
//!
//! ## Implementation Details
//!
//! The visualization:
//! - Serializes the graph to JSON using the [`serde`][crate::serde] module
//! - Embeds the JSON data into an HTML template
//! - Uses D3.js for rendering and interaction
//! - Uses D3-DAG for automatic graph layout
//! - Includes all assets inline (no external resources)
//!
//! ## See Also
//!
//! - [`graph`][crate::graph]: The stack graph structure being visualized
//! - [`partial`][crate::partial]: Partial paths shown in visualization
//! - [`serde`][crate::serde]: Serialization of graph data

use serde_json::Error;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::serde::Filter;
use crate::stitching::Database;

static CSS: &'static str = include_str!("visualization/visualization.css");
static D3: &'static str = include_str!("visualization/d3.min.js");
static D3_DAG: &'static str = include_str!("visualization/d3-dag.min.js");
static JS: &'static str = include_str!("visualization/visualization.js");

static PKG: &'static str = env!("CARGO_PKG_NAME");
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

//-----------------------------------------------------------------------------
// StackGraph

impl StackGraph {
    pub fn to_html_string(
        &self,
        title: &str,
        partials: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
    ) -> Result<String, Error> {
        let filter = VisualizationFilter(filter);
        let graph = serde_json::to_string(&self.to_serializable_filter(&filter))?;
        let paths = serde_json::to_string(&db.to_serializable_filter(self, partials, &filter))?;
        let html = format!(
            r#"
<!DOCTYPE html>
<html lang="en">

<head>

<meta charset="utf-8">
<title>{title}</title>

<!-- <link href="visualization.css" type="text/css" rel="stylesheet"></link> -->
<style>
{CSS}
</style>

<!-- <script type="text/javascript" src="d3.v7.min.js"></script> -->
<script type="text/javascript">
{D3}
</script>

<!-- <script type="text/javascript" src="d3-dag.v0.10.0.min.js"></script> -->
<script type="text/javascript">
{D3_DAG}
</script>

<!-- <script type="text/javascript" src="visualization.js"></script> -->
<script charset="utf-8">
{JS}
</script>

<script type="text/javascript">
  let graph = {graph};
  let paths = {paths};
</script>

<style>
  html, body, #container {{
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
  }}
</style>

</head>

<body>
  <div id="container">
  </div>
  <script type="text/javascript">
    const container = d3.select("\#container");
    new StackGraph(container, graph, paths, {{ version: "{PKG} {VERSION}" }});
  </script>
</body>

</html>
"#
        );
        Ok(html)
    }
}

struct VisualizationFilter<'a>(&'a dyn Filter);

impl Filter for VisualizationFilter<'_> {
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self.0.include_file(graph, file)
    }

    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool {
        self.0.include_node(graph, node)
    }

    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool {
        self.0.include_edge(graph, source, sink)
    }

    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool {
        self.0.include_partial_path(graph, paths, path)
            && !path.edges.is_empty()
            && path.starts_at_reference(graph)
            && (path.ends_at_definition(graph) || path.ends_in_jump(graph))
    }
}
