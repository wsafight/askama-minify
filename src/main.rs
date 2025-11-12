use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Askama 模板压缩工具
#[derive(Parser, Debug)]
#[command(name = "askama-minify")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 要压缩的文件或文件夹路径
    #[arg(value_name = "PATH")]
    path: PathBuf,

    /// 递归处理文件夹（默认启用）
    #[arg(short, long, default_value_t = true)]
    recursive: bool,

    /// 输出文件或文件夹路径（如果已存在则报错）
    #[arg(short = 'd', long)]
    output: Option<PathBuf>,

    /// 输出文件的后缀名（例如: "min" 会生成 .min.html）
    /// 如果指定了 output 但未指定 suffix，则不添加后缀
    #[arg(short = 's', long)]
    suffix: Option<String>,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let path = &args.path;

    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()).into());
    }

    // 验证输出路径
    if let Some(ref output) = args.output {
        if output.exists() {
            return Err(format!("输出路径已存在: {}", output.display()).into());
        }
    }

    if path.is_file() {
        let output_path = if let Some(ref output) = args.output {
            output.clone()
        } else {
            // 没有指定 output 时，使用 suffix（默认为 "min"）
            let suffix = args.suffix.as_deref().unwrap_or("min");
            generate_output_path(path, suffix)
        };

        minify_file(path, &output_path)?;
        println!("✓ 已压缩: {} -> {}", path.display(), output_path.display());
    } else if path.is_dir() {
        let output_dir = if let Some(ref output) = args.output {
            // 创建输出目录
            fs::create_dir_all(output)?;
            output.clone()
        } else {
            path.to_path_buf()
        };

        let mut count = 0;
        let walker = if args.recursive {
            WalkDir::new(path)
        } else {
            WalkDir::new(path).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let file_path = entry.path();
            if file_path.is_file() && is_template_file(file_path) {
                let output_path = if args.output.is_some() {
                    // 保持相对路径结构
                    let relative = file_path.strip_prefix(path).unwrap();
                    let target = output_dir.join(relative);

                    // 创建父目录
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // 如果指定了 output 但没有指定 suffix，则不添加后缀
                    if let Some(ref suffix) = args.suffix {
                        generate_output_path(&target, suffix)
                    } else {
                        target
                    }
                } else {
                    // 没有指定 output 时，使用 suffix（默认为 "min"）
                    let suffix = args.suffix.as_deref().unwrap_or("min");
                    generate_output_path(file_path, suffix)
                };

                match minify_file(file_path, &output_path) {
                    Ok(_) => {
                        println!(
                            "✓ 已压缩: {} -> {}",
                            file_path.display(),
                            output_path.display()
                        );
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!("✗ 压缩失败 {}: {}", file_path.display(), e);
                    }
                }
            }
        }
        println!("\n总共压缩了 {} 个文件", count);
    }

    Ok(())
}

fn is_template_file(path: &Path) -> bool {
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

fn generate_output_path(path: &Path, suffix: &str) -> PathBuf {
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.{}.{}", file_stem, suffix, extension))
}

fn minify_html(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut in_pre = false;
    let mut in_textarea = false;
    let mut in_template_brace = false; // {{ }}
    let mut in_template_chevron = false; // {% %}
    let mut last_was_space = false;
    let mut tag_name = String::new();

    while let Some(ch) = chars.next() {
        // 检测模板语法开始
        if ch == '{' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    in_template_brace = true;
                    result.push(ch);
                    continue;
                } else if next_ch == '%' {
                    in_template_chevron = true;
                    result.push(ch);
                    continue;
                }
            }
        }

        // 在模板语法内，保持原样
        if in_template_brace || in_template_chevron {
            result.push(ch);
            // 检测模板语法结束
            if in_template_brace && ch == '}' && result.ends_with("}}") {
                in_template_brace = false;
            } else if in_template_chevron && ch == '}' && result.ends_with("%}") {
                in_template_chevron = false;
            }
            last_was_space = false;
            continue;
        }

        // HTML 注释处理
        if ch == '<' && chars.peek() == Some(&'!') {
            let mut comment = String::from("<");
            comment.push(chars.next().unwrap()); // '!'

            if chars.peek() == Some(&'-') {
                comment.push(chars.next().unwrap()); // first '-'
                if chars.peek() == Some(&'-') {
                    comment.push(chars.next().unwrap()); // second '-'
                    // 这是一个注释，跳过直到 -->
                    while let Some(c) = chars.next() {
                        comment.push(c);
                        if comment.ends_with("-->") {
                            break;
                        }
                    }
                    last_was_space = false;
                    continue; // 跳过注释
                }
            }
            result.push_str(&comment);
            continue;
        }

        // 标签处理
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
            result.push(ch);
            last_was_space = false;

            // 读取标签名
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_whitespace() || next_ch == '>' || next_ch == '/' {
                    break;
                }
                tag_name.push(chars.next().unwrap().to_ascii_lowercase());
            }

            result.push_str(&tag_name);

            // 检查特殊标签
            if tag_name == "script" {
                in_script = true;
            } else if tag_name == "style" {
                in_style = true;
            } else if tag_name == "pre" {
                in_pre = true;
            } else if tag_name == "textarea" {
                in_textarea = true;
            } else if tag_name == "/script" {
                in_script = false;
            } else if tag_name == "/style" {
                in_style = false;
            } else if tag_name == "/pre" {
                in_pre = false;
            } else if tag_name == "/textarea" {
                in_textarea = false;
            }
            continue;
        }

        if ch == '>' {
            in_tag = false;
            result.push(ch);
            last_was_space = false;
            continue;
        }

        // 在标签内、script、style、pre、textarea 内保留空格
        if in_tag || in_script || in_style || in_pre || in_textarea {
            if ch.is_whitespace() {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
        } else {
            // 标签外的内容
            if ch.is_whitespace() {
                if !last_was_space && !result.is_empty() {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
        }
    }

    result
}

fn minify_file(input_path: &Path, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(input_path)?;
    let minified = minify_html(&content);
    fs::write(output_path, minified)?;
    Ok(())
}
