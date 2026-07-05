# Lichora Web 与宿主场景融合设计

落地计划见 `web-host-overlay-implementation.md`。本文描述产品语义和系统边界，实施文档负责拆解当前代码如何迁移。

## 定位

Lichora 的 Web/Host 融合能力由两条链路组成：

- 渲染融合：浏览器画面如何透明、半透明或不透明地叠加在宿主场景上。
- 输入归属：某个屏幕坐标的鼠标、滚轮、焦点和拖动应该归浏览器还是宿主。

渲染透明和输入归属必须解耦。网页区域视觉上透明，不等于事件自动交给宿主；网页区域视觉上半透明，也不等于一定由浏览器处理。输入归属应由受控网页和宿主适配层共同维护的低频命中数据决定。

当前首个宿主是 Unity，因此文档会提到 `RawImage`、`EventSystem` 和 raycast。但输入归属模型不绑定 Unity，后续可以接入其他宿主。

## 主要场景

### Unity 作为网页中的视口

网页是主界面，Unity 只是页面中的一个视口区域。例如中间是地球、地图或三维模型，视口侧边可能悬浮放大、缩小、标绘、测量等 Web 工具按钮。

该场景的默认归属应是浏览器：

```text
defaultOwner = web
```

只需要标记 Unity 视口：

```html
<div class="earth-view" data-lichora="host"></div>
```

普通 Web 面板、工具栏和按钮默认属于浏览器，不需要标记。若某个工具按钮放在视口 DOM 内或覆盖在视口上方，才需要显式标成浏览器：

```html
<button class="zoom-in" data-lichora="web">+</button>
```

第一阶段只支持 `div` 能自然表达的基础形状：矩形和圆角矩形。圆角从 CSS `border-radius` 推导。平行四边形、多边形、`clip-path`、SVG path 和 mask 不进入下一阶段。

### Unity 作为整个窗口背景

Unity 三维场景铺满窗口，网页面板半透明悬浮在场景之上。该场景的默认归属应是宿主：

```text
defaultOwner = host
```

只需要标记 Web 面板：

```html
<section class="tool-panel" data-lichora="web"></section>
<section class="property-panel" data-lichora="web"></section>
```

大面积 Unity 背景不需要声明区域，避免把背景拆成大量 pass rect。半透明 Web 面板仍由浏览器接收输入；面板之外的区域由宿主接收输入。

## 目标

- 支持网页内嵌 Unity 视口和 Unity 全屏背景两类主场景。
- 用统一输入归属模型表达浏览器和宿主的输入边界。
- 第一阶段支持矩形和圆角矩形，不支持任意多边形或像素级命中。
- 支持 Web 弹窗、菜单、下拉层、工具按钮对宿主区域的覆盖关系。
- 鼠标移动热路径不依赖 JS 往返，也不读取纹理像素。
- 失败、过期或非法数据默认归浏览器接收输入，避免网页 UI 失控。
- 与当前 Rust IPC v2 typed output 主链路一致，不恢复旧 MemoryStacks 或 .NET 协议模型。
- 为 BrowserAlpha、正式 DOM bridge、调试可视化和多页面叠加保留演进位置。

## 非目标

- 不让颜色键透明自动等同于输入穿透。
- 不支持第三方任意网站自动推断输入归属作为强保证。
- 不在每次 `MouseMove` 时同步查询 DOM。
- 不用像素颜色、alpha 或纹理采样作为下一阶段主命中依据。
- 不支持 `clip-path`、mask、SVG path、任意多边形、平行四边形窗口。
- 不要求第一阶段完整支持 iframe、closed Shadow DOM、跨域子页面和复杂 CSS stacking context。
- 不把网页端 SDK 做成 UI 框架，也不接管业务组件状态。

## 输入归属模型

旧的 `PassMap` 只表达“哪些区域穿透到宿主”。这对 Unity 小视口场景可用，但对 Unity 全屏背景场景不自然。下一阶段应升级为输入归属模型：

```text
InputOwnershipMap
  version: u64
  viewportWidth: i32
  viewportHeight: i32
  deviceScaleFactor: f32
  enabled: bool
  defaultOwner: Web | Host
  regions: InputRegion[]

InputRegion
  id: u32
  owner: Web | Host
  shape: Rect | RoundedRect
  x: f32
  y: f32
  width: f32
  height: f32
  radius: f32
  disabled: bool
```

命中查询：

```text
ResolveOwner(cssX, cssY) -> Web | Host
```

规则：

- 未命中任何有效 region 时返回 `defaultOwner`。
- 命中多个 region 时以后声明或更高 z-order 的 region 为准。
- region 与 viewport 尺寸不匹配、过期、非法时忽略。
- 整张 map 无效时默认浏览器接收输入。

当前 `OverlayPassMap` 可以作为过渡实现继续存在，但文档和代码命名应逐步迁移到 ownership 语义。兼容层可以把 `data-overlay="pass"` 转成 `defaultOwner=web` 下的 `owner=host` region。

## DOM 标记

HTML 属性使用产品名，但保持短：

```html
<div data-lichora="host"></div>
<div data-lichora="web"></div>
```

语义：

| 标记 | 含义 |
| --- | --- |
| `data-lichora="host"` | 该区域由宿主接收输入 |
| `data-lichora="web"` | 该区域由浏览器接收输入 |
| 不写 | 使用当前 map 的 `defaultOwner` |

形状默认从 CSS 推导：

| CSS | shape |
| --- | --- |
| `border-radius: 0` | `Rect` |
| `border-radius > 0` | `RoundedRect` |

第一阶段只读取可稳定表达为单一圆角矩形的 `border-radius`。复杂的四角不同半径可以先降级为矩形，或取最小统一半径；具体策略在实施文档中固定。

## 可见性和覆盖

可见性处理分三层。

### 元素有效性

生成 region 前先过滤不可用元素：

- `hidden`
- `disabled`
- `display: none`
- `visibility: hidden | collapse`
- `opacity <= 0`
- `width <= 0` 或 `height <= 0`

这只判断元素自身是否应生成区域，不等价于完整 CSS 可见性系统。

### 基础形状

第一阶段只生成 `Rect` 和 `RoundedRect`。视觉上超出这两类的 CSS 效果不参与精确命中：

- `clip-path` 不支持。
- mask 不支持。
- transform 后的复杂真实形状不支持。
- box-shadow 不扩大命中区域。
- 半透明像素不按 alpha 判断。

### 上层覆盖

同一张 ownership map 内，覆盖关系应按输入归属处理，而不是只按 pass-through 处理：

- 自身、后代和祖先不算覆盖物。
- 同归属的上层元素不改变归属。
- 相反归属的上层元素覆盖当前区域时，以更上层元素的归属为准。

对 Unity 小视口场景，Web 工具按钮覆盖在 host 视口上时，按钮区域应解析为 Web。对 Unity 全屏背景场景，Web 面板自然解析为 Web，面板外解析为 Host。

## 渲染融合

### Opaque

默认模式。浏览器帧按不透明 BGRA 发布，Unity 作为普通 UI 图像显示。适合普通网页嵌入，不提供视觉融合。

### ColorKey

颜色键透明适合快速演示和任意网页的低门槛接入。它只影响画面，不决定输入归属。

限制：

- 抗锯齿和阴影边缘可能残留。
- 半透明 UI、视频、canvas 和 WebGL 可能被误过滤。
- 需要明确 filter color、threshold 和 softness。
- 不应用作受控网页长期主方案。

### BrowserAlpha

受控网页融合的目标渲染策略是真 alpha：

- CEF OSR 使用透明背景。
- BGRA frame 保留 alpha。
- IPC frame ring 不丢弃 alpha。
- Unity 材质使用 alpha blend。
- 页面 CSS 明确透明背景。

BrowserAlpha 应与输入归属模型独立开关。半透明 Web 面板可以视觉上透出 Unity，但输入仍归 Web；透明 host 视口可以显示 Unity 并把输入归 Host。

## 宿主适配层

`PageRenderer` 保留为 Unity 公开入口，但内部职责应拆开：

| 组件 | 职责 |
| --- | --- |
| `BrowserSurface` | 管理 RawImage、材质、纹理、透明模式 |
| `BrowserFramePump` | 从 IPC frame reader 拷贝最新帧并更新纹理 |
| `BrowserInputRouter` | 根据 ownership map 和 pointer capture 决定事件归属 |
| `InputOwnershipMap` | 保存网页端同步来的输入归属区域 |
| `InputRegion` | 表达 Rect 或 RoundedRect 区域 |
| `OwnershipHitFilter` | 与 Unity raycast 协作，在 Host 区域让宿主继续命中底层对象 |
| `BrowserPointerCapture` | 管理 pointer down 到 up/cancel 期间的归属锁定 |
| `BrowserPageSession` | 管理页面 id、尺寸、IPC reader/writer 和 restart 重连 |

命中链路：

```text
Unity pointer system
  -> BrowserCoordinateMapper
  -> BrowserInputRouter.ResolveOwner
  -> BrowserPointerCapture
  -> BrowserPageSession or host scene
```

仅仅“不向浏览器发送鼠标事件”不够。浏览器 RawImage 覆盖全屏时，Unity UI raycast 必须在 Host 区域返回 false，让底层宿主对象继续被命中。

## Rust 浏览器进程

Rust 侧负责 bridge 和 IPC，不参与宿主 raycast 决策：

- 接收网页端 ownership map 更新。
- 校验版本、大小、region 数量和坐标范围。
- 转换为 typed output payload。
- 推送给宿主适配层。
- 在 status 中统计更新次数、丢弃次数、解析错误和版本号。

Rust 侧不在 mouse move 时执行 DOM 查询。高频输入路径继续保持 latest-only mouse state；ownership map 是低频布局状态。

## 网页端 SDK

SDK 仍建议命名为 `@lichora/overlay`，但职责从 pass map 升级为 input ownership 描述。

推荐 API：

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

兼容 API 等价于 `defaultOwner = web` 下注册 `owner = host` 的矩形或圆角矩形区域。

SDK 职责：

- 扫描 `data-lichora="host|web"`。
- 兼容扫描 `data-overlay="pass"`。
- 使用 `getBoundingClientRect()` 和 `getComputedStyle()` 生成 Rect/RoundedRect。
- 使用 `ResizeObserver`、`MutationObserver`、scroll 和 resize 监听布局变化。
- 基于基础可见性过滤无效元素。
- 按基础覆盖关系计算最终 region。
- 通过 bridge 发送 ownership map。
- 节流输出，限制 region 数量和 payload 大小。

SDK 不负责：

- 不改变业务页面布局。
- 不管理业务弹窗开关状态。
- 不依赖 React、Vue 或其他 UI 框架。
- 不用像素或 alpha 采样推断命中。

## IPC 设计

当前 `OverlayPassMap` 是过渡 payload。下一阶段建议新增或升级为：

```text
OutputPayloadKind::InputOwnershipMap
```

payload 字段：

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

`shape` 第一阶段：

| 值 | 含义 |
| --- | --- |
| `1` | Rect |
| `2` | RoundedRect |

宿主适配层只消费 typed payload，不解析网页业务 JSON。console 前缀 bridge 可以继续作为过渡，但必须封装在 Rust `overlay` 模块内部。

## 坐标和缩放

网页端使用 CSS viewport 坐标，即 `getBoundingClientRect()` 的结果。宿主适配层把 Unity pointer 坐标转换到同一 CSS viewport 坐标后查询 ownership map。

需要显式记录并验证：

- host screen position
- Canvas render mode 和 camera
- RawImage `RectTransform`
- browser texture width/height
- CSS viewport width/height
- device scale factor
- 页面 scroll 对 `getBoundingClientRect()` 的影响

resize 后旧 map 必须清空或标记过期，直到新 viewport 尺寸的 map 到达。过期期间默认浏览器接收输入。

## 焦点和 pointer capture

pointer capture 规则：

| 事件 | 规则 |
| --- | --- |
| pointer down | 按当前 owner 锁定目标 |
| pointer move | 有锁定时沿用锁定目标 |
| pointer up | 发送给锁定目标后释放 |
| pointer cancel / runtime restart | 释放锁定 |
| wheel | 不锁定，每次按当前点判断 |

焦点规则：

- 点击 Web 区域：浏览器获得焦点，键盘和 IME 进入浏览器。
- 点击 Host 区域：默认释放浏览器焦点，宿主接管键盘。
- 后续可增加 `KeepBrowserFocusOnHostClick`，但不作为默认行为。

## 验证策略

源码侧：

- Web SDK 用 bun test 覆盖 default owner、Rect、RoundedRect、基础可见性和覆盖关系。
- Rust IPC typed payload 用 golden test 覆盖 encode/decode。
- Unity C# 对 `InputOwnershipMap.ResolveOwner`、Rect/RoundedRect hit test 和 pointer capture 做纯逻辑测试或静态可调用 helper。
- C# 修改执行 CSharpier。
- Rust 修改执行 `cargo fmt --all` 和 `cargo clippy --all-targets --all-features -- -D warnings`。

用户实测：

- Unity 小视口：网页按钮可点，视口区域拖动 Unity，视口圆角外不穿透。
- 视口工具按钮：按钮覆盖在视口上方时由 Web 处理，按钮外继续由 Host 处理。
- Unity 全屏背景：面板内由 Web 处理，面板外拖动 Unity 场景。
- 半透明面板：视觉上透出 Unity，但输入仍归 Web。
- resize、DPI scale、Canvas overlay/camera 模式下坐标一致。

## 当前过渡状态

当前代码已经实现了 `OverlayPassMap`、`data-overlay="pass"`、临时 console bridge 和 Unity host 动态 pass rect。该实现用于验证输入链路，但不应继续作为长期模型扩展圆角、默认归属和全屏背景场景。

下一阶段应把概念迁移为 `InputOwnershipMap`，并保留兼容层：

```text
data-overlay="pass"
  -> defaultOwner = web
  -> owner = host region
```

这样既不破坏当前实测链路，又能避免继续在 pass map 上叠加越来越多补丁规则。
