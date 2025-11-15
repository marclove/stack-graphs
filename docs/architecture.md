# Architecture Overview

This document provides a high-level overview of the stack-graphs system architecture, including the relationships between crates, key data structures, and processing workflows.

## Table of Contents

- [System Architecture](#system-architecture)
- [Crate Organization](#crate-organization)
- [Core Data Structures](#core-data-structures)
- [Processing Pipeline](#processing-pipeline)
- [Path Finding Algorithm](#path-finding-algorithm)
- [Database and Caching](#database-and-caching)
- [Language Implementation Architecture](#language-implementation-architecture)

## System Architecture

The stack-graphs system is organized in layers, from low-level graph operations to high-level language-specific analysis:

```
┌─────────────────────────────────────────────────────────┐
│        Language Implementations                          │
│  (Python, JavaScript, TypeScript, Java)                  │
│  - TSG rules                                             │
│  - Built-in definitions                                  │
│  - Language-specific APIs                                │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│      tree-sitter-stack-graphs                            │
│  - TSG executor                                          │
│  - Tree-sitter integration                               │
│  - Test harness                                          │
│  - CLI tool                                              │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│            stack-graphs (Core)                           │
│  - Stack graph data structure                            │
│  - Path finding algorithms                               │
│  - Partial path stitching                                │
│  - Database storage                                      │
│  - Serialization                                         │
└─────────────────────────────────────────────────────────┘
```

Additionally, the `lsp-positions` crate provides position utilities used by the other crates.

## Crate Organization

### stack-graphs (Core Library)

**Location**: `/stack-graphs/`

**Purpose**: Provides the fundamental stack graph data structure and algorithms independent of any specific parser or language.

**Key Modules**:

- `graph` - Core stack graph data structure
  - Node types (scope, push/pop symbol, etc.)
  - Edge management
  - File management
  - Symbol interning
- `paths` - Path representation and validation
  - Complete paths
  - Path extensions
  - Error types
- `partial` - Partial path computation
  - Partial path structure
  - Forward/backward partial path finding
  - Cycle detection
- `stitching` - Path stitching algorithm
  - Database for partial paths
  - Forward/backward stitching
  - Complete path assembly
- `storage` - SQLite database backend (feature-gated)
  - Persistent storage
  - Incremental updates
  - Query interface
- `visualization` - HTML graph visualization (feature-gated)
  - DOT format export
  - Interactive HTML output
- `serde` - Serialization support (feature-gated)
  - JSON serialization
  - Binary serialization
- `c` - C bindings (for FFI)
- `arena` - Arena-based memory management
  - Handle-based references
  - Batch deallocation

**Dependencies**:
- `tree-sitter` (for some shared types)
- `rusqlite` (for storage feature)
- `serde` (for serialization feature)
- Various utility crates

### tree-sitter-stack-graphs

**Location**: `/tree-sitter-stack-graphs/`

**Purpose**: Integrates tree-sitter parsing with stack graph construction, providing a framework for implementing language-specific rules.

**Key Modules**:

- `lib.rs` - Main library interface
  - Language configuration
  - File processing
  - Integration APIs
- `loader` - File and language loading
  - Multi-language support
  - Built-in handling
  - File discovery
- `functions` - TSG built-in functions
  - `node-location` - Get source location
  - `source-text` - Extract source text
  - Custom functions
- `cli/` - Command-line interface (feature-gated)
  - `index` - Index files into database
  - `query` - Query for definitions/references
  - `test` - Run test files
  - `visualize` - Generate visualizations
  - `init` - Initialize new language
  - `status`, `clean`, `database` - Utility commands
- `test` - Test harness infrastructure
  - Test annotation parsing
  - Test execution
  - Assertion checking

**Dependencies**:
- `stack-graphs` (core library)
- `tree-sitter-graph` (DSL execution)
- `tree-sitter` (parsing)
- `clap` (CLI)
- `tower-lsp` (LSP support)

### Language Implementations

**Locations**:
- `/languages/tree-sitter-stack-graphs-python/`
- `/languages/tree-sitter-stack-graphs-javascript/`
- `/languages/tree-sitter-stack-graphs-typescript/`
- `/languages/tree-sitter-stack-graphs-java/`

**Structure** (each language follows this pattern):

```
tree-sitter-stack-graphs-LANGUAGE/
├── src/
│   ├── stack-graphs.tsg      # TSG rules (main implementation)
│   ├── builtins.LANG          # Built-in definitions
│   └── builtins.cfg           # Built-in configuration
├── rust/
│   ├── lib.rs                 # Rust API
│   ├── bin.rs                 # CLI wrapper
│   └── test.rs                # Test infrastructure
├── test/                      # Test files
│   └── *.LANG                 # Files with test annotations
├── Cargo.toml
└── README.md
```

**Dependencies**:
- `tree-sitter-stack-graphs` (framework)
- `tree-sitter-LANGUAGE` (language parser)

### lsp-positions

**Location**: `/lsp-positions/`

**Purpose**: Converts between UTF-8 byte offsets (used by Rust) and UTF-16 code unit offsets (used by LSP).

**Key Types**:
- `Position` - Line and column position
- `Span` - Start and end positions
- `Offset` - Byte offset into a file

## Core Data Structures

### StackGraph

The central data structure that holds all nodes and edges.

```rust
pub struct StackGraph {
    // Arena-based storage for nodes
    nodes: Arena<Node>,

    // Symbol deduplication
    symbols: Arena<Symbol>,
    symbol_handles: HashMap<&'static str, Handle<Symbol>>,

    // File management
    files: Arena<File>,
    file_paths: HashMap<&'static str, Handle<File>>,

    // String interning
    interned_strings: InternedStringArena,

    // Root node (singleton)
    root: Handle<Node>,
}
```

**Key Characteristics**:
- **Arena-based**: All nodes allocated in arenas, handles instead of pointers
- **Append-only**: Nodes cannot be deleted (only entire graph)
- **Interned strings**: Symbols and file paths deduplicated
- **Type-safe handles**: `Handle<T>` provides type-safe references

### Node Types

```rust
pub enum Node {
    Root,                   // Singleton root node
    Scope(ScopeNode),       // Scopes (functions, classes, etc.)
    PushSymbol(PushSymbolNode),         // Push symbol
    PopSymbol(PopSymbolNode),           // Pop symbol (definition)
    PushScopedSymbol(PushScopedSymbolNode),  // Push with scope
    PopScopedSymbol(PopScopedSymbolNode),    // Pop with scope
    DropScopes(DropScopesNode),         // Clear scope stack
    JumpTo(JumpToNode),     // Jump to scope
}
```

Each node type has specific fields and stack manipulation behavior.

### Edges

```rust
pub struct Edge {
    pub source: Handle<Node>,
    pub sink: Handle<Node>,
    pub precedence: i32,
}
```

Edges connect nodes and have precedence to control ordering during path finding.

### Paths

```rust
// Complete path (stacks empty at start/end)
pub struct Path {
    start_node: Handle<Node>,
    end_node: Handle<Node>,
    edges: Vec<Handle<Edge>>,
    symbol_stack: Vec<Symbol>,  // Always empty at start/end
    scope_stack: Vec<Scope>,    // Always empty at start/end
}

// Partial path (stacks may be non-empty)
pub struct PartialPath {
    start_node: Handle<Node>,
    end_node: Handle<Node>,
    edges: Vec<Handle<Edge>>,
    symbol_stack_precondition: SymbolStackCondition,
    symbol_stack_postcondition: SymbolStackCondition,
    scope_stack_precondition: ScopeStackCondition,
    scope_stack_postcondition: ScopeStackCondition,
}
```

## Processing Pipeline

The typical workflow for processing a source file:

```
┌──────────────┐
│ Source File  │
└──────┬───────┘
       │
       ▼
┌────────────────┐
│ Tree-Sitter    │  Parse source into syntax tree
│ Parse          │
└────────┬───────┘
         │
         ▼
┌────────────────┐
│ Execute TSG    │  Apply stack graph rules to syntax tree
│ Rules          │  Create nodes and edges
└────────┬───────┘
         │
         ▼
┌────────────────┐
│ StackGraph     │  In-memory graph with all nodes/edges
└────────┬───────┘
         │
         ▼
┌────────────────┐
│ Find Partial   │  Compute partial paths for this file
│ Paths          │  Starting from each node
└────────┬───────┘
         │
         ▼
┌────────────────┐
│ Store in       │  Cache partial paths in SQLite
│ Database       │
└────────┬───────┘
         │
         ▼
┌────────────────┐
│ Query Time:    │  Given a reference, find definitions
│ Stitch Paths   │  by stitching partial paths together
└────────────────┘
```

## Path Finding Algorithm

### Phase 1: Partial Path Finding

For each file, compute all partial paths:

1. **Initialize**: Start from each node in the file
2. **Explore**: Follow edges, maintaining stack state
3. **Detect Cycles**: Track visited states to prevent infinite loops
4. **Create Partials**: When reaching file boundary or terminal node, create partial path
5. **Cache**: Store partial paths in database

**Key Algorithm**: Depth-first search with memoization of stack states.

### Phase 2: Path Stitching

Given a reference node, find all definitions:

1. **Start**: Begin with reference node (stacks empty)
2. **Find Candidates**: Query database for partial paths that can extend current path
3. **Check Compatibility**: Verify postcondition of current path matches precondition of candidate
4. **Extend**: Concatenate compatible partial paths
5. **Repeat**: Continue until reaching paths with empty stacks (complete paths)
6. **Filter**: Return only paths ending at definition nodes

**Key Optimizations**:
- Database indexing on preconditions
- Caching of stitched paths
- Pruning of invalid paths early

## Database and Caching

### Schema

The SQLite database stores:

```sql
-- Files indexed in the database
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL
);

-- Partial paths for each file
CREATE TABLE partial_paths (
    id INTEGER PRIMARY KEY,
    file_id INTEGER REFERENCES files(id),
    start_node BLOB NOT NULL,
    end_node BLOB NOT NULL,
    symbol_stack_precondition BLOB,
    symbol_stack_postcondition BLOB,
    scope_stack_precondition BLOB,
    scope_stack_postcondition BLOB,
    edges BLOB NOT NULL
);

-- Indexes for efficient lookup
CREATE INDEX idx_partial_paths_file ON partial_paths(file_id);
CREATE INDEX idx_partial_paths_start ON partial_paths(start_node);
CREATE INDEX idx_partial_paths_end ON partial_paths(end_node);
```

### Incremental Updates

When a file changes:

1. Delete old partial paths for that file
2. Reparse the file
3. Rebuild stack graph for that file
4. Compute new partial paths
5. Insert into database

**Other files' partial paths remain valid!**

## Language Implementation Architecture

Each language implementation follows a standard pattern:

### TSG Rules (stack-graphs.tsg)

The main implementation file, written in the Tree-Sitter Graph DSL:

```scheme
; Example: Function definition
(function_definition
  name: (identifier) @name
  body: (block) @body) @func
{
  ; Create a scope for the function
  node func_scope
  attr (func_scope) type = "scope"

  ; Create a definition
  node def
  attr (def) type = "pop_symbol"
            symbol = (source-text @name)
            source_node = @func
            is_definition

  ; Connect: definition -> scope
  edge def -> func_scope
}
```

### Built-in Definitions

Each language includes standard library definitions:

- `builtins.LANG` - Source file with all built-in symbols
- `builtins.cfg` - Configuration specifying special treatment
- Processed same as user code, but cached separately

### Rust Wrapper API

Provides a Rust API for the language:

```rust
pub struct PythonConfiguration {
    // Configuration options
}

impl PythonConfiguration {
    pub fn new() -> Self { ... }

    pub fn language_configuration<'a>(
        &'a self,
        graph: &'a mut StackGraph
    ) -> Result<LanguageConfiguration<'a>, ...> {
        // Set up tree-sitter parser
        // Load TSG rules
        // Configure built-ins
        // Return configuration
    }
}
```

### Test Suite

Tests use annotated source files:

```python
def add(x, y):
#       ^ defined: 1
#          ^ defined: 2
    return x + y
#          ^ defined: 1
#              ^ defined: 2
```

The test harness:
1. Parses annotations
2. Builds stack graph
3. Queries for references
4. Verifies they resolve to correct definitions

## Memory Management

Stack graphs use arena-based allocation:

- **Arenas**: Bulk allocate memory, hand out handles
- **Handles**: Type-safe indices into arenas
- **No deletion**: Individual items cannot be freed
- **Bulk deallocation**: Entire arena freed at once

**Advantages**:
- Fast allocation (no individual malloc calls)
- No use-after-free bugs (handles always valid)
- Good cache locality
- Simple lifetime management

**Trade-offs**:
- Memory grows monotonically
- Cannot reclaim memory for individual nodes
- Suitable for batch processing workflows

## Concurrency Model

The current implementation is primarily single-threaded with some parallel processing support:

- **Graph Construction**: Single-threaded (tree-sitter parsers not thread-safe)
- **Path Finding**: Can parallelize across files
- **Database**: SQLite with locking (limited concurrency)
- **Future Work**: More parallel processing opportunities

## Extension Points

The architecture supports extension in several ways:

1. **New Languages**: Implement TSG rules for new language grammars
2. **Custom Functions**: Add TSG functions for language-specific operations
3. **Storage Backends**: Alternative database implementations
4. **Visualization**: Custom output formats
5. **Analysis**: Build higher-level analyses on top of name resolution

## Performance Characteristics

- **Graph Construction**: O(n) in source code size
- **Partial Path Finding**: Exponential worst case, but practical in most cases
- **Database Storage**: O(n) in number of partial paths
- **Path Stitching**: Depends on graph connectivity, optimized with indexing

**Scalability**:
- Successfully used on large codebases (millions of lines)
- Incremental analysis key to performance
- Database indexing critical for query performance

## Summary

The stack-graphs architecture provides:

- **Layered design** separating concerns
- **Language-agnostic core** for maximum reusability
- **Tree-sitter integration** for easy language implementation
- **Database-backed caching** for incremental analysis
- **Extensible framework** for adding new languages and features

For more details on specific components, see:
- [Stack Graphs Library Guide](stack-graphs-library.md)
- [Tree-Sitter Integration Guide](tree-sitter-integration.md)
- [Language Implementation Guide](language-implementation.md)
