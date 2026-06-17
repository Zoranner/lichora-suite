# 开发指南

## 环境要求

### Linux（目标平台）
- Rust 1.70+
- CEF 145.0.27
- GCC/Clang
- pkg-config

### Windows（开发测试）
- Rust 1.70+
- Visual Studio Build Tools
- CEF 145.0.27

## 快速开始

### Linux 环境

1. **设置 CEF 环境**
```bash
chmod +x setup-linux.sh
./setup-linux.sh

# 添加环境变量到 shell 配置
echo 'export CEF_PATH="$HOME/.local/share/cef"' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$CEF_PATH"' >> ~/.bashrc
source ~/.bashrc
```

2. **编译项目**
```bash
chmod +x build.sh
./build.sh
```

3. **运行测试**
```bash
cargo run --example test_browser
```

### Windows 环境

1. **下载 CEF**
   - 访问 https://cef-builds.spotifycdn.com/index.html#windows64
   - 下载 145.0.27 版本
   - 解压到 `C:\cef`

2. **设置环境变量**
```powershell
$env:CEF_PATH = "C:\cef"
$env:PATH += ";C:\cef"
```

3. **编译项目**
```powershell
.\build.ps1
```

## 模块说明

### Browser 模块 (`src/browser/`)
- `entry.rs` - 主浏览器实例管理
- `handler.rs` - 页面事件处理
- `render.rs` - OffScreen 渲染实现

### Modules 模块 (`src/modules/`)
- `keyboard.rs` - 键盘输入处理
- `mouse_state.rs` - 鼠标位置追踪
- `mouse_event.rs` - 鼠标事件处理
- `ime.rs` - 输入法支持
- `caret.rs` - 光标位置追踪
- `script.rs` - JavaScript 执行

### IPC 模块 (`src/ipc/`)
- `shared_memory.rs` - 共享内存封装

## 共享内存协议

### 与 Unity 的通信协议

所有模块使用共享内存与 Unity 进行通信，协议与 C# 版本保持一致。

#### 帧数据（Render）
```
偏移  大小  字段
0     4     width (i32)
4     4     height (i32)
8     N     pixels (BGRA, N = width * height * 4)
```

#### 键盘事件（Keyboard）16 字节
```
偏移  大小  字段
0     1     flag (u8)        1=新事件, 0=已处理
1     1     eventType (u8)   1=KeyDown, 2=KeyUp, 3=Char
2     4     windowsKeyCode (i32, LE)
6     4     nativeKeyCode (i32, LE)
10    1     modifiers (u8)   0x01=Ctrl, 0x02=Shift, 0x04=Alt
11    4     character (i32, LE)
15    1     padding
```

#### 鼠标状态（MouseState）6 字节
```
偏移  大小  字段
0     1     flag (u8)        1=有效, 0=忽略
1     2     x (i16, LE)
3     2     y (i16, LE)
5     1     buttonState (u8) bit0=Left, bit1=Right, bit2=Middle
```

## 开发流程

1. **功能开发**
   - 创建功能分支
   - 实现功能
   - 编写测试
   - 提交代码

2. **测试**
   - 单元测试：`cargo test`
   - 集成测试：`cargo run --example test_browser`
   - 性能测试：使用帧率监控

3. **构建发布**
```bash
# Linux
./build.sh --package

# 生成位置
# dist/linux/headless_browser
```

## 调试技巧

### 启用日志
```bash
RUST_LOG=debug cargo run
```

### 检查共享内存
```rust
// 在代码中添加
log::debug!("Shared memory size: {}", self.shmem.size());
log::debug!("Frame data: {:?}", &data[..100]);
```

### 性能分析
```bash
# 使用 perf (Linux)
perf record cargo run --release
perf report
```

## 常见问题

### CEF 找不到
确保设置了 `CEF_PATH` 和 `LD_LIBRARY_PATH` 环境变量。

### 共享内存创建失败
检查权限和内存名称是否与 Unity 端一致。

### 渲染黑屏
- 检查 GPU 加速是否正常
- 验证帧数据格式（BGRA）
- 确认宽高设置正确

## 相关资源

- [cef-rs 文档](https://docs.rs/cef)
- [CEF API 参考](https://magpcss.org/ceforum/apidocs3/)
- [Rust FFI 指南](https://doc.rust-lang.org/nomicon/ffi.html)
