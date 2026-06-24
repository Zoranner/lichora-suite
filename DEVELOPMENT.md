# 开发指南

## 环境要求

Windows：

- Rust toolchain
- Visual Studio Build Tools
- CEF 145.0.27 runtime，默认由 `setup-windows.ps1` 准备并通过 `CEF_PATH` 指向

Linux：

- Rust toolchain
- CEF 145.0.27 runtime
- GCC/Clang
- pkg-config

## 构建与发布

Windows 发布构建：

```powershell
.\setup-windows.ps1
.\build.ps1 -Release
```

`build.ps1` 成功后把 Rust 可执行文件和 CEF runtime 复制到：

```text
dist/win-x64/
```

该目录是 Unity 侧应引用的 Windows runtime 目录。`debug.log` 是 CEF 运行期日志，已通过 `.gitignore` 排除。

## Unity handler 运行模型

Unity 集成不再按每个浏览器启动一个 `--guid --url` 进程。新主线是 IPC v2 session 模型：

- Unity 启动一个 handler 进程，首个非选项参数为 session 或 handler GUID。
- Browser 进程创建项目自有 IPC session。
- 管理命令走 `control` queue，不再使用单槽 flag。
- 鼠标移动走 latest-only，点击、滚轮、键盘、IME 和脚本请求走 typed input queue。
- Capture 走 `FrameRing`，状态诊断走 `status` page。

示例：

```powershell
.\dist\win-x64\headless_browser.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

## 模块边界

- `src/main.rs`：CLI、handler loop、heartbeat watchdog 和 Unity 管理命令分发。
- `src/browser/`：CEF app/client、浏览器实例生命周期、OSR render handler。
- `src/modules/`：浏览器输入、输出和 capture 的业务适配层。
- `src/ipc/`：IPC v2 runtime，包括 mmap、header、queue、latest slot、frame ring 和 status page。
- `src/protocol/`：IPC v2 typed payload、命令、事件和 frame 结构。

文档和发布治理改动不应顺手修改 `src`。协议行为变更必须同时更新 `PROTOCOL.md` 和 `../docs/design/architecture.md`。

## IPC v2 开发约束

- 不兼容旧 MemoryStacks payload。
- 不再使用 `MemoryStacks_` 文件前缀。
- 不再使用 4 字节大端 length envelope。
- 不再使用首字节 flag 表示事件可用。
- 不在业务模块手写 offset；offset 必须封装在 typed protocol 或 IPC runtime 中。
- 所有跨进程共享结构使用 little-endian。
- 每个通道必须有 version、channel kind、sequence/ack 或 seqlock 提交语义。

Capture 走 `FrameRing`，鼠标移动走 latest-only，事件走 SPSC queue。详细设计见 `../docs/design/architecture.md`。

## 验证边界

按仓库约束，不能通过 Unity Editor、Unity batchmode、Unity Test Framework batchmode、BuildPipeline、`dotnet build` 或 Unity 生成的 `.sln/.csproj` 验证。

Rust 代码变更通常需要 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。仅文档和发布治理改动可用文本扫描、脚本审查和 diff 检查收口。
