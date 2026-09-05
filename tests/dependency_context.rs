use askama::Template;
use askama_minify::template_minify;

macro_rules! assert_preserved {
    ($source:tt) => {{
        #[derive(Template)]
        #[template(source = $source, ext = "html")]
        struct Original;

        #[template_minify(source = $source, ext = "html")]
        #[derive(Template)]
        struct Minified;

        assert_eq!(Minified.render().unwrap(), Original.render().unwrap());
    }};
}

#[test]
fn preserves_include_boundaries() {
    assert_preserved!(r#"<span>A</span>{% include "context/words.html" %}<span>B</span>"#);
}

#[test]
fn preserves_nested_includes_in_pre() {
    assert_preserved!(r#"<pre>{% include "context/nested.html" %}</pre>"#);
}

#[test]
fn preserves_includes_in_textarea_and_attributes() {
    assert_preserved!(r#"<textarea>{% include "context/fragment.html" %}</textarea>"#);
    assert_preserved!(r#"<div title="{% include "context/fragment.html" %}">ok</div>"#);
}

#[test]
fn preserves_includes_in_script_and_style() {
    assert_preserved!(
        r#"<script type="text/plain">{% include "context/fragment.html" %}</script>"#
    );
    assert_preserved!(r#"<style>{% include "context/fragment.html" %}</style>"#);
}

#[test]
fn preserves_blocks_in_sensitive_parent_contexts() {
    #[derive(Template)]
    #[template(path = "context/pre_page.html")]
    struct Original;

    #[template_minify(path = "context/pre_page.html")]
    #[derive(Template)]
    struct Minified;

    assert_eq!(Minified.render().unwrap(), Original.render().unwrap());
}

#[test]
fn keeps_minifying_graphs_with_ordinary_dynamic_attributes() {
    #[template_minify(
        source = r#"<!-- removed --><main title="{{ title }}">{% include "context/words.html" %}</main>"#,
        ext = "html"
    )]
    #[derive(Template)]
    struct WithAttribute<'a> {
        title: &'a str,
    }

    assert_eq!(
        WithAttribute { title: "ok" }.render().unwrap(),
        "<main title=\"ok\"> middle </main>"
    );
}

#[test]
fn preserves_html_dependencies_in_text_templates() {
    #[derive(Template)]
    #[template(source = r#"{% include "context/fragment.html" %}"#, ext = "txt")]
    struct Original;

    #[template_minify(source = r#"{% include "context/fragment.html" %}"#, ext = "txt")]
    #[derive(Template)]
    struct Minified;

    assert_eq!(Minified.render().unwrap(), Original.render().unwrap());
}
