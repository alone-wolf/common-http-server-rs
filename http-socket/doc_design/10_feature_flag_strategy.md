# Feature Flag 策略

## 1. 目标

- 让依赖按需引入。
- 避免为未使用能力支付编译与运行成本。
- 保持核心 crate 轻量。

## 2. 规划中的 feature

- `ws`: WebSocket transport 支持。
- `sse`: Server-Sent Events transport 支持。
- `poll`: 长轮询 transport 支持。
- `client`: 客户端侧能力。
- `server`: 服务端侧能力。
- `tower`: middleware/layer 生态集成。
- `tracing`: tracing 观测能力。
- `metrics`: 指标采集能力。

## 3. 依赖隔离规则

- 每个 feature 只引入自身必需依赖。
- 未启用 feature 的模块不得参与编译路径。
- 核心模块不得反向依赖可选 feature 模块。

## 4. 兼容策略

- 新增 feature 必须默认关闭。
- 删除或重命名 feature 视为潜在破坏性变更，需经过版本策略评审。
