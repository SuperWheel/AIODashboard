# Tasks 008: 插件安全与生命周期加固

> 2026-09-08：实现与本机验收完成。历史 001/007 保留，本页记录 v2 当前交付；不代表受限运行时或发布签名已实现。

> 同日后续演进：[桌面统一导入 Plan](../../../docs/plans/2026-09-08-plugin-import-entry.md) 增加 ZIP / 文件夹导入入口。本页 T8 的 CLI-only 是 008 原交付范围，新增入口的实现与验收记录见该 Plan。

## P1 身份与权限

- [x] T1 Core 上下文 token、内容摘要、revision 绑定；插件写入由 Core 派生 actor；普通 GUI 拒绝冒用来源。
- [x] T2 显式 Core/UI/网络/cron 权限，UI 同时匹配 contributions；today 及 props 要求 context.read；领域事件只订阅，自定义事件限定本插件；permission error、plugin.denied 与审计覆盖调用/上下文/加载源。
- [x] T3 KV 独立命名空间、UTF-8 键值总量 quota 与并发事务；缺省/null quota 都为无权限。

## P2 网络、安装与生命周期

- [x] T4 标准 URL、DNS 全地址检查与连接固定；GET、禁重定向、超时/大小限制与成功失败审计。
- [x] T5 ZIP/目录普通文件校验、symlink/hardlink/路径/大小限制、内容摘要、journal 恢复、精确 id 回滚、默认停用和来源/版本元数据。
- [x] T6 onload/import 异步超时和取消、失败清理、disposer、串行合并 reload；CLI 指纹轮询及 cron 同步；失效代不能继续使用 API。

## P3 SDK、界面与验证

- [x] T7 SDK/test 0.2.0 共用权限实现，声明式设置持久化，JS/TS 脚手架含 vendor/README/LICENSE/测试配置，plugin dev --run 构建与测试。
- [x] T8 GUI 展示来源、摘要、完整性、权限、quota、legacy 与 Trusted 提示；审阅/启停/设置/重载/停用全部。安装/回滚采用计划允许的 CLI-only 方案并写入界面和手册。
- [x] T9 Rust/CLI/Tauri、Vitest、SDK/mock/示例、脚手架独立构建、CI 和真实桌面验收完成。
- [x] T10 同步 README、architecture、PLUGIN_API、dev-log、007 状态与协议迁移记录；插件 v2、CLI schema 6、数据库 V8。

## 验证证据（2026-09-07—08）

- `bash scripts/check.sh full` 已执行并通过一轮；最后补齐加载源审计后，full 的 Rust fmt/clippy/test/build 全通过（98 测试），新增 IPC 测试曾因测试订阅遗漏 owner 参数失败。修正测试后运行 `bash scripts/check.sh frontend` 全通过（Vitest 39、SDK/test 6、两包及前端生产构建）；按仓库规则复用未变更的 Rust 门禁结果。
- Rust 覆盖上下文失效/内容变化、actor 伪造、Core/UI/事件矩阵、UTF-8/并发 quota、URL/DNS/映射地址、回环测试服务器的重定向/大小/超时、ZIP 链接/路径、安装故障注入与恢复、精确 id 回滚、默认停用及错误摘要。
- Vitest 覆盖各类 disposer、异步失败/超时/取消、串行重载和迟到回调、事件拒绝、Rust null quota、open/source/call 拒绝事件。
- JS/TS CLI 脚手架在临时目录独立 `npm install --ignore-scripts`，随后 `plugin dev --run --json` 均返回 build_and_tests_run=true。
- macOS debug app bundle 构建成功；真实 Tauri 界面验证权限模态框、Echo/番茄钟加载、network 设置保存/重载/应用重启持久化、CLI 停用自动同步、安装/回滚版本/来源/verified 状态、重复重载没有重复命令、双主题、关窗后台及重启后的计时状态。
- 故障样例先注册命令再抛出 acceptance onload failure：界面显示具体错误并自动停用，命令消失；回滚后重新授权可运行。CLI safe-mode 后所有插件视图/命令/设置自动移除。
- 真实验收发现并修复：Rust Option quota 输出 null，原 TS 校验误判无存储插件；补充宿主与 mock 回归。修复后重新构建并复验成功。
- `git diff --check` 通过；保留工作区既有其他任务改动，未将混合工作区直接提交。

## 验收边界与事件记录

- Trusted 模式同步死循环不可由异步看门狗抢占；本次验证 safe-mode 清除启用状态与贡献，不用卡死正式应用的方式测试。系统休眠后的 cron 精准度及发行签名/公证不在本轮保证内。
- 前次通过 LaunchServices 启动验收包时未可靠保留临时环境，窗口曾读取正式库。已立即退出并告知用户；只读核对正式库为 V8、10 条任务，临时库 0 条任务。无法据此断言迁移发生时点；没有反向修改迁移或操作正式任务。
- 后续直接运行 bundle 内可执行文件，显式设定 DASHBOARD_DB_PATH、DASHBOARD_PLUGINS_DIR、DASHBOARD_WIDGET_SNAPSHOT_PATH，复验只使用 `/tmp/aiodashboard-plugin-acceptance`。验收应用现已退出，临时样例全部停用。
