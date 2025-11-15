# Core Concepts

This guide introduces the fundamental concepts you need to understand stack graphs. We'll start with the basics and progressively build to more advanced topics.

## Table of Contents

- [Name Resolution](#name-resolution)
- [From Scope Graphs to Stack Graphs](#from-scope-graphs-to-stack-graphs)
- [The Symbol Stack](#the-symbol-stack)
- [The Scope Stack](#the-scope-stack)
- [Stack Graph Nodes](#stack-graph-nodes)
- [Paths and Name Bindings](#paths-and-name-bindings)
- [Incrementality](#incrementality)
- [Partial Paths and Stitching](#partial-paths-and-stitching)

## Name Resolution

**Name resolution** is the process of determining what a name (identifier) in a program refers to. For example, in this Python code:

```python
def greet(name):
    message = "Hello, " + name
    print(message)

greet("World")
```

A name resolver needs to determine:
- `greet` on line 5 refers to the function defined on line 1
- `name` on line 2 refers to the parameter defined on line 1
- `message` on line 3 refers to the variable defined on line 2
- `print` refers to a built-in function

This seems simple, but name resolution can become extremely complex with:
- Multiple scopes (functions, classes, modules)
- Imports and exports between files
- Name shadowing
- Dynamic features (reflection, eval, etc.)

## From Scope Graphs to Stack Graphs

### Scope Graphs

Stack graphs are based on the **scope graphs** formalism developed at TU Delft. In scope graphs, the name binding structure of a program is represented as a graph where:

- **Nodes** represent scopes, definitions, and references
- **Edges** represent relationships between these elements
- **Paths** through the graph represent valid name bindings

For example, a scope graph for the Python code above might look like:

```
[module scope]
    |
    ├─> [greet definition]
    |
    └─> [greet function scope]
            |
            ├─> [name parameter definition]
            ├─> [message variable definition]
            └─> [references to name, message, print]
```

### The Challenge of Incrementality

Traditional scope graphs have a limitation: they're not **incremental**. If you change one file in a large codebase, you need to rebuild the entire graph.

Consider resolving `SomeClass.field_name` where `SomeClass` is imported from another file:

```python
from other_module import SomeClass

obj = SomeClass()
value = obj.field_name  # What does field_name refer to?
```

In traditional scope graphs, the reference to `field_name` would be directly connected to its definition in `other_module`. This means:
- The graph for `other_module` contains nodes from *every file* that uses it
- Changing *any* client file requires updating `other_module`'s graph
- This doesn't scale to large codebases

### Stack Graphs: Adding Incrementality

Stack graphs solve this by introducing **two stacks** that are maintained during path finding:

1. **Symbol Stack** - tracks what symbols we're currently trying to resolve
2. **Scope Stack** - tracks which scopes we should search in

The key insight: when resolving `obj.field_name`, we can:
1. **Push** `field_name` onto the symbol stack
2. Start resolving `obj` to find its type
3. Once we find that `obj` is a `SomeClass`, navigate to `SomeClass`'s scope
4. **Pop** `field_name` from the stack and look for it in that scope

This allows each file's graph to be self-contained, depending only on its own source code!

## The Symbol Stack

The **symbol stack** keeps track of what symbols we're trying to resolve as we traverse the graph.

### How It Works

As we follow a path through the stack graph:

- **Push Symbol** nodes add a symbol to the top of the stack
- **Pop Symbol** nodes remove a symbol from the top of the stack (and verify it matches)
- The stack must be **empty** at the start and end of a valid path

### Example: Member Access

For `obj.field_name`:

```
Start: symbol_stack = []

1. Push "field_name"
   symbol_stack = ["field_name"]

2. Push "obj"
   symbol_stack = ["field_name", "obj"]

3. Find definition of obj → it's a SomeClass
   Pop "obj" (matches!)
   symbol_stack = ["field_name"]

4. Navigate to SomeClass's scope

5. Find definition of field_name
   Pop "field_name" (matches!)
   symbol_stack = []

End: symbol_stack = []  ✓ Valid binding!
```

### Scoped Symbols

Some symbols carry additional scope information. For example, when accessing a member through an object, the symbol carries a reference to the object's class scope. These are called **scoped symbols**.

## The Scope Stack

The **scope stack** controls which scopes we search for symbols in. It enables modeling of lexical scoping rules.

### How It Works

- **Push Scoped Symbol** nodes add both a symbol to the symbol stack AND a scope to the scope stack
- **Drop Scopes** nodes remove scopes from the scope stack
- **Jump to Scope** nodes jump to a scope that's on the scope stack

### Example: Nested Functions

Consider this Python code:

```python
x = 10

def outer():
    y = 20

    def inner():
        z = 30
        print(x, y, z)  # Can access all three variables

    inner()
```

When resolving `x` from inside `inner()`:

```
Start: scope_stack = []

1. Start in inner's scope
   scope_stack = [inner_scope]

2. Don't find x in inner_scope, jump to parent
   scope_stack = [inner_scope, outer_scope]

3. Don't find x in outer_scope, jump to parent
   scope_stack = [inner_scope, outer_scope, module_scope]

4. Find x in module_scope!
   Pop x from symbol stack

End: Found the binding
```

## Stack Graph Nodes

Stack graphs use several types of nodes, each with specific behavior:

### Scope Nodes

**Scope nodes** represent scopes in the source language (functions, classes, blocks, modules, etc.).

- Created for each scope in the source code
- Can be marked as **exported** if they need to be referenced from other files
- Connected via edges to form the scope hierarchy

### Push Symbol Nodes

**Push symbol nodes** add a symbol to the symbol stack.

- Typically created for variable/function references
- Can be marked as a **reference** for code navigation
- Push the symbol name onto the symbol stack

### Pop Symbol Nodes

**Pop symbol nodes** remove a symbol from the symbol stack.

- Typically created for variable/function definitions
- Can be marked as a **definition** for code navigation
- Verify that the popped symbol matches the expected symbol
- Fail the path if symbols don't match

### Push Scoped Symbol Nodes

**Push scoped symbol nodes** add both a symbol to the symbol stack AND a scope to the scope stack.

- Used for member access (e.g., `object.field`)
- Store a reference to the scope to be pushed
- Enable jumping into imported or referenced scopes

### Pop Scoped Symbol Nodes

**Pop scoped symbol nodes** remove a scoped symbol from the symbol stack.

- Check that the symbol matches
- Also verify the attached scope matches what's expected
- Used for definitions in specific scopes (e.g., class members)

### Drop Scopes Node

**Drop scopes nodes** remove all scopes from the scope stack.

- Used when entering a new scope that shouldn't access parent scopes
- Not common in most languages (which have lexical scoping)

### Jump to Scope Node

**Jump to scope nodes** jump to a scope on the scope stack without modifying the stacks.

- Used to navigate between scopes
- Enables looking up symbols in parent scopes

### Root Node

The **root node** is a special singleton node that connects files together.

- Exactly one root node per stack graph
- Edges from exported symbols connect to the root
- Edges from the root connect to imported symbols
- Enables cross-file name resolution

## Paths and Name Bindings

A **path** in a stack graph represents a potential name binding. Valid paths must satisfy:

1. **Stack Discipline**: Both stacks must be empty at start and end
2. **Symbol Matching**: Pop operations must match their corresponding pushes
3. **Edge Continuity**: Each edge's source must match the previous edge's target
4. **Scope Validity**: Jump operations must have scopes available on the scope stack

### Path Example

For resolving `print(message)` where `message` is a local variable:

```
[reference to message]
    → Push "message" onto symbol stack

[lexical scope edge]
    → Look in current scope

[definition of message]
    → Pop "message" from symbol stack (matches!)

Result: Valid path found! "message" refers to the local variable.
```

## Incrementality

The key advantage of stack graphs is **incrementality**: analyzing a file produces a self-contained graph that depends only on that file's source code.

### File Graphs

Each source file produces its own **file graph**:

- Contains nodes and edges only for that file
- Has **export nodes** for symbols it exports
- Has **import nodes** for symbols it imports
- Connects to the root node for cross-file references

### Database Storage

Stack graphs can be stored in a database with:

- One entry per file
- Indexed by file path
- Can be updated independently when files change
- Supports efficient incremental analysis

### Partial Paths

To enable incrementality, stack graphs use **partial paths**:

- A **partial path** is a path fragment with pre- and post-conditions on the stacks
- Can start or end with non-empty stacks
- Cached in the database per file
- **Stitched** together at query time to form complete paths

## Partial Paths and Stitching

### Partial Path Structure

A partial path consists of:

- **Start node** and **end node**
- **Precondition**: required state of stacks before the path
- **Postcondition**: resulting state of stacks after the path
- **Edges**: the sequence of edges in the path

### Example Partial Path

For an export statement like `export function greet() {}`:

```
Partial Path:
  Start: [greet definition in file scope]
  End: [root node]
  Precondition: symbol_stack = [], scope_stack = []
  Postcondition: symbol_stack = [], scope_stack = []
  Edges: definition → export_scope → root
```

### Stitching

When resolving a reference, the **stitching algorithm**:

1. Finds all partial paths that could extend the current path
2. Checks if the current path's postcondition matches each partial path's precondition
3. Concatenates matching partial paths
4. Continues until complete paths are found (empty stacks at both ends)

This allows:
- Computing partial paths **once** per file
- Caching them in the database
- Reusing them across queries
- Only recomputing when a file changes

### Stitching Example

Resolving an imported function call:

```
File A (caller):
  Partial path: [reference to greet] → [import from B] → [root node]
  Precondition: symbol_stack = []
  Postcondition: symbol_stack = ["greet"]

Root:
  Can traverse the root node (no stack changes)

File B (definition):
  Partial path: [root node] → [export] → [greet definition]
  Precondition: symbol_stack = ["greet"]
  Postcondition: symbol_stack = []

Result: Stitching these together forms a complete path!
  symbol_stack: [] → ["greet"] → ["greet"] → []  ✓
```

## Summary

Stack graphs extend scope graphs with:

- **Two stacks** (symbol and scope) to enable incrementality
- **Various node types** that manipulate these stacks
- **Paths as bindings** where valid paths satisfy stack discipline
- **Partial paths** that can be cached per file and stitched together
- **Incremental analysis** that doesn't require reanalyzing unchanged files

In the next guide, we'll explore the architecture of the stack-graphs implementation and how these concepts are realized in code.
