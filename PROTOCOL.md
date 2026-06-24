# IPC Protocol

当前协议进入 v2 重设计阶段。旧 MemoryStacks 单槽 flag 协议不再作为后续实现目标，也不要求新实现保持运行时兼容。

新的 source of truth 见：

```text
../docs/design/architecture.md
```

本文件后续应随 `crates/ipc` golden tests 改写为 IPC v2 的正式协议规范。旧协议内容已移除，避免后续实现继续围绕 MemoryStacks envelope、4 字节 length header 和单槽 flag 做兼容设计。
