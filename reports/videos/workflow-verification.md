# Finished workflow verification [DOC-FILMS-MOTION]

All checks below read the encoded website MP4s. OCR excludes captions and samples at least three frames from each step. Editor checks also exclude the step-label band; terminal checks retain the result lines and require explicit exit-status values absent from the labels. These content checks supplement visual review; they do not measure editing quality.

| Film | Recorded step | Visible application content |
| --- | --- | --- |
| getting-started | start-extension | Deslop |
| getting-started | review-finding | Eta.cs |
| getting-started | review-sources | Zeta.cs |
| getting-started | review-compare | Tally, step |
| configuration | config-create | threshold |
| configuration | config-exclude | archive, reference |
| configuration | config-override | exit status: 0 |
| configuration | config-limit | exit status: 3 |
| ai-integration | agent-ask | find-similar, Eta.cs |
| ai-integration | agent-search | find-similar, Eta.cs |
| ai-integration | agent-results | Eta.cs, Zeta.cs |
| ai-integration | review-compare | Tally, step |
| how-it-works | review-finding | Eta.cs |
| how-it-works | review-compare | Tally, step |
| how-it-works | review-sources | Zeta.cs |
| how-it-works | review-refresh | SharedCounter |
| osprey | osprey-finding | beta.osp |
| osprey | osprey-compare | alpha.osp, beta.osp |
| osprey | osprey-python | osprey |
| osprey | osprey-config | osprey-python-project, exit status: 0 |
| github-action | action-create | checkout |
| github-action | action-version | v0.41.0 |
| github-action | action-limit | exit status: 3 |
| github-action | action-changes | changes.patch, only-changed, exit status: 3 |
| vscode-cluster-panel | review-finding | Eta.cs |
| vscode-cluster-panel | review-compare | Tally, step |
| vscode-cluster-panel | review-pair | Tally, step |
| vscode-cluster-panel | review-counts | Duplication |
| accuracy-transparency | review-counts | Duplication |
| accuracy-transparency | review-sources | Zeta.cs |
| accuracy-transparency | review-compare | Tally, step |
| accuracy-transparency | config-exclude | archive, reference |
| research-background | review-sources | Zeta.cs |
| research-background | review-edit | limit |
| research-background | review-edited-compare | limit |
| research-background | review-pair | Tally, step |
| for-ai | agent-duplicates | deslop/duplicates, Eta.cs, Zeta.cs |
| for-ai | agent-details | Eta.cs, Zeta.cs |
| for-ai | agent-search | find-similar, Eta.cs |
| for-ai | review-pair | Tally, step |

## Real agent evidence

The saved pi session contains four successful Deslop MCP results: duplicates, finding details and two similarity requests. Every result identifies Eta.cs and Zeta.cs at lines 1–19, with exactly two copies.

## Narration

Independent local Whisper transcription checked all 40 Qwen3-TTS Aiden speech files against the current scripts.

| Film | Step | Word-sequence match |
| --- | ---: | ---: |
| github-action | 1 | 100.0% |
| github-action | 2 | 100.0% |
| github-action | 3 | 94.1% |
| github-action | 4 | 98.8% |
| getting-started | 1 | 97.4% |
| getting-started | 2 | 100.0% |
| getting-started | 3 | 97.6% |
| getting-started | 4 | 97.5% |
| vscode-cluster-panel | 1 | 93.7% |
| vscode-cluster-panel | 2 | 100.0% |
| vscode-cluster-panel | 3 | 100.0% |
| vscode-cluster-panel | 4 | 98.8% |
| how-it-works | 1 | 100.0% |
| how-it-works | 2 | 100.0% |
| how-it-works | 3 | 100.0% |
| how-it-works | 4 | 100.0% |
| research-background | 1 | 100.0% |
| research-background | 2 | 100.0% |
| research-background | 3 | 100.0% |
| research-background | 4 | 100.0% |
| accuracy-transparency | 1 | 97.6% |
| accuracy-transparency | 2 | 100.0% |
| accuracy-transparency | 3 | 97.6% |
| accuracy-transparency | 4 | 100.0% |
| configuration | 1 | 97.5% |
| configuration | 2 | 100.0% |
| configuration | 3 | 97.6% |
| configuration | 4 | 96.4% |
| ai-integration | 1 | 97.7% |
| ai-integration | 2 | 97.7% |
| ai-integration | 3 | 93.3% |
| ai-integration | 4 | 97.6% |
| for-ai | 1 | 100.0% |
| for-ai | 2 | 100.0% |
| for-ai | 3 | 100.0% |
| for-ai | 4 | 93.3% |
| osprey | 1 | 97.4% |
| osprey | 2 | 100.0% |
| osprey | 3 | 96.2% |
| osprey | 4 | 97.8% |

Regenerate with `node scripts/videos/verify-user-workflows.mjs`.
