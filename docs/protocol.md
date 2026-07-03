# IPC Protocol

当前协议为 Lichora IPC v2 typed IPC。宿主适配层通过 `lichora_ipc_native` 调用 Rust IPC core，浏览器 runtime 直接使用同一个 Rust API。

当前 source of truth：

```text
../crates/ipc/src
../crates/ipc/tests
```

协议字段、通道语义和 wire format 以 `../crates/ipc` 源码和 golden tests 为准。旧协议字段表已移除，不再维护运行时兼容说明。Web/Host 融合扩展设计见 `design/web-host-overlay.md`。
