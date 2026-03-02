# Changelog

## 0.1.0
- 建立 `http-socket` 核心模块骨架与 feature 开关。
- 引入握手协商模型（advertise/select/contract）与选择策略接口。
- 实现会话状态机、transport attach/swap 与重连回滚路径。
- 提供认证、存储、观测、中间件扩展 trait 边界。
- 新增 `axum` 插件化集成层（Layer + Router 扩展）。
- 补齐基础测试矩阵与示例：`axum_http_socket_server`、`http_socket_client`。
- 完善发布元数据与 README，更新发布检查清单和 axum 集成文档引用。
