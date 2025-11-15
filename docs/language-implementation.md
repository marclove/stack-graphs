# Language Implementation Guide

This guide explains how to implement stack graph support for a new programming language using the Tree-Sitter Stack Graphs (TSG) framework.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Overview of the Process](#overview-of-the-process)
- [Step 1: Initialize the Project](#step-1-initialize-the-project)
- [Step 2: Understand Your Language's Scoping](#step-2-understand-your-languages-scoping)
- [Step 3: Write TSG Rules](#step-3-write-tsg-rules)
- [Step 4: Handle Built-ins](#step-4-handle-built-ins)
- [Step 5: Write Tests](#step-5-write-tests)
- [Step 6: Package as a Rust Crate](#step-6-package-as-a-rust-crate)
- [Common Patterns](#common-patterns)
- [Debugging Tips](#debugging-tips)
- [Best Practices](#best-practices)

## Prerequisites

Before implementing stack graph support for a language, you need:

1. **A tree-sitter grammar** for your language
   - If one doesn't exist, create it first using the [tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/creating-parsers)
   - Test that the grammar correctly parses your language

2. **Understanding of stack graphs**
   - Read the [Core Concepts](core-concepts.md) guide
   - Understand how the symbol and scope stacks work
   - Familiarity with different node types

3. **Knowledge of your language's scoping rules**
   - How are names resolved?
   - What scoping constructs exist (functions, classes, modules, etc.)?
   - How does the language handle imports/exports?
   - What built-in names exist?

## Overview of the Process

Implementing stack graph support involves:

```
1. Initialize project structure
   └─> Creates boilerplate files and configuration

2. Write TSG rules
   └─> Define how syntax nodes map to stack graph nodes

3. Create built-in definitions
   └─> Define standard library symbols

4. Write comprehensive tests
   └─> Verify that name resolution works correctly

5. Package and document
   └─> Create a reusable Rust crate
```

## Step 1: Initialize the Project

### Use the CLI Tool

The `tree-sitter-stack-graphs` CLI can bootstrap a new language implementation:

```bash
# Install or build the CLI
cargo install --path tree-sitter-stack-graphs --features cli

# Initialize a new language implementation
tree-sitter-stack-graphs init \
  --language mylang \
  --grammar-path /path/to/tree-sitter-mylang \
  tree-sitter-stack-graphs-mylang
```

This creates a directory structure like:

```
tree-sitter-stack-graphs-mylang/
├── Cargo.toml
├── src/
│   ├── stack-graphs.tsg      # Your TSG rules go here
│   ├── builtins.mylang        # Built-in definitions
│   └── builtins.cfg           # Built-ins configuration
├── rust/
│   ├── lib.rs                 # Rust wrapper
│   ├── bin.rs                 # CLI binary
│   └── test.rs                # Test runner
├── test/
│   └── ...                    # Test files
└── README.md
```

### Manual Setup

If you prefer to set up manually:

1. Create the directory structure above
2. Copy `Cargo.toml` from an existing implementation
3. Update package name, dependencies, and build script
4. Create the TSG rules file
5. Set up built-ins files
6. Write the Rust wrapper code

## Step 2: Understand Your Language's Scoping

Before writing TSG rules, analyze your language's scoping behavior:

### Questions to Answer

1. **What creates a new scope?**
   - Functions? Classes? Blocks? Modules?
   - Are there different kinds of scopes with different lookup rules?

2. **How are names resolved?**
   - Lexical scoping? Dynamic scoping?
   - Can names be imported from other files?
   - Are there special lookup rules (e.g., Python's LEGB rule)?

3. **What about shadowing?**
   - Can local names shadow outer names?
   - Are there restrictions on shadowing?

4. **How do imports/exports work?**
   - What syntax is used?
   - Can you import/export individual names or whole modules?
   - Are there aliasing mechanisms?

### Example: Python

For Python, we'd analyze:

```python
# Global scope
x = 10

def outer():
    # Enclosing scope
    y = 20

    def inner():
        # Local scope
        z = 30
        print(x, y, z)  # Can access all three

    inner()

outer()
```

Key insights:
- Functions create new scopes
- Nested functions can access enclosing scopes (closure)
- Name resolution follows LEGB: Local, Enclosing, Global, Built-in
- Assignment creates a binding in the local scope

## Step 3: Write TSG Rules

TSG (Tree-Sitter Graph) rules define how syntax nodes map to stack graph nodes.

### Basic Structure

A TSG file contains stanzas that match syntax patterns and create stack graph nodes:

```scheme
; Match a syntax pattern
(function_definition
  name: (identifier) @func_name
  parameters: (parameters) @params
  body: (block) @body) @function
{
  ; Create stack graph nodes and edges
  node func_scope
  attr (func_scope) type = "scope"

  node func_def
  attr (func_def) type = "pop_symbol"
                 symbol = (source-text @func_name)
                 is_definition
                 source_node = @function

  edge func_def -> func_scope
}
```

### Node Types in TSG

You can create different types of stack graph nodes:

```scheme
; Scope node (default type)
node my_scope
attr (my_scope) type = "scope"

; Reference node (push symbol)
node my_ref
attr (my_ref) type = "push_symbol"
             symbol = "some_name"
             is_reference
             source_node = @syntax_node

; Definition node (pop symbol)
node my_def
attr (my_def) type = "pop_symbol"
             symbol = "some_name"
             is_definition
             source_node = @syntax_node

; Scoped symbol (for member access)
node my_scoped_ref
attr (my_scoped_ref) type = "push_scoped_symbol"
                    symbol = "field_name"
                    scope = some_exported_scope
                    is_reference
                    source_node = @syntax_node
```

### Example: Simple Variable Definition

Let's implement a simple assignment in Python:

```python
x = 10
```

The tree-sitter parse tree might look like:

```
(assignment
  left: (identifier)   ; "x"
  right: (integer))    ; "10"
```

TSG rules:

```scheme
(assignment
  left: (identifier) @name) @assignment
{
  ; Create a definition for the variable
  node def
  attr (def) type = "pop_symbol"
             symbol = (source-text @name)
             is_definition
             source_node = @name

  ; Connect to the module scope (assumed to exist)
  edge def -> @local_scope
}
```

### Example: Function with Parameters

For a function like:

```python
def greet(name):
    message = "Hello, " + name
    return message
```

TSG rules:

```scheme
(function_definition
  name: (identifier) @func_name
  parameters: (parameters) @params
  body: (block) @body) @function
{
  ; Function scope
  node func_scope
  attr (func_scope) type = "scope"

  ; Function definition
  node func_def
  attr (func_def) type = "pop_symbol"
                 symbol = (source-text @func_name)
                 is_definition
                 source_node = @function

  ; Connect definition to scope
  edge func_def -> func_scope

  ; Make the function scope a child of the parent scope
  edge func_scope -> @local_scope

  ; Process function body in the new scope
  let @local_scope = func_scope
  scan @body {
    ; Body statements will be processed with func_scope as local_scope
  }
}

; Handle parameters
(parameter
  name: (identifier) @param_name) @param
{
  ; Each parameter is a definition in the function scope
  node param_def
  attr (param_def) type = "pop_symbol"
                  symbol = (source-text @param_name)
                  is_definition
                  source_node = @param

  edge param_def -> @local_scope
}

; Handle variable references
(identifier) @id
{
  ; Create a reference
  node ref
  attr (ref) type = "push_symbol"
             symbol = (source-text @id)
             is_reference
             source_node = @id

  ; Look it up in the current scope
  edge ref -> @local_scope
}
```

### Handling Imports and Exports

For cross-file references, connect to the root node:

```scheme
; Export a name (make it available to other files)
(export_statement
  name: (identifier) @export_name) @export
{
  node export_def
  attr (export_def) type = "pop_symbol"
                   symbol = (source-text @export_name)
                   is_definition
                   source_node = @export

  ; Connect to both local scope and root
  edge export_def -> @local_scope
  edge export_def -> ROOT_NODE
}

; Import a name from another file
(import_statement
  name: (identifier) @import_name) @import
{
  node import_ref
  attr (import_ref) type = "push_symbol"
                   symbol = (source-text @import_name)
                   is_reference
                   source_node = @import

  ; Look it up through the root node
  edge import_ref -> ROOT_NODE

  ; Also make it available in local scope
  node import_def
  attr (import_def) type = "pop_symbol"
                   symbol = (source-text @import_name)

  edge import_def -> import_ref
  edge import_def -> @local_scope
}
```

## Step 4: Handle Built-ins

Most languages have built-in functions/types that are always available.

### Create a Built-ins File

Create a source file in your language with all built-in definitions:

```python
# builtins.py
def print(*args): pass
def len(obj): pass
def str(obj): pass
# ... all other built-ins
```

### Configure Built-in Handling

In `builtins.cfg`, specify any special treatment:

```
[builtins]
# Path to the built-ins file
source = "src/builtins.py"

# Special nodes that should be treated as built-ins
[builtins.nodes]
# Mark certain scopes as providing built-in definitions
```

### Load Built-ins in Code

In your Rust wrapper (` rust/lib.rs`):

```rust
impl MyLangConfiguration {
    pub fn load_builtins(&self, graph: &mut StackGraph) -> Result<()> {
        let builtins_source = include_str!("../src/builtins.mylang");
        let builtins_file = graph.get_or_create_file("<builtins>");

        // Parse and process built-ins file
        self.build_stack_graph_into(
            graph,
            builtins_file,
            "<builtins>",
            builtins_source,
            &NoCancellation
        )?;

        Ok(())
    }
}
```

## Step 5: Write Tests

Tests use annotated source files to verify name resolution:

```python
# test/simple_function.py

def greet(name):
#         ^ defined: 1
    return "Hello, " + name
#                      ^ defined: 1

result = greet("World")
#        ^ defined: 2

def greet(name):
#    ^ defined: 2
    pass
```

Annotations:
- `^ defined: N` - marks a definition (N is a unique ID)
- `^ defined: N` (under a name use) - asserts this reference resolves to definition N

### Run Tests

```bash
# Test a single file
cargo run --features cli -- test test/simple_function.py

# Test with visualization
cargo run --features cli -- test -V test/simple_function.py

# Test all files in a directory
cargo run --features cli -- test test/
```

### Test Output

Passing tests show nothing. Failing tests show:

```
test/simple_function.py:
  Line 4: Expected reference to definition 1, but found no definitions
```

## Step 6: Package as a Rust Crate

### Create the Rust API

In `rust/lib.rs`, provide a clean API:

```rust
use tree_sitter_stack_graphs::loader::{LanguageConfiguration, FileLanguageConfigurations};

pub struct MyLangConfiguration {
    // Configuration fields
}

impl MyLangConfiguration {
    pub fn new() -> Self {
        Self { /* ... */ }
    }

    pub fn language_configuration(
        &self,
        graph: &mut StackGraph
    ) -> Result<LanguageConfiguration> {
        // Set up the language configuration
        // Load TSG rules
        // Return configured instance
    }
}
```

### Publish to crates.io

1. Add comprehensive documentation
2. Write a good README
3. Test on multiple platforms
4. Publish: `cargo publish`

## Common Patterns

### Lexical Scoping

Most languages use lexical scoping where inner scopes can access outer scopes:

```scheme
; When creating a new scope, connect it to the parent scope
node new_scope
attr (new_scope) type = "scope"

edge new_scope -> @local_scope
```

### Member Access

For `object.field`:

```scheme
(attribute
  object: (_) @object
  attribute: (identifier) @field) @attr
{
  ; Push the field name with the object's scope attached
  node field_ref
  attr (field_ref) type = "push_scoped_symbol"
                  symbol = (source-text @field)
                  scope = ; (exported scope from object's type)
                  is_reference
                  source_node = @attr
}
```

### Namespaces/Modules

For hierarchical namespaces:

```scheme
; Create an exported scope for the namespace
node ns_scope
attr (ns_scope) type = "scope"
               is_exported

; Export this namespace through the root
edge ns_scope -> ROOT_NODE
```

## Debugging Tips

### Visualize the Stack Graph

```bash
cargo run --features cli -- visualize myfile.py > graph.html
```

Open `graph.html` in a browser to see:
- All nodes and their types
- Edges between nodes
- Symbol and scope information

### Use Verbose Test Output

```bash
cargo run --features cli -- test -V test/myfile.py
```

Shows:
- Partial paths found
- Path stitching process
- Why paths failed or succeeded

### Check Partial Paths

```bash
cargo run --features cli -- status --database db.sqlite --file myfile.py
```

Shows:
- How many partial paths were computed
- Whether the file was successfully indexed

### Add Debug Attributes

In TSG rules, add debug info:

```scheme
node my_node
attr (my_node) type = "scope"

; Add debug information
debug-attr (my_node) "description" = "function scope"
debug-attr (my_node) "syntax" = (source-text @some_node)
```

This appears in visualizations.

## Best Practices

### 1. Start Simple

Begin with basic variable definitions and references. Add complexity incrementally:

1. Simple variables
2. Functions
3. Classes/objects
4. Imports/exports
5. Advanced features

### 2. Test Continuously

Write tests for each feature as you implement it. Don't wait until the end.

### 3. Study Existing Implementations

Look at the Python, JavaScript, or TypeScript implementations for patterns:

```bash
# View Python implementation
cat languages/tree-sitter-stack-graphs-python/src/stack-graphs.tsg
```

### 4. Document Your Rules

Add comments to your TSG file explaining complex patterns:

```scheme
; Python's special __init__.py handling:
; When a package is imported, its __init__.py is executed
; and its definitions become part of the package namespace
```

### 5. Handle Edge Cases

Consider:
- Recursive definitions
- Forward references
- Circular imports
- Name shadowing
- Multiple definitions with the same name

### 6. Optimize for Common Cases

Focus on getting the most common language features working well before handling obscure edge cases.

### 7. Provide Good Error Messages

When things can't be resolved, make sure error messages are helpful:

```rust
// In your Rust code
Err(format!("Cannot resolve import: {} not found", name))
```

## Example: Complete Simple Language

Here's a complete example for a minimal language:

### Language Syntax

```
program = statement*
statement = let_binding | expression
let_binding = "let" identifier "=" expression
expression = identifier | number
```

### TSG Rules

```scheme
; File scope
(program) @prog
{
  node file_scope
  attr (file_scope) type = "scope"

  let @local_scope = file_scope
  scan @prog {
    ; Process statements in file scope
  }
}

; Variable definition
(let_binding
  name: (identifier) @var_name
  value: (_) @value) @let
{
  node var_def
  attr (var_def) type = "pop_symbol"
                symbol = (source-text @var_name)
                is_definition
                source_node = @var_name

  edge var_def -> @local_scope
}

; Variable reference
(identifier) @id
{
  node var_ref
  attr (var_ref) type = "push_symbol"
               symbol = (source-text @id)
               is_reference
               source_node = @id

  edge var_ref -> @local_scope
}
```

### Test File

```
# test/variables.test
let x = 42
#   ^ defined: 1

let y = x
#       ^ defined: 1

let z = y
#       ^ defined: 2
#   ^ defined: 3
```

## Further Reading

- [TSG Language Reference](tsg-language-reference.md) - Complete TSG syntax
- [Core Concepts](core-concepts.md) - Stack graph fundamentals
- [Testing Guide](testing-guide.md) - Advanced testing techniques
- [Tree-Sitter Documentation](https://tree-sitter.github.io/tree-sitter/) - Parser generator docs

## Getting Help

While this project is archived, you can:

- Study the existing language implementations
- Read the comprehensive test suites
- Examine the TSG rule files in detail
- Fork and continue development

## Summary

Implementing stack graph support for a language involves:

1. Understanding the language's scoping rules
2. Writing TSG rules to map syntax to stack graph nodes
3. Defining built-in symbols
4. Writing comprehensive tests
5. Packaging as a reusable crate

Start simple, test continuously, and iterate. The existing implementations provide excellent examples to learn from.
