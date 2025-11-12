#!/bin/bash

# 测试脚本 - askama-minify
# 测试各种使用场景

set -e  # 遇到错误立即退出

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "  Askama Minify 测试脚本"
echo "========================================"
echo ""

# 编译项目
echo -e "${YELLOW}[1/10] 编译项目...${NC}"
cargo build --release
echo -e "${GREEN}✓ 编译完成${NC}"
echo ""

# 准备测试
BIN="./target/release/askama-minify"
TEST_DIR="test_temp"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

# 创建测试文件（包含 CSS、JS、注释和边缘情况）
cat > "$TEST_DIR/example.html" << 'EOF'
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>{{ title }}</title>
    <!-- HTML comment should be removed -->
    <style>
        /* CSS comment should be removed */
        body {
            margin: 0;
            padding: 20px;
            background-color: #f0f0f0; /* inline comment */
            font-family: Arial, sans-serif;
        }

        /* Multi-line CSS comment
           should be removed */
        .container {
            max-width: 800px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
        }

        .item {
            margin-bottom: 20px;
            padding: 15px;
            border-left: 4px solid #007bff;
        }

        /* Comment with special chars: <>&"' */
    </style>
</head>
<body>
    <!-- Body comment with {{ template }} syntax -->
    <div class="container">
        <h1>{{ heading }}</h1>
        {% for item in items %}
        <div class="item">
            <h2>{{ item.name }}</h2>
            <p>{{ item.description }}</p>
        </div>
        {% endfor %}
    </div>

    <script>
        // Single line JS comment should be removed
        console.log("Hello World");

        /* Multi-line JS comment
           should be removed */
        function greet(name) {
            return "Hello, " + name; // inline comment
        }

        // Test edge cases
        const str1 = "<!-- not a comment -->";
        const str2 = '// also not a comment';
        const str3 = `/* template ${1+1} */`;

        // Operators that look like comments
        const division = 10 / 2;
        const compare = 5 > 3;
        const shift = 1 >> 2;

        /* Comment with operators: > >= >> << */
        const numbers = [1, 2, 3, 4, 5];
        const doubled = numbers.map(n => n * 2);
    </script>
</body>
</html>
EOF

mkdir -p "$TEST_DIR/templates/sub"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/example1.html"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/example2.html"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/sub/example3.html"

echo -e "${YELLOW}[2/10] 测试场景 1: 默认行为（生成 .min.html）${NC}"
$BIN "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/example.min.html" ]; then
    echo -e "${GREEN}✓ 生成了 example.min.html${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 example.min.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[3/10] 测试场景 2: 自定义后缀${NC}"
$BIN -s compressed "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/example.compressed.html" ]; then
    echo -e "${GREEN}✓ 生成了 example.compressed.html${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 example.compressed.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[4/10] 测试场景 3: 指定输出文件（不添加后缀）${NC}"
$BIN -d "$TEST_DIR/output.html" "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/output.html" ]; then
    echo -e "${GREEN}✓ 生成了 output.html（无后缀）${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 output.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[5/10] 测试场景 4: 文件夹压缩（默认后缀 min）${NC}"
$BIN "$TEST_DIR/templates/"
if [ -f "$TEST_DIR/templates/example1.min.html" ] && \
   [ -f "$TEST_DIR/templates/example2.min.html" ] && \
   [ -f "$TEST_DIR/templates/sub/example3.min.html" ]; then
    echo -e "${GREEN}✓ 生成了所有 .min.html 文件（包括子目录）${NC}"
else
    echo -e "${RED}✗ 失败: 未生成所有文件${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[6/10] 测试场景 5: 文件夹输出到指定目录（不添加后缀）${NC}"
$BIN -d "$TEST_DIR/output_dir" "$TEST_DIR/templates/"
if [ -f "$TEST_DIR/output_dir/example1.html" ] && \
   [ -f "$TEST_DIR/output_dir/example2.html" ] && \
   [ -f "$TEST_DIR/output_dir/sub/example3.html" ]; then
    echo -e "${GREEN}✓ 生成了所有文件到 output_dir（无后缀，保持目录结构）${NC}"
else
    echo -e "${RED}✗ 失败: 未正确生成文件${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[7/10] 测试场景 6: 文件夹输出到指定目录并添加后缀${NC}"
$BIN -d "$TEST_DIR/output_prod" -s prod "$TEST_DIR/templates/"
if [ -f "$TEST_DIR/output_prod/example1.prod.html" ] && \
   [ -f "$TEST_DIR/output_prod/example2.prod.html" ] && \
   [ -f "$TEST_DIR/output_prod/sub/example3.prod.html" ]; then
    echo -e "${GREEN}✓ 生成了所有 .prod.html 文件到 output_prod${NC}"
else
    echo -e "${RED}✗ 失败: 未正确生成文件${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[8/10] 测试场景 7: 验证压缩效果${NC}"
ORIGINAL_SIZE=$(wc -c < "$TEST_DIR/example.html")
MINIFIED_SIZE=$(wc -c < "$TEST_DIR/example.min.html")
REDUCTION=$((100 - (MINIFIED_SIZE * 100 / ORIGINAL_SIZE)))

echo "  原始文件大小: $ORIGINAL_SIZE 字节"
echo "  压缩后大小: $MINIFIED_SIZE 字节"
echo "  压缩率: ${REDUCTION}%"

if [ $MINIFIED_SIZE -lt $ORIGINAL_SIZE ]; then
    echo -e "${GREEN}✓ 文件成功压缩${NC}"

    # 验证压缩率在合理范围内（35-55%）
    if [ $REDUCTION -ge 35 ] && [ $REDUCTION -le 55 ]; then
        echo -e "${GREEN}✓ 压缩率符合预期（35-55%）${NC}"
    else
        echo -e "${YELLOW}⚠ 压缩率 ${REDUCTION}% 超出预期范围 35-55%${NC}"
    fi
else
    echo -e "${RED}✗ 失败: 压缩后文件更大${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[9/10] 验证功能完整性${NC}"

# 验证 Askama 模板语法保留
if grep -q "{{ title }}" "$TEST_DIR/example.min.html" && \
   grep -q "{{ heading }}" "$TEST_DIR/example.min.html" && \
   grep -q "{% for item in items %}" "$TEST_DIR/example.min.html" && \
   grep -q "{% endfor %}" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ 模板语法已完整保留${NC}"
else
    echo -e "${RED}✗ 失败: 模板语法被破坏${NC}"
    exit 1
fi

# 验证 CSS 已压缩（检查是否包含压缩的 CSS）
if grep -q '<style>body{' "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ CSS 已成功压缩${NC}"
else
    echo -e "${RED}✗ 失败: CSS 未正确压缩${NC}"
    exit 1
fi

# 验证 JavaScript 已压缩
if grep -q "<script>" "$TEST_DIR/example.min.html" && \
   grep -q "</script>" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ JavaScript 已包含在输出中${NC}"
else
    echo -e "${RED}✗ 失败: script 标签缺失${NC}"
    exit 1
fi

echo ""

echo -e "${YELLOW}[10/10] 测试场景 8: 注释移除和边缘情况${NC}"

# 验证 HTML 注释移除
if ! grep -q "<!-- HTML comment" "$TEST_DIR/example.min.html" && \
   ! grep -q "<!-- Body comment" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ HTML 注释已移除${NC}"
else
    echo -e "${RED}✗ 失败: HTML 注释未移除${NC}"
    exit 1
fi

# 验证 CSS 注释移除
if ! grep -q "/\* CSS comment" "$TEST_DIR/example.min.html" && \
   ! grep -q "inline comment" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ CSS 注释已移除${NC}"
else
    echo -e "${RED}✗ 失败: CSS 注释未移除${NC}"
    exit 1
fi

# 验证 JS 注释移除
if ! grep -q "// Single line" "$TEST_DIR/example.min.html" && \
   ! grep -q "/\* Multi-line JS" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ JavaScript 注释已移除${NC}"
else
    echo -e "${RED}✗ 失败: JavaScript 注释未移除${NC}"
    exit 1
fi

# 验证字符串中的注释语法被保留
if grep -q '"<!-- not a comment -->"' "$TEST_DIR/example.min.html" && \
   grep -q "'// also not a comment'" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ 字符串中的注释语法已保留${NC}"
else
    echo -e "${RED}✗ 失败: 字符串内容被破坏${NC}"
    exit 1
fi

# 验证运算符保留
if grep -q "10/2" "$TEST_DIR/example.min.html" && \
   grep -q "5>3" "$TEST_DIR/example.min.html" && \
   grep -q "1>>2" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ 运算符（/, >, >>）已正确保留${NC}"
else
    echo -e "${RED}✗ 失败: 运算符被破坏${NC}"
    exit 1
fi

# 验证模板语法在注释中也被正确处理
if grep -q "{{ heading }}" "$TEST_DIR/example.min.html" && \
   grep -q "{% for item in items %}" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ 模板语法完整保留（即使原文件有带模板的注释）${NC}"
else
    echo -e "${RED}✗ 失败: 模板语法被破坏${NC}"
    exit 1
fi

echo ""

# 清理
echo -e "${YELLOW}清理测试文件...${NC}"
rm -rf "$TEST_DIR"
echo -e "${GREEN}✓ 清理完成${NC}"
echo ""

echo "========================================"
echo -e "${GREEN}  所有测试通过！ ✓${NC}"
echo "========================================"
