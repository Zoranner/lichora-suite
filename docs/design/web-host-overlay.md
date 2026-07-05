# Lichora Web 与宿主场景融合设计

落地重构计划见 `web-host-overlay-implementation.md`。该计划结合当前 Unity host 和 Rust IPC 代码状态，明确先重构宿主边界，再实现 `StaticPassRects`、`OverlayPassMap`、网页端 SDK 和 `BrowserAlpha`。

## 定位

本文设计 `Lichora` 的 Web/Host 融合能力：网页作为宿主引擎场景上的 UI 层使用，部分区域可以显示为透明，并在受控网页模式下允许鼠标穿透到宿主场景。

当前首个宿主适配是 Unity，因此本文会在实现细节里提到 Unity 的 `RawImage`、`EventSystem` 和 raycast；但协议、网页端 SDK、pass map 和输入路由模型不绑定 Unity，后续可以接入其他引擎或原生渲染宿主。

该能力不是单一透明开关，而是由两条链路共同组成：

- 渲染透明链路：浏览器画面如何与宿主场景合成。
- 输入命中链路：某个屏幕坐标的鼠标、滚轮和焦点应该归浏览器还是宿主。

现有 `BrowserRender.shader` 已提供颜色过滤透明效果，但它只改变显示结果，不改变宿主 UI/raycast 和浏览器输入归属。新设计必须把显示透明和输入穿透解耦，避免继续把交互规则塞进颜色过滤逻辑。

## 目标

- 支持任意网站的颜色键透明显示，保持现有低门槛用法。
- 支持受控网页声明 pass-through 区域，让宿主场景在这些区域接收鼠标交互。
- 支持网页弹窗、菜单、下拉框、拖拽层等临时遮挡关系。
- 鼠标移动热路径不依赖每帧跨进程或 JS 往返查询。
- 宿主侧输入路由、浏览器渲染、网页端 DOM 命中规则边界清晰。
- 与当前 Rust IPC v2 主链路一致，不恢复旧 MemoryStacks 或 .NET 协议模型。
- 为后续真 alpha、任意形状区域、调试可视化和多页面叠加保留扩展位置。

## 非目标

- 不让颜色过滤透明自动等同于输入穿透。
- 不支持任意第三方网站自动推断可穿透区域作为强保证。
- 不在每次 `MouseMove` 时同步调用 JS 判断命中结果。
- 不用像素颜色或 alpha 采样作为受控网页模式的主命中依据。
- 不把网页端 SDK 做成 UI 框架，也不接管业务组件状态。
- 不要求第一版支持 iframe、Shadow DOM closed root、跨域子页面的完整命中语义。

## 模式

### 当前基础

当前 Unity 适配层使用 `PageRenderer + RawImage` 承载浏览器画面。`PageRenderer` 创建 `Texture2D(BGRA32)`，每帧从 IPC frame ring 拷贝最新帧并执行 `Texture2D.Apply(false)`。鼠标事件从 Unity `EventSystem` 进入 `PointableUI`，映射为浏览器归一化坐标后交给 `PageHandler`，再写入 IPC input。

当前 shader 已有 `Blend SrcAlpha OneMinusSrcAlpha` 和颜色过滤逻辑，但场景里的 `RawImage.raycastTarget` 仍是整块启用。也就是说，shader 把某些像素画成透明，只影响最终画面，不影响宿主 UI/raycast；透明区域仍会挡住宿主场景输入。

Rust/IPC 侧当前已经有 `session`、`control`、`status`、`input`、`frame`、`output` 六类通道。输入路径里鼠标移动是 latest-only，点击、滚轮、键盘、IME 和脚本请求是 queue。输出 typed payload 现有 `Caret`、`SurroundingText`、`ScriptResult`、`PageEvent`，其中 caret 和 surrounding text 已在运行链路中使用，脚本结果和正式 DOM bridge 还不是完整能力。

### 任意网站透明显示模式

该模式面向第三方网站或不可控页面，目标是视觉融合，不承诺输入穿透。

配置建议：

```text
RenderTransparencyMode = ColorKey
PointerHitMode = FullBrowserSurface
```

渲染侧使用颜色键过滤。现有 `FilteredColor` 应升级为明确的 `ColorKeySettings`：

| 字段 | 含义 |
| --- | --- |
| `enabled` | 是否启用颜色键透明 |
| `color` | 被过滤的目标颜色 |
| `threshold` | 颜色距离阈值 |
| `softness` | 边缘过渡强度 |
| `platformPolicy` | 平台策略，例如 Windows shader、Linux fallback |

输入侧仍认为整个浏览器 RawImage 是浏览器表面：

- 鼠标移动、点击、滚轮全部发给浏览器。
- 宿主 UI/raycast 不穿透浏览器控件。
- 页面视觉上透明的位置也不会自动把事件交给宿主。

这个模式的价值是兼容范围广、接入成本低；边界是交互不透明。文档和 Inspector 文案必须明确这个边界，避免用户误以为透明像素必然穿透。

如果后续确实需要“按颜色键或 alpha 做输入穿透”，应作为实验性的 `PixelHitTest` 策略，而不是任意网站模式的默认行为。该策略只能用于明确知道网页背景色和内容颜色不会冲突的页面，并且必须避免每次 raycast 大范围读取纹理数据。

### 受控网页融合模式

该模式面向用户自己开发的网页 UI。网页端显式声明哪些区域允许穿透，宿主适配层根据缓存的 pass map 在本地路由输入。

配置建议：

```text
RenderTransparencyMode = ColorKey | BrowserAlpha
PointerHitMode = DomPassMap
```

网页声明示例：

```html
<header class="topbar">...</header>
<aside class="left-panel">...</aside>
<main data-overlay="pass"></main>
<aside class="right-panel">...</aside>
```

核心规则：

- 未声明区域默认由浏览器接收，避免误穿透。
- 显式 pass-through 区域默认交给宿主。
- 浏览器交互元素、弹窗、菜单、下拉框和拖拽层默认仍由浏览器接收；公开 HTML 标记只声明需要穿透给宿主的区域。
- 鼠标按下时建立 pointer capture，按下归谁，拖动过程就归谁，直到按钮释放。
- 键盘和 IME 焦点只在浏览器获得焦点后进入浏览器；宿主区域点击应释放或保持浏览器焦点由配置决定。

## 组件边界

### 宿主适配层

现有 `PageRenderer` 仍应保持主要公开入口，但内部职责应拆开：

| 组件 | 职责 |
| --- | --- |
| `BrowserSurface` | 管理 RawImage、材质、纹理、透明渲染模式 |
| `BrowserInputRouter` | 根据 pass map 决定输入是否穿透 |
| `PassMap` | 保存网页端同步来的 pass 区域和版本 |
| `PassHitFilter` | 对宿主 UI/raycast 返回浏览器阻挡或穿透 |
| `BrowserPointerCapture` | 管理按下、拖动、释放期间的归属锁定 |
| `BrowserPageSession` | 继续作为页面 IPC facade，不承载 UI 命中规则 |

`PointableUI` 当前同时处理坐标映射、鼠标状态、键盘、IME 和焦点。后续改造时应把鼠标命中决策前移：

```text
Host pointer system
  -> PassHitFilter.IsHitValid
  -> BrowserInputRouter.ResolvePass
  -> BrowserPointerCapture
  -> BrowserPageSession or host scene
```

如果当前浏览器表面覆盖全屏，必须通过宿主命中过滤在 pass-through 点返回 false，否则宿主场景永远收不到事件。仅仅不发送浏览器鼠标事件不够，因为宿主 UI 层仍会挡住底层场景。

### Rust 浏览器进程

Rust 侧继续保持浏览器进程和 IPC 适配职责：

- 注入或加载 overlay bridge 脚本。
- 接收网页端 pass map 更新。
- 将 pass map 作为 typed output payload 推送给宿主适配层。
- 在 status 中统计 pass map 更新次数、丢弃次数、解析错误和版本号。
- 不在 Rust 侧参与宿主 raycast 决策。

Rust 侧不应该在每次鼠标移动时执行 DOM 查询。高频输入路径仍保持 latest-only mouse state；pass map 是低频布局状态。

### 网页端 SDK

受控网页模式需要一个轻量网页端库，建议命名为 `@lichora/overlay`。它不是 UI 框架，只负责描述命中语义。

推荐 API：

```ts
pass(element: Element, options?: PassOptions): Dispose
unpass(element: Element): void
enable(): void
disable(): void
refreshPassMap(): void
```

同时支持 HTML 属性：

```html
<div data-overlay="pass"></div>
```

属性命名约定：

| 属性 | 含义 |
| --- | --- |
| `data-overlay="pass"` | 默认穿透给宿主 |

公开 HTML 属性只保留 `pass`。未声明区域默认浏览器接收，不需要业务页面给按钮、标题栏、侧栏、弹窗或菜单再标记 `hit`。

SDK 职责：

- 扫描带声明属性的 DOM 节点。
- 使用 `ResizeObserver`、`MutationObserver`、scroll/resize 监听布局变化。
- 生成 pass map，包含穿透区域、版本和 viewport 信息。
- 识别 pass 区域内当前被普通 DOM 顶层元素覆盖的部分，并让覆盖物默认回到浏览器接收。
- 在必要时使用 `document.elementFromPoint` 做调试或兜底校验。
- 通过 bridge 将 pass map 发给浏览器进程。
- 对输出频率做节流，避免布局抖动把 IPC output 压满。
- 限制 region 数量和 payload 大小，超过限制时降级为默认浏览器接收。

SDK 不负责：

- 不改变业务页面布局。
- 不替业务组件管理弹窗开关状态。
- 不处理宿主场景交互逻辑。
- 不依赖 React、Vue 或其他前端框架。

SDK 注入和生命周期需要协议化。受控网页可以主动引入 SDK；如果由浏览器进程注入，则应在主 frame 创建、导航完成和 SPA history 变化后保证 bridge 可用。iframe 和 Shadow DOM 第一版不做完整保证，后续按实际页面约束扩展。

## Pass Map 数据模型

宿主侧需要的是稳定、低频更新、可本地查询的数据。

建议数据结构：

```text
PassMap
  version: u64
  viewportWidth: i32
  viewportHeight: i32
  deviceScaleFactor: f32
  enabled: bool
  regions: PassRegion[]

PassRegion
  id: u32
  shape: Rect
  x: f32
  y: f32
  width: f32
  height: f32
  disabled: bool
```

第一版建议只支持矩形区域，原因是：

- DOM `getBoundingClientRect()` 直接输出矩形。
- 宿主本地查询成本低。
- 标题栏、侧边栏围出的中间三维视口、普通弹窗遮挡等主场景都能表达。

后续如确实需要复杂形状，可以扩展 polygon 或 alpha mask，但不要作为第一版主路径。

## 命中规则

每次指针事件先做坐标转换：

```text
screen position
  -> browser RawImage local position
  -> normalized browser position
  -> CSS viewport position
  -> PassMap.Contains(cssX, cssY)
```

命中决策：

- 如果 pass 已全局禁用，事件发给浏览器。
- 如果已有 pointer capture，直接使用 capture target。
- 如果坐标不在 pass region 内，事件发给浏览器。
- 如果坐标在 pass region 内，但当前点被普通 DOM 覆盖物占用，事件发给浏览器。
- 如果坐标在 pass region 内，且没有浏览器覆盖物占用，宿主 UI/raycast 继续向下命中，事件不发给浏览器。
- pass map 过期或不可用时默认浏览器接收，避免网页 UI 失控。

pointer capture 规则：

| 事件 | 规则 |
| --- | --- |
| pointer down | 按当前命中结果锁定 target |
| pointer move | 有锁定时沿用锁定 target |
| pointer up | 发送给锁定 target 后释放 |
| pointer cancel / browser restart | 释放锁定 |
| wheel | 默认不捕获，每次按当前位置命中 |

拖动宿主相机或场景对象时，如果 down 在 Host 区域，后续 move 即使经过网页侧栏也不被浏览器抢走。拖动网页滑块时，如果 down 在浏览器区域，后续 move 即使经过 pass-through 中心区也继续给浏览器。

## 弹窗遮挡

声明 pass-through 不应等于永久穿透。最终命中结果必须尊重当前页面的最上层遮挡。

公开 HTML 只声明 `data-overlay="pass"`。弹窗、菜单、下拉框、tooltip、拖拽层和其他普通 DOM 覆盖物不需要额外声明。SDK 生成 pass map 时应按当前 DOM 布局和可见性计算 pass region 的有效穿透范围：被普通浏览器 UI 覆盖的部分不穿透。

示例：中间区域声明 `data-overlay="pass"`，弹窗覆盖到中间区域：

- 鼠标在弹窗按钮上：浏览器处理。
- 鼠标在弹窗内容上：浏览器处理。
- 鼠标在 backdrop 上：默认浏览器处理，用于关闭弹窗；如果业务确实希望遮罩也穿透，应把 backdrop 自身放进 pass 区域或通过 JS API 更新 pass。
- 弹窗关闭后：中间区域恢复穿透宿主。

网页端 SDK 应把这类遮挡关系折算进 pass region 的有效区域。宿主适配层只做本地区域判断，不需要理解 DOM 树或弹窗语义。

## 渲染透明策略

### ColorKey

当前 shader 颜色过滤应保留并整理为正式策略。它适合任意网站和快速接入，但存在天然限制：

- 抗锯齿和阴影边缘可能有残留。
- 半透明 UI、视频、canvas 和 WebGL 可能被误过滤。
- 颜色透明不代表输入穿透。
- Linux 当前 shader 路径需要单独平台策略。

### BrowserAlpha

受控网页模式的理想路径是真 alpha：

- CEF OSR 背景允许透明。
- BGRA frame 保留 alpha。
- IPC frame ring 不丢弃 alpha。
- 宿主 texture 和材质使用 alpha blend。
- 页面 CSS 明确透明背景。

该模式能更自然支持圆角、阴影、半透明面板和动效。它应作为受控网页融合模式的目标渲染策略，但不阻塞第一版输入穿透；第一版可以先使用 ColorKey 加 DomPassMap。

## IPC 设计

现有 IPC v2 已有 `output` typed payload，可承载页面事件。建议新增明确 payload，而不是把 pass map 塞进通用脚本结果字符串里。

输出通道需要区分 latest state 和 event queue：

- `OverlayPassMap` 是状态，适合 latest。宿主适配层只需要最新版本，旧布局可以直接丢弃。
- DOM bridge 普通事件、请求响应和 SDK 诊断适合 queue。宿主适配层如果需要消费这些事件，必须补齐 output queue 的读取 API，不能只读 latest。
- `ScriptResult` 类型已存在，但当前脚本执行链路不能被当成可靠 request/response bridge；正式 SDK 不应依赖“发脚本后必有返回”这个假设。

新增 latest 输出类型：

```text
OutputPayloadKind::OverlayPassMap = 5
```

payload 建议：

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

第一版 `PassRegion`：

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

第一版 region 全部表示 pass-through 区域，因此不需要公开 `target` 和 `priority`。默认规则始终是：不在有效 pass region 内就由浏览器接收。后续如果需要多 target 或复杂优先级，可以通过 payload minor version 扩展。

如果为了快速迭代，也可以先让网页端 SDK 通过 console 或 script bridge 输出 JSON，再由 Rust 解析后转换成 typed payload。但宿主适配层不应该长期解析业务 JSON；宿主侧应读取结构化 `OverlayPassMap`。

如需要网页端发普通事件，应新增 queue 输出类型：

```text
OutputPayloadKind::DomBridgeEvent = 6
```

事件 payload 至少包含：

```text
u64 sequence
u32 bridge_version
u32 event_type
u32 payload_bytes
bytes payload_json_utf8
```

该事件用于低频控制和诊断，不用于鼠标移动命中查询。

状态诊断新增 counters：

- `overlay_pass_map_version`
- `overlay_pass_map_updates`
- `overlay_pass_map_dropped`
- `overlay_pass_map_parse_errors`
- `overlay_region_count`
- `overlay_last_update_age_ms`

## JS Bridge

网页端 SDK 到浏览器进程需要稳定 bridge。可选路径：

- 优先：CEF message router 或 process message，作为正式 bridge。
- 过渡：约定 console 前缀，例如 `__LICHORA_OVERLAY_PASS_MAP__:`，由 display handler 捕获。
- 兜底：宿主适配层主动 `ExecuteScript` 请求刷新，但不能用于高频鼠标查询。

正式设计应避免把 console 作为长期协议。console 可以用于 caret/surrounding text 这类低风险内部探针，但 overlay pass map 是用户可见能力，应逐步转到明确 bridge。

bridge 必须做输入约束：

- 校验 bridge 版本和 schema。
- 限制单条消息大小和单位时间消息数。
- 限制 hit region 数量。
- 对异常 JSON、未知字段和非法坐标计数并丢弃。
- 可配置允许的 origin 或本地页面范围，避免任意页面滥用 overlay bridge。
- bridge 失效、消息非法或 pass 全局禁用时，宿主侧默认浏览器接收输入。

## 坐标和缩放

坐标链路必须显式记录：

- host screen position。
- Canvas render mode 和 camera。
- RawImage `RectTransform` local rect。
- Browser texture width/height。
- CSS viewport width/height。
- device scale factor。
- 页面 scroll offset 对 `getBoundingClientRect()` 的影响。

网页端 pass map 使用 CSS viewport 坐标，即 `getBoundingClientRect()` 结果。宿主适配层将指针映射到浏览器 CSS viewport 后查询 region。这样可避免页面滚动后还要把文档坐标换算回来。

resize 时必须清空或标记旧 pass map 过期，直到新 viewport 尺寸的 pass map 到达。过期期间默认浏览器接收输入。

## 焦点、键盘和 IME

输入穿透主要针对鼠标和滚轮。键盘与 IME 需要按焦点处理：

- 点击浏览器区域：浏览器获得焦点，启用现有键盘和 IME 链路。
- 点击 Host pass-through 区域：默认释放浏览器焦点，宿主接管键盘。
- 可配置 `KeepBrowserFocusOnHostClick`，用于网页输入框需要保持文本焦点但鼠标操作宿主场景的高级场景。
- 当 pointer capture target 为 Host 时，不应向浏览器发送鼠标按钮事件。
- 当浏览器失焦时，应取消 IME composition 或按现有 IME 状态机安全收口。

## 配置入口

`PageRenderer` Inspector 建议新增分组：

```text
Rendering
  Transparency Mode: None | ColorKey | BrowserAlpha
  Color Key Settings

Input
  Pointer Hit Mode: FullBrowserSurface | StaticPassRects | DomPassMap
  Keep Browser Focus On Host Click
  Debug Hit Regions
```

`StaticPassRects` 是不依赖网页 SDK 的中间模式：用户在宿主适配层配置固定 pass 矩形区域。它适合快速验证输入路由，也能作为 DOM pass map 不可用时的调试手段，但不是最终受控网页模式的主路径。

## 工程结构调整

建议在后续实现中把相关代码整理到独立目录：

```text
hosts/unity-host/Scripts/
  Rendering/
    BrowserSurface.cs
    BrowserTransparencySettings.cs
  Input/
    BrowserInputRouter.cs
    BrowserPointerCapture.cs
  Overlay/
    PassMap.cs
    PassRegion.cs
    PassHitFilter.cs
    BrowserOverlaySettings.cs
  Ipc/
    BrowserIpcOutputPayload.cs
```

网页端 SDK 建议放在独立包目录：

```text
packages/overlay/
  package.json
  src/
    index.ts
    pass-map.ts
    bridge.ts
    observers.ts
  dist/
    lichora-overlay.js
```

如果暂时不引入 Node 构建，也可以先把纯 JS 版本放到 package 的 `Resources` 或 sample 中，但正式发版前应形成独立可版本化的网页端库。

Rust 侧建议新增：

```text
src/modules/overlay.rs
crates/ipc/src/typed_payload.rs
```

`overlay.rs` 只处理 bridge 消息、JSON 到结构化 pass map 的转换和 output publish，不参与宿主输入决策。

## 验证策略

源码侧验证：

- Rust IPC typed payload golden test 覆盖 `OverlayPassMap` 编解码。
- 当前 Unity 适配层对 `PassMap.Contains` 做纯 C# 单元级逻辑验证。
- 坐标转换函数用固定 RectTransform 数据做静态测试或可调用 helper 测试。
- CSharpier 检查所有改动 C# 文件。
- Rust 执行 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。

用户实测场景：

- 任意网站颜色键透明，鼠标仍操作网页。
- 受控网页顶部栏、左右侧栏操作网页，中间区域拖动宿主摄像机。
- 中间 pass-through 区域弹出 modal，modal 覆盖区域浏览器可点击。
- 拖动宿主相机时经过侧栏不被浏览器抢输入。
- 拖动网页滑块时经过中间区域不穿透到宿主。
- resize、DPI scale、Canvas overlay/camera 模式下坐标一致。

性能验收：

- 鼠标移动不产生 IPC 事件积压。
- pass map 只在布局变化时更新。
- 宿主命中判断无 per-frame GC。
- Debug hit region 可关闭，关闭后不产生额外绘制和日志。

## 实施顺序

建议按可验证链路推进：

- 整理 `PageRenderer` 的透明配置，把 `FilteredColor` 升级为 `RenderTransparencyMode` 和 `ColorKeySettings`。
- 增加宿主本地 `StaticPassRects`，验证 hit filter、input router 和 pointer capture。
- 增加 `PassMap` 数据模型和本地 contains 逻辑。
- 扩展 IPC typed output，加入 `OverlayPassMap` 编解码和宿主侧 decoder。
- 增加网页端 `@lichora/overlay` SDK，先支持 `data-overlay="pass"` 和普通 DOM 覆盖物遮挡计算。
- Rust 注入/bridge pass map，并通过 output 推送宿主适配层。
- 增加 BrowserAlpha 渲染策略，替代受控网页模式下的颜色键依赖。

这样做可以先把宿主输入路由根基打稳，再接网页端 SDK，最后优化真实 alpha。每一步都有独立验收点，不需要一次性把所有复杂度压到浏览器端。
