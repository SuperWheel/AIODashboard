# 求职台界面试用插件

用户明确选择界面试用版。本目录只交付演示 UI，不实现真实求职领域。

- 所有记录和任务均为虚构内存示例，不申请 Core/KV/network 权限，不将试用数据写入宿主。
- `src/review-source.html` 是已审阅界面输入，`build.mjs` 生成可加载的 `main.js`。不要修改生成文件后跳过构建。
- 使用宿主 React；图标在构建时从项目已有 Lucide 库静态生成，运行时不加载外部资源或第二份 React。
- 导航/主题复用宿主；View 与 TodayCard 共用一次激活期间的 DOM/controller，切页不丢演示修改，重载明确恢复样本。
- DOM、覆盖层和 CSS 仅作用于本插件；监听器、定时器和节点在卸载时清理。禁止修改宿主侧栏、任务组件或真实数据。
- 检查：`npm run build && npm test`，项目 `bash scripts/check.sh frontend`，独立 DB 打包安装及真实窗口验收。
- 正式数据/CLI/任务关联功能需继续 Spec 009，不以试用 JS 业务替代 Rust Core。
