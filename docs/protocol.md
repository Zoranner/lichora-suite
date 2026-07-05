# IPC Protocol

当前协议为 Lichora IPC v2 typed IPC。宿主适配层通过 `lichora_ipc_native` 调用 Rust IPC core，浏览器 runtime 直接使用同一个 Rust API。

当前 source of truth：

```text
../crates/lichora-ipc/src
../crates/lichora-ipc/tests
```

协议字段、通道语义和 wire format 以 `../crates/lichora-ipc` 源码和 golden tests 为准。旧协议字段表已移除，不再维护运行时兼容说明。Web/Host 融合扩展设计见 `design/web-host-overlay.md`。

## Typed Payload Header

typed payload 使用固定 8 字节 header，所有多字节整数和浮点数字段均为 little-endian。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| magic | `u8[4]` | input 为 `EBIP`，output 为 `EBOP` |
| major | `u8` | 当前为 `1` |
| minor | `u8` | 当前为 `0` |
| kind | `u16` | payload kind |

当前 output payload kind：

| kind | 名称 |
| --- | --- |
| `1` | `Caret` |
| `2` | `SurroundingText` |
| `3` | `ScriptResult` |
| `4` | `PageEvent` |
| `5` | `OverlayPassMap` |

## OverlayPassMap Typed Output

`OutputPayloadKind::OverlayPassMap = 5`。该 payload 使用 output typed payload header，因此 header 为 `EBOP`、版本 `1.0`、`kind = 5`。

`OverlayPassMap` 是当前已落地的兼容 payload，用于表达“默认浏览器、局部穿透宿主”的旧模型。下一阶段 Web/Host 融合主模型是 `InputOwnershipMap`，用于同时表达 `defaultOwner = Web | Host` 和 region `owner = Web | Host`。在 ownership payload 落地前，宿主适配层可以把 `OverlayPassMap` 映射为 `defaultOwner = Web` 下的 Host regions。

header 后的 payload 字段如下：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| version | `u64` | pass map 版本，由生产侧递增 |
| viewport_width | `i32` | CSS viewport 宽度 |
| viewport_height | `i32` | CSS viewport 高度 |
| device_scale_factor | `f32` | 设备缩放因子 |
| enabled | `u8` | `0` 表示禁用，非 `0` 表示启用 |
| reserved | `u8[7]` | 保留字段，当前写入 `0` |
| region_count | `u32` | 后续 region 条目数量 |

每个 region 条目固定 24 字节：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| id | `u32` | region id |
| shape | `u8` | 第一版 `1` 表示矩形 |
| disabled | `u8` | `0` 表示启用，非 `0` 表示禁用 |
| reserved | `u16` | 保留字段，当前写入 `0` |
| x | `f32` | CSS viewport x 坐标 |
| y | `f32` | CSS viewport y 坐标 |
| width | `f32` | CSS viewport 宽度 |
| height | `f32` | CSS viewport 高度 |

第一版只定义 `shape = 1` 的矩形区域。坐标使用 CSS viewport 坐标，不是 Unity local 坐标，也不是设备像素坐标；宿主侧负责把本地 pointer 坐标映射到 CSS viewport 坐标后查询 pass map。

当 pass map 禁用、非法、viewport 与当前页面尺寸不匹配，或宿主侧认为 pass map 已过期时，默认由浏览器接收输入，避免页面 UI 因过期穿透状态失控。

当前 Rust IPC core 已实现 `OverlayPassMap` typed output 编解码和 golden test。Unity host 已解码并消费该 typed payload，将有效矩形转换为动态 pass rect。Web SDK 已能生成 pass map，并通过临时 console bridge 交给 Rust 转换为 typed `OverlayPassMap`。正式 CEF message route 或 process message bridge、`InputOwnershipMap` payload、Unity 内 DOM ownership 区域实测仍属于后续验收项。
