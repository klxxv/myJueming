# 多译本工程与自研列布局验证

日期：2026-09-13。范围：ADR-018、动态导入向导、分段解释、多译本同屏、展示层解耦。

## 自动化覆盖

- `crates/jueming-core/tests/import.rs`：物理非空行、句末与文本结束边界、原始片段、实际清洗步骤、旧标记格式兼容。
- `crates/jueming-kernel/tests/comparison.rs`：三份同语种译本创建／打开，独立资源配置，按文档对 Link／Unlink，原文 Split／Merge 继承所有关系，指定第三译本插空隔离，旧 Revision 恢复，全部译本 TXT／XML／JSON 导出，重复占用及跨译本混选拒绝，真实文件预览后变化拒绝，同语种文档级搜索／替换隔离。
- `tests/comparison/run-contracts.mjs`：两文档的 1:1、1:n、n:1、n:m、跨行与未匹配投影完全复用旧实现；多译本的重叠关系不丢失或复制 Segment，切换译本只按 DocumentId。
- `tests/comparison/browser-regression.mjs`：实际 App + 确定性 Host 桥，Chromium 与 WebKit 均通过。2400 Segment 可视区裁剪、共用对齐带、交叠 n:m 各自起点、单组 600:600 内部虚拟化、正文换行后最大高度、列拖拽、键盘调序／调宽、绑定高亮、Unlink、选择译本进入双列编辑、离开返回保留列布局、全译本查找跳转及护眼主题。
- `tests/import-wizard/browser-regression.mjs`：实际 Vue 向导 + 确定性文件选择与 Kernel 桥，Chromium 与 WebKit 均通过。逐份导入、增加／移除步骤、分段解释和前后对比、20 段分页、请求竞态、空内容、创建失败重试、会话失效、SHA 请求、创建期间锁定、键盘焦点、macOS／Windows 路径。

浏览器测试验证生产组件交互与命令参数；真实磁盘、编码、校验及历史行为由 Rust 测试覆盖。没有将桥接测试描述为原生系统文件对话框自动化。原有双列 App 回归另已通过：正文分批加载、远距离滚动、编辑失败／取消／重试、模式切换、视图查找和批注失败／重试。

## 验证结果

前端 typecheck／production build、production contract suite、Rust Clippy 与全 workspace tests 均已通过。全量 Rust 测试首次运行在已有 dataflow_v2 的缓存清理测试中出现缓存 pin 失败；单项及完整重跑均通过，未修改该数据流实现或放宽测试。Tauri debug build（`--no-bundle`）通过，产物为 `target/debug/jueming-desktop`。
