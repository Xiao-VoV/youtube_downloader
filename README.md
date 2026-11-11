# 开发预览

Rust workspace：macOS 优先，GPUI Kit 固定为 0.9.1。使用本机已安装的 yt-dlp / FFmpeg / ffprobe / Deno，不自动安装工具或远程组件。

```sh
cd code
cargo check --workspace
cargo test -p youtube-engine
cargo run -p youtube-desktop
```

GUI：选择目录 → 粘贴 YouTube 单视频链接 → 解析 → 选择预设 → 下载。支持取消与完成后 Finder 定位。每次下载独立子目录，预设 MP4 限 H.264/AAC ≤1080p，没有匹配格式则失败，不隐式转码。MP3 是显式转换。

## 模块边界
- [desktop 入口](crates/desktop/src/main.rs)：初始化和关闭窗口保护。
- [窗口状态与动作](crates/desktop/src/app.rs)：输入、目录、解析与下载动作。
- [后台桥接](crates/desktop/src/bridge.rs)：有界消息流，进程收尾后才解除忙碌状态。
- [界面组件](crates/desktop/src/view.rs)：按标题、来源、预设、任务和产物拆分短函数。
- [消息与错误文案](crates/desktop/src/messages.rs)：UI 消息契约。
- [engine](crates/engine/src/lib.rs)：与 GPUI 无关的工具、协议、进程监督和产物验证。
- [离线测试](crates/engine/tests/offline.rs)：假工具覆盖解析、协议、取消、限额与文件安全，不访问站点。

## 维护约定
按职责拆模块，不按文件大小机械拆分。优先短函数和显式输入输出；跨线程消息不携带 UI 句柄；阻塞 IO 不进入 UI 回调。注释说明边界、资源所有权、安全约束及非显然的取舍，不重复代码。新增能力优先扩展独立模型和测试，再接 UI；未来队列调度、持久化应独立模块而非继续堆入窗口状态。

当前是单任务开发预览，不是完整产品：队列、重试、历史、持久化、格式选择器、设置页及跨平台发行尚未实现。编译成功不等于实际 YouTube 下载成功；尚未进行真实媒体下载或原生界面交互验收。运行数据必须位于用户明确选择的仓库外目录。
