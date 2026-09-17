# Design 008: 插件安全与生命周期加固

## 决策与实现

1. **Trusted WebView 保留**：同进程共享 DOM/JS/React；权限和 token 约束公开 API，不是恶意代码沙箱。Worker/WASM/独立进程另立 Spec。
2. **Core 持有上下文**：`PluginRuntime` 保存随机 token → id、完整 manifest、内容 SHA-256、registry revision。Tauri 只传输；Core 派生 `Actor::Plugin(id)`。普通 GUI command 只接受 user/缺省来源。
3. **逐次验证**：源读取及每次调用验证注册、启用、revision 和授权内容摘要；停用再启用不会复活旧 token。HTTP 返回前再次验证撤销状态。
4. **显式能力**：Core/UI/网络/事件/cron/KV 缺省拒绝，`context.read` 控制 today 及 UI props。UI 同时匹配 permissions 和 contributions；KV 按 UTF-8 键值总量事务性执行 quota。缺省或 Rust 输出的 null quota 都表示无存储权限。
5. **事件边界**：领域事件只订阅；自定义事件为 `plugin.<id>:<topic>`，冒号避免反向域名存在前缀关系时串用。结构化 permission error、plugin.denied 和 activity log 记录拒绝。
6. **网络**：标准 URL 解析、GET、白名单、DNS 全地址检查并固定连接地址、禁止自动重定向与环境代理；DNS/连接/总请求及解压后响应均有限额，成功失败均审计。
7. **安装恢复**：跨进程文件锁、staging、持久 journal 和 SQLite install_operation 配合；未提交文件替换恢复旧目录，已提交操作完成备份。目录/ZIP 统一验证普通文件、路径、数量/大小及真实 id。备份按精确 id 及 manifest 验证，回滚重新授权，KV 保留。
8. **生命周期**：共享 SDK Lifecycle 管理 disposer；8 秒异步看门狗、取消标记、迟到结果清理、串行合并 reload。GUI 2 秒轮询插件指纹，CLI 安装/启停/回滚自动联动，cron 同步重扫。加载失败自动停用并显示提示。
9. **设置与分发**：声明式设置由宿主表单渲染并写插件 KV。SDK/test 0.2.0 共用权限实现；JS/TS 脚手架含本地 vendor、README、MIT LICENSE、构建及离线测试。安装/回滚在 008 原交付时为 CLI-only，GUI 提供审阅、启停、重载、设置、状态和停用全部。2026-09-08 的[桌面统一导入 Plan](../../../docs/plans/2026-09-08-plugin-import-entry.md) 增补原生选择器、只读预检及带来源 / 目标状态校验的 Core 安装，回滚仍用 CLI。

## 兼容与迁移

- 插件契约升级为 `plugin.protocol/v2`，旧 v1/缺省版本仅用于诊断列表，必须迁移后启用。echo/pomodoro/network 为显式权限示例。
- AI CLI `meta.schema_version` 从 5 升至 6：列表新增授权/完整性/revision 信息，安装默认停用，权限拒绝使用 `permission_denied` / exit 4。
- 只增加 `SCHEMA_V8`：content_sha256、approved_sha256、revision、install_operation；迁移将旧注册项停用，要求重新确认。V1–V7 为历史记录，不作为本轮权限兼容方案。
- ZIP 摘要与实际内容摘要分开；摘要只证明字节一致，不能证明作者身份。本地开发目录显示 local-unverified。

## 残余风险与限制

- 同步死循环会阻塞 WebView，异步超时无法抢占它；须 CLI `plugin safe-mode` 后重启。
- 插件自行添加的全局监听、原生定时器、模块顶层副作用需自行清理；SDK 仅管理经其注册的资源。
- cron 不是持久任务队列：应用退出不补发，系统休眠影响时机。CLI 的领域写入不会自动转成 WebView 领域事件。
- 不引入公共市场、签名服务、账号或跨设备同步。详细限制与命令见 [PLUGIN_API](../../../docs/PLUGIN_API.md)。
