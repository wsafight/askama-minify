use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::minifier;

/// 判断是否为模板文件
pub fn is_template_file(path: &Path) -> bool {
    // 跳过已经压缩的 .min.html 文件
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if file_name.contains(".min.") {
        return false;
    }

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(ext_str.as_str(), "html" | "htm" | "xml" | "svg")
    } else {
        false
    }
}

/// 根据文件路径和后缀生成输出路径
pub fn generate_output_path(path: &Path, suffix: &str) -> PathBuf {
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.{}.{}", file_stem, suffix, extension))
}

/// 压缩单个文件并返回文件大小信息
pub fn minify_file(input_path: &Path, output_path: &Path) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(input_path)?;
    let original_size = content.len();
    let minified = minifier::minify_html(&content);
    let minified_size = minified.len();
    fs::write(output_path, minified)?;
    Ok((original_size, minified_size))
}

/// 处理单个文件的压缩
pub fn process_single_file(
    path: &Path,
    output: Option<&PathBuf>,
    suffix: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = if let Some(output) = output {
        output.clone()
    } else {
        // 没有指定 output 时，使用 suffix（默认为 "min"）
        let suffix = suffix.unwrap_or("min");
        generate_output_path(path, suffix)
    };

    let (original_size, minified_size) = minify_file(path, &output_path)?;
    let reduction = if original_size > 0 {
        ((original_size - minified_size) as f64 / original_size as f64 * 100.0) as i32
    } else {
        0
    };

    println!(
        "✓ 已压缩: {} -> {} ({} → {} bytes, -{}%)",
        path.display(),
        output_path.display(),
        original_size,
        minified_size,
        reduction
    );
    Ok(())
}

/// 处理文件夹的压缩
pub fn process_directory(
    path: &Path,
    output: Option<&PathBuf>,
    suffix: Option<&str>,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = if let Some(output) = output {
        // 创建输出目录
        fs::create_dir_all(output)?;
        output.clone()
    } else {
        path.to_path_buf()
    };

    let mut count = 0;
    let mut total_original_size = 0;
    let mut total_minified_size = 0;

    let walker = if recursive {
        WalkDir::new(path)
    } else {
        WalkDir::new(path).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();
        if file_path.is_file() && is_template_file(file_path) {
            let output_path = if output.is_some() {
                // 保持相对路径结构
                let relative = file_path.strip_prefix(path).unwrap();
                let target = output_dir.join(relative);

                // 创建父目录
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }

                // 如果指定了 output 但没有指定 suffix，则不添加后缀
                if let Some(suffix) = suffix {
                    generate_output_path(&target, suffix)
                } else {
                    target
                }
            } else {
                // 没有指定 output 时，使用 suffix（默认为 "min"）
                let suffix = suffix.unwrap_or("min");
                generate_output_path(file_path, suffix)
            };

            match minify_file(file_path, &output_path) {
                Ok((original_size, minified_size)) => {
                    let reduction = if original_size > 0 {
                        ((original_size - minified_size) as f64 / original_size as f64 * 100.0) as i32
                    } else {
                        0
                    };

                    println!(
                        "✓ 已压缩: {} -> {} ({} → {} bytes, -{}%)",
                        file_path.display(),
                        output_path.display(),
                        original_size,
                        minified_size,
                        reduction
                    );
                    count += 1;
                    total_original_size += original_size;
                    total_minified_size += minified_size;
                }
                Err(e) => {
                    eprintln!("✗ 压缩失败 {}: {}", file_path.display(), e);
                }
            }
        }
    }

    let total_reduction = if total_original_size > 0 {
        ((total_original_size - total_minified_size) as f64 / total_original_size as f64 * 100.0) as i32
    } else {
        0
    };

    println!(
        "\n总共压缩了 {} 个文件 ({} → {} bytes, 总压缩率: {}%)",
        count, total_original_size, total_minified_size, total_reduction
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_template_file() {
        assert!(is_template_file(Path::new("test.html")));
        assert!(is_template_file(Path::new("test.htm")));
        assert!(!is_template_file(Path::new("test.min.html")));
        assert!(!is_template_file(Path::new("test.txt")));
    }

    #[test]
    fn test_generate_output_path() {
        let path = Path::new("test.html");
        let output = generate_output_path(path, "min");
        assert_eq!(output, PathBuf::from("test.min.html"));
    }
}
