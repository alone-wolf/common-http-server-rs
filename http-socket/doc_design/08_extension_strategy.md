# 扩展机制设计

## 1. 扩展目标

在不修改核心内核代码的前提下，支持业务定制与生态接入。

## 2. 扩展点定义

1. **Middleware 扩展**
   - 基于 tower layer 组织横切逻辑（限流、鉴权、重试、审计等）。

2. **Authentication 扩展**
   - 通过 `Authenticator` trait 在协商/恢复阶段注入认证策略。

3. **Codec 扩展**
   - 通过 codec trait + codec registry 支持文本/二进制协议扩展。

4. **Persistence 扩展**
   - 通过 `SessionStore` trait 对接内存、Redis、SQL 或自定义后端。

5. **Observability 扩展**
   - 通过 tracing/metrics 抽象接口对接不同可观测系统。

## 3. 插件接入规则

- 扩展只能依赖稳定公共 trait，不可依赖核心私有结构。
- 扩展失败必须返回可分类错误，不得 panic。
- 扩展默认可禁用，核心功能不依赖某个具体插件实现。
