# Headless Browser

Rust/CEF headless browser process for the Unity `EmbeddedBrowser` package.

当前集成方式是 Unity handler 模式：Unity 启动一个 `headless_browser.exe` 进程，并通过 `Handler.{handlerGuid}` 共享内存发送浏览器创建、移除、缩放和关闭命令。每个浏览器实例继续使用独立的模块共享内存，例如 `Capture.{browserGuid}`、`MouseState.{browserGuid}` 和 `KeyEvent.{browserGuid}`。

## 当前状态

- Windows 目标产物为 `dist/win-x64/headless_browser.exe`。
- `dist/win-x64` 同时放置 CEF runtime 文件，例如 `libcef.dll`、pak/dat/bin 文件和 `locales/`。
- Capture 使用 v2 布局：三槽像素缓冲、32 字节 header、dirty rect payload 和 ack sequence 门控。
- MouseState 固定为 6 字节，由 Unity 连续覆盖，Browser 端不回写 ack。

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
- CEF runtime：`libcef.dll`、`chrome_elf.dll`、`icudtl.dat`、`resources.pak`、`chrome_*.pak`、`v8_context_snapshot.bin`、`locales/` 等
- 可选调试文件：`headless_browser.pdb`

`dist/win-x64/debug.log` 是运行期日志，不应提交。

## Unity handler 模式

Unity 侧启动 handler 进程时，第一个非选项参数是 handler GUID：

```powershell
.\dist\win-x64\headless_browser.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto --heartbeat-timeout-ms=30000
```

进程启动后创建：

- `Handler.{handlerGuid}`：Unity 写入管理命令，Browser 读取并清除。
- `HEARTBEAT.{handlerGuid}`：Unity 周期性写入心跳，Browser watchdog 监控停跳。

浏览器实例由 `Handler` 命令创建。`AddBrowser` 命令携带 browser GUID、宽高和 URL；创建成功后该实例使用 browser GUID 派生各模块共享内存。

单 URL 模式仍可用于本地手工调试：

```powershell
.\target\release\headless_browser.exe --url https://example.com --width 1280 --height 720
```

不要把单 URL 模式写成 Unity 集成入口；Unity 集成入口是 handler GUID 模式。

## 共享内存模块

| 模块名 | 大小 | 方向 | 用途 |
| --- | --- | --- | --- |
| `Handler` | 3000 字节 | Unity -> Browser | handler 管理命令 |
| `HEARTBEAT` | 16 字节 | Unity -> Browser | handler 存活心跳 |
| `KeyEvent` | 16 字节 | Unity -> Browser | 键盘输入 |
| `MouseEvents` | 10 字节 | Unity -> Browser | 鼠标点击和滚轮 |
| `MouseState` | 6 字节 | Unity -> Browser | 鼠标位置和按钮状态 |
| `IME` | 2048 字节 | Unity -> Browser | 输入法 |
| `Caret` | 5 字节 | Browser -> Unity | 光标位置 |
| `Capture` | `3 * 2560 * 1440 * 4 + 32` 字节 | Browser -> Unity | Capture v2 帧数据 |
| `Script` | 10005 字节 | Unity -> Browser | JavaScript 执行 |

详细布局见 [PROTOCOL.md](PROTOCOL.md)。

## 开发文档

- [DEVELOPMENT.md](DEVELOPMENT.md)
- [PROTOCOL.md](PROTOCOL.md)
