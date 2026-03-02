# 发布检查清单（执行版）

- [ ] `cargo fmt --check` 通过
- [ ] `cargo clippy --all-targets --all-features` 通过
- [ ] `cargo test` 通过
- [ ] `cargo test --no-default-features` 通过
- [ ] `cargo test --features \"server,ws,tracing\"` 通过
- [ ] `cargo test --features \"client,poll,metrics\"` 通过
- [ ] `cargo test --features \"axum\"` 通过
- [ ] 示例可运行：`cargo run --example axum_http_socket_server --features "axum"`、`cargo run --example http_socket_client`
- [ ] `doc_design` 与 `doc_dev` 内容和当前 API 一致
- [ ] `CHANGELOG.md` 已更新
