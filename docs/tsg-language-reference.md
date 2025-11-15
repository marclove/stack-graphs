# TSG Language Reference

This document provides a complete reference for the Tree-Sitter Graph (TSG) language, which is used to define how stack graphs are constructed from tree-sitter parse trees.

## Table of Contents

- [Overview](#overview)
- [File Structure](#file-structure)
- [Stanzas](#stanzas)
- [Node Creation](#node-creation)
- [Node Attributes](#node-attributes)
- [Edge Creation](#edge-creation)
- [Variables](#variables)
- [Control Flow](#control-flow)
- [Functions](#functions)
- [Scoping Rules](#scoping-rules)
- [Best Practices](#best-practices)

## Overview

TSG is a domain-specific language for pattern matching on tree-sitter parse trees and constructing graphs. For stack graphs, we use TSG to define rules that create stack graph nodes and edges based on the syntactic structure of source code.

### Basic Example

```scheme
; Match a function definition in the parse tree
(function_definition
  name: (identifier) @func_name) @function
{
  ; Create a stack graph node
  node definition
  attr (definition) type = "pop_symbol"
                   symbol = (source-text @func_name)
}
```

## File Structure

A TSG file consists of a series of **stanzas**. Each stanza:

1. Matches a pattern in the parse tree
2. Executes TSG statements when the pattern matches
3. Creates stack graph nodes and edges

```scheme
; Stanza 1
(pattern1) @capture1 {
  ; TSG statements
}

; Stanza 2
(pattern2) @capture2 {
  ; TSG statements
}
```

## Stanzas

### Basic Stanza Syntax

```scheme
(node_type
  field: (child_type) @capture) @parent
{
  ; Statements executed when this pattern matches
}
```

Components:
- `node_type`: The type of syntax node to match (from tree-sitter grammar)
- `field:`: Named field in the syntax node (optional)
- `@capture`: Captures the matched node in a variable
- `{ }`: Statements to execute on match

### Pattern Matching

#### Matching Node Types

```scheme
; Match any identifier
(identifier) @id

; Match specific node types
(function_definition) @func
(class_definition) @class
```

#### Matching with Fields

```scheme
; Match nodes with specific named fields
(assignment
  left: (identifier) @var_name
  right: (_) @value) @stmt
```

#### Wildcard Matching

```scheme
; Match any node type
(_) @any_node

; Match any child in a specific field
(function_definition
  body: (_) @body)
```

#### Matching Specific Text

```scheme
; Match nodes with specific text content
((identifier) @keyword
  (#eq? @keyword "self"))
```

### Predicates

Predicates filter matches:

```scheme
; Check equality
((identifier) @name
  (#eq? @name "special"))

; Check non-equality
((identifier) @name
  (#not-eq? @name "ignore"))

; Pattern matching
((identifier) @name
  (#match? @name "^test_"))
```

## Node Creation

### Creating Nodes

```scheme
; Create a node with default type (scope)
node my_node

; Node names are scoped to the stanza
; Different stanzas can have nodes with the same name
```

### Node Types

Available node types:

- `scope` - Scope node (default)
- `push_symbol` - Push symbol onto symbol stack
- `pop_symbol` - Pop symbol from symbol stack (typically a definition)
- `push_scoped_symbol` - Push symbol with attached scope
- `pop_scoped_symbol` - Pop scoped symbol
- `drop_scopes` - Remove all scopes from scope stack

## Node Attributes

### Setting Attributes

```scheme
node my_node
attr (my_node) type = "pop_symbol"
               symbol = "some_symbol"
               is_definition
```

### Required Attributes by Node Type

#### Scope Nodes

```scheme
node scope_node
attr (scope_node) type = "scope"
                 is_exported  ; optional, for exported scopes
```

#### Push Symbol Nodes

```scheme
node ref_node
attr (ref_node) type = "push_symbol"
               symbol = (source-text @identifier)  ; required
               is_reference                        ; optional
               source_node = @identifier           ; required if is_reference
```

#### Pop Symbol Nodes

```scheme
node def_node
attr (def_node) type = "pop_symbol"
               symbol = (source-text @identifier)  ; required
               is_definition                       ; optional
               source_node = @identifier           ; required if is_definition
```

#### Push Scoped Symbol Nodes

```scheme
node scoped_ref
attr (scoped_ref) type = "push_scoped_symbol"
                 symbol = "field_name"              ; required
                 scope = exported_scope_node        ; required
                 is_reference                       ; optional
                 source_node = @syntax_node         ; required if is_reference
```

#### Pop Scoped Symbol Nodes

```scheme
node scoped_def
attr (scoped_def) type = "pop_scoped_symbol"
                 symbol = "field_name"              ; required
                 is_definition                      ; optional
                 source_node = @syntax_node         ; required if is_definition
```

### Attribute Reference

- `type`: Node type (see above)
- `symbol`: Symbol name (string or function result)
- `scope`: Reference to an exported scope node (for scoped symbols)
- `is_definition`: Marks this as a proper definition for code navigation
- `is_reference`: Marks this as a proper reference for code navigation
- `is_exported`: Marks a scope as exported (can be referenced from other files)
- `source_node`: Syntax node this stack graph node represents
- `empty_source_span`: Use empty span at start of source_node's span

## Edge Creation

### Basic Edge Syntax

```scheme
; Create a directed edge from source to sink
edge source_node -> sink_node
```

### Edge Precedence

```scheme
; Higher precedence edges are preferred during path finding
edge node1 -> node2 precedence: 10
edge node3 -> node4 precedence: 5
```

Lower precedence values are preferred (searched first).

### Special Edge Targets

```scheme
; Edge to the root node (for exports/imports)
edge my_node -> ROOT_NODE

; Edge to a captured syntax node's associated stack graph node
edge my_node -> @some_capture
```

## Variables

### Variable Types

TSG has several types of variables:

1. **Captures**: Created by `@name` in patterns
2. **Nodes**: Created by `node name`
3. **Scoped variables**: Created by `let @name = value`

### Captures

Captures bind syntax nodes:

```scheme
(function_definition
  name: (identifier) @func_name
  body: (block) @body) @function
{
  ; @func_name, @body, and @function are captures
  ; They refer to the matched syntax nodes
}
```

### Scoped Variables

Scoped variables pass information between stanzas:

```scheme
(function_definition) @func
{
  node func_scope

  ; Set @local_scope for child nodes
  let @local_scope = func_scope

  scan @func {
    ; Child stanzas can access @local_scope
  }
}
```

Common scoped variables:

- `@local_scope`: Current lexical scope
- `@root_node`: File's root node
- `@file`: Current file handle

## Control Flow

### Scanning

`scan` processes child nodes:

```scheme
(block) @block
{
  ; Process all children of the block
  scan @block {
    ; Stanzas here match against children
  }
}
```

### Conditional Scanning

```scheme
(function_definition
  body: (block) @body) @func
{
  node func_scope
  let @local_scope = func_scope

  ; Only process function body in this scope
  scan @body {
    ; Child patterns
  }
}
```

### Scanning Specific Fields

```scheme
(class_definition
  name: (_) @name
  body: (_) @body) @class
{
  ; Process only the body
  scan @body {
    ; Patterns for class members
  }
}
```

## Functions

TSG provides built-in functions:

### source-text

Extracts text from a syntax node:

```scheme
(identifier) @id
{
  node ref
  attr (ref) symbol = (source-text @id)
}
```

### node-location

Gets the location of a syntax node:

```scheme
(function_definition) @func
{
  let location = (node-location @func)
  ; location contains start/end positions
}
```

### String Operations

```scheme
; Concatenate strings
let combined = (concat "prefix_" (source-text @id))

; Trim whitespace
let trimmed = (trim (source-text @id))
```

## Scoping Rules

### Lexical Scope

Implement lexical scoping by connecting scopes:

```scheme
(function_definition) @func
{
  ; Create function scope
  node func_scope

  ; Connect to parent scope (lexical scoping)
  edge func_scope -> @local_scope

  ; Set as local scope for function body
  let @local_scope = func_scope

  scan @func {
    ; Function body processed with func_scope as local scope
  }
}
```

### File Scope

Set up initial scope for a file:

```scheme
(module) @mod
{
  ; Create file-level scope
  node file_scope
  attr (file_scope) is_exported  ; Make it accessible from other files

  ; Set as initial local scope
  let @local_scope = file_scope

  ; Connect to root for imports/exports
  edge file_scope -> ROOT_NODE

  scan @mod {
    ; Process module contents
  }
}
```

### Module/Package Scope

For languages with module systems:

```scheme
(module_definition
  name: (identifier) @mod_name) @module
{
  ; Create exported scope for the module
  node mod_scope
  attr (mod_scope) is_exported

  ; Export through root with module name
  node mod_export
  attr (mod_export) type = "pop_symbol"
                   symbol = (source-text @mod_name)
  edge mod_export -> ROOT_NODE
  edge mod_export -> mod_scope
}
```

## Best Practices

### 1. Use Descriptive Node Names

```scheme
; Good
node function_definition_node
node parameter_definition_node

; Less clear
node n1
node n2
```

### 2. Comment Complex Patterns

```scheme
; Python's complex name resolution for nested functions
; Following LEGB rule: Local, Enclosing, Global, Built-in
(function_definition) @func
{
  ; Implementation with explanatory comments
}
```

### 3. Handle Edge Cases

```scheme
; Handle both single and multiple imports
(import_statement
  (identifier) @single_import) @import
{
  ; Handle single import
}

(import_statement
  (import_list
    (identifier) @multi_import)) @import
{
  ; Handle each import in list
}
```

### 4. Reuse Patterns

```scheme
; Define common patterns for reuse
(identifier) @id
{
  node ref
  attr (ref) type = "push_symbol"
             symbol = (source-text @id)
             is_reference
             source_node = @id
  edge ref -> @local_scope
}
```

### 5. Debug with Attributes

```scheme
node my_node
debug-attr (my_node) "type" = "function_scope"
debug-attr (my_node) "name" = (source-text @name)
```

Debug attributes appear in visualizations but don't affect behavior.

### 6. Test Incrementally

Test each feature as you implement it:

```scheme
; Implement simple variables first
; Test thoroughly
; Then add functions
; Test again
; Then add classes
; Test again
```

## Example: Complete Function Implementation

Here's a complete example showing function definitions with parameters:

```scheme
; File scope initialization
(module) @module
{
  node module_scope
  let @local_scope = module_scope

  scan @module {
    ; Process module contents
  }
}

; Function definition
(function_definition
  name: (identifier) @func_name
  parameters: (parameters) @params
  body: (block) @body) @function
{
  ; Create function scope
  node func_scope
  attr (func_scope) type = "scope"

  ; Connect to parent scope (lexical scoping)
  edge func_scope -> @local_scope

  ; Create definition for function name
  node func_def
  attr (func_def) type = "pop_symbol"
                 symbol = (source-text @func_name)
                 is_definition
                 source_node = @function

  ; Function is defined in the parent scope
  edge func_def -> @local_scope

  ; Process parameters in function scope
  let @local_scope = func_scope
  scan @params {
    ; Parameter definitions processed here
  }

  ; Process body in function scope
  scan @body {
    ; Body statements processed here
  }
}

; Parameter definition
(parameter
  (identifier) @param_name) @param
{
  node param_def
  attr (param_def) type = "pop_symbol"
                  symbol = (source-text @param_name)
                  is_definition
                  source_node = @param

  ; Parameter is defined in the function scope
  edge param_def -> @local_scope
}

; Variable reference
(identifier) @id
{
  node var_ref
  attr (var_ref) type = "push_symbol"
               symbol = (source-text @id)
               is_reference
               source_node = @id

  ; Look up in current scope
  edge var_ref -> @local_scope
}
```

## Further Reading

- [Language Implementation Guide](language-implementation.md) - How to implement a language
- [Core Concepts](core-concepts.md) - Stack graph fundamentals
- [Tree-Sitter Documentation](https://tree-sitter.github.io/tree-sitter/) - Parse tree structure

## Summary

TSG is a pattern-matching language that:

1. Matches patterns in tree-sitter parse trees
2. Creates stack graph nodes and edges
3. Manages scoping with variables
4. Provides functions for extracting information

Key concepts:
- **Stanzas** match syntax patterns
- **Nodes** represent stack graph nodes
- **Edges** connect nodes
- **Scoped variables** pass information between stanzas
- **scan** processes child nodes

Master these concepts to effectively implement stack graph rules for any language.
