# 长期演进路线

## 1. 协议版本演进

- 通过版本集合协商支持 `N` 与 `N-1` 并行。
- 新版本默认增量扩展，不立即移除旧版本。
- 弃用路径需提供明确迁移窗口与公告。

## 2. 传输扩展（QUIC/WebTransport）

- 新 transport 通过 `TransportFactory`/注册机制接入。
- 传输新增不影响 `SessionCore` 核心状态管理。
- 协商层识别新 transport 能力即可完成接入。

## 3. 编解码扩展（二进制 codec）

- 通过 codec id/version 注册新编解码器。
- 默认保留现有基础 codec 作为兼容回退。
- codec 升级通过能力协商生效，非隐式替换。

## 4. 分布式会话存储

- 通过 `SessionStore` 抽象实现内存版到分布式后端平滑迁移。
- 会话恢复逻辑不绑定具体存储产品。
- 核心保证 session 元数据 schema 的演进兼容。

## 5. 防止破坏性变更的机制

- 公共枚举 `non_exhaustive`。
- 新能力优先增量字段/新 trait 方法（提供默认实现）。
- 通过 feature gate 控制试验能力。
- 对外承诺“协商协议 + 扩展 trait”双稳定面。
