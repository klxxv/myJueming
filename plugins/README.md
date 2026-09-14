# 决明本地算法插件

这两个包是决明应用的算法扩展，不是 Codex 插件。

- `fuzzy-matching`：`text.similarity` Slot 的两个 Provider：`fuzzy.edit_distance`、`fuzzy.char_ngram`。
- `xlmr-word-alignment`：`relation.word_alignment` Slot 的 `xlmr.contextual_alignment`，使用固定的本地 XLM-R 权重。

## 增加或替换 Provider

1. 从 `schemas/pipeline/catalog-v2.json` 查看目标 Slot 的具名端口与版本化 Schema。
2. 新建包目录；`plugin.json` 指定独立 package_id/release、protocol_version=1、runtime=python-3.12、entrypoint、operators、slots 映射。可选 config_schemas 为 Operator 提供 JSON Schema 配置校验。
3. Worker 是独立进程，使用下面的 NDJSON 协议。返回值必须满足每个输出端口的 Schema；原文范围使用原始 UTF-8 字节，不使用 DOM、Python字符或模型 token 索引。
4. 使用 `python3 scripts/package-research-plugin.py <包源码目录> <输出目录>` 生成带文件摘要的目录包。开发者使用原生 `plugins.install_local {package_path}` 安装。该动作不向 Agent/MCP 开放；普通用户仍只使用功能开关。
5. 功能准备时校验与握手后注册；用 `operators.list` 查实际可用实现，v2 Plan 的 operator_id 选择新实现。新包不需要修改通用执行器、下游 Schema 或 MCP 工具清单。

独立包目前复用受管理的 Python 3.12 与完整研究资源集；不能要求用户运行 pip。增加新的二进制依赖时必须更新平台资源构建与锁文件。包为可信本地代码；进程隔离和能力约定不构成操作系统沙箱。

## Worker 协议 1

stdout 只发送 JSON 行；一帧最多 8 MiB（包括换行），每次调用有独立 UUID request_id。正常完成后必须 echo 身份。算法日志不得写 stdout。宿主对 stdin/stdout 均设置有界队列；模型执行超时 300 秒。取消会终止独立进程，下一次调用重新启动并握手。

握手请求：

```json
{"request_id":"<uuid>","method":"hello"}
```

握手响应必须列出 manifest 中相同顺序的 Operator。XLM-R 包在回应前真正加载模型，因此 ready 表示资源和运行组件均可用。

```json
{"request_id":"<uuid>","data":{"protocol_version":1,"operators":["example.similarity"]}}
```

执行请求中的 inputs 是“端口名 → 按基数排列的值数组”；多个命名端口互不混淆：

```json
{"request_id":"<uuid>","method":"execute","operator_id":"example.similarity","inputs":{"pairs":[{"pairs":[{"pair_id":"p1","left":"heavy","right":"hevy"}]}]},"config":{}}
```

```json
{"request_id":"<uuid>","data":{"scores":{"items":[{"pair_id":"p1","score":0.8,"score_kind":"normalized_edit_similarity","provider_id":"example.similarity"}]}}}
```

失败返回 `{"request_id":"<uuid>","error":"说明"}`。Host 校验后自行发布 Manifest；Worker 不返回任意 `.jm` 文件路径，也不直接提交人工事实。可运行模板见两个 `worker.py`。

## 打包受管理资源

构建机需要 uv 0.8.22；运行应用的用户不需要 Python/uv。`plugins/requirements-lock.txt` 固定运行依赖。

```sh
python3 scripts/prepare-research-release.py --uv uv
# macOS Universal：两个运行环境共享一份模型和插件源码
python3 scripts/prepare-research-release.py --uv uv --universal
```

有已准备的可重定位运行环境、依赖和模型时：

```sh
python3 scripts/build-research-bundle.py --runtime <CPython目录> --packages <依赖目录> --model <固定模型目录> --output apps/desktop/src-tauri/resources/research
```

完整包直接校验和加载。生成薄包资源清单时，构建者可传 `--download-base https://<受控的不可变发布路径>`；必须先实际发布清单对应的所有文件。不要把示例地址当作已存在的下载服务。

Universal 的 `bundle.<arch>.json` 指向 `<arch>/runtime`、`<arch>/packages` 与共享 `model`、`plugins`。单架构使用 `bundle.json`。生成物均忽略，不提交运行环境、模型或真实 `.jm`。

版本和来源见 [THIRD_PARTY.md](THIRD_PARTY.md)。测试与已知算法限制见 [验证记录](../docs/testing/local-research-runtime-validation-v0.1.md)。
