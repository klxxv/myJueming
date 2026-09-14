# 本地数据流与研究插件验证 v0.1

日期：2026-09-13。对应[实施交接](../architecture/local-research-runtime-implementation-v0.1.md)与 ADR-017。

## 验证环境

macOS 15.5 x86_64；Intel Xeon E5-2680 v3 2.50 GHz，24 GiB 内存。Rust 1.98.1。应用算法使用独立可重定位 CPython 3.12.11，PyTorch 2.2.2、Transformers 4.44.2、NumPy 1.26.4；完整锁文件在 `plugins/requirements-lock.txt`。测试没有使用系统 Python 代替应用运行环境。

完整资源放在被 git 忽略的 `apps/desktop/src-tauri/resources/research`，约 1.93 GB；两个独立目录包生成于 `target/research-plugin-packages/`。生成物不进入源码提交。最终 debug 构建资源目录的清单与源一致，17,888 个文件、1,927,742,636 字节全部通过逐文件大小及 SHA-256 校验。

## 已通过

| 验证 | 范围与证据 |
| --- | --- |
| Rust workspace tests | 108 项通过；保留旧分词、canonical 编辑/对齐/顺序/历史/存储、Agent 与 MCP 回归；真实模型相关的 2 项需显式开启，单列验证 |
| Rust fmt / Clippy | workspace 格式、all-targets 且 warnings denied 通过 |
| 前端类型 / production build | vue-tsc 与 Vite 构建通过 |
| Tauri debug build | macOS x64 `tauri build --debug --no-bundle` 通过；含新的共享 DTO/IPC 与资源配置 |
| 研究交互 | `tests/research/browser-regression.mjs` 23 项：范围转换、人工确认、两算法选择、完整分母、归并、草稿与历史守卫、过期上下文待复核、词距 0–10、工程切换、模型覆盖状态 |
| v2 编辑器 | `tests/research/pipeline-v2-browser.mjs` 12 项：Registry 端口、配置、方法保存/运行、产物、草稿保护；view 只用于合法 Schema |
| 设置 | `tests/research/settings-browser.mjs` 15 项：入口、开关、进度、取消/重试、偏好回执、护眼、键盘、关闭保留资源；就绪使用真实 worker_state=ready |
| 真实独立 Worker | 实际 CPython、模型与离线标志启动；两个模糊 Provider + XLM-R → 候选 → canonical 确认、归并、对齐变化待复核、Undo、重开 |
| Worker 故障隔离 | 使用同一受管理 Python 的故障夹具：阻塞 stdin 仍可取消；取消后重新握手并执行；响应身份与帧上限 |
| 数据池 / DAG | 固定旧 Revision 读取、不兼容边拒绝、真实倒排/词项索引、原文 UTF-8 范围、计算复用仍保留 Run 来源、缓存回收保留 Run 产物 |
| MCP | 工具目录实际包含新入口；经真实 LocalAppHost 查询 Slot/Schema/Operator；拒绝 native_only 及缺 binding 的工程操作 |
| 人工事实 | 无效字节边界和过期摘要拒绝；保存/重开/归并/撤销/重做/恢复一致；1.1 格式不因 Undo 降级；重复侧车记录拒绝 |

浏览器回归使用真实 Vue 界面加可控 Host 桥接，证明界面协议与交互，不冒充真实算法推理。真实算法证明来自 Rust 集成测试与下述独立评估。旧 architecture 浏览器回归同样通过。

真实集成测试最终复测完整通过 5 项（196.60 秒），覆盖最终覆盖状态 DTO 的完整调用路径；多数启用时间用于 debug 模式逐文件 SHA 校验。不能把此时间当作 release 性能。最终复测日志由执行会话记录，发布 CI 会在构建资源后对宿主平台再次运行实际模型测试。

## XLM-R 效果结果：尚未通过质量发布门槛

固定权重 `FacebookAI/xlm-roberta-base@e73636d4f797dec63c3081bb6ed5c7b0bb3f2089`，第 8 层上下文表示，子词 cosine 双向最近邻，保留阈值 0.15。没有声称实现 SimAlign 原算法或使用经过对齐微调的模型。

原始记录：[xlmr-baseline-evaluation-v0.1.json](xlmr-baseline-evaluation-v0.1.json)。版本化夹具为 `tests/fixtures/research/xlmr-alignment-v1.json`，6 条人工标注工程样例；它不是代表性评测集。

| 场景 | 标注范围 | 实际投影 | 边界完全匹配 |
| --- | --- | --- | --- |
| heavy rain 直译 | 大雨 | 雨 | 否 |
| 一夜降雨 | 暴雨 | 暴雨 | 是 |
| forecast 边界 | 大雨 | 雨 | 否 |
| through the heavy rain | 冒雨 | 雨 | 否 |
| 省译 | 无 | 出门 | 否 |
| emoji + 一对多上下文 | 大雨 | 雨 | 否 |

完全匹配 1/6。直接词组常漏边界，省译存在错误对应；不应依据分数批量确认，也不能将“找不到”自动分类为省译。模型内部 `coverage=complete` 仅表示输入未截断，不代表译法定位正确。

候选投影及研究 DTO 保留覆盖状态；界面分别提示输入截断、没有对应连线、缺少对齐上下文及未启用自动定位。没有匹配边也不会丢失覆盖信息。

独立模型加载 9.797 秒；首条推理 4.374 秒，后续短样例约 0.074–0.116 秒；子进程峰值 RSS 约 860 MiB。数据受当前硬件、长度和缓存影响。评估报告按 UTF-8 字节集合计算范围 precision/recall，明确不是语言学词级整体准确率。人工修正耗时和代表性语料性能尚未测量。

复现：

```sh
JUEMING_RESEARCH_BUNDLE="$PWD/apps/desktop/src-tauri/resources/research" cargo test -p jueming-application --test research -- --include-ignored --nocapture
python3 tests/research/run-xlmr-eval.py --bundle "$PWD/apps/desktop/src-tauri/resources/research" --output /tmp/jueming-xlmr-evaluation.json
```

## 未通过或尚未执行的交付检查

- Windows x64 和 macOS arm64 干净环境真实运行、Universal 最终安装包烟测尚未执行；本机结果仅证明 macOS x64。
- CI 增加了完整资源准备、原生 runner 真实离线模型测试和 Windows 研究便携 ZIP；未创建发布 tag，也未发布安装包。Universal 资源共享一份模型，避免按架构重复权重。
- 下载路径实现了 HTTPS、大小/摘要校验和原子准备；没有已发布的官方薄包资源端点，也没有把 mock 网络响应当成实际官方下载安装验收。当前可交付验证路径是完整资源包。
- 需要扩充词对齐评测集，改进词边界匹配及意译/省译处理，再重新评估。架构允许替换相同 Slot Provider，不必修改下游研究 Schema。
- 当前没有闲置模型卸载、任意反向 Worker Data RPC、跨 Revision 细粒度缓存、自动 v1→v2 方法转换或完整插件市场。它们不以可用按钮呈现。

因此当前成果是可运行、可审核、可扩展的本地首版；不能宣称已经满足跨平台正式发布及高准确率自动定位的全部条件。
