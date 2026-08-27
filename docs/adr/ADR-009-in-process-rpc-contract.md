# ADR-009: 本地 in-process Operation/Data contract

_Status: Accepted · Date: 2026-08-27 · Scope: Kernel boundary_

---

## 📋 Context

桌面 MVP 不需要为本地功能引入网络服务，但总体架构要求 UI、插件和未来 Server 使用稳定的 Operation RPC/Data RPC 语义。若直接把 Rust 内部调用暴露给 UI，未来 transport 替换会重写前端。

## 🎯 Decision

MVP 在 Tauri 内用 in-process/local command 实现逻辑 Operation RPC 和 Data RPC：控制面传 ID、Handle、参数和状态；数据面传 Slice、Batch 和 Artifact 投影。TypeScript 只通过 `KernelClient` façade 访问，事件用于进度和变更通知。

## ⚡ Consequences

本地实现简单且保留 Web/Server 兼容语义，UI 不依赖 SQLite 或 Rust struct。代价是必须维护版本化 envelope、幂等 command、stale revision 错误和 typed adapter。

## 🔗 References

- [Phase 0 合同：KernelClient](../architecture/mvp-phase0-contracts-v0.1.md)
- [全局架构：Operation/Data RPC](../../jueming_global_architecture_handoff_v0.2.md)
