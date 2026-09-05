use crate::args::{MacroArgs, TemplateInput};
use crate::minifier;
use crate::minifier::template::{
    TemplateTag, block_keyword, find_comment_end, find_raw_end, find_tag_end, next_template_tag,
};
use proc_macro2::Span;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;
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
            processor.sensitive_context = !is_html(&ext.value());
            processor
                .prepare_source(&source.value(), &virtual_path)
                .map_err(|message| syn::Error::new_spanned(source, message))?;
            let source = processor
                .rewrite_dependencies(source.value(), &virtual_path)
                .map_err(|message| syn::Error::new_spanned(source, message))?;
            let source = processor.minify_source(source, &ext.value(), false);
            let mut include_paths = processor.include_paths;
            include_paths.extend(search.config_path);

            Ok(LoadedTemplate {
                source,
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
            processor.sensitive_context = !is_html(&ext);
            processor
                .prepare_file(&resolved)
                .map_err(|message| syn::Error::new_spanned(path, message))?;
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

fn is_html(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")
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
    sources: HashMap<PathBuf, Arc<str>>,
    sensitive_context: bool,
    has_dependencies: bool,
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
            sources: HashMap::new(),
            sensitive_context: false,
            has_dependencies: false,
        }
    }

    fn prepare_file(&mut self, source_path: &Path) -> Result<(), String> {
        let source_path = fs::canonicalize(source_path).map_err(|error| {
            format!(
                "failed to resolve template `{}`: {error}",
                source_path.display()
            )
        })?;
        if self.sources.contains_key(&source_path) {
            return Ok(());
        }
        let file = read_template_file(&source_path)?;
        self.include_paths.push(source_path.clone());
        self.sources
            .insert(source_path.clone(), Arc::clone(&file.source));
        self.sensitive_context |= file.sensitive_context;
        self.prepare_dependencies(&file.source, &source_path)
    }

    fn prepare_source(&mut self, source: &str, source_path: &Path) -> Result<(), String> {
        self.sensitive_context |= minifier::has_sensitive_template_context(source);
        self.prepare_dependencies(source, source_path)
    }

    fn prepare_dependencies(&mut self, source: &str, source_path: &Path) -> Result<(), String> {
        for (_, reference) in dependency_references(source) {
            if let Some(resolved) = self.resolve_dependency(&reference, source_path) {
                self.has_dependencies = true;
                self.prepare_file(&resolved)?;
            }
        }
        Ok(())
    }

    fn resolve_dependency(
        &self,
        reference: &DependencyReference<'_>,
        source_path: &Path,
    ) -> Option<PathBuf> {
        if reference.path.contains('\\') {
            return None;
        }
        resolve_dependency_path(
            reference.path,
            source_path,
            self.manifest_dir,
            self.template_dirs,
        )
    }

    fn minify_source(&self, source: String, ext: &str, fragment: bool) -> String {
        if (self.sensitive_context && self.has_dependencies) || !is_html(ext) {
            return source;
        }
        let key = MinificationKey { source, fragment };
        if let Some(result) = minification_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&key)
            .cloned()
        {
            return result;
        }
        let result = if fragment {
            minifier::minify_html_fragment(&key.source)
        } else {
            minifier::minify_html(&key.source)
        };
        minification_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key, result.clone());
        result
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
        let source = self.sources[&source_path].to_string();
        let source = self.rewrite_dependencies(source, &source_path)?;
        let ext = ext_override
            .map(ToOwned::to_owned)
            .or_else(|| extension_from_path(&source_path));
        let source = match ext {
            Some(ext) => self.minify_source(source, &ext, ext_override.is_none()),
            None => source,
        };
        let generated_path = generated_dependency_path(&self.generated_dir, &source_path, &source);

        if ext_override.is_none() {
            write_generated_template(&generated_path, &source)?;
        }

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
        let mut copied_cursor = 0;

        for ((start, end), reference) in dependency_references(&source) {
            let block = &source[start..end];
            if let Some(resolved) = self.resolve_dependency(&reference, source_path) {
                let (generated_path, _) = self.process_file(&resolved, None)?;
                let replacement = escaped_dependency_path(&generated_path, reference.quote);
                let result = result.get_or_insert_with(|| String::with_capacity(source.len()));
                result.push_str(&source[copied_cursor..start]);
                result.push_str(&block[..reference.path_start]);
                result.push_str(&replacement);
                result.push_str(&block[reference.path_end..]);
                copied_cursor = end;
            }
        }

        let Some(mut result) = result else {
            return Ok(source);
        };
        result.push_str(&source[copied_cursor..]);
        Ok(result)
    }
}

#[derive(Clone)]
struct CachedTemplateFile {
    modified: Option<SystemTime>,
    len: u64,
    source: Arc<str>,
    sensitive_context: bool,
}

fn read_template_file(path: &Path) -> Result<CachedTemplateFile, String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedTemplateFile>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let read_error = |error| format!("failed to read template `{}`: {error}", path.display());
    let metadata = fs::metadata(path).map_err(read_error)?;
    let modified = metadata.modified().ok();
    if let Some(file) = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(path)
        .filter(|file| {
            modified.is_some() && file.modified == modified && file.len == metadata.len()
        })
        .cloned()
    {
        return Ok(file);
    }
    let mut source = fs::read_to_string(path).map_err(read_error)?;
    // Askama removes one final newline when reading a template file.
    if source.ends_with('\n') {
        source.pop();
    }
    let file = CachedTemplateFile {
        modified,
        len: metadata.len(),
        sensitive_context: minifier::has_sensitive_template_context(&source),
        source: Arc::from(source),
    };
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(path.to_path_buf(), file.clone());
    Ok(file)
}

#[derive(Eq, Hash, PartialEq)]
struct MinificationKey {
    source: String,
    fragment: bool,
}

fn minification_cache() -> &'static Mutex<HashMap<MinificationKey, String>> {
    static CACHE: OnceLock<Mutex<HashMap<MinificationKey, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn write_generated_template(path: &Path, source: &str) -> Result<(), String> {
    static WRITTEN: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let mut written = WRITTEN
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if written.contains(path) && path.is_file() {
        return Ok(());
    }
    let directory = path
        .parent()
        .expect("generated templates have a parent directory");
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "failed to create generated template directory `{}`: {error}",
            directory.display()
        )
    })?;
    // Compensate for the newline Askama removes from each generated file.
    fs::write(path, format!("{source}\n")).map_err(|error| {
        format!(
            "failed to write generated template `{}`: {error}",
            path.display()
        )
    })?;
    written.insert(path.to_path_buf());
    Ok(())
}

fn dependency_references(
    source: &str,
) -> impl Iterator<Item = ((usize, usize), DependencyReference<'_>)> {
    let mut cursor = 0;
    std::iter::from_fn(move || {
        while let Some((start, tag)) = next_template_tag(source, cursor) {
            if tag == TemplateTag::Comment {
                cursor = find_comment_end(source, start).unwrap_or(source.len());
                continue;
            }
            let marker = if tag == TemplateTag::Expression {
                "}}"
            } else {
                "%}"
            };
            let end = find_tag_end(source, start + 2, marker)?;
            cursor = end;
            if tag == TemplateTag::Expression {
                continue;
            }
            let block = &source[start..end];
            if block_keyword(block) == "raw" {
                cursor = find_raw_end(source, end).unwrap_or(source.len());
                continue;
            }
            if let Some(reference) = dependency_reference(block) {
                return Some(((start, end), reference));
            }
        }
        None
    })
}

struct DependencyReference<'a> {
    path: &'a str,
    path_start: usize,
    path_end: usize,
    quote: char,
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestProject(PathBuf);

    impl TestProject {
        fn new() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "askama-minify-test-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, source: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, source).unwrap();
            path
        }

        fn processor(&self) -> DependencyProcessor<'_> {
            DependencyProcessor::new(&self.0, &[], self.0.join("generated"))
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

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

    #[test]
    fn detects_cycles_after_preparing_dependencies() {
        let project = TestProject::new();
        let root = project.write("root.html", r#"{% include "child.html" %}"#);
        project.write("child.html", r#"{% include "root.html" %}"#);
        let mut processor = project.processor();
        processor.prepare_file(&root).unwrap();

        assert!(
            processor
                .process_file(&root, Some("html"))
                .unwrap_err()
                .contains("cyclic template dependency")
        );
    }

    #[test]
    fn refreshes_cached_files_when_the_source_changes() {
        let project = TestProject::new();
        let path = project.write("page.html", "<p>old</p>\n");
        let first = read_template_file(&path).unwrap();
        let cached = read_template_file(&path).unwrap();
        assert!(Arc::ptr_eq(&first.source, &cached.source));

        project.write("page.html", "<p>updated</p>\n");
        let updated = read_template_file(&path).unwrap();
        assert_eq!(&*updated.source, "<p>updated</p>");
    }

    #[test]
    fn tracks_cached_dependencies_and_recreates_missing_generated_files() {
        let project = TestProject::new();
        let root = project.write("root.html", r#"<main>{% include "child.html" %}</main>"#);
        let child = project.write("child.html", " <p>  child  </p> \n");
        let mut first = project.processor();
        first.prepare_file(&root).unwrap();
        let (root_output_path, source) = first.process_file(&root, Some("html")).unwrap();
        assert!(!root_output_path.exists());
        let generated = first.generated[&fs::canonicalize(&child).unwrap()].clone();
        let original_mtime = fs::metadata(&generated).unwrap().modified().unwrap();

        let mut second = project.processor();
        second.prepare_file(&root).unwrap();
        assert_eq!(second.process_file(&root, Some("html")).unwrap().1, source);
        assert_eq!(second.include_paths, first.include_paths);
        assert_eq!(
            fs::metadata(&generated).unwrap().modified().unwrap(),
            original_mtime
        );

        fs::remove_file(&generated).unwrap();
        let mut third = project.processor();
        third.prepare_file(&root).unwrap();
        third.process_file(&root, Some("html")).unwrap();
        assert_eq!(
            fs::read_to_string(&generated).unwrap(),
            " <p> child </p> \n"
        );
    }

    #[test]
    fn separates_document_fragment_and_preserved_outputs_in_the_cache() {
        let project = TestProject::new();
        let mut processor = project.processor();
        let source = "  words   here  ";
        assert_eq!(
            processor.minify_source(source.into(), "html", false),
            "words here"
        );
        assert_eq!(
            processor.minify_source(source.into(), "html", true),
            " words here "
        );
        processor.sensitive_context = true;
        processor.has_dependencies = true;
        assert_eq!(processor.minify_source(source.into(), "html", true), source);
        assert_eq!(processor.minify_source(source.into(), "txt", false), source);
    }

    #[test]
    #[ignore = "run with --release --ignored --nocapture to measure template scanning"]
    fn benchmark_template_scanning() {
        for count in [2_000, 4_000, 8_000, 16_000, 32_000] {
            let input = "{{ value }}".repeat(count);
            let processor = TestProject::new();
            let start = std::time::Instant::now();
            std::hint::black_box(
                processor
                    .processor()
                    .rewrite_dependencies(input, Path::new("inline.html"))
                    .unwrap(),
            );
            println!(
                "{count} expressions: {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}
