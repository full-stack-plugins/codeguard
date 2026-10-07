# CSS v1.1 实施计划

> 日期：2026-10-07；OpenSpec 7.4 / 8.46-8.48。
> 状态：**零实现**，需从源构建。

## 现状

| 项目 | 状态 |
|---|---|
| WASM grammar | ❌ 不在 CodeGraph 32 种覆盖中 |
| 原生工具（stylelint） | ❌ 未安装 |
| 语法能力 | not_integrated |
| 文档能力 | not_integrated |
| 规范能力 | not_integrated |
| 漏洞能力 | not_integrated |

## 实施步骤

### 第 1 步：获取 CSS grammar

**选项 A：从 tree-sitter-css 构建**
```bash
# 克隆 tree-sitter-css
git clone https://github.com/tree-sitter/tree-sitter-css.git
# 构建 WASM
tree-sitter build --wasm
# 复制到 grammars/css/parser.wasm
```

**选项 B：从 CodeGraph 扩展**
- CodeGraph 当前覆盖 32 种语言，不含 CSS
- 需要向 CodeGraph 提交 CSS grammar 支持

### 第 2 步：语法精度验证

1. 创建 CSS 语料（200 合法 + 300 违规）
2. 运行 WASM grammar 解析
3. 计算 Wilson 下界
4. 目标：Wilson ≥ 0.98，零假阳性

### 第 3 步：原生工具集成

1. 集成 stylelint（CSS lint）
2. 集成 stylelint-order（属性排序）
3. 集成 stylelint-config-standard（标准配置）

### 第 4 步：四能力验收

1. **syntax**：CSS 语法检查（WASM + stylelint）
2. **documentation**：CSS 注释检查（类/ID/属性文档）
3. **conventions**：CSS 规范检查（stylelint 规则）
4. **vulnerabilities**：CSS 依赖检查（npm audit）

## 阻塞项

1. **CSS grammar**：需从 tree-sitter-css 构建或向 CodeGraph 提交
2. **stylelint**：需安装 stylelint 及相关插件
3. **验收语料**：需构建 CSS 语法样本集

## 预计工作量

| 步骤 | 工作量 | 依赖 |
|---|---|---|
| 获取 CSS grammar | 中 | tree-sitter-css 或 CodeGraph |
| 语法精度验证 | 小 | grammar 可用 |
| 原生工具集成 | 中 | stylelint 安装 |
| 四能力验收 | 大 | 以上全部 |

## 结论

CSS 需要从零实现，当前无 grammar 资产和原生工具。
建议优先获取 CSS grammar（从 tree-sitter-css 构建），
然后逐步集成原生工具并完成四能力验收。
