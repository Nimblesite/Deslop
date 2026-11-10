---
layout: layouts/blog.njk
title: "如何查找 C 语言中的重复代码"
date: 2026-11-08
author: Christian Findlay
tags:
  - posts
  - c
  - duplicate-code
  - ai-generated-code
category: engineering
description: "在 .c 和 .h 文件中查找重复函数和复制的清理代码，并在 VS Code 或命令行报告中查看结果。"
excerpt: "两段 C 代码共用一份复制来的错误处理时，只修一处，另一处仍可能泄漏文件句柄。Deslop 现在可以帮你找到这两处代码。"
heroImage: "/assets/img/blog/c-structural-dedup.webp"
heroImageWidth: "1600"
heroImageHeight: "840"
heroImageAlt: "两个匹配的石墨色组件在带有猩红色点缀的炭黑色表面上合为一个银色组件。"
ogImage: "/assets/img/blog/c-structural-dedup-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "重复的石墨色组件合并为一个银色组件，采用 Deslop 的炭黑色与猩红色。"
lang: zh
---

在一个 C 文件中修好了漏洞，另一个文件里可能还留着同样的问题。假设两个函数都要打开文件、分配缓冲区并读取数据。你给第一个函数的读取不足分支补上了 `fclose`，另一个函数却仍会泄漏文件句柄。

Deslop 现在可以在 `.c` 和 `.h` 文件中查找重复代码。它能找到重复的函数，也能找到函数里复制的小段代码，即使变量已经改名或常量已经改动。

## 在 VS Code 中查找重复的 C 错误处理

在一个包含两个文件的示例中，音频加载器和数据帧读取器都会分配缓冲区，再从文件中读取数据。Deslop 找到了这两个文件中相似的错误处理。打开数据帧读取器后，还可以看到另一处值得检查的重复。读取不足的分支和正常关闭函数都会释放缓冲区、关闭文件，再释放外层结构体。

在 VS Code 中对照两条路径，先确认各自拥有和释放哪些资源。如果适合提取公共的清理函数，就保留每个调用方原有的日志和返回值。测试正常读取和读取不足两种情况，然后再扫描一次。

## 扫描会报告什么

重复的头文件引用、函数声明和普通循环头在 C 中很常见。Deslop 不会仅凭这些内容报告重复，也不会比较宏定义体。如果生成的文件占据了报告的大部分，可以[配置报告显示哪些结果](/zh/docs/configuration/)。

## 扫描你的 C 项目

[安装 Deslop](/zh/docs/) 后，在项目根目录运行：

```bash
deslop .
```

打开 HTML 报告，或者通过 VS Code 扩展跳转到各处副本。先看排名最高的结果，判断这些代码是否应该一起修复。
