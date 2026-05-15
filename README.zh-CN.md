# Askama Minify

[English](README.md)

编译期压缩 Askama 模板的过程宏 crate。

`askama-minify` 从 `0.3.0` 开始不再提供 CLI。它会在编译期读取模板、压缩 HTML 和内联 CSS/JavaScript，然后把结果注入为 Askama 的 `#[template(source = "...", ext = "...")]`。

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
- `html` 和 `htm` 模板会压缩 HTML；其中的 CSS 和 JavaScript 默认使用保守的内置实现。
- 如需使用 `lightningcss` 做更完整的 CSS 压缩，可开启 `advanced-css` feature：

```toml
askama-minify = { version = "0.3", features = ["advanced-css"] }
```

- 开启 `advanced-css` 时，包含 Askama 语法的 CSS 仍会回退到内置压缩器，避免无效 CSS 解析拖慢编译。
- 非 HTML 扩展会保留原模板内容，只注入为 Askama 的 `source`。

## 架构

`askama-minify` 按过程宏处理流程拆分成多个小模块：

- `src/lib.rs`：过程宏入口。解析属性参数和目标 item，然后交给展开模块。
- `src/args.rs`：解析 `path`、`source`、`ext`，并收集需要转发给 Askama 的额外参数。
- `src/item.rs`：解析可 derive 的目标 item，并拒绝已有的 `#[template(...)]` 属性。
- `src/loader.rs`：解析模板路径、读取模板文件、推断扩展名，并决定是否压缩。
- `src/expand.rs`：生成 `#[template(source = "...", ext = "...")]` 属性，并为文件模板追加 `include_str!` 跟踪。
- `src/minifier.rs`：内部 HTML 压缩入口。
- `src/minifier/html.rs`：HTML 扫描器，保留 Askama 语法，并分发内联 `<style>` 和 `<script>` 内容。
- `src/minifier/css.rs`：CSS 压缩。默认使用保守内置压缩器，开启 `advanced-css` 后使用 `lightningcss`。
- `src/minifier/js.rs`：保守的 JavaScript 空白/注释压缩，保留字符串内容和必要换行。
- `src/minifier/template.rs`：共享的 Askama 片段复制逻辑，处理 `{{ ... }}`、`{% ... %}` 和 `{# ... #}`。
- `src/minifier/util.rs`：共享字符串裁剪工具。

宏展开流程：

```text
template_minify 属性
  -> 解析 MacroArgs
  -> 解析 TemplateItem
  -> 加载或读取模板源码
  -> 压缩 HTML 模板
  -> 注入 Askama #[template(source = "...", ext = "...")]
  -> 为 path 模板输出 include_str! 跟踪
```
