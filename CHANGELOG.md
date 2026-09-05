# Changelog

## 0.3.4

- Restore Rust 1.85 compatibility.
- Preserve included fragment boundaries and template dependency graphs with sensitive insertion contexts.
- Fall back to original JavaScript and advanced CSS when minification would introduce HTML end tags.
- Preserve CSS comment token boundaries, Askama strings and nested comments, raw blocks, HTML attribute values, and Unicode whitespace.
- Recognize unquoted script MIME types and raw-text closing tags with a slash.
- Scan template delimiters in one forward pass and cache file reads and minified output within each compiler process.
- Avoid writing unused root templates and rewriting identical generated dependency files.
- Add regression coverage for dependency contexts, cache invalidation, cycles, and missing generated files, plus a standalone scanning benchmark.
