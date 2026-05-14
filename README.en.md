# Askama Minify

A procedural macro crate that minifies Askama templates at compile time.

Starting with `0.3.0`, `askama-minify` no longer ships a CLI. It reads a template during compilation, minifies HTML/CSS/JavaScript, and injects the result as Askama's `#[template(source = "...", ext = "...")]`.

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

Inline templates are also supported:

```rust
#[template_minify(source = "<h1>{{ title }}</h1>", ext = "html")]
#[derive(Template)]
struct InlineTemplate<'a> {
    title: &'a str,
}
```

When `source` is used, `ext` is required.

Extra arguments are forwarded to Askama:

```rust
#[template_minify(path = "page.html", escape = "none")]
#[derive(Template)]
struct PageTemplate;
```

Template files are tracked through `include_str!`, so Cargo rebuilds when the source template changes.

Only `html` and `htm` templates are minified as HTML. Other extensions are injected unchanged as Askama `source` templates.

## Publishing

Publish locally from a clean working tree:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo publish --dry-run --locked
cargo publish --locked
```
