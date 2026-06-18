# 共享内存协议规范

本文档定义 Unity 和 `headless_browser` 之间的 MemoryStacks 共享内存协议。所有多字节字段均为 little-endian。

## MemoryStacks 包装

底层共享内存文件前 4 字节由 MemoryStacks 用作 payload length。本文档后续偏移均指 payload 内偏移，不包含这 4 字节长度头。

普通事件模块使用首字节 flag：

```text
[0] = 1  新数据可用
[0] = 0  数据已处理或无效
```

`MouseState` 是连续状态，不执行 Browser 端 ack；Unity 持续覆盖最新状态。

## 模块列表

| 模块名 | 大小 | 方向 | 用途 |
| --- | --- | --- | --- |
| `Handler.{handlerGuid}` | 3000 字节 | Unity -> Browser | handler 管理命令 |
| `HEARTBEAT.{handlerGuid}` | 16 字节 | Unity -> Browser | handler 心跳 |
| `KeyEvent.{browserGuid}` | 16 字节 | Unity -> Browser | 键盘输入事件 |
| `MouseEvents.{browserGuid}` | 10 字节 | Unity -> Browser | 鼠标点击和滚轮 |
| `MouseState.{browserGuid}` | 6 字节 | Unity -> Browser | 鼠标位置状态 |
| `IME.{browserGuid}` | 2048 字节 | Unity -> Browser | 输入法组合事件 |
| `Caret.{browserGuid}` | 5 字节 | Browser -> Unity | 光标位置 |
| `Capture.{browserGuid}` | 44236832 字节 | Browser -> Unity | Capture v2 帧数据 |
| `Script.{browserGuid}` | 10005 字节 | Unity -> Browser | JavaScript 执行 |

## Handler 模块

`Handler.{handlerGuid}` 由 Unity 写入，Browser 在处理命令后清除 payload。命令大小上限为 3000 字节。

```text
偏移  大小  类型    描述
0     1     u8      flag，1=新命令
1     1     u8      commandType，0=Shutdown，1=AddBrowser，2=RemoveBrowser，3=ResizeBrowser
2     36    bytes   browser GUID，ASCII，空余位置填 0
38    2     i16     width，AddBrowser/ResizeBrowser 使用
40    2     i16     height，AddBrowser/ResizeBrowser 使用
42    2     i16     addressLength，AddBrowser 使用
44    N     bytes   URL UTF-8 bytes，AddBrowser 使用
```

`Shutdown` 只需要 `[flag=1, commandType=0]`。`RemoveBrowser` 只需要 flag、commandType 和 browser GUID。

## Heartbeat 模块

`HEARTBEAT.{handlerGuid}` 由 Unity 周期性写入，Browser handler watchdog 读取。重复或倒退的 sequence 会被忽略。

```text
偏移  大小  类型  描述
0     8     i64   sequence，必须递增且大于 0
8     8     i64   utcTicks
```

## Keyboard 模块

`KeyEvent.{browserGuid}` 大小为 16 字节。

```text
偏移  大小  类型  描述
0     1     u8    flag，1=新事件
1     1     u8    KeyEventType，1=KeyDown，2=KeyUp，3=Char
2     4     i32   Windows key code
6     4     i32   native key code
10    1     u8    modifiers，0x01=Ctrl，0x02=Shift，0x04=Alt
11    4     i32   character
15    1     u8    padding
```

## Mouse Events 模块

`MouseEvents.{browserGuid}` 最大 10 字节。

点击事件使用 6 字节：

```text
偏移  大小  类型  描述
0     1     u8    flag
1     1     u8    MouseEventType，1=LeftDown，2=LeftUp，3=RightDown，4=RightUp，5=MiddleDown，6=MiddleUp
2     2     i16   x
4     2     i16   y
```

滚轮事件使用 10 字节：

```text
偏移  大小  类型  描述
0     1     u8    flag
1     1     u8    MouseEventType.Scroll，值为 7
2     2     i16   x
4     2     i16   y
6     2     i16   deltaX
8     2     i16   deltaY
```

## Mouse State 模块

`MouseState.{browserGuid}` 固定 6 字节。

```text
偏移  大小  类型  描述
0     1     u8    valid，1=有效，0=忽略
1     2     i16   x
3     2     i16   y
5     1     u8    buttonState，bit0=Left，bit1=Right，bit2=Middle
```

该模块是最新状态通道，不使用 ack。Browser 端读取后合并移动事件，Unity 可以直接覆盖旧值。

## IME 模块

`IME.{browserGuid}` 最大 2048 字节。

```text
偏移  大小  类型    描述
0     1     u8      flag
1     1     u8      ImeOperationType，1=SetComposition，2=CommitText，3=CancelComposition
2     2     i16     textLength
4     2     i16     cursorPosition
6     N     bytes   UTF-8 文本，最大 2040 字节
```

## Caret 模块

`Caret.{browserGuid}` 当前公开模块大小为 5 字节。

```text
偏移  大小  类型  描述
0     1     u8    valid，1=有效
1     2     i16   x
3     2     i16   y
```

## Script 模块

`Script.{browserGuid}` 最大 10005 字节。

```text
偏移  大小  类型    描述
0     1     u8      flag，1=新脚本
1     2     i16     scriptLength，最大 10000
3     2     bytes   reserved
5     N     bytes   JavaScript UTF-8 bytes
```

## Capture v2 模块

`Capture.{browserGuid}` 使用三槽 payload 加尾部 header。固定尺寸为：

```text
slotSize = 2560 * 1440 * 4 = 14745600
slotCount = 3
headerSize = 32
payloadSize = slotSize * slotCount + headerSize = 44236832
```

payload 布局：

```text
偏移                    大小       描述
0                       slotSize   slot 0
slotSize                slotSize   slot 1
slotSize * 2            slotSize   slot 2
slotSize * 3            32         capture header
```

32 字节 capture header：

```text
偏移  大小  类型  描述
0     4     i32   width
4     4     i32   height
8     4     i32   slot，当前帧所在槽，0..2
12    4     i32   sequence，Browser 每发布一帧递增
16    4     i32   frameType，0=full，1=dirty
20    4     i32   rectCount，dirty frame 的矩形数量
24    4     i32   payloadSize，当前 slot 中有效 payload 字节数
28    4     i32   ackSequence，Unity 写回已消费的 sequence
```

full frame payload：

```text
slotOffset  width * height * 4 bytes  BGRA pixels
```

dirty frame payload 由若干 rect payload 顺序组成，每个 rect 先写 16 字节 rect header，再写该矩形逐行 BGRA 像素：

```text
偏移  大小  类型  描述
0     4     i32   x
4     4     i32   y
8     4     i32   width
12    4     i32   height
16    N     bytes rect BGRA pixels，N = width * height * 4
```

dirty frame 发布条件：

- 上一帧 `sequence` 大于 0。
- Unity 已将 header 的 `ackSequence` 写回上一帧 `sequence`。
- CEF 提供的 dirty rect 非空，数量不超过 64。
- 裁剪后的 dirty payload 不超过单槽容量。
- dirty 面积小于整帧面积的 70%。
- 尺寸未变化。

任一条件不满足时，Browser 发布 full frame。Unity 消费任意 frame 后，必须把 header offset 28 的 `ackSequence` 写成已消费的 `sequence`，否则下一帧 dirty 会回退为 full frame。
