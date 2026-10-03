# Erlang OTP 28 原生差分与固定 grammar 漏检

日期：2026-10-03。对应 OpenSpec 14.4、14.17、14.19 的局部精度验收，Erlang 仍为未验收候选。

本机现有 `erlc`（OTP 28）逐一编译 13 份独立的 `sample.erl`：8 份合法、5 份故意破损。固定 CodeGuard Erlang WASM（SHA-256 `dbab33f03e07b89f4385fcdd48d87d86ba35c82a0a426788d55b8c25410bc491`，ABI 14）通过隔离 `grammar probe erlang` 对同一字节解析。12/13 的有效/无效分类与原生一致，没有初检未完成项；唯一分歧为缺少最终句点的 `-module(sample).\nf() -> ok`：`erlc` 拒绝，WASM 却返回零恢复。

常规候选语料测试固定这一唯一分歧；显式原生差分测试要求 `CODEGUARD_ERLC_BIN` 和 `CODEGUARD_ERL_BIN` 指向既有 OTP 28 工具，并核对每份样例的原生标签、源码字节未变、候选分类与未解析状态。两项目标测试均各 1/1 通过；CI 只执行无需安装 Erlang 的候选语料测试。固定资产清单的已知限制同步给 `grammar status` 与项目候选报告，避免智能体把零恢复解释成原生语法通过。

仍需修复或替换 grammar，并以独立的多版本、多模块 Erlang 语料复验；原生优先统一命令、系统误报/漏报率、资源预算、发行包与宿主反馈未验收。此反例不能被白名单直接放过，也不能用于关闭原生检查义务。
