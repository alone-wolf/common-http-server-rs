# API 哲学声明

## 1. 主要风格

公共 API 采用 **Builder-first**，并以 trait 作为扩展点。

- 入口对象：`ServerBuilder`、`ClientBuilder`、`SessionBuilder`。
- 扩展注入：认证、存储、观测、codec 通过 trait 注册。

## 2. 取舍分析

### 编译期安全

- Builder 在 `build()` 前保证必要配置齐全。
- 关键非法组合在构建阶段直接拒绝。

### 工程易用性

- 相比全量 typestate，Builder API 更直观，学习成本更低。
- 默认参数与链式配置更适合框架用户快速落地。

### 演进成本

- 新能力可通过新增可选 builder 字段无破坏扩展。
- trait-first 全量暴露会使 API 表面过大，长期维护成本更高。

## 3. API 面约束

- 公共入口保持小而稳定。
- 公共枚举优先 `non_exhaustive`。
- 避免把内部泛型参数直接暴露到顶层运行时对象。
- 仅在高价值场景引入局部 typestate，不做全局 typestate 泛滥。
