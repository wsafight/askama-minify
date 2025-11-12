mod minifier;
mod file_processor;

use clap::Parser;
use std::path::PathBuf;

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
        file_processor::process_single_file(
            path,
            args.output.as_ref(),
            args.suffix.as_deref(),
        )?;
    } else if path.is_dir() {
        file_processor::process_directory(
            path,
            args.output.as_ref(),
            args.suffix.as_deref(),
            args.recursive,
        )?;
    }

    Ok(())
}
