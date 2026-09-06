# ADR-016：Pipeline 方法与派生制品

状态：已接受（待主代理登记 ADR 索引）

## 背景

Pipeline 需要把用户明确保存的分词方法应用于某个既有 canonical Revision 的文本，同时保留可复现的输入与结果。它不能成为第二套正文、Segment 编辑路径或 canonical history。

## 决策

新增独立的 `jueming-pipeline` 应用服务。它管理以 UUIDv7 标识的 Method、MethodRevision 和 Node，并把定义和执行结果保存在项目的 `.jm/extensions/pipeline/` 中。方法修订是 append-only；编辑会创建新的 MethodRevision，不会覆盖已发布的修订。

初始封闭 operator 集为 `source`、`normalize`、`chinese_tokenize` 和 `artifact`。图的边带有静态 slot 类型；服务拒绝未知 operator、未知上游节点、错误 slot、缺失或多余输入，以及有向环。没有通用 shell operator、插件运行时、POS、NER、OCR 或远端模型路径。

执行请求必须显式给出 canonical `RevisionId`、稳定 `SegmentId` 与该 revision 所见的原始文本。调用方负责从 Kernel 读取该 snapshot，并在调用前/后以当前 revision 做乐观并发检查。Pipeline 不写 `project.json`、`revisions/`、Segment、Alignment 或 HumanAnnotation。结果 JSON 记录输入 revision、segment、内容摘要、method revision、图摘要与 tokenizer 版本；其 token 是可再生的 derived artifact，不是 canonical data。

`jieba-rs` 采用 crates.io / docs.rs 当前稳定版 `0.10.3`，仅使用默认字典分词能力。自定义字典作为方法修订内的参数，按确定顺序加入 tokenizer；其改动产生新的 MethodRevision。执行阶段先在内存中完成并检查取消标记，只有成功才用 sibling-temp 原子替换发布 artifact；取消或失败绝不留下半发布结果。

持久化根为 `.jm/extensions/pipeline/`：`manifest-v1.json` 是版本化索引，`methods/<method-id>/<method-revision-id>.json` 是不可变定义，`artifacts/<artifact-id>.json` 是已发布结果。写入均原子替换。canonical 历史继续只位于 `.jm/revisions/`。

## 后果

历史界面可通过 `input_revision_id` 显示 Pipeline 产物所属的 canonical snapshot，但不能把方法修订误显示为 canonical Revision。若未来需要将 derived artifact 作为批注锚点，必须新增独立的 sidecar anchor 合同；本决策不定义该写入。

旧项目没有 `extensions/pipeline` 时表现为空方法列表；首次显式创建方法或成功执行时才建立目录。manifest 的 `format_version` 不兼容时必须失败而不是猜测迁移。
