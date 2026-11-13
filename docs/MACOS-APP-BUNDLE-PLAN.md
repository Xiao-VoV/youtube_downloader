# YTDer macOS `.app` 打包方案（规划，不在本轮实现）

## 目标

将当前可由 Cargo 运行的 YTDer 原生程序包装为用户双击即可启动的 macOS `.app`，避免用户直接启动终端进程，同时为后续签名、公证和 Release 分发保留清晰边界。

## 建议目录

```text
YTDer.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── YTDer
    ├── Resources/
    │   ├── Assets.car 或应用图标资源
    │   └── README/版本信息（可选）
    └── _CodeSignature/（签名后生成）
```

## 实施阶段

1. **Release 二进制**：使用固定 Rust toolchain 构建 `youtube-desktop` release，确认 Apple Silicon 架构；如需 Intel，再构建 x86_64 并通过 `lipo` 形成 Universal 2。
2. **应用元数据**：增加 `Info.plist`，设置 `CFBundleIdentifier`、`CFBundleName=YTDer`、版本号、最小 macOS 版本、主可执行文件和图标声明。
3. **资源与工具策略**：明确 yt-dlp、FFmpeg、ffprobe、Deno 是用户配置/托管工具，不将用户工具和 Cookie 写入 Bundle；仅将应用自身资源放入 `Contents/Resources`。
4. **本地运行验收**：Finder 双击打开、窗口关闭、重复启动行为、权限提示、用户数据目录和日志路径。
5. **签名**：使用 Developer ID Application 对 `.app` 签名；签名前固定 Bundle 内容并记录构建摘要。
6. **公证与 Staple**：提交 Apple notarization，验证 Gatekeeper、隔离属性和另一台干净 macOS 的首次启动。
7. **分发产物**：优先提供 `.dmg` 或压缩 `.app`，同时发布 SHA-256 和版本说明。

## 本轮明确不做

- 不创建 `.app`；
- 不新增打包脚本、签名配置或证书引用；
- 不下载工具链、安装器或真实媒体；
- 不声称当前 Release 已具备 Finder 双击启动体验。

## 需要用户确认的决策

- Bundle Identifier（例如 `com.example.ytder`，最终值不能假定）；
- 仅 Apple Silicon 还是 Universal 2；
- 最低 macOS 版本；
- 是否使用 Developer ID 签名和公证；
- FFmpeg 是否始终由用户提供，还是未来单独设计可信依赖安装流程。
