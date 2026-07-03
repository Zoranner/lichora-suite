# Lichora Web Host Overlay 落地设计

## 背景

`docs/design/web-host-overlay.md` 已经定义了 Web 与宿主场景融合的产品方向：浏览器画面可以透明显示，受控网页可以声明鼠标穿透区域，宿主在本地完成输入路由。结合当前代码状态，直接在 `PageRenderer`、`PointableUI` 和 `PageHandler` 上继续加功能会形成补丁式实现。

当前 Unity host 的主要问题是职责混杂：

- `PageRenderer` 同时管理页面生命周期、材质、纹理、IPC、resize、重启恢复、键盘、IME、鼠标和脚本执行。
- `PointableUI` 同时承担 Unity 事件接收、浏览器坐标映射、焦点、键盘、IME 和鼠标状态。
- `PageHandler` 名义上是页面 IPC facade，但同时处理鼠标差分、按钮事件推导、frame copy 和 output drain。
- `BrowserOutputState` 只消费 output latest，并把 caret、surrounding text、script result、page event 的入口揉在一起；后续 `OverlayPassMap` 不适合继续塞进这个状态类。
- Rust capture 当前默认把浏览器帧发布为不透明 BGRA，这是普通网页显示的正确默认；未来 `BrowserAlpha` 必须是显式模式，不能恢复为全局保留 alpha。

因此，第一目标不是补一个 PassMap，而是重建 Unity host 的页面、渲染、输入、输出边界。后续 overlay、DOM bridge 和 BrowserAlpha 都建立在这个边界上。

## 设计原则

- 不要求向后兼容旧公开字段和旧组件继承关系，可以删除或替换 `FilteredColor`、`PointableUI` 基类模式和隐式输入行为。
- 先把输入穿透的宿主本地链路做正确，再接网页端 SDK 和 Rust bridge。
- 透明显示和输入穿透必须解耦。颜色键透明不自动代表点击穿透。
- 鼠标移动热路径不依赖 JS 往返，也不读取纹理像素判断命中。
- 失败默认归浏览器接收输入，避免网页 UI 因 pass map 过期或非法而失控。
- 每个阶段都要能独立验证，不能让 Unity、Rust、JS 三端同时大幅变化后才验证。

## 目标架构

`PageRenderer` 保留为 Unity 组件入口，但内部只负责装配和生命周期协调。真正职责拆到聚合对象：

| 组件 | 职责 |
| --- | --- |
| `BrowserPageSession` | 保存 page id、address、尺寸；创建和销毁页面级 IPC writer/reader；处理 runtime restart 后重连 |
| `BrowserSurface` | 管理 `RawImage`、`Texture2D`、材质实例、透明模式和 frame apply |
| `BrowserFramePump` | 从 IPC frame reader 拷贝最新帧，驱动 `BrowserSurface` 更新纹理 |
| `BrowserOutputPump` | 读取 output latest 和 output queue，分发 caret、surrounding text、page event、overlay state |
| `BrowserFocusController` | 管理浏览器焦点、键盘输入、IME 开关和 Linux native IME 生命周期 |
| `BrowserPointerInputSource` | 接收 Unity pointer/scroll 事件，生成标准化 pointer event |
| `BrowserCoordinateMapper` | 统一屏幕坐标、RawImage local 坐标、浏览器像素坐标和 CSS viewport 坐标 |
| `BrowserInputRouter` | 根据 hit mode、pass map 和 pointer capture 决定事件归浏览器还是宿主 |
| `BrowserPointerCapture` | pointer down 后锁定归属，直到 pointer up/cancel/restart |
| `PassMap` | 保存本地可查询的穿透区域状态和版本 |
| `PassHitFilter` | 与 Unity raycast 协作，在穿透区域让宿主继续命中底层对象 |

建议目录：

```text
hosts/unity-host/Scripts/
  Runtime/
    BrowserPageSession.cs
    BrowserRuntimeEvents.cs
  Rendering/
    BrowserSurface.cs
    BrowserFramePump.cs
    BrowserRenderSettings.cs
    BrowserTransparencyMode.cs
  Input/
    BrowserPointerInputSource.cs
    BrowserInputRouter.cs
    BrowserPointerCapture.cs
    BrowserCoordinateMapper.cs
    BrowserFocusController.cs
    PointerHitMode.cs
  Overlay/
    PassMap.cs
    PassRegion.cs
    PassHitFilter.cs
    BrowserOverlaySettings.cs
  Ipc/
    BrowserIpcOutputPayload.cs
    BrowserIpcOutputPump.cs
```

当前 `PageHandler` 应拆成更窄的 `BrowserPageSession` 和若干 pump/router，不再作为所有页面行为的集中类。`PointableUI` 不再作为浏览器输入基类；如果其他项目还需要通用 pointable 控件，应独立保留，但 Lichora host 不再继承它。

## 渲染模型

第一版渲染模式：

| 模式 | 说明 |
| --- | --- |
| `Opaque` | 默认模式。Rust 发布 BGRA 时 alpha 固定为 255，Unity shader 默认输出 alpha 1 |
| `ColorKey` | 任意网站快速融合模式。Unity shader 按颜色键调整 alpha，但输入仍由 hit mode 决定 |
| `BrowserAlpha` | 后续受控网页模式。Rust 保留 CEF alpha，Unity 材质使用 alpha blend |

`BrowserSurface` 负责把配置映射到材质参数和纹理创建参数：

- 浏览器网页纹理默认按 sRGB 创建，即 `Texture2D(..., linear: false)`。
- `Opaque` 和 `ColorKey` 可以共享当前 `BrowserRender.shader`，但参数必须从 `BrowserRenderSettings` 注入，不再用 `FilteredColor` bool。
- `BrowserAlpha` 不作为第一阶段实现内容，只在接口上预留；启用前必须完成 Rust capture mode 和 CEF transparent background 配置。

Rust `CaptureModule` 应保留当前 opaque 默认行为。未来新增显式配置：

```text
FrameAlphaMode
  Opaque
  PreserveBrowserAlpha
```

该配置应来自 handler/page 创建命令，而不是全局环境变量。

## 输入模型

输入路由分三层：

```text
Unity pointer event
  -> BrowserPointerInputSource
  -> BrowserCoordinateMapper
  -> BrowserInputRouter
  -> BrowserPageSession 或宿主继续 raycast
```

第一版 hit mode：

| 模式 | 行为 |
| --- | --- |
| `FullBrowserSurface` | 整块浏览器表面接收鼠标、滚轮和焦点，等价于当前行为 |
| `StaticPassRects` | 使用 Unity inspector 配置的本地矩形区域穿透宿主 |
| `DomPassMap` | 使用网页 SDK 通过 IPC 推送的 pass map |

`StaticPassRects` 必须先实现。它不依赖 Rust 和网页端，能验证真正困难的 Unity 本地链路：

- `PassHitFilter` 在 pass-through 点返回 false，让底层宿主对象继续被 raycast。
- `BrowserInputRouter` 在 pass-through 点不向浏览器发送 pointer down/move/up。
- `BrowserPointerCapture` 保证拖动期间归属稳定。
- wheel 不使用 capture，每次按当前位置重新判断。

pointer capture 规则：

| 事件 | 规则 |
| --- | --- |
| pointer down | 当前命中浏览器则锁定浏览器，命中宿主则锁定宿主 |
| pointer move | 有锁定时沿用锁定目标 |
| pointer up | 发送给锁定目标后释放 |
| pointer cancel / runtime restart | 释放锁定 |
| wheel | 不锁定，每次按当前点判断 |

焦点规则：

- 点击浏览器区域时，浏览器获得焦点，键盘和 IME 进入浏览器。
- 点击宿主 pass-through 区域时，默认释放浏览器焦点。
- 后续可加 `KeepBrowserFocusOnHostClick`，但不是第一阶段默认行为。

## Overlay PassMap

`PassMap` 是宿主本地查询的数据，不是高频事件流：

```text
PassMap
  version: ulong
  viewportWidth: int
  viewportHeight: int
  deviceScaleFactor: float
  enabled: bool
  regions: PassRegion[]

PassRegion
  id: uint
  x: float
  y: float
  width: float
  height: float
  disabled: bool
```

第一版只支持矩形。坐标使用 CSS viewport 坐标。`BrowserCoordinateMapper` 负责把 Unity pointer 坐标转换为 CSS viewport 坐标后查询 `PassMap.Contains(cssX, cssY)`。

过期策略：

- resize 后旧 pass map 标记过期。
- handler restart 后清空 pass map。
- pass map viewport 与当前浏览器尺寸不匹配时默认浏览器接收。
- region 数量超过限制或 payload 非法时丢弃，并记录诊断计数。

## IPC 扩展

在 Unity 本地输入链路稳定后，再扩展 IPC：

Rust `crates/lichora-ipc/src/typed_payload.rs` 新增：

```text
OutputPayloadKind::OverlayPassMap = 5
OutputPayload::OverlayPassMap(OverlayPassMapOutput)
```

编码建议：

```text
u64 version
i32 viewport_width
i32 viewport_height
f32 device_scale_factor
u8 enabled
u8 reserved[7]
u32 region_count
repeated PassRegion
```

`PassRegion`：

```text
u32 id
u8 shape
u8 disabled
u16 reserved
f32 x
f32 y
f32 width
f32 height
```

Unity 端 `BrowserIpcOutputPayload` 解码该结构，但不直接改输入状态。它只把 `OverlayPassMap` 交给 `BrowserOutputPump`，再由 `BrowserInputRouter` 的 pass map store 消费。

当前 Rust output channel 同时 publish latest 和 queue。`OverlayPassMap` 应作为 latest state 读取；DOM bridge 普通事件后续走 queue，不与 pass map 混用。

## Web SDK 和 Bridge

网页端 SDK 在 IPC 和 Unity 本地链路稳定后实现，目录为：

```text
packages/overlay/
  package.json
  src/
    index.ts
    pass-map.ts
    bridge.ts
    observers.ts
```

Node 相关命令必须使用 bun。

第一版 SDK 只负责：

- 扫描 `data-overlay="pass"` 元素。
- 使用 `getBoundingClientRect()` 生成矩形 pass region。
- 使用 `ResizeObserver`、`MutationObserver`、scroll 和 resize 触发更新。
- 节流输出。
- 限制 region 数量和 payload 大小。
- 通过 bridge 发送 pass map。

SDK 不负责 UI 框架适配，不管理业务弹窗状态，不处理宿主场景逻辑。普通 DOM 覆盖物遮挡可以作为第二轮能力，不进入第一版核心链路。

bridge 路线：

- 正式目标是 CEF message route 或 process message。
- 如果先用 console 前缀过渡，必须封装在 Rust `overlay` 模块中，并标记为临时 bridge，不让 Unity 侧解析 JSON。
- 宿主侧长期只消费 typed `OverlayPassMap`。

## Rust 模块边界

新增 Rust 模块：

```text
src/modules/overlay.rs
```

职责：

- 接收网页 bridge 消息。
- 校验版本、大小、region 数量和坐标范围。
- 转换为 typed `OverlayPassMap`。
- publish 到 output latest。
- 记录 parse error、dropped、version、region count 等计数。

不承担：

- 不参与 Unity raycast。
- 不在 mouse move 时查询 DOM。
- 不把 JSON 透传给 Unity。

后续 status counters 可扩展：

- `overlay_pass_map_version`
- `overlay_pass_map_updates`
- `overlay_pass_map_dropped`
- `overlay_pass_map_parse_errors`
- `overlay_region_count`
- `overlay_last_update_age_ms`

## 实施阶段

### Unity host 边界重构

目标：不改变功能表面，拆开 `PageRenderer`、`PointableUI`、`PageHandler` 的职责。

交付：

- `PageRenderer` 成为装配入口。
- `BrowserSurface` 独立管理纹理和材质。
- `BrowserPageSession` 独立管理 IPC reader/writer 和页面尺寸。
- `BrowserFramePump` 独立处理 frame copy。
- `BrowserOutputPump` 独立处理 output latest。
- `BrowserFocusController` 接管键盘和 IME。
- 移除 `PageRenderer : PointableUI` 的浏览器输入继承关系。

验证：

- CSharpier 检查所有改动 C# 文件。
- 文本扫描确认 `PageRenderer` 不再直接处理键盘、IME 和 frame copy 细节。
- 用户侧 Unity 实测现有浏览器显示、点击、键盘、IME、resize、restart 不退化。

### 本地输入穿透

目标：实现不依赖网页 SDK 的 `StaticPassRects`，验证输入路由根基。

交付：

- `PointerHitMode`
- `BrowserOverlaySettings`
- `PassMap`
- `PassRegion`
- `PassHitFilter`
- `BrowserInputRouter`
- `BrowserPointerCapture`
- `BrowserCoordinateMapper`

验证：

- 全浏览器模式保持当前行为。
- 中间 static pass rect 可点击底层宿主。
- down 在宿主区域后拖动经过浏览器区域不抢输入。
- down 在浏览器区域后拖动经过 pass 区域仍发给浏览器。
- wheel 按当前位置路由。

### IPC PassMap

目标：让宿主能消费结构化 overlay pass map。

交付：

- Rust typed payload 增加 `OverlayPassMap`。
- Rust golden tests 覆盖编码。
- Unity output payload decoder 增加 `OverlayPassMap`。
- `BrowserOutputPump` 更新 `PassMap` store。
- 文档更新 `docs/protocol.md`。

验证：

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- 相关 Rust tests。
- CSharpier 检查 Unity C# 文件。

### Web SDK 和 Rust bridge

目标：受控网页可以声明 `data-overlay="pass"` 并推送 pass map。

交付：

- `packages/overlay` bun package。
- Rust `src/modules/overlay.rs`。
- bridge 到 typed output 的转换。
- 示例受控页面。

验证：

- bun test 或等价 bun 命令。
- Rust 编码和解析测试。
- Unity 用户实测 DOM pass 区域穿透。

### BrowserAlpha

目标：受控网页使用真实 alpha 渲染，不依赖颜色键。

交付：

- Rust `FrameAlphaMode`。
- handler/page 创建命令携带 alpha mode。
- CEF transparent background 配置。
- Unity `BrowserTransparencyMode.BrowserAlpha` 材质策略。

验证：

- 普通网站默认 opaque 不退化。
- BrowserAlpha 页面保留透明和半透明 UI。
- Color Space 为 Linear 时颜色不发白。

## 验证边界

不能启动 Unity Editor、batchmode、BuildPipeline，也不能用 Unity 生成的 `.sln/.csproj` 做编译验证。Unity 行为只能通过源码检查、CSharpier、静态一致性检查和用户侧运行反馈确认。

Rust 代码修改后必须执行：

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

涉及 release 产物时执行：

```powershell
$env:CEF_PATH = Join-Path $env:LOCALAPPDATA 'Lichora\cef\145.0.27-windows64'
.\build.ps1 -Release
```

C# 修改后执行：

```powershell
csharpier check <changed-cs-files>
```

Node/SDK 修改后只使用 bun，不使用 npm、npx 或 npm lockfile。

## 风险

- Unity UI raycast 过滤需要谨慎处理 Canvas overlay、camera 和 world space 三种模式；坐标映射必须集中在 `BrowserCoordinateMapper`。
- 拆掉 `PointableUI` 继承会触碰键盘和 IME 链路，必须先保留现有行为再引入穿透。
- output latest 与 queue 的消费边界要先定清楚，否则 pass map、script result 和 page event 会互相覆盖。
- BrowserAlpha 与当前 opaque frame 默认策略冲突，必须用显式 mode 控制。
- console bridge 如果作为过渡实现，必须限制在 Rust 内部，不要让 Unity 侧依赖 console JSON 协议。

## 下一步

下一步应先为“Unity host 边界重构”写实施计划。该计划应按文件拆分任务，先保证现有行为等价，再进入 `StaticPassRects`。不要在同一批改动里同时做 Rust IPC、网页 SDK 和 Unity 输入穿透。
