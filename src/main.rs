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
    #[arg(short = 's', long, default_value = "min")]
    suffix: String,
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
            generate_output_path(path, &args.suffix)
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

                    generate_output_path(&target, &args.suffix)
                } else {
                    generate_output_path(file_path, &args.suffix)
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

fn minify_file(input_path: &Path, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read(input_path)?;

    let cfg = minify_html::Cfg {
        minify_doctype: false,
        allow_noncompliant_unquoted_attribute_values: false,
        allow_optimal_entities: true,
        allow_removing_spaces_between_attributes: true,
        keep_closing_tags: false,
        keep_comments: false,
        keep_html_and_head_opening_tags: false,
        keep_input_type_text_attr: false,
        keep_ssi_comments: false,
        minify_css: true,
        minify_js: true,
        preserve_brace_template_syntax: true,
        preserve_chevron_percent_template_syntax: true,
        remove_bangs: false,
        remove_processing_instructions: false,
    };

    let minified = minify_html::minify(&content, &cfg);
    fs::write(output_path, minified)?;
    Ok(())
}
