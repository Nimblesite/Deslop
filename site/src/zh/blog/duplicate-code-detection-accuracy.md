---
layout: layouts/blog.njk
title: "重复代码检测：0.34.0 之后的准确性改进"
date: 2026-09-27
author: Christian Findlay
lang: zh
tags:
  - posts
  - duplicate-code-detection
  - code-quality
  - technical-debt
category: engineering
description: "Deslop 下一版本聚焦误报、漏报与不完整的克隆范围，并提供更清晰的比较和重复度指标。"
excerpt: "找到配对只是准确性的一部分。检测器还需要报告完整副本，排除无关的相似外形，并统计正确的源代码行。"
heroImage: "/assets/img/blog/towards-100-percent-accuracy-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "绘图桌上的放大镜与精密测量工具。"
ogImage: "/assets/img/blog/towards-100-percent-accuracy-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
---

重复代码检测器的价值在于找到值得检查的副本。误报浪费审查时间；漏报让同一个缺陷留在另一处；不完整的结果则可能认出了副本，却隐藏了大部分复制内容。

Deslop 的 `main` 分支在 **0.34.0** 之后针对这三类问题做了改动。这些是下一次安装包发布的准备工作，不表示现有 0.34.0 下载已包含它们。可用安装包见[版本发布](/zh/releases/)。

## 找到完整副本

一个缺陷会把较长的精确副本拆成多个短小、重叠的结果。仅知道两个文件共享代码并不够，审查者需要完整的复制范围。

[范围恢复改动](https://github.com/Nimblesite/Deslop/pull/570)只在完整源范围逐字节一致、且符合连续语法树兄弟节点边界时合并精确结果。无法合法合并时，已复制的窗口仍须可见，不能被丢弃。对应的 CLI 测试断言完整复制文本、两个文件路径、恰好两个出现位置、类别和排名。

其他[召回改动](https://github.com/Nimblesite/Deslop/pull/567)找回经过编辑的方法中的公共初始化代码，以及同一文件内的副本。范围边界同样重要：报告公共初始化时，不能把无关的相邻方法也拉进来。

## 区分重命名副本与无关的相似外形

变量重命名或测试消息变化，不一定改变被复制的工作。这些情况与小幅编辑归入 **Nearly identical code**。仍有大量复制内容但改动较大的情况归入 **Similar code**。

只有布局相同，证据要弱得多。**Same shape, different content** 仅供参考，不贡献克隆数、重复质量或重复百分比。反复出现的控制流骨架不足以认定技术债务。

内容检查区分一致的重命名与操作变化。重命名协作对象可能保留副本关系；让它执行另一种操作，则需要不同的证据。**Identical code** 要求源文本在空白规则下相同，而不是仅有归一化语法树相同。[分类指南](/zh/docs/how-it-works/)解释这些标签与 Type I、II、III、IV 的关系。

## 让每次比较都可检查

相似度测量现在属于实际比较的两个位置，不再把某一对的评分当作整个大型簇的置信度。

VS Code 中的 **Compare To Canonical** 与参考副本比较；**Select for Compare** 和 **Compare with Selected** 可选择其他配对。CLI 新增 `--compare`，返回两个明确源范围的引擎判定。质量用于排列审查优先级，不证明提取安全。

构建产物也会根据已识别的名称和行形态被排除；生成文件头则从解析后的注释读取。这些检查让分析聚焦源码，而不会把字符串中的相似标记误认为生成文件声明。

## 准确性不能只看配对数量

克隆登记表包含必须找到的 **CLEARLY IN** 配对，以及不能作为克隆报告的 **CLEARLY OUT** 配对。不确定样本只记录，不强行判定。版本比较固定仓库提交，并考虑旧类别名称：旧版仅形状匹配不能满足已知克隆断言。

我们还检查**克隆范围覆盖率**：报告覆盖了已判定复制范围的多少部分。每个副本只找到一小段，也可能在配对正确性上看起来不错，却仍漏掉用户需要看到的代码。

仓库重复百分比回答另一个问题：报告中的克隆覆盖了已分析代码行的多大比例？它不是准确率。重叠范围只计一次，仅形状匹配计零。[准确性透明度](/zh/docs/accuracy-transparency/)记录计算方式和语料库测量的适用范围。

这些改动让具体缺陷可以被测试，让比较更容易检查。静态代码分析工具应提供这样的证据：排除了哪些误报，找回了哪些漏报，报告是否展示了审查者真正需要的代码。
