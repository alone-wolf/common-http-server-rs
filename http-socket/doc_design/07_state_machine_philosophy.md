# 状态机设计哲学

## 1. 选择

采用 **Hybrid 状态模型**：

- 构建期与握手前置阶段：局部 typestate。
- 长生命周期运行阶段：runtime enum state。

## 2. 为什么不是纯 typestate

- 纯 typestate 在长生命周期 async 场景中会显著抬高 API 复杂度。
- 对业务开发者不友好，且易产生泛型蔓延。

## 3. 为什么不是纯 runtime enum

- 构建期关键错误（缺失能力、非法组合）应尽早在编译/构建阶段暴露。
- 局部 typestate 可提前阻断高价值错误。

## 4. 运行时状态建议

建议状态集：

- `Connecting`
- `Negotiating`
- `Active`
- `Resuming`
- `Draining`
- `Closed`

## 5. 调试与安全收益

- runtime enum 易于日志与指标标注。
- typestate 保证关键入口不可误用。
- 状态迁移规则集中管理，避免隐式跳转。
