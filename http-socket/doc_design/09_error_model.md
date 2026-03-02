# 错误模型与传播策略

## 1. 策略选择

采用 **分层错误枚举 + 顶层框架错误包装**。

- 子系统错误：`TransportError`、`NegotiationError`、`ProtocolError`、`AuthError`、`StoreError`。
- 顶层错误：`FrameworkError`（统一向上暴露）。

## 2. 传播规则

- 子系统内部保留精确错误语义。
- 跨边界时转换为上层可理解的错误类型。
- 保留 `source()` 错误链与上下文字段（session_id、transport、phase）。

## 3. 恢复分类

每个错误需可判定恢复策略：

- `Retryable`
- `BackoffRequired`
- `Fatal`

## 4. 禁止项

- 禁止用字符串拼接替代结构化错误。
- 禁止在核心链路吞掉根因。
- 禁止把所有错误粗暴折叠为单一未知错误。
