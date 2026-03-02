# 会话所有权模型与重连交换

## 1. 所有权总览

- `Runtime` 持有连接接入与调度能力。
- `SessionManager` 持有 `SessionId -> SessionCore` 的映射。
- `SessionCore` 持有协议状态、协商契约、逻辑发送队列。
- `TransportHandle` 持有具体 socket/stream 资源。

## 2. 所有权原则

- session 是主实体，transport 是可替换附属实体。
- 应用层拿到的是 `SessionRef`（轻量句柄），不是底层 transport 所有权。
- 协议状态与会话身份不随 transport 变更而重建。

## 3. 重连流程

1. 创建新 `TransportHandle` 并完成 resume/认证校验。
2. 校验协商结果与旧 session 契约兼容。
3. 原子替换 `SessionCore` 绑定的 transport 引用。
4. 清理旧 transport，保留 session 连续性。

## 4. 文本所有权图

`Runtime -> SessionManager -> Arc<SessionCore> -> TransportHandle`

- `SessionCore` 是会话真相来源（source of truth）。
- `TransportHandle` 可替换但不可越权修改 `SessionCore` 主状态。
