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

Unity 集成不再按每个浏览器启动一个 `--guid --url` 进程。当前模型是：

- Unity 启动一个 handler 进程，首个非选项参数为 handler GUID。
- Browser 进程打开 `Handler.{handlerGuid}`，读取 `AddBrowser`、`RemoveBrowser`、`ResizeBrowser` 和 `Shutdown`。
- Browser 进程打开 `HEARTBEAT.{handlerGuid}`，通过递增 sequence 判断 Unity 是否仍存活。
- 每个 browser GUID 派生独立输入、输出和脚本模块共享内存。

示例：

```powershell
.\dist\win-x64\headless_browser.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

## 模块边界

- `src/main.rs`：CLI、handler loop、heartbeat watchdog 和 Unity 管理命令分发。
- `src/browser/`：CEF app/client、浏览器实例生命周期、OSR render handler。
- `src/modules/`：共享内存模块协议和各输入输出模块。
- `src/ipc/`：MemoryStacks 文件映射封装。

文档和发布治理改动不应顺手修改 `src`。协议行为变更必须同时更新 `PROTOCOL.md`。

## Capture v2 开发约束

Capture 共享内存不是旧的 `[width, height, pixels]` 单帧布局。当前布局为：

```text
3 * (2560 * 1440 * 4 byte slot) + 32 byte header
```

header 字段：

```text
0   i32 width
4   i32 height
8   i32 slot
12  i32 sequence
16  i32 frameType，0=full，1=dirty
20  i32 rectCount
24  i32 payloadSize
28  i32 ackSequence
```

dirty rect payload 中每个矩形使用 16 字节 header：

```text
x(i32), y(i32), width(i32), height(i32), BGRA pixels
```

Browser 只有在上一帧 sequence 已由 Unity ack 后才发布 dirty frame；否则必须回退 full frame。尺寸变化、dirty rect 为空、dirty rect 数量超过 64、dirty payload 超槽或 dirty 面积达到整帧 70% 时，也回退 full frame。

## MouseState 开发约束

`MouseState` 是 6 字节连续状态：

```text
flag(u8), x(i16), y(i16), buttonState(u8)
```

Browser 端不清 flag、不写 ack。Unity 端可以持续覆盖最新状态；Browser 端负责合并移动事件并按 CEF 输入事件发送。

## 验证边界

按仓库约束，不能通过 Unity Editor、Unity batchmode、Unity Test Framework batchmode、BuildPipeline、`dotnet build` 或 Unity 生成的 `.sln/.csproj` 验证。

Rust 代码变更通常需要 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。仅文档和发布治理改动可用文本扫描、脚本审查和 diff 检查收口。
