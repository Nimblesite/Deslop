# Finished motion and pi verification [DOC-FILMS-MOTION]

- Delivered films inspected: 10
- Animated chapter graphics checked: 40
- Graphic checks sample six encoded frames inside the illustration, excluding headings and the progress bar.
- Every illustration has at least four distinct sampled frames.
- Encoded agent frames must visibly contain `deslop/find-similar`, `Eta.cs` and `Zeta.cs`.

| Film | Chapters with motion | Agent result frame (s) | Distinct camera widths |
| --- | ---: | ---: | ---: |
| accuracy-transparency | 4 | - | - |
| ai-integration | 4 | 13.478 | 123 |
| configuration | 4 | - | - |
| for-ai | 4 | 36.962 | 123 |
| getting-started | 4 | - | - |
| github-action | 4 | - | - |
| how-it-works | 4 | - | - |
| osprey | 4 | - | - |
| research-background | 4 | - | - |
| vscode-cluster-panel | 4 | - | - |

Regenerate with `node scripts/videos/verify-motion-films.mjs` after rendering and cutting through Videos. Audio stream hashes and media hashes are verified by `cut-doc-footage.mjs --install`.
