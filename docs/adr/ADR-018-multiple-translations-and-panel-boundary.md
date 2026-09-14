# ADR-018：多译本工程与自研窗格边界

状态：Accepted，2026-09-13；依据用户明确要求升级工程格式、同时比较多份译本，并将自研 Panel 与内部列表及 Alignment 分离。

## 决策

工程 2.0 使用显式 comparison（一个 source_document_id 与有序 target_document_ids），共享原文但每份译本拥有独立 Document、SegmentOrder、导入资源及分段规则；同语种译本以 DocumentId 区分。1.0／1.1 双文档工程继续原样读取，不静默升级；旧程序拒绝 2.0。

Alignment 仍是两个 Document 之间的关系，引用稳定 SegmentId，支持 1:1、1:n、n:1、n:m。每个 Segment 在同一原文／译本对内最多属于一个 active Alignment；原文可以同时与不同译本绑定。Link、Group、Unlink 只影响明确文档对；原文 Split 继承所有译本关系，Merge 必须在每个文档对内满足原有同一关系或全部未绑定约束。Revision 记录整个多译本工程。

Panel 是可拆分、调整宽度、拖拽排序的自研展示容器，只保存设备上的布局偏好；列表负责正文与可视区，Alignment 投影负责稳定 ID 关系、共用对齐带与绑定图标。拖拽列不提交 canonical ChangeSet，不改变 Document 顺序、SegmentOrder 或 Alignment。多列按关系连通分组取各列最大高度撑齐；连通的不同关系仍分别显示真实绑定 ID 与基数，禁止生成虚构关系。双列继续现有 AlignedWorkspaceViewport + useAlignedBlockLayout；多列复用其按带布局、测量与裁剪原则，保留双列操作入口。

新建向导每增加一份译本新增独立导入／分段预览步骤。Kernel 在创建前校验所有已预览输入的 SHA-256，任何输入变化、空分段或解码失败都不能产生部分工程。预览展示真实物理行／句末分界与实际清洗阶段，不宣称自动语义对齐。
