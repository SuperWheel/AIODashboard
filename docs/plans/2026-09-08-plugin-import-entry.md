# Plan：插件统一导入入口

> 级别：Plan（在现有插件安装能力上接入桌面入口，涉及 Core / Tauri / React）
> 状态：已完成（2026-09-08）
> 提出日期：2026-09-08 · 完成日期：2026-09-08
> 关联：[Spec 008](../../openspec/changes/008-plugin-security-hardening/proposal.md) · [开发日志](../dev-log.md)

## 需求

现状：Rust Core 与 CLI 已支持 ZIP / 本地插件目录安装，桌面插件页只有启停、权限审阅、设置和重载；用户导入插件仍需运行命令。预期：在插件页提供统一的「导入插件」入口，完成选择来源、检查信息、确认导入及后续启用。

### 当前代码核对

- `apps/desktop/src/components/PluginsView.tsx`：页头只有「重载」「停用全部」，正文明确提示安装与回滚使用 CLI。
- `apps/desktop/src/api.ts` 与 `apps/desktop/src-tauri/src/lib.rs`：尚无导入预检和安装命令；桌面依赖尚未接入原生文件选择器。
- `crates/dashboard-core/src/plugin_package.rs`：`install` 已支持 ZIP / 目录，复用清单校验、内容摘要、锁、安装事务与备份；安装后停用。内部 `replace` 的 activity actor 固定为 `Actor::Cli`，GUI 接入时必须正确区分来源。
- `apps/desktop/src/components/PluginApprovalModal.tsx`：已有按内容摘要审阅并启用的交互，可用于导入后的下一步。
- Spec 008 当前标记已完成（2026-09-08），安装 / 回滚采用当时允许的 CLI-only 方案；本计划增补 GUI 导入。

### 交互方案

1. 插件页右上角主按钮「导入插件」打开统一弹窗；空列表也提供同一入口。
2. 弹窗提供「选择 ZIP」和「选择文件夹」，通过原生选择器单选。ZIP 使用现有打包格式；文件夹为包含 `manifest.json` 与入口文件的插件根目录，目录名应与插件 ID 一致。
3. Core 预检后展示插件名称、ID、版本、描述、来源和请求权限。已存在相同 ID 时展示「当前版本 → 导入版本」并提示替换及重新启用；同版本重新导入、较低版本也明确展示，不能当作全新安装。
4. 用户点击「确认导入」或「确认替换」后安装。处理中禁用重复提交；失败保留来源和具体错误，允许重新选择或重试。
5. 成功后刷新插件列表，显示「已导入，尚未启用」，提供「完成」与「审阅并启用」。启用复用现有权限确认和内容摘要校验流程。

### 实施约束

- ZIP 与文件夹共用 Core 的校验与安装实现；前端只负责交互，Tauri 只负责系统对话框、参数转换与 Core 调用。
- 预检只读取来源并使用自动清理的临时目录，不注册、安装或启用插件。安装提交时在现有锁内复核来源内容摘要和目标插件状态；预检后来源或已安装版本发生变化时，要求重新预检，避免覆盖用户未看过的内容或版本。
- GUI 安装由 Tauri 固定使用 `Actor::User`，CLI 继续使用 `Actor::Cli`，调用方不能从前端选择 actor；安装仍写 activity log 并刷新 Widget Snapshot。
- 保持现有 CLI 参数、JSON 输出字段、exit code、插件协议及数据库版本。新增桌面命令属于客户端接入；若实际需要变更公共协议或数据库 schema，升级为 Spec。
- 导入成功后撤销对应插件旧会话、同步 cron 并触发宿主重载，清理旧版本命令与视图；复用默认停用、备份和失败恢复行为。
- 保留现有 Trusted 模式提示及权限审阅；只将准确完成的操作显示为成功。

## 分步计划

- [x] 1. Core：抽取共用来源读取与校验，补充导入预检和带内容 / 目标状态校验的安装用例；参数化内部安装 actor，兼容现有 CLI 调用。
- [x] 2. Tauri：接入原生文件 / 文件夹选择器，新增预检与导入包装命令；耗时文件处理放入阻塞任务，完成后撤销旧会话并同步 cron。
- [x] 3. React：新增导入弹窗与相关类型 / API；插件页主按钮和空状态共用入口，接通预检、版本替换、忙碌、取消、失败、成功与审阅启用。
- [x] 4. 补充 Core / IPC / 前端关键行为测试，验证 ZIP、目录和更新场景；保留现有安装恢复与 CLI 回归。
- [x] 5. 完成 full 门禁和隔离数据库的真实桌面走查；同步 README、PLUGIN_API、dev-log，并在关联 Spec 中注明 GUI 导入的后续演进。

## 验证

- [x] Core：ZIP 与目录预检返回相同语义的信息；预检与取消不修改已安装目录、注册表、授权状态或安装日志。
- [x] Core：新安装默认停用；更新保留前一版并使旧授权失效；GUI / CLI 的安装日志 actor 分别正确。
- [x] Core：来源在预检后改变、目标插件被并发替换或修改时拒绝陈旧提交；同版本 / 较低版本不会跳过替换确认。
- [x] 回归：损坏 ZIP、缺少 manifest / 入口、旧协议、超限和非法路径均返回具体错误；失败不损坏已有插件，沿用已有安装与恢复测试。
- [x] IPC / 前端：取消选择不提交、选择新来源使旧预检失效、较早的异步结果不能覆盖新选择、处理中不重复提交、失败可重试、成功刷新并进入权限审阅。
- [x] 最小相关门禁通过：`bash scripts/check.sh full`（跨 Core / Tauri / React）；同批次已通过且未再修改的范围复用结果。
- [x] 人工验证核心路径：在显式指定临时 DB / 插件目录 / Snapshot 的 Tauri 实例中，完成 ZIP 新安装、文件夹安装、同 ID 替换、取消与无效包、审阅启用及新版本显示。直接运行 bundle 内二进制，避免 LaunchServices 丢失临时环境。
- [x] `git diff --check` 通过，且此次改动不覆盖已有其他任务的工作区修改。

## 结果记录（完成后填写）

- 2026-09-08 调研与提案：已检查当前 Core / CLI / Tauri / React 代码及 Spec 008，当前 `cargo check --workspace` 通过。本轮仅新增计划、索引和调研日志，功能尚未实施；没有将历史门禁记录作为本轮功能验收。
- 2026-09-08 实施：用户确认后完成 `plugin_package` 共用来源读取、只读预检、来源及目标状态校验和 actor 参数化；新增 Tauri `plugin_import.rs` 与原生 Dialog 依赖；新增 React 导入弹窗、异步交互状态和共用权限展示。桌面页头及空列表共用入口，导入后继续现有审阅启用流程。
- 自动检查：`cargo check --workspace` 通过；`bash scripts/check.sh full` 全通过（104 Rust、45 Vitest、6 SDK/mock/示例测试及各构建）。GUI 发现关闭图标被通用按钮 padding 挤小后，只修改图标按钮布局并重新通过 `bash scripts/check.sh frontend`，复用未变更的 Rust 结果。
- 桌面证据：使用 `/tmp/aiodashboard-plugin-import-h2zgj5xq` 内 DB / plugins / widget.json 直接启动 bundle 二进制，原生 ZIP 与文件夹选择器、预检取消、无效 ZIP、三次安装、权限启用、v1 → v2 替换、旧命令清理全部通过；预检后 CLI 改变启停状态，GUI 正确拒绝旧确认并可重新检查后完成。`acceptance.json` 记录 3 条 actor=user 的安装活动和来源 / 版本 / 备份 / 完整性；CLI status 确认临时库路径，活跃任务和项目数均为 0。测试插件只注册空操作命令。
- 最终包：`target/debug/bundle/macos/AIODashboard.app` 已重新构建；同一临时环境重启后复核导入入口、关闭图标及关闭动作通过。验收后停用测试插件并退出实例。完整门禁日志 `/tmp/aiodashboard-plugin-import-full.log`，图标修正后前端门禁日志 `/tmp/aiodashboard-plugin-import-frontend-final.log`，最终打包日志 `/tmp/aiodashboard-plugin-import-bundle-final.log`。
- 文档与范围：同步 README / PLUGIN_API / architecture / dev-log，并在 Spec 008 保留原交付记录、追加后续演进链接。CLI JSON schema 6、插件协议 v2、数据库 V8 均未变更。工作区已有其他任务的大量未提交修改，本轮保留并未将混合工作区直接提交。
