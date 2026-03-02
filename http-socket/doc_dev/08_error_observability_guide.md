# 错误与可观测性实现规范

## 1. 错误分层

建议错误层：

- `TransportError`
- `NegotiationError`
- `ProtocolError`
- `AuthError`
- `StoreError`
- `FrameworkError`（外层统一）

## 2. 错误传播规则

- 子系统内部使用本层错误。
- 跨层边界转换时保留 source chain。
- 关键上下文必须附带：`session_id`、`phase`、`transport`。

## 3. 恢复语义

错误需标注恢复策略：

- `Retryable`
- `BackoffRequired`
- `Fatal`

恢复策略用于重连与告警分流。

## 4. 可观测性最小集

至少输出以下事件：

- 协商开始/成功/失败。
- session 状态迁移。
- transport attach/swap 成功与失败。
- 扩展执行耗时与失败分类。

## 5. 指标建议

- 协商成功率与失败原因分布。
- 平均重连耗时。
- 每状态会话数量。
- 按错误类别计数。
