# Stack Graphs Documentation

Welcome to the Stack Graphs documentation! This directory contains comprehensive guides to help you understand, use, and contribute to the stack-graphs project.

## What are Stack Graphs?

Stack graphs provide a unified framework for performing name resolution (finding where identifiers are defined) across any programming language. They extend the scope graphs formalism with incrementality, allowing efficient analysis that can be reused when only some files change.

## Documentation Overview

### For New Users

1. **[Getting Started](getting-started.md)** - Quick start guide to using stack graphs
2. **[Core Concepts](core-concepts.md)** - Fundamental concepts you need to understand stack graphs
3. **[Architecture Overview](architecture.md)** - High-level architecture of the stack-graphs system

### For Library Users

4. **[Stack Graphs Library Guide](stack-graphs-library.md)** - Using the core `stack-graphs` crate
5. **[Tree-Sitter Integration Guide](tree-sitter-integration.md)** - Creating stack graphs from tree-sitter grammars
6. **[LSP Positions Library](lsp-positions.md)** - Working with LSP-compatible text positions

### For Language Implementers

7. **[Language Implementation Guide](language-implementation.md)** - How to add stack graph support for a new language
8. **[TSG Language Reference](tsg-language-reference.md)** - Complete reference for the Tree-Sitter Graph DSL
9. **[Testing Stack Graph Rules](testing-guide.md)** - How to test your stack graph implementations

### For Contributors

10. **[Development Guide](development-guide.md)** - Setting up your development environment
11. **[API Reference](api-reference.md)** - Links to generated API documentation

## Quick Links

- [Main Repository README](../README.md)
- [Contributing Guidelines](../CONTRIBUTING.md)
- [Code of Conduct](../CODE_OF_CONDUCT.md)
- [Scope Graphs Research](https://pl.ewi.tudelft.nl/research/projects/scope-graphs/) - Academic foundation for stack graphs

## Project Status

**IMPORTANT:** This repository is no longer actively maintained by GitHub. The project is archived but remains available for community use and forking.

## Getting Help

While the project is archived, you can:

- Read through the comprehensive documentation in this directory
- Examine the extensive test suites in each language implementation
- Refer to the [API documentation on docs.rs](https://docs.rs/stack-graphs/)
- Fork the repository to continue development

## License

This project is dual-licensed under Apache-2.0 and MIT licenses. See [LICENSE-APACHE](../LICENSE-APACHE) and [LICENSE-MIT](../LICENSE-MIT) for details.
