# Headless Browser

Rust/CEF headless browser process for the Unity `EmbeddedBrowser` package.

当前主线是 IPC v2 重构：Unity 启动一个 `headless_browser.exe` 进程，浏览器实例、输入、渲染帧、输出事件和状态诊断通过 Rust 共享 IPC core 通信。`headless_browser` 直接使用 Rust API，Unity 通过原生插件调用同一个 core；旧 MemoryStacks 单槽 flag 协议不再作为新实现目标。

## 当前状态

- Windows 目标产物为 `dist/win-x64/headless_browser.exe` 和 `dist/win-x64/browser_ipc_native.dll`。
- `dist/win-x64` 同时放置 CEF runtime 文件，例如 `libcef.dll`、pak/dat/bin 文件和 `locales/`。
- IPC v2 架构、wire format 和实施计划见 `../docs/design/architecture.md`。
- Capture 将作为 `FrameRing` 一等通道实现，不再是 MemoryStacks 特例。
- Mouse move 使用 latest-only 状态；点击、滚轮、键盘、IME 和脚本请求使用 typed queue。
- Status page 是必需通道，用于定位输入积压、丢帧、ack 延迟和进程状态。

## 项目结构

```text
headless_browser/
├── Cargo.toml
├── Cargo.lock
├── build.ps1
├── build.sh
├── dist/
│   └── win-x64/
│       ├── headless_browser.exe
│       ├── browser_ipc_native.dll
│       ├── libcef.dll
│       ├── *.pak / *.dat / *.bin
│       └── locales/
├── src/
│   ├── main.rs
│   ├── browser/
│   ├── ipc/
│   └── modules/
├── DEVELOPMENT.md
└── PROTOCOL.md
```

## Windows 发布目录

```powershell
.\setup-windows.ps1
.\build.ps1 -Release
```

脚本成功后，发布目录为：

```text
dist/win-x64/
```

该目录必须包含：

- `headless_browser.exe`
- `browser_ipc_native.dll`，供 Unity `BrowserIpcNative` 通过 `DllImport("browser_ipc_native")` 加载
- CEF runtime：`libcef.dll`、`chrome_elf.dll`、`icudtl.dat`、`resources.pak`、`chrome_*.pak`、`v8_context_snapshot.bin`、`locales/` 等
- 可选调试文件：`headless_browser.pdb`

同一 DLL 还会复制到 `../BrowserRenderer/Assets/Packages/Plugins/Windows/browser_ipc_native.dll`，这是 Unity Windows 插件加载位置。不要把 `target/` 或 `dist/` 产物提交到 Git；需要版本化 Unity 插件二进制时，应连同对应 `.meta` 一起纳入 `BrowserRenderer` 仓库。

`dist/win-x64/debug.log` 是运行期日志，不应提交。

## Unity handler 模式

Unity 侧启动 handler 进程时，第一个非选项参数是 handler GUID：

```powershell
.\dist\win-x64\headless_browser.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto --heartbeat-timeout-ms=30000
```

进程启动后创建 IPC v2 session，并通过 `control`、`status`、`input`、`frame` 和 `output` 通道完成浏览器管理、输入、帧发布和诊断。

单 URL 模式仍可用于本地手工调试：

```powershell
.\target\release\headless_browser.exe --url https://example.com --width 1280 --height 720
```

不要把单 URL 模式写成 Unity 集成入口；Unity 集成入口是 handler GUID 模式。

## IPC v2 通道

| 通道 | 方向 | 用途 |
| --- | --- | --- |
| `session` | 双向 | 协议版本、capabilities 和生命周期 |
| `control` | Unity -> Browser | 页面创建、移除、缩放、关闭和 DevTools |
| `status` | Browser -> Unity | 进程、输入、帧、错误和 counters |
| `input` | Unity -> Browser | 鼠标、键盘、IME 和脚本请求 |
| `frame` | Browser -> Unity | BGRA frame ring 和 dirty rect |
| `output` | Browser -> Unity | caret、surrounding text、脚本结果和页面事件 |

详细设计见 [Architecture](../docs/design/architecture.md)。`PROTOCOL.md` 后续会随 `crates/ipc` golden tests 改写为 v2 的正式 wire spec。

## 开发文档

- [DEVELOPMENT.md](DEVELOPMENT.md)
- [PROTOCOL.md](PROTOCOL.md)
- [Architecture](../docs/design/architecture.md)
