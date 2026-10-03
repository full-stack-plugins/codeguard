# VB.NET 未缩进方法候选误报复现

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的精度阻塞，不是验收完成。

使用已发布的 0.1.3 macOS arm64 WASM 二进制，对以下两份源码分别执行 `codeguard grammar probe vbnet <物理绝对路径> --format=json`：

```vbnet
Public Class C
Public Function F() As Integer
Return 1
End Function
End Class
```

未缩进版本返回一处 `MISSING ":"`，`start_row=1`、`start_column_byte=30`、`start_byte=45`。给方法声明和主体增加四、八个空格后，同一固定 grammar 返回零恢复节点。两份报告均为 `status=incomplete`、`grammar_qualified=false`、`delivery_decision=not_evaluated`、退出码 3；WASM SHA-256 为 `e38a09e1826c7ec06340c0531a98bbb5ee79bdd710bba3d3349b79a19721e844`。

该差异符合已记录的 grammar 误报风险，但本机未安装 .NET 编译器，尚无原生语法判定证据。修复必须在固定来源 grammar 上重建、核验发行字节与许可证，使用合法/非法样例及原生编译器差分复测；不能通过忽略所有 `MISSING ":"`、给源码补缩进或把候选升级为源码违规来规避。
