# 开发指南

## 环境要求

Windows：

- Rust toolchain
- Visual Studio Build Tools
- CEF 145.0.27，默认由 `setup-windows.ps1` 下载到 `%LOCALAPPDATA%\Lichora\cef\145.0.27-windows64` 并通过 `CEF_PATH` 指向。该目录不是单纯 runtime 目录，还必须包含 `CMakeLists.txt`、`cmake/`、`include/`、`libcef_dll/` 和 `archive.json`，供 `cef-dll-sys` 构建 `libcef_dll_wrapper`。

Linux：

- Rust toolchain
- CEF 145.0.27 Release 目录，并通过 `CEF_PATH` 指向
- GCC/Clang
- pkg-config

## 构建与发布

Windows 发布构建：

```powershell
.\setup-windows.ps1
.\build.ps1 -Release
```

如需放到其他目录，可显式传入安装路径：

```powershell
.\setup-windows.ps1 -InstallPath "E:\Repositories\.cache\cef\145.0.27-windows64"
```

`build.ps1` 成功后把 Rust 可执行文件、native IPC 插件和 CEF runtime 复制到：

```text
dist/win-x64/
```

该目录是宿主适配层应引用的 Windows runtime 目录。`debug.log` 是 CEF 运行期日志，已通过 `.gitignore` 排除。

`CEF_PATH` 指向开发机上的 CEF 根目录，`dist/win-x64` 才是运行时发布目录。不要把只含 `libcef.dll`、pak/dat/bin 和 `locales/` 的旧运行时目录设置为 `CEF_PATH`，否则 CEF 绑定 crate 无法编译。

Linux 发布构建：

```bash
export CEF_PATH=/path/to/cef/Release
./build.sh --release
```

`build.sh` 会构建 `lichora`、`lichora-ipc-native`、`process-host` 和 Linux native IME 插件，并组装：

```text
dist/linux-x64/
```

同时会把 `liblichora_ipc_native.so`、`libprocess_host.so` 和 `libnative_ime.so` 复制到 Unity host 插件目录。当前 Linux IPC importer 已禁用，插件 manifest 标记为 `release: false`；文件被复制不代表 Unity Linux IPC runtime 已进入发布支持面。

## Unity handler 运行模型

宿主集成按 IPC v2 session 模型运行。当前首个宿主适配是 Unity：

- Unity 启动一个 handler 进程，首个非选项参数为 session 或 handler GUID。
- handler GUID 用作 IPC v2 session namespace；`session` 和 `status` 已有协议与映射，宿主会打开两者，当前仅 `status` 暴露宿主读取 API，browser runtime 写入和 Unity 业务消费仍未形成完整闭环。
- 管理命令走 `control` queue；handler runtime 当前只处理 `Shutdown`、`AddBrowser`、`RemoveBrowser` 和 `ResizeBrowser`。
- 鼠标移动走 latest-only，点击、滚轮、键盘、IME 和脚本请求走 typed input queue。
- Capture 走 `FrameRing`。output 当前实际消费 caret、surrounding text 和 ownership；`ScriptResult`、`PageEvent` 仅完成协议解码，Unity Runtime 会丢弃。
- Web SDK 会尝试调用 `window.lichora`，但 runtime 当前未注入该对象，实际 bridge 走 console fallback。

示例：

```powershell
.\dist\win-x64\lichora.exe 12345678-1234-1234-1234-123456789abc --graphics-mode=auto
```

## 模块边界

- `src/main.rs`：进程入口、CEF 子进程分流、日志初始化和运行模式组合；旧 heartbeat 启动参数不再支持。
- `src/cli.rs`：CLI 参数模型、graphics mode 解析、handler GUID/URL 模式识别和 usage 输出。
- `src/handler.rs`：Unity handler loop、control queue、浏览器实例编排、控制命令分发和 handler 运行日志。
- `src/browser/`：CEF app/client、浏览器实例生命周期、OSR render handler。
- `src/modules/`：浏览器输入、输出和 capture 的业务适配层。
- `src/ipc/`：IPC v2 runtime 适配层。
- `crates/lichora-ipc/`：共享 IPC core，包括 mmap、header、queue、latest slot、frame ring、typed payload 和 status/output payload。
- `crates/lichora-ipc-native/`：宿主原生插件 C ABI；typed browser handle 是输入、帧和输出热路径的唯一接口，旧 `*_for_browser` session-handle 导出不再保留。
- `crates/process-host/`：宿主进程管理 C ABI，供 Unity IL2CPP 等无法稳定使用托管 `Process` 的环境调用。
- `hosts/unity-host/Scripts/Native/`：Unity 原生桥接程序集，集中承载公开 IPC/process/IME facade，以及内部 C# P/Invoke、raw handle、错误映射、输入 payload 编码和 native event buffer；Runtime 不接触底层原生实现类型。
- `hosts/unity-host/Scripts/Protocol/`：无 Unity/Native 引用的浏览器输出 payload 解码与 DTO；只由 Runtime 显式引用，Native 不依赖 Protocol。
- `hosts/unity-host/Scripts/Model/`：无 Unity/Native 引用的 ownership 基础枚举程序集，只承载 `InputOwner` 和 `InputRegionShape`；Unity 序列化和坐标相关的 `InputRegion`、`InputOwnershipMap`、render snapshot 仍属于 Runtime。

文档和发布治理改动不应顺手修改 `src`。协议行为变更必须同时更新 `docs/protocol.md`、`crates/lichora-ipc` tests 和相关 `docs/design` 文档。

## IPC v2 开发约束

- 不在业务模块手写 offset；offset 必须封装在 typed protocol 或 IPC runtime 中。
- 所有跨进程共享结构使用 little-endian。
- 每个通道必须有 version、channel kind、sequence/ack 或 seqlock 提交语义。

Capture 走 `FrameRing`，鼠标移动走 latest-only，事件走 SPSC queue。Web/Host 融合设计见 `design/web-host-overlay.md`。

## 验证边界

按仓库约束，不能通过 Unity Editor、Unity batchmode、Unity Test Framework batchmode、BuildPipeline、`dotnet build` 或 Unity 生成的 `.sln/.csproj` 验证。

Rust 代码变更通常需要 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。仅文档和发布治理改动可用文本扫描、脚本审查和 diff 检查收口。

仓库级质量检查入口为 `.\scripts\check-quality.ps1`。Unity 部分只进行源码与配置静态检查，不启动 Unity Editor，不执行 Unity batchmode、BuildPipeline 或 Unity 生成项目编译。

当前架构依据以根目录 `README.md`、本文档、`protocol.md` 和 `design/` 下的设计文档为准。`archive/` 下的材料只用于历史追溯。

## 仓库质量检查入口

从仓库根目录执行：

```powershell
.\scripts\check-quality.ps1
```

默认检查顺序为 Rust `cargo fmt --all -- --check`、Rust `cargo clippy --all-targets --all-features -- -D warnings`、Rust `cargo test --no-default-features --all-targets`，然后在 overlay 中执行 `bun install --frozen-lockfile` 和 `bun run check`，再执行 Unity C# 的 CSharpier，最后执行 `scripts/check-plugin-contract.ps1`。Bun 按 lockfile 安装固定依赖版本，本地 `node_modules` 由 `.gitignore` 忽略。Unity C# 文件从 `hosts/unity-host` Package 根递归收集，因此 Runtime、Editor、Samples 和未跟踪的新源码都会进入检查；该 Package 目录不包含 Unity `Library` 等宿主工程生成目录。

脚本支持 `-SkipRust`、`-SkipOverlay`、`-SkipUnity` 和 `-SkipPackaging`。任何阶段失败都会立即退出，并输出失败阶段。该入口不启动 Unity Editor，不执行 Unity batchmode、Unity Test Framework batchmode、BuildPipeline、`dotnet build`、`msbuild` 或 Unity 生成项目编译。
