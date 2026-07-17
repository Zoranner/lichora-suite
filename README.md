# Lichora Runtime

Lichora Runtime 是 Lichora 的浏览器运行时。它负责启动 CEF 离屏浏览器，把网页渲染为宿主可读取的帧数据，并通过原生 IPC 接收输入和输出页面事件。

Unity、其他引擎或桌面程序不需要直接集成 CEF。宿主只需要启动 runtime，并通过配套 native plugins 与它通信。

## 下载

GitHub Release 会发布以下运行时包：

| 平台 | 文件 |
| --- | --- |
| Windows x64 | `lichora-win-x64-<tag>.zip` |
| Linux x64 | `lichora-linux-x64-<tag>.tar.gz` |
| macOS x64 | `lichora-macos-x64-<tag>.tar.gz` |

Release 还会发布：

```text
lichora-native-plugins-<tag>.zip
```

这个聚合包只包含宿主侧需要加载的 native plugins，适合 Unity host 或其他宿主适配项目单独下载使用。

当前 release 不包含 Linux arm64 或 macOS arm64 产物。

## 运行时目录

解压平台包后，目录中会包含 runtime 可执行文件、原生库和 CEF runtime 文件。

Windows x64：

```text
lichora.exe
lichora_ipc_native.dll
process_host.dll
libcef.dll
locales/
*.pak
*.dat
```

Linux x64：

```text
lichora
liblichora_ipc_native.so
libprocess_host.so
libnative_ime.so
libcef.so
locales/
*.pak
*.dat
```

macOS x64：

```text
lichora
liblichora_ipc_native.dylib
libprocess_host.dylib
CEF runtime files
```

## 宿主集成

宿主程序通常不直接让用户启动 `lichora`。推荐流程是：

1. 宿主生成一个 session / handler GUID。
2. 宿主通过 `process_host` 启动 runtime。
3. 宿主通过 `lichora_ipc_native` 打开 IPC 通道。
4. 宿主发送创建浏览器、调整尺寸、输入事件等命令。
5. 宿主读取 frame / output 通道并显示结果。

启动 handler 的基本形式：

```powershell
.\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

Linux / macOS：

```bash
./lichora 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

单 URL 模式只适合本地手工调试，不是宿主集成入口：

```powershell
.\lichora.exe --url https://example.com --width 1280 --height 720
```

## 配套项目

- `lichora-host-unity`：Unity 宿主包。
- `lichora-overlay`：网页侧 overlay / input ownership SDK，包名为 `@lichora/overlay`。

## 从源码构建

Windows：

```powershell
.\build.ps1 -Release
```

Linux / macOS：

```bash
./build.sh --release
```

如果没有设置 `CEF_PATH`，构建脚本会下载并安装对应平台的 CEF。已经设置 `CEF_PATH` 时，该路径必须指向完整 CEF build layout，而不是只包含 runtime 文件的目录。

常用环境变量：

| 变量 | 用途 |
| --- | --- |
| `CEF_PATH` | 指定完整 CEF build layout |
| `CEF_RUNTIME_PATH` | 指定要打包进 `dist` 的 CEF runtime 文件目录 |
| `CEF_VERSION` | 覆盖默认 CEF 版本前缀 |
| `CEF_PLATFORM` | 覆盖 CEF 平台 archive 名称 |

## 质量检查

提交 runtime 代码前运行：

```powershell
.\scripts\check-quality.ps1
```

该入口会运行 Rust 格式检查、Clippy 和测试。它只覆盖 runtime 仓库，不覆盖 Unity host 或 overlay；这两个项目在各自仓库维护检查入口。

## 文档

- [开发说明](docs/development.md)
- [IPC 协议](docs/protocol.md)
- [工程评审记录](docs/reviews/)
