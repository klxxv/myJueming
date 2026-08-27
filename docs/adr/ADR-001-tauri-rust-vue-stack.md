# ADR-001: Tauri 2、Rust Kernel 与 Vue 3 技术栈

_Status: Accepted · Date: 2026-08-27 · Scope: MVP desktop runtime_

---

## 📋 Context

MVP 需要 Windows 离线桌面应用，同时把领域规则、事务、搜索和持久化从 UI 中隔离。架构 handoff 已要求 Rust Core、Tauri UI 和可替换的 Kernel Host；实施计划进一步冻结 Vue 3 Composition API 与 TypeScript strict。

## 🎯 Decision

采用 Tauri 2 作为桌面容器，Rust stable 作为 Kernel，Vue 3 Composition API + `<script setup lang="ts">` 作为前端，Vite 构建，pnpm workspace 管理，所有 IPC 通过 typed `KernelClient` façade。

## ⚡ Consequences

领域规则拥有单一 Rust 实现，前端可专注平行交互；未来可替换为 Web/Server transport。代价是需要维护 Rust DTO adapter、Tauri IPC 和 TypeScript 类型同步，不能让组件直接 `invoke`。

## 🔗 References

- [Phase 0 合同](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划技术栈](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
