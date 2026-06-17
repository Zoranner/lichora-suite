# 共享内存协议规范

本文档定义了 Unity (C#) 和 HeadlessBrowser (Rust) 之间的共享内存通信协议。

## 通信模型

- **机制**：命名共享内存
- **命名格式**：`{ModuleName}.{guid}`
- **同步方式**：握手式事件系统
- **轮询频率**：~100Hz (10ms 间隔)

### 握手协议

所有模块使用单字节标志位进行同步：

```
[0] = 1  →  新数据可用（写入方设置）
[0] = 0  →  数据已处理（读取方处理完成后设置）
```

## 模块列表

| 模块名 | 大小 | 方向 | 用途 |
|--------|------|------|------|
| KeyEvent | 16 字节 | Unity → Browser | 键盘输入事件 |
| MouseEvents | 10 字节 | Unity → Browser | 鼠标点击和滚轮 |
| MouseState | 6 字节 | Unity → Browser | 鼠标位置状态 |
| IME | 2048 字节 | Unity → Browser | 输入法组合事件 |
| Caret | 5 字节 | Browser → Unity | 光标位置 |
| Capture | 变长 | Browser → Unity | 屏幕位图 |
| Script | 10005 字节 | Unity → Browser | JavaScript 执行 |

---

## 详细协议定义

### 1. Keyboard 模块 (KeyEvent.{guid})

**大小**：16 字节

```
偏移  大小  类型      描述
----  ----  --------  ---------------------------
[0]   1     u8        事件标志 (1=新事件, 0=已处理)
[1]   1     u8        KeyEventType 枚举
[2]   4     i32       Windows 键码 (小端)
[6]   4     i32       原生键码 (小端)
[10]  1     u8        KeyModifiers 标志
[11]  4     i32       字符 (小端)
[15]  1     -         填充
```

**KeyEventType 枚举**：
```rust
KeyDown = 1
KeyUp = 2
Char = 3
```

**KeyModifiers 标志**：
```rust
None = 0
Ctrl = 0x01
Shift = 0x02
Alt = 0x04
```

---

### 2. Mouse Events 模块 (MouseEvents.{guid})

**大小**：10 字节（最大）

#### 点击事件 (6 字节)
```
偏移  大小  类型      描述
----  ----  --------  ---------------------------
[0]   1     u8        事件标志
[1]   1     u8        MouseEventType 枚举
[2]   2     i16       鼠标 X 坐标 (小端)
[4]   2     i16       鼠标 Y 坐标 (小端)
```

#### 滚轮事件 (10 字节)
```
偏移  大小  类型      描述
----  ----  --------  ---------------------------
[0]   1     u8        事件标志
[1]   1     u8        MouseEventType.Scroll (=7)
[2]   2     i16       鼠标 X 坐标 (小端)
[4]   2     i16       鼠标 Y 坐标 (小端)
[6]   2     i16       滚轮 Delta X (小端)
[8]   2     i16       滚轮 Delta Y (小端)
```

**MouseEventType 枚举**：
```rust
LeftDown = 1
LeftUp = 2
RightDown = 3
RightUp = 4
MiddleDown = 5
MiddleUp = 6
Scroll = 7
```

**滚轮计算**：`delta = scrollDelta * 40`

---

### 3. Mouse State 模块 (MouseState.{guid})

**大小**：6 字节

```
偏移  大小  类型      描述
----  ----  --------  ---------------------------
[0]   1     u8        有效标志 (1=有效, 0=忽略)
[1]   2     i16       鼠标 X 坐标 (小端)
[3]   2     i16       鼠标 Y 坐标 (小端)
[5]   1     u8        按钮状态标志
```

**按钮状态标志**：
```rust
bit 0 (0x01) = 左键按下
bit 1 (0x02) = 右键按下
bit 2 (0x04) = 中键按下
```

---

### 4. IME 模块 (IME.{guid})

**大小**：2048 字节（最大）

```
偏移      大小    类型      描述
----      ----    --------  ---------------------------
[0]       1       u8        事件标志
[1]       1       u8        ImeOperationType 枚举
[2]       2       i16       文本长度 (小端)
[4]       2       i16       光标位置 (小端)
[6-2047]  变长    byte[]    UTF-8 编码文本 (最大 2040 字节)
```

**ImeOperationType 枚举**：
```rust
SetComposition = 1    // 设置组合预览文本
CommitText = 2        // 提交选中文本
CancelComposition = 3 // 取消输入法组合
```

---

### 5. Caret 模块 (Caret.{guid})

**大小**：5 字节
**方向**：Browser → Unity

```
偏移  大小  类型      描述
----  ----  --------  ---------------------------
[0]   1     u8        有效标志 (1=有效, 0=无效)
[1]   2     i16       光标 X 坐标 (小端)
[3]   2     i16       光标 Y 坐标 (小端)
```

---

### 6. Script 模块 (Script.{guid})

**大小**：10005 字节（最大）

```
偏移       大小    类型      描述
----       ----    --------  ---------------------------
[0]        1       u8        事件标志 (1=新脚本, 0=已处理)
[1]        2       i16       脚本长度 (小端, 最大 10000)
[3]        2       -         保留 (填充)
[5-10004]  变长    byte[]    UTF-8 编码 JavaScript 代码
```

---

### 7. Capture 模块 (Capture.{guid})

**大小**：变长（取决于分辨率）
**方向**：Browser → Unity

```
偏移      大小            类型      描述
----      ----            --------  ---------------------------
[0-3]     4               i32       数据宽度 (小端)
[4-7]     4               i32       数据高度 (小端)
[8-]      width*height*4  byte[]    BGRA 像素数据
```

**约束**：
- 最大宽度：3840
- 最大高度：2160
- 像素格式：每像素 4 字节 (BGRA)

---

## 实现注意事项

### 字节序
所有多字节字段使用**小端序**（Little-Endian）。

### 内存对齐
结构体使用 `#[repr(C, packed)]` 确保与 C# 内存布局兼容。

### 线程安全
- Unity 端：主线程写入
- Browser 端：专用线程读取
- 使用标志位进行简单同步，无需锁

### 错误处理
- 读取前检查数据长度
- 解析失败时忽略当前事件
- 标志位清除前完成所有处理

---

## 代码示例

### Rust 端读取事件
```rust
fn process_keyboard_event(shmem: &SharedMemoryWrapper) {
    if let Ok(data) = shmem.read_bytes() {
        if let Some(event) = KeyboardEvent::from_bytes(&data) {
            if event.flag == 1 {
                // 处理事件
                handle_key(&event);

                // 清除标志
                shmem.write_bytes(&[0]);
            }
        }
    }
}
```

### Rust 端写入帧
```rust
fn write_capture_frame(shmem: &mut SharedMemoryWrapper, width: i32, height: i32, pixels: &[u8]) {
    let frame = CaptureFrame { width, height, pixels: pixels.to_vec() };
    let data = frame.to_bytes();
    shmem.write_bytes(&data);
}
```

---

## 版本历史

- **v1.0** - 初始协议定义
- 基于现有 C# 实现（CefSharp 版本）
- 与 Unity 端 MemoryModuleBase.cs 完全兼容
