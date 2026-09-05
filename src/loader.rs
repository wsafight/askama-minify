use crate::args::{MacroArgs, TemplateInput};
use crate::minifier;
use proc_macro2::Span;
use std::collections::HashMap;
use std::fs;
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

            Ok(LoadedTemplate {
                source: source.value(),
                ext: ext.value(),
                include_paths: Vec::new(),
            })
        }
        TemplateInput::Path(path) => {
            let search = template_search(args)?;
            let resolved =
                resolve_template_path(&path.value(), &search.manifest_dir, &search.template_dirs)
                    .map_err(|message| syn::Error::new_spanned(path, message))?;
            let source = fs::read_to_string(&resolved).map_err(|error| {
                syn::Error::new_spanned(
                    path,
                    format!("failed to read template `{}`: {error}", resolved.display()),
                )
            })?;
            let source = rewrite_relative_dependencies(
                source,
                &resolved,
                &search.manifest_dir,
                &search.template_dirs,
            );
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

            let mut include_paths = vec![resolved];
            include_paths.extend(search.config_path);

            Ok(LoadedTemplate {
                source,
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

fn rewrite_relative_dependencies(
    source: String,
    source_path: &Path,
    manifest_dir: &Path,
    template_dirs: &[PathBuf],
) -> String {
    let mut result = None;
    let mut scan_cursor = 0;
    let mut copied_cursor = 0;

    while let Some(start_offset) = source[scan_cursor..].find("{%") {
        let start = scan_cursor + start_offset;
        let Some(end_offset) = source[start + 2..].find("%}") else {
            break;
        };
        let end = start + 2 + end_offset + 2;
        let block = &source[start..end];

        if let Some(rewritten) =
            rewrite_dependency_block(block, source_path, manifest_dir, template_dirs)
        {
            let result = result.get_or_insert_with(|| String::with_capacity(source.len()));
            result.push_str(&source[copied_cursor..start]);
            result.push_str(&rewritten);
            copied_cursor = end;
        }
        scan_cursor = end;
    }

    let Some(mut result) = result else {
        return source;
    };
    result.push_str(&source[copied_cursor..]);
    result
}

fn rewrite_dependency_block(
    block: &str,
    source_path: &Path,
    manifest_dir: &Path,
    template_dirs: &[PathBuf],
) -> Option<String> {
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
    if original_path.contains('\\') {
        return None;
    }
    let resolved =
        resolve_dependency_path(original_path, source_path, manifest_dir, template_dirs)?;
    let mut replacement = logical_dependency_path(&resolved, template_dirs)
        .unwrap_or_else(|| resolved.to_string_lossy().into_owned());
    if replacement.contains('\\') {
        replacement = replacement.replace('\\', "/");
    }
    if replacement.contains(quote) {
        replacement = replacement.replace(quote, &format!("\\{quote}"));
    }
    if replacement == original_path {
        return None;
    }

    let content_offset = 2 + (inner.len() - content.len());
    let absolute_start = content_offset + path_start;
    let absolute_end = content_offset + path_end;
    let mut rewritten = String::with_capacity(block.len() + replacement.len());
    rewritten.push_str(&block[..absolute_start]);
    rewritten.push_str(&replacement);
    rewritten.push_str(&block[absolute_end..]);
    Some(rewritten)
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

fn logical_dependency_path(path: &Path, template_dirs: &[PathBuf]) -> Option<String> {
    template_dirs.iter().find_map(|dir| {
        path.strip_prefix(dir)
            .ok()
            .map(|relative| relative.to_string_lossy().into_owned())
    })
}

fn extension_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(ToOwned::to_owned)
}
