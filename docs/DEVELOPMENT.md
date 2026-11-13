# YTDer 开发指南

[English README](<../README.md>) · [中文 README](<../README.zh.md>)

本文保留面向维护者的配置与模块说明。项目处于开发预览阶段；功能清单应区分源码实现、离线测试、原生交互验收和真实站点验证，不能相互替代。

## 构建与本机工具

从独立代码仓库根目录运行：

```sh
cargo check --locked --workspace
cargo test --locked -p youtube-engine
cargo run --locked -p youtube-desktop
```

GPUI Kit 固定为 0.9.1。应用优先从启动进程的 `PATH` 中查找 yt-dlp、FFmpeg、ffprobe 和 Deno；macOS 额外搜索 `/opt/homebrew/bin`、`/usr/local/bin`、`~/.local/bin` 和 `~/.deno/bin`，以支持 Finder 启动的最小环境。探测与工具子进程使用相同搜索路径，不执行登录 shell、不修改全局环境；其他安装位置请在工具设置中指定。不自动安装工具。yt-dlp 是解析/下载必需工具，FFmpeg 用于需要合并或转换的场景；Deno 存在不代表所有 JavaScript 挑战可解。ffprobe 当前用于发现和健康检查，不应将文件校验宣传为独立媒体完整性验证。

### 安装版工具探测回归

```sh
cargo build --locked -p youtube-engine --example health
/usr/bin/env PATH=/usr/bin:/bin:/usr/sbin:/sbin target/debug/examples/health
```

这会模拟 Finder 的最小 PATH，执行本机离线版本检查，不访问媒体站点。还需重新打包后从 Finder 打开应用，在工具设置中检查自动探测和健康检查；不应使用旧安装包判断源码修复是否生效。

## 下载配置

- 工作流：任务列表 → 新建下载 → 选择已保存配置或编辑配置 → 解析链接 → 查看命令 → 确认下载。
- 分辨率可选“最佳可用”（不设高度上限）或指定上限。MP4 限 H.264/AAC，默认 ≤1080p；“最佳”仍遵循编码限制，不等于全站最高分辨率。无匹配时失败，不隐式视频转码。MKV 可选其他源编码；MP3 是显式转换。
- 原始音频走 yt-dlp 的音频选择/提取流程，不应承诺逐字节原始文件。
- 高级参数每行一个 argv token，例如 `--limit-rate` 与 `2M` 分占两行。当前明确支持字幕、字幕语言/格式、限速、重试次数、分片并发参数。
- 未知选项以及执行钩子、配置/插件加载、输出路径/进度协议覆盖被拒绝。yt-dlp 内部重试不是应用级自动任务重试。
- 确认页默认显示结构化摘要，完整命令与高级参数折叠展示；展开后每个经引用、脱敏的参数单独一行，支持复制。脱敏命令不能直接用于复现认证；执行使用真实结构化 argv，不调用 shell。

## 配置存储与认证

macOS 配置位于 `~/Library/Application Support/Shiguang/profiles.json`，仅保存浏览器选择而非 Cookie 内容；含认证信息的代理拒绝落盘。切换配置会清除解析结果。解析与下载共享代理及 Cookie 设置，浏览器读取可能触发系统授权。

选择浏览器后可输入具体 Profile 名或目录（如 `Default`、`Profile 1`、Firefox profile 目录），留空由 yt-dlp 默认选择。Chrome 可通过 `chrome://version` 查看 Profile Path。当前不扫描或枚举浏览器数据，输入框不接受 yt-dlp 的 `:`、`+`、`#` 复合语法；具体浏览器版本的读取能力需实机验证。

已有配置另存与切换；覆盖更新和删除管理尚需完善。不要将本地 JSON 存储表述为加密保险库。

## 模块导航

| 模块 | 职责 |
| --- | --- |
| [desktop 入口](<../crates/desktop/src/main.rs>) | 初始化与关闭窗口保护 |
| [窗口状态与动作](<../crates/desktop/src/app.rs>) | 输入、目录、解析与下载动作 |
| [后台桥接](<../crates/desktop/src/bridge.rs>) | 有界消息流及后台结果交接 |
| [界面框架](<../crates/desktop/src/view.rs>) | 侧栏与路由 |
| [工作流页面](<../crates/desktop/src/view/workspace.rs>) | 任务、新建和命令确认 |
| [配置页面](<../crates/desktop/src/view/profiles.rs>) | 常用配置控制 |
| [配置存储](<../crates/desktop/src/settings.rs>) | 配置模型、输入快照与本机保存边界 |
| [消息契约](<../crates/desktop/src/messages.rs>) | 后台/UI 消息类型 |
| [engine](<../crates/engine/src/lib.rs>) | 不依赖 GPUI 的工具、协议、进程监督和产物路径验证 |
| [离线测试](<../crates/engine/tests/offline.rs>) | 假工具覆盖协议、取消、限额与文件安全，不访问站点 |

并行任务与历史相关模块处于开发中，是否完成接线、通过测试及原生验收需以具体提交为准。

## 维护约定

按职责拆模块，不按文件大小机械拆分。优先短函数和显式输入输出；跨线程消息不携带 UI 句柄；阻塞 IO 不进入 UI 回调。注释说明边界、资源所有权、安全约束及非显然的取舍，不重复代码。新增能力优先扩展独立模型和测试，再接 UI；队列调度和任务持久化应保持独立职责。

## 验证与安全说明

- 假工具测试不访问真实站点，编译成功和工具健康检查都不能证明实际下载成功。
- 输出路径与普通文件检查不代表媒体可播放、内容正确或文件完整。
- 外部工具运行于当前用户权限，不是 OS 沙箱；结构化 argv 不等于隔离恶意工具。
- 取消/失败不承诺自动删除所有部分产物，不能称保证续传或无损暂停。
- 解析失败会在草稿页显示诊断预览，支持复制当前会话完整错误（包含有界 stderr、退出码或启动失败原因）。原始诊断只保留在内存，草稿落盘仍仅保存通用失败摘要，重启后无法恢复完整日志。
- 浏览器 Profile、代理、目录和日志可能包含个人信息，复制错误或提交 Issue 前需自行检查。
- 运行数据应位于用户明确选择的仓库外目录。

## 正式开源前检查

- [ ] 选择项目许可证，添加 LICENSE 及必要第三方声明；不要使用未获授权的品牌/素材。
- [ ] 确定公开仓库地址，再补充真实的 Issues、Releases 与 CI 链接；不要放虚构徽章。
- [ ] 等待开发中的源码稳定，重新运行构建与测试，并记录提交和环境。
- [ ] 使用有授权的视频完成真实下载与完整原生交互验收。
- [ ] 更新与当前界面一致的截图，移除个人目录、Cookie、代理凭据及私人视频信息。
- [ ] 明确系统支持范围、打包方式与签名/公证状态，再对外发布安装包。
