# Builder-first API 落地约束

## 1. 对外入口规范

统一通过 Builder 创建运行实体：

- `ServerBuilder`
- `ClientBuilder`
- `SessionBuilder`

要求：

- 入口对象职责单一。
- 默认配置可安全运行。
- 必填项在 `build()` 前必须显式校验。

## 2. Builder 字段分层

- 基础层：网络地址、超时、并发上限。
- 协商层：版本集合、transport 偏好、可选 capability。
- 扩展层：auth/store/middleware/observability 注入。

## 3. API 稳定性约束

- 新增可选字段优先，不改已有语义。
- 公共枚举默认 `non_exhaustive` 设计思维。
- 避免在顶层 public type 暴露深层泛型参数。

## 4. 错误返回约束

- `build()` 返回结构化错误，不返回字符串错误。
- 运行期错误统一收敛到 `FrameworkError` 外层。
- 对调用方暴露稳定错误类别，细节通过 source 追踪。
