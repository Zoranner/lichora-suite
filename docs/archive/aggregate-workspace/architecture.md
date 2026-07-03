# EmbeddedBrowser Architecture

> 历史设计说明：本文来自旧聚合工作区设计阶段，保留用于追溯迁移背景。当前 Lichora 协议和开发入口见 `../../protocol.md`、`../../development.md` 和 `../../design/web-host-overlay.md`。

## 定位

本文是 `EmbeddedBrowser` 新阶段的唯一设计入口，覆盖工程结构、IPC 架构、wire format 和实施计划。

新设计不兼容旧 MemoryStacks 协议，不沿用旧 `.NET HeadlessBrowser` 的组织方式。旧实现可以作为历史参考和问题来源，但不能限制新结构。

## 目标

- Rust 入口轻量化，runtime、browser、IPC、protocol、CEF adapter 分层清楚。
- Unity 端 `PageRenderer` 保持易用，底层拆成进程 runtime、页面 session、native IPC bridge、输入、frame、status、Unity UI bridge。
- IPC core 由 Rust 实现一次；`headless_browser` 直接使用 Rust API，Unity 通过原生插件 P/Invoke 调用同一个 core。
- 鼠标交互稳定跟手，移动状态不被事件积压拖慢。
- Capture 发布、消费、ack 和丢帧规则清晰，不出现半新半旧的撕裂帧。
- status page 和 counters 是必需能力，可以直接定位输入积压、丢帧、协议不匹配和进程异常退出。
- 旧 `.NET HeadlessBrowser`、`MemoryStacks` 和历史评审退到 archive/reference，不参与新主链路。
- 发布目录、本机配置、生成物和验证边界清楚。

## 非目标

- 不保留旧协议运行时 fallback。
- 不保留 `MemoryStacks_` 文件名、4 字节 length envelope 或首字节 flag 事件模型。
- 不把 wincast 的 H.264/TCP 推流协议搬进本项目。
- 不把鼠标移动做成普通事件队列。
- 不让 Unity C# 维护第二套 mmap、queue、frame ring 或 wire codec。
- 不同时重写纹理上传策略、CEF 业务行为和 IPC 协议；它们按阶段拆开验证。

## 仓库边界

当前聚合目录不是单一 Git 仓库。状态、提交和验证必须落到具体子仓。

| 目录 | 新定位 |
| --- | --- |
| `BrowserRenderer/` | Unity UPM 包，当前 Unity 主链路 |
| `headless_browser/` | Rust/CEF 浏览器进程，当前 native runtime 主链路 |
| `HeadlessBrowser/` | 旧 .NET 实现，历史/参考，不参与 IPC v2 |
| `MemoryStacks/` | 旧共享内存库，历史/参考，IPC v2 完成后主链路不依赖 |
| `native-ime/` | 平台 IME 辅助能力，按实际引用决定是否保留 |
| `docs/` | 当前设计、历史评审、归档说明 |

## Rust 目标结构

```text
headless_browser/
  Cargo.toml
  Cargo.lock
  crates/
    ipc/
      src/
        transport/
        channel/
        protocol/
        status/
      tests/
        protocol_golden.rs
        ipc_channel.rs
    ipc_native/
      src/
        lib.rs              # C ABI for Unity P/Invoke
      include/
        ipc.h
  build.ps1 / build.sh
  src/
    main.rs                 # CEF subprocess dispatch, logging, CLI, runtime start
    lib.rs                  # minimal public API
    cli/
      args.rs
      graphics.rs
      single_instance.rs
    runtime/
      config.rs
      handler.rs
      registry.rs
      session.rs
      heartbeat.rs
      status.rs
      shutdown.rs
      run_log.rs
    browser/
      config.rs
      cef_app.rs
      client.rs
      lifecycle.rs
      render.rs
      input_adapter.rs
      output_adapter.rs
    modules/
      capture.rs            # dirty rect / frame business adaptation only
      ime.rs
      keyboard.rs
      mouse.rs
      script.rs
      text.rs
  tests/
    protocol_golden.rs
    ipc_channel.rs
    runtime_commands.rs
```

`ipc/` 是无 Unity、无 CEF 依赖的纯 IPC core。它拥有 mmap、命名、header、seqlock、SPSC queue、latest slot、frame ring、status page、typed payload codec 和 golden tests。

`ipc_native/` 只暴露 C ABI，供 Unity P/Invoke。它包装 `ipc` core 的 opaque handle、错误码、结构体和 buffer 操作，不放业务逻辑。拆成两个 crate 是为了隔离纯 core 与 C ABI/cdylib/export header 边界，不是两套实现。

`headless_browser` 通过 `ipc` 的 Rust API 访问通道，不通过 FFI 调自己。

`main.rs` 只保留 CEF subprocess 分流、日志初始化、CLI 解析和启动 runtime。handler loop、heartbeat、browser map、process lock、run log、graphics mode 细节移出。

`browser/` 只表达 CEF browser lifecycle、render callback 和 input dispatch。`render.rs` 依赖 `FramePublisher` / `OutputPublisher` trait，不直接依赖共享内存 wrapper。

`modules/` 从“共享内存模块”改为“浏览器业务适配模块”，只保留 dirty rect 裁剪、surrounding text 解析、IME/keyboard/mouse 转换等算法。

## Unity 目标结构

```text
BrowserRenderer/Assets/Packages/
  package.json
  Runtime/
    KimoTech.EmbeddedBrowser.Runtime.asmdef
    Components/
      PageRenderer.cs
    Runtime/
      BrowserRuntime.cs
      BrowserProcessService.cs
      BrowserRestartPolicy.cs
      BrowserConfig.cs
    Sessions/
      BrowserPageSession.cs
      BrowserPageState.cs
    Ipc/
      BrowserIpcNative.cs
      NativeIpcHandle.cs
      NativeIpcResult.cs
      NativeIpcTypes.cs
      BrowserIpcClient.cs
    Input/
      BrowserInputBridge.cs
      KeyCodeMapper.cs
      NativeImeBridge.cs
      LinuxNativeImeModule.cs
    Rendering/
      BrowserTextureTarget.cs
      BrowserFrameReceiver.cs
      CaptureFrame.cs
    UI/
      PointableUI.cs
      PageRendererBridge.cs
      Native/
        NativeProcessSpawner.cs
      Resources/
        EmbeddedBrowser/
          BrowserRender.mat
          BrowserRender.shader
  Editor/
    KimoTech.EmbeddedBrowser.Editor.asmdef
    PageRendererEditor.cs
  Plugins/
    Windows/
      browser_ipc_native.dll
    Linux/
      libbrowser_ipc_native.so
  Tests/
    Editor/
    Runtime/
  Documentation~/
```

如果一次移动文件风险过高，可以阶段内先落到 `Scripts/Runtime|Sessions|Ipc|Input|Rendering`，但最终结构不应继续以 `Scripts` 作为所有职责的大桶。

`PageRenderer` 保持唯一公开 MonoBehaviour 入口，Inspector API 尽量不变；拆出地址解析、纹理创建/重建/Apply、frame copy、restart reconnect。

`BrowserStatic` 退场或缩到 Unity lifecycle wrapper。进程启动、重启、status 和页面 registry 转到普通 C# service。

`PageHandler` 退为 `BrowserPageSession` facade，不再聚合旧共享内存模块。

`Ipc/` 只包含 P/Invoke 声明、opaque handle 包装、错误码转换、Unity-friendly facade。协议字段和通道算法由 Rust `ipc` core 统一实现。

Unity C# 可以保留少量 mirror enum 和 blittable struct，用于调用 FFI，但不能成为第二套协议实现。字段布局变化以 Rust core 和本文为准。

`Resources/` 如果继续使用，应放在 `Resources/EmbeddedBrowser/` 这类命名空间目录，避免全局路径污染。更理想是通过序列化默认材质或 package runtime asset 引用。

`allowUnsafeCode` 应尽量限制到真正需要把 `NativeArray` 指针传给 native plugin 的 Rendering/IPC bridge assembly。

## 进程模型

```text
Unity process
  PageRenderer
  BrowserPageSession bridge
  Ipc C# facade
    BrowserIpcNative P/Invoke
    BrowserIpcClient
    ControlClient
    InputWriter
    FrameReceiver
    OutputReader
    StatusReader

Rust headless_browser process
  IpcRuntime
    SessionRuntime
    ControlQueue
    StatusPage
    BrowserRegistry
  BrowserSession
    InputChannels
    FrameRing
    OutputChannels
    BrowserEntry
    CEF event loop

Rust shared IPC core
  ipc
    transport
    channel
    protocol
    frame ring
    status page
  ipc_native
    C ABI for Unity
```

## IPC 通道

| 通道 | 方向 | 语义 |
| --- | --- | --- |
| `session` | 双向 | 协议版本、capabilities、全局生命周期 |
| `control` | Unity -> Rust | AddBrowser、RemoveBrowser、ResizeBrowser、Shutdown、DevTools |
| `status` | Rust -> Unity | handler、browser、input、frame、error counters |
| `input` | Unity -> Rust | 鼠标、键盘、IME、脚本请求 |
| `frame` | Rust -> Unity | BGRA frame ring、dirty rect、frame ack |
| `output` | Rust -> Unity | caret、surrounding text、script result、页面事件 |

文件命名采用项目自有前缀：

```text
EmbeddedBrowser_{sessionId}_session
EmbeddedBrowser_{sessionId}_control
EmbeddedBrowser_{sessionId}_status
EmbeddedBrowser_{sessionId}_{browserId}_input
EmbeddedBrowser_{sessionId}_{browserId}_frame
EmbeddedBrowser_{sessionId}_{browserId}_output
```

## 背压模型

- 鼠标移动：latest-only。Unity 覆盖最新状态，Rust 只读最新值，不追历史移动。
- 鼠标点击、键盘、IME commit、脚本请求：SPSC queue。不能被 latest 覆盖，队列满时记录 drop 或返回错误。
- 鼠标滚轮：SPSC queue，允许合并连续 delta。
- IME composition：允许覆盖未消费的旧 composition，但 commit/cancel 不覆盖。
- Capture frame：frame ring + ack。同尺寸上一帧未 ack 时丢弃 paint 并计数；resize 可发布 full frame 打断等待。
- 状态诊断：latest/status page。持续覆盖，不排队。

## Native FFI

Unity 侧调用 Rust native plugin 的边界保持粗粒度，避免每个字段一次 P/Invoke：

- `ebi_session_open` / `ebi_session_close`
- `ebi_control_send`
- `ebi_input_set_mouse_latest`
- `ebi_input_push_event`
- `ebi_frame_try_copy_latest`
- `ebi_frame_ack`
- `ebi_output_try_read`
- `ebi_status_read`
- `ebi_error_message`

FFI 规则：

- 所有导出函数使用 C ABI 和稳定错误码。
- C# 持有 opaque handle，不持有 Rust 内部指针。
- 可变字符串由调用方传入 UTF-8 pointer + length。
- frame copy 支持 Unity 传入 `NativeArray<byte>` 的目标指针和长度，由 Rust core 校验尺寸并复制。
- native 层不回调 Unity；Unity 在主线程或受控 worker 中主动 poll。
- 不跨 FFI 抛异常，不跨 FFI 分配需要 C# 释放的复杂对象。
- Rust API 和 FFI API 共用同一套 codec、error code 和 tests。

## Wire Format

所有多字节字段使用 little-endian。每个共享内存段从 64 字节 `ChannelHeader` 开始。

```text
offset size type  name
0      4    u32   magic = 0x32494245  // "EBI2"
4      2    u16   versionMajor = 2
6      2    u16   versionMinor
8      4    u32   headerSize = 64
12     4    u32   channelKind
16     4    u32   flags
20     4    u32   capacityBytes
24     8    u64   producerSequence
32     8    u64   consumerAck
40     8    i64   producerTicks
48     4    i32   statusCode
52     4    u32   payloadOffset
56     8    u64   headerCommit
```

`headerCommit` 使用 seqlock：producer 写入前递增为奇数，写完 payload/header 后递增为偶数；consumer 前后读取 commit，两次相同且为偶数才接受数据。

Channel kind：

```text
1 Session
2 Control
3 Status
4 Input
5 Frame
6 Output
```

SPSC queue header：

```text
offset size type  name
0      4    u32   itemCapacity
4      4    u32   itemSize
8      8    u64   writeSequence
16     8    u64   readSequence
24     8    u64   droppedCount
32     8    u64   mergedCount
40     24   bytes reserved
64     N    bytes items
```

Frame header extends `ChannelHeader`：

```text
64     4    i32  width
68     4    i32  height
72     4    u32  pixelFormat       // 1 = BGRA32
76     4    u32  slotCount
80     4    u32  slotSize
84     4    u32  currentSlot
88     8    u64  frameSequence
96     8    u64  acknowledgedFrame
104    4    u32  frameType         // 1 = full, 2 = dirty
108    4    u32  rectCount
112    4    u32  payloadBytes
116    4    u32  dirtyHeaderSize
120    8    u64  droppedPaintCount
128    8    u64  publishedFrameCount
136    8    u64  submittedPaintCount
144    8    u64  reserved
```

完整字段和 golden vectors 后续应随 `crates/ipc` 测试一起维护；不要在 README 复制完整字段表。

## 可观测性

status page 至少提供：

- process state、browser count、protocol version。
- last command id、last command status、last error code/message。
- input received/processed/dropped。
- frame submitted/published/dropped/acked。
- last frame sequence、last frame ack、last frame age。
- browser state：creating、loading、ready、resizing、closing、failed。

这些 counters 必须优先用于卡顿排查。日志只作为补充。

## 发布和配置

`headless_browser/dist/` 必须明确归属：

- 如果是本地生成物：加入 ignore，发布由脚本生成，Unity 配置使用相对模板或本地覆盖。
- 如果是正式交付物：走 release artifact、制品库或 LFS，不普通提交整包。

`debug.log`、临时 PDB、浏览器 cache、Unity `Library/Temp/Logs/UserSettings` 都不得成为默认提交内容。

Unity 默认配置：

- 不提交本机绝对路径。
- `GraphicsMode` 使用 `Auto`、`On`、`Off`。
- Rust runtime path 以相对路径或发布流程写入。

## 当前状态与后续治理

- 当前主链路是 Rust IPC v2：`headless_browser` 使用 Rust API，Unity 通过 `browser_ipc_native` P/Invoke 调用同一套 IPC core。
- `BrowserRenderer` 包元数据不应声明 `org.nuget.memorystacks`；如再次出现，应作为包治理问题处理，而不是协议兼容需求。
- Unity 侧 `Ipc/` 只做 native bridge 和 facade，不复制 Rust IPC core。
- Capture 使用 `FrameRing`；鼠标移动使用 latest-only；点击、滚轮、键盘、IME 和脚本请求使用 typed queue；output/status 作为诊断和结果通道。
- 旧 `.NET HeadlessBrowser`、旧共享内存库和历史评审只作为参考资料保留，不参与当前运行链路。
- 后续结构整理应继续把公开组件、runtime service、session、input、rendering 和 native bridge 分层，避免重新引入旧模块桶。
- `dist/`、CEF runtime、PDB、运行日志和 Unity 插件二进制的版本化策略必须在发布任务中单独确认。

## 验证

Rust 修改后：

```powershell
cargo fmt --all
cargo test -p ipc
cargo test -p ipc_native
cargo test --lib
cargo clippy --all-targets --all-features -- -D warnings
```

Unity C# 修改后：

```powershell
csharpier check <changed-cs-files>
git -C BrowserRenderer diff --check
```

边界扫描：

```powershell
rg -n "MemoryStacks|MemoryStack|SharedMemoryWrapper|write_byte_at|read_bytes|WriteBytes|ReadBytes|MemoryModuleBase|MemoryMappedChannel|SpscQueue|FrameRingReader" BrowserRenderer headless_browser -g "*.cs" -g "*.rs" -g "*.md"
```

禁止项：

- 不启动 Unity Editor。
- 不运行 Unity batchmode、Unity Test Framework batchmode 或 BuildPipeline。
- 不用 `dotnet build` / `msbuild` 编译 Unity 生成项目。
- 不修改 Unity 生成的 `.csproj`、`.sln`、`Library/`、`Temp/`。

Unity 内效果只能由用户在 Editor/Player 中实测；源码侧不得把未实测状态说成端到端完成。
