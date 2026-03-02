# http-socket 开发实施文档索引

本目录面向实现阶段，基于 `doc_design` 的架构 DNA，提供可执行的开发规则、里程碑与验收标准。

## 文档列表

1. [01_dev_scope_and_principles.md](./01_dev_scope_and_principles.md): 开发范围与硬性原则
2. [02_crate_module_blueprint.md](./02_crate_module_blueprint.md): crate 模块蓝图与依赖方向
3. [03_milestone_and_acceptance.md](./03_milestone_and_acceptance.md): 分阶段里程碑与验收
4. [04_api_contract_dev.md](./04_api_contract_dev.md): Builder-first API 落地约束
5. [05_handshake_protocol_flow.md](./05_handshake_protocol_flow.md): 协商协议流程与状态约束
6. [06_session_runtime_flow.md](./06_session_runtime_flow.md): 会话运行时与重连交换实现指南
7. [07_extension_dev_guide.md](./07_extension_dev_guide.md): 扩展点接入规范
8. [08_error_observability_guide.md](./08_error_observability_guide.md): 错误与可观测性实现规范
9. [09_test_strategy_matrix.md](./09_test_strategy_matrix.md): 测试矩阵与质量门禁
10. [10_feature_release_workflow.md](./10_feature_release_workflow.md): feature、版本与发布流程
11. [11_milestone_task_breakdown.md](./11_milestone_task_breakdown.md): M0-M5 统一执行计划（主入口）
12. [12_pr_splitting_plan.md](./12_pr_splitting_plan.md): PR 拆分历史文档（已归档）
13. [13_pr_work_breakdown.md](./13_pr_work_breakdown.md): PR 工作包历史文档（已归档）
14. [14_release_checklist.md](./14_release_checklist.md): 发布前执行检查清单
15. [15_axum_integration.md](./15_axum_integration.md): axum 插件化接入方案

## 与设计文档关系

- `doc_design` 负责“为什么这样设计”。
- `doc_dev` 负责“如何按该设计正确实现”。
- 若实现与文档冲突，先更新设计评审，再进入代码修改。
