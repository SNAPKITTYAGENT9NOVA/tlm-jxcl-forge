# jxcl-parser

Parses a token stream into an assembly AST (instructions, labels, directives).

## Architecture

**Owns:** The assembly AST and its parser.

**Category:** toolchain · **Source:** extraction:jxcl/src/assembler/parser.rs

## Public API

`parse`, `AstNode`

## Dependencies

Workspace crates:

- `jxcl-lexer`
- `jxcl-instructions`
- `jxcl-operands`
- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, error-path.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
