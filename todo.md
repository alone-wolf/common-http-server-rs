# TODO

1. 集成在 sdk 的分布式 http server 状态收集器 + 中心级监控服务
2. http-panel 是实例级别的监控面板
3. http-center-panel 是中心级别的监控面板
4. 收集器和中心服务之间默认通过 ws 传递信息，http 轮询是 plan B
5. server 和 panel 之间也是使用 ws + http fallback 的方案
6. http-socket 即前述 ws + http fallback 的方案，支持（待补充具体能力）
