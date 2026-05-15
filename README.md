# Askama Minify

[简体中文](README.zh-CN.md)

A procedural macro crate that minifies Askama templates at compile time.

Starting with `0.3.0`, `askama-minify` no longer ships a CLI. It reads a template during compilation, minifies HTML and inline CSS/JavaScript, and injects the result as Askama's `#[template(source = "...", ext = "...")]`.

Important: place `#[template_minify(...)]` above `#[derive(Template)]` so the macro can generate Askama's `#[template(...)]` attribute before the derive macro runs.

## Usage

```rust
use askama::Template;
use askama_minify::template_minify;

#[template_minify(path = "index.html")]
#[derive(Template)]
struct IndexTemplate<'a> {
    title: &'a str,
}
```

Path resolution:

- `CARGO_MANIFEST_DIR/<path>`
- `CARGO_MANIFEST_DIR/templates/<path>`

This means the common Askama layout can use the short path directly:

```rust
#[template_minify(path = "index.html")]
#[derive(Template)]
struct IndexTemplate;
```

With the corresponding file:

```text
templates/index.html
```

## Inline Templates

```rust
use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<h1>{{ title }}</h1>", ext = "html")]
#[derive(Template)]
struct InlineTemplate<'a> {
    title: &'a str,
}
```

When `source` is used, `ext` is required.

## Forwarded Askama Arguments

`path`, `source`, and `ext` are handled by `askama-minify`. Any other arguments are forwarded to Askama's `#[template(...)]` attribute:

```rust
#[template_minify(path = "page.html", escape = "none")]
#[derive(Template)]
struct PageTemplate;
```

This expands to a minified source template:

```rust
#[template(source = "...", ext = "html", escape = "none")]
```

## Notes

- Template files are tracked through `include_str!`, so Cargo rebuilds when the source template changes.
- `html` and `htm` templates are minified as HTML. CSS and JavaScript use conservative built-in minifiers by default.
- Enable the `advanced-css` feature to use `lightningcss` for fuller CSS minification:

```toml
askama-minify = { version = "0.3", features = ["advanced-css"] }
```

- When `advanced-css` is enabled, CSS containing Askama syntax still falls back to the built-in minifier to avoid failed CSS parser work during compilation.
- Non-HTML extensions are injected unchanged as Askama `source` templates.

## Architecture

`askama-minify` is split into small modules around the procedural macro pipeline:

- `src/lib.rs`: proc-macro entry point. It parses the attribute and target item, then delegates expansion.
- `src/args.rs`: parses `path`, `source`, `ext`, and collects extra Askama arguments for forwarding.
- `src/item.rs`: parses the target derive item and rejects an existing `#[template(...)]` attribute.
- `src/loader.rs`: resolves template paths, reads template files, infers extensions, and chooses whether to minify.
- `src/expand.rs`: builds the generated `#[template(source = "...", ext = "...")]` attribute and adds `include_str!` tracking for file templates.
- `src/minifier.rs`: public internal entry for HTML minification.
- `src/minifier/html.rs`: HTML scanner that preserves Askama syntax and delegates inline `<style>` and `<script>` content.
- `src/minifier/css.rs`: CSS minification. It uses the built-in conservative minifier by default and `lightningcss` when `advanced-css` is enabled.
- `src/minifier/js.rs`: conservative JavaScript whitespace/comment minification that preserves string contents and relevant line terminators.
- `src/minifier/template.rs`: shared Askama block copier for `{{ ... }}`, `{% ... %}`, and `{# ... #}`.
- `src/minifier/util.rs`: shared string trimming helpers.

The expansion flow is:

```text
template_minify attribute
  -> parse MacroArgs
  -> parse TemplateItem
  -> load or read source template
  -> minify HTML templates
  -> inject Askama #[template(source = "...", ext = "...")]
  -> emit include_str! tracking for path-based templates
```
