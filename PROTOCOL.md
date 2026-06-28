# IPC Protocol

当前协议为 IPC v2 typed IPC。Unity 通过 `browser_ipc_native` 调用 Rust IPC core，`headless_browser` 直接使用同一个 Rust API。

新的 source of truth 见：

```text
../docs/design/architecture.md
```

协议字段、通道语义和 wire format 以设计文档和 `headless_browser` 源码中的 IPC/protocol 实现为准。旧协议字段表已移除，不再维护运行时兼容说明。
