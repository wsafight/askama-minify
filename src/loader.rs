use crate::args::{MacroArgs, TemplateInput};
use crate::minifier;
use std::fs;
use std::path::{Path, PathBuf};
use syn::LitStr;

pub(crate) struct LoadedTemplate {
    pub(crate) source: String,
    pub(crate) ext: String,
    pub(crate) include_path: Option<PathBuf>,
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
                include_path: None,
            })
        }
        TemplateInput::Path(path) => {
            let resolved = resolve_template_path(&path.value())
                .map_err(|message| syn::Error::new_spanned(path, message))?;
            let source = fs::read_to_string(&resolved).map_err(|error| {
                syn::Error::new_spanned(
                    path,
                    format!("failed to read template `{}`: {error}", resolved.display()),
                )
            })?;
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

            Ok(LoadedTemplate {
                source,
                ext,
                include_path: Some(resolved),
            })
        }
    }
}

pub(crate) fn minify_template_source(source: &str, ext: &str) -> String {
    if matches!(ext.to_ascii_lowercase().as_str(), "html" | "htm") {
        minifier::minify_html(source)
    } else {
        source.to_owned()
    }
}

fn resolve_template_path(path: &str) -> Result<PathBuf, String> {
    let raw = Path::new(path);
    if raw.is_absolute() && raw.is_file() {
        return Ok(raw.to_path_buf());
    }

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "CARGO_MANIFEST_DIR is not set".to_string())?;
    let candidates = [
        manifest_dir.join(raw),
        manifest_dir.join("templates").join(raw),
    ];

    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .ok_or_else(|| {
            let tried = candidates
                .iter()
                .map(|candidate| format!("`{}`", candidate.display()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("template `{path}` was not found; tried {tried}")
        })
}

fn extension_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(ToOwned::to_owned)
}
