# 冗余精简问题讨论清单（2026-02-28）

## 1. `first_request` 疑似死字段，可删除
- 位置：`src/protection/ddos_protection.rs:167, 180, 387, 512`
- 现状：`first_request` 仅被写入，未参与任何读取逻辑。
- 建议：移除字段及相关赋值，减少状态复杂度。
- 状态：已确认保留（不修改）
评论：这个字段属于数据记录的功能，不能删除

## 2. `DdosError` 中有未使用分支
- 位置：`src/protection/ddos_protection.rs:610, 623`
- 现状：`RateLimited`、`SuspiciousActivity` 未被构造使用，仅在响应映射里出现。
- 建议：删除未使用分支，降低 API 与维护噪音。
- 状态：已确认保留（不修改）
评论：不能删除，这都是异常状态分支

## 3. 全局指标结构存在过度设计
- 位置：`src/protection/ddos_protection.rs:30, 214, 224, 313, 567, 596`
- 现状：只使用固定键 `global`，但使用了 `DashMap<String, DdosMetrics>`。
- 建议：改成单一全局指标容器（例如 `RwLock<DdosMetrics>`），简化读写路径。
- 状态：已确认保留（不修改）
评论：这里可以在未来保留对多scope的计量，这属于保留可扩展性

## 4. cleanup 删除队列可能重复插入同一 IP
- 位置：`src/protection/ddos_protection.rs:548, 553`
- 现状：同一 IP 满足两个条件时会重复 `push`，造成重复 `remove`。
- 建议：改为 `HashSet<IpAddr>` 去重后再删除。
- 状态：已完成
评论：好的，按照你的建议修改

## 5. URL 长度计算重复
- 位置：`src/protection/size_limit.rs:96, 104, 112`
- 现状：`path_and_query().map(...).unwrap_or(0)` 在同一分支重复计算三次。
- 建议：提取为局部变量 `url_len`，减少重复表达式。
- 状态：已完成
评论：好的，按照你的建议修改

## 6. CORS 文档“测试/调试”段落可合并
- 位置：`doc/CORS_GUIDE.md:169, 178, 195, 200`
- 现状：测试命令与调试命令分散在两个相邻章节，信息重复且跳转成本高。
- 建议：合并为“测试与调试”章节，统一示例与日志说明。
- 状态：已完成
评论：好的，按照你的建议修改
