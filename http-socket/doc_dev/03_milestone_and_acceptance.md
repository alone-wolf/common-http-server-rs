# 里程碑与验收标准

## M0: 骨架与边界落地

目标：完成模块骨架、feature 开关、基础类型定义。

验收：

- [ ] 模块结构与依赖方向符合 `02_crate_module_blueprint.md`。
- [ ] `ws/sse/poll/client/server/tower/tracing/metrics` feature 已声明。
- [ ] 无业务逻辑耦合进入核心模块。

## M1: 协商与能力契约

目标：实现 client advertise / server select 协商流程。

验收：

- [ ] 能正确选择双方交集 transport 与协议版本。
- [ ] 协商结果形成 session capability contract。
- [ ] 支持“无交集”失败路径并返回结构化错误。

## M2: SessionCore 与状态机

目标：实现 session 生命周期推进与合法迁移检查。

验收：

- [ ] `Connecting -> Negotiating -> Active` 主流程可运行。
- [ ] `Resuming/Draining/Closed` 迁移受控。
- [ ] 非法迁移被拒绝并带上下文错误。

## M3: transport 绑定与重连交换

目标：完成 transport handle attach/swap 生命周期。

验收：

- [ ] 重连后 session_id 不变。
- [ ] 协议状态连续，旧 transport 被正确清理。
- [ ] 不出现“双写”或幽灵连接状态。

## M4: 扩展面接入

目标：接入 auth/store/observability/middleware trait 扩展点。

验收：

- [ ] 扩展实现不需修改 core 私有结构。
- [ ] 扩展失败可分类并向上报告。
- [ ] 未启用扩展时核心流程仍可运行。

## M5: 稳定性与发布准备

目标：补齐测试矩阵、兼容规则、文档与示例。

验收：

- [ ] 测试矩阵覆盖协商、状态迁移、重连与错误分层。
- [ ] feature 组合最小集可通过编译测试。
- [ ] 文档与 API 行为一致。
