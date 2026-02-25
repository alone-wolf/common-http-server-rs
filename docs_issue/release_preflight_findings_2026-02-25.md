# 发布前代码/文档复查问题清单（2026-02-25）

## 1) [代码] `http-panel` 挂在根路径时会拼出 `//api/snapshot`

状态：**已修复**

- 位置：`http-panel/src/lib.rs:276`、`http-panel/src/lib.rs:366`
- 问题：
  - 当前前端脚本：`panelBasePath = "/"` 时，请求地址会变成 `//api/snapshot`。
  - 这在浏览器中会被当作协议相对 URL，存在请求目标异常风险。
- 影响：
  - 当 `panel_routes` 挂载到根路径（`/`）时，面板自动刷新可能失败或请求到错误主机。
- 修复结果：
  1. 已增加 `apiBasePath`，根路径下会正确回退为空前缀。
  2. `fetch` 已改为基于 `apiBasePath` 拼接。
  3. 已增加对应 HTML 生成断言测试覆盖该逻辑。

---

## 2) [代码设计] `GlobalAuthConfig.realm` 当前是“可配置但未生效”

状态：**已修复**

- 位置：
  - 定义：`src/core/middleware_orchestrator.rs:133`
  - 设置：`src/core/middleware_orchestrator.rs:120`
  - 生效注入：`src/core/middleware_orchestrator.rs:291`
- 问题：
  - `realm` 现在只存储，不参与匹配、日志、响应或指标标签。
- 影响：
  - 使用方会误以为 `realm` 会影响运行时行为（例如审计分域、日志区分）。
- 修复结果：
  1. 命中 `AuthRule` 时会把 realm 写入 `request.extensions` 与 `response.extensions`。
  2. 新增 `AuthRealm` 类型并公开导出，供 handler 读取。
  3. 已新增测试覆盖 handler 读取 realm 与 response extensions 中 realm 的场景。

---

## 3) [文档] README“仅引入 http-panel 子 crate”标题与示例不一致

状态：**已修复**

- 位置：`README.md:122`
- 问题：
  - 标题写“仅引入”，但示例同时声明了 `common-http-server-rs` 与 `websocket`。
- 影响：
  - 用户会疑惑“到底是 only `http-panel` 还是必须三者都声明”。
- 修复结果：
  1. 标题已改为：`引入 http-panel（并显式声明其上游类型依赖）`。
  2. 已补充依赖关系说明文字，解释为何示例同时声明主 crate 与 websocket。

---

## 4) [发布流程/文档] workspace 子 crate 的发布策略需要显式说明

状态：**本轮不处理（按当前要求）**

- 位置：`websocket/Cargo.toml:48`、`http-panel/Cargo.toml:16`
- 问题：
  - 两个子 crate 使用了指向 workspace 的 `path` 依赖；当前文档主要面向 Git 依赖使用，但未明确 crates.io 发布顺序/策略。
- 影响：
  - 若后续要发布到 crates.io，容易在发布顺序和依赖解析上踩坑。
- 建议修复方案：
  1. 在 `doc/README.md` 或新增 `doc/PUBLISHING.md` 明确：
     - 当前推荐发布方式是 Git 依赖；
     - 若发布 crates.io，需先发布 `common-http-server-rs`，再发布 `websocket` / `http-panel`。
  2. 增加一个发布检查清单（版本一致性、tag、changelog、`cargo package` 验证）。

---

## 附：本轮已通过的基础质量检查

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-features`
- `cargo check --workspace --examples --all-features`
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

说明：`websocket` 中 1 个网络绑定相关测试为 `ignored`，在当前受限环境下单独执行会因绑定权限失败，这是环境限制，不是功能回归。
