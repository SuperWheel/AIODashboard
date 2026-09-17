# @aiodashboard/plugin-sdk

AIODashboard plugin.protocol/v2 的 TypeScript 公共类型与共享权限/生命周期实现（0.2.0）。插件运行时由宿主注入 `api`，不要把宿主 React 或 Tauri 依赖打包进插件。

```ts
import type { PluginApi } from "@aiodashboard/plugin-sdk";

export async function onload(api: PluginApi) {
  api.ui.registerCommand({
    id: "hello",
    title: "Hello",
    handler: () => api.log.info("hello"),
  });
}
```

能力必须在 `manifest.json` 的 `permissions` 中声明。类型包只描述公共契约，不绕过宿主权限。

UI 注册还须在 `contributions.commands` 声明对应 id；所有注册返回 disposer。设置通过 `registerSettings` 注册并使用 `api.settings.get/set`，计入显式 KV quota。未声明/null quota 都表示无存储权限。

仓库根目录 `npm ci && npm run build && npm test` 构建 SDK/test 并测试契约。CLI 脚手架内置 vendor，支持离线引用；当前未宣称公共 npm 发布。详见 [插件手册](../../docs/PLUGIN_API.md)。
