# Host integration handoff

Main should register `"crates/jueming-application"` in the root workspace and
add this desktop dependency:

```toml
jueming-application = { path = "../../../crates/jueming-application" }
```

`AppKernelState` exposes `pub(crate) host: Arc<LocalAppHost>`. Register the
following trusted native commands in `lib.rs`:

```rust
commands::agent::agent_call,
commands::agent::agent_projection,
```

`agent_call({ call })` invokes `host.dispatch_native(call)` because it runs
inside the local Tauri UI process. External MCP code must retain its supplied
`Arc<LocalAppHost>` and call `host.dispatch(call)`, which returns
`native_approval_required` for `proposal.approve`. Forward `host.subscribe()`
to Tauri event name `agent_subscribe`; clients subscribe before projection and
discard events whose decimal sequence is not newer than the snapshot.

`app.bind_session` produces the only valid binding ID. A native
`app.get_projection` echoes that ID; `agent_projection` has no caller binding
and returns `binding_id: null`. The projection contains one authoritative
native UI context and the latest executed search response. External dispatch
rejects `app.get_projection`, `ui.publish_context`, and `ui.ack` directly.

Before a project opens, trusted `ui.publish_context` accepts only a null
binding with null `project_id` and `revision_id`; external `ui.get_context`
and `ui.get_selection` may read that app-only context. `ui.navigate` permits
only `project` and `settings` in that state. Every accepted navigation or
reveal contains an injected `request_id` and `operation_id`; projections
include unacknowledged `pending_ui_actions`. Trusted `ui.ack` must provide both
IDs and one of `ui_applied`, `failed`, or `cancelled`.
The action's `binding_id` is audit metadata only: an external client may create
an action and a different current native binding may acknowledge it when the
action's explicit `project_id` and `revision_id` still match. Terminal actions
are removed from `pending_ui_actions` while their operation status remains
queryable.

For one-shot, race-free UI searches, call `search.execute` once with:

```typescript
{
  spec: { query, regex, case_sensitive, language_id },
  expected_revision_id: "42",
  page_size: 50,
}
```

It atomically adopts and publishes the supplied spec, then returns the first
page. Pass its `session_id` to `search.get_results` for later pages. The host
caps bindings, retry entries, preview claims, search sessions, and pending UI
actions; terminal action statuses remain queryable while their pending records
are released.

Replace proposals preallocate `canonical_operation_id`, then call:

```rust
KernelService::apply_replace_with_operation_id(
    project_path, snapshot, request, operation_id,
) -> Result<ProjectSnapshot, KernelError>
```

The operation ID is stamped into the canonical Revision before persistence.
An `applying` proposal recovers only by finding that exact ID in history; a
final journal failure returns `committed_recovery_needed` after publishing the
actual revision event.

The application crate also reserves `host/pipeline.rs` and routes every
`pipeline.*` method through `pipeline_call(method, params, binding_id,
trusted_native)`. The Pipeline worker owns that submodule and emits
`pipeline_changed` through the host's existing publisher.
