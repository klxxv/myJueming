# CI 维护与排障

工作流：[Package desktop](../../.github/workflows/package-desktop.yml)。包管理器与 Node 版本取自根 `package.json`；依赖安装必须使用冻结锁文件。

## 查看失败

```sh
gh run list --workflow package-desktop.yml --limit 5
gh run view <run-id> --log-failed
```

先查看失败步骤：类型检查、Node 测试、Rust 编译和打包是不同门禁。复现失败的具体命令后，再运行完整的相关检查。

2026-09-16 排查运行 `35006065229`：

- macOS 在 `AgentModelSettings.vue` 报 TS2307：代码引用 `lucide-vue-next`，实际依赖为 `@lucide/vue`。统一导入路径，无需安装旧包或放宽 TypeScript 检查。
- Windows 的搜索组件合同测试报 `HTMLButtonElement is not defined`：Node 内存 renderer 没有浏览器按钮构造器。测试宿主补齐按钮身份、属性和尺寸读取，断言真实 TanStack 行高测量被执行，并在卸载组件后恢复全局环境。

## 本地验证

```sh
pnpm install --frozen-lockfile
pnpm typecheck
node tests/agent-context/search-production-component-contract.mjs
pnpm test:agent-ui
pnpm build
```

Rust 与原生打包检查见[根 README](../../README.md)。Windows / macOS 的完整运行结果以 GitHub Actions 为准，本机通过不等于另一平台通过。

## 手动验证与发布

工作流包含 `workflow_dispatch`；文件进入默认分支后，可在 Actions 页面手动选择分支，或执行：

```sh
gh workflow run package-desktop.yml --ref <branch>
```

手动分支运行执行质量检查与 macOS Universal 编译。发布仍由 `v*` 标签控制：标签必须与 Tauri 应用版本一致，两端验证通过后才进入打包和 Release 发布。以标签为 ref 的手动运行也会进入发布流程。只有 Release job 获得 `contents: write`。

已有失败运行重跑会使用原提交；要验证修复，应先将修复提交到远端，再查看新运行。

官方参数参考：[GitHub 手动运行工作流](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)、[Tauri CLI](https://v2.tauri.app/reference/cli/)。

## Rust 缓存回收测试的进程隔离

运行 `35007120708` 的前端检查与 macOS Universal 验证已通过，Windows 随后在 `dataflow_v2` 的缓存回收断言失败。`GC_GATE` 是进程级锁，同一测试二进制里的其他图执行可能仍持有读锁；独立临时工程目录不会隔离这把锁。

缓存复用测试保留在 `dataflow_v2.rs`；回收验证放到独立集成测试二进制 `data_pool_gc.rs`，按顺序验证执行 pin 拒绝回收、view pin 保留输出及依赖、释放后删除产物并将实际缓存命中转为 miss。保留生产 `try_write` 行为，不通过串行化整个测试套件或忽略错误掩盖争用。
