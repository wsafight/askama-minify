use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<div>   {{ value }}   </div>", ext = "html")]
#[derive(Template)]
struct SourceTemplate<'a> {
    value: &'a str,
}

#[template_minify(path = "tests/templates/basic.html")]
#[derive(Template)]
struct FileTemplate<'a> {
    title: &'a str,
}

#[template_minify(path = "from_templates_dir.html")]
#[derive(Template)]
struct TemplatesDirTemplate<'a> {
    name: &'a str,
}

#[template_minify(path = "tests/templates/no_extension", ext = "html")]
#[derive(Template)]
struct ExtOverrideTemplate<'a> {
    value: &'a str,
}

#[template_minify(source = "<div>{{ raw }}</div>", ext = "html", escape = "none")]
#[derive(Template)]
struct EscapeNoneTemplate<'a> {
    raw: &'a str,
}

#[template_minify(source = "line 1\n  {{ value }}\nline 3", ext = "txt")]
#[derive(Template)]
struct TextTemplate<'a> {
    value: &'a str,
}

#[template_minify(
    source = r#"
        <style>
            /* removed */
            .box {
                color: red;
                margin: 0;
            }
        </style>
        <script>
            // removed
            const value = 1;
        </script>
    "#,
    ext = "html"
)]
#[derive(Template)]
struct AssetTemplate;

#[template_minify(path = "nested/page.html")]
#[derive(Template)]
struct RelativeIncludeTemplate<'a> {
    value: &'a str,
}

#[cfg(feature = "askama-config")]
#[template_minify(path = "custom.html", config = "tests/askama-custom.toml")]
#[derive(Template)]
struct CustomTemplateDirectory<'a> {
    value: &'a str,
}

#[test]
fn renders_minified_inline_source() {
    let rendered = SourceTemplate { value: "ok" }.render().unwrap();
    assert_eq!(rendered, "<div> ok </div>");
}

#[test]
fn renders_minified_template_file() {
    let rendered = FileTemplate { title: "Hello" }.render().unwrap();
    assert_eq!(rendered, "<section> <h1>Hello</h1> </section>");
}

#[test]
fn resolves_paths_from_templates_directory() {
    let rendered = TemplatesDirTemplate { name: "Ada" }.render().unwrap();
    assert_eq!(rendered, "<main> <p>Ada</p> </main>");
}

#[test]
fn supports_extension_override_for_extensionless_files() {
    let rendered = ExtOverrideTemplate { value: "42" }.render().unwrap();
    assert_eq!(rendered, "<p>42</p>");
}

#[test]
fn forwards_askama_template_arguments() {
    let rendered = EscapeNoneTemplate {
        raw: "<strong>trusted</strong>",
    }
    .render()
    .unwrap();

    assert_eq!(rendered, "<div><strong>trusted</strong></div>");
}

#[test]
fn leaves_non_html_templates_unminified() {
    let rendered = TextTemplate { value: "ok" }.render().unwrap();

    assert_eq!(rendered, "line 1\n  ok\nline 3");
}

#[test]
fn minifies_embedded_css_and_javascript() {
    let rendered = AssetTemplate.render().unwrap();

    assert!(rendered.contains("<style>.box{color:red;margin:0}</style>"));
    if cfg!(feature = "js-minify") {
        assert!(rendered.contains("<script>const value=1</script>"));
        assert!(!rendered.contains("removed"));
    } else {
        assert!(rendered.contains("// removed"));
        assert!(rendered.contains("const value = 1;"));
    }
}

#[test]
fn preserves_relative_include_resolution() {
    let rendered = RelativeIncludeTemplate { value: "ok" }.render().unwrap();

    assert_eq!(rendered, "<main><p>ok</p></main>");
}

#[cfg(feature = "askama-config")]
#[test]
fn resolves_paths_from_askama_config() {
    let rendered = CustomTemplateDirectory { value: "ok" }.render().unwrap();

    assert_eq!(rendered, "<article> ok </article>");
}
