# Native LookML reader

`perishables-semantic` includes a native LookML lexer, block-tree parser, and project-scoped semantic normalizer in `crates/perishables-semantic/src/lookml.rs`. Native file grading catches SQL field-reference and view-inheritance defects that assembled API exports omit.

## Grammar subset

The parser produces an ordered intermediate tree of `Item::Prop(String, Value)` and `Item::Block(String, Option<String>, Vec<Item>)` values exposed via `perishables_semantic::parse_tree`.

- **Blocks** cover `view`, `explore`, `dimension`, `dimension_group`, `measure`, `join`, `set`, and `parameter` blocks with bareword or quoted names.
- **Properties** cover barewords, double-quoted strings with backslash escapes, bracketed lists (`[a, b]`), and raw SQL or HTML chunks (`sql`, `sql_on`, `sql_table_name`, `html`) terminated strictly by `;;`.
- **Comments** starting with `#` outside quoted strings and outside raw `;;` SQL chunks run to end of line. `#` characters inside SQL chunks remain intact as SQL content.
- **Strict error boundary** ensures any file with unbalanced braces, unterminated strings, or missing `;;` terminators returns `Err(String)` rather than a partial grade.

## Mapping to the normalized model

- Each `view` becomes a model in `Definition::model_names`.
- `dimension` and `dimension_group` blocks map to `Dimension` items carrying `label`, `description`, and `synonyms` alias claims.
- `measure` blocks map to `Metric` items whose `measure_refs` are extracted from `${field}` and `${view.field}` interpolations inside `sql`, excluding Looker's `${TABLE}` pseudo-reference.
- `explore` `join` blocks map to `Relationship` edges from the explore's base view (`view_name`, defaulting to the explore name) to each joined view.

## Project scope across multiple files

When `perishables grade-looker` or `read_lookml_project` receives multiple `.lookml` files, project-level resolution activates.

1. **Cross-file merging** combines views, explores, and `include` directives across all supplied files.
2. **`extends` resolution** records each `extends: [parent]` declaration as a structural relationship (flagging missing parent views via `DANGLING_RELATIONSHIP`) and iteratively inherits parent dimensions and measures not overridden by the child view while preserving qualified `child.field` addressability.
3. **Qualified field resolution** populates `Definition::qualified_fields` (`view.field`) across own and inherited fields so dotted `${other_view.field}` references inside measure and dimension SQL resolve project-wide and fail with `DANGLING_METRIC_REFERENCE` when a target view or field is absent.
4. **`include` pattern coverage** matches model `include` globs against provided file paths and records unmatched patterns as informational coverage notes.
