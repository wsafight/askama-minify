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

- 优先使用 Askama `[general].dirs` 配置的目录
- 未配置自定义目录时使用 `CARGO_MANIFEST_DIR/templates/<path>`
- 最后以 `CARGO_MANIFEST_DIR/<path>` 作为兼容回退

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

- 模板文件及其 `include`、`extends`、`import` 依赖会通过 `include_str!` 注入到展开结果里，任一源模板变更后 Cargo 都能重新编译。
- Askama 配置文件也会被跟踪。模板依赖会相对源模板递归解析和压缩，再通过隔离的生成文件交给 Askama。
- 被引入的 HTML 片段会保留边界空白。如果依赖图在 `pre`、`textarea`、`script`、`style` 或属性值等敏感上下文中插入片段，或包含无法确定上下文的不完整标签，会保守保留这组模板。这也适用于这些位置的继承块和导入宏。
- `html` 和 `htm` 模板会压缩 HTML。没有 Askama 语法的 JavaScript 会在解析成功后压缩；包含 Askama 语法或暂不支持语法的脚本会原样保留。
- JavaScript 和高级 CSS 的压缩结果如果可能引入 HTML 结束标签，会回退到原文。Askama 字符串、嵌套注释、raw 块、带引号的 HTML 属性及非 ASCII 空白均会保留。
- 内置 CSS 压缩器遇到自定义属性语法时会原样保留样式表，因为自定义属性值中的 token 空白可能影响语义。
- 必要的 CSS 注释边界会以空注释保留，避免变成改变选择器含义的空格。
- 模板文件内容和压缩结果会在同一编译器进程内缓存。文件元数据变化会使文件缓存失效；原始依赖仍会全部跟踪，相同的生成文件不会被反复写入。

## Features

默认 features 保持完整功能：

- `askama-config`：使用轻量的 `basic-toml` 读取 `askama.toml` 中的 `[general].dirs`；配置解析和 glob 结果会在同一编译器进程内缓存。
- `js-minify`：启用基于解析器的 JavaScript 压缩。关闭后内联 JavaScript 原样保留，同时无需编译 JavaScript 解析器。
- `advanced-css`：使用 `lightningcss` 做更完整的 CSS 压缩。由于它会显著扩大编译依赖图，因此默认不启用。

如需最小依赖图，可以关闭默认 features。此时模板路径使用默认的 `templates/` 目录（并保留项目根目录兼容回退），内联 JavaScript 原样保留：

```toml
askama-minify = { version = "0.3", default-features = false }
```

启用高级 CSS 压缩：

```toml
askama-minify = { version = "0.3", features = ["advanced-css"] }
```

- 包含 Askama 语法的 CSS 仍会回退到内置压缩器，避免无效 CSS 解析拖慢编译。
- 非 HTML 扩展会保留原模板内容，只注入为 Askama 的 `source`。

## 架构

`askama-minify` 按过程宏处理流程拆分成多个小模块：

- `src/lib.rs`：过程宏入口。解析属性参数和目标 item，然后交给展开模块。
- `src/args.rs`：解析 `path`、`source`、`ext`，并收集需要转发给 Askama 的额外参数。
- `src/item.rs`：解析可 derive 的目标 item，并拒绝已有的 `#[template(...)]` 属性。
- `src/loader.rs`：读取 Askama 配置，递归解析并压缩模板依赖，然后为 Askama 写入隔离的生成模板。
- `src/expand.rs`：生成 `#[template(source = "...", ext = "...")]` 属性，并为模板文件追加 `include_str!` 跟踪。
- `src/minifier.rs`：内部 HTML 压缩入口。
- `src/minifier/html.rs`：HTML 扫描器，保留 Askama 语法，并分发内联 `<style>` 和 `<script>` 内容。
- `src/minifier/css.rs`：CSS 压缩。默认使用保守内置压缩器，开启 `advanced-css` 后使用 `lightningcss`。
- `src/minifier/js.rs`：基于解析器压缩 JavaScript；遇到 Askama 语法或暂不支持的 JavaScript 时无损回退。
- `src/minifier/template.rs`：共享的 Askama 片段复制逻辑，处理 `{{ ... }}`、`{% ... %}` 和 `{# ... #}`。
- `src/minifier/util.rs`：共享字符串裁剪工具。

宏展开流程：

```text
template_minify 属性
  -> 解析 MacroArgs
  -> 解析 TemplateItem
  -> 加载或读取模板源码
  -> 递归解析并压缩模板依赖
  -> 注入 Askama #[template(source = "...", ext = "...")]
  -> 为源模板文件输出 include_str! 跟踪
```

## 编译性能基准

运行 `scripts/benchmark-compile.sh` 可测量最小、默认和 all-features 三种依赖图的冷 `cargo check` 时间与峰值内存。每次测量都使用独立的临时 target 目录，并排除依赖下载时间。

单独测量依赖扫描耗时，不包含编译器启动和依赖构建：

```sh
cargo test --release --lib --locked benchmark_template_scanning -- --ignored --nocapture
```
