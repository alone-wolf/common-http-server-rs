# M0-M5 实施任务拆分（统一执行版）

本文件是当前唯一维护的实施主计划，用于替代历史上的多份里程碑/PR 重复拆分文档。

## 使用规则

- 里程碑目标与验收：以 `03_milestone_and_acceptance.md` 为准。
- 本文件只维护“可执行任务 + 关键产出 + 依赖关系”。
- PR 拆分建议放在本文件末尾，不再单独维护镜像文档。

## M0：工程骨架与边界固定

### 任务

1. 固定目录和模块边界（`core/protocol/transport/api/extension/error`）。
2. 在 `Cargo.toml` 定义 feature（`ws/sse/poll/client/server/tower/tracing/metrics`）。
3. 建立基础公共类型（`SessionId`、`ProtocolVersion`、`TransportKind`、`CapabilityKey`）。
4. 固定依赖方向（禁止 `core -> api` 反向依赖）。

### 产出

- 模块骨架与 `mod.rs`。
- feature 声明与默认组合。
- 基础类型及最小测试。

## M1：协商协议与能力契约

### 任务

1. 定义 `ClientAdvertise` / `ServerSelect`。
2. 实现版本/transport 交集选择与 capability 过滤。
3. 定义策略接口（偏好顺序、必需能力）。
4. 产出 `CapabilityContract`。

### 产出

- 协商核心函数。
- 协商策略接口。
- 协商失败路径测试。

## M2：SessionCore 与状态机

### 任务

1. 定义 runtime 状态（`Connecting/Negotiating/Active/Resuming/Draining/Closed`）。
2. 实现状态迁移守卫。
3. 建立 `SessionCore`（会话身份、契约、transport 引用、发送队列）。

### 产出

- 状态机实现。
- `SessionCore` 主体实现。
- 合法/非法迁移测试。

## M3：Transport 绑定与重连交换

### 任务

1. 定义 `TransportHandle`、`TransportRegistry`。
2. 实现 attach 主流程。
3. 实现 swap 原子替换与失败回滚。
4. 定义 resume token 校验接口。
5. 定义契约兼容检查。

### 产出

- attach/swap 生命周期代码。
- 重连成功/拒绝/回滚测试。

## M4：扩展面接入

### 任务

1. 定义 `Authenticator` trait。
2. 定义 `SessionStore` trait。
3. 定义 observability hook（event/metric）。
4. 定义 middleware 扩展边界（feature `tower`/`axum`）。

### 产出

- 扩展 trait 集与默认 noop/allow-all 实现。
- 扩展失败分类与上抛路径。
- 扩展集成测试。

## M5：稳定性与发布准备

### 任务

1. 补齐测试矩阵覆盖。
2. 完成示例与 API 文档。
3. 完成 feature 组合验证。
4. 更新变更记录和发布清单。

### 产出

- 测试与 feature 组合验证结果。
- 对齐实现的文档与示例。
- 发布前检查记录。

## 建议 PR 切分（保留摘要）

1. 骨架与 feature。
2. 基础类型与错误。
3. 协商数据结构。
4. 协商算法。
5. 状态机与 `SessionCore`。
6. transport 抽象与 attach。
7. swap 与回滚。
8. 扩展面。
9. feature 组合与集成测试。
10. 文档/示例/发布收尾。

## 归档说明

历史拆分文档 `12_pr_splitting_plan.md` 与 `13_pr_work_breakdown.md` 不再维护，改为指向本文件，避免重复更新。
