# Publishing Guide

本文档定义 `common-http-server-rs` workspace 的发布方式与检查清单。

## 发布模式

### 1) 推荐：Git 发布（当前默认）

适用于你当前仓库结构与对外使用方式（`Cargo.toml` 通过 `git` + `package` 引入 `websocket` / `http-panel`）。

步骤：

1. 确认 `main` 分支 CI 通过（fmt/check/test/clippy）。
2. 更新 `README.md` / `doc/*` 中涉及版本和示例的内容。
3. 创建 tag（例如 `v0.1.0`）并 push：
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
4. 在 GitHub 创建 Release，附上变更说明。

### 2) 可选：crates.io 发布

如需发布到 crates.io，请按依赖顺序发布，避免解析失败：

1. `common-http-server-rs`
2. `websocket`
3. `http-panel`

> 说明：`websocket` 与 `http-panel` 依赖主 crate；发布顺序错误会导致 crates.io 依赖解析失败。

## 发布前检查清单

- 代码质量：
  - `cargo fmt --all -- --check`
  - `cargo check --workspace --all-features`
  - `cargo test --workspace`
  - `cargo test --workspace --all-features`
  - `cargo check --workspace --examples --all-features`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- 版本信息：
  - `Cargo.toml` 的 `version`、`README` 中示例版本策略一致
  - 变更日志（Release Notes）已整理
- 文档一致性：
  - `README.md`、`doc/README.md`、关键 guide 与最新 API/示例一致
- 工作区状态：
  - `git status` 干净
  - 已 push 到远端并由 CI 验证通过

## 备注

- `websocket` 中有一条需要本地 socket bind 权限的 integration 风格测试为 `ignored`；CI 通过替代单测覆盖关键协商分支。
- 如未来需要严格网络链路回归，可增加专用 job（允许 bind socket）定时执行。
