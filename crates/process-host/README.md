# process-host

宿主原生插件，为 Lichora host adapter 提供跨平台进程管理能力，解决部分宿主环境不适合直接使用托管 `Process.Start` 的问题。

## 导出 API

| 函数                                  | 说明                                         |
| ------------------------------------- | -------------------------------------------- |
| `process_host_spawn(executable, args)` | 启动进程，返回 PID，失败返回 -1              |
| `process_host_wait(pid)`              | 阻塞等待进程退出，返回退出码                 |
| `process_host_is_running(pid)`        | 检查进程是否仍在运行，返回 1/0               |
| `process_host_kill(pid, force)`       | 终止进程，force=0 优雅终止，force=1 强制终止 |

## 构建

需要安装 [Rust](https://rustup.rs)。

Windows（本机）：

```powershell
cargo build -p process-host --release
# 产物：target/release/process_host.dll
```

Linux（本机）：

```bash
cargo build -p process-host --release
# 产物：target/release/libprocess_host.so
```

Linux（从 Windows 交叉编译，需要安装 [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)）：

```powershell
cargo zigbuild -p process-host --release --target x86_64-unknown-linux-gnu
# 产物：target/x86_64-unknown-linux-gnu/release/libprocess_host.so
```

## 发布产物

runtime 构建脚本会把库复制到对应平台的发布目录：

```
dist/win-x64/process_host.dll
dist/linux-x64/libprocess_host.so
```

具体宿主适配层负责从 runtime 发布包中取用该库。
