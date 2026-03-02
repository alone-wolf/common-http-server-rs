# http-socket 架构设计文档索引

本目录用于在实现前固定 `http-socket` 的核心架构 DNA，避免后续 API 与内部实现漂移。

## 文档列表

1. [01_framework_dna.md](./01_framework_dna.md): 框架身份定义与边界
2. [02_architecture_philosophy.md](./02_architecture_philosophy.md): 架构哲学声明
3. [03_api_philosophy.md](./03_api_philosophy.md): 公共 API 哲学与约束
4. [04_transport_capability_model.md](./04_transport_capability_model.md): 传输能力模型与协商规则
5. [05_dispatch_strategy.md](./05_dispatch_strategy.md): 分发策略与多态边界
6. [06_session_ownership_model.md](./06_session_ownership_model.md): 会话所有权模型与重连交换
7. [07_state_machine_philosophy.md](./07_state_machine_philosophy.md): 状态机设计哲学
8. [08_extension_strategy.md](./08_extension_strategy.md): 扩展机制与插件边界
9. [09_error_model.md](./09_error_model.md): 错误模型与传播策略
10. [10_feature_flag_strategy.md](./10_feature_flag_strategy.md): Feature Flag 策略
11. [11_long_term_evolution.md](./11_long_term_evolution.md): 长期演进路线
12. [12_pre_implementation_guardrails.md](./12_pre_implementation_guardrails.md): 实施前约束清单

## 使用方式

- 本目录中的结论是实现阶段的约束，不是建议项。
- 若后续实现与文档冲突，优先更新设计评审，再修改实现。
- 新增能力必须遵守“向后兼容 + 协商优先 + 可选扩展”原则。
