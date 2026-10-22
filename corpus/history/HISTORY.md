# Corpus run history

Every `make score-corpus` run over the whole judged corpus, newest first ([CORPUS-HISTORY]). Each run is tied to the commit it was built from, and to its release version when the commit is a tagged release. This page is rendered from the records in `corpus/history/runs/` each time a run is added — never edit it by hand. A run fails only when a judged pair that passed before fails now.

## Runs

| run (UTC) | source | result | correct / judged | score | regressions | known defects | wall | CPU time | peak memory | machine |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ passed | 313/324 | 96.6% | 0 | 11 | 335.02 s | 716.15 s | 4620 MB | Apple M4 Max · 14 CPUs · 36864 MB · Darwin 26.6.2 |

## By repository

### axios (javascript)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 56/56 (100.0%) | 55/55 | 1/1 | 0 | 0 | 1.80 s | 2.41 s | 306% | 145 MB |

### bloc (dart)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 29/29 (100.0%) | 29/29 | 0/0 | 0 | 0 | 5.07 s | 11.49 s | 667% | 470 MB |

### click (python)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 18/18 (100.0%) | 18/18 | 0/0 | 0 | 0 | 1.40 s | 2.02 s | 226% | 139 MB |

### cobra (go)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 21/21 (100.0%) | 21/21 | 0/0 | 0 | 0 | 1.94 s | 3.25 s | 341% | 216 MB |

### composer (php)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | 🔴 no | 4/7 (57.1%) | 1/1 | 3/6 | 0 | 3 | 15.23 s | 25.04 s | 1035% | 755 MB |

### Deslop (rust)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 16/16 (100.0%) | 15/15 | 1/1 | 0 | 0 | 7.00 s | 13.95 s | 792% | 711 MB |

### dio (dart)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | 🔴 no | 1/2 (50.0%) | 1/1 | 0/1 | 0 | 1 | 1.27 s | 2.35 s | 291% | 152 MB |

### django (python)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 9/9 (100.0%) | 9/9 | 0/0 | 0 | 0 | 35.82 s | 71.61 s | 1132% | 1982 MB |

### FSharp.Data (fsharp)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 27/27 (100.0%) | 13/13 | 14/14 | 0 | 0 | 3.99 s | 8.09 s | 470% | 395 MB |

### Giraffe (fsharp)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | 🔴 no | 0/1 (0.0%) | 0/0 | 0/1 | 0 | 1 | 3.51 s | 6.16 s | 504% | 248 MB |

### guzzle (php)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 18/18 (100.0%) | 18/18 | 0/0 | 0 | 0 | 6.14 s | 10.14 s | 451% | 449 MB |

### hugo (go)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | 🔴 no | 4/9 (44.4%) | 2/2 | 2/7 | 0 | 5 | 19.76 s | 41.18 s | 1253% | 1038 MB |

### nest (typescript)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 20/20 (100.0%) | 20/20 | 0/0 | 0 | 0 | 11.65 s | 30.15 s | 1285% | 795 MB |

### Polly (csharp)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 38/38 (100.0%) | 36/36 | 2/2 | 0 | 0 | 15.72 s | 43.46 s | 1268% | 908 MB |

### react (javascript)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | 🔴 no | 5/6 (83.3%) | 5/5 | 0/1 | 0 | 1 | 119.51 s | 289.66 s | 1354% | 4620 MB |

### ripgrep (rust)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 5/5 (100.0%) | 5/5 | 0/0 | 0 | 0 | 10.35 s | 10.59 s | 435% | 358 MB |

### serilog (csharp)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 4/4 (100.0%) | 4/4 | 0/0 | 0 | 0 | 2.48 s | 3.04 s | 228% | 171 MB |

### tokio (rust)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 8/8 (100.0%) | 8/8 | 0/0 | 0 | 0 | 13.29 s | 24.09 s | 705% | 814 MB |

### zod (typescript)

| run (UTC) | source | matches judgments | correct / judged | IN found | OUT absent | false negatives | false positives | wall | CPU time | peak CPU | peak memory |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-22T21:05:36.093Z | `4e7c7df5ebee` | ✅ yes | 30/30 (100.0%) | 30/30 | 0/0 | 0 | 0 | 59.08 s | 117.47 s | 838% | 1257 MB |

## Judged pairs the latest run gets wrong (11)

- ⚠️ known defect — **composer** — false positive, reported in cluster 0cf7f14da5f33bec — `tests/Composer/Test/EventDispatcher/EventDispatcherTest.php:25-29` + `tests/Composer/Test/Package/Loader/ValidatingArrayLoaderTest.php:15-19`
- ⚠️ known defect — **composer** — false positive, reported in cluster 5ec541fb3e08b3bb — `tests/Composer/Test/DependencyResolver/PoolTest.php:1-19` + `tests/Composer/Test/Policy/MalwarePolicyConfigTest.php:1-19`
- ⚠️ known defect — **composer** — false positive, reported in cluster f57c27698097d9e8 — `src/Composer/Installer/PackageEvent.php:41-78` + `src/Composer/Plugin/PreFileDownloadEvent.php:62-74`
- ⚠️ known defect — **dio** — false positive, reported in cluster bfafbfe1c231a403 — `dio_test/lib/src/test/headers_tests.dart:49-51` + `plugins/web_adapter/test/cors_preflight_test.dart:203-206`
- ⚠️ known defect — **giraffe** — false positive, reported in cluster 0d363ff6a95bf6f5 — `tests/Giraffe.Tests/EndpointRoutingTests.fs:18-24` + `tests/Giraffe.Tests/EndpointRoutingTests.fs:180-186`
- ⚠️ known defect — **hugo** — false positive, reported in cluster 653e1acec8f2b101 — `tpl/internal/go_templates/texttemplate/exec_test.go:605-611` + `tpl/internal/go_templates/texttemplate/exec_test.go:618-624`
- ⚠️ known defect — **hugo** — false positive, reported in cluster d45c3c1126d4b4de — `tpl/collections/init.go:151-164` + `tpl/urls/init.go:47-58`
- ⚠️ known defect — **hugo** — false positive, reported in cluster 4ebf077d1a08095a — `tpl/lang/init.go:35-52` + `tpl/strings/init.go:51-69`
- ⚠️ known defect — **hugo** — false positive, reported in cluster 32e5ab3bceaa19c3 — `tpl/internal/go_templates/htmltemplate/exec_test.go:497-500` + `tpl/internal/go_templates/htmltemplate/exec_test.go:664-668`
- ⚠️ known defect — **hugo** — false positive, reported in cluster 5449877f70106279 — `config/allconfig/allconfig_integration_test.go:135-166` + `tpl/transform/transform_integration_test.go:353-381`
- ⚠️ known defect — **react** — false positive, reported in cluster 298cab00dad225a3 — `fixtures/attribute-behavior/src/attributes.js:1019-1030` + `fixtures/attribute-behavior/src/attributes.js:1941-1952`

## Regressions

None. No judged pair that passed in one run has failed in a later one.
