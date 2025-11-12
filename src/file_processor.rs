use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::minifier;

// 常量定义
const DEFAULT_SUFFIX: &str = "min";
const MIN_MARKER: &str = ".min.";
const VALID_EXTENSIONS: &[&str] = &["html", "htm", "xml", "svg"];

/// 计算压缩率（百分比）
fn calculate_reduction_percent(original_size: usize, minified_size: usize) -> i32 {
    if original_size > 0 {
        ((original_size - minified_size) as f64 / original_size as f64 * 100.0) as i32
    } else {
        0
    }
}

/// 打印文件压缩结果
fn print_compression_result(
    input_path: &Path,
    output_path: &Path,
    original_size: usize,
    minified_size: usize,
) {
    let reduction = calculate_reduction_percent(original_size, minified_size);
    println!(
        "✓ 已压缩: {} -> {} ({} → {} bytes, -{}%)",
        input_path.display(),
        output_path.display(),
        original_size,
        minified_size,
        reduction
    );
}

/// 判断是否为模板文件
pub fn is_template_file(path: &Path) -> bool {
    // 跳过已经压缩的文件
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    if file_name.contains(MIN_MARKER) {
        return false;
    }

    // 优化：使用 eq_ignore_ascii_case 避免创建新字符串
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext_str| {
            VALID_EXTENSIONS
                .iter()
                .any(|&valid_ext| ext_str.eq_ignore_ascii_case(valid_ext))
        })
        .unwrap_or(false)
}

/// 根据文件路径和后缀生成输出路径
pub fn generate_output_path(path: &Path, suffix: &str) -> Result<PathBuf, String> {
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("无效的文件名: {}", path.display()))?;

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let parent = path.parent().unwrap_or(Path::new("."));

    let output_name = if extension.is_empty() {
        format!("{}.{}", file_stem, suffix)
    } else {
        format!("{}.{}.{}", file_stem, suffix, extension)
    };

    Ok(parent.join(output_name))
}

/// 压缩单个文件并返回文件大小信息
pub fn minify_file(
    input_path: &Path,
    output_path: &Path,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(input_path)?;
    let original_size = content.len();

    // 优化：空文件直接复制
    if original_size == 0 {
        fs::write(output_path, "")?;
        return Ok((0, 0));
    }

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
        let suffix = suffix.unwrap_or(DEFAULT_SUFFIX);
        generate_output_path(path, suffix)
            .map_err(|e| format!("生成输出路径失败: {}", e))?
    };

    let (original_size, minified_size) = minify_file(path, &output_path)?;
    print_compression_result(path, &output_path, original_size, minified_size);
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
        fs::create_dir_all(output)?;
        output.clone()
    } else {
        path.to_path_buf()
    };

    let mut success_count = 0;
    let mut error_count = 0;
    let mut total_original_size = 0;
    let mut total_minified_size = 0;

    let walker = if recursive {
        WalkDir::new(path)
    } else {
        WalkDir::new(path).max_depth(1)
    };

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();
        if !file_path.is_file() || !is_template_file(file_path) {
            continue;
        }

        let output_path = if output.is_some() {
            // 保持相对路径结构
            let relative = file_path.strip_prefix(path)?;
            let target = output_dir.join(relative);

            // 创建父目录
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }

            // 如果指定了 output 但没有指定 suffix，则不添加后缀
            if let Some(suffix) = suffix {
                generate_output_path(&target, suffix)
                    .map_err(|e| format!("生成输出路径失败: {}", e))?
            } else {
                target
            }
        } else {
            let suffix = suffix.unwrap_or(DEFAULT_SUFFIX);
            generate_output_path(file_path, suffix)
                .map_err(|e| format!("生成输出路径失败: {}", e))?
        };

        match minify_file(file_path, &output_path) {
            Ok((original_size, minified_size)) => {
                print_compression_result(file_path, &output_path, original_size, minified_size);
                success_count += 1;
                total_original_size += original_size;
                total_minified_size += minified_size;
            }
            Err(e) => {
                eprintln!("✗ 压缩失败 {}: {}", file_path.display(), e);
                error_count += 1;
            }
        }
    }

    let total_reduction = calculate_reduction_percent(total_original_size, total_minified_size);

    if error_count > 0 {
        println!(
            "\n总共处理了 {} 个文件：{} 个成功，{} 个失败 ({} → {} bytes, 总压缩率: {}%)",
            success_count + error_count,
            success_count,
            error_count,
            total_original_size,
            total_minified_size,
            total_reduction
        );
    } else {
        println!(
            "\n总共压缩了 {} 个文件 ({} → {} bytes, 总压缩率: {}%)",
            success_count, total_original_size, total_minified_size, total_reduction
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_template_file() {
        assert!(is_template_file(Path::new("test.html")));
        assert!(is_template_file(Path::new("test.HTML")));
        assert!(is_template_file(Path::new("test.htm")));
        assert!(!is_template_file(Path::new("test.min.html")));
        assert!(!is_template_file(Path::new("test.txt")));
    }

    #[test]
    fn test_generate_output_path() {
        let path = Path::new("test.html");
        let output = generate_output_path(path, "min").unwrap();
        assert_eq!(output, PathBuf::from("test.min.html"));
    }

    #[test]
    fn test_calculate_reduction_percent() {
        assert_eq!(calculate_reduction_percent(100, 50), 50);
        assert_eq!(calculate_reduction_percent(100, 0), 100);
        assert_eq!(calculate_reduction_percent(0, 0), 0);
    }
}
