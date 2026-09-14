# 导入编码自动识别验证（2026-09-13）

合同见 [ADR-019](../adr/ADR-019-import-encoding-detection.md)。新建向导每份文本默认自动识别；选择具体编码重新预览可覆盖检测结果。创建提交已预览的具体编码和 SHA-256。

## 验证范围

- `jueming-core/tests/encoding_detection.rs` 使用真实编码字节覆盖 UTF-8/ASCII/UTF-8 BOM、UTF-16 LE/BE BOM、GB18030（含四字节字符）、Big5、Shift_JIS、EUC-JP、ISO-2022-JP、EUC-KR、Windows-1251/1252；另覆盖截断字节、错误代理项、冲突 BOM、UTF-32 拒绝和手动解码。
- `jueming-kernel/tests/auto_decoding.rs` 验证三份文件分别为 UTF-16 LE、Windows-1252 和 UTF-8 BOM 时，预览内容与创建、重新打开工程一致；记录每份具体编码、profile、原始 SHA-256 和字节数。输入修改、损坏 BOM 或解码失败不生成半工程；粘贴仍作为 Unicode 文本处理。
- `jueming-protocol/tests/contracts.rs` 验证旧预览请求缺省手动模式，新增自动请求可读取，`auto` 不能作为持久化编码，旧枚举值保持兼容。
- `tests/import-wizard/browser-regression.mjs` 在 Chromium、WebKit 中使用真实 Vue 向导和确定性 Kernel 桥验证独立自动识别、结果/依据显示、手动覆盖、过期自动结果丢弃、各译本已解析 profile 的提交，以及既有分页、主题、焦点、失败重试和关闭保护。浏览器桥不替代上述 Rust 真实字节验证。

## 结果

- 前端 typecheck / build：通过。
- Rust format / 全 workspace Clippy（warnings 为错误）：通过。
- Rust 全 workspace tests：125 项通过、0 失败；2 项已有测试保持 ignored。
- Chromium / WebKit 向导回归：通过。
- Tauri debug build（`--no-bundle`）：通过。

## 已知边界

统计识别存在歧义，尤其是短文本；界面展示统计推测并提示检查实际文字，支持手动覆盖。无 BOM 的 UTF-16 可手动指定 LE/BE；UTF-32 暂不支持。不会使用所选语言、文件名或系统 locale 推测编码，也不会以替换字符掩盖解码错误。
