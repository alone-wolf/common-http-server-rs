# Architecture Redesign Proposal

本文针对当前架构进行“面向工程实现能力”的重构设计，重点回答：模块如何拆、如何组、如何演进。

## 1. 当前架构简评

## 优点

- 能力完整：core/auth/protection/monitoring 已具备可生产基础。
- 组合能力增强：`MiddlewareOrchestrator` 已形成统一编排入口。
- workspace 扩展方向明确：`websocket`、`http-panel` 已解耦。

## 主要瓶颈

- 运行时编排逻辑与策略对象仍偏“代码式”，配置驱动不足。
- 单实例能力强，但“中心化治理”与“跨实例协同”尚未成体系。
- transport（ws/http fallback）尚未抽象为通用组件，易散落到业务代码。

## 2. 重构目标

- 模块边界清晰：核心运行时、策略引擎、观测面、传输层、控制面解耦。
- 组合方式统一：声明式配置 + 编排器装配 + 插件扩展。
- 演进路径平滑：兼容旧 API，增量迁移到新组件。

## 3. 建议的模块拆分

## 3.1 内核层（Runtime Kernel）

- `app-kernel`（可在当前 crate 的 `core` 中逐步内聚）：
  - `AppBuilder/Server` 生命周期
  - 启动校验、graceful shutdown、基础 runtime layers
  - 统一错误模型与响应约定

## 3.2 策略层（Policy Plane）

- `auth-engine`：
  - AuthRule/Realm/Priority/Fallback
  - 支持同 mode 多后端、按路径/租户分流
- `protection-engine`：
  - DDoS/IP/Rate/Size 组合策略
  - 策略模板与动态更新接口
- `observability-engine`：
  - 监控口径、指标标签、统计排除规则
  - 指标/事件聚合协议

## 3.3 传输层（Transport Plane）

- `http-socket`（新抽象）：
  - 首选 WS（双向、低延迟）
  - fallback HTTP（轮询/上报）
  - 连接状态机（重连、心跳、背压、超时、降级）

## 3.4 控制面（Control Plane）

- `collector-sdk`（实例侧）
- `center-service`（中心侧）
- `http-center-panel`（中心视图）

## 3.5 展示层（Panel Layer）

- `http-panel`（实例级）
- `http-center-panel`（中心级）
- 保持“查询 API 与页面渲染”分层，便于前后端替换

## 4. 组合模型（推荐）

建议把“功能组合”抽象成三种 profile：

- `StandaloneProfile`：单实例（当前主路径）
- `ClusterNodeProfile`：实例 + collector
- `ControlPlaneProfile`：中心服务 + center panel

每个 profile 由同一 orchestrator 进行装配，但加载的模块集合不同，避免重复实现。

## 5. 新的编排接口草案

建议在 orchestrator 上引入“声明式配置入口”：

```rust
OrchestratorConfig {
  runtime: RuntimeConfig,
  observability: ObservabilityConfig,
  protection: ProtectionConfig,
  auth: AuthConfigSet,      // Vec<AuthRule>
  transport: TransportConfig, // http-socket
  control_plane: Option<ControlPlaneConfig>,
}
```

然后由：

- `MiddlewareOrchestrator::from_config(config)`
- `AppProfile::apply(config, app_builder)`

统一生成实际路由与中间件链路。

## 6. 工程实现收益

- **可维护性**：策略逻辑集中，减少分散配置导致的行为不一致。
- **可测试性**：按引擎与 profile 建立分层测试（单元/组合/端到端）。
- **可扩展性**：新增 auth provider 或 transport 只需扩展对应平面模块。
- **可治理性**：中心化后可统一观察与控制，不依赖每个业务自行拼装。

## 7. 渐进式迁移方案

## Step 1：兼容层

- 保留现有 API（`with_global_auth_config` 等），内部映射到 `Vec<AuthRule>`。
- 新增配置化入口但不强制。

## Step 2：配置驱动

- 为 orchestrator 增加 `from_config`。
- 文档和示例切换为声明式配置主路径。

## Step 3：控制面接入

- 引入 collector 与 center-service，先做只读汇聚。
- 再演进到策略下发与闭环治理。

## 8. 推荐的仓库结构演进（workspace）

- `common-http-server-rs`（runtime kernel + policy engines）
- `websocket`（实时通道能力）
- `http-panel`（实例面板）
- `http-socket`（新：WS + HTTP fallback 抽象）
- `collector-sdk`（新：节点采集）
- `center-service`（新：中心汇聚与治理）
- `http-center-panel`（新：中心面板）

## 9. 总结

当前架构已经具备“高质量单实例框架”基础。下一步最关键的不是堆功能，而是：

1. 把能力抽象为稳定模块边界（尤其 auth rule 与 transport）；
2. 让编排从“手工代码”升级到“配置驱动 + profile 装配”；
3. 在此基础上推进中心化控制面，形成真正的平台工程能力。

