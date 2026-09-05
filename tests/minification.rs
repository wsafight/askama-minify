use askama::Template;
use askama_minify::template_minify;

#[template_minify(source = "<div>{{ title }}</div>", ext = "html")]
#[derive(Template)]
struct TemplateSyntax<'a> {
    title: &'a str,
}

#[template_minify(
    source = "{% if enabled %}\n  <span>{{ value }}</span>\n{% endif %}",
    ext = "html"
)]
#[derive(Template)]
struct BlockTemplate<'a> {
    enabled: bool,
    value: &'a str,
}

#[template_minify(
    source = "<div>{# keep as askama comment #}{{ value }}</div>",
    ext = "html"
)]
#[derive(Template)]
struct AskamaCommentTemplate<'a> {
    value: &'a str,
}

#[template_minify(
    source = r#"<div title="a   >   b" data-value='x   y'> text </div>"#,
    ext = "html"
)]
#[derive(Template)]
struct AttributeTemplate;

#[template_minify(source = "<div>   <p>   text   </p>   </div>", ext = "html")]
#[derive(Template)]
struct WhitespaceTemplate;

#[template_minify(source = "<div> a <!-- remove me --> b </div>", ext = "html")]
#[derive(Template)]
struct HtmlCommentTemplate;

#[template_minify(source = "<pre>  code  \n  block  </pre>", ext = "html")]
#[derive(Template)]
struct PreTemplate;

#[template_minify(source = "<textarea>  value  \n  next  </textarea>", ext = "html")]
#[derive(Template)]
struct TextareaTemplate;

#[template_minify(
    source = "<style>/* removed */ body { margin: 0; color: red; }</style>",
    ext = "html"
)]
#[derive(Template)]
struct StyleTemplate;

#[template_minify(source = "<script>// removed\nconst value = 1;</script>", ext = "html")]
#[derive(Template)]
struct ScriptTemplate;

#[template_minify(
    source = "<script type=\"module\">import { value } from './value.js';\nconsole.log(value);</script>",
    ext = "html"
)]
#[derive(Template)]
struct ModuleScriptTemplate;

#[template_minify(
    source = r#"<script>const value = {{ value }};</script>"#,
    ext = "html"
)]
#[derive(Template)]
struct ScriptAskamaTemplate {
    value: i32,
}

#[template_minify(
    source = r#"<style>.box { color: {{ color }}; }</style>"#,
    ext = "html"
)]
#[derive(Template)]
struct StyleAskamaTemplate<'a> {
    color: &'a str,
}

#[template_minify(
    source = r#"<script>const value = "</div>"; const tag = "</style>";</script>"#,
    ext = "html"
)]
#[derive(Template)]
struct ScriptHtmlLikeStringTemplate;

#[template_minify(
    source = r#"<style>.icon::before { content: "</script>"; }</style>"#,
    ext = "html"
)]
#[derive(Template)]
struct StyleHtmlLikeStringTemplate;

#[template_minify(
    source = r#"<script>const a = "// not a comment"; const b = "/* also not */";</script>"#,
    ext = "html"
)]
#[derive(Template)]
struct ScriptCommentMarkerStringTemplate;

#[template_minify(
    source = "<script>function value() { return\n1; }\nconst a = b\n++c;</script>",
    ext = "html"
)]
#[derive(Template)]
struct ScriptLineTerminatorTemplate;

#[template_minify(
    source = "<script>function value() { return/* keep line */\n1; }\nconst a = b// keep line\n++c;</script>",
    ext = "html"
)]
#[derive(Template)]
struct ScriptCommentLineTerminatorTemplate;

#[template_minify(source = "<script>const x = a + +b;</script>", ext = "html")]
#[derive(Template)]
struct ScriptOperatorBoundaryTemplate;

#[template_minify(source = "<script>const re = /[ ]/;</script>", ext = "html")]
#[derive(Template)]
struct ScriptRegexWhitespaceTemplate;

#[template_minify(source = "<textarea><widget   data-value></textarea>", ext = "html")]
#[derive(Template)]
struct TextareaMarkupLikeTextTemplate;

#[template_minify(source = "<style>:root { --OFF: ; }</style>", ext = "html")]
#[derive(Template)]
struct EmptyCustomPropertyTemplate;

#[template_minify(source = "<script type=\"text/plain\">a + +b</script>", ext = "html")]
#[derive(Template)]
struct NonJavascriptScriptTemplate;

#[template_minify(source = "<script>const value = 1;", ext = "html")]
#[derive(Template)]
struct UnclosedScriptTemplate;

#[template_minify(source = "<style>.box { color: red; }", ext = "html")]
#[derive(Template)]
struct UnclosedStyleTemplate;

#[template_minify(
    source = r#"<script>const closing = "<\/script>";</script>"#,
    ext = "html"
)]
#[derive(Template)]
struct EscapedScriptEndTemplate;

#[test]
fn preserves_template_syntax() {
    let rendered = TemplateSyntax { title: "ok" }.render().unwrap();

    assert_eq!(rendered, "<div>ok</div>");
}

#[test]
fn preserves_askama_blocks() {
    let rendered = BlockTemplate {
        enabled: true,
        value: "ok",
    }
    .render()
    .unwrap();

    assert_eq!(rendered, " <span>ok</span> ");
}

#[test]
fn preserves_askama_comments() {
    let rendered = AskamaCommentTemplate { value: "ok" }.render().unwrap();

    assert_eq!(rendered, "<div>ok</div>");
}

#[test]
fn preserves_attribute_values() {
    let rendered = AttributeTemplate.render().unwrap();

    assert_eq!(
        rendered,
        r#"<div title="a   >   b" data-value='x   y'> text </div>"#
    );
}

#[test]
fn removes_whitespace() {
    let rendered = WhitespaceTemplate.render().unwrap();

    assert_eq!(rendered, "<div> <p> text </p> </div>");
}

#[test]
fn removes_html_comments_without_extra_spaces() {
    let rendered = HtmlCommentTemplate.render().unwrap();

    assert_eq!(rendered, "<div> a b </div>");
}

#[test]
fn preserves_pre_content() {
    let rendered = PreTemplate.render().unwrap();

    assert!(rendered.contains("  code  \n  block  "));
}

#[test]
fn preserves_textarea_content() {
    let rendered = TextareaTemplate.render().unwrap();

    assert!(rendered.contains("  value  \n  next  "));
}

#[test]
fn minifies_style_content() {
    let rendered = StyleTemplate.render().unwrap();

    if cfg!(feature = "advanced-css") {
        assert_eq!(rendered, "<style>body{color:red;margin:0}</style>");
    } else {
        assert_eq!(rendered, "<style>body{margin:0;color:red}</style>");
    }
}

#[test]
fn minifies_script_content() {
    let rendered = ScriptTemplate.render().unwrap();

    if cfg!(feature = "js-minify") {
        assert!(!rendered.contains("removed"));
        assert!(rendered.contains("<script>const value=1</script>"));
    } else {
        assert_eq!(rendered, "<script>// removed\nconst value = 1;</script>");
    }
}

#[test]
fn handles_javascript_modules() {
    let rendered = ModuleScriptTemplate.render().unwrap();

    if cfg!(feature = "js-minify") {
        assert_eq!(
            rendered,
            "<script type=\"module\">import{value as a}from\"./value.js\";console.log(a)</script>"
        );
    } else {
        assert_eq!(
            rendered,
            "<script type=\"module\">import { value } from './value.js';\nconsole.log(value);</script>"
        );
    }
}

#[test]
fn preserves_askama_inside_script() {
    let rendered = ScriptAskamaTemplate { value: 1 }.render().unwrap();

    assert_eq!(rendered, "<script>const value = 1;</script>");
}

#[test]
fn preserves_askama_inside_style() {
    let rendered = StyleAskamaTemplate { color: "red" }.render().unwrap();

    assert_eq!(rendered, "<style>.box{color:red}</style>");
}

#[test]
fn keeps_html_like_strings_in_script() {
    let rendered = ScriptHtmlLikeStringTemplate.render().unwrap();

    assert!(rendered.contains("</div>"));
    assert!(rendered.contains("</style>"));
}

#[test]
fn keeps_html_like_strings_in_style() {
    let rendered = StyleHtmlLikeStringTemplate.render().unwrap();

    if cfg!(feature = "advanced-css") {
        assert_eq!(
            rendered,
            r#"<style>.icon:before{content:"</script>"}</style>"#
        );
    } else {
        assert_eq!(
            rendered,
            r#"<style>.icon::before{content:"</script>"}</style>"#
        );
    }
}

#[test]
fn preserves_comment_markers_inside_script_strings() {
    let rendered = ScriptCommentMarkerStringTemplate.render().unwrap();

    assert!(rendered.contains("// not a comment"));
    assert!(rendered.contains("/* also not */"));
}

#[test]
fn preserves_script_line_terminators() {
    let rendered = ScriptLineTerminatorTemplate.render().unwrap();

    assert!(!rendered.contains("return 1"));
    assert!(!rendered.contains("b++c"));
}

#[test]
fn preserves_script_line_terminators_from_comments() {
    let rendered = ScriptCommentLineTerminatorTemplate.render().unwrap();

    assert!(!rendered.contains("return 1"));
    assert!(!rendered.contains("b++c"));
}

#[test]
fn preserves_javascript_operator_boundaries() {
    let rendered = ScriptOperatorBoundaryTemplate.render().unwrap();

    assert!(!rendered.contains("a++b"));
}

#[test]
fn preserves_whitespace_inside_javascript_regexes() {
    let rendered = ScriptRegexWhitespaceTemplate.render().unwrap();

    assert!(rendered.contains("/[ ]/"));
}

#[test]
fn preserves_markup_like_text_inside_textarea() {
    let rendered = TextareaMarkupLikeTextTemplate.render().unwrap();

    assert_eq!(rendered, "<textarea><widget   data-value></textarea>");
}

#[test]
fn preserves_empty_css_custom_properties() {
    let rendered = EmptyCustomPropertyTemplate.render().unwrap();

    assert!(rendered.contains("--OFF: "));
    assert!(!rendered.contains("--OFF:;"));
}

#[test]
fn preserves_non_javascript_script_data() {
    let rendered = NonJavascriptScriptTemplate.render().unwrap();

    assert_eq!(rendered, "<script type=\"text/plain\">a + +b</script>");
}

#[test]
fn flushes_unclosed_script_content_at_eof() {
    let rendered = UnclosedScriptTemplate.render().unwrap();

    assert!(rendered.starts_with("<script>"));
    if cfg!(feature = "js-minify") {
        assert!(rendered.contains("const value=1"));
    } else {
        assert!(rendered.contains("const value = 1;"));
    }
}

#[test]
fn flushes_unclosed_style_content_at_eof() {
    let rendered = UnclosedStyleTemplate.render().unwrap();

    assert_eq!(rendered, "<style>.box{color:red}");
}

#[test]
fn does_not_create_script_end_tags_during_minification() {
    let rendered = EscapedScriptEndTemplate.render().unwrap();

    assert_eq!(rendered.to_ascii_lowercase().matches("</script").count(), 1);
    assert!(rendered.contains("<\\/script>"));
}
