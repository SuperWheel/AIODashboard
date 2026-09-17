# 网络示例

只经 api.fetch 发起 GET；manifest 白名单为 api.github.com。不在 onload 自动联网。
设置由宿主渲染、校验并保存在本插件 KV。测试使用注入的固定响应，不依赖外网。
停用后所有命令和设置自动注销。仅运行可信来源代码；Trusted WebView 不是沙箱。
