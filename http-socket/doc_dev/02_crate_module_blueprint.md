# crate 模块蓝图与依赖方向

## 1. 当前模块结构（与仓库对齐）

```text
src/
  lib.rs
  api/
    mod.rs
    builder/mod.rs
    client.rs
    server.rs
  core/
    mod.rs
    state.rs
    session/
      mod.rs
      core.rs
      attach.rs
      swap.rs
      resume.rs
      consistency.rs
      types.rs
  protocol/
    mod.rs
    capability.rs
    version.rs
    handshake/
      mod.rs
      message.rs
      contract.rs
      policy.rs
      select.rs
  transport/
    mod.rs
    types.rs
    handle.rs
    registry.rs
    ws.rs (feature = "ws")
    sse.rs (feature = "sse")
    poll.rs (feature = "poll")
  extension/
    mod.rs
    auth.rs
    middleware.rs
    store.rs
    observability.rs
  integration/
    mod.rs
    axum.rs (feature = "axum")
  error/
    mod.rs
```

## 2. 依赖方向（单向）

- `api -> core/protocol/transport/error`
- `core -> protocol/error/transport`
- `transport -> error`
- `extension -> core/error/protocol`
- `integration -> extension/error`
- `protocol -> transport types`

禁止：

- `core` 依赖 `api`。
- `core` 依赖具体第三方扩展实现。
- `error` 反向依赖业务模块。

## 3. 分层说明

- `core`: session 真相状态、状态迁移、生命周期协调。
- `protocol`: 协商模型与选择算法。
- `transport`: 抽象 handle 与 feature-gated 传输句柄。
- `extension`: 只定义扩展边界（trait/hook）。
- `integration`: 与外部框架的适配层（当前为 axum）。
- `api`: 对外 Builder 与入口对象。

## 4. 未来演进（非当前实现）

`frame/codec` 等协议细分模块属于后续演进，不应在当前蓝图中当作“已实现结构”引用。
