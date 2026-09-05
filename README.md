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

- directories configured through Askama's `[general].dirs`
- `CARGO_MANIFEST_DIR/templates/<path>` when no custom directories are configured
- `CARGO_MANIFEST_DIR/<path>` as a compatibility fallback

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
- Askama config files are tracked too. Relative `include`, `extends`, and `import` paths in file templates keep resolving from the original template directory.
- `html` and `htm` templates are minified as HTML. JavaScript without Askama syntax is parsed before it is minified; scripts containing Askama syntax or unsupported JavaScript are preserved unchanged.
- The built-in CSS minifier preserves stylesheets containing custom-property syntax unchanged because custom-property token whitespace can be significant.

## Features

The default features preserve the full behavior:

- `askama-config`: reads `[general].dirs` from `askama.toml`, using the lightweight `basic-toml` parser. Parsed config and glob results are cached within each compiler process.
- `js-minify`: enables parser-backed JavaScript minification. Disable it to preserve inline JavaScript unchanged and avoid compiling the JavaScript parser.
- `advanced-css`: uses `lightningcss` for fuller CSS minification. This is opt-in because it adds a substantially larger compiler dependency graph.

For the smallest dependency graph, disable default features. In this mode template paths use the default `templates/` directory (plus the manifest-root compatibility fallback), and inline JavaScript is preserved:

```toml
askama-minify = { version = "0.3", default-features = false }
```

Enable advanced CSS minification with:

```toml
askama-minify = { version = "0.3", features = ["advanced-css"] }
```

- CSS containing Askama syntax still falls back to the built-in minifier to avoid failed CSS parser work during compilation.
- Non-HTML extensions are injected unchanged as Askama `source` templates.

## Architecture

`askama-minify` is split into small modules around the procedural macro pipeline:

- `src/lib.rs`: proc-macro entry point. It parses the attribute and target item, then delegates expansion.
- `src/args.rs`: parses `path`, `source`, `ext`, and collects extra Askama arguments for forwarding.
- `src/item.rs`: parses the target derive item and rejects an existing `#[template(...)]` attribute.
- `src/loader.rs`: reads Askama config, resolves template and dependency paths, reads template files, infers extensions, and chooses whether to minify.
- `src/expand.rs`: builds the generated `#[template(source = "...", ext = "...")]` attribute and adds `include_str!` tracking for file templates.
- `src/minifier.rs`: public internal entry for HTML minification.
- `src/minifier/html.rs`: HTML scanner that preserves Askama syntax and delegates inline `<style>` and `<script>` content.
- `src/minifier/css.rs`: CSS minification. It uses the built-in conservative minifier by default and `lightningcss` when `advanced-css` is enabled.
- `src/minifier/js.rs`: parser-backed JavaScript minification with a lossless fallback for Askama syntax and unsupported JavaScript.
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

## Compile-time Benchmark

Run `scripts/benchmark-compile.sh` to measure cold `cargo check` time and peak memory for the minimal, default, and all-features dependency graphs. Each measurement uses an isolated temporary target directory and excludes dependency download time.
