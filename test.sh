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
echo -e "${YELLOW}[1/8] 编译项目...${NC}"
cargo build --release
echo -e "${GREEN}✓ 编译完成${NC}"
echo ""

# 准备测试
BIN="./target/release/askama-minify"
TEST_DIR="test_temp"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

# 创建测试文件
cat > "$TEST_DIR/example.html" << 'EOF'
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>{{ title }}</title>
    <style>
        body {
            margin: 0;
            padding: 20px;
            background: #f0f0f0;
        }
        .container {
            max-width: 800px;
            margin: 0 auto;
        }
    </style>
</head>
<body>
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
        console.log("Hello World");
        function greet(name) {
            return "Hello, " + name;
        }
    </script>
</body>
</html>
EOF

mkdir -p "$TEST_DIR/templates/sub"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/example1.html"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/example2.html"
cp "$TEST_DIR/example.html" "$TEST_DIR/templates/sub/example3.html"

echo -e "${YELLOW}[2/8] 测试场景 1: 默认行为（生成 .min.html）${NC}"
$BIN "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/example.min.html" ]; then
    echo -e "${GREEN}✓ 生成了 example.min.html${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 example.min.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[3/8] 测试场景 2: 自定义后缀${NC}"
$BIN -s compressed "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/example.compressed.html" ]; then
    echo -e "${GREEN}✓ 生成了 example.compressed.html${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 example.compressed.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[4/8] 测试场景 3: 指定输出文件（不添加后缀）${NC}"
$BIN -d "$TEST_DIR/output.html" "$TEST_DIR/example.html"
if [ -f "$TEST_DIR/output.html" ]; then
    echo -e "${GREEN}✓ 生成了 output.html（无后缀）${NC}"
else
    echo -e "${RED}✗ 失败: 未生成 output.html${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}[5/8] 测试场景 4: 文件夹压缩（默认后缀 min）${NC}"
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

echo -e "${YELLOW}[6/8] 测试场景 5: 文件夹输出到指定目录（不添加后缀）${NC}"
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

echo -e "${YELLOW}[7/8] 测试场景 6: 文件夹输出到指定目录并添加后缀${NC}"
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

echo -e "${YELLOW}[8/8] 测试场景 7: 验证压缩效果${NC}"
ORIGINAL_SIZE=$(wc -c < "$TEST_DIR/example.html")
MINIFIED_SIZE=$(wc -c < "$TEST_DIR/example.min.html")
REDUCTION=$((100 - (MINIFIED_SIZE * 100 / ORIGINAL_SIZE)))

echo "  原始文件大小: $ORIGINAL_SIZE 字节"
echo "  压缩后大小: $MINIFIED_SIZE 字节"
echo "  压缩率: ${REDUCTION}%"

if [ $MINIFIED_SIZE -lt $ORIGINAL_SIZE ]; then
    echo -e "${GREEN}✓ 文件成功压缩${NC}"
else
    echo -e "${RED}✗ 失败: 压缩后文件更大${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}验证 Askama 模板语法保留:${NC}"
if grep -q "{{ title }}" "$TEST_DIR/example.min.html" && \
   grep -q "{% for item in items %}" "$TEST_DIR/example.min.html"; then
    echo -e "${GREEN}✓ 模板语法已保留${NC}"
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
