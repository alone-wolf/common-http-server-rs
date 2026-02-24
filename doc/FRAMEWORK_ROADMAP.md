# Framework Roadmap and Capability Milestones

本文基于当前 `common-http-server-rs` 代码现状（core/auth/protection/monitoring + websocket/http-panel + orchestrator）与后续“分布式监控中心化”构想，给出一个分阶段发展路径。

## 1. 目标与原则

- 目标：从“单实例可复用 HTTP 脚手架”演进到“多实例可观测、可治理、可扩展的平台化框架”。
- 原则：
  - 先收敛基础能力（稳定性、组合能力、可测试性），再扩张分布式能力。
  - 关键能力优先组件化（orchestrator、auth rules、transport、collector）。
  - 默认简单可用，复杂能力按需开启（feature/config）。

## 2. 分阶段发展路径

## Phase 0（已具备，当前基线）

### 框架可提供能力

- 核心启动与路由编排：`Server/AppBuilder/AppConfig/ServerConfig`
- 基础鉴权：Basic/API Key/JWT
- 防护能力：DDoS、IP Filter、Rate Limit、Size Limit
- 监控能力：Prometheus、请求统计、健康检查
- 扩展能力：WebSocket（server/client）、实例面板 `http-panel`
- 全局编排：`MiddlewareOrchestrator`（runtime layers/monitoring/protection/auth）

### 主要价值

- 单实例开发效率高，具备生产可用的中间件基础能力。
- 通过 orchestrator 降低了中间件漏挂/重复挂风险。

## Phase 1（近期，建议优先）

### 目标

把“单实例工程可维护性”做扎实，形成稳定的组合与治理能力。

### 新增/强化能力

- 多规则鉴权体系完善：
  - 多 auth rule（同 mode 不同后端配置）
  - priority + fallback + realm 观测标签
- 策略配置标准化：
  - 统一 config schema（auth/protection/monitoring/orchestrator）
  - 启动校验 + 配置快照导出
- 可观测性增强：
  - 统一 request-id / trace-id 贯穿
  - 面板与指标字段口径文档化（避免统计语义漂移）
- 工程保障：
  - 组合测试矩阵（auth + protection + monitoring + ws）
  - 示例覆盖“多 auth 规则 + 多租户语义”

### 交付结果

- 框架具备可稳定复用的“应用内平台”能力。
- 业务方可以按路由、按租户、按域进行策略编排。

## Phase 2（中期，中心化可观测）

### 目标

建立“分布式状态采集 + 中心汇聚”的最小闭环。

### 新增能力

- collector（实例侧）：
  - 采集 HTTP/WS 状态、基础系统指标、策略命中事件
  - 默认 WS 上报，HTTP 轮询 fallback
- center service（中心侧）：
  - 节点注册、心跳、状态汇总、离线检测
  - 聚合查询 API（租户/分组/实例维度）
- `http-center-panel`（中心面板）：
  - 实例列表、拓扑视图、健康热力图、异常事件流

### 交付结果

- 从“单机观察”升级到“集群态势感知”。
- 面向运维/治理的第一版控制平面成型。

## Phase 3（中长期，治理与闭环）

### 目标

从“可观察”升级到“可治理、可闭环”。

### 新增能力

- 策略中心：
  - 动态下发（rate-limit/ip/auth/log-level）
  - 版本管理、灰度发布、回滚
- 告警系统：
  - 阈值规则 + 聚合抑制 + 通知渠道
- 事件与审计：
  - 安全事件/策略命中事件归档
  - 审计查询与追踪关联

### 交付结果

- 具备“发现问题 -> 定位问题 -> 下发策略 -> 验证效果”的完整运维闭环。

## Phase 4（平台化与生态）

### 目标

形成可扩展的“插件化平台框架”。

### 新增能力

- 插件机制（policy/auth/provider/storage/notifier）
- OpenTelemetry 原生对接（trace/metrics/log）
- 多环境模板（dev/staging/prod）与标准交付脚手架

### 交付结果

- 可在不同业务线、不同部署形态下复用同一框架底座。

## 3. 能力里程碑矩阵

| 里程碑 | 单实例能力 | 多实例能力 | 治理能力 | 工程成熟度 |
|---|---|---|---|---|
| Phase 0 | 完整 | 无 | 基础 | 中 |
| Phase 1 | 增强 | 无 | 初步（规则化） | 中高 |
| Phase 2 | 稳定 | 初步（中心汇聚） | 可视化治理 | 高 |
| Phase 3 | 稳定 | 完整 | 策略闭环 | 高 |
| Phase 4 | 平台化 | 完整 | 插件化治理 | 很高 |

## 4. 推荐执行顺序（务实版）

1. 先完成 Phase 1（多规则、配置标准化、测试矩阵）。
2. 再做 Phase 2 的最小闭环（collector + center + center panel）。
3. 最后推进 Phase 3 的策略中心与告警闭环。

