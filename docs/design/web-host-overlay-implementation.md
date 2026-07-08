# Lichora Web Host Overlay 落地设计

## 背景

`docs/design/web-host-overlay.md` 已经把 Web/Host 融合主线调整为输入归属模型。旧 `PassMap` 只能表达“默认浏览器、局部穿透给宿主”，适合 Unity 小视口场景的早期验证，但不能自然表达 Unity 全屏背景、局部 Web 面板、圆角视口和 Web 工具按钮覆盖 Host 视口等场景。

当前代码已经完成两层重构：

- Unity host 已拆出 `BrowserSurface`、`BrowserFramePump`、`BrowserPageSession`、`BrowserOutputPump`、`BrowserInputController`、`BrowserFocusController` 和 `BrowserCoordinateMapper`。
- Unity host 已建立 `InputOwnershipMap`、`InputRegion`、`InputOwnershipSettings`、`BrowserInputRouter` 和 `BrowserPointerCapture`，运行时输入查询已经以 ownership 为唯一主模型。
- Unity host 旧 `Overlay/` pass 运行时类型已删除；当前仅在 IPC 兼容边界保留 `OverlayPassMap` 名称，并在 `Ownership/Legacy/LegacyOverlayPassMapStore` 中一次性转成动态 `InputRegion`。
- Rust IPC 已有 `OutputPayloadKind::InputOwnershipMap` typed payload，旧 `OverlayPassMap` 只作为兼容 payload 保留。
- Rust runtime 已能解析 `__LICHORA_INPUT_OWNERSHIP_MAP__:`，旧 `__LICHORA_OVERLAY_PASS_MAP__:` 只在入口转换成 ownership map 后再发布到 IPC。
- `packages/overlay` 已以 ownership API 为主入口，能扫描 `data-lichora="host|web"`，并保留旧 `data-overlay="pass"`、`pass()`、`unpass()` 的兼容映射。

这些能力证明了输入链路、IPC output、Web SDK 和 Unity raycast 协作是可行的。后续不应恢复 `PassMap` 或 `Overlay/` 目录，也不应在兼容 payload 上继续叠加默认归属、圆角和覆盖规则；新能力应进入 `InputOwnershipMap` 主链路。

## 设计原则

- 输入归属是主模型，渲染透明只是视觉策略，两者必须解耦。
- `OverlayPassMap`、`data-overlay="pass"`、`pass()` 和 `unpass()` 只作为兼容过渡，不再承载新能力。
- ownership 目录和模块边界是落地前提；后续功能必须进入这些边界，不回到 overlay/pass 命名。
- 第一阶段只支持 `Rect` 和 `RoundedRect`，不实现 polygon、clip-path、mask、SVG path 或像素 alpha hit test。
- 鼠标移动热路径只查询宿主本地 map，不同步调用 JS，也不读取纹理像素。
- map 过期、非法、viewport 不匹配或 bridge 失效时默认归浏览器接收输入。
- 不要求向后兼容旧 C# 公开字段；但旧网页标记和旧 SDK API 需要保留兼容映射。
- legacy adapter 只能位于边界层。核心输入路由、命中判断和新 SDK API 不引用 `PassMap`、`PassRegion` 或 `OverlayPassMap` 命名。
- 旧格式只允许在入口转换一次。转换完成后，内部流转、查询、测试和新协议都使用 ownership 模型，不能在核心链路里反复判断“是否 pass”。
- Unity host 不再保留旧 pass 动态状态副本；旧 payload 到达后直接写入 `InputOwnershipSettings` 的动态 regions。

## 目标模型

宿主本地查询的数据结构应从 pass map 改为 ownership map：

```text
InputOwnershipMap
  version: ulong
  viewportWidth: int
  viewportHeight: int
  deviceScaleFactor: float
  enabled: bool
  defaultOwner: Web | Host
  regions: InputRegion[]

InputRegion
  id: uint
  owner: Web | Host
  shape: Rect | RoundedRect
  x: float
  y: float
  width: float
  height: float
  radius: float
  disabled: bool
```

查询规则：

- `ResolveOwner(cssX, cssY)` 返回 `Web` 或 `Host`。
- 未命中任何有效 region 时返回 `defaultOwner`。
- 命中多个 region 时以后声明或更高 z-order 的 region 为准。
- `disabled`、空尺寸、非法坐标、viewport 不匹配或过期 region 不参与命中。
- 整张 map 无效时默认返回 `Web`。

兼容映射：

```text
OverlayPassMap
  -> InputOwnershipMap(defaultOwner = Web)
  -> each pass region becomes InputRegion(owner = Host, shape = Rect)

data-overlay="pass"
  -> data-lichora="host" under defaultOwner = Web

pass(element)
  -> region(element, "host") under defaultOwner = Web
```

## DOM 声明

新公开标记：

```html
<div data-lichora="host"></div>
<div data-lichora="web"></div>
```

使用规则：

- Unity 是网页小视口时，`defaultOwner = web`，只标 Unity 视口为 `data-lichora="host"`。
- Unity 是全屏背景时，`defaultOwner = host`，只标悬浮 Web 面板为 `data-lichora="web"`。
- Web 工具按钮覆盖在 Host 视口上方时，按钮需要显式为 `web`，按钮外继续为 `host`。
- 旧 `data-overlay="pass"` 继续支持，但只按兼容规则映射为 Host region，不再扩展新语义。

第一阶段形状从 DOM 推导：

| CSS | shape |
| --- | --- |
| `border-radius: 0` | `Rect` |
| 单一稳定 `border-radius > 0` | `RoundedRect` |

四角不同半径先取最小统一半径；无法稳定表达为单一圆角矩形时降级为 `Rect`。

## 宿主适配层

`PageRenderer` 继续作为 Unity 公开入口，但 overlay 相关类型不能只做原地改名。当前已经把“输入归属”作为独立子域建立目录，旧 pass 类型收纳到 legacy adapter。

目标目录：

```text
hosts/unity-host/Scripts/
  Input/
    BrowserInputController.cs
    BrowserInputRouter.cs
    BrowserPointerCapture.cs
    BrowserCoordinateMapper.cs
    BrowserFocusController.cs
    BrowserPointerInputSource.cs
  Ownership/
    InputOwner.cs
    InputRegionShape.cs
    InputRegion.cs
    InputOwnershipMap.cs
    InputOwnershipSettings.cs
    Legacy/
      LegacyOverlayPassMapStore.cs
  Ipc/
    BrowserIpcOutputPayload.cs
    BrowserOutputPump.cs
  Rendering/
    BrowserSurface.cs
    BrowserFramePump.cs
    BrowserRenderSettings.cs
    BrowserTransparencyMode.cs
  Runtime/
    BrowserPageSession.cs
```

边界规则：

- `Input/` 只依赖 `Ownership/` 的公开查询接口，不了解 pass map 兼容格式。
- `Ownership/` 承担本地 map、region、圆角命中和过期策略。
- `Ownership/Legacy/` 是唯一允许引用旧 IPC `OverlayPassMap` payload 的 Unity 目录。旧 C# `PassMap`、`PassRegion`、`PassHitFilter` 和 `BrowserOverlaySettings` 不再作为运行时类型存在。
- `Ipc/` 只负责 typed payload decode，不做 Unity raycast 判断。
- `Rendering/` 只处理帧和材质，不参与输入归属。
- `Runtime/` 只处理页面生命周期和 IPC 句柄，不承载命中规则。

类型迁移：

| 当前类型 | 下一阶段目标 | 职责 |
| --- | --- | --- |
| `PassMap` | `InputOwnershipMap` | 已替换；保存 default owner、regions、viewport 和版本 |
| `PassRegion` | `InputRegion` | 已替换；表达 Rect 或 RoundedRect 区域及 owner |
| `PassHitFilter` | `BrowserInputRouter` + `IInputOwnershipResolver` | 已替换；统一判断当前点属于 Web 还是 Host |
| `BrowserOverlayPassMapStore` | `LegacyOverlayPassMapStore` | 已迁入 legacy 边界；消费旧 payload 并维护 ownership 动态 regions |
| `PointerHitMode.StaticPassRects` | `InputOwnershipSettings.OwnershipMap.StaticRegions` | 已替换为本地静态 ownership 配置 |
| `PointerHitMode.DomPassMap` | `InputOwnershipSettings.OwnershipMap.DynamicRegions` | 已替换为 Web SDK 推送的 ownership 动态区域 |

旧 `Overlay/` 目录已经删除。后续如果需要兼容旧语义，只能在 `Ownership/Legacy/` 或协议兼容层完成转换，不得恢复 `Overlay/PassMap.cs` 作为事实源。

命中链路：

```text
Unity pointer event
  -> BrowserPointerInputSource
  -> BrowserCoordinateMapper
  -> BrowserInputRouter.ResolveOwner
  -> BrowserPointerCapture
  -> BrowserPageSession 或宿主继续 raycast
```

pointer capture 规则保持不变：

| 事件 | 规则 |
| --- | --- |
| pointer down | 按当前 owner 锁定目标 |
| pointer move | 有锁定时沿用锁定目标 |
| pointer up | 发送给锁定目标后释放 |
| pointer cancel / runtime restart | 释放锁定 |
| wheel | 不锁定，每次按当前点判断 |

焦点规则：

- 点击 Web 区域时，浏览器获得焦点，键盘和 IME 进入浏览器。
- 点击 Host 区域时，默认释放浏览器焦点。
- `KeepBrowserFocusOnHostClick` 可以作为后续配置，但不是默认行为。

## IPC 迁移

当前 `InputOwnershipMap` 是已经落地的 typed output，`OverlayPassMap` 继续用于兼容读取和回归验证：

```text
OutputPayloadKind::InputOwnershipMap
```

payload：

```text
u64 version
i32 viewport_width
i32 viewport_height
f32 device_scale_factor
u8 enabled
u8 default_owner
u8 reserved[6]
u32 region_count
repeated InputRegion
```

`InputRegion`：

```text
u32 id
u8 owner
u8 shape
u8 disabled
u8 reserved
f32 x
f32 y
f32 width
f32 height
f32 radius
```

第一阶段枚举：

| 字段 | 值 | 含义 |
| --- | --- | --- |
| `owner` | `1` | Web |
| `owner` | `2` | Host |
| `shape` | `1` | Rect |
| `shape` | `2` | RoundedRect |

迁移规则：

- Rust 可以继续接收旧 pass map JSON，但只能在 `ownership/legacy_pass.rs` 转换一次。转换结果进入 ownership 数据结构，之后由新 payload 发布。
- Unity host 通过 `LegacyOverlayPassMapStore` 把旧 `OverlayPassMap` 直接适配成 `InputOwnershipMap(defaultOwner = Web)` 下的 Host dynamic regions。
- `OverlayPassMap` 只用于旧网页或旧 SDK。
- 宿主侧不解析网页 JSON，只消费 typed payload。

Rust 侧目标结构：

```text
src/modules/
  overlay.rs
  ownership/
    mod.rs
    legacy_pass.rs
    map_payload.rs

crates/lichora-ipc/src/
  typed_payload.rs
```

职责边界：

- `ownership/mod.rs` 只导出 ownership 入口，不暴露旧 pass 细节。
- `ownership/legacy_pass.rs` 是唯一允许理解旧 pass JSON 的 Rust 模块，只负责旧 pass JSON 到 ownership map 的转换。
- `ownership/map_payload.rs` 负责解析新 ownership console payload、限额和字段校验。
- `typed_payload.rs` 负责 wire format，不承载 DOM 或 Unity 语义。

## Web SDK 和 Bridge

`packages/overlay` 继续作为 Web SDK 包，但公开主模型改为 ownership：

```ts
setDefaultOwner(owner: "web" | "host"): void
region(element: Element, owner: "web" | "host", options?: RegionOptions): Dispose
unregion(element: Element): void
enable(): void
disable(): void
refresh(): void
```

兼容 API：

```ts
pass(element: Element, options?: PassOptions): Dispose
unpass(element: Element): void
refreshPassMap(): void
```

SDK 职责：

- 扫描 `data-lichora="host|web"`。
- 兼容扫描 `data-overlay="pass"`。
- 使用 `getBoundingClientRect()` 和 `getComputedStyle()` 生成 `Rect` 或 `RoundedRect`。
- 过滤 hidden、disabled、`display: none`、`visibility: hidden | collapse`、`opacity <= 0` 和空尺寸元素。
- 用 `ResizeObserver`、`MutationObserver`、scroll 和 resize 触发布局刷新。
- 节流输出，限制 region 数量和 payload 大小。
- 通过 bridge 发送 ownership map。

SDK 不负责：

- 不改变业务页面布局。
- 不管理业务弹窗开关状态。
- 不依赖 React、Vue 或其他 UI 框架。
- 不用像素或 alpha 采样推断命中。

bridge 路线：

- 当前 console prefix bridge 是临时方案；新 ownership prefix 已独立进入 Rust `ownership` 模块，旧 pass prefix 仍留在 Rust `overlay` 兼容入口。
- Rust `browser/dom_bridge.rs` 是网页消息到 typed output 的唯一分发入口；`DisplayHandler.on_console_message` 只作为当前 transport，后续 CEF message route 或 process message 也必须复用该入口。
- Web SDK 优先发送 typed bridge 消息：`inputOwnershipMap` 和兼容的 `overlayPassMap`；console prefix 只作为 fallback transport。
- 正式目标是 CEF message route 或 process message。
- 普通 DOM bridge 事件后续走 output queue，不和 ownership map latest state 混用。

Web SDK 目标结构：

```text
packages/overlay/src/
  index.ts
  ownership-map.ts
  region-registry.ts
  dom-scan.ts
  visibility.ts
  shape.ts
  bridge.ts
  observers.ts
  legacy-pass.ts
```

职责边界：

- `ownership-map.ts` 定义 `InputOwnershipMap`、`InputRegion`、`InputOwner` 和序列化入口。
- `region-registry.ts` 管理 API 注册的 owner regions。
- `dom-scan.ts` 扫描 `data-lichora="host|web"`，只产生候选元素。
- `visibility.ts` 只判断元素有效性，不处理形状或 bridge。
- `shape.ts` 从 DOM rect 和 CSS 推导 `Rect/RoundedRect`。
- `legacy-pass.ts` 是唯一允许扫描 `data-overlay="pass"` 和导出旧 pass API 语义的 Web SDK 模块，并把旧 API 映射到 host-owned regions。
- `bridge.ts` 只负责发送 payload，不参与 DOM 扫描和命中规则。

这样拆分后，旧 pass 链路只是输入来源之一，不会污染 ownership map 的核心模型。

## 阶段计划

### 建立目标目录边界

目标：先把 ownership 子域从旧 overlay/pass 命名里拆出来，不改变运行行为。这一步的验收不是“旧链路还能跑”而是“新核心链路已经不依赖 pass 命名”。

交付：

- Unity host 新建 `Ownership/` 目录和核心类型骨架。
- Web SDK 新建 ownership 相关模块骨架。
- Rust 新建 `src/modules/ownership/` 内部模型目录。
- 旧 pass 类型只通过 `Legacy/` 或 `legacy-pass` adapter 暴露给新模型。

验证：

- 现有 pass-map 示例行为不变。
- 旧类型迁移后没有新增协议字段。
- 文本扫描确认新核心模块不引用旧 pass 类型。
- 文本扫描只允许 legacy 目录和兼容测试引用 `PassMap|PassRegion|OverlayPassMap|pass-through`。

### 收口当前兼容链路

目标：确认当前 `OverlayPassMap` 链路只作为兼容层存在。

交付：

- 文档统一说明 `OverlayPassMap` 是过渡 payload。
- `packages/overlay` README 改为 ownership 主语。
- 示例页增加 ownership 版本，旧 pass-map 示例保留为兼容示例。

验证：

- `git diff --check`
- `bun test` in `packages/overlay`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`

### 宿主本地 ownership 模型

目标：在 Unity host 内部完成 `InputOwnershipMap` 查询和 raycast 过滤，不依赖 Web SDK 改造。

状态：源码实现已完成，等待 Unity 内实测回归。

交付：

- 已新增 `InputOwnershipMap`、`InputRegion`、`InputOwner`、`InputRegionShape` 和 `InputOwnershipSettings`。
- `BrowserInputRouter` 已改为 `ResolveOwner`，`BrowserInputController` 只依赖 `IInputOwnershipResolver`。
- `PageRenderer` 的公开输入配置源已改为 `InputOwnershipSettings`。
- 旧 `OverlayPassMap` 通过 `Ownership/Legacy/LegacyOverlayPassMapStore` 直接映射到 ownership dynamic regions。
- 旧 `Overlay/` C# pass 类型已经删除。

验证：

- `defaultOwner = Web`，Host 小视口可穿透。
- `defaultOwner = Host`，Web 面板可接收输入。
- pointer capture 在 Web/Host 两种归属下稳定。
- RoundedRect 内外命中符合预期。

### IPC ownership payload

目标：增加结构化 ownership payload，并保留旧 `OverlayPassMap` 兼容读取。

状态：源码实现已完成，等待 Unity 内实测回归和正式 CEF message bridge 替换 console bridge。

交付：

- Rust IPC typed payload 增加 `InputOwnershipMap`。
- Rust golden tests 覆盖 encode/decode。
- Unity decoder 增加 ownership payload。
- `BrowserInputOwnershipStore` 优先消费 ownership payload，旧 pass payload 走 adapter。
- `docs/protocol.md` 增加新 payload 字段说明。

验证：

- 旧 pass payload 仍能驱动 Host 区域。
- 新 ownership payload 能覆盖 `defaultOwner = Host` 场景。
- 非法 payload 被丢弃，并默认 Web 输入。
- 核心 Rust ownership 模块不引用 `OverlayPassMapOutput`，旧 payload 只在 legacy adapter 中出现。

### Web SDK ownership API

目标：Web 端直接生成 ownership map。

状态：源码实现已完成，等待接入真实业务页面实测。

交付：

- `data-lichora="host|web"` 扫描。
- `setDefaultOwner()`、`region()`、`unregion()`、`refresh()`。
- `data-overlay="pass"` 与旧 API 兼容映射。
- `Rect/RoundedRect` 推导和测试。
- 小视口示例页和全屏背景示例页。

验证：

- `bun test`
- 小视口：默认 Web，视口 Host，视口上方按钮 Web。
- 全屏背景：默认 Host，半透明面板 Web。
- hidden、disabled、opacity、空尺寸过滤正确。

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
- Linear Color Space 下颜色不发白。

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

- 旧 pass 命名仍存在于代码中，迁移时需要 adapter，不能一边改协议一边改 SDK 导致回归定位困难。
- Unity UI raycast 过滤需要谨慎处理 Canvas overlay、camera 和 world space 三种模式；坐标映射必须集中在 `BrowserCoordinateMapper`。
- RoundedRect 只适合基础 div 形状，不能被误解为支持 clip-path 或 mask。
- output latest 与 queue 的消费边界要保持清晰，ownership map 是 latest state，普通 DOM bridge 事件是 queue。
- BrowserAlpha 与当前 opaque frame 默认策略冲突，必须用显式 mode 控制。

## 下一步

当前主链路已经从本地模型推进到 Rust IPC 和 Web SDK。下一阶段不再继续扩展旧 pass 语义，按下面顺序收口：

- 先由 Unity 实测确认 ownership bridge：小视口 `defaultOwner = web`、全屏背景 `defaultOwner = host`、圆角区域、按钮覆盖视口、弹窗部分遮挡和页面隐藏状态。
- 再把 console prefix bridge 替换为正式 CEF message route 或 process message，明确 latest ownership state 与普通 output queue 的边界。
- 正式 bridge 落地时先接 CEF browser/render process message transport，再移除 Web SDK 对 console prefix 的默认依赖；不要把新的 transport 解析逻辑写回 `DisplayHandler`。
- 然后推进 BrowserAlpha，用显式透明模式解决半透明 Web 面板和 Linear Color Space 的视觉问题。
- 最后设置兼容窗口；旧 `OverlayPassMap`、`data-overlay="pass"` 和旧 pass API 在无业务依赖后再删除。
