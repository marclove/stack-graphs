# Getting Started with Stack Graphs

This guide will help you get started with using stack graphs in your projects. We'll cover installation, basic usage, and simple examples.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Understanding the Crates](#understanding-the-crates)
- [Quick Start: Using Existing Language Support](#quick-start-using-existing-language-support)
- [Quick Start: Using the Core Library](#quick-start-using-the-core-library)
- [Next Steps](#next-steps)

## Prerequisites

Before you begin, make sure you have:

- **Rust** installed (version 1.70 or later recommended)
  - Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Basic understanding of Rust** (for using the library)
- **Familiarity with your target language** (if implementing stack graph rules)

Optional but recommended:
- **tree-sitter CLI** for testing grammars
- **SQLite** (usually pre-installed on most systems)

## Installation

### As a Library Dependency

Add stack graphs to your `Cargo.toml`:

```toml
[dependencies]
# Core stack graphs library
stack-graphs = "0.14"

# For creating stack graphs from tree-sitter grammars
tree-sitter-stack-graphs = "0.10"

# For working with LSP-compatible positions
lsp-positions = "0.3"
```

### As a Command-Line Tool

Build the CLI tool with all features enabled:

```bash
# Clone the repository
git clone https://github.com/github/stack-graphs.git
cd stack-graphs

# Build with CLI support
cargo build --release --features cli

# The binary will be at target/release/tree-sitter-stack-graphs
```

Or build a specific language implementation:

```bash
# Build the Python implementation
cd languages/tree-sitter-stack-graphs-python
cargo build --release

# Build the TypeScript implementation
cd languages/tree-sitter-stack-graphs-typescript
cargo build --release
```

## Understanding the Crates

The stack-graphs project consists of several crates:

### Core Crates

1. **`stack-graphs`** - The core library
   - Defines the stack graph data structure
   - Implements path-finding algorithms
   - Provides database storage
   - No dependencies on tree-sitter

2. **`tree-sitter-stack-graphs`** - Tree-sitter integration
   - Builds stack graphs from tree-sitter parse trees
   - Executes Tree-Sitter Graph (TSG) rules
   - Includes a CLI for testing and using stack graphs
   - Provides infrastructure for language implementations

3. **`lsp-positions`** - LSP position utilities
   - Converts between UTF-8 and UTF-16 positions
   - Required for LSP (Language Server Protocol) compatibility
   - Handles position mapping in files

### Language Implementation Crates

4. **`tree-sitter-stack-graphs-python`** - Python support
5. **`tree-sitter-stack-graphs-javascript`** - JavaScript support
6. **`tree-sitter-stack-graphs-typescript`** - TypeScript support
7. **`tree-sitter-stack-graphs-java`** - Java support

Each language crate includes:
- TSG rules for that language
- Built-in definitions (standard library)
- Test suite
- Rust wrapper API

## Quick Start: Using Existing Language Support

If you want to use stack graphs for a supported language (Python, JavaScript, TypeScript, or Java), here's the quickest way to get started.

### Index a Project

```bash
# Navigate to a language implementation
cd languages/tree-sitter-stack-graphs-python

# Build with CLI enabled
cargo build --release --all-features

# Index a Python project
cargo run --release --all-features -- \
  index \
  --source-root /path/to/python/project

# Query for definitions
cargo run --release --all-features -- \
  query \
  definitions \
  /path/to/python/project/file.py:10:5
```

### Programmatic Usage

```rust
use tree_sitter_stack_graphs_python::PythonConfiguration;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a stack graph instance
    let mut stack_graph = stack_graphs::graph::StackGraph::new();

    // Create a loader for Python
    let config = PythonConfiguration::new();
    let mut loader = config.language_configuration(&mut stack_graph)?;

    // Load built-in definitions
    loader.load_standard_library(&mut stack_graph)?;

    // Parse and index a file
    let file_path = "example.py";
    let source_code = std::fs::read_to_string(file_path)?;

    let file_handle = loader.load_file(
        &mut stack_graph,
        file_path,
        &source_code,
        &NoCancellation,
    )?;

    // Now you can query the stack graph
    println!("Indexed file: {:?}", file_handle);

    Ok(())
}
```

## Quick Start: Using the Core Library

If you want to build stack graphs programmatically without tree-sitter, use the core library directly.

### Creating a Simple Stack Graph

```rust
use stack_graphs::graph::{StackGraph, Node};
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::{Database, ForwardPartialPathStitcher};

fn main() {
    // Create a new stack graph
    let mut graph = StackGraph::new();

    // Add a file to the graph
    let file = graph.add_file("example.py").expect("Failed to add file");

    // Create some nodes
    let root = graph.root_node();
    let module_scope = graph.add_scope_node(NodeID::new_in_file(file, 0), false).expect("Failed to create scope");

    // Add a symbol
    let greeting = graph.add_symbol("greeting");

    // Create a definition
    let def_node = graph.add_pop_symbol_node(file, greeting, false)
        .expect("Failed to create definition");

    // Create a reference
    let ref_node = graph.add_push_symbol_node(module_scope, greeting, false)
        .expect("Failed to create reference");

    // Connect them with edges
    graph.add_edge(ref_node, module_scope, 0);
    graph.add_edge(module_scope, def_node, 0);

    println!("Created stack graph with {} nodes", graph.iter_nodes().count());
}
```

### Finding Paths

```rust
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::{Database, ForwardPartialPathStitcher};
use stack_graphs::NoCancellation;

fn find_definitions(graph: &StackGraph, reference_node: Handle<Node>) {
    // Create a database for caching partial paths
    let mut db = Database::new();

    // Compute partial paths for all files
    let mut partials = PartialPaths::new();

    for file in graph.iter_files() {
        // Build partial paths for this file
        partials.find_all_partial_paths_in_file(
            graph,
            file,
            &NoCancellation,
            |_graph, _paths, path| {
                // Store each partial path in the database
                db.add_partial_path(graph, path.clone());
            }
        ).expect("Failed to find partial paths");
    }

    // Now stitch paths together starting from the reference
    let mut paths = Vec::new();
    ForwardPartialPathStitcher::find_all_complete_paths(
        graph,
        &mut partials,
        &db,
        reference_node,
        &NoCancellation,
        |_graph, _partials, path| {
            paths.push(path.clone());
        }
    ).expect("Failed to stitch paths");

    println!("Found {} paths from reference", paths.len());
}
```

## Using the CLI Tool

The `tree-sitter-stack-graphs` CLI provides several useful commands:

### Initialize a New Language Implementation

```bash
tree-sitter-stack-graphs init \
  --language python \
  --grammar-path /path/to/tree-sitter-python \
  my-python-sg
```

### Test Stack Graph Rules

```bash
# Test a single file with annotations
tree-sitter-stack-graphs test path/to/test_file.py

# Test with visualization (creates HTML output)
tree-sitter-stack-graphs test -V path/to/test_file.py

# Test all files in a directory
tree-sitter-stack-graphs test path/to/test_files/
```

### Index Files

```bash
# Index a single file
tree-sitter-stack-graphs index path/to/file.py

# Index an entire directory
tree-sitter-stack-graphs index --source-root /path/to/project

# Use a specific SQLite database
tree-sitter-stack-graphs index \
  --database /path/to/database.sqlite \
  --source-root /path/to/project
```

### Query for Definitions and References

```bash
# Find definitions of symbol at position
tree-sitter-stack-graphs query definitions file.py:10:5

# Find references to symbol at position
tree-sitter-stack-graphs query references file.py:10:5
```

### Visualize Stack Graphs

```bash
# Generate HTML visualization of a file's stack graph
tree-sitter-stack-graphs visualize file.py > graph.html

# Open in browser
open graph.html  # macOS
xdg-open graph.html  # Linux
```

## Understanding Test Annotations

Stack graph tests use special comment annotations:

```python
# Define a variable
def greet(name):
#         ^ defined: 1
    return "Hello, " + name
#                      ^ defined: 1
```

Annotations:
- `^ defined: N` - marks a definition (N is a unique ID)
- `^ reference: N` - marks a reference that should resolve to definition N

Run tests with:
```bash
tree-sitter-stack-graphs test your_test.py
```

## Next Steps

Now that you've got the basics, explore these topics:

1. **[Core Concepts](core-concepts.md)** - Understand how stack graphs work
2. **[Architecture Overview](architecture.md)** - Learn about the system architecture
3. **[Language Implementation Guide](language-implementation.md)** - Create stack graph rules for a new language
4. **[Stack Graphs Library Guide](stack-graphs-library.md)** - Deep dive into the core library API
5. **[Tree-Sitter Integration Guide](tree-sitter-integration.md)** - Learn about TSG rules

## Common Issues

### Build Failures

If you encounter build errors:

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Database Lock Errors

If you see "database is locked" errors:

```bash
# Remove the database file and reindex
rm path/to/database.sqlite
tree-sitter-stack-graphs index --source-root /path/to/project
```

### Slow Performance

Stack graph analysis can be slow on large codebases:

- Use `--max-file-time` to limit time per file
- Use `--num-threads` to enable parallel processing
- Consider indexing only changed files in incremental workflows

## Getting Help

- Review the [API documentation](https://docs.rs/stack-graphs/)
- Check the [language implementation tests](../languages/) for examples
- Read the [scope graphs research papers](https://pl.ewi.tudelft.nl/research/projects/scope-graphs/)

## License

Stack graphs is dual-licensed under Apache-2.0 and MIT. See [LICENSE-APACHE](../LICENSE-APACHE) and [LICENSE-MIT](../LICENSE-MIT) for details.
