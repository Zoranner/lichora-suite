# Headless Browser Rust

基于 cef-rs 的 Headless 浏览器实现，为 Unity 提供 Windows 和 Linux 支持。

## 项目状态

### 阶段一：项目搭建（已完成 ✅）

- [x] 项目结构搭建
- [x] 基础模块框架
- [x] 共享内存封装
- [x] 开发环境脚本
- [x] CEF 初始化实现
- [x] OffScreen 渲染框架
- [x] 输入模块实现
- [x] 共享内存协议定义

### 阶段二：核心功能（已完成 ✅）

- [x] 输入模块与 SharedMemory 实际连接
- [x] 键盘/鼠标/IME 事件转发到 CEF
- [x] JavaScript 执行
- [x] 渲染帧写入 Capture 共享内存
- [x] 光标位置通过 on_ime_composition_range_changed 更新

### 阶段三：测试部署（待开始）

- [ ] Windows 环境 CEF 集成测试
- [ ] Linux 环境 CEF 集成测试
- [ ] Unity 集成测试
- [ ] 性能优化
- [ ] Windows / Linux 发布打包

## 项目结构

```
headless_browser_rust/
├── Cargo.toml                 # 项目配置
├── src/
│   ├── lib.rs                 # FFI 导出（与 Unity 通信）
│   ├── main.rs                # 主程序入口
│   ├── browser/
│   │   ├── mod.rs
│   │   ├── cef_app.rs         # CEF App 和 Client 实现
│   │   ├── entry.rs           # BrowserEntry 对应
│   │   ├── handler.rs         # PageHandler 对应
│   │   └── render.rs          # RenderHandler 实现
│   ├── modules/
│   │   ├── mod.rs
│   │   ├── protocol.rs        # 共享内存协议定义
│   │   ├── base.rs            # MemoryModuleBase 对应
│   │   ├── caret.rs           # CaretModule
│   │   ├── capture.rs         # CaptureModule
│   │   ├── ime.rs             # ImeModule
│   │   ├── keyboard.rs        # KeyboardModule
│   │   ├── mouse_event.rs     # MouseEventModule
│   │   ├── mouse_state.rs     # MouseStateModule
│   │   └── script.rs          # ScriptModule
│   └── ipc/
│       └── shared_memory.rs   # 共享内存封装
└── examples/
    └── test_browser.rs        # 测试程序
```

## 快速开始

### Linux 环境

1. **安装 Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. **设置 CEF 环境**
```bash
chmod +x setup-linux.sh
./setup-linux.sh

# 添加环境变量
echo 'export CEF_PATH="$HOME/.local/share/cef"' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$CEF_PATH"' >> ~/.bashrc
source ~/.bashrc
```

3. **编译**
```bash
chmod +x build.sh
./build.sh
```

4. **运行**
```bash
./target/release/headless_browser --help
./target/release/headless_browser https://example.com
```

### Windows 环境

1. **设置 CEF 环境**
```powershell
.\setup-windows.ps1
```

2. **编译**
```powershell
.\build.ps1 -Release
```

3. **运行**
```powershell
.\target\release\headless_browser.exe --help
.\target\release\headless_browser.exe https://example.com
```

### 命令行参数

```
headless_browser [OPTIONS] [URL]

Options:
  -u, --url <URL>      初始 URL (默认: https://example.com)
  -w, --width <WIDTH>  浏览器宽度 (默认: 1280)
  -h, --height <HEIGHT> 浏览器高度 (默认: 720)
  -g, --guid <GUID>    共享内存 GUID (默认: 自动生成)
  -s, --scale <SCALE>  设备缩放因子 (默认: 1.0)
  -f, --fps <FPS>      帧率 (默认: 60)
      --help           显示帮助
```

## 共享内存协议

### 模块列表

| 模块名 | 大小 | 方向 | 用途 |
|--------|------|------|------|
| KeyEvent | 16 字节 | Unity → Browser | 键盘输入 |
| MouseEvents | 10 字节 | Unity → Browser | 鼠标点击/滚轮 |
| MouseState | 6 字节 | Unity → Browser | 鼠标位置 |
| IME | 2048 字节 | Unity → Browser | 输入法 |
| Caret | 5 字节 | Browser → Unity | 光标位置 |
| Capture | 变长 | Browser → Unity | 屏幕帧 |
| Script | 10005 字节 | Unity → Browser | JS 执行 |

详细协议见 [PROTOCOL.md](PROTOCOL.md)

## Unity 端集成

### 无需修改现有代码

现有的 `MemoryModuleBase.cs` 完全兼容，因为 Rust 版本实现了相同的二进制协议。

### 进程启动

```csharp
// 根据平台选择不同的可执行文件
#if UNITY_STANDALONE_LINUX
    string browserPath = "headless_browser";
#else
    string browserPath = "HeadlessBrowser.exe";
#endif

// 启动进程
Process.Start(browserPath, $"--guid {memoryGuid} --url {url}");
```

## 开发文档

- [开发指南](DEVELOPMENT.md)
- [协议规范](PROTOCOL.md)

## 参考资料

- [cef-rs GitHub](https://github.com/tauri-apps/cef-rs)
- [OSR 示例](https://github.com/tauri-apps/cef-rs/blob/dev/examples/osr)
- [CEF 官方文档](https://bitbucket.org/chromiumembedded/cef)

## 许可证

MIT License
