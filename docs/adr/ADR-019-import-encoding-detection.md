# ADR-019：导入编码识别与确定性解码

状态：Accepted（2026-09-13，用户要求导入自动识别编码）

## 决策

导入预览使用 Rust 开源库 chardetng 识别历史文本编码，并以 encoding_rs 在 Windows/macOS 上执行同一套严格解码。先识别 UTF-8/UTF-16 BOM，再验证 UTF-8（含 ASCII），其余字节交给 chardetng；ISO-2022-JP 的转义序列须参与检测。无 BOM 的 UTF-16 可手动指定字节序，UTF-32 暂不支持。检测不根据文件名、所选语言或系统 locale 猜测。

`PreviewImportRequest.auto_detect_encoding` 是可选布尔字段，缺省 false 以兼容已有调用方；新建向导默认 true。自动识别只属于预览请求，不能写入 canonical `Encoding` 或 `ImportProfile`。预览返回已解析的 profile 和识别依据（BOM、有效 UTF-8、统计推测、粘贴的 Unicode 文本或手动选择）。界面显示结果，统计推测明确提示检查文字并允许手动覆盖，不提供虚构的置信度。

扩展具体编码枚举，保留既有 `utf8`、`utf8-bom`、`gb18030` 序列化值，新增 encoding_rs 支持的 UTF-16 和历史文本编码（排除 replacement、x-user-defined）。GBK/GB2312 检测结果归一到 GB18030。预览与创建提交同一具体 profile 和原始字节 SHA-256；创建不再次猜测。解码错误不能通过替换字符掩盖，任何导入失败不得产生半工程。新增编码工程可在当前版本往返读取，旧版本不保证识别新增枚举；已有工程的编码字段不改写。

## 依据

- [chardetng 官方仓库](https://github.com/hsivonen/chardetng)：MIT/Apache-2.0 授权，Firefox 使用的 Rust 检测器；GB18030 检测为 GBK，UTF-16 需要 BOM 层处理，短文本存在歧义。
- [chardetng API](https://docs.rs/chardetng/latest/chardetng/struct.EncodingDetector.html)：支持全字节流检测，无需网络或外部运行时。
- [encoding_rs API](https://docs.rs/encoding_rs/latest/encoding_rs/struct.Encoding.html)：提供不替换错误字节的严格解码接口。替换原来的 Windows 专用 GB18030 调用，解决 macOS 不可用的问题。
