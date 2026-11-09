---
layout: layouts/docs.njk
docFilm: osprey
title: Osprey — 语法风格与重复代码检测
description: 使用 Deslop 扫描 Osprey 的全部三种语法风格，包括原生 ML 布局与显式选择的 Python 风格。
eleventyNavigation:
  key: Osprey
  order: 12
icon: code
docsGroup: guides
lang: zh
---

# Osprey 支持

Deslop 使用 tree-sitter 分析 Osprey，并通过 CLI、实时编辑器和 MCP 工具共用的同一个 Rust 引擎发送检测结果。在语言分组和报告视图中，Osprey 显示为官方青色 Osprey 图标。入门介绍请阅读 [Osprey in Deslop](/blog/osprey-first-class/)（英文）。

{% include "partials/doc-film.njk" %}

## 语法支持

本页描述的是当前开发版本中的集成情况，并不保证较早安装的 VSIX 已包含这些改动。

| 源码风格 | 选择方式 | 集成状态 |
| --- | --- | --- |
| 默认花括号风格 | `.osp` | 原生语法、CLI 测试夹具以及真实 LSP/MCP 语言标签检查均已验证。 |
| Python 风格 | 显式选择加入的 `.py` | CLI 路由、与普通 Python 的隔离以及真实 LSP/MCP 语言标签检查均已验证。 |
| ML 布局 | `.ospml` | 原生语法在源码语料与结构检查后注册；CLI 以及真实 LSP/MCP 语言标签检查均已验证。 |

默认语法保留效应（effects）、处理器（handlers）、模式和表达式，包括字符串插值中的表达式。名称和字面量会被规范化以便进行结构比较；由共享的证据检查判断候选片段是否包含被复制的实现。注释和导入语句不计为被复制的实现。

## 扫描项目

在安装了 Deslop Live 的编辑器中打开项目，或使用 CLI：

```bash
deslop ./my-osprey-project --output ./target/deslop-report
```

在选择提取方式之前，请先查看排序后的检测结果并比较各个出现位置。检测支持并不意味着提供 Osprey 自动重构，也不能证明两段实现可以互换。代码簇类别与排序见 [工作原理](/zh/docs/how-it-works/)，排除规则与阈值见 [配置](/zh/docs/configuration/)。

## 选择 Osprey 的 Python 风格

普通 `.py` 文件保持 Python 身份。开发版集成只在源码开头的注释中识别 Osprey 风格标记：

```python
# osprey: flavor=python
```

包含该文本的字符串，或位于代码之后的注释，都无法选择风格。也可以在现有的 `osprey.toml` 中逐个列出源文件：

```toml
[project]
entry = "src/main.py"
python_sources = ["src/domain.py", "src/storage.py"]
```

Python 入口文件和列出的文件会加入 Osprey。路径是相对于清单文件的具体文件，不支持整个目录或通配符选择。其他 `.py` 文件保持普通 Python 身份。

项目的 `flavor` 设置接受 `default`、`ml` 或 `python`，并覆盖该项目所包含源码的语法。若没有覆盖设置，与扩展名矛盾的标记会产生错误。格式错误的清单和未知的风格值同样会产生错误。源码根目录在存在 `src` 时默认为 `src`，否则为项目目录；`source_roots` 可覆盖此默认值。嵌套项目优先。

## 报告中的语言身份

引擎在解析器和缓存身份中区分 `osprey`、`osprey-ml` 和 `osprey-python`。出现位置和文件指标都携带所选语言，因此即使 Osprey Python 与普通 Python 都使用 `.py`，客户端也能区分二者。界面将这三种身份归为 Osprey 一组，并使用同一个王冠图标。

同属一个语言家族并不代表不同风格之间存在克隆等价关系。Osprey 的 Python 风格不会继承 Python 的自动重构操作。无效的 ML 语法会产生可见的解析错误，而不是基于部分语法树生成报告。
