# 协商协议流程与状态约束

## 1. 协商阶段

1. 客户端发送 `ClientAdvertise`。
2. 服务端计算交集并应用策略。
3. 服务端返回 `ServerSelect`。
4. 双方确认 capability contract 并进入 `Active`。

## 2. 协商输入输出

### ClientAdvertise

必须包含：

- transport 列表（按客户端偏好排序）。
- 协议版本集合。
- 可选能力集合（auth、codec、压缩、resume 等）。

### ServerSelect

必须包含：

- 选定 transport。
- 选定协议版本。
- 启用能力子集（必须属于客户端声明集合）。

## 3. 冲突处理规则

- 版本无交集：协商失败，返回可恢复或不可恢复错误。
- transport 无交集：按策略拒绝或建议降级。
- 必需 capability 缺失：协商失败，禁止进入 Active。

## 4. 重连再协商

- 允许在重连期间再协商。
- 若新能力集合与旧 session 契约不兼容，拒绝 attach。
- 升级/降级必须记录事件并更新契约版本。
