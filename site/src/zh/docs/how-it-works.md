---
layout: layouts/docs.njk
title: 工作原理 — tree-sitter AST、MinHash LSH、HNSW 嵌入
description: Deslop 的流水线 —— tree-sitter 解析、AST 归一化、Merkle 指纹、MinHash + LSH、HNSW 嵌入、融合 0.85 阈值、最严重者优先的排名。250 ms 响应式循环。
eleventyNavigation:
  key: 工作原理
  order: 2
icon: account_tree
lang: zh
---

# 工作原理

Deslop 是一条固定的、确定性的流水线。没有任何步骤在源代码上使用正则。每个步骤都以缓存键标识，因此未变更的文件会被跳过。每个阶段的输出都很小、结构化且可审计。

```
discover → parse → normalize → fingerprint → cluster
           → LSH → embed → fuse → rank → render
```

每个阶段都对应一条研究脉络；文件指针见[研究背景](/zh/docs/research-background/)以及规范的[算法实现状态表](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/SPEC.md#algorithm-implementation-status)。

## Discover（发现）

`.gitignore` 会被遵循。只有扩展名映射到受支持语言语法的文件才会被分析 —— 其余一切，包括二进制文件，都会被跳过。不跟随符号链接。每个候选文件的内容都用 BLAKE3 进行哈希，该哈希是每个阶段所用复合缓存键的一个组成部分，因此未变更的文件会被跳过。

## Parse（解析）

每种语言都通过 tree-sitter 提供一份语法：

| 语言 | 状态 |
| --- | --- |
| C# | v1 |
| Rust | v1 |
| Python | v1 |
| Dart | v1 |
| TypeScript / JavaScript | 路线图 |
| Go | 路线图 |

解析器产出一棵 AST。这条流水线上从不会有任何源代码级别的正则参与 —— 永远不会。

## Normalize（归一化）

相同的代码可能仅在标识符和字面量上有所不同（Type-2 重命名）。Deslop 会剥除：

- 标识符名称（重写为 `__ident__`）
- 字符串 / 数字 / 字符字面量（重写为 `__literal__`）
- 注释、空白、琐碎字符

按语言定制的归一化规则，跨语言保持完全一致的输出格式。一个方法的重命名副本会哈希出与原始版本相同的指纹。

## Fingerprint（指纹）

每棵节点数 ≥ `--min-nodes` 的子树都会获得一个**自底向上的 BLAKE3 Merkle 哈希**，它将节点类型与其子节点的有序哈希组合在一起。这是 Chilowicz 2009 的语法树指纹技术，应用于 tree-sitter AST。第二趟 —— 宽度为 2 到 8 的兄弟窗口指纹 —— 通过对父节点之间不共享结构的连续语句序列进行哈希，扩展了**近乎相同代码** [Type-3] 的召回率（`crates/deslop-core/src/sibling.rs`）。子树以字节范围发出 —— 行号是渲染期才关心的事项。磁盘上的缓存以 `(content_hash, language, tool_version, min_nodes)` 为键。

## Cluster（聚类）

跨文件或同一文件内相同的 Merkle 哈希会立即构成一个**相同代码**簇（Type-1 / Type-2）。这一趟是 O(n) 的，无需任何近似匹配即可找出代价最高的重复。来自 LSH 与嵌入趟次的幸存配对会被并入，并通过**传递闭包**进行聚类（[`crates/deslop-core/src/cluster.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs)）—— 即使 A 与 C 从未直接配对，A↔B 和 B↔C 也会产生同一个簇。

## LSH（近似匹配）

对于**近乎相同的代码**（Type-3，结构相似但不完全相同），Deslop 为每棵子树构建一条宽度为 5 的归一化 AST 类型 k-gram 流，计算出一个 **128 值的 MinHash 签名**（Broder 1997），并将它们分组为 **32 个频段、每段 4 行**，用于 [Indyk-Motwani 局部敏感哈希](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)。候选配对就是发生碰撞的频段；随后从完整签名的一致程度估计 Jaccard。SourcererCC 的词袋设计是灵感来源，但 Deslop 在归一化的 AST 类型而非原始源代码 token 上运行其 k-gram。实现位于 [`crates/deslop-core/src/lsh.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/lsh.rs) 与 [`crates/deslop-core/src/tokens.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/tokens.rs)。

## Embed（语义）

可选，**默认关闭** —— 通过 `--embeddings auto`（探测并在失败时附带警告回退）或 `--embeddings required`（若提供方不可达则硬失败）来启用。启用后，每棵子树都会经过一个代码嵌入模型（默认使用本地 Ollama —— 开箱即用 `nomic-embed-text`，可通过 `--embedding-model` 选用任意 Ollama 嵌入模型）。最近邻搜索在一个 **HNSW** 索引上运行（`instant-distance`，纯 Rust，确定性种子），余弦阈值定义于 [`crates/deslop-core/src/embedding/pairs.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/embedding/pairs.rs)。这会产出**行为相同、代码不同**的候选（Type-4）—— 语义等价但语法不同的代码，例如命令式循环与 LINQ 表达式之别。SSCD（Wiley 2024）验证了 HNSW + ANN 是规模化场景下正确的召回层；Deslop 采用相同的形态，并按 [`fusion.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/fusion.md) 将其与结构趟次和 LSH 趟次配对。

嵌入缓存以 `(content_hash, provider_id, model_id, model_version)` 为键，因此切换模型只会使嵌入层失效 —— 结构与 LSH 缓存得以保留。

## Fuse（融合）

每个候选配对都会获得三个独立的评分：

| 信号 | 范围 | 检测 | 来源 |
| --- | --- | --- | --- |
| `structural` | 0 / 1 | 相同代码 [Type-1/2] —— 完全相同的 Merkle 桶 | `pair.rs::collect_structural_pairs` |
| `token_jaccard` | 0..1 | 近乎相同的代码 [Type-3] —— MinHash 频段碰撞 | `lsh.rs::band_collisions` + `tokens.rs` |
| `embedding_cos` | 0..1 | 行为相同、代码不同 [Type-3/4] —— HNSW top-k | `embedding/pairs.rs` |

依据集成式 LLM 2025 的发现（求平均有害；求和/取最大有益），融合得分为 `clamp(structural + token_jaccard + embedding_cos, 0, 1)`（[`pair.rs::PairScore::fused`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs)）。当融合得分越过 `FUSED_THRESHOLD = 0.85` 时配对得以幸存。仅由 LSH 产生的配对还要承受一道更严格的信息含量下限（`token_jaccard ≥ 0.90` 且两端点均 ≥ 40 个 AST 节点），这样嘈杂的近似匹配就无法搭着 LSH 的便车混入簇中。除非 `.deslop.toml` 明确启用，否则跨语言配对会被丢弃。

## Rank（排名）

排名评分就是整个面向用户的产品。其实现位于 [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs)：

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

更大的片段权重更高（`clone_node_count`）。副本越多权重越高（`cluster_size − 1`，因此单成员簇得分为零）。`log2(1 + spanned_bytes)` 项随收益增长，但对极大跨度趋于平缓，因此一个被复制四次的 50 行方法的排名高于一个被复制一次的 5000 行文件。报告顶部永远是收益最大者 —— 而非最先找到的簇。

## Render（渲染）

三个渲染器读取同一份物化视图：

- **JSON** —— 规范且严格类型化。携带内嵌的 `schema_doc`、`action_hints`、覆盖全仓库的 `metrics` 以及 `embedding_provenance`。
- **TXT** —— ASCII、面向行、无 ANSI。可通过管道送入 `head`、`grep`、`awk`。
- **HTML** —— 独立、内联 CSS、零网络依赖。它嵌入源代码片段并采用 tree-sitter 驱动的语法高亮；当某文件的源代码不再可用时，卡片会回退为仅含路径、不带片段的摘要。

智能体消费 JSON。人类在终端阅读 TXT，或在浏览器中打开 HTML。TXT 或 HTML 所作的每一项陈述也都呈现在 JSON 中。

## 实时 = 响应式

上述全部内容也会在 LSP 服务器内增量运行（`crates/deslop-core/src/live/`）。

文件监视器对编辑进行批处理（250 ms 防抖，2 s 上限），并通过 `PipelineSession::update_files` 重新运行流水线。最新报告被保留在内存中，随后 LSP 会：

- 在 LSP 线路上广播 `deslop/reportChanged`，并且
- 通过本地 IPC 端点提供运行中的语料，使得捆绑的 MCP 服务器无需重新解析即可应答 `find-similar`。macOS 与 Linux 使用 `.deslop-cache/deslop.sock`；Windows 使用通过 `.deslop-cache/deslop.port` 发现的 token 门控 TCP 回环端点。

`.deslop-cache/live-report.json` 仅作为冷启动种子写入 —— 以便刚启动的 LSP 能在其首趟扫描运行期间应答查询 —— 而非在每次编辑时写入。

每个 VS Code 界面 —— 气泡、Top Offenders 树、状态栏、悬停、代码透镜 —— 以及每个智能体 MCP 查询都从那同一份内存中报告读取。CLI 则是 CI 门禁的冷缓存回退。
