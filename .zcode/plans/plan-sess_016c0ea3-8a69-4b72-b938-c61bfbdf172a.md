# 任务模块改造：打卡式长期任务 + 日期主库 + 四种卡片（Spec 级，变更包 003）

## 需求复述
把 PlanningDays 的任务卡片体系完整移植进 AIODashboard，替换现有 todo 式任务模块：
- 任务变为长期打卡对象：每日目标次数 + 单位 + 图标 + 主题色，append-only 幂等打卡账本（+1/-1/撤销），历史目标区间，活动区间（归档/恢复）
- 同步移植日期主库：纪念日/倒计时日、任务归属按天生效并保留历史、主库综合热力图、归档三选一流程
- 前端四种卡片（日/周/月/年）+ 任务详情（周/月/年总览）；设计语言以圆角矩形为主（进度圆环保留，**+/- 按钮为圆角矩形且并排位于同一侧**，不与圆环分列两侧），贴合现有双主题 Bento
- due_at 废弃、project_id 保留；旧数据 todo/doing→启用、done→归档
- CLI 语义变更 → meta.schema_version 1→2，dev-log 记迁移说明

## 切片 0：Spec 建档
openspec/changes/003-task-checkin-cards/ 四件套（proposal/spec/design/tasks）；按 Spec 流程先产出测试用例清单给用户确认，再 TDD 实现。

## 切片 1：domain + storage（SCHEMA_V3）
- Task：status→active/archived；+icon/color_hex/unit/card_style；删 due_at/completed_at（表重建，同 V2 模式）；保留 project_id
- 新表：completion_records（operation_id UNIQUE 幂等）、task_target_periods、task_activity_periods、date_libraries（前缀 dlb_）、task_library_membership_periods；重叠区间写入报一致性错误
- 逻辑日=本地时区 YYYY-MM-DD，换算扩展 context_service::local_today_range，不另写

## 切片 2：core
- checkin_service：record/decrement/undo（补偿记录：一条正向只补偿一次、不低于 0、notApplicable 拒绝）
- 五态/六态判定纯函数；周期总览构建器（days+summary+buckets；完整完成率、当前/最长连续天数照搬 PlanningDays 公式）
- library_service：CRUD/归档恢复/移库（今日起生效，历史留原库）；主库逐日聚合热力图（min(count/target,1) 求均值）
- 全部 log_activity + snapshot::refresh

## 切片 3：protocol v2 + CLI
- SCHEMA_VERSION→2；task create/update 加 --target/--unit/--icon/--color/--card-style
- 新增 task checkin|decrement|undo|overview；新增 library 命令组
- complete/reopen 重映射为「补满今日/今日清零」并标 deprecated；迁移说明进 dev-log

## 切片 4：Tauri
打卡三操作、周期总览、主库全套 command，带 actor；插件桥同步更新

## 切片 5：前端卡片体系
- HeatmapCell 六态：color-mix 色阶（双主题自动成立）；虚线=不适用、粗框=今天、点/横杠/双点/✓ 符号标记；图例
- TaskCard 四模式共用 header；目标=1 圆环完成钮，目标>1 时圆角矩形 [-][+] 并排同侧 + 进度圆环；周 7 格/月阵列/年 GitHub 热力图（横滚定位最近周）
- card_style 每任务持久化 + 右键切换
- TasksView→卡片墙；新增任务编辑器（补上没有的编辑入口）；任务详情页（周/月/年 + ⌘1/2/3、⌥←/→）
- Today 页今日任务换打卡式（未完成/进行中/已完成分组）；Space/Enter/⌘Z
- Widget Snapshot 今日统计改打卡口径 Σmin(count,target)/Σtarget

## 切片 6：前端日期主库
侧边栏入口；主库卡片网格（三段式）+ 详情（综合热力图+直属任务+批量移库）；归档三选一；已归档页

## 切片 7：测试与文档
core 单测 + integration.rs 新链路（打卡/主库/协议 v2 信封）；check.sh 全绿；dev-log、README、architecture.md、PLUGIN_API.md 同步

## 明确不做
主库背景图、通知提醒、历史补打卡 UI、阶段/里程碑（ProgressNode）、Widget 本体

## 风险
工作量集中在切片 2 与 5/6；协议 v2 是有意 breaking change（due 字段消失）。