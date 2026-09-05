use crate::args::{MacroArgs, TemplateInput};
use crate::minifier;
use proc_macro2::Span;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use syn::LitStr;

pub(crate) struct LoadedTemplate {
    pub(crate) source: String,
    pub(crate) ext: String,
    pub(crate) include_paths: Vec<PathBuf>,
}

#[derive(Clone)]
struct TemplateSearch {
    manifest_dir: PathBuf,
    template_dirs: Vec<PathBuf>,
    config_path: Option<PathBuf>,
}

#[derive(Eq, Hash, PartialEq)]
struct TemplateSearchKey {
    manifest_dir: PathBuf,
    config_path: Option<PathBuf>,
}

#[cfg(feature = "askama-config")]
#[derive(serde::Deserialize)]
struct AskamaConfig {
    general: Option<GeneralConfig>,
}

#[cfg(feature = "askama-config")]
#[derive(serde::Deserialize)]
struct GeneralConfig {
    dirs: Option<Vec<String>>,
}

pub(crate) fn load_template(args: &MacroArgs) -> syn::Result<LoadedTemplate> {
    match &args.input {
        TemplateInput::Source(source) => {
            let Some(ext) = &args.ext else {
                return Err(syn::Error::new_spanned(
                    source,
                    "`source` templates require `ext = \"...\"`",
                ));
            };
            let search = template_search(args)?;
            let mut processor = DependencyProcessor::new(
                &search.manifest_dir,
                &search.template_dirs,
                std::env::temp_dir().join(format!("askama-minify-{}", std::process::id())),
            );
            let virtual_path = search
                .manifest_dir
                .join(format!("__askama_minify_inline.{}", ext.value()));
            let source = processor
                .rewrite_dependencies(source.value(), &virtual_path)
                .map_err(|message| syn::Error::new_spanned(source, message))?;
            let mut include_paths = processor.include_paths;
            include_paths.extend(search.config_path);

            Ok(LoadedTemplate {
                source: minify_template_source(source, &ext.value()),
                ext: ext.value(),
                include_paths,
            })
        }
        TemplateInput::Path(path) => {
            let search = template_search(args)?;
            let resolved =
                resolve_template_path(&path.value(), &search.manifest_dir, &search.template_dirs)
                    .map_err(|message| syn::Error::new_spanned(path, message))?;
            let ext = args
                .ext
                .as_ref()
                .map(LitStr::value)
                .or_else(|| extension_from_path(&resolved))
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        path,
                        "could not infer template extension; add `ext = \"...\"`",
                    )
                })?;
            let mut processor = DependencyProcessor::new(
                &search.manifest_dir,
                &search.template_dirs,
                std::env::temp_dir().join(format!("askama-minify-{}", std::process::id())),
            );
            let (_, source) = processor
                .process_file(&resolved, Some(&ext))
                .map_err(|message| syn::Error::new_spanned(path, message))?;

            let mut include_paths = processor.include_paths;
            include_paths.extend(search.config_path);

            Ok(LoadedTemplate {
                source: source.expect("the root template is processed exactly once"),
                ext,
                include_paths,
            })
        }
    }
}

pub(crate) fn minify_template_source(source: String, ext: &str) -> String {
    if ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm") {
        minifier::minify_html(&source)
    } else {
        source
    }
}

fn template_search(args: &MacroArgs) -> syn::Result<TemplateSearch> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| syn::Error::new(Span::call_site(), "CARGO_MANIFEST_DIR is not set"))?;
    let config_path = args
        .config
        .as_ref()
        .map(|path| manifest_dir.join(path.value()))
        .or_else(|| {
            let path = manifest_dir.join("askama.toml");
            path.is_file().then_some(path)
        });
    let key = TemplateSearchKey {
        manifest_dir: manifest_dir.clone(),
        config_path: config_path.clone(),
    };

    if let Some(search) = template_search_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
        .cloned()
    {
        return Ok(search);
    }

    #[cfg(feature = "askama-config")]
    let template_dirs = if let Some(config_path) = &config_path {
        let error_span = args
            .config
            .as_ref()
            .map_or_else(Span::call_site, LitStr::span);
        let config = fs::read_to_string(config_path).map_err(|error| {
            syn::Error::new(
                error_span,
                format!(
                    "failed to read Askama config `{}`: {error}",
                    config_path.display()
                ),
            )
        })?;
        let config = basic_toml::from_str::<AskamaConfig>(&config).map_err(|error| {
            syn::Error::new(
                error_span,
                format!(
                    "failed to parse Askama config `{}`: {error}",
                    config_path.display()
                ),
            )
        })?;
        configured_template_dirs(&config, &manifest_dir)
    } else {
        vec![manifest_dir.join("templates")]
    };

    #[cfg(not(feature = "askama-config"))]
    let template_dirs = vec![manifest_dir.join("templates")];

    let search = TemplateSearch {
        manifest_dir,
        template_dirs,
        config_path,
    };
    template_search_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, search.clone());
    Ok(search)
}

fn template_search_cache() -> &'static Mutex<HashMap<TemplateSearchKey, TemplateSearch>> {
    static CACHE: OnceLock<Mutex<HashMap<TemplateSearchKey, TemplateSearch>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "askama-config")]
fn configured_template_dirs(config: &AskamaConfig, manifest_dir: &Path) -> Vec<PathBuf> {
    let Some(dirs) = config
        .general
        .as_ref()
        .and_then(|general| general.dirs.as_ref())
    else {
        return vec![manifest_dir.join("templates")];
    };

    let mut resolved = Vec::new();
    for dir in dirs {
        let path = manifest_dir.join(dir);
        if dir.contains('*') {
            let Some(pattern) = path.to_str() else {
                continue;
            };
            if let Ok(matches) = glob::glob(pattern) {
                resolved.extend(matches.filter_map(Result::ok));
            } else {
                resolved.push(path);
            }
        } else {
            resolved.push(path);
        }
    }
    resolved
}

fn resolve_template_path(
    path: &str,
    manifest_dir: &Path,
    template_dirs: &[PathBuf],
) -> Result<PathBuf, String> {
    let raw = Path::new(path);
    if raw.is_absolute() && raw.is_file() {
        return Ok(raw.to_path_buf());
    }

    let resolved = template_dirs
        .iter()
        .map(|dir| dir.join(raw))
        .chain(std::iter::once_with(|| manifest_dir.join(raw)))
        .find(|candidate| candidate.is_file());
    if let Some(resolved) = resolved {
        return Ok(resolved);
    }

    let tried = template_dirs
        .iter()
        .map(|dir| dir.join(raw))
        .chain(std::iter::once_with(|| manifest_dir.join(raw)))
        .map(|candidate| format!("`{}`", candidate.display()))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("template `{path}` was not found; tried {tried}"))
}

struct DependencyProcessor<'a> {
    manifest_dir: &'a Path,
    template_dirs: &'a [PathBuf],
    generated_dir: PathBuf,
    generated: HashMap<PathBuf, PathBuf>,
    processing: HashSet<PathBuf>,
    include_paths: Vec<PathBuf>,
}

impl<'a> DependencyProcessor<'a> {
    fn new(manifest_dir: &'a Path, template_dirs: &'a [PathBuf], generated_dir: PathBuf) -> Self {
        Self {
            manifest_dir,
            template_dirs,
            generated_dir,
            generated: HashMap::new(),
            processing: HashSet::new(),
            include_paths: Vec::new(),
        }
    }

    fn process_file(
        &mut self,
        source_path: &Path,
        ext_override: Option<&str>,
    ) -> Result<(PathBuf, Option<String>), String> {
        let source_path = fs::canonicalize(source_path).map_err(|error| {
            format!(
                "failed to resolve template `{}`: {error}",
                source_path.display()
            )
        })?;
        if let Some(generated_path) = self.generated.get(&source_path) {
            return Ok((generated_path.clone(), None));
        }
        if !self.processing.insert(source_path.clone()) {
            return Err(format!(
                "cyclic template dependency involving `{}`",
                source_path.display()
            ));
        }
        self.include_paths.push(source_path.clone());

        let source = fs::read_to_string(&source_path).map_err(|error| {
            format!(
                "failed to read template `{}`: {error}",
                source_path.display()
            )
        })?;
        let source = self.rewrite_dependencies(source, &source_path)?;
        let ext = ext_override
            .map(ToOwned::to_owned)
            .or_else(|| extension_from_path(&source_path));
        let source = match ext {
            Some(ext) => minify_template_source(source, &ext),
            None => source,
        };
        let generated_path = generated_dependency_path(&self.generated_dir, &source_path, &source);

        fs::create_dir_all(&self.generated_dir).map_err(|error| {
            format!(
                "failed to create generated template directory `{}`: {error}",
                self.generated_dir.display()
            )
        })?;
        fs::write(&generated_path, &source).map_err(|error| {
            format!(
                "failed to write generated template `{}`: {error}",
                generated_path.display()
            )
        })?;

        self.processing.remove(&source_path);
        self.generated
            .insert(source_path.clone(), generated_path.clone());
        Ok((generated_path, Some(source)))
    }

    fn rewrite_dependencies(
        &mut self,
        source: String,
        source_path: &Path,
    ) -> Result<String, String> {
        let mut result = None;
        let mut scan_cursor = 0;
        let mut copied_cursor = 0;

        while let Some((start, tag)) = next_template_tag(&source, scan_cursor) {
            if tag == TemplateTag::Comment {
                scan_cursor = find_comment_end(&source, start).unwrap_or(source.len());
                continue;
            }

            let end_marker = if tag == TemplateTag::Expression {
                "}}"
            } else {
                "%}"
            };
            let Some(end) = find_tag_end(&source, start + 2, end_marker) else {
                break;
            };
            if tag == TemplateTag::Expression {
                scan_cursor = end;
                continue;
            }

            let block = &source[start..end];
            if block_keyword(block) == "raw" {
                scan_cursor = find_raw_end(&source, end).unwrap_or(source.len());
                continue;
            }

            if let Some(reference) = dependency_reference(block)
                && !reference.path.contains('\\')
                && let Some(resolved) = resolve_dependency_path(
                    reference.path,
                    source_path,
                    self.manifest_dir,
                    self.template_dirs,
                )
            {
                let (generated_path, _) = self.process_file(&resolved, None)?;
                let replacement = escaped_dependency_path(&generated_path, reference.quote);
                let result = result.get_or_insert_with(|| String::with_capacity(source.len()));
                result.push_str(&source[copied_cursor..start]);
                result.push_str(&block[..reference.path_start]);
                result.push_str(&replacement);
                result.push_str(&block[reference.path_end..]);
                copied_cursor = end;
            }
            scan_cursor = end;
        }

        let Some(mut result) = result else {
            return Ok(source);
        };
        result.push_str(&source[copied_cursor..]);
        Ok(result)
    }
}

struct DependencyReference<'a> {
    path: &'a str,
    path_start: usize,
    path_end: usize,
    quote: char,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TemplateTag {
    Block,
    Expression,
    Comment,
}

fn next_template_tag(source: &str, cursor: usize) -> Option<(usize, TemplateTag)> {
    [
        ("{%", TemplateTag::Block),
        ("{{", TemplateTag::Expression),
        ("{#", TemplateTag::Comment),
    ]
    .into_iter()
    .filter_map(|(opening, tag)| {
        source[cursor..]
            .find(opening)
            .map(|offset| (cursor + offset, tag))
    })
    .min_by_key(|(position, _)| *position)
}

fn find_tag_end(source: &str, start: usize, marker: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;

    for (offset, ch) in source[start..].char_indices() {
        let position = start + offset;
        if let Some(active_quote) = quote {
            if ch == active_quote && !escaped {
                quote = None;
            }
            escaped = ch == '\\' && !escaped;
            if ch != '\\' {
                escaped = false;
            }
        } else if matches!(ch, '"' | '\'') {
            quote = Some(ch);
        } else if source[position..].starts_with(marker) {
            return Some(position + marker.len());
        }
    }
    None
}

fn find_comment_end(source: &str, start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = start + 2;

    loop {
        let opening = source[cursor..].find("{#").map(|offset| cursor + offset);
        let closing = source[cursor..].find("#}").map(|offset| cursor + offset);
        match (opening, closing) {
            (Some(opening), Some(closing)) if opening < closing => {
                depth += 1;
                cursor = opening + 2;
            }
            (_, Some(closing)) => {
                depth -= 1;
                cursor = closing + 2;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => return None,
        }
    }
}

fn find_raw_end(source: &str, mut cursor: usize) -> Option<usize> {
    while let Some(offset) = source[cursor..].find("{%") {
        let start = cursor + offset;
        let end = find_tag_end(source, start + 2, "%}")?;
        if block_keyword(&source[start..end]) == "endraw" {
            return Some(end);
        }
        cursor = end;
    }
    None
}

fn block_keyword(block: &str) -> &str {
    let Some(inner) = block
        .strip_prefix("{%")
        .and_then(|block| block.strip_suffix("%}"))
    else {
        return "";
    };
    let content = inner.trim_start_matches(|ch: char| ch.is_whitespace() || "-+~".contains(ch));
    let keyword_end = content
        .find(|ch: char| !ch.is_ascii_alphabetic())
        .unwrap_or(content.len());
    &content[..keyword_end]
}

fn dependency_reference(block: &str) -> Option<DependencyReference<'_>> {
    let inner = &block[2..block.len().checked_sub(2)?];
    let content = inner.trim_start_matches(|ch: char| ch.is_whitespace() || "-+~".contains(ch));
    let keyword_end = content
        .find(|ch: char| !ch.is_ascii_alphabetic())
        .unwrap_or(content.len());
    let keyword = &content[..keyword_end];
    if !matches!(keyword, "include" | "extends" | "import") {
        return None;
    }

    let arguments = &content[keyword_end..];
    let leading = arguments.len() - arguments.trim_start().len();
    let quote_index = keyword_end + leading;
    let quote = content[quote_index..].chars().next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }

    let path_start = quote_index + quote.len_utf8();
    let path_end = find_unescaped_quote(content, path_start, quote)?;
    let original_path = &content[path_start..path_end];

    let content_offset = 2 + (inner.len() - content.len());
    Some(DependencyReference {
        path: original_path,
        path_start: content_offset + path_start,
        path_end: content_offset + path_end,
        quote,
    })
}

fn find_unescaped_quote(value: &str, start: usize, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (offset, ch) in value[start..].char_indices() {
        if ch == quote && !escaped {
            return Some(start + offset);
        }
        escaped = ch == '\\' && !escaped;
        if ch != '\\' {
            escaped = false;
        }
    }
    None
}

fn resolve_dependency_path(
    path: &str,
    source_path: &Path,
    manifest_dir: &Path,
    template_dirs: &[PathBuf],
) -> Option<PathBuf> {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return raw.is_file().then(|| raw.to_path_buf());
    }

    let relative = source_path.parent()?.join(raw);
    if relative.is_file() {
        return Some(relative);
    }
    template_dirs
        .iter()
        .map(|dir| dir.join(raw))
        .chain(std::iter::once_with(|| manifest_dir.join(raw)))
        .find(|candidate| candidate.is_file())
}

fn generated_dependency_path(generated_dir: &Path, source_path: &Path, source: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source_path.hash(&mut hasher);
    source.hash(&mut hasher);
    let mut file_name = format!("{:016x}", hasher.finish());
    if let Some(extension) = source_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        file_name.push('.');
        file_name.push_str(extension);
    }
    generated_dir.join(file_name)
}

fn escaped_dependency_path(path: &Path, quote: char) -> String {
    let mut path = path.to_string_lossy().replace('\\', "/");
    if path.contains(quote) {
        path = path.replace(quote, &format!("\\{quote}"));
    }
    path
}

fn extension_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_end_ignores_markers_inside_strings() {
        let source = r#"{% include "name%}.html" %}suffix"#;

        assert_eq!(find_tag_end(source, 2, "%}"), Some(source.len() - 6));
    }

    #[test]
    fn nested_comments_hide_dependency_tags() {
        let source = r#"{# outer {# {% include "ignored.html" %} #} #}{% include "used.html" %}"#;
        let (_, tag) = next_template_tag(source, 0).unwrap();
        let cursor = find_comment_end(source, 0).unwrap();
        let (start, next_tag) = next_template_tag(source, cursor).unwrap();

        assert_eq!(tag, TemplateTag::Comment);
        assert_eq!(next_tag, TemplateTag::Block);
        assert_eq!(
            dependency_reference(&source[start..]).unwrap().path,
            "used.html"
        );
    }

    #[test]
    fn raw_blocks_hide_dependency_tags() {
        let source =
            r#"{% raw %}{% include "ignored.html" %}{% endraw %}{% include "used.html" %}"#;
        let first_end = find_tag_end(source, 2, "%}").unwrap();
        let cursor = find_raw_end(source, first_end).unwrap();
        let (start, _) = next_template_tag(source, cursor).unwrap();

        assert_eq!(
            dependency_reference(&source[start..]).unwrap().path,
            "used.html"
        );
    }
}
