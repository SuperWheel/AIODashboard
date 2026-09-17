# @aiodashboard/plugin-test

plugin.protocol/v2 离线测试桩（0.2.0），与宿主共用 SDK 权限与 Lifecycle。使用 `createPluginTestContext(manifest, options?)` 创建上下文，再调用插件 `onload(ctx.api)`；`ctx.dispose()` 清理注册资源。

- 检查默认拒绝、Core/UI、KV UTF-8 quota、事件命名空间与 disposer。
- 网络必须提供 fixture；未实现的 Core 用例显式抛错，不伪装真实宿主。
- 可将 `ctx.kv` 传给下一上下文检查设置/状态持久化。
- 根目录 `npm run build && npm test` 覆盖契约与 echo/pomodoro/network 示例。

测试桩不替代真实 Tauri/CSP/托盘验收；完整 API 见 [开发手册](../../docs/PLUGIN_API.md)。
