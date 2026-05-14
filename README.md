# Askama Minify

编译期压缩 Askama 模板的过程宏 crate。

`askama-minify` 从 `0.3.0` 开始不再提供 CLI。它会在编译期读取模板、压缩 HTML/CSS/JavaScript，然后把结果注入为 Askama 的 `#[template(source = "...", ext = "...")]`。

注意：`#[template_minify(...)]` 必须放在 `#[derive(Template)]` 上方，这样宏会先生成 Askama 需要的 `#[template(...)]` 属性。

## 用法

```rust
use askama::Template;
use askama_minify::template_minify;

#[template_minify(path = "index.html")]
#[derive(Template)]
struct IndexTemplate<'a> {
    title: &'a str,
}
```

路径解析规则：

- 先尝试 `CARGO_MANIFEST_DIR/<path>`
- 再尝试 `CARGO_MANIFEST_DIR/templates/<path>`

所以常见的 Askama 目录结构可以直接写：

```rust
#[template_minify(path = "index.html")]
#[derive(Template)]
struct IndexTemplate;
```

对应文件：

```text
templates/index.html
```

## 内联模板

```rust
use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<h1>{{ title }}</h1>", ext = "html")]
#[derive(Template)]
struct InlineTemplate<'a> {
    title: &'a str,
}
```

使用 `source` 时必须显式传入 `ext`。

## 转发 Askama 参数

`path`、`source`、`ext` 由 `askama-minify` 处理，其它参数会继续转发给 Askama 的 `#[template(...)]`：

```rust
#[template_minify(path = "page.html", escape = "none")]
#[derive(Template)]
struct PageTemplate;
```

会展开为压缩后的：

```rust
#[template(source = "...", ext = "html", escape = "none")]
```

## 说明

- 模板文件会通过 `include_str!` 注入到展开结果里，模板内容变更后 Cargo 能重新编译。
- `html` 和 `htm` 模板会压缩 HTML；其中的 CSS 使用 `lightningcss` 压缩。
- JavaScript 和 HTML 压缩目前仍是保守的内置实现，遇到无法安全处理的 CSS 会回退到原内容。
- 非 HTML 扩展会保留原模板内容，只注入为 Askama 的 `source`。

## 发布

从干净工作区本地发布：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo publish --dry-run --locked
cargo publish --locked
```
