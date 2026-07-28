---
layout: layouts/docs.njk
title: 研究背景 — 代码克隆检测算法
description: Deslop 的研究脉络 — Baxter 1998、Chilowicz 2009 的 Merkle 指纹、Broder 1997 的 MinHash、Indyk-Motwani 1998 的 LSH、SSCD 2024 的 HNSW。每一条都对应一个真实文件。
eleventyNavigation:
  key: 研究背景
  order: 8
icon: science
lang: zh
---

# 研究背景

Deslop 是一套针对代码库的克隆分析系统，适用于重复增长速度快于人工评审能力的场景。其设计借鉴了经典的代码克隆检测研究，但实现是具体且可审计的：发现、tree-sitter 解析、AST 归一化、Merkle 指纹、兄弟窗口匹配、MinHash LSH、可选的嵌入搜索、配对融合、传递闭包、排名以及报告渲染，全部位于 `deslop-core` 中。

本页区分两件事：

- **研究背景**：塑造该工具的文献与算法家族。
- **已实现的行为**：当前代码实际所做之事，并附有审计者可验证的文件/函数指针。

## 为什么 AI 改变了克隆问题

在 AI 编码助手出现之前，代码克隆就已经是一种维护风险。既有的担忧并非每一处重复都自动算错；而是被复制的逻辑必须在各类修复、安全补丁和功能变更中保持一致。AI 辅助开发改变了经济账：再生成一个相似实现的成本下降了，因此重复逻辑可能在日常功能开发中进入仓库，而不再只是通过刻意的复制粘贴。

近期研究支持这种风险模型：

- **LLM 代码重复**：[*Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*](https://arxiv.org/abs/2504.12608) 研究了 19 个代码 LLM，报告称重复出现在字符、语句和块级别，包括结构上冗余的代码。该论文还在开源和工业环境中评估了一种重复缓解技术。
- **LLM 生成的克隆**：[*Unveiling the potential of large language models in generating semantic and cross-language clones*](https://arxiv.org/abs/2309.06424) 评估了 GPT-3 生成语义克隆和跨语言克隆变体的能力，这与 Type-4 及跨语言重复检测直接相关。
- **商用 AI 代码生成器**：[*An Empirical Study of Code Clones from Commercial AI Code Generators*](https://dl.acm.org/doi/10.1145/3729397)（FSE 2025）报告称，所研究的商用生成器的 Type-1 和 Type-2 克隆率高达 7.50%，并讨论了版权、缺陷传播和漏洞传播的风险。
- **AI 时代的克隆检测**：[*Are Classical Clone Detectors Good Enough For the AI Era?*](https://arxiv.org/abs/2509.25754) 在 GPTCloneBench 及传统克隆基准上评估了九种克隆检测器，凸显了当克隆由 AI 生成时归一化与语义变化为何至关重要。
- **生产仓库中的技术债**：[*Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*](https://arxiv.org/abs/2603.28592) 分析了经核实的 AI 编写的提交，并跟踪这些提交引入的静态分析问题。它的范围比重复更广，但支持将 AI 生成的代码视为需要在仓库层面进行评估的技术债来源。

因此 Deslop 提出的商业主张是保守的：重复逻辑是一种不断累积的维护负债，而 AI 会提高产生这种负债的速率。Deslop 度量这种负债，以便团队和智能体能够决定是抽取、复用、隐藏还是显式接受某个克隆。

## 克隆分类法

Deslop 遵循代码克隆文献中通用的标准克隆分类法：

| 克隆类别 | 含义 | Deslop 信号 |
| --- | --- | --- |
| Type-1 | 除布局或注释外完全复制的文本 | 解析与归一化后的结构哈希 |
| Type-2 | 结构相同，但标识符被重命名或字面量被改动 | 标识符/字面量折叠后的结构哈希 |
| Type-3 | 插入、删除或改动了语句的近似克隆 | 兄弟窗口指纹与 token MinHash LSH |
| Type-4 | 行为相似但语法或结构不同 | 可选的嵌入余弦相似度 |

公开报告的分桶在 `crates/deslop-core/src/buckets.rs` 中实现。代码将信号三元组映射到五个线协议标签：`identical`、`nearly_identical`、`structural_only`、`loosely_similar` 和 `same_behavior`。`structural_only` 桶标记那些唯一证据是归一化 AST 形状的簇；它们默认在排名中降权。`same_behavior` 桶只有在嵌入信号足够强时才可达。

## 算法基础

每一行将一条研究脉络与实现它的文件（✅）、跟踪它的计划（⏳）或表示我们调研过但未采用（🚫）的说明配对。

| 研究脉络 | Deslop 从中采纳了什么 | 状态与实现指针 |
| --- | --- | --- |
| [Baxter et al. 1998 — AST 克隆检测](https://ieeexplore.ieee.org/document/738528) | 将代码解析为语法树，归一化无关的拼写，并比较树结构而非原始文本。 | ✅ `crates/deslop-core/src/lang/shared.rs`、`lang/csharp.rs`、`lang/rust_lang.rs`、`lang/python.rs`、`lang/dart.rs`（通过 `pipeline/corpus.rs::default_parsers` 注册） |
| Chilowicz et al. 2009 — 语法树指纹 | 对子树进行哈希，使精确的结构克隆产生相等的指纹；通过兄弟序列扩展覆盖近似克隆。 | ✅ `crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` 中的自底向上 BLAKE3 Merkle，以及 `crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` 中宽度 2..8 的兄弟窗口 |
| [SourcererCC (Sajnani et al. 2016)](https://arxiv.org/abs/1512.06448) | 用于可扩展近似检测的 token k-gram 与 Jaccard 相似度。 | ✅ 调整为基于**归一化 AST 种类 k-gram**而非原始源 token：`crates/deslop-core/src/tokens.rs`、`crates/deslop-core/src/pipeline/signatures.rs` |
| [MinHash (Broder 1997)](https://ieeexplore.ieee.org/document/666900) | 从紧凑签名估计 Jaccard。 | ✅ `crates/deslop-core/src/lsh.rs::minhash_signature` 中的 128 值签名；由 `estimate_jaccard` 估计 Jaccard |
| LSH 分带 (Indyk & Motwani 1998) | 以亚线性时间将相似指纹分桶。 | ✅ `crates/deslop-core/src/lsh.rs::band_collisions` 中的 32 带 × 4 行 |
| In Defense of MinHash Over SimHash (Shrivastava & Li 2014) | 对二值化特征使用 MinHash 而非 SimHash。 | ✅ 选用 MinHash；未使用 SimHash 和 Winnowing |
| 神经语义克隆检测 (CodeBERT、GraphCodeBERT、UniXCoder) | 将嵌入用作 Type-4 克隆的召回层。 | ✅ `crates/deslop-core/src/embedding/provider.rs` 中的 `EmbeddingProvider` trait；`embedding/ollama.rs` 中的 Ollama provider；默认模型 `nomic-embed-text` |
| [SSCD (Ahmed et al., Wiley 2024) — 规模化的 BERT + ANN](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) | 将基于 BERT 式嵌入的 HNSW ANN 作为 Type-3/4 召回路径。 | ✅ 带确定性种子的 `instant-distance` HNSW、top-k 检索、余弦阈值 0.80：`crates/deslop-core/src/embedding/pairs.rs` |
| [Ensemble-LLM 2025 (arXiv 2510.15480) — max/sum 融合](https://arxiv.org/abs/2510.15480) | 平均会有害；融合信号时 max 与 sum 有帮助。 | ✅ `crates/deslop-core/src/pair.rs::PairScore::fused` 中三信号的 max 归一化求和，钳制到 `[0,1]`。多模型嵌入集成**尚未**实现（当前为单一嵌入模型；provider trait 保留了扩展可能） |
| 混合克隆检测（没有纯 RAG 论文推荐使用纯嵌入） | 对结构、token 与嵌入配对取并集，融合，聚簇。 | ✅ `crates/deslop-core/src/pair.rs::candidate_pairs`，`crates/deslop-core/src/cluster.rs` 中的传递闭包 |
| 样板过滤（成熟工具的惯例） | 在指纹化前丢弃 import / namespace / decorator 克隆；以低噪声提示的形式重新呈现。 | ✅ `crates/deslop-core/src/boilerplate.rs` 与 `report_boilerplate.rs` |
| HyClone 2025 (arXiv 2508.01357) — 执行验证的 Type-4 | 生成测试输入并验证语义等价的配对。 | 🚫 未实现；原论文中针对 Python |
| Rator 2025 (Springer) — 节点自由度编码 | 通过节点自由度编码子树；细粒度 Top-2/Top-3 定位。 | 🚫 未实现；LSH 已覆盖我们当前的召回预算 |
| Autofix `refactor.extract`（LSP 代码动作） | 将 Type-1 簇重写为单一的共享方法。 | ⏳ 在 [`docs/plans/autofix-extract-method-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/autofix-extract-method-plan.md) 中跟踪；受阻于 Type-1/Type-2 分桶拆分 (gh #42) |
| AI 辅助抽取（MCP `extract-method-plan` / `-apply`） | 在智能体协助下进行 Type-2/3 抽取。 | ⏳ 在 [`docs/plans/autofix-extract-ai-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/autofix-extract-ai-plan.md) 中跟踪；受阻于 Type-1 路径的落地 |

## 已实现的流水线

批处理 CLI 和实时服务都通过 `PipelineSession` 运行。`crates/deslop-core/src/pipeline/run.rs` 中的批处理入口委托给 `PipelineSession::initialise`，当前语料由 `crates/deslop-core/src/pipeline/session.rs` 中的 `PipelineSession::render` 渲染。

### 1. 文件发现

`crates/deslop-core/src/discover.rs::discover_files` 使用 `ignore` crate 遍历目标根目录，应用标准的忽略过滤器，不跟随符号链接，按已注册的语言扩展名过滤，应用 `.deslop.toml` 排除规则，并将存活下来的文件注册到 `FileRegistry` 中。

当前由 `crates/deslop-core/src/pipeline/corpus.rs::default_parsers` 注册的受支持语言解析器有：C#、Rust、Python、Dart、JavaScript、TypeScript、TSX、PHP、F# 和 Go。Java 与 C/C++ 尚未在当前核心流水线中注册。

### 2. 解析与归一化

每个语言解析器都通过 `crates/deslop-core/src/lang/shared.rs` 中的共享机制使用 tree-sitter。`build_normalised_root` 遍历命名的 tree-sitter 节点，生成一棵以 `__file__` 为根的 `NormalizedNode` 树。

归一化是语言相关的：

- C#：`crates/deslop-core/src/lang/csharp.rs`
- Rust：`crates/deslop-core/src/lang/rust_lang.rs`
- Python：`crates/deslop-core/src/lang/python.rs`
- Dart：`crates/deslop-core/src/lang/dart.rs`
- JavaScript：`crates/deslop-core/src/lang/javascript.rs`
- TypeScript / TSX：`crates/deslop-core/src/lang/typescript.rs`
- PHP：`crates/deslop-core/src/lang/php.rs`
- F#：`crates/deslop-core/src/lang/fsharp.rs`
- Go：`crates/deslop-core/src/lang/go.rs`

共享常量是用于类标识符节点的 `__ident__` 和用于类字面量节点的 `__literal__`。注释和琐碎内容通过让语言相关的归一化器返回 `None` 而被丢弃。这正是使被重命名的 Type-2 克隆能够哈希到一起的机制。

### 3. 样板抑制

import 及类前导结构会在发出克隆指纹之前被抑制。该过滤器位于 `crates/deslop-core/src/boilerplate.rs`，而结构与兄弟收集器从 `crates/deslop-core/src/fingerprint.rs` 和 `crates/deslop-core/src/sibling.rs` 中调用它。

这对生成式和 AI 辅助的仓库很重要，因为重复的 import、namespace 包装和前导脚手架原本可能主导候选集。报告仍通过 `Report.boilerplate_hints` 暴露样板提示。

### 4. Merkle 结构指纹

`crates/deslop-core/src/fingerprint.rs::collect_non_boilerplate_fingerprints` 自底向上遍历每棵归一化的 AST。对于每个节点数至少为 `--min-nodes` 的子树，它会发出：

- 对归一化节点种类与子哈希的 BLAKE3 Merkle 哈希，
- 源文件 id，
- 精确的字节范围，
- 子树节点数。

共享 Merkle 哈希的配对以 `structural = 1.0` 进入候选集。实现内部使用字节范围；行号和列号标签是渲染时的投影。

### 5. 兄弟窗口指纹

`crates/deslop-core/src/sibling.rs::collect_non_boilerplate_sibling_fingerprints` 在宽度 2 到 8 的连续兄弟窗口的合计节点数满足 `--min-nodes` 时为其发出指纹。合成哈希前缀为 `__sibling_window__`。

这是首个 Type-3 扩展：两个方法可能在周围结构上存在差异，却仍共享一段重复的语句序列。

### 6. Token MinHash 与 LSH

对于每个指纹，`crates/deslop-core/src/tokens.rs` 提取归一化节点种类的前序流并构建 5 宽 k-gram。随后 `crates/deslop-core/src/lsh.rs` 构建 128 值 MinHash 签名，分成 32 带、每带 4 行。

`band_collisions` 返回签名在至少一个带中碰撞的候选配对。Jaccard 相似度由 `estimate_jaccard` 根据完整签名的一致度来估计。LSH 桶使用星型拓扑而非完整的 N 平方枚举，从而使大型重复桶保持可处理。

### 7. 可选的嵌入

嵌入由 `crates/deslop-core/src/embedding/mode.rs` 中的 `EmbeddingMode` 控制，并由 `crates/deslop-core/src/pipeline/embedding_pass.rs` 执行。

当前 CLI 的行为很重要：

- `crates/deslop/src/main.rs` 将 `--embeddings` 默认设置为 `off`。
- `auto` 会探测 provider，若 provider 不可用则在不使用嵌入的情况下继续。
- `required` 会传播 provider 故障，使 CLI 以非零退出码退出。
- 默认 provider 键为 `ollama`。
- `crates/deslop-core/src/embedding/ollama.rs` 将默认端点设置为 `http://127.0.0.1:11434`，默认模型设置为 `nomic-embed-text`。

当嵌入运行时，片段会缓存于 `.deslop-cache/embeddings/...` 下；HNSW 最近邻搜索在 `crates/deslop-core/src/embedding/pairs.rs` 中实现。配对生成器使用 top-k 邻居和 0.80 的最小余弦阈值。

### 8. 配对融合与聚簇

`crates/deslop-core/src/pair.rs::candidate_pairs` 对三个来源取并集：

- 结构哈希桶配对，
- LSH 带碰撞配对，
- 嵌入最近邻配对。

每个配对携带：

- `structural`，
- `token_jaccard`，
- `embedding_cos`，
- `fused`。

`PairScore::fused` 对这三个信号求和并将结果钳制到 `[0.0, 1.0]`。`FUSED_THRESHOLD` 为 `0.85`。仅 LSH 配对有额外的守卫：`token_jaccard >= 0.90` 且两个端点都必须至少有 40 个 AST 节点。

跨语言比较默认关闭。`crates/deslop-core/src/config.rs` 将 `allow_cross_language_comparison` 初始化为 `false`，且 `crates/deslop-core/src/pair.rs::candidate_pairs_for_language_policy` 会丢弃跨语言配对，除非配置启用它们。

`cluster_by_transitive_closure` 从存活的候选配对中形成连通分量。这意味着 A 匹配 B 且 B 匹配 C 可以产生一个簇，即便 A 与 C 并未直接配对。

### 9. 排名

簇在 `crates/deslop-core/src/cluster.rs` 中被物化并排序。同一文件中重叠的出现位置会在报告输出前被合并。排名公式为：

```text
weight = clone_node_count * (cluster_size - 1) * log2(1 + spanned_bytes)
```

这不是一个 LOC 公式。LOC 用于仓库度量，但排名权重当前使用节点数、簇大小和跨越字节数。

### 10. 报告渲染

规范报告是 `crates/deslop-core/src/report.rs` 中的 `Report`。`render_report` 应用 `report_hide`、计算仓库度量、纳入来自 `docs/specs/REPORTING-CONTEXT.md` 的内嵌 schema 文档，并附上动作提示和嵌入溯源。

仓库级度量在 `crates/deslop-core/src/report_metrics.rs` 中计算。`duplicated_loc` 由未隐藏的克隆出现位置行范围派生，并按文件去重，使重叠的兄弟窗口范围不会膨胀分子。

JSON 报告是规范输出。文本和 HTML 渲染器是基于该报告的派生视图。

## 增量与实时分析

同一个 `PipelineSession` 同时拥有完整分析与增量分析的状态。`PipelineSession::update_files` 重新解析或丢弃发生变化的文件，然后在当前内存语料上重新运行签名构建、LSH、可选嵌入、配对融合、聚簇和报告渲染。

磁盘上的指纹缓存在 CLI 中是可选启用的。`crates/deslop/src/main.rs` 暴露了 `--incremental`；当其被设置时，`crates/deslop-core/src/fpcache.rs` 会以语言、工具版本、`min_nodes` 和内容哈希为键，将归一化树和指纹存储于 `.deslop-cache/fingerprints/...` 下。

实时分析在 `crates/deslop-core/src/live/` 下实现：

- `session.rs` 拥有实时的 `AnalysisSession`。
- `scheduler.rs` 串行化经过防抖的文件变更工作。
- `debouncer.rs` 以静默窗口与硬上限合并一连串文件变更事件。
- `watcher.rs` 按解析器扩展名和排除项过滤文件系统事件。
- `api.rs` 暴露报告、范围、簇、嵌入和配置操作。

`crates/deslop-lsp/src/backend.rs` 中的 LSP 服务器包装了 `LiveService`，并暴露诊断、悬停、code lens 以及自定义的 `deslop/*` 方法。LSP 嵌入以 `EmbeddingMode::Off` 启动，除非由客户端显式配置。

`crates/deslop-mcp/src/` 中的 MCP 服务器通过 stdio 暴露 JSON-RPC 工具，并用 `crates/deslop-mcp/src/safety.rs::resolve_within_root` 保护文件系统输入。每个读取工具——`find-similar`、`top-offenders`、`report-get`、`report-for-file` 和 `report-for-range`——都通过本地 IPC 端点转发到 LSP 的实时 `AnalysisSession`，因此每个答案都是针对运行中的语料而非陈旧的状态文件运行的。Unix 主机使用 `.deslop-cache/deslop.sock`；Windows 使用从 `.deslop-cache/deslop.port` 发现的 token 门控 TCP 回环端点。MCP 自身不保留任何磁盘缓存；它持有一个到该端点的 `report/subscribe` 连接，并在 LSP 广播报告变更时向其客户端推送 `resources/updated` + `deslop/reportChanged`。

## 审计者验证映射

| 主张 | 在代码中验证 | 有用的测试 |
| --- | --- | --- |
| C#、Rust、Python、Dart、JavaScript、TypeScript、TSX、PHP、F# 和 Go 目前已注册。 | `crates/deslop-core/src/pipeline/corpus.rs::default_parsers` | `cargo test -p deslop --test js_ts_signatures`、`cargo test -p deslop --test cli detects_type2_clone_in_csharp_fixture`、`cargo test -p deslop --test cli detects_type2_clone_in_php_fixture`、`cargo test -p deslop --test cli detects_type2_clone_in_fsharp_fixture`、`cargo test -p deslop --test cli detects_type2_clone_in_go_fixture` |
| 已注册的语言集合能够抵达编辑器界面。 | `clients/vscode/package.json` 激活事件、`clients/vscode/src/types/languages.ts` | `cargo test -p deslop-core --test lang_registry_vsix_parity` |
| Type-2 归一化折叠标识符和字面量。 | `crates/deslop-core/src/lang/shared.rs`、语言解析器文件 | `cargo test -p deslop --test cli debug_ast_dump_matches_committed_golden` |
| 结构克隆是 BLAKE3 Merkle 子树哈希。 | `crates/deslop-core/src/fingerprint.rs` | `cargo test -p deslop --test sibling_dedup` |
| Type-3 召回使用兄弟窗口和 MinHash LSH。 | `crates/deslop-core/src/sibling.rs`、`crates/deslop-core/src/tokens.rs`、`crates/deslop-core/src/lsh.rs` | `cargo test -p deslop --test sibling_ranking` |
| 嵌入是可选的，且在 CLI 中默认关闭。 | `crates/deslop/src/main.rs`、`crates/deslop-core/src/pipeline/embedding_pass.rs` | `cargo test -p deslop --test cli default_run_records_embeddings_off_provenance` |
| 嵌入邻居按 HNSW 余弦阈值过滤。 | `crates/deslop-core/src/embedding/pairs.rs` | `cargo test -p deslop-core --test embedding_pairs` |
| 融合评分是有界的。 | `crates/deslop-core/src/pair.rs::PairScore::fused` | `cargo test -p deslop --test fused_score_bounds` |
| 除非配置，否则跨语言比较被禁用。 | `crates/deslop-core/src/config.rs`、`crates/deslop-core/src/pair.rs::candidate_pairs_for_language_policy` | `cargo test -p deslop --test cross_language` |
| 实时/LSP 路径在 `PipelineSession` 之上使用 `LiveService`。 | `crates/deslop-core/src/live/`、`crates/deslop-lsp/src/backend.rs` | `cargo test -p deslop-lsp --test notifications` |
| MCP 工具是带根目录安全检查的 stdio JSON-RPC 包装器。 | `crates/deslop-mcp/src/server.rs`、`crates/deslop-mcp/src/tools/mod.rs`、`crates/deslop-mcp/src/safety.rs` | `cargo test -p deslop-mcp --test cli` |

要进行广泛的本地审计，运行仓库的 CI 目标：

```bash
make ci
```

要进行有针对性的代码评审，下列搜索能展示核心算法路径：

```bash
rg -n "fn render|candidate_pairs_for_language_policy|cluster_by_transitive_closure|build_ranked_fused_clusters|render_report" crates/deslop-core/src
rg -n "default_parsers|EmbeddingMode::Off|DEFAULT_OLLAMA_MODEL|rank_weight" crates/deslop-core/src crates/deslop/src
rg -n "find_similar_snippet|find_similar_for_snippet|resolve_within_root" crates/deslop-mcp/src crates/deslop-core/src/live
```

## 参考文献

- Baxter, Yahin, Moura, Sant'Anna, and Bier, *Clone Detection Using Abstract Syntax Trees*, 1998. [PDF](https://leodemoura.github.io/files/ICSM98.pdf)
- Chilowicz, Duris, and Roussel, *Syntax Tree Fingerprinting for Source Code Similarity Detection*, 2009. [PDF](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- Sajnani, Saini, Svajlenko, Roy, and Lopes, *SourcererCC: Scaling Code Clone Detection to Big Code*, 2016. [arXiv](https://arxiv.org/abs/1512.06448)
- Roy and Cordy, *NICAD: Accurate Detection of Near-Miss Intentional Clones Using Flexible Pretty-Printing and Code Normalization*, 2008. [ResearchGate entry](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)
- Shrivastava and Li, *Densifying One Permutation Hashing via Rotation for Fast Near Neighbor Search*, 2014. [PDF](http://proceedings.mlr.press/v33/shrivastava14.pdf)
- Roy, Alam, Al-omari, Roy, Roy, and Schneider, *Unveiling the potential of large language models in generating semantic and cross-language clones*, 2023. [arXiv](https://arxiv.org/abs/2309.06424)
- Roy et al., *GPTCloneBench: A comprehensive benchmark of semantic clones and cross-language clones using GPT-3 model and SemanticCloneBench*, 2023. [arXiv](https://arxiv.org/abs/2308.13963)
- Eagal, Stolee, and Ore, *Analyzing the dependability of Large Language Models for code clone generation*, 2025. [Journal of Systems and Software](https://www.sciencedirect.com/science/article/pii/S0164121225002171)
- Liu et al., *Code Copycat Conundrum: Demystifying Repetition in LLM-based Code Generation*, 2025. [arXiv](https://arxiv.org/abs/2504.12608)
- Alam et al., *Are Classical Clone Detectors Good Enough For the AI Era?*, 2025. [arXiv](https://arxiv.org/abs/2509.25754)
- Wu et al., *An Empirical Study of Code Clones from Commercial AI Code Generators*, 2025. [FSE 2025 entry](https://conf.researchr.org/details/fse-2025/fse-2025-research-papers/111/An-Empirical-Study-of-Code-Clones-from-Commercial-AI-Code-Generators)
- Liu, Widyasari, Zhao, Irsan, and Lo, *Debt Behind the AI Boom: A Large-Scale Empirical Study of AI-Generated Code in the Wild*, 2026. [arXiv](https://arxiv.org/abs/2603.28592)
