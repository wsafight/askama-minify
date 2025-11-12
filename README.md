# Askama Minify

一个用于压缩 Askama 模板文件的 CLI 工具。支持压缩单个文件或批量处理整个文件夹。

## 功能特点

- 压缩 HTML、HTM、XML、SVG 文件
- 保留 Askama 模板语法（`{{ }}` 和 `{% %}`）
- 压缩内联 CSS 和 JavaScript
- 移除不必要的空格和注释
- 支持单文件或文件夹批量处理
- 递归处理子文件夹
- 自定义输出路径和文件后缀名

## 安装

```bash
cargo build --release
```

编译后的二进制文件位于 `target/release/askama-minify`

## 测试

运行完整的测试套件：

```bash
./test.sh
```

测试脚本会验证：
- 默认压缩行为
- 自定义后缀
- 指定输出路径
- 文件夹批量处理
- 递归子目录处理
- 压缩效果（约 40% 压缩率）
- Askama 模板语法保留

## 使用方法

### 基本用法

压缩单个文件（默认生成 `.min.html` 文件）：
```bash
askama-minify template.html
```

压缩整个文件夹：
```bash
askama-minify templates/
```

### 选项

- `-d, --output <OUTPUT>`: 指定输出文件或文件夹路径（如果已存在则报错）
- `-s, --suffix <SUFFIX>`: 输出文件的后缀名（不指定时：如果设置了 `-d` 则不添加后缀，否则默认使用 `min`）
- `-r, --recursive`: 递归处理子文件夹（默认启用）
- `-h, --help`: 显示帮助信息
- `-V, --version`: 显示版本信息

### 后缀规则

- **未指定 `-d` 且未指定 `-s`**: 使用默认后缀 `min`，生成 `file.min.html`
- **未指定 `-d` 但指定 `-s`**: 使用指定后缀，生成 `file.<suffix>.html`
- **指定 `-d` 但未指定 `-s`**: 不添加后缀，直接使用指定的输出路径
- **同时指定 `-d` 和 `-s`**: 在输出路径基础上添加指定后缀

### 示例

**基础压缩**（生成 `template.min.html`）：
```bash
askama-minify template.html
```

**自定义后缀名**（生成 `template.compressed.html`）：
```bash
askama-minify -s compressed template.html
```

**指定输出文件**（不添加后缀）：
```bash
askama-minify -d output.html template.html
```

**指定输出文件夹**（不添加后缀，保持目录结构）：
```bash
askama-minify -d output_dir/ templates/
```

**指定输出文件夹并添加后缀**：
```bash
askama-minify -d output_dir/ -s prod templates/
```

**自定义后缀压缩文件夹**（在原目录生成）：
```bash
askama-minify -s prod templates/
```

**不递归处理子文件夹**：
```bash
askama-minify --recursive=false templates/
```

**查看帮助信息**：
```bash
askama-minify --help
```

## 支持的文件类型

- `.html`
- `.htm`
- `.xml`
- `.svg`

## 压缩配置

该工具使用以下压缩配置：

- 保留 DOCTYPE 声明（不压缩）
- 保留模板语法（`{{ }}` 和 `{% %}`）
- 移除 HTML 注释
- 压缩内联 CSS
- 压缩内联 JavaScript
- 移除属性间的多余空格
- 移除可选的闭合标签
- 移除可选的 HTML 和 HEAD 开始标签

## 示例

### 输入文件

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>{{ title }}</title>
    <style>
        body {
            margin: 0;
            padding: 20px;
        }
    </style>
</head>
<body>
    <h1>{{ heading }}</h1>
    {% for item in items %}
    <div class="item">
        <h2>{{ item.name }}</h2>
    </div>
    {% endfor %}
</body>
</html>
```

### 输出文件

```html
<!doctype html><html lang=zh-CN><meta charset=UTF-8><title>{{ title }}</title><style>body{margin:0;padding:20px}</style><body><h1>{{ heading }}</h1>{% for item in items %} <div class=item><h2>{{ item.name }}</h2></div>{% endfor %}
```

## 许可证

MIT
