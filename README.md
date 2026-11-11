# 开发预览

Rust workspace：macOS 优先，GPUI Kit 固定为 0.9.1。使用本机已安装的 yt-dlp / FFmpeg / ffprobe / Deno，不自动安装工具或远程组件。

```sh
cd code
cargo check --workspace
cargo test -p youtube-engine
cargo run -p youtube-desktop
```

GUI：任务列表 → 新建下载 → 选择已保存配置或编辑配置 → 解析链接 → 查看命令 → 确认下载。支持取消与完成后 Finder 定位。配置页提供浏览器 Cookie（默认关闭）、代理、分辨率上限、MP4/MKV/原音频/MP3、目录及高级参数。分辨率可选“最佳可用”（不设高度上限），或指定上限。MP4 限 H.264/AAC，默认 ≤1080p；“最佳”仍遵循编码限制，不等于全站最高分辨率；追求不限源编码时可选 MKV。不匹配则失败，不隐式转码。MKV 可选其他源编码；MP3 是显式转换。

高级参数每行一个 argv token，例如 `--limit-rate` 与 `2M` 分占两行；当前明确支持字幕、字幕语言/格式、限速、重试次数、分片并发参数。未知选项以及执行钩子、配置/插件加载、输出路径/进度协议覆盖被拒绝。确认页默认显示结构化摘要，完整命令与高级参数折叠展示；展开后每个经引用、脱敏的参数单独一行，支持复制，不能将脱敏命令直接用于复现认证；执行始终使用真实结构化 argv，不调用 shell。

配置保存在 `~/Library/Application Support/Shiguang/profiles.json`，仅保存浏览器选择而非 Cookie 内容；含认证信息的代理拒绝落盘。切换配置会清除解析结果。解析与下载共享代理及 Cookie 设置，浏览器读取可能触发系统授权。选择浏览器后可输入具体 Profile 名或目录（如 `Default`、`Profile 1`、Firefox profile 目录），留空由 yt-dlp 默认选择；Chrome 可通过 `chrome://version` 查看 Profile Path。当前不扫描或枚举浏览器数据，输入框不接受 yt-dlp 的 `:`、`+`、`#` 复合语法；具体浏览器版本的读取能力需实机验证。当前仅支持另存新配置与切换，尚无覆盖更新/删除管理。

## 模块边界
- [desktop 入口](crates/desktop/src/main.rs)：初始化和关闭窗口保护。
- [窗口状态与动作](crates/desktop/src/app.rs)：输入、目录、解析与下载动作。
- [后台桥接](crates/desktop/src/bridge.rs)：有界消息流，进程收尾后才解除忙碌状态。
- [界面框架](crates/desktop/src/view.rs)：侧栏与路由；[工作流页面](crates/desktop/src/view/workspace.rs)负责任务、新建和命令确认；[配置页面](crates/desktop/src/view/profiles.rs)负责常用控制。
- [配置存储](crates/desktop/src/settings.rs)：配置模型、输入快照与本机保存边界。
- [消息与错误文案](crates/desktop/src/messages.rs)：UI 消息契约。
- [engine](crates/engine/src/lib.rs)：与 GPUI 无关的工具、协议、进程监督和产物验证。
- [离线测试](crates/engine/tests/offline.rs)：假工具覆盖解析、协议、取消、限额与文件安全，不访问站点。

## 维护约定
按职责拆模块，不按文件大小机械拆分。优先短函数和显式输入输出；跨线程消息不携带 UI 句柄；阻塞 IO 不进入 UI 回调。注释说明边界、资源所有权、安全约束及非显然的取舍，不重复代码。新增能力优先扩展独立模型和测试，再接 UI；未来队列调度、持久化应独立模块而非继续堆入窗口状态。

当前是单任务开发预览，不是完整产品：队列、自动任务重试、历史与任务持久化、完整格式选择器及跨平台发行尚未实现。已实现配置持久化和有限高级参数；不是任意 yt-dlp CLI 编辑器。workspace check/build 与 20 项引擎测试通过，原生任务页启动及截图成功；编译成功不等于实际 YouTube 下载成功。尚未进行真实媒体下载或完整原生交互验收。运行数据应位于用户明确选择的仓库外目录。
