# 会话运行时与重连交换实现指南

## 1. 运行时角色

- `Runtime`: 接入与调度。
- `SessionManager`: session 索引与生命周期管理。
- `SessionCore`: 协议状态与能力契约主实体。
- `TransportHandle`: 底层连接资源。

## 2. 生命周期主路径

`Connecting -> Negotiating -> Active -> Draining -> Closed`

重连分支：`Active -> Resuming -> Active`

## 3. transport 交换步骤

1. 新 transport 建立成功。
2. 执行 resume token 与认证校验。
3. 比较 capability contract 是否兼容。
4. 原子替换 `SessionCore` 中 transport 绑定。
5. 回收旧 transport，继续处理队列。

## 4. 并发一致性约束

- 单 session 仅允许一次有效 attach/swap 提交。
- swap 期间要有短临界区，避免双活 transport。
- 队列与状态推进必须遵循同一时序源。

## 5. 失败回滚

- attach 失败不得污染旧 session 状态。
- 回滚后保留旧 transport（若仍可用）或进入可恢复失败态。
