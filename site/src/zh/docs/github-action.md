---
layout: layouts/docs.njk
title: GitHub Action — 在 CI 中为重复代码设置门禁
description: Deslop.live GitHub Action 的完整配置 — 每一个输入与输出、退出码约定、阈值优先级、只度量不拦截、受支持的运行器、产物处理，以及固定的标签如何决定所安装的 CLI 版本。
keywords: deslop, github action, ci 门禁, 重复代码, fail-over, 重复率阈值, 代码质量, 持续集成
eleventyNavigation:
  key: GitHub Action
  order: 7
icon: rule
docsGroup: guides
lang: zh
---

# GitHub Action

[GitHub Marketplace 上的 **Deslop.live**](https://github.com/marketplace/actions/deslop-live) 会在运行器上安装已发布的 `deslop` CLI，分析工作区，渲染报告，并在重复率突破上限时让作业失败。

它是一个复合（composite）Action。没有镜像需要拉取，也没有安装后的额外下载 — 它会获取与运行器匹配的预编译归档，在解压之前校验已发布的 SHA-256，然后把 `deslop` 放到 `PATH` 上。

## 快速开始

```yaml
name: deslop
on: [push, pull_request]

jobs:
  duplication-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        with:
          fail-over: "5.0"   # 或省略以使用 .deslop.toml 中的 [threshold]
```

该 Action 不需要令牌，除默认的 `contents: read` 之外不需要任何权限。

## 版本固定与 CLI 版本

**你固定的标签就是你得到的 CLI 版本。** `version` 输入默认取 `github.action_ref` 并去掉开头的 `v`，因此 `uses: Nimblesite/Deslop@v{{ releases.pin }}` 会安装同一个 `deslop` 版本。两者不可能产生偏移。

请固定到确切版本而不是可变引用 — Dependabot 会替你升级。这里刻意**没有 `@v1` 别名**：可变的主版本标签正是版本固定要避免的供应链形态。本页每段示例中的版本号{% if releases.latest %}都是最新发布版 **{{ releases.pin }}**{% else %}都指向最新发布版{% endif %} — 它在页面构建时解析，从不写入仓库，因此不可能落后于它所安装的 CLI。

如果你固定到某个提交 SHA 或分支，该引用不携带版本，因此 `version` 变为**必填**。缺失它是一个明确指出修复方式的硬错误，绝不会静默回退到「latest」：

```yaml
      - uses: Nimblesite/Deslop@8f4c1e2a9b7d3f6a5c8e1b4d7a0f3c6e9b2d5a8f
        with:
          version: "{{ releases.pin }}"
```

## 输入

| 输入 | 默认值 | 用途 |
| --- | --- | --- |
| `path` | `.` | 要分析的目录 |
| `version` | 固定的标签 | 要安装的 CLI 版本。固定到提交 SHA 时必填 |
| `fail-over` | *(未设置)* | 超过该百分比则作业失败。未设置时遵循 `.deslop.toml` |
| `no-fail-over` | `false` | 为本次运行清除已配置的阈值 |
| `min-nodes` | `30` | 克隆候选的最小 AST 子树节点数 |
| `config` | *(未设置)* | 显式指定 `.deslop.toml` 路径 |
| `diff` | *(未设置)* | 以其新增行限定报告范围的统一 diff — 补丁文件路径。仍会分析整棵树；diff 只限定报告内容，绝不限定扫描范围。`-`（标准输入）仅适用于 CLI：复合 action 无法提供标准输入，因此 Action 会拒绝该值，而不是悄悄地度量一个空 diff |
| `only-changed` | `false` | 只报告 diff 触及的簇，并按 diff 范围的百分比设门禁 — 重复的新增行数除以新增行数 — 使存量债务无法让合并前检查失败。需要 `diff` |
| `cache` | `true` | 通过 Actions 缓存在运行之间保留解析存储，预热运行只重新解析有变化的文件 |
| `output` | `deslop-report` | 报告路径前缀；会追加 `.json`、`.txt`、`.html` |
| `nojson` / `notext` / `nohtml` | `false` | 抑制某种输出格式 |
| `log-level` | `info` | `error`、`warn`、`info`、`debug` 或 `trace` |
| `upload-artifact` | `true` | 上传渲染出的报告 |
| `artifact-name` | `deslop-report` | 上传产物的名称 |

## 输出

| 输出 | 含义 |
| --- | --- |
| `duplication-percent` | 重复行数占已分析行数的百分比 |
| `cluster-count` | 报告正文中的簇数量 —— 在 `only-changed` 下为经过筛选后保留的、受 diff 影响的簇 |
| `threshold-percent` | 本次运行所对照的上限 |
| `exit-code` | `0` 成功、`1` 运行时错误、`2` 用法错误、`3` 突破阈值 |
| `report-json` / `report-text` / `report-html` | 渲染出的报告路径 |
| `gate-scope` | 门禁所度量的总体 —— 在 `only-changed` 下为 `added-lines`，否则为 `repository` |
| `gate-percent` | 门禁与其上限相比较的百分比，覆盖 `gate-scope` 所指的总体 |
| `gate-threshold-percent` | `gate-percent` 所对照的上限 |

**即使门禁被触发，输出仍会发布**，因此后续步骤可以在评论中发布该数值，或逐步收紧预算。设置 `nojson: true` 会让它们为空 — 它们是从 JSON 报告中读取的。

## 阈值优先级

`fail-over` 优先于 [`.deslop.toml`](/zh/docs/configuration/) 中的 `[threshold] max_duplication_percent`。不设置它即遵循配置文件，对于已经把自身上限提交进仓库的项目，这是更好的默认。

- `fail-over: "0"` 对**任何**重复都失败。
- `no-fail-over: "true"` 为本次运行清除已配置的上限，因此作业只度量、永不失败。它与 `fail-over` 互斥。
- 百分比必须是 `[0.0, 100.0]` 区间内的有限数。其他任何值都会以 `2` 退出。

## 只度量，不拦截

在每个 PR 上报告数字并交由人来判断，而不是阻塞合并：

```yaml
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        id: deslop
        with:
          no-fail-over: "true"   # 只度量，不拦截
      - run: echo "{% raw %}${{ steps.deslop.outputs.duplication-percent }}{% endraw %}% 重复"
```

## 退出码

该 Action **如实呈现** CLI 的状态；它绝不重新解释。

| 代码 | 含义 |
| --- | --- |
| `0` | 分析成功，且重复率在阈值之内（或未设置阈值）。 |
| `1` | 运行时错误 — 扫描路径错误、解析/IO 失败，或 `required` 的嵌入提供方不可达。绝不是 panic。 |
| `2` | 用法错误 — 未知参数，或超出范围/非有限的阈值。 |
| `3` | **重复率突破阈值。** 报告仍会完整写出，以便 CI 呈现最严重的问题。 |

突破阈值会让步骤失败，并给出指明实测百分比与上限的消息。`1` 与 `2` 会给出各自不同的消息，因此配置错误绝不会被误认为重复率突破。

## 报告与产物

默认情况下，该 Action 会写出 `deslop-report.json`、`deslop-report.txt` 和 `deslop-report.html`，并把三者作为名为 `deslop-report` 的工作流产物上传。

```yaml
      - uses: Nimblesite/Deslop@v{{ releases.pin }}
        with:
          output: reports/duplication
          artifact-name: duplication-reports
          nohtml: "true"        # 仅 JSON + 文本
```

HTML 报告是给人看的；JSON 报告是用来解析的。各自的结构见[报告输出](/zh/docs/configuration/#report-output)。

## 受支持的运行器

| `runner.os` | `runner.arch` | 发布产物 |
| --- | --- | --- |
| `Linux` | `X64` | `linux-x64` |
| `Linux` | `ARM64` | `linux-arm64` |
| `macOS` | `X64` | `macos-x64` |
| `macOS` | `ARM64` | `macos-arm64` |
| `Windows` | `X64` | `windows-x64` |

其他任何组合都是明确指出该组合的硬错误。没有 Windows ARM64 构建。

## 供应链说明

- 归档及其已发布的 `.sha256` 附属文件都会被下载，并且**在解压任何内容之前**校验摘要。不匹配即中止作业。
- 每个输入都通过 `env` 到达其脚本，绝不插值进 shell 命令体，因此精心构造的输入无法注入 shell。
- 只有运行器自有的常量会被写入 `$GITHUB_PATH` 与 `$GITHUB_ENV`，因此调用方提供的值无法影响后续步骤解析可执行文件的位置。

## 不使用 GitHub Actions？

自托管运行器、非 GitHub 的 CI，或镜像中已经带有该 CLI — 直接驱动二进制：

```bash
brew install nimblesite/tap/deslop                                       # macOS / Linux
scoop bucket add nimblesite https://github.com/Nimblesite/scoop-bucket   # Windows
scoop install deslop

deslop . --fail-over 5.0
```

退出码 `3` 会像任何非零状态一样让步骤失败。各平台的归档见[发布页](https://github.com/Nimblesite/Deslop/releases)。
