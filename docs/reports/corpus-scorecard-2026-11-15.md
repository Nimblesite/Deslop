# ✅ PASSED — Corpus accuracy scorecard

> [!NOTE]
> ⚠️ **40 accepted defects** — explicitly accepted on record. None fails the run. Each is still a bug.
>
> - ⚠️ **benchmarkdotnet** — `false negative` (tracked in `corpus/register/outcomes.json`): `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:147-156` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:554-561`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **benchmarkdotnet** — `false negative` (tracked in `corpus/register/outcomes.json`): `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1027-1037` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1329-1339`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **composer** — `false positive` (tracked in `corpus/register/outcomes.json`): `src/Composer/Repository/Vcs/GitHubDriver.php:267-281` + `src/Composer/Repository/Vcs/GitHubDriver.php:282-296`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **composer** — `false positive` (tracked in `corpus/register/outcomes.json`): `src/Composer/Command/RequireCommand.php:92-94` + `src/Composer/Command/UpdateCommand.php:65-67`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **composer** — `false positive` (tracked in `corpus/register/outcomes.json`): `tests/Composer/Test/Downloader/HgDownloaderTest.php:67-69` + `tests/Composer/Test/Installer/SuggestedPackagesReporterTest.php:226-228`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **composer** — `false positive` (tracked in `corpus/register/outcomes.json`): `tests/Composer/Test/DependencyResolver/SolverTest.php:741-742` + `tests/Composer/Test/DependencyResolver/SolverTest.php:833-838`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **console** — `false negative` (tracked in `corpus/register/outcomes.json`): `Tests/Completion/Output/FishCompletionOutputTest.php:1-33` + `Tests/Completion/Output/ZshCompletionOutputTest.php:1-33`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **console** — `false negative` (tracked in `corpus/register/outcomes.json`): `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:116-125` + `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:59-68`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **date-fns** — `false negative` (tracked in `corpus/register/outcomes.json`): `pkgs/core/src/locale/kk/_lib/formatRelative/index.ts:52-64` + `pkgs/core/src/locale/mk/_lib/formatRelative/index.ts:82-94`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **django** — `false positive` (tracked in `corpus/register/outcomes.json`): `tests/invalid_models_tests/test_relative_fields.py:313-318` + `tests/migrations/migrations_test_apps/lookuperror_a/models.py:8-18`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **express** — `false negative` (tracked in `corpus/register/outcomes.json`): `test/app.router.js:119-129` + `test/app.router.js:95-105`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **express** — `false negative` (tracked in `corpus/register/outcomes.json`): `test/app.use.js:144-149` + `test/app.use.js:193-198`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **express** — `false negative` (tracked in `corpus/register/outcomes.json`): `test/app.param.js:275-282` + `test/app.param.js:99-106`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **express** — `false negative` (tracked in `corpus/register/outcomes.json`): `test/express.json.js:515-540` + `test/express.text.js:372-397`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **express** — `false negative` (tracked in `corpus/register/outcomes.json`): `test/res.jsonp.js:164-171` + `test/res.jsonp.js:335-342`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **fscheck** — `false negative` (tracked in `corpus/register/outcomes.json`): `examples/FsCheck.Examples.CSharp/Program.cs:13-26` + `examples/FsCheck.Examples.MsTest/ClassesToTest/EnumerableExtensions.cs:10-23`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **fsharpplus** — `false negative` (tracked in `corpus/register/outcomes.json`): `src/FSharpPlus/Data/NonEmptyList.fs:57-76` + `src/FSharpPlus/Data/NonEmptySet.fs:66-83`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **fsharpplus** — `false negative` (tracked in `corpus/register/outcomes.json`): `tests/FSharpPlus.Tests/General.fs:51-63` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:46-58`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **gin** — `false negative` (tracked in `corpus/register/outcomes.json`): `tree.go:861-871` + `tree.go:877-887`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **gin** — `false negative` (tracked in `corpus/register/outcomes.json`): `tree.go:764-799` + `tree.go:848-871`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **http** — `false negative` (tracked in `corpus/register/outcomes.json`): `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:167-174` + `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:225-233`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **hugo** — `false positive` (tracked in `corpus/register/outcomes.json`): `hugolib/page__meta_test.go:24-47` + `hugolib/site_output_test.go:657-689`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **hugo** — `false positive` (tracked in `corpus/register/outcomes.json`): `tpl/internal/go_templates/htmltemplate/exec_test.go:507-514` + `tpl/internal/go_templates/texttemplate/exec_test.go:615-622`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **hugo** — `false positive` (tracked in `corpus/register/outcomes.json`): `commands/server.go:545-546` + `commands/server.go:550-551`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **hugo** — `false positive` (tracked in `corpus/register/outcomes.json`): `tpl/tplimpl/templatestore_integration_test.go:1076-1114` + `tpl/tplimpl/templatestore_integration_test.go:908-925`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **hugo** — `false positive` (tracked in `corpus/register/outcomes.json`): `hugolib/disableKinds_test.go:175-176` + `resources/image_test.go:304-306`: on record as failing since 2026-11-02T01:31:12.995Z
> - ⚠️ **newtonsoft.json** — `false negative` (tracked in `corpus/register/outcomes.json`): `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:588-599` + `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:609-620`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **rayon** — `false negative` (tracked in `corpus/register/outcomes.json`): `rayon-core/src/broadcast/test.rs:163-168` + `rayon-core/src/broadcast/test.rs:201-206`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **viper** — `false negative` (tracked in `corpus/register/outcomes.json`): `viper.go:539-553` + `viper.go:572-587`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **viper** — `false negative` (tracked in `corpus/register/outcomes.json`): `internal/encoding/dotenv/codec_test.go:24-55` + `internal/encoding/toml/codec_test.go:66-97`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **vite** — `false negative` (tracked in `corpus/register/outcomes.json`): `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:178-205` + `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:191-218`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **vite** — `false negative` (tracked in `corpus/register/outcomes.json`): `playground/multiple-entrypoints/entrypoints/a10.js:20-31` + `playground/multiple-entrypoints/entrypoints/a11.js:19-30`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **vite** — `false negative` (tracked in `corpus/register/outcomes.json`): `playground/multiple-entrypoints/entrypoints/a12.js:18-28` + `playground/multiple-entrypoints/entrypoints/a8.js:22-32`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **vite** — `false positive` (tracked in `corpus/register/outcomes.json`): `packages/vite/src/types/chokidar.d.ts:77-92` + `packages/vite/src/types/ws.d.ts:262-271`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `contrib/infback9/infback9.c:334-342` + `infback.c:294-302`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `examples/zpipe.c:40-48` + `examples/zpipe.c:96-104`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `gzread.c:426-433` + `gzread.c:552-559`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `contrib/infback9/inftree9.c:228-236` + `contrib/infback9/inftree9.c:302-310`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `gzread.c:426-442` + `gzread.c:517-533`: on record as failing since 2026-11-08T06:47:26.057Z
> - ⚠️ **zlib** — `false negative` (tracked in `corpus/register/outcomes.json`): `gzwrite.c:271-281` + `gzwrite.c:298-308`: on record as failing since 2026-11-08T06:47:26.057Z

Generated 2026-11-15T06:47:29.276Z.

Scope: **47 repositories** across **11 languages** — c, csharp, dart, fsharp, go, javascript, osprey, php, python, rust, typescript.

Scored against the clone registers in `corpus/register/` — independent ground truth judged in isolation from this codebase (`docs/specs/corpus.md` [CORPUS-REGISTER]). A CLEARLY IN nobody reports is a **false negative**; a CLEARLY OUT that gets reported is a **false positive**. Both are bugs. A repository no register judges is still listed, with its curated checks and its cost.

## Corpus standing

Score is `correct / judged` over every judged pair in the corpus. Curated checks are the `corpus/<name>.json` assertions a corpus test evaluated. Each engine has its own column, so every measure reads across one row. Matched IN coverage is covered / judged lines among reported CLEARLY IN pairs; missed and CLEARLY OUT pairs are excluded. Coverage, clusters, wall time, CPU and memory are description — reported beside the score and never folded into it or its gate. Cost totals cover every repository scanned: wall and CPU time are summed, and each peak is the largest single scan's.

| measure | `deslop@89751512d3a4` | `deslop@fb68ce65f2d5` | change |
|---|---|---|---|
| score | 97.5% | 97.5% | +0.1 pts |
| correct / judged | 1578/1619 | 1579/1619 | +1 correct |
| false negatives | 29 | 29 | — |
| false positives | 12 | 11 | -1 |
| curated checks passed | 0/0 | 0/0 | — |
| new curated check failures | 0 | 0 | — |
| matched IN coverage | 91064/91747 (99.3%) | 91013/91747 (99.2%) | — |
| clusters | 82176 | 82199 | +23 |
| wall | 709.04 s | 520.71 s | -188325 ms |
| CPU time | 2176.73 s | 2024.06 s | -152.7 s |
| peak CPU | — | — | — |
| peak memory | 6889 MB | 6805 MB | -84 MB |

## Per repository — accuracy

One row per repository, one column per engine, so the two runs sit beside each other. `IN found` is the CLEARLY IN pairs the engine reported; `OUT absent` the CLEARLY OUT pairs it correctly stayed silent on. The last column says whether a defect is **new** against the first engine or **standing** in both.

| repository | judged | score `89751512d3a4` | score `fb68ce65f2d5` | IN found `89751512d3a4` | IN found `fb68ce65f2d5` | OUT absent `89751512d3a4` | OUT absent `fb68ce65f2d5` | defects |
|---|---|---|---|---|---|---|---|---|
| axios (javascript) | 59 IN + 2 OUT | 100.0% | 100.0% | 59/59 | 59/59 | 2/2 | 2/2 | clean |
| BenchmarkDotNet (csharp) | 72 IN + 3 OUT | 97.3% | 97.3% | 70/72 | 70/72 | 3/3 | 3/3 | 2 standing FN |
| bloc (dart) | 29 IN + 0 OUT | 100.0% | 100.0% | 29/29 | 29/29 | 0/0 | 0/0 | clean |
| click (python) | 18 IN + 0 OUT | 100.0% | 100.0% | 18/18 | 18/18 | 0/0 | 0/0 | clean |
| cobra (go) | 22 IN + 0 OUT | 100.0% | 100.0% | 22/22 | 22/22 | 0/0 | 0/0 | clean |
| composer (php) | 21 IN + 24 OUT | 91.1% | 91.1% | 21/21 | 21/21 | 20/24 | 20/24 | 4 standing FP |
| console (php) | 62 IN + 1 OUT | 96.8% | 96.8% | 60/62 | 60/62 | 1/1 | 1/1 | 2 standing FN |
| date-fns (typescript) | 61 IN + 0 OUT | 98.4% | 98.4% | 60/61 | 60/61 | 0/0 | 0/0 | 1 standing FN |
| Deslop (rust) | 15 IN + 1 OUT | 100.0% | 100.0% | 15/15 | 15/15 | 1/1 | 1/1 | clean |
| dio (dart) | 1 IN + 3 OUT | 100.0% | 100.0% | 1/1 | 1/1 | 3/3 | 3/3 | clean |
| django (python) | 15 IN + 13 OUT | 96.4% | 96.4% | 15/15 | 15/15 | 12/13 | 12/13 | 1 standing FP |
| express (javascript) | 90 IN + 0 OUT | 94.4% | 94.4% | 85/90 | 85/90 | 0/0 | 0/0 | 5 standing FN |
| fastapi (python) | 80 IN + 0 OUT | 100.0% | 100.0% | 80/80 | 80/80 | 0/0 | 0/0 | clean |
| flask (python) | 47 IN + 0 OUT | 100.0% | 100.0% | 47/47 | 47/47 | 0/0 | 0/0 | clean |
| FsCheck (fsharp) | 34 IN + 0 OUT | 97.1% | 97.1% | 33/34 | 33/34 | 0/0 | 0/0 | 1 standing FN |
| FSharp.Data (fsharp) | 13 IN + 14 OUT | 100.0% | 100.0% | 13/13 | 13/13 | 14/14 | 14/14 | clean |
| FSharpPlus (fsharp) | 65 IN + 0 OUT | 96.9% | 96.9% | 63/65 | 63/65 | 0/0 | 0/0 | 2 standing FN |
| gin (go) | 70 IN + 0 OUT | 97.1% | 97.1% | 68/70 | 68/70 | 0/0 | 0/0 | 2 standing FN |
| Giraffe (fsharp) | 1 IN + 1 OUT | 100.0% | 100.0% | 1/1 | 1/1 | 1/1 | 1/1 | clean |
| guzzle (php) | 36 IN + 10 OUT | 100.0% | 100.0% | 36/36 | 36/36 | 10/10 | 10/10 | clean |
| http (dart) | 84 IN + 3 OUT | 98.9% | 98.9% | 83/84 | 83/84 | 3/3 | 3/3 | 1 standing FN |
| hugo (go) | 12 IN + 29 OUT | 85.4% | 87.8% | 12/12 | 12/12 | 23/29 | 24/29 | 5 standing FP |
| mocktail (dart) | 11 IN + 0 OUT | 100.0% | 100.0% | 11/11 | 11/11 | 0/0 | 0/0 | clean |
| monolog (php) | 59 IN + 0 OUT | 100.0% | 100.0% | 59/59 | 59/59 | 0/0 | 0/0 | clean |
| nest (typescript) | 21 IN + 1 OUT | 100.0% | 100.0% | 21/21 | 21/21 | 1/1 | 1/1 | clean |
| Newtonsoft.Json (csharp) | 34 IN + 0 OUT | 97.1% | 97.1% | 33/34 | 33/34 | 0/0 | 0/0 | 1 standing FN |
| Polly (csharp) | 36 IN + 2 OUT | 100.0% | 100.0% | 36/36 | 36/36 | 2/2 | 2/2 | clean |
| rayon (rust) | 81 IN + 0 OUT | 98.8% | 98.8% | 80/81 | 80/81 | 0/0 | 0/0 | 1 standing FN |
| react (javascript) | 5 IN + 3 OUT | 100.0% | 100.0% | 5/5 | 5/5 | 3/3 | 3/3 | clean |
| ripgrep (rust) | 5 IN + 1 OUT | 100.0% | 100.0% | 5/5 | 5/5 | 1/1 | 1/1 | clean |
| serde (rust) | 109 IN + 0 OUT | 100.0% | 100.0% | 109/109 | 109/109 | 0/0 | 0/0 | clean |
| serilog (csharp) | 7 IN + 0 OUT | 100.0% | 100.0% | 7/7 | 7/7 | 0/0 | 0/0 | clean |
| tokio (rust) | 9 IN + 0 OUT | 100.0% | 100.0% | 9/9 | 9/9 | 0/0 | 0/0 | clean |
| viper (go) | 50 IN + 0 OUT | 96.0% | 96.0% | 48/50 | 48/50 | 0/0 | 0/0 | 2 standing FN |
| vite (typescript) | 75 IN + 1 OUT | 94.7% | 94.7% | 72/75 | 72/75 | 0/1 | 0/1 | 3 standing FN, 1 standing FP |
| zlib (c) | 62 IN + 0 OUT | 90.3% | 90.3% | 56/62 | 56/62 | 0/0 | 0/0 | 6 standing FN |
| zod (typescript) | 31 IN + 5 OUT | 100.0% | 100.0% | 31/31 | 31/31 | 5/5 | 5/5 | clean |
| osprey (osprey) | no register | — | — | — | — | — | — | — |
| jq (c) | no register | — | — | — | — | — | — | — |
| hiredis (c) | no register | — | — | — | — | — | — | — |
| clap (rust) | no register | — | — | — | — | — | — | — |
| trpc (typescript) | no register | — | — | — | — | — | — | — |
| riverpod (dart) | no register | — | — | — | — | — | — | — |
| MediatR (csharp) | no register | — | — | — | — | — | — | — |
| httpx (python) | no register | — | — | — | — | — | — | — |
| Argu (fsharp) | no register | — | — | — | — | — | — | — |
| libuv (c) | no register | — | — | — | — | — | — | — |

## Per repository — curated checks

The hand-curated assertions in `corpus/<name>.json` that `make test-corpus` holds each repository to: duplicates that must be found, ranking rules, the scan's scope and its resource ceilings. `known` failures are tracked in `corpus/known-failures.json`; a `NEW` one is a regression. `ceilings only` means the manifest curates no accuracy claim, so a pass there says nothing about whether detection is correct. `—` means the run evaluated none.

| repository | checks `89751512d3a4` | checks `fb68ce65f2d5` | failing |
|---|---|---|---|
| axios (javascript) | — | — | — |
| BenchmarkDotNet (csharp) | — | — | — |
| bloc (dart) | — | — | — |
| click (python) | — | — | — |
| cobra (go) | — | — | — |
| composer (php) | — | — | — |
| console (php) | — | — | — |
| date-fns (typescript) | — | — | — |
| Deslop (rust) | — | — | — |
| dio (dart) | — | — | — |
| django (python) | — | — | — |
| express (javascript) | — | — | — |
| fastapi (python) | — | — | — |
| flask (python) | — | — | — |
| FsCheck (fsharp) | — | — | — |
| FSharp.Data (fsharp) | — | — | — |
| FSharpPlus (fsharp) | — | — | — |
| gin (go) | — | — | — |
| Giraffe (fsharp) | — | — | — |
| guzzle (php) | — | — | — |
| http (dart) | — | — | — |
| hugo (go) | — | — | — |
| mocktail (dart) | — | — | — |
| monolog (php) | — | — | — |
| nest (typescript) | — | — | — |
| Newtonsoft.Json (csharp) | — | — | — |
| Polly (csharp) | — | — | — |
| rayon (rust) | — | — | — |
| react (javascript) | — | — | — |
| ripgrep (rust) | — | — | — |
| serde (rust) | — | — | — |
| serilog (csharp) | — | — | — |
| tokio (rust) | — | — | — |
| viper (go) | — | — | — |
| vite (typescript) | — | — | — |
| zlib (c) | — | — | — |
| zod (typescript) | — | — | — |
| osprey (osprey) | — | — | — |
| jq (c) | — | — | — |
| hiredis (c) | — | — | — |
| clap (rust) | — | — | — |
| trpc (typescript) | — | — | — |
| riverpod (dart) | — | — | — |
| MediatR (csharp) | — | — | — |
| httpx (python) | — | — | — |
| Argu (fsharp) | — | — | — |
| libuv (c) | — | — | — |


## Judged-range coverage

Each cell shows covered / judged lines across the pair's ranges. Overlapping occurrences count each line once within a range. A dash means no pair was reported. Coverage describes extent and never changes the verdict.

| repository | verdict | judged pair | coverage `89751512d3a4` | coverage `fb68ce65f2d5` |
|---|---|---|---|---|
| axios | IN | `test/unit/adapters/http.js:1321-1341` + `test/unit/adapters/http.js:1364-1384` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:133-143` + `test/module/typings/esm/index.ts:156-166` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:141-173` + `test/module/typings/esm/index.ts:164-196` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/specs/xsrf.spec.js:115-128` + `test/specs/xsrf.spec.js:100-113` | 28/28 (100.0%) | 28/28 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:153-163` + `test/module/typings/esm/index.ts:176-186` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:274-282` + `test/module/typings/cjs/index.ts:251-259` | 18/18 (100.0%) | 18/18 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:76-106` + `test/module/typings/esm/index.ts:99-129` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:250-275` + `test/module/typings/cjs/index.ts:227-252` | 52/52 (100.0%) | 52/52 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:277-298` + `test/module/typings/cjs/index.ts:254-275` | 44/44 (100.0%) | 44/44 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:42-51` + `test/module/typings/cjs/index.ts:19-28` | 20/20 (100.0%) | 20/20 (100.0%) |
| axios | IN | `test/unit/adapters/fetch.js:132-152` + `test/unit/adapters/fetch.js:206-226` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:262-279` + `test/module/typings/cjs/index.ts:239-256` | 36/36 (100.0%) | 36/36 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:137-169` + `test/module/typings/esm/index.ts:160-192` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:167-197` + `test/module/typings/esm/index.ts:190-220` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:149-181` + `test/module/typings/esm/index.ts:172-204` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:179-212` + `test/module/typings/esm/index.ts:202-235` | 68/68 (100.0%) | 68/68 (100.0%) |
| axios | IN | `test/unit/adapters/fetch.js:249-263` + `test/unit/adapters/fetch.js:174-188` | 30/30 (100.0%) | 30/30 (100.0%) |
| axios | IN | `test/unit/adapters/fetch.js:134-154` + `test/unit/adapters/fetch.js:208-228` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:95-125` + `test/module/typings/cjs/index.ts:72-102` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:156-186` + `test/module/typings/cjs/index.ts:133-163` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/specs/utils/endsWith.js:1-12` + `test/specs/utils/kindOfTest.js:1-12` | 24/24 (100.0%) | 24/24 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:145-177` + `test/module/typings/esm/index.ts:168-200` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/test.js:80-92` + `test/module/test.js:94-106` | 26/26 (100.0%) | 26/26 (100.0%) |
| axios | IN | `test/specs/interceptors.spec.js:589-601` + `test/specs/interceptors.spec.js:575-587` | 26/26 (100.0%) | 26/26 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:190-196` + `test/module/typings/cjs/index.ts:167-173` | 14/14 (100.0%) | 14/14 (100.0%) |
| axios | IN | `test/specs/core/mergeConfig.spec.js:229-269` + `test/specs/core/mergeConfig.spec.js:173-213` | 82/82 (100.0%) | 82/82 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:129-159` + `test/module/typings/esm/index.ts:152-182` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:91-121` + `test/module/typings/cjs/index.ts:68-98` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:223-251` + `test/module/typings/esm/index.ts:246-274` | 58/58 (100.0%) | 58/58 (100.0%) |
| axios | IN | `test/specs/requests.spec.js:236-255` + `test/specs/requests.spec.js:257-276` | 40/40 (100.0%) | 40/40 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:1400-1420` + `test/unit/adapters/http.js:1364-1384` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:264-275` + `test/module/typings/esm/index.ts:287-298` | 24/24 (100.0%) | 24/24 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:180-212` + `test/module/typings/cjs/index.ts:157-189` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:2000-2012` + `test/unit/adapters/fetch.js:251-263` | 26/26 (100.0%) | 26/26 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:171-201` + `test/module/typings/esm/index.ts:194-224` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:161-193` + `test/module/typings/esm/index.ts:184-216` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:80-111` + `test/module/typings/esm/index.ts:103-134` | 64/64 (100.0%) | 64/64 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:175-205` + `test/module/typings/esm/index.ts:198-228` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:345-354` + `test/module/typings/esm/index.ts:374-383` | 20/20 (100.0%) | 20/20 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:1321-1341` + `test/unit/adapters/http.js:1400-1420` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:2018-2038` + `test/unit/adapters/http.js:1960-1979` | 41/41 (100.0%) | 41/41 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:219-249` + `test/module/typings/esm/index.ts:242-272` | 62/62 (100.0%) | 62/62 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:242-248` + `test/module/typings/cjs/index.ts:219-225` | 14/14 (100.0%) | 14/14 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:176-208` + `test/module/typings/cjs/index.ts:153-185` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:183-193` + `test/module/typings/esm/index.ts:206-216` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:1057-1077` + `test/unit/adapters/http.js:1158-1178` | 42/42 (100.0%) | 42/42 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:246-256` + `test/module/typings/cjs/index.ts:223-233` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:104-116` + `test/module/typings/esm/index.ts:127-139` | 26/26 (100.0%) | 26/26 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:84-116` + `test/module/typings/esm/index.ts:107-139` | 66/66 (100.0%) | 66/66 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:96-102` + `test/module/typings/esm/index.ts:119-125` | 14/14 (100.0%) | 14/14 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:141-151` + `test/module/typings/esm/index.ts:164-174` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/module/typings/esm/index.ts:254-276` + `test/module/typings/cjs/index.ts:231-253` | 46/46 (100.0%) | 46/46 (100.0%) |
| axios | IN | `test/module/typings/cjs/index.ts:279-289` + `test/module/typings/cjs/index.ts:294-304` | 22/22 (100.0%) | 22/22 (100.0%) |
| axios | IN | `test/specs/interceptors.spec.js:267-281` + `test/specs/interceptors.spec.js:238-252` | 30/30 (100.0%) | 30/30 (100.0%) |
| axios | IN | `test/unit/adapters/fetch.js:251-263` + `test/unit/adapters/http.js:2060-2072` | 26/26 (100.0%) | 26/26 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:65-104` + `test/helpers/server.js:13-52` | 80/80 (100.0%) | 80/80 (100.0%) |
| axios | IN | `test/unit/adapters/fetch.js:205-264` + `test/unit/adapters/fetch.js:131-189` | 119/119 (100.0%) | 119/119 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:1958-2014` + `test/unit/adapters/http.js:2016-2074` | 116/116 (100.0%) | 116/116 (100.0%) |
| axios | IN | `test/unit/adapters/http.js:2196-2209` + `test/unit/adapters/http.js:969-982` | 28/28 (100.0%) | 28/28 (100.0%) |
| axios | OUT | `test/manual/promise.js:9-9` + `lib/axios.js:50-60` | — | — |
| axios | OUT | `index.d.ts:218-227` + `index.d.ts:197-206` | — | — |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncStateMachineEmitter.cs:501-531` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:156-168` | 44/44 (100.0%) | 44/44 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Diagnostics.Windows/JitStatsDiagnoser.cs:59-72` + `src/BenchmarkDotNet.Diagnostics.Windows/JitStatsDiagnoser.cs:74-87` | 28/28 (100.0%) | 28/28 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Helpers/CodeAnnotations.cs:1307-1318` + `src/BenchmarkDotNet/Helpers/CodeAnnotations.cs:1344-1355` | 24/24 (100.0%) | 24/24 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/AsyncBenchmarkAnalyzerTests.cs:35-107` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:196-268` | 146/146 (100.0%) | 146/146 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Detectors/Cpu/LinuxCpuInfoParserTests.cs:46-61` + `tests/BenchmarkDotNet.Tests/Detectors/Cpu/LinuxCpuInfoParserTests.cs:63-78` | 32/32 (100.0%) | 32/32 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:554-561` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:147-156` | — | — |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:296-306` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:131-141` | 22/22 (100.0%) | 22/22 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1013-1085` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1111-1189` | 152/152 (100.0%) | 152/152 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Analyzers/Attributes/GeneralParameterAttributesAnalyzer.cs:12-63` + `src/BenchmarkDotNet.Analyzers/General/BenchmarkClassAnalyzer.cs:12-63` | 104/104 (100.0%) | 104/104 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1716-1727` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:918-930` | 25/25 (100.0%) | 25/25 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:84-112` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:115-143` | 58/58 (100.0%) | 58/58 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1281-1358` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:676-753` | 156/156 (100.0%) | 156/156 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Running/BenchmarkRunnerDirty.cs:65-74` + `src/BenchmarkDotNet/Running/BenchmarkRunnerDirty.cs:29-38` | 20/20 (100.0%) | 20/20 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Diagnostics.dotTrace/DotTraceDiagnoser.cs:1-82` + `src/BenchmarkDotNet.Diagnostics.dotMemory/DotMemoryDiagnoser.cs:1-79` | 161/161 (100.0%) | 161/161 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/GithubMarkdownDisassemblyExporterMultiCorerunTest.cs:21-37` + `tests/BenchmarkDotNet.Tests/Disassemblers/GithubMarkdownDisassemblyExporterMultiCorerunTest.cs:43-55` | 30/30 (100.0%) | 30/30 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/BenchmarkCancellationAttributeAnalyzerTests.cs:80-120` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:1269-1313` | 86/86 (100.0%) | 86/86 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:13-48` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:56-100` | 81/81 (100.0%) | 81/81 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:130-169` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:346-379` | 74/74 (100.0%) | 74/74 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Reports/SummaryTableTests.cs:259-269` + `tests/BenchmarkDotNet.Tests/Reports/SummaryTableTests.cs:282-292` | 22/22 (100.0%) | 22/22 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1027-1037` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1329-1339` | — | — |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Engine/EngineActualStageTests.cs:31-44` + `tests/BenchmarkDotNet.Tests/Engine/EngineWarmupStageTests.cs:80-93` | 28/28 (100.0%) | 28/28 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/AllSetupAndCleanupTest.cs:171-210` + `tests/BenchmarkDotNet.IntegrationTests/AllSetupAndCleanupTest.cs:109-148` | 80/80 (100.0%) | 80/80 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenMovk.cs:23-33` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.Movz.cs:23-33` | 22/22 (100.0%) | 22/22 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Validators/ExecutionValidatorTests.cs:620-698` + `tests/BenchmarkDotNet.Tests/Validators/ReturnValueValidatorTests.cs:331-419` | 168/168 (100.0%) | 168/168 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1912-1919` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:662-669` | 16/16 (100.0%) | 16/16 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:138-145` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:201-208` | 16/16 (100.0%) | 16/16 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/Mono/RoslynMonoToolchain.cs:1-53` + `src/BenchmarkDotNet/Toolchains/Mono/RoslynMonoAotToolchain.cs:1-53` | 106/106 (100.0%) | 106/106 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1211-1246` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:452-480` | 65/65 (100.0%) | 65/65 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Exporters.Plotting/ScottPlotExporter.cs:179-253` + `src/BenchmarkDotNet.Exporters.Plotting/ScottPlotExporter.cs:255-337` | 158/158 (100.0%) | 158/158 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/EventProcessorTests.cs:29-69` + `tests/BenchmarkDotNet.IntegrationTests/EventProcessorTests.cs:71-112` | 83/83 (100.0%) | 83/83 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:435-441` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:351-357` | 14/14 (100.0%) | 14/14 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:162-171` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:349-356` | 18/18 (100.0%) | 18/18 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Analyzers/AnalyzerHelper.cs:29-36` + `src/BenchmarkDotNet.Analyzers/AnalyzerHelper.cs:40-47` | 16/16 (100.0%) | 16/16 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/RequiredMemberAnalyzerTests.cs:36-75` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:1637-1683` | 87/87 (100.0%) | 87/87 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:966-997` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1869-1902` | 66/66 (100.0%) | 66/66 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Analyzers/Attributes/ParamsAllValuesAttributeAnalyzer.cs:21-50` + `src/BenchmarkDotNet.Analyzers/BenchmarkRunner/RunAnalyzer.cs:47-82` | 66/66 (100.0%) | 66/66 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenLdr.cs:95-108` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenLdr.cs:19-32` | 28/28 (100.0%) | 28/28 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Detectors/OsDetector.cs:140-147` + `src/BenchmarkDotNet/Detectors/OsDetector.cs:131-139` | 17/17 (100.0%) | 17/17 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:711-809` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:811-909` | 198/198 (100.0%) | 198/198 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:383-443` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:114-163` | 111/111 (100.0%) | 111/111 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:67-104` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:115-152` | 76/76 (100.0%) | 76/76 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:552-559` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:310-318` | 2/17 (11.8%) | 2/17 (11.8%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryResolvePreCode.cs:59-74` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryResolvePreCode.cs:25-40` | 32/32 (100.0%) | 32/32 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/ParamSourceTests.cs:17-33` + `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableParamsSourceTests.cs:19-35` | 18/34 (52.9%) | 18/34 (52.9%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Mathematics/StatisticsTests.cs:93-100` + `tests/BenchmarkDotNet.Tests/Mathematics/StatisticsTests.cs:69-76` | 16/16 (100.0%) | 16/16 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:16-40` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:291-315` | 50/50 (100.0%) | 50/50 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/wwwroot/custom-main-v8.mjs:110-140` + `tests/BenchmarkDotNet.IntegrationTests/wwwroot/custom-main-node.mjs:85-110` | 57/57 (100.0%) | 57/57 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Exporters.Plotting.Tests/ScottPlotExporterTests.cs:189-206` + `tests/BenchmarkDotNet.Tests/Exporters/MarkdownExporterVerifyTests.cs:158-175` | 36/36 (100.0%) | 36/36 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableBenchmarksTests.cs:111-121` + `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableBenchmarksTests.cs:79-91` | 24/24 (100.0%) | 24/24 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:897-965` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:756-825` | 139/139 (100.0%) | 139/139 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Detectors/Cpu/CpuInfoParserTests_Wmic.cs:76-85` + `tests/BenchmarkDotNet.Tests/Detectors/Cpu/CpuInfoParserTests_Wmic.cs:44-53` | 20/20 (100.0%) | 20/20 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/DotNetCli/DotNetCliExecutor.cs:87-125` + `src/BenchmarkDotNet/Toolchains/Wasm/WasmExecutor.cs:190-244` | 94/94 (100.0%) | 94/94 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:42-95` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/BenchmarkCancellationAttributeAnalyzerTests.cs:78-120` | 97/97 (100.0%) | 97/97 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Exporters/OpenMetricsExporterTests.cs:24-99` + `tests/BenchmarkDotNet.Tests/Exporters/OpenMetricsExporterTests.cs:101-179` | 155/155 (100.0%) | 155/155 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:897-934` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:756-794` | 77/77 (100.0%) | 77/77 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:97-122` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:41-66` | 52/52 (100.0%) | 52/52 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryTranslateAddressToName.cs:237-260` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryTranslateAddressToName.cs:267-288` | 46/46 (100.0%) | 46/46 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:380-434` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:658-716` | 114/114 (100.0%) | 114/114 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Helpers/HashCode.cs:85-92` + `src/BenchmarkDotNet/Helpers/HashCode.cs:98-105` | 16/16 (100.0%) | 16/16 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:138-146` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:303-311` | 18/18 (100.0%) | 18/18 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:55-100` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:12-48` | 83/83 (100.0%) | 83/83 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:1461-1491` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1321-1359` | 70/70 (100.0%) | 70/70 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:49-59` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:232-242` | 22/22 (100.0%) | 22/22 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Analyzers/Attributes/GeneralArgumentAttributesAnalyzer.cs:12-42` + `src/BenchmarkDotNet.Analyzers/General/BenchmarkClassAnalyzer.cs:73-110` | 69/69 (100.0%) | 69/69 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:526-545` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:243-264` | 42/42 (100.0%) | 42/42 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:573-592` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:421-440` | 40/40 (100.0%) | 40/40 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:47-80` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:11-45` | 69/69 (100.0%) | 69/69 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/RuntimeAndToolchainCodeFixProviderTests.cs:12-39` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/RuntimeAndToolchainCodeFixProviderTests.cs:41-68` | 56/56 (100.0%) | 56/56 (100.0%) |
| BenchmarkDotNet | IN | `src/BenchmarkDotNet.Analyzers/Attributes/ArgumentsAttributeAnalyzer.cs:12-45` + `src/BenchmarkDotNet.Analyzers/Attributes/GeneralParameterAttributesAnalyzer.cs:57-90` | 68/68 (100.0%) | 68/68 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Tests/ConfigParserTests.cs:1103-1115` + `tests/BenchmarkDotNet.Tests/ConfigParserTests.cs:1117-1129` | 26/26 (100.0%) | 26/26 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.IntegrationTests/JitListenerTests.cs:139-149` + `tests/BenchmarkDotNet.IntegrationTests/JitListenerTests.cs:15-25` | 22/22 (100.0%) | 22/22 (100.0%) |
| BenchmarkDotNet | IN | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:870-907` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:25-63` | 77/77 (100.0%) | 77/77 (100.0%) |
| BenchmarkDotNet | OUT | `tests/BenchmarkDotNet.Tests/Environments/PhysicalMemoryInfoTests.cs:14-21` + `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:55-62` | — | — |
| BenchmarkDotNet | OUT | `tests/BenchmarkDotNet.Tests/Environments/PhysicalMemoryInfoTests.cs:14-21` + `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:30-35` | — | — |
| BenchmarkDotNet | OUT | `src/BenchmarkDotNet/Engines/JitListener.cs:57-63` + `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:54-62` | — | — |
| bloc | IN | `examples/flutter_form_validation/lib/models/password.dart:9-18` + `examples/flutter_firebase_login/packages/form_inputs/lib/src/password.dart:19-28` | 20/20 (100.0%) | 20/20 (100.0%) |
| bloc | IN | `extensions/vscode/src/commands/new-cubit.command.ts:16-70` + `extensions/vscode/src/commands/new-bloc.command.ts:20-74` | 110/110 (100.0%) | 110/110 (100.0%) |
| bloc | IN | `packages/replay_bloc/test/replay_bloc_test.dart:271-306` + `packages/replay_bloc/test/replay_bloc_test.dart:529-564` | 72/72 (100.0%) | 72/72 (100.0%) |
| bloc | IN | `extensions/vscode/src/commands/new-cubit.command.ts:96-132` + `extensions/vscode/src/commands/new-bloc.command.ts:110-146` | 74/74 (100.0%) | 74/74 (100.0%) |
| bloc | IN | `examples/flutter_todos/test/edit_todo/bloc/edit_todo_bloc_test.dart:170-186` + `examples/flutter_todos/test/edit_todo/bloc/edit_todo_bloc_test.dart:74-90` | 34/34 (100.0%) | 34/34 (100.0%) |
| bloc | IN | `examples/flutter_todos/test/stats/view/stats_page_test.dart:130-169` + `examples/flutter_todos/test/stats/view/stats_page_test.dart:89-128` | 80/80 (100.0%) | 80/80 (100.0%) |
| bloc | IN | `examples/flutter_login/lib/login/models/password.dart:1-14` + `examples/flutter_login/lib/login/models/username.dart:1-14` | 28/28 (100.0%) | 28/28 (100.0%) |
| bloc | IN | `examples/flutter_todos/lib/main_production.dart:1-13` + `examples/flutter_todos/lib/main_staging.dart:1-13` | 26/26 (100.0%) | 26/26 (100.0%) |
| bloc | IN | `examples/flutter_firebase_login/lib/login/view/login_form.dart:53-92` + `examples/flutter_firebase_login/lib/sign_up/view/sign_up_form.dart:42-81` | 80/80 (100.0%) | 80/80 (100.0%) |
| bloc | IN | `packages/replay_bloc/test/replay_bloc_test.dart:202-240` + `packages/replay_bloc/test/replay_bloc_test.dart:489-527` | 78/78 (100.0%) | 78/78 (100.0%) |
| bloc | IN | `packages/replay_bloc/test/replay_cubit_test.dart:433-490` + `packages/replay_bloc/test/replay_cubit_test.dart:188-245` | 116/116 (100.0%) | 116/116 (100.0%) |
| bloc | IN | `packages/bloc/test/bloc_event_transformer_test.dart:5-41` + `packages/bloc/test/bloc_event_transformer_test_legacy.dart:5-41` | 74/74 (100.0%) | 74/74 (100.0%) |
| bloc | IN | `examples/flutter_login/test/login/view/login_form_test.dart:22-45` + `examples/flutter_login/test/login/view/login_form_test.dart:47-70` | 48/48 (100.0%) | 48/48 (100.0%) |
| bloc | IN | `packages/bloc_lint/lib/src/text_document.dart:191-197` + `packages/bloc_tools/lib/src/lsp/text_document.dart:195-201` | 14/14 (100.0%) | 14/14 (100.0%) |
| bloc | IN | `examples/flutter_login/test/login/models/username_test.dart:2-38` + `examples/flutter_login/test/login/models/password_test.dart:2-38` | 74/74 (100.0%) | 74/74 (100.0%) |
| bloc | IN | `packages/replay_bloc/test/replay_bloc_test.dart:71-91` + `packages/replay_bloc/test/replay_bloc_test.dart:423-443` | 42/42 (100.0%) | 42/42 (100.0%) |
| bloc | IN | `examples/flutter_timer/lib/timer/view/timer_page.dart:83-96` + `examples/flutter_timer/lib/timer/view/timer_page.dart:97-110` | 28/28 (100.0%) | 28/28 (100.0%) |
| bloc | IN | `packages/bloc/test/bloc_test.dart:641-703` + `packages/bloc/test/bloc_test.dart:542-604` | 126/126 (100.0%) | 126/126 (100.0%) |
| bloc | IN | `packages/bloc/test/bloc_test.dart:541-604` + `packages/bloc/test/bloc_test.dart:640-703` | 128/128 (100.0%) | 128/128 (100.0%) |
| bloc | IN | `examples/flutter_todos/lib/edit_todo/bloc/edit_todo_event.dart:3-30` + `examples/flutter_login/lib/login/bloc/login_event.dart:3-30` | 56/56 (100.0%) | 56/56 (100.0%) |
| bloc | IN | `packages/bloc_tools/lib/src/lsp/text_document.dart:41-99` + `packages/bloc_lint/lib/src/text_document.dart:100-158` | 118/118 (100.0%) | 118/118 (100.0%) |
| bloc | IN | `examples/flutter_weather/lib/weather/cubit/weather_cubit.dart:49-60` + `examples/flutter_weather/lib/weather/cubit/weather_cubit.dart:25-36` | 24/24 (100.0%) | 24/24 (100.0%) |
| bloc | IN | `packages/bloc/test/bloc_test.dart:303-316` + `packages/bloc/test/bloc_test.dart:352-365` | 28/28 (100.0%) | 28/28 (100.0%) |
| bloc | IN | `extensions/vscode/src/templates/cubit-state.template.ts:4-55` + `extensions/vscode/src/templates/bloc-state.template.ts:4-55` | 104/104 (100.0%) | 104/104 (100.0%) |
| bloc | IN | `examples/flutter_todos/test/edit_todo/bloc/edit_todo_event_test.dart:26-42` + `examples/flutter_todos/test/edit_todo/bloc/edit_todo_event_test.dart:8-24` | 34/34 (100.0%) | 34/34 (100.0%) |
| bloc | IN | `examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:102-114` + `examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:224-236` | 26/26 (100.0%) | 26/26 (100.0%) |
| bloc | IN | `examples/flutter_todos/lib/main_staging.dart:1-13` + `examples/flutter_todos/lib/main_development.dart:1-13` | 26/26 (100.0%) | 26/26 (100.0%) |
| bloc | IN | `packages/bloc_lint/test/src/analysis_options_test.dart:135-147` + `packages/bloc_lint/test/src/linter_test.dart:49-61` | 26/26 (100.0%) | 26/26 (100.0%) |
| bloc | IN | `packages/bloc/test/bloc_test.dart:303-315` + `packages/bloc/test/bloc_test.dart:352-364` | 26/26 (100.0%) | 26/26 (100.0%) |
| click | IN | `tests/test_arguments.py:340-345` + `tests/test_arguments.py:394-399` | 12/12 (100.0%) | 12/12 (100.0%) |
| click | IN | `src/click/_compat.py:176-191` + `src/click/_compat.py:194-209` | 32/32 (100.0%) | 32/32 (100.0%) |
| click | IN | `tests/test_basic.py:550-562` + `tests/test_basic.py:608-620` | 26/26 (100.0%) | 26/26 (100.0%) |
| click | IN | `tests/test_stream_lifecycle.py:498-505` + `tests/test_stream_lifecycle.py:340-347` | 16/16 (100.0%) | 16/16 (100.0%) |
| click | IN | `src/click/_compat.py:303-316` + `src/click/_compat.py:287-300` | 28/28 (100.0%) | 28/28 (100.0%) |
| click | IN | `src/click/decorators.py:352-377` + `src/click/decorators.py:324-349` | 52/52 (100.0%) | 52/52 (100.0%) |
| click | IN | `src/click/shell_completion.py:432-442` + `src/click/shell_completion.py:530-540` | 22/22 (100.0%) | 22/22 (100.0%) |
| click | IN | `src/click/exceptions.py:238-260` + `src/click/exceptions.py:274-296` | 46/46 (100.0%) | 46/46 (100.0%) |
| click | IN | `src/click/shell_completion.py:527-540` + `src/click/shell_completion.py:451-464` | 28/28 (100.0%) | 28/28 (100.0%) |
| click | IN | `src/click/decorators.py:258-290` + `src/click/decorators.py:133-165` | 66/66 (100.0%) | 66/66 (100.0%) |
| click | IN | `src/click/shell_completion.py:454-464` + `src/click/shell_completion.py:530-540` | 22/22 (100.0%) | 22/22 (100.0%) |
| click | IN | `tests/test_context.py:593-620` + `tests/test_context.py:539-566` | 56/56 (100.0%) | 56/56 (100.0%) |
| click | IN | `tests/test_termui.py:977-985` + `tests/test_termui.py:997-1005` | 18/18 (100.0%) | 18/18 (100.0%) |
| click | IN | `tests/test_testing.py:18-26` + `tests/test_testing.py:36-44` | 18/18 (100.0%) | 18/18 (100.0%) |
| click | IN | `tests/test_options.py:939-946` + `tests/test_options.py:972-979` | 16/16 (100.0%) | 16/16 (100.0%) |
| click | IN | `tests/test_context.py:52-59` + `tests/test_context.py:75-82` | 16/16 (100.0%) | 16/16 (100.0%) |
| click | IN | `src/click/decorators.py:550-556` + `src/click/decorators.py:592-598` | 14/14 (100.0%) | 14/14 (100.0%) |
| click | IN | `tests/test_basic.py:207-213` + `tests/test_basic.py:230-236` | 14/14 (100.0%) | 14/14 (100.0%) |
| cobra | IN | `completions_test.go:3159-3183` + `completions_test.go:3059-3083` | 50/50 (100.0%) | 50/50 (100.0%) |
| cobra | IN | `doc/yaml_docs.go:53-57` + `doc/md_docs.go:125-129` | 10/10 (100.0%) | 10/10 (100.0%) |
| cobra | IN | `powershell_completions.go:31-37` + `fish_completions.go:28-34` | 14/14 (100.0%) | 14/14 (100.0%) |
| cobra | IN | `completions_test.go:3060-3071` + `completions_test.go:3160-3171` | 24/24 (100.0%) | 24/24 (100.0%) |
| cobra | IN | `command.go:573-580` + `command.go:563-570` | 16/16 (100.0%) | 16/16 (100.0%) |
| cobra | IN | `completions_test.go:3142-3155` + `completions_test.go:3568-3581` | 28/28 (100.0%) | 28/28 (100.0%) |
| cobra | IN | `completions_test.go:1504-1533` + `completions_test.go:1404-1433` | 60/60 (100.0%) | 60/60 (100.0%) |
| cobra | IN | `completions_test.go:1337-1354` + `completions_test.go:2303-2320` | 36/36 (100.0%) | 36/36 (100.0%) |
| cobra | IN | `command.go:1414-1425` + `command.go:1344-1355` | 24/24 (100.0%) | 24/24 (100.0%) |
| cobra | IN | `command.go:412-420` + `command.go:422-430` | 18/18 (100.0%) | 18/18 (100.0%) |
| cobra | IN | `bash_completions.go:567-573` + `bash_completions.go:581-587` | 14/14 (100.0%) | 14/14 (100.0%) |
| cobra | IN | `completions_test.go:391-423` + `completions_test.go:442-474` | 66/66 (100.0%) | 66/66 (100.0%) |
| cobra | IN | `completions_test.go:2292-2320` + `completions_test.go:1326-1354` | 58/58 (100.0%) | 58/58 (100.0%) |
| cobra | IN | `args_test.go:240-256` + `args_test.go:190-206` | 34/34 (100.0%) | 34/34 (100.0%) |
| cobra | IN | `command.go:505-515` + `command.go:464-473` | 21/21 (100.0%) | 21/21 (100.0%) |
| cobra | IN | `completions_test.go:3122-3139` + `completions_test.go:3220-3237` | 36/36 (100.0%) | 36/36 (100.0%) |
| cobra | IN | `completions_test.go:1718-1735` + `completions_test.go:1385-1402` | 36/36 (100.0%) | 36/36 (100.0%) |
| cobra | IN | `completions_test.go:1753-1763` + `completions_test.go:1797-1807` | 22/22 (100.0%) | 22/22 (100.0%) |
| cobra | IN | `doc/rest_docs_test.go:80-96` + `doc/yaml_docs_test.go:57-73` | 34/34 (100.0%) | 34/34 (100.0%) |
| cobra | IN | `completions_test.go:3258-3269` + `completions_test.go:3060-3071` | 24/24 (100.0%) | 24/24 (100.0%) |
| cobra | IN | `completions_test.go:3240-3253` + `completions_test.go:3334-3347` | 28/28 (100.0%) | 28/28 (100.0%) |
| cobra | IN | `doc/rest_docs.go:30-49` + `doc/md_docs.go:32-49` | 38/38 (100.0%) | 38/38 (100.0%) |
| composer | IN | `src/Composer/Util/ConfigValidator.php:190-197` + `src/Composer/Util/ConfigValidator.php:201-208` | 16/16 (100.0%) | 16/16 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/DownloadManagerTest.php:915-922` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:827-834` | 16/16 (100.0%) | 16/16 (100.0%) |
| composer | IN | `tests/Composer/Test/Util/GitHubTest.php:21-131` + `tests/Composer/Test/Util/ForgejoTest.php:18-132` | 226/226 (100.0%) | 226/226 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/FossilDownloaderTest.php:98-161` + `tests/Composer/Test/Downloader/HgDownloaderTest.php:97-153` | 121/121 (100.0%) | 121/121 (100.0%) |
| composer | IN | `src/Composer/Command/DiagnoseCommand.php:365-399` + `src/Composer/Command/DiagnoseCommand.php:326-360` | 70/70 (100.0%) | 70/70 (100.0%) |
| composer | IN | `src/Composer/Command/RemoveCommand.php:50-61` + `src/Composer/Command/RequireCommand.php:87-98` | 24/24 (100.0%) | 24/24 (100.0%) |
| composer | IN | `src/Composer/Command/DependsCommand.php:23-58` + `src/Composer/Command/ProhibitsCommand.php:23-59` | 73/73 (100.0%) | 73/73 (100.0%) |
| composer | IN | `src/Composer/Command/RemoveCommand.php:51-56` + `src/Composer/Command/UpdateCommand.php:65-70` | 10/12 (83.3%) | 10/12 (83.3%) |
| composer | IN | `tests/Composer/Test/Downloader/GitDownloaderTest.php:589-629` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:541-587` | 88/88 (100.0%) | 88/88 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/GitDownloaderTest.php:105-116` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:141-152` | 24/24 (100.0%) | 24/24 (100.0%) |
| composer | IN | `tests/Composer/Test/Plugin/Fixtures/plugin-v2/Installer/Plugin2.php:9-27` + `tests/Composer/Test/Plugin/Fixtures/plugin-v9/Installer/Plugin.php:11-29` | 38/38 (100.0%) | 38/38 (100.0%) |
| composer | IN | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:1637-1687` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:1787-1837` | 102/102 (100.0%) | 102/102 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/GitDownloaderTest.php:253-261` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:390-398` | 18/18 (100.0%) | 18/18 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/DownloadManagerTest.php:871-878` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:1059-1066` | 16/16 (100.0%) | 16/16 (100.0%) |
| composer | IN | `src/Composer/Console/Input/InputArgument.php:31-69` + `src/Composer/Console/Input/InputOption.php:31-72` | 81/81 (100.0%) | 81/81 (100.0%) |
| composer | IN | `tests/Composer/Test/Repository/ComposerRepositoryTest.php:1093-1104` + `tests/Composer/Test/Repository/ComposerRepositoryTest.php:734-745` | 24/24 (100.0%) | 24/24 (100.0%) |
| composer | IN | `tests/Composer/Test/Downloader/DownloadManagerTest.php:260-332` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:820-903` | 157/157 (100.0%) | 157/157 (100.0%) |
| composer | IN | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:327-334` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:298-305` | 16/16 (100.0%) | 16/16 (100.0%) |
| composer | IN | `tests/Composer/Test/Autoload/Fixtures/autoload_psr4_2.php:1-11` + `tests/Composer/Test/Autoload/Fixtures/autoload_psr4.php:1-11` | 22/22 (100.0%) | 22/22 (100.0%) |
| composer | IN | `src/Composer/FilterList/FilterListEntryBuilder.php:69-77` + `src/Composer/FilterList/FilterListProvider/FilterListProviderSet.php:146-156` | 20/20 (100.0%) | 20/20 (100.0%) |
| composer | IN | `tests/Composer/Test/Autoload/Fixtures/autoload_static_functions.php:1-28` + `tests/Composer/Test/Autoload/Fixtures/autoload_static_functions_with_include_paths.php:1-28` | 56/56 (100.0%) | 56/56 (100.0%) |
| composer | OUT | `tests/Composer/Test/EventDispatcher/EventDispatcherTest.php:25-29` + `tests/Composer/Test/Package/Loader/ValidatingArrayLoaderTest.php:15-19` | — | — |
| composer | OUT | `tests/Composer/Test/DependencyResolver/PoolTest.php:1-19` + `tests/Composer/Test/Policy/MalwarePolicyConfigTest.php:1-19` | — | — |
| composer | OUT | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:56-94` + `src/Composer/Plugin/PrePoolCreateEvent.php:30-62` | — | — |
| composer | OUT | `src/Composer/Plugin/PrePoolCreateEvent.php:34-62` + `src/Composer/Util/RemoteFilesystem.php:34-46` | — | — |
| composer | OUT | `src/Composer/Command/RequireCommand.php:56-70` + `src/Composer/Util/RemoteFilesystem.php:34-48` | — | — |
| composer | OUT | `src/Composer/Installer/PackageEvent.php:41-78` + `src/Composer/Plugin/PreFileDownloadEvent.php:62-74` | — | — |
| composer | OUT | `src/Composer/Repository/Vcs/GitHubDriver.php:267-281` + `src/Composer/Repository/Vcs/GitHubDriver.php:282-296` | 30/30 (100.0%) | 30/30 (100.0%) |
| composer | OUT | `src/Composer/Command/RequireCommand.php:92-94` + `src/Composer/Command/UpdateCommand.php:65-67` | 5/6 (83.3%) | 5/6 (83.3%) |
| composer | OUT | `tests/Composer/Test/Downloader/HgDownloaderTest.php:67-69` + `tests/Composer/Test/Installer/SuggestedPackagesReporterTest.php:226-228` | 6/6 (100.0%) | 6/6 (100.0%) |
| composer | OUT | `src/Composer/Command/CheckPlatformReqsCommand.php:32-33` + `src/Composer/Command/LicensesCommand.php:41-42` | — | — |
| composer | OUT | `src/Composer/Command/InitCommand.php:56-57` + `src/Composer/Command/CreateProjectCommand.php:90-91` | — | — |
| composer | OUT | `src/Composer/Package/Dumper/ArrayDumper.php:116-126` + `tests/Composer/Test/Util/PerforceTest.php:131-131` | — | — |
| composer | OUT | `src/Composer/Command/BumpCommand.php:48-49` + `src/Composer/Command/SearchCommand.php:40-41` | — | — |
| composer | OUT | `tests/Composer/Test/DependencyResolver/SolverTest.php:741-742` + `tests/Composer/Test/DependencyResolver/SolverTest.php:833-838` | 5/8 (62.5%) | 5/8 (62.5%) |
| composer | OUT | `src/Composer/FilterList/FilterListEntry.php:27-57` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:61-94` | — | — |
| composer | OUT | `src/Composer/DependencyResolver/PoolBuilder.php:47-78` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:56-94` | — | — |
| composer | OUT | `src/Composer/Package/Loader/ValidatingArrayLoader.php:39-49` + `src/Composer/DependencyResolver/Solver.php:32-45` | — | — |
| composer | OUT | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:66-94` + `src/Composer/Advisory/SecurityAdvisory.php:24-54` | — | — |
| composer | OUT | `src/Composer/Command/BaseCommand.php:34-39` + `src/Composer/IO/BufferIO.php:16-21` | — | — |
| composer | OUT | `tests/Composer/Test/Util/FilesystemTest.php:68-70` + `tests/Composer/Test/Util/FilesystemTest.php:98-100` | — | — |
| composer | OUT | `src/Composer/Command/ValidateCommand.php:44-45` + `src/Composer/Command/InitCommand.php:56-57` | — | — |
| composer | OUT | `src/Composer/Repository/VcsRepository.php:46-60` + `src/Composer/Installer.php:104-140` | — | — |
| composer | OUT | `src/Composer/DependencyResolver/Request.php:44-54` + `src/Composer/DependencyResolver/PoolOptimizer.php:34-59` | — | — |
| composer | OUT | `src/Composer/Repository/VcsRepository.php:58-72` + `src/Composer/Config.php:115-129` | — | — |
| console | IN | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:135-147` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:97-109` | 26/26 (100.0%) | 26/26 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:267-283` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:214-230` | 34/34 (100.0%) | 34/34 (100.0%) |
| console | IN | `Tests/Input/InputArgumentTest.php:149-180` + `Tests/Input/InputOptionTest.php:319-350` | 64/64 (100.0%) | 64/64 (100.0%) |
| console | IN | `Tests/ApplicationTest.php:2348-2399` + `Tests/ApplicationTest.php:2401-2452` | 104/104 (100.0%) | 104/104 (100.0%) |
| console | IN | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:169-182` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:136-149` | 28/28 (100.0%) | 28/28 (100.0%) |
| console | IN | `Tests/ApplicationTest.php:2036-2047` + `Tests/ApplicationTest.php:1792-1803` | 24/24 (100.0%) | 24/24 (100.0%) |
| console | IN | `Tests/CommandLoader/FactoryCommandLoaderTest.php:1-59` + `Tests/CommandLoader/ContainerCommandLoaderTest.php:1-60` | 119/119 (100.0%) | 119/119 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:236-282` + `Tests/ArgumentResolver/ValueResolver/VariadicValueResolverTest.php:98-138` | 88/88 (100.0%) | 88/88 (100.0%) |
| console | IN | `Descriptor/ReStructuredTextDescriptor.php:40-92` + `Descriptor/MarkdownDescriptor.php:31-78` | 101/101 (100.0%) | 101/101 (100.0%) |
| console | IN | `Tests/Helper/FormatterHelperTest.php:56-67` + `Tests/Helper/FormatterHelperTest.php:69-79` | 23/23 (100.0%) | 23/23 (100.0%) |
| console | IN | `Tests/Style/SymfonyStyleTest.php:282-296` + `Tests/Style/SymfonyStyleTest.php:396-411` | 31/31 (100.0%) | 31/31 (100.0%) |
| console | IN | `Tests/Tester/CommandTesterTest.php:241-267` + `Tests/Tester/CommandTesterTest.php:212-239` | 55/55 (100.0%) | 55/55 (100.0%) |
| console | IN | `Tests/Helper/QuestionHelperTest.php:740-751` + `Tests/Helper/QuestionHelperTest.php:759-770` | 24/24 (100.0%) | 24/24 (100.0%) |
| console | IN | `Exception/InvalidOptionException.php:1-42` + `Exception/InvalidArgumentException.php:1-40` | 82/82 (100.0%) | 82/82 (100.0%) |
| console | IN | `Tests/Helper/TreeHelperTest.php:66-75` + `Tests/Helper/TreeHelperTest.php:262-271` | 20/20 (100.0%) | 20/20 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:362-375` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:36-49` | 28/28 (100.0%) | 28/28 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:289-317` + `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:263-287` | 54/54 (100.0%) | 54/54 (100.0%) |
| console | IN | `Tests/Terminal/Image/ITerm2ProtocolTest.php:81-108` + `Tests/Terminal/Image/KittyGraphicsProtocolTest.php:70-106` | 65/65 (100.0%) | 65/65 (100.0%) |
| console | IN | `Tests/Completion/Output/FishCompletionOutputTest.php:1-33` + `Tests/Completion/Output/ZshCompletionOutputTest.php:1-33` | — | — |
| console | IN | `Tests/Helper/TableTest.php:1853-1879` + `Tests/Helper/TableTest.php:1881-1914` | 61/61 (100.0%) | 14/61 (23.0%) |
| console | IN | `Tests/ApplicationTest.php:2314-2327` + `Tests/ApplicationTest.php:2810-2824` | 29/29 (100.0%) | 29/29 (100.0%) |
| console | IN | `Tests/Helper/TableTest.php:1396-1413` + `Tests/Helper/TableTest.php:1328-1345` | 36/36 (100.0%) | 36/36 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:911-919` + `Tests/Helper/ProgressBarTest.php:502-510` | 18/18 (100.0%) | 18/18 (100.0%) |
| console | IN | `Completion/Output/FishCompletionOutput.php:1-36` + `Completion/Output/ZshCompletionOutput.php:1-36` | 72/72 (100.0%) | 72/72 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:60-94` + `Tests/ArgumentResolver/ValueResolver/DefaultValueResolverTest.php:51-79` | 64/64 (100.0%) | 64/64 (100.0%) |
| console | IN | `Event/ConsoleAlarmEvent.php:1-47` + `Event/ConsoleSignalEvent.php:1-56` | 103/103 (100.0%) | 103/103 (100.0%) |
| console | IN | `Tests/Helper/QuestionHelperTest.php:1117-1131` + `Tests/Helper/QuestionHelperTest.php:1158-1168` | 26/26 (100.0%) | 26/26 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:237-352` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:211-300` | 206/206 (100.0%) | 206/206 (100.0%) |
| console | IN | `Tests/Helper/TableTest.php:1855-1863` + `Tests/Helper/TableTest.php:1532-1540` | 18/18 (100.0%) | 14/18 (77.8%) |
| console | IN | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:475-485` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:427-437` | 22/22 (100.0%) | 22/22 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:195-209` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:220-234` | 30/30 (100.0%) | 30/30 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:118-131` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:165-178` | 28/28 (100.0%) | 28/28 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:161-173` + `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:36-48` | 26/26 (100.0%) | 26/26 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:260-326` + `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:50-109` | 127/127 (100.0%) | 127/127 (100.0%) |
| console | IN | `Tests/SignalRegistry/SignalRegistryTest.php:185-207` + `Tests/SignalRegistry/SignalRegistryTest.php:209-231` | 46/46 (100.0%) | 46/46 (100.0%) |
| console | IN | `Tests/Input/File/InputFileTest.php:149-159` + `Tests/Input/File/InputFileTest.php:137-147` | 22/22 (100.0%) | 22/22 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:834-841` + `Tests/Helper/ProgressBarTest.php:1236-1243` | 16/16 (100.0%) | 16/16 (100.0%) |
| console | IN | `Tests/Terminal/Image/KittyGraphicsProtocolTest.php:19-48` + `Tests/Terminal/Image/ITerm2ProtocolTest.php:19-47` | 59/59 (100.0%) | 59/59 (100.0%) |
| console | IN | `Tests/Descriptor/MarkdownDescriptorTest.php:1-45` + `Tests/Descriptor/ReStructuredTextDescriptorTest.php:1-45` | 90/90 (100.0%) | 90/90 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/DateTimeValueResolverTest.php:245-265` + `Tests/ArgumentResolver/ValueResolver/DateTimeValueResolverTest.php:97-117` | 42/42 (100.0%) | 42/42 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:163-184` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:140-161` | 44/44 (100.0%) | 44/44 (100.0%) |
| console | IN | `Tests/ApplicationTest.php:2851-2861` + `Tests/ApplicationTest.php:2335-2345` | 22/22 (100.0%) | 22/22 (100.0%) |
| console | IN | `Tests/Input/ArgvInputTest.php:664-755` + `Tests/Input/ArrayInputTest.php:342-449` | 200/200 (100.0%) | 200/200 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:1817-1825` + `Tests/Helper/ProgressBarTest.php:1840-1850` | 20/20 (100.0%) | 20/20 (100.0%) |
| console | IN | `Tests/Input/InputOptionTest.php:98-104` + `Tests/Input/InputOptionTest.php:91-97` | 14/14 (100.0%) | 14/14 (100.0%) |
| console | IN | `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:116-125` + `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:59-68` | — | — |
| console | IN | `Tests/Helper/TreeHelperTest.php:133-145` + `Tests/Helper/TreeHelperTest.php:119-131` | 26/26 (100.0%) | 26/26 (100.0%) |
| console | IN | `Tests/ColorTest.php:51-64` + `Tests/ColorTest.php:36-49` | 28/28 (100.0%) | 28/28 (100.0%) |
| console | IN | `Tests/Helper/TreeHelperTest.php:103-116` + `Tests/Helper/TreeHelperTest.php:50-60` | 25/25 (100.0%) | 25/25 (100.0%) |
| console | IN | `Tests/DependencyInjection/RegisterCommandArgumentLocatorsPassTest.php:347-354` + `Tests/DependencyInjection/RegisterCommandArgumentLocatorsPassTest.php:365-372` | 16/16 (100.0%) | 16/16 (100.0%) |
| console | IN | `Tests/Input/InputArgumentTest.php:139-170` + `Tests/Input/InputOptionTest.php:301-340` | 72/72 (100.0%) | 72/72 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:587-594` + `Tests/Helper/ProgressBarTest.php:612-619` | 16/16 (100.0%) | 16/16 (100.0%) |
| console | IN | `Tests/DependencyInjection/ConsoleArgumentValueResolverPassTest.php:56-91` + `Tests/DependencyInjection/ConsoleArgumentValueResolverPassTest.php:25-54` | 66/66 (100.0%) | 66/66 (100.0%) |
| console | IN | `Tests/Helper/TreeHelperTest.php:63-86` + `Tests/Helper/TreeHelperTest.php:259-282` | 48/48 (100.0%) | 48/48 (100.0%) |
| console | IN | `Tests/Input/ArgvInputTest.php:848-907` + `Tests/Input/ArrayInputTest.php:522-595` | 134/134 (100.0%) | 134/134 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/VariadicValueResolverTest.php:50-138` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:262-352` | 180/180 (100.0%) | 180/180 (100.0%) |
| console | IN | `Attribute/Argument.php:86-112` + `Attribute/Option.php:113-146` | 61/61 (100.0%) | 61/61 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:1682-1690` + `Tests/Helper/ProgressBarTest.php:1867-1875` | 18/18 (100.0%) | 18/18 (100.0%) |
| console | IN | `Tests/Input/ArrayInputTest.php:542-550` + `Tests/Input/ArrayInputTest.php:552-560` | 18/18 (100.0%) | 18/18 (100.0%) |
| console | IN | `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:267-283` + `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:241-257` | 34/34 (100.0%) | 34/34 (100.0%) |
| console | IN | `Tester/Constraint/CommandIsInvalid.php:1-43` + `Tester/Constraint/CommandFailed.php:1-43` | 86/86 (100.0%) | 86/86 (100.0%) |
| console | IN | `Tests/Helper/ProgressBarTest.php:348-357` + `Tests/Helper/ProgressBarTest.php:330-339` | 20/20 (100.0%) | 20/20 (100.0%) |
| console | OUT | `Output/OutputInterface.php:23-28` + `Helper/Table.php:34-39` | — | — |
| date-fns | IN | `pkgs/core/src/locale/ru/_lib/formatDistance/index.ts:116-252` + `pkgs/core/src/locale/uk/_lib/formatDistance/index.ts:110-246` | 274/274 (100.0%) | 274/274 (100.0%) |
| date-fns | IN | `pkgs/core/src/differenceInWeeks/test.ts:83-150` + `pkgs/core/src/differenceInQuarters/test.ts:82-149` | 136/136 (100.0%) | 136/136 (100.0%) |
| date-fns | IN | `pkgs/core/src/getWeekOfMonth/test.ts:71-102` + `pkgs/core/src/getWeek/test.ts:47-83` | 69/69 (100.0%) | 69/69 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/ar-EG/_lib/match/index.ts:94-107` + `pkgs/core/src/locale/ja/_lib/match/index.ts:77-90` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/tz/playground.mjs:112-120` + `pkgs/tz/playground.mjs:59-67` | 18/18 (100.0%) | 18/18 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/bs/_lib/match/index.ts:47-60` + `pkgs/core/src/locale/de/_lib/match/index.ts:48-61` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/mk/_lib/formatDistance/index.ts:10-110` + `pkgs/core/src/locale/gl/_lib/formatDistance/index.ts:10-110` | 202/202 (100.0%) | 202/202 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/pt/_lib/formatDistance/index.ts:10-87` + `pkgs/core/src/locale/ro/_lib/formatDistance/index.ts:10-87` | 78/156 (50.0%) | 78/156 (50.0%) |
| date-fns | IN | `pkgs/core/src/subISOWeekYears/test.ts:63-74` + `pkgs/core/src/addDays/test.ts:48-59` | 24/24 (100.0%) | 24/24 (100.0%) |
| date-fns | IN | `pkgs/core/src/isSameHour/test.ts:31-44` + `pkgs/core/src/isBefore/test.ts:37-50` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/pt-BR/_lib/match/index.ts:1-135` + `pkgs/core/src/locale/cy/_lib/match/index.ts:1-143` | 278/278 (100.0%) | 278/278 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/tr/_lib/match/index.ts:107-151` + `pkgs/core/src/locale/ja/_lib/match/index.ts:76-120` | 90/90 (100.0%) | 90/90 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/bg/_lib/formatRelative/index.ts:21-113` + `pkgs/core/src/locale/be/_lib/formatRelative/index.ts:1-107` | 200/200 (100.0%) | 200/200 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/_lib/parsers/SecondParser.ts:8-26` + `pkgs/core/src/parse/_lib/parsers/Hour0to23Parser.ts:8-26` | 38/38 (100.0%) | 38/38 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/ar-TN/_lib/match/index.ts:33-82` + `pkgs/core/src/locale/ar/_lib/match/index.ts:33-82` | 100/100 (100.0%) | 100/100 (100.0%) |
| date-fns | IN | `pkgs/core/src/getISOWeek/test.ts:44-75` + `pkgs/core/src/getDaysInMonth/test.ts:23-58` | 68/68 (100.0%) | 68/68 (100.0%) |
| date-fns | IN | `pkgs/tz/src/date/tests.ts:1696-1707` + `pkgs/tz/src/date/tests.ts:1709-1720` | 24/24 (100.0%) | 24/24 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/test.ts:530-593` + `pkgs/core/src/parse/test.ts:595-658` | 128/128 (100.0%) | 128/128 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/_lib/utils.ts:77-93` + `pkgs/core/src/parse/_lib/utils.ts:95-111` | 34/34 (100.0%) | 34/34 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/test.ts:944-967` + `pkgs/core/src/parse/test.ts:1016-1039` | 48/48 (100.0%) | 48/48 (100.0%) |
| date-fns | IN | `pkgs/core/src/_lib/format/formatters/index.ts:261-290` + `pkgs/core/src/_lib/format/formatters/index.ts:232-258` | 57/57 (100.0%) | 57/57 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/ca/_lib/match/index.ts:29-85` + `pkgs/core/src/locale/az/_lib/match/index.ts:27-82` | 113/113 (100.0%) | 113/113 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/fr/_lib/match/index.ts:79-134` + `pkgs/core/src/locale/de/_lib/match/index.ts:80-135` | 112/112 (100.0%) | 112/112 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/en-US/_lib/match/index.ts:18-89` + `pkgs/core/src/locale/da/_lib/match/index.ts:18-89` | 144/144 (100.0%) | 144/144 (100.0%) |
| date-fns | IN | `pkgs/tz/playground.mjs:28-34` + `pkgs/tz/playground.mjs:112-118` | 14/14 (100.0%) | 14/14 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/_lib/parsers/MonthParser.ts:23-90` + `pkgs/core/src/parse/_lib/parsers/StandAloneMonthParser.ts:8-75` | 136/136 (100.0%) | 136/136 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/zh-TW/test.ts:1-33` + `pkgs/core/src/locale/zh-HK/test.ts:1-33` | 66/66 (100.0%) | 66/66 (100.0%) |
| date-fns | IN | `pkgs/core/src/setISOWeekYear/test.ts:36-91` + `pkgs/core/src/subISOWeekYears/test.ts:29-82` | 110/110 (100.0%) | 110/110 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/lv/_lib/localize/index.ts:119-249` + `pkgs/core/src/locale/lt/_lib/localize/index.ts:111-233` | 254/254 (100.0%) | 254/254 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/test.ts:631-638` + `pkgs/core/src/parse/test.ts:290-297` | 16/16 (100.0%) | 16/16 (100.0%) |
| date-fns | IN | `pkgs/core/src/differenceInCalendarMonths/test.ts:7-98` + `pkgs/core/src/differenceInCalendarYears/test.ts:7-98` | 184/184 (100.0%) | 184/184 (100.0%) |
| date-fns | IN | `pkgs/core/src/differenceInQuarters/index.ts:36-43` + `pkgs/core/src/differenceInWeeks/index.tp.ts:6-13` | 16/16 (100.0%) | 16/16 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/bg/_lib/match/index.ts:1-128` + `pkgs/core/src/locale/mk/_lib/match/index.ts:1-126` | 254/254 (100.0%) | 254/254 (100.0%) |
| date-fns | IN | `pkgs/core/src/getOverlappingDaysInIntervals/test.ts:42-76` + `pkgs/core/src/areIntervalsOverlapping/test.ts:33-64` | 67/67 (100.0%) | 67/67 (100.0%) |
| date-fns | IN | `pkgs/tz/src/date/tests.ts:1378-1393` + `pkgs/tz/src/date/tests.ts:1365-1376` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/fr/_lib/match/index.ts:99-133` + `pkgs/core/src/locale/eu/_lib/match/index.ts:128-162` | 70/70 (100.0%) | 70/70 (100.0%) |
| date-fns | IN | `pkgs/core/src/endOfHour/index.ts:1-50` + `pkgs/core/src/startOfHour/index.ts:1-43` | 93/93 (100.0%) | 93/93 (100.0%) |
| date-fns | IN | `pkgs/core/src/endOfQuarter/index.ts:7-53` + `pkgs/core/src/lastDayOfQuarter/index.ts:7-46` | 87/87 (100.0%) | 87/87 (100.0%) |
| date-fns | IN | `pkgs/core/src/setQuarter/test.ts:7-84` + `pkgs/core/src/setMonth/test.ts:7-84` | 156/156 (100.0%) | 156/156 (100.0%) |
| date-fns | IN | `pkgs/core/src/isSameISOWeek/test.ts:7-59` + `pkgs/core/src/isSameQuarter/test.ts:7-59` | 106/106 (100.0%) | 106/106 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/_lib/parsers/LocalWeekParser.ts:13-22` + `pkgs/core/src/parse/_lib/parsers/ISOWeekParser.ts:13-22` | 20/20 (100.0%) | 20/20 (100.0%) |
| date-fns | IN | `pkgs/core/examples/ts/test.ts:5-20` + `pkgs/core/examples/cts/test.ts:1-20` | 36/36 (100.0%) | 36/36 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/is/_lib/match/index.ts:14-79` + `pkgs/core/src/locale/mt/_lib/match/index.ts:14-86` | 139/139 (100.0%) | 139/139 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/lt/_lib/localize/index.ts:198-218` + `pkgs/core/src/locale/vi/_lib/localize/index.ts:270-290` | 42/42 (100.0%) | 42/42 (100.0%) |
| date-fns | IN | `pkgs/core/examples/cdn/legacyStrict.js:1-47` + `pkgs/core/examples/cdn/legacy.js:1-36` | 83/83 (100.0%) | 83/83 (100.0%) |
| date-fns | IN | `pkgs/core/src/constructFrom/test.ts:10-17` + `pkgs/core/src/constructFrom/test.ts:32-39` | 16/16 (100.0%) | 16/16 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/cy/_lib/formatDistance/index.ts:102-113` + `pkgs/core/src/locale/ca/_lib/formatDistance/index.ts:105-116` | 24/24 (100.0%) | 24/24 (100.0%) |
| date-fns | IN | `pkgs/core/src/differenceInCalendarYears/test.ts:31-98` + `pkgs/core/src/differenceInCalendarQuarters/test.ts:31-98` | 136/136 (100.0%) | 136/136 (100.0%) |
| date-fns | IN | `pkgs/core/src/format/test.ts:244-263` + `pkgs/core/src/format/test.ts:132-151` | 40/40 (100.0%) | 40/40 (100.0%) |
| date-fns | IN | `pkgs/core/src/nextTuesday/test.ts:42-52` + `pkgs/core/src/roundToNearestMinutes/test.ts:264-274` | 22/22 (100.0%) | 22/22 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/zh-TW/_lib/localize/index.ts:1-197` + `pkgs/core/src/locale/zh-HK/_lib/localize/index.ts:1-197` | 394/394 (100.0%) | 394/394 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/pt/_lib/localize/index.ts:10-19` + `pkgs/core/src/locale/pt-BR/_lib/localize/index.ts:10-19` | 20/20 (100.0%) | 20/20 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/zh-HK/_lib/formatDistance/index.ts:73-86` + `pkgs/core/src/locale/zh-TW/_lib/formatDistance/index.ts:73-86` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/core/src/isLastDayOfMonth/test.ts:7-15` + `pkgs/core/src/isTomorrow/test.ts:11-19` | 18/18 (100.0%) | 18/18 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/test.ts:531-559` + `pkgs/core/src/parse/test.ts:399-427` | 58/58 (100.0%) | 58/58 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/test.ts:1011-1044` + `pkgs/core/src/parse/test.ts:1090-1123` | 68/68 (100.0%) | 68/68 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/index.ts:374-379` + `pkgs/core/src/lastDayOfWeek/index.ts:41-46` | 12/12 (100.0%) | 12/12 (100.0%) |
| date-fns | IN | `pkgs/core/src/endOfISOWeekYear/test.ts:13-40` + `pkgs/core/src/startOfWeekYear/test.ts:13-40` | 56/56 (100.0%) | 56/56 (100.0%) |
| date-fns | IN | `pkgs/core/src/parse/_lib/parsers/ISOTimezoneWithZParser.ts:1-50` + `pkgs/core/src/parse/_lib/parsers/ISOTimezoneParser.ts:1-50` | 100/100 (100.0%) | 100/100 (100.0%) |
| date-fns | IN | `pkgs/core/src/eachWeekendOfMonth/test.ts:41-54` + `pkgs/core/src/eachWeekendOfMonth/test.ts:55-68` | 28/28 (100.0%) | 28/28 (100.0%) |
| date-fns | IN | `pkgs/core/src/locale/kk/_lib/formatRelative/index.ts:52-64` + `pkgs/core/src/locale/mk/_lib/formatRelative/index.ts:82-94` | — | — |
| Deslop | IN | `crates/deslop/tests/python_issue_104_module_preamble.rs:24-33` + `crates/deslop/tests/python_issue_133_constant_table.rs:45-54` | 20/20 (100.0%) | 20/20 (100.0%) |
| Deslop | IN | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:243-251` + `crates/deslop-lsp/tests/observability_heartbeat.rs:237-245` | 18/18 (100.0%) | 18/18 (100.0%) |
| Deslop | IN | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:253-258` + `crates/deslop-lsp/tests/observability_heartbeat.rs:248-253` | 12/12 (100.0%) | 12/12 (100.0%) |
| Deslop | IN | `crates/deslop-test-support/src/skip_policy.rs:394-400` + `crates/deslop-test-support/src/test_target_parity/suite_scan.rs:190-196` | 14/14 (100.0%) | 14/14 (100.0%) |
| Deslop | IN | `crates/deslop-core/src/pipeline/corpus.rs:128-133` + `crates/deslop-core/src/pipeline/session/mod.rs:459-464` | 12/12 (100.0%) | 12/12 (100.0%) |
| Deslop | IN | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:218-222` + `crates/deslop-lsp/tests/observability_heartbeat.rs:231-235` | 10/10 (100.0%) | 10/10 (100.0%) |
| Deslop | IN | `crates/deslop/tests/common/corpora.rs:79-84` + `crates/deslop/tests/common/corpora.rs:93-98` | 12/12 (100.0%) | 12/12 (100.0%) |
| Deslop | IN | `crates/deslop-core/tests/content_gate_admits.rs:63-76` + `crates/deslop-core/tests/content_gate_rejects.rs:47-60` | 28/28 (100.0%) | 28/28 (100.0%) |
| Deslop | IN | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:260-268` + `crates/deslop-lsp/tests/observability_heartbeat.rs:255-263` | 18/18 (100.0%) | 18/18 (100.0%) |
| Deslop | IN | `crates/deslop-mcp/tests/cli.rs:1801-1808` + `crates/deslop-mcp/tests/cli.rs:2761-2768` | 16/16 (100.0%) | 16/16 (100.0%) |
| Deslop | IN | `crates/deslop-core/src/cluster_filters/structural_families/tests.rs:305-314` + `crates/deslop-core/src/cluster_filters/structural_families/tests.rs:357-366` | 20/20 (100.0%) | 20/20 (100.0%) |
| Deslop | IN | `crates/deslop-core/src/refactor/emit.rs:108-113` + `crates/deslop-core/src/refactor/merge/mod.rs:245-250` | 12/12 (100.0%) | 12/12 (100.0%) |
| Deslop | IN | `crates/deslop/tests/cli/thresholds.rs:131-137` + `crates/deslop/tests/cli/thresholds.rs:161-167` | 14/14 (100.0%) | 14/14 (100.0%) |
| Deslop | IN | `crates/deslop-core/src/live/session.rs:753-759` + `crates/deslop-core/src/live/session.rs:779-785` | 14/14 (100.0%) | 14/14 (100.0%) |
| Deslop | IN | `crates/deslop/tests/cli/invocation.rs:29-35` + `crates/deslop-lsp/tests/cli.rs:324-330` | 14/14 (100.0%) | 14/14 (100.0%) |
| Deslop | OUT | `crates/deslop-core/src/cluster_filters/calls/args.rs:134-139` + `crates/deslop-core/src/cluster_filters/calls/asserts.rs:164-168` | — | — |
| dio | IN | `dio/test/interceptor_test.dart:1585-1628` + `dio/test/interceptor_test.dart:1541-1583` | 87/87 (100.0%) | 87/87 (100.0%) |
| dio | OUT | `dio_test/lib/src/test/headers_tests.dart:49-51` + `plugins/web_adapter/test/cors_preflight_test.dart:203-206` | — | — |
| dio | OUT | `dio/lib/src/options.dart:508-529` + `dio/lib/src/dio_exception.dart:214-228` | — | — |
| dio | OUT | `dio/lib/src/options.dart:680-694` + `dio/lib/src/response.dart:33-49` | — | — |
| django | IN | `tests/timezones/tests.py:493-511` + `tests/timezones/tests.py:212-230` | 38/38 (100.0%) | 38/38 (100.0%) |
| django | IN | `tests/model_meta/results.py:482-529` + `tests/model_meta/results.py:356-403` | 96/96 (100.0%) | 96/96 (100.0%) |
| django | IN | `tests/migrations/test_operations.py:4388-4412` + `tests/migrations/test_operations.py:4319-4343` | 50/50 (100.0%) | 50/50 (100.0%) |
| django | IN | `django/core/management/commands/sqlsequencereset.py:10-22` + `django/core/management/commands/sqlflush.py:12-24` | 26/26 (100.0%) | 26/26 (100.0%) |
| django | IN | `tests/flatpages_tests/test_csrf.py:72-105` + `tests/flatpages_tests/test_middleware.py:71-104` | 68/68 (100.0%) | 68/68 (100.0%) |
| django | IN | `tests/flatpages_tests/test_middleware.py:17-52` + `tests/flatpages_tests/test_csrf.py:32-67` | 72/72 (100.0%) | 72/72 (100.0%) |
| django | IN | `tests/flatpages_tests/test_templatetags.py:13-47` + `tests/flatpages_tests/test_views.py:15-49` | 70/70 (100.0%) | 70/70 (100.0%) |
| django | IN | `django/db/models/fields/related_descriptors.py:578-590` + `django/db/models/fields/related_descriptors.py:297-309` | 26/26 (100.0%) | 26/26 (100.0%) |
| django | IN | `tests/migrations/test_commands.py:1973-1987` + `tests/migrations/test_commands.py:1895-1909` | 30/30 (100.0%) | 30/30 (100.0%) |
| django | IN | `tests/admin_widgets/tests.py:1442-1452` + `tests/admin_widgets/tests.py:1455-1465` | 22/22 (100.0%) | 22/22 (100.0%) |
| django | IN | `tests/raw_query/tests.py:260-267` + `tests/raw_query/tests.py:249-256` | 16/16 (100.0%) | 16/16 (100.0%) |
| django | IN | `tests/backends/tests.py:719-726` + `tests/backends/tests.py:638-645` | 16/16 (100.0%) | 16/16 (100.0%) |
| django | IN | `tests/multiple_database/tests.py:939-946` + `tests/multiple_database/tests.py:922-929` | 16/16 (100.0%) | 16/16 (100.0%) |
| django | IN | `tests/model_meta/results.py:546-563` + `tests/model_meta/results.py:588-605` | 36/36 (100.0%) | 36/36 (100.0%) |
| django | IN | `tests/servers/tests.py:265-271` + `tests/servers/tests.py:273-279` | 14/14 (100.0%) | 14/14 (100.0%) |
| django | OUT | `tests/forms_tests/widget_tests/test_choicewidget.py:53-56` + `tests/admin_filters/tests.py:1721-1723` | — | — |
| django | OUT | `tests/requests_tests/tests.py:1472-1474` + `tests/test_client_regress/tests.py:1410-1412` | — | — |
| django | OUT | `tests/runtests.py:548-558` + `tests/runtests.py:650-660` | — | — |
| django | OUT | `tests/runtests.py:669-679` + `django/contrib/gis/management/commands/ogrinspect.py:100-109` | — | — |
| django | OUT | `tests/validators/tests.py:343-347` + `tests/validators/tests.py:367-371` | — | — |
| django | OUT | `tests/template_backends/test_jinja2.py:129-132` + `tests/responses/test_cookie.py:132-135` | — | — |
| django | OUT | `tests/auth_tests/test_views.py:168-170` + `tests/middleware/tests.py:405-407` | — | — |
| django | OUT | `tests/forms_tests/field_tests/test_decimalfield.py:64-81` + `tests/humanize_tests/tests.py:64-81` | — | — |
| django | OUT | `django/db/migrations/operations/models.py:818-830` + `tests/utils_tests/test_dateformat.py:200-200` | — | — |
| django | OUT | `tests/invalid_models_tests/test_relative_fields.py:313-318` + `tests/migrations/migrations_test_apps/lookuperror_a/models.py:8-18` | 17/17 (100.0%) | 17/17 (100.0%) |
| django | OUT | `tests/test_utils/tests.py:725-735` + `tests/migrations/test_commands.py:1363-1365` | — | — |
| django | OUT | `tests/custom_lookups/tests.py:656-657` + `tests/admin_views/tests.py:827-828` | — | — |
| django | OUT | `django/contrib/staticfiles/management/commands/collectstatic.py:40-90` + `tests/runtests.py:644-688` | — | — |
| express | IN | `test/res.cookie.js:189-201` + `test/res.cookie.js:217-229` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/app.response.js:111-141` + `test/app.response.js:79-109` | 62/62 (100.0%) | 62/62 (100.0%) |
| express | IN | `test/express.raw.js:179-185` + `test/express.raw.js:466-472` | 14/14 (100.0%) | 14/14 (100.0%) |
| express | IN | `test/express.urlencoded.js:263-293` + `test/express.json.js:143-173` | 62/62 (100.0%) | 62/62 (100.0%) |
| express | IN | `test/express.static.js:265-325` + `test/express.static.js:340-400` | 122/122 (100.0%) | 122/122 (100.0%) |
| express | IN | `test/express.urlencoded.js:241-314` + `test/express.text.js:95-169` | 149/149 (100.0%) | 149/149 (100.0%) |
| express | IN | `test/express.urlencoded.js:111-121` + `test/express.json.js:242-252` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/app.router.js:966-983` + `test/app.router.js:1071-1093` | 41/41 (100.0%) | 41/41 (100.0%) |
| express | IN | `test/req.subdomains.js:12-19` + `test/req.subdomains.js:84-91` | 16/16 (100.0%) | 16/16 (100.0%) |
| express | IN | `test/express.json.js:77-87` + `test/express.json.js:60-70` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/req.hostname.js:8-32` + `test/req.hostname.js:47-71` | 50/50 (100.0%) | 50/50 (100.0%) |
| express | IN | `test/res.sendFile.js:617-626` + `test/res.sendFile.js:637-646` | 20/20 (100.0%) | 20/20 (100.0%) |
| express | IN | `test/express.raw.js:86-187` + `test/express.text.js:95-199` | 207/207 (100.0%) | 207/207 (100.0%) |
| express | IN | `test/res.status.js:106-116` + `test/res.status.js:34-44` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/app.param.js:71-85` + `test/app.param.js:306-320` | 30/30 (100.0%) | 30/30 (100.0%) |
| express | IN | `test/express.urlencoded.js:372-425` + `test/express.urlencoded.js:317-370` | 108/108 (100.0%) | 108/108 (100.0%) |
| express | IN | `test/app.router.js:437-449` + `test/app.router.js:491-503` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/express.json.js:186-193` + `test/express.urlencoded.js:306-313` | 16/16 (100.0%) | 16/16 (100.0%) |
| express | IN | `test/express.raw.js:285-296` + `test/express.json.js:476-487` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/express.urlencoded.js:139-153` + `test/express.urlencoded.js:93-107` | 30/30 (100.0%) | 30/30 (100.0%) |
| express | IN | `test/express.text.js:171-199` + `test/express.json.js:196-224` | 58/58 (100.0%) | 58/58 (100.0%) |
| express | IN | `test/req.hostname.js:60-71` + `test/req.hostname.js:8-19` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/app.use.js:490-495` + `test/app.use.js:497-502` | 12/12 (100.0%) | 12/12 (100.0%) |
| express | IN | `test/express.text.js:496-545` + `test/express.urlencoded.js:735-784` | 100/100 (100.0%) | 100/100 (100.0%) |
| express | IN | `test/express.urlencoded.js:11-21` + `test/express.text.js:10-20` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/res.append.js:17-26` + `test/res.append.js:109-118` | 20/20 (100.0%) | 20/20 (100.0%) |
| express | IN | `test/res.sendFile.js:578-594` + `test/res.sendFile.js:596-612` | 34/34 (100.0%) | 34/34 (100.0%) |
| express | IN | `test/express.text.js:201-293` + `test/express.json.js:297-391` | 188/188 (100.0%) | 188/188 (100.0%) |
| express | IN | `test/express.raw.js:330-359` + `test/express.json.js:506-534` | 59/59 (100.0%) | 59/59 (100.0%) |
| express | IN | `test/app.render.js:418-446` + `test/app.render.js:448-476` | 58/58 (100.0%) | 58/58 (100.0%) |
| express | IN | `test/acceptance/mvc.js:33-44` + `test/acceptance/mvc.js:103-114` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/res.redirect.js:179-192` + `test/res.redirect.js:131-144` | 28/28 (100.0%) | 28/28 (100.0%) |
| express | IN | `test/express.urlencoded.js:646-696` + `test/express.text.js:400-450` | 102/102 (100.0%) | 102/102 (100.0%) |
| express | IN | `test/express.text.js:151-156` + `test/express.raw.js:141-146` | 12/12 (100.0%) | 12/12 (100.0%) |
| express | IN | `test/express.urlencoded.js:607-665` + `test/express.text.js:361-419` | 118/118 (100.0%) | 118/118 (100.0%) |
| express | IN | `test/req.ips.js:10-38` + `test/req.ip.js:10-38` | 58/58 (100.0%) | 58/58 (100.0%) |
| express | IN | `test/req.stale.js:8-48` + `test/req.fresh.js:9-49` | 82/82 (100.0%) | 82/82 (100.0%) |
| express | IN | `test/express.json.js:255-294` + `test/express.urlencoded.js:111-145` | 75/75 (100.0%) | 75/75 (100.0%) |
| express | IN | `test/req.is.js:149-191` + `test/req.is.js:193-235` | 86/86 (100.0%) | 86/86 (100.0%) |
| express | IN | `test/req.query.js:75-83` + `test/req.query.js:65-73` | 18/18 (100.0%) | 18/18 (100.0%) |
| express | IN | `test/express.json.js:504-574` + `test/express.urlencoded.js:607-676` | 141/141 (100.0%) | 141/141 (100.0%) |
| express | IN | `test/req.host.js:95-103` + `test/req.host.js:79-87` | 18/18 (100.0%) | 18/18 (100.0%) |
| express | IN | `test/express.json.js:367-389` + `test/express.raw.js:253-275` | 46/46 (100.0%) | 46/46 (100.0%) |
| express | IN | `test/app.request.js:79-109` + `test/app.request.js:111-141` | 62/62 (100.0%) | 62/62 (100.0%) |
| express | IN | `test/app.use.js:92-100` + `test/app.use.js:473-481` | 18/18 (100.0%) | 18/18 (100.0%) |
| express | IN | `test/app.use.js:420-446` + `test/app.use.js:229-255` | 54/54 (100.0%) | 54/54 (100.0%) |
| express | IN | `test/express.json.js:634-646` + `test/express.text.js:458-470` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/express.static.js:619-631` + `test/express.static.js:598-610` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/express.raw.js:195-207` + `test/express.raw.js:224-236` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/req.accepts.js:33-44` + `test/req.accepts.js:20-31` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/express.urlencoded.js:525-576` + `test/express.json.js:394-445` | 104/104 (100.0%) | 104/104 (100.0%) |
| express | IN | `test/app.use.js:297-307` + `test/app.use.js:230-240` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/express.raw.js:299-306` + `test/express.urlencoded.js:545-552` | 16/16 (100.0%) | 16/16 (100.0%) |
| express | IN | `test/res.location.js:183-214` + `test/res.location.js:260-291` | 64/64 (100.0%) | 64/64 (100.0%) |
| express | IN | `test/express.text.js:86-92` + `test/express.text.js:78-84` | 14/14 (100.0%) | 14/14 (100.0%) |
| express | IN | `test/app.router.js:119-129` + `test/app.router.js:95-105` | — | — |
| express | IN | `test/res.sendFile.js:541-555` + `test/res.sendFile.js:765-779` | 30/30 (100.0%) | 30/30 (100.0%) |
| express | IN | `test/res.attachment.js:92-104` + `test/res.attachment.js:78-90` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/res.sendFile.js:693-709` + `test/res.sendFile.js:522-538` | 34/34 (100.0%) | 34/34 (100.0%) |
| express | IN | `test/express.json.js:607-647` + `test/express.text.js:453-493` | 82/82 (100.0%) | 82/82 (100.0%) |
| express | IN | `test/app.router.js:742-778` + `test/app.router.js:704-740` | 74/74 (100.0%) | 74/74 (100.0%) |
| express | IN | `test/res.attachment.js:106-117` + `test/res.attachment.js:24-35` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/app.use.js:144-149` + `test/app.use.js:193-198` | — | — |
| express | IN | `test/req.accepts.js:73-84` + `test/req.accepts.js:99-110` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/app.router.js:892-899` + `test/app.router.js:861-868` | 16/16 (100.0%) | 16/16 (100.0%) |
| express | IN | `test/res.render.js:91-119` + `test/res.render.js:245-273` | 58/58 (100.0%) | 58/58 (100.0%) |
| express | IN | `test/express.text.js:495-546` + `test/express.urlencoded.js:734-785` | 104/104 (100.0%) | 104/104 (100.0%) |
| express | IN | `test/acceptance/vhost.js:25-34` + `test/acceptance/vhost.js:36-45` | 20/20 (100.0%) | 20/20 (100.0%) |
| express | IN | `test/res.send.js:151-162` + `test/res.send.js:194-205` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/req.range.js:21-32` + `test/req.range.js:8-19` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/app.use.js:393-408` + `test/app.use.js:365-380` | 32/32 (100.0%) | 32/32 (100.0%) |
| express | IN | `test/res.render.js:189-205` + `test/res.render.js:207-223` | 22/34 (64.7%) | 22/34 (64.7%) |
| express | IN | `test/express.text.js:172-184` + `test/express.urlencoded.js:226-238` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/express.text.js:106-123` + `test/express.urlencoded.js:295-313` | 37/37 (100.0%) | 37/37 (100.0%) |
| express | IN | `test/app.param.js:99-106` + `test/app.param.js:275-282` | — | — |
| express | IN | `test/req.host.js:74-119` + `test/req.hostname.js:74-119` | 92/92 (100.0%) | 92/92 (100.0%) |
| express | IN | `test/app.use.js:372-382` + `test/app.use.js:400-410` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/express.text.js:372-397` + `test/express.json.js:515-540` | — | — |
| express | IN | `test/express.json.js:394-428` + `test/express.urlencoded.js:525-559` | 70/70 (100.0%) | 70/70 (100.0%) |
| express | IN | `test/express.urlencoded.js:211-314` + `test/express.json.js:90-194` | 209/209 (100.0%) | 209/209 (100.0%) |
| express | IN | `test/res.sendFile.js:711-727` + `test/res.sendFile.js:693-709` | 34/34 (100.0%) | 34/34 (100.0%) |
| express | IN | `test/res.append.js:9-27` + `test/res.append.js:65-83` | 38/38 (100.0%) | 38/38 (100.0%) |
| express | IN | `test/app.param.js:182-193` + `test/app.param.js:203-214` | 24/24 (100.0%) | 24/24 (100.0%) |
| express | IN | `test/res.jsonp.js:164-171` + `test/res.jsonp.js:335-342` | — | — |
| express | IN | `test/express.text.js:207-221` + `test/express.urlencoded.js:434-448` | 30/30 (100.0%) | 30/30 (100.0%) |
| express | IN | `test/app.request.js:25-49` + `test/app.request.js:111-141` | 56/56 (100.0%) | 56/56 (100.0%) |
| express | IN | `test/express.urlencoded.js:678-696` + `test/express.text.js:432-450` | 38/38 (100.0%) | 38/38 (100.0%) |
| express | IN | `test/express.urlencoded.js:31-41` + `test/express.urlencoded.js:66-76` | 22/22 (100.0%) | 22/22 (100.0%) |
| express | IN | `test/res.sendFile.js:966-978` + `test/res.sendFile.js:952-964` | 26/26 (100.0%) | 26/26 (100.0%) |
| express | IN | `test/res.sendFile.js:471-484` + `test/res.sendFile.js:504-517` | 28/28 (100.0%) | 28/28 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_nullable_and_defaults.py:64-76` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:127-139` | 26/26 (100.0%) | 26/26 (100.0%) |
| fastapi | IN | `tests/test_response_model_include_exclude.py:116-124` + `tests/test_response_model_include_exclude.py:147-153` | 16/16 (100.0%) | 16/16 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_file/test_optional_list.py:296-325` + `tests/test_request_params/test_file/test_optional_list.py:27-56` | 60/60 (100.0%) | 60/60 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_optional_str.py:163-214` + `tests/test_request_params/test_form/test_optional_str.py:87-134` | 100/100 (100.0%) | 100/100 (100.0%) |
| fastapi | IN | `tests/test_security_api_key_cookie.py:7-23` + `tests/test_security_api_key_query.py:7-23` | 34/34 (100.0%) | 34/34 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_body/test_optional_list.py:27-86` + `tests/test_request_params/test_body/test_optional_list.py:115-177` | 123/123 (100.0%) | 123/123 (100.0%) |
| fastapi | IN | `tests/test_path.py:703-737` + `tests/test_path.py:527-561` | 70/70 (100.0%) | 70/70 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_cookie/test_required_str.py:92-158` + `tests/test_request_params/test_cookie/test_required_str.py:297-378` | 149/149 (100.0%) | 149/149 (100.0%) |
| fastapi | IN | `tests/test_security_http_basic_realm_description.py:8-49` + `tests/test_security_http_basic_realm.py:10-49` | 82/82 (100.0%) | 82/82 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_path_operation_configurations/test_tutorial003_tutorial004.py:109-209` + `tests/test_tutorial/test_path_operation_configurations/test_tutorial001.py:88-188` | 202/202 (100.0%) | 202/202 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_security/test_tutorial004.py:186-372` + `tests/test_tutorial/test_security/test_tutorial005.py:230-432` | 390/390 (100.0%) | 390/390 (100.0%) |
| fastapi | IN | `tests/test_include_router_defaults_overrides.py:160-178` + `tests/test_include_router_defaults_overrides.py:181-199` | 38/38 (100.0%) | 38/38 (100.0%) |
| fastapi | IN | `tests/test_multipart_installation.py:89-99` + `tests/test_multipart_installation.py:102-112` | 22/22 (100.0%) | 22/22 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_body/test_optional_list.py:104-188` + `tests/test_request_params/test_body/test_optional_list.py:206-299` | 179/179 (100.0%) | 179/179 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_cookie/test_optional_str.py:250-306` + `tests/test_request_params/test_cookie/test_optional_str.py:165-215` | 108/108 (100.0%) | 108/108 (100.0%) |
| fastapi | IN | `tests/test_path.py:290-325` + `tests/test_path.py:505-540` | 72/72 (100.0%) | 72/72 (100.0%) |
| fastapi | IN | `tests/test_security_oauth2.py:36-54` + `tests/test_security_openid_connect.py:21-38` | 37/37 (100.0%) | 37/37 (100.0%) |
| fastapi | IN | `fastapi/param_functions.py:1066-1383` + `fastapi/param_functions.py:375-731` | 675/675 (100.0%) | 675/675 (100.0%) |
| fastapi | IN | `tests/test_path.py:625-635` + `tests/test_path.py:721-731` | 22/22 (100.0%) | 22/22 (100.0%) |
| fastapi | IN | `fastapi/security/oauth2.py:553-640` + `fastapi/security/oauth2.py:442-534` | 181/181 (100.0%) | 181/181 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_query_params_str_validations/test_tutorial007.py:10-54` + `tests/test_tutorial/test_query_params_str_validations/test_tutorial008.py:10-54` | 90/90 (100.0%) | 90/90 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_file/test_list.py:269-306` + `tests/test_request_params/test_file/test_list.py:416-453` | 76/76 (100.0%) | 76/76 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_list.py:413-422` + `tests/test_request_params/test_body/test_list.py:410-419` | 20/20 (100.0%) | 20/20 (100.0%) |
| fastapi | IN | `tests/test_strict_content_type_router_level.py:4-18` + `tests/test_strict_content_type_nested.py:8-19` | 27/27 (100.0%) | 27/27 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_dependencies/test_tutorial011.py:8-45` + `tests/test_tutorial/test_dependencies/test_tutorial002_tutorial003_tutorial004.py:10-79` | 108/108 (100.0%) | 108/108 (100.0%) |
| fastapi | IN | `tests/test_path.py:644-678` + `tests/test_path.py:558-592` | 70/70 (100.0%) | 70/70 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_nullable_and_defaults.py:54-80` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:22-48` | 54/54 (100.0%) | 54/54 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_body/test_required_str.py:26-67` + `tests/test_request_params/test_body/test_required_str.py:299-353` | 97/97 (100.0%) | 97/97 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_query_params_str_validations/test_tutorial002.py:63-147` + `tests/test_tutorial/test_query_params_str_validations/test_tutorial006c.py:67-148` | 167/167 (100.0%) | 167/167 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_cookie/test_required_str.py:67-76` + `tests/test_request_params/test_cookie/test_optional_str.py:60-69` | 20/20 (100.0%) | 20/20 (100.0%) |
| fastapi | IN | `scripts/tests/test_translation_fixer/test_code_blocks/test_code_blocks_wrong_lang_code.py:37-58` + `scripts/tests/test_translation_fixer/test_code_blocks/test_code_blocks_lines_number_mismatch.py:13-34` | 44/44 (100.0%) | 44/44 (100.0%) |
| fastapi | IN | `tests/test_security_http_basic_realm_description.py:52-84` + `tests/test_security_api_key_query_optional.py:45-73` | 62/62 (100.0%) | 62/62 (100.0%) |
| fastapi | IN | `tests/test_include_router_defaults_overrides.py:1757-1931` + `tests/test_include_router_defaults_overrides.py:654-828` | 350/350 (100.0%) | 350/350 (100.0%) |
| fastapi | IN | `tests/test_include_router_defaults_overrides.py:414-431` + `tests/test_include_router_defaults_overrides.py:377-394` | 36/36 (100.0%) | 36/36 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_query/test_optional_list.py:215-237` + `tests/test_request_params/test_query/test_optional_list.py:318-348` | 54/54 (100.0%) | 54/54 (100.0%) |
| fastapi | IN | `tests/test_dependency_wrapped.py:267-317` + `tests/test_dependency_wrapped.py:349-401` | 104/104 (100.0%) | 104/104 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_nullable_and_defaults.py:263-277` + `tests/test_request_params/test_nullable_and_defaults.py:633-647` | 30/30 (100.0%) | 30/30 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_header/test_required_str.py:399-411` + `tests/test_request_params/test_header/test_required_str.py:259-271` | 26/26 (100.0%) | 26/26 (100.0%) |
| fastapi | IN | `docs_src/security/tutorial007_an_py310.py:12-36` + `docs_src/security/tutorial007_py310.py:11-33` | 48/48 (100.0%) | 48/48 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_header_param_models/test_tutorial003.py:47-57` + `tests/test_tutorial/test_header_param_models/test_tutorial001.py:26-36` | 22/22 (100.0%) | 22/22 (100.0%) |
| fastapi | IN | `tests/test_query.py:8-20` + `tests/test_dependency_overrides.py:51-63` | 26/26 (100.0%) | 26/26 (100.0%) |
| fastapi | IN | `tests/test_security_api_key_cookie_description.py:7-23` + `tests/test_security_api_key_query_description.py:7-23` | 34/34 (100.0%) | 34/34 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_response_model/test_tutorial006.py:10-26` + `tests/test_tutorial/test_response_model/test_tutorial005.py:10-26` | 34/34 (100.0%) | 34/34 (100.0%) |
| fastapi | IN | `docs_src/settings/app02_py310/main.py:1-21` + `docs_src/settings/app02_an_py310/main.py:1-22` | 43/43 (100.0%) | 43/43 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_header/test_optional_list.py:215-239` + `tests/test_request_params/test_header/test_optional_list.py:320-352` | 58/58 (100.0%) | 58/58 (100.0%) |
| fastapi | IN | `tests/test_router_events.py:96-111` + `tests/test_router_events.py:277-292` | 6/32 (18.8%) | 6/32 (18.8%) |
| fastapi | IN | `tests/test_request_params/test_header/test_required_str.py:133-167` + `tests/test_request_params/test_header/test_required_str.py:372-411` | 75/75 (100.0%) | 75/75 (100.0%) |
| fastapi | IN | `tests/test_application.py:105-122` + `tests/test_tutorial/test_request_forms/test_tutorial001.py:110-127` | 36/36 (100.0%) | 36/36 (100.0%) |
| fastapi | IN | `docs_src/security/tutorial005_an_py310.py:151-170` + `docs_src/security/tutorial005_py310.py:150-167` | 38/38 (100.0%) | 38/38 (100.0%) |
| fastapi | IN | `tests/test_dependency_yield_scope_websockets.py:61-72` + `tests/test_dependency_yield_scope.py:59-70` | 24/24 (100.0%) | 24/24 (100.0%) |
| fastapi | IN | `tests/test_security_oauth2_password_bearer_optional.py:11-30` + `tests/test_security_oauth2_password_bearer_optional_description.py:15-34` | 40/40 (100.0%) | 40/40 (100.0%) |
| fastapi | IN | `tests/test_security_http_bearer_description.py:6-16` + `tests/test_security_http_digest_description.py:6-16` | 22/22 (100.0%) | 22/22 (100.0%) |
| fastapi | IN | `docs_src/settings/app03_an_py310/config.py:1-9` + `docs_src/settings/app03_py310/config.py:1-9` | 18/18 (100.0%) | 18/18 (100.0%) |
| fastapi | IN | `fastapi/params.py:284-317` + `fastapi/param_functions.py:2355-2388` | 68/68 (100.0%) | 68/68 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_header_params/test_tutorial001.py:38-116` + `tests/test_tutorial/test_header_params/test_tutorial002.py:49-127` | 158/158 (100.0%) | 158/158 (100.0%) |
| fastapi | IN | `docs_src/additional_responses/tutorial003_py310.py:6-15` + `docs_src/additional_responses/tutorial001_py310.py:6-15` | 20/20 (100.0%) | 20/20 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_nullable_and_defaults.py:601-615` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:627-641` | 30/30 (100.0%) | 30/30 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_nullable_and_defaults.py:128-152` + `tests/test_request_params/test_nullable_and_defaults.py:75-99` | 50/50 (100.0%) | 50/50 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_header/test_list.py:195-246` + `tests/test_request_params/test_header/test_list.py:302-356` | 107/107 (100.0%) | 107/107 (100.0%) |
| fastapi | IN | `docs_src/templates/tutorial001_py310.py:1-18` + `docs_src/templates/tutorial002_py310.py:1-18` | 36/36 (100.0%) | 36/36 (100.0%) |
| fastapi | IN | `docs_src/app_testing/app_b_py310/main.py:1-36` + `docs_src/app_testing/app_b_an_py310/main.py:1-38` | 74/74 (100.0%) | 74/74 (100.0%) |
| fastapi | IN | `tests/test_security_api_key_query.py:12-39` + `tests/test_security_api_key_query_description.py:12-39` | 56/56 (100.0%) | 56/56 (100.0%) |
| fastapi | IN | `fastapi/param_functions.py:1694-1728` + `fastapi/params.py:665-699` | 70/70 (100.0%) | 70/70 (100.0%) |
| fastapi | IN | `tests/test_dependency_overrides.py:111-168` + `tests/test_dependency_overrides.py:51-108` | 116/116 (100.0%) | 116/116 (100.0%) |
| fastapi | IN | `tests/test_stream_bare_type.py:48-53` + `tests/test_stream_bare_type.py:56-61` | 12/12 (100.0%) | 12/12 (100.0%) |
| fastapi | IN | `tests/test_security_openid_connect_optional.py:52-82` + `tests/test_security_http_base.py:39-64` | 57/57 (100.0%) | 57/57 (100.0%) |
| fastapi | IN | `fastapi/routing.py:5322-5702` + `fastapi/routing.py:4945-5320` | 757/757 (100.0%) | 757/757 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_header/test_list.py:143-177` + `tests/test_request_params/test_header/test_list.py:388-427` | 75/75 (100.0%) | 75/75 (100.0%) |
| fastapi | IN | `docs_src/body_multiple_params/tutorial004_an_py310.py:1-33` + `docs_src/body_multiple_params/tutorial004_py310.py:1-31` | 64/64 (100.0%) | 64/64 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_path_params_numeric_validations/test_tutorial001.py:10-49` + `tests/test_tutorial/test_path_params_numeric_validations/test_tutorial002_tutorial003.py:8-49` | 82/82 (100.0%) | 82/82 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_form/test_required_str.py:26-66` + `tests/test_request_params/test_form/test_required_str.py:185-235` | 92/92 (100.0%) | 92/92 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_header_params/test_tutorial002.py:10-46` + `tests/test_tutorial/test_header_params/test_tutorial001.py:10-35` | 63/63 (100.0%) | 63/63 (100.0%) |
| fastapi | IN | `tests/test_router_events.py:67-83` + `tests/test_router_events.py:150-168` | 36/36 (100.0%) | 36/36 (100.0%) |
| fastapi | IN | `tests/test_default_response_class_router.py:139-150` + `tests/test_default_response_class_router.py:111-122` | 24/24 (100.0%) | 24/24 (100.0%) |
| fastapi | IN | `tests/test_request_params/test_cookie/test_optional_str.py:154-169` + `tests/test_request_params/test_query/test_optional_str.py:151-166` | 32/32 (100.0%) | 32/32 (100.0%) |
| fastapi | IN | `tests/test_tutorial/test_header_param_models/test_tutorial002.py:97-204` + `tests/test_tutorial/test_header_param_models/test_tutorial001.py:102-209` | 216/216 (100.0%) | 216/216 (100.0%) |
| fastapi | IN | `docs_src/security/tutorial004_an_py310.py:113-150` + `docs_src/security/tutorial005_an_py310.py:143-177` | 73/73 (100.0%) | 73/73 (100.0%) |
| fastapi | IN | `tests/test_path.py:95-128` + `tests/test_path.py:44-77` | 68/68 (100.0%) | 68/68 (100.0%) |
| fastapi | IN | `fastapi/dependencies/models.py:139-148` + `fastapi/dependencies/models.py:169-182` | 24/24 (100.0%) | 24/24 (100.0%) |
| fastapi | IN | `tests/test_dependency_wrapped.py:188-229` + `tests/test_dependency_wrapped.py:334-381` | 90/90 (100.0%) | 90/90 (100.0%) |
| flask | IN | `tests/test_user_error_handler.py:229-235` + `tests/test_user_error_handler.py:47-53` | 14/14 (100.0%) | 14/14 (100.0%) |
| flask | IN | `tests/test_basic.py:180-187` + `tests/test_basic.py:142-149` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `src/flask/sansio/app.py:714-719` + `src/flask/sansio/app.py:773-778` | 12/12 (100.0%) | 12/12 (100.0%) |
| flask | IN | `src/flask/app.py:55-76` + `src/flask/sansio/app.py:37-57` | 43/43 (100.0%) | 43/43 (100.0%) |
| flask | IN | `tests/test_basic.py:183-190` + `tests/test_basic.py:145-152` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `tests/test_blueprints.py:433-440` + `tests/test_blueprints.py:483-490` | 6/16 (37.5%) | 6/16 (37.5%) |
| flask | IN | `tests/test_testing.py:309-320` + `tests/test_testing.py:326-337` | 24/24 (100.0%) | 24/24 (100.0%) |
| flask | IN | `tests/test_blueprints.py:622-633` + `tests/test_blueprints.py:639-650` | 24/24 (100.0%) | 24/24 (100.0%) |
| flask | IN | `tests/test_blueprints.py:943-961` + `tests/test_blueprints.py:923-941` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `tests/test_basic.py:146-153` + `tests/test_basic.py:184-191` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `tests/test_basic.py:281-289` + `tests/test_basic.py:303-311` | 18/18 (100.0%) | 18/18 (100.0%) |
| flask | IN | `tests/test_templating.py:305-315` + `tests/test_templating.py:332-342` | 22/22 (100.0%) | 22/22 (100.0%) |
| flask | IN | `examples/tutorial/tests/test_blog.py:46-54` + `examples/tutorial/tests/test_blog.py:57-65` | 18/18 (100.0%) | 18/18 (100.0%) |
| flask | IN | `tests/test_basic.py:1435-1443` + `tests/test_basic.py:1446-1454` | 18/18 (100.0%) | 18/18 (100.0%) |
| flask | IN | `tests/test_basic.py:143-150` + `tests/test_basic.py:181-188` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `tests/test_appctx.py:92-101` + `tests/test_appctx.py:115-124` | 20/20 (100.0%) | 20/20 (100.0%) |
| flask | IN | `tests/test_blueprints.py:586-600` + `tests/test_blueprints.py:636-650` | 30/30 (100.0%) | 30/30 (100.0%) |
| flask | IN | `tests/test_blueprints.py:927-945` + `tests/test_blueprints.py:947-965` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `tests/test_blueprints.py:935-953` + `tests/test_blueprints.py:955-973` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `src/flask/globals.py:65-77` + `src/flask/ctx.py:528-540` | 26/26 (100.0%) | 26/26 (100.0%) |
| flask | IN | `tests/test_helpers.py:274-284` + `tests/test_helpers.py:230-240` | 22/22 (100.0%) | 22/22 (100.0%) |
| flask | IN | `tests/test_blueprints.py:951-969` + `tests/test_blueprints.py:931-949` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `src/flask/sansio/scaffold.py:28-38` + `src/flask/sansio/blueprints.py:18-26` | 20/20 (100.0%) | 20/20 (100.0%) |
| flask | IN | `tests/test_blueprints.py:286-296` + `tests/test_basic.py:1745-1755` | 22/22 (100.0%) | 22/22 (100.0%) |
| flask | IN | `tests/test_blueprints.py:625-633` + `tests/test_blueprints.py:659-667` | 18/18 (100.0%) | 18/18 (100.0%) |
| flask | IN | `tests/test_appctx.py:58-67` + `tests/test_reqctx.py:30-39` | 20/20 (100.0%) | 20/20 (100.0%) |
| flask | IN | `tests/test_basic.py:148-153` + `tests/test_basic.py:186-192` | 13/13 (100.0%) | 13/13 (100.0%) |
| flask | IN | `tests/test_blueprints.py:1041-1060` + `tests/test_blueprints.py:1022-1038` | 37/37 (100.0%) | 37/37 (100.0%) |
| flask | IN | `tests/test_cli.py:412-424` + `tests/test_cli.py:397-409` | 26/26 (100.0%) | 26/26 (100.0%) |
| flask | IN | `tests/test_blueprints.py:459-473` + `tests/test_blueprints.py:493-507` | 18/30 (60.0%) | 18/30 (60.0%) |
| flask | IN | `tests/test_basic.py:778-793` + `tests/test_basic.py:796-811` | 32/32 (100.0%) | 32/32 (100.0%) |
| flask | IN | `tests/test_blueprints.py:426-440` + `tests/test_blueprints.py:476-490` | 30/30 (100.0%) | 30/30 (100.0%) |
| flask | IN | `tests/test_cli.py:63-70` + `tests/test_cli.py:81-88` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `tests/test_user_error_handler.py:187-200` + `tests/test_user_error_handler.py:166-179` | 28/28 (100.0%) | 28/28 (100.0%) |
| flask | IN | `tests/test_cli.py:400-409` + `tests/test_cli.py:415-424` | 20/20 (100.0%) | 20/20 (100.0%) |
| flask | IN | `tests/test_cli.py:513-518` + `tests/test_cli.py:504-509` | 12/12 (100.0%) | 12/12 (100.0%) |
| flask | IN | `tests/test_blueprints.py:153-172` + `tests/test_blueprints.py:839-859` | 41/41 (100.0%) | 41/41 (100.0%) |
| flask | IN | `src/flask/blueprints.py:55-128` + `src/flask/app.py:363-443` | 155/155 (100.0%) | 155/155 (100.0%) |
| flask | IN | `src/flask/sansio/blueprints.py:413-420` + `src/flask/sansio/scaffold.py:376-441` | 74/74 (100.0%) | 74/74 (100.0%) |
| flask | IN | `tests/test_blueprints.py:157-172` + `tests/test_blueprints.py:843-859` | 33/33 (100.0%) | 33/33 (100.0%) |
| flask | IN | `tests/test_blueprints.py:765-793` + `tests/test_blueprints.py:796-832` | 66/66 (100.0%) | 66/66 (100.0%) |
| flask | IN | `tests/test_basic.py:144-151` + `tests/test_basic.py:182-189` | 16/16 (100.0%) | 16/16 (100.0%) |
| flask | IN | `tests/test_views.py:218-234` + `tests/test_views.py:237-253` | 34/34 (100.0%) | 34/34 (100.0%) |
| flask | IN | `tests/test_blueprints.py:619-633` + `tests/test_blueprints.py:653-667` | 18/30 (60.0%) | 18/30 (60.0%) |
| flask | IN | `tests/test_templating.py:91-110` + `tests/test_templating.py:71-88` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `tests/test_blueprints.py:939-957` + `tests/test_blueprints.py:919-937` | 38/38 (100.0%) | 38/38 (100.0%) |
| flask | IN | `tests/test_basic.py:818-828` + `tests/test_basic.py:830-840` | 22/22 (100.0%) | 22/22 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Arbitrary.fs:1018-1025` + `tests/FsCheck.Test/Arbitrary.fs:1027-1034` | 16/16 (100.0%) | 16/16 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test.Xunit.v3/PropertyAttributeTests.fs:1-94` + `tests/FsCheck.Test.Xunit/PropertyAttributeTests.fs:1-93` | 187/187 (100.0%) | 187/187 (100.0%) |
| FsCheck | IN | `docs/TestData.fsx:133-142` + `tests/FsCheck.Test/Runner.fs:95-104` | 20/20 (100.0%) | 20/20 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Property.fs:315-338` + `tests/FsCheck.Test/Property.fs:352-388` | 61/61 (100.0%) | 61/61 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/CheckExtensions.fs:25-30` + `src/FsCheck.Xunit/CheckExtensions.fs:23-28` | 12/12 (100.0%) | 12/12 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/StateMachine.fs:184-222` + `docs/StatefulTesting.fsx:16-61` | 85/85 (100.0%) | 85/85 (100.0%) |
| FsCheck | IN | `src/FsCheck/Internals.Numeric.fs:9-31` + `src/FsCheck/Internals.Numeric.fs:34-56` | 46/46 (100.0%) | 46/46 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:16-169` + `src/FsCheck.Xunit/PropertyAttribute.fs:13-162` | 304/304 (100.0%) | 304/304 (100.0%) |
| FsCheck | IN | `src/FsCheck/AssemblyInfo.fs:3-23` + `src/FsCheck.Xunit/AssemblyInfo.fs:3-23` | 42/42 (100.0%) | 42/42 (100.0%) |
| FsCheck | IN | `src/FsCheck/DefaultArbTypes.fs:84-92` + `src/FsCheck/DefaultArbTypes.fs:93-101` | 18/18 (100.0%) | 18/18 (100.0%) |
| FsCheck | IN | `examples/FsCheck.Docs.CSharpSnippets/Properties.cs:107-112` + `examples/FsCheck.Examples.MsTest/SampleTests.cs:106-114` | 15/15 (100.0%) | 15/15 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:48-118` + `src/FsCheck.NUnit/FsCheckPropertyAttribute.fs:53-123` | 142/142 (100.0%) | 142/142 (100.0%) |
| FsCheck | IN | `examples/FsCheck.Examples.MsTest/ClassesToTest/EnumerableExtensions.cs:10-23` + `examples/FsCheck.Examples.CSharp/Program.cs:13-26` | — | — |
| FsCheck | IN | `examples/FsCheck.Examples.NUnit.CSharp/SyncVersusAsyncExamples.cs:1-58` + `examples/FsCheck.Examples.Xunit.CSharp/SyncVersusAsyncExamples.cs:1-57` | 115/115 (100.0%) | 115/115 (100.0%) |
| FsCheck | IN | `src/FsCheck/Random.fs:179-205` + `src/FsCheck/Random.fs:143-178` | 63/63 (100.0%) | 63/63 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:137-160` + `src/FsCheck.Xunit/PropertyAttribute.fs:134-157` | 48/48 (100.0%) | 48/48 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Arbitrary.fs:869-875` + `tests/FsCheck.Test/Arbitrary.fs:859-865` | 14/14 (100.0%) | 14/14 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/AssemblyInfo.fs:3-22` + `src/FsCheck.NUnit/AssemblyInfo.fs:3-22` | 40/40 (100.0%) | 40/40 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Runner.fs:260-276` + `tests/FsCheck.Test/Runner.fs:241-257` | 34/34 (100.0%) | 34/34 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/TypeClass.fs:49-55` + `tests/FsCheck.Test/TypeClass.fs:34-41` | 15/15 (100.0%) | 15/15 (100.0%) |
| FsCheck | IN | `src/FsCheck/Internals.DefaultArbs.fs:89-101` + `src/FsCheck/Internals.DefaultArbs.fs:119-131` | 26/26 (100.0%) | 26/26 (100.0%) |
| FsCheck | IN | `src/FsCheck/Fluent.Gen.fs:114-127` + `src/FsCheck/Fluent.Gen.fs:128-141` | 28/28 (100.0%) | 28/28 (100.0%) |
| FsCheck | IN | `src/FsCheck/DefaultArbTypes.fs:102-110` + `src/FsCheck/DefaultArbTypes.fs:93-101` | 18/18 (100.0%) | 18/18 (100.0%) |
| FsCheck | IN | `examples/FsCheck.Examples.MsTest/ClassesToTest/Doc.cs:45-60` + `examples/FsCheck.Examples.MsTest/ClassesToTest/Doc.cs:62-77` | 32/32 (100.0%) | 32/32 (100.0%) |
| FsCheck | IN | `src/FsCheck/Internals.DefaultArbs.fs:1013-1023` + `src/FsCheck/Internals.DefaultArbs.fs:999-1009` | 22/22 (100.0%) | 22/22 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Arbitrary.fs:652-660` + `tests/FsCheck.Test/Arbitrary.fs:642-650` | 18/18 (100.0%) | 18/18 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit/CheckExtensions.fs:1-60` + `src/FsCheck.Xunit.v3/CheckExtensions.fs:1-62` | 122/122 (100.0%) | 122/122 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:24-46` + `src/FsCheck.NUnit/FsCheckPropertyAttribute.fs:20-51` | 55/55 (100.0%) | 55/55 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit.v3/Runner.fs:1-20` + `src/FsCheck.Xunit/Runner.fs:1-20` | 40/40 (100.0%) | 40/40 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test.Xunit.v3/PropertyAttributeTests.fs:8-70` + `tests/FsCheck.Test.NUnit/PropertiesAttributeTests.fs:6-68` | 126/126 (100.0%) | 126/126 (100.0%) |
| FsCheck | IN | `examples/FsCheck.Docs.CSharpSnippets/Properties.cs:116-123` + `examples/FsCheck.Examples.CSharp/Program.cs:112-119` | 16/16 (100.0%) | 16/16 (100.0%) |
| FsCheck | IN | `src/FsCheck.Xunit/PropertyAttribute.fs:178-197` + `src/FsCheck.Xunit.v3/PropertyAttribute.fs:197-216` | 40/40 (100.0%) | 40/40 (100.0%) |
| FsCheck | IN | `tests/FsCheck.Test/Property.fs:352-363` + `tests/FsCheck.Test/Property.fs:365-376` | 24/24 (100.0%) | 24/24 (100.0%) |
| FsCheck | IN | `src/FsCheck/FSharp.Gen.fs:116-126` + `src/FsCheck/FSharp.Gen.fs:128-139` | 23/23 (100.0%) | 23/23 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:675-757` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:526-608` | 166/166 (100.0%) | 166/166 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Json.Core/JsonSchema.fs:181-187` + `src/FSharp.Data.Json.Core/JsonSchema.fs:197-203` | 14/14 (100.0%) | 14/14 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:736-768` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:587-619` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:725-757` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:576-608` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:746-779` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:597-630` | 68/68 (100.0%) | 68/68 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:664-696` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:515-547` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Http/Http.fs:2369-2404` + `src/FSharp.Data.Http/Http.fs:2333-2368` | 72/72 (100.0%) | 72/72 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:564-575` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:713-724` | 24/24 (100.0%) | 24/24 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:704-735` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:555-586` | 64/64 (100.0%) | 64/64 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:155-168` + `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:90-103` | 28/28 (100.0%) | 28/28 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Html.Core/HtmlOperations.fs:620-652` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:769-801` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharp.Data | IN | `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:449-454` + `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:442-447` | 12/12 (100.0%) | 12/12 (100.0%) |
| FSharp.Data | IN | `src/FSharp.Data.Json.Core/JsonSchema.fs:420-425` + `src/FSharp.Data.Json.Core/JsonSchema.fs:404-409` | 12/12 (100.0%) | 12/12 (100.0%) |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:57-73` + `tests/FSharp.Data.Core.Tests/WorldBankRuntime.fs:23-173` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.DesignTime/CommonProviderImplementation/Helpers.fs:12-17` + `src/FSharp.Data.DesignTime/Xml/XmlGenerator.fs:7-12` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:418-445` + `src/FSharp.Data.Http/Http.fs:697-724` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:690-720` + `src/FSharp.Data.Http/Http.fs:248-278` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.DesignTime/Json/JsonGenerator.fs:7-12` + `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:8-13` | — | — |
| FSharp.Data | OUT | `docs/library/HtmlProvider.fsx:105-113` + `docs/library/HtmlProvider.fsx:370-404` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:2046-2048` + `src/FSharp.Data.Http/Http.fs:2043-2045` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.DesignTime/Xml/XmlGenerator.fs:10-22` + `src/FSharp.Data.DesignTime/Json/JsonGenerator.fs:10-20` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:2044-2046` + `src/FSharp.Data.Http/Http.fs:2047-2049` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.Http/Http.fs:418-449` + `src/FSharp.Data.Http/Http.fs:359-390` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.DesignTime/Json/JsonProvider.fs:6-13` + `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:7-14` | — | — |
| FSharp.Data | OUT | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:9-19` + `src/FSharp.Data.DesignTime/Json/JsonProvider.fs:8-18` | — | — |
| FSharp.Data | OUT | `tests/FSharp.Data.Tests/JsonProvider.fs:1-9` + `tests/FSharp.Data.Core.Tests/HtmlRuntimeTypes.fs:1-9` | — | — |
| FSharp.Data | OUT | `tests/FSharp.Data.Tests/JsonProvider.fs:832-839` + `docs/library/TypeInference.fsx:185-210` | — | — |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Task.fs:18-51` + `tests/FSharpPlus.Tests/ValueTask.fs:42-78` | 71/71 (100.0%) | 71/71 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Internals.fs:351-441` + `src/FSharpPlus/Internals.fs:443-532` | 181/181 (100.0%) | 181/181 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus.TypeLevel/Providers/Matrix.Provider.fs:73-91` + `src/FSharpPlus.TypeLevel/Providers/Matrix.Provider.fs:163-181` | 38/38 (100.0%) | 38/38 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:59-126` + `src/FSharpPlus/Extensions/Dict.fs:56-122` | 135/135 (100.0%) | 135/135 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/NonEmptySet.fs:66-83` + `src/FSharpPlus/Data/NonEmptyList.fs:57-76` | — | — |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/ValueTask.fs:155-202` + `tests/FSharpPlus.Tests/Task.fs:194-238` | 93/93 (100.0%) | 93/93 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Builders.fs:20-28` + `tests/FSharpPlusFable.Tests/FSharpTests/General.fs:19-32` | 23/23 (100.0%) | 23/23 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Traversable.fs:400-407` + `src/FSharpPlus/Control/Traversable.fs:201-212` | 20/20 (100.0%) | 20/20 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/Extensions.fs:163-170` + `tests/FSharpPlusFable.Tests/FSharpTests/Extensions.fs:199-206` | 16/16 (100.0%) | 16/16 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Error.fs:137-149` + `src/FSharpPlus/Data/Error.fs:226-238` | 26/26 (100.0%) | 26/26 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Collection.fs:166-173` + `src/FSharpPlus/Control/Collection.fs:182-189` | 16/16 (100.0%) | 16/16 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/ValueTask.fs:251-257` + `src/FSharpPlus/Extensions/Task.fs:267-273` | 14/14 (100.0%) | 14/14 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Seq.fs:1006-1024` + `src/FSharpPlus/Data/Seq.fs:1026-1058` | 52/52 (100.0%) | 52/52 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Option.fs:1-101` + `src/FSharpPlus/Data/ValueOption.fs:1-102` | 203/203 (100.0%) | 203/203 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Indexable.fs:201-207` + `src/FSharpPlus/Control/Indexable.fs:182-188` | 14/14 (100.0%) | 14/14 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Helpers.fs:22-77` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:67-116` | 106/106 (100.0%) | 106/106 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Dict.fs:28-105` + `src/FSharpPlus/Extensions/Dictionary.fs:30-107` | 156/156 (100.0%) | 156/156 (100.0%) |
| FSharpPlus | IN | `tests/benchmarks/ExtensionsBenchmarks.fs:27-32` + `src/FSharpPlus/Extensions/Extensions.fs:313-318` | 12/12 (100.0%) | 12/12 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Traversable.fs:360-366` + `src/FSharpPlus/Control/Traversable.fs:160-166` | 14/14 (100.0%) | 14/14 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Numeric.fs:592-640` + `src/FSharpPlus/Control/Numeric.fs:541-590` | 99/99 (100.0%) | 99/99 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Traversable.fs:335-351` + `src/FSharpPlus/Control/Traversable.fs:135-151` | 34/34 (100.0%) | 34/34 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Seq.fs:398-440` + `src/FSharpPlus/Data/Seq.fs:442-484` | 86/86 (100.0%) | 86/86 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/ValueTask.fs:378-418` + `src/FSharpPlus/Extensions/Task.fs:405-456` | 93/93 (100.0%) | 93/93 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Free.fs:107-122` + `docsrc/content/type-free.fsx:72-87` | 32/32 (100.0%) | 32/32 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/General.fs:15-68` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:2-64` | 117/117 (100.0%) | 117/117 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Option.fs:1-89` + `src/FSharpPlus/Extensions/ValueOption.fs:1-103` | 192/192 (100.0%) | 192/192 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Error.fs:184-211` + `src/FSharpPlus/Data/Error.fs:95-122` | 56/56 (100.0%) | 56/56 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Seq.fs:486-528` + `src/FSharpPlus/Data/Seq.fs:398-440` | 86/86 (100.0%) | 86/86 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/ValueTask.fs:207-249` + `src/FSharpPlus/Extensions/ValueTask.fs:163-206` | 87/87 (100.0%) | 87/87 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Dictionary.fs:228-310` + `src/FSharpPlus/Extensions/Dict.fs:227-309` | 166/166 (100.0%) | 166/166 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/ValueTask.fs:439-460` + `tests/FSharpPlus.Tests/Task.fs:546-567` | 44/44 (100.0%) | 44/44 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Error.fs:63-73` + `src/FSharpPlus/Data/Error.fs:156-166` | 22/22 (100.0%) | 22/22 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:160-242` + `src/FSharpPlus/Extensions/Dict.fs:137-204` | 151/151 (100.0%) | 151/151 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Free.fs:161-167` + `tests/FSharpPlus.Tests/Free.fs:79-85` | 14/14 (100.0%) | 14/14 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Validations.fs:151-198` + `tests/FSharpPlus.Tests/Validations.fs:204-251` | 96/96 (100.0%) | 96/96 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Collection.fs:13-78` + `src/FSharpPlus/Control/Collection.fs:81-134` | 120/120 (100.0%) | 120/120 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Traversals.fs:116-171` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Traversable.fs:134-191` | 114/114 (100.0%) | 114/114 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:211-255` + `tests/FSharpPlus.Tests/General.fs:168-215` | 93/93 (100.0%) | 93/93 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Dictionary.fs:18-57` + `src/FSharpPlus/Extensions/Dict.fs:17-55` | 79/79 (100.0%) | 79/79 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Foldable.fs:362-374` + `src/FSharpPlus/Control/Foldable.fs:392-404` | 26/26 (100.0%) | 26/26 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Collection.fs:376-408` + `src/FSharpPlus/Control/Collection.fs:267-299` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Task.fs:103-132` + `src/FSharpPlus/Extensions/Task.fs:263-286` | 54/54 (100.0%) | 54/54 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Task.fs:302-1106` + `tests/FSharpPlus.Tests/ValueTask.fs:203-999` | 1602/1602 (100.0%) | 1602/1602 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/ZipApplicative.fs:221-253` + `src/FSharpPlus/Control/Applicative.fs:167-199` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Control/Numeric.fs:571-580` + `src/FSharpPlus/Control/Numeric.fs:621-630` | 20/20 (100.0%) | 20/20 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Dict.fs:190-226` + `src/FSharpPlus/Extensions/Dictionary.fs:192-227` | 73/73 (100.0%) | 73/73 (100.0%) |
| FSharpPlus | IN | `docsrc/content/type-free.fsx:4-110` + `tests/FSharpPlus.Tests/Free.fs:75-145` | 178/178 (100.0%) | 178/178 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Seq.fs:267-316` + `src/FSharpPlus/Extensions/Seq.fs:317-351` | 85/85 (100.0%) | 85/85 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/General.fs:42-67` + `tests/FSharpPlus.Tests/Monads.fs:14-35` | 48/48 (100.0%) | 48/48 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/ValueTask.fs:121-153` + `tests/FSharpPlus.Tests/ValueTask.fs:155-187` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/ResizeArray.fs:106-138` + `src/FSharpPlus/Extensions/ResizeArray.fs:139-161` | 56/56 (100.0%) | 56/56 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/ValueOption.fs:84-92` + `src/FSharpPlus/Data/Option.fs:83-91` | 18/18 (100.0%) | 18/18 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Asyncs.fs:69-98` + `tests/FSharpPlus.Tests/Asyncs.fs:38-67` | 60/60 (100.0%) | 60/60 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Extensions.fs:349-418` + `src/FSharpPlus/Extensions/Extensions.fs:436-505` | 140/140 (100.0%) | 140/140 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:152-225` + `tests/FSharpPlus.Tests/General.fs:101-185` | 159/159 (100.0%) | 159/159 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Collections.fs:34-52` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Collections.fs:18-39` | 41/41 (100.0%) | 41/41 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:127-231` + `src/FSharpPlus/Extensions/Dict.fs:123-189` | 172/172 (100.0%) | 172/172 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Task.fs:160-192` + `tests/FSharpPlus.Tests/Task.fs:194-226` | 66/66 (100.0%) | 66/66 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Task.fs:31-102` + `src/FSharpPlus/Extensions/ValueTask.fs:28-105` | 150/150 (100.0%) | 150/150 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Extensions/Dictionary.fs:192-205` + `src/FSharpPlus/Extensions/Dict.fs:190-204` | 29/29 (100.0%) | 29/29 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlus.Tests/Task.fs:3-12` + `tests/FSharpPlus.Tests/ValueTask.fs:3-12` | 20/20 (100.0%) | 20/20 (100.0%) |
| FSharpPlus | IN | `src/FSharpPlus/Data/Error.fs:102-124` + `src/FSharpPlus/Data/Error.fs:191-213` | 46/46 (100.0%) | 46/46 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:46-58` + `tests/FSharpPlus.Tests/General.fs:51-63` | — | — |
| FSharpPlus | IN | `docsrc/content/type-any.fsx:5-46` + `docsrc/content/type-all.fsx:5-45` | 83/83 (100.0%) | 83/83 (100.0%) |
| FSharpPlus | IN | `tests/FSharpPlusFable.Tests/FSharpTests/General/MonoidCompile.fs:5-38` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Monoid.fs:5-38` | 68/68 (100.0%) | 68/68 (100.0%) |
| gin | IN | `binding/binding_test.go:876-886` + `binding/binding_test.go:1178-1188` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `routes_test.go:296-303` + `routes_test.go:227-237` | 19/19 (100.0%) | 19/19 (100.0%) |
| gin | IN | `recovery_test.go:203-214` + `recovery_test.go:272-284` | 25/25 (100.0%) | 25/25 (100.0%) |
| gin | IN | `binding/binding_test.go:914-922` + `binding/binding_test.go:935-943` | 18/18 (100.0%) | 18/18 (100.0%) |
| gin | IN | `binding/binding_test.go:1205-1214` + `binding/binding_test.go:1037-1046` | 20/20 (100.0%) | 20/20 (100.0%) |
| gin | IN | `binding/binding_test.go:874-892` + `binding/binding_test.go:858-872` | 34/34 (100.0%) | 34/34 (100.0%) |
| gin | IN | `middleware_test.go:118-157` + `middleware_test.go:81-116` | 76/76 (100.0%) | 76/76 (100.0%) |
| gin | IN | `binding/form_mapping_test.go:616-663` + `binding/form_mapping_test.go:865-912` | 96/96 (100.0%) | 96/96 (100.0%) |
| gin | IN | `routes_test.go:366-396` + `routes_test.go:334-363` | 61/61 (100.0%) | 61/61 (100.0%) |
| gin | IN | `gin_integration_test.go:326-336` + `gin_integration_test.go:359-369` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `binding/form_mapping_benchmark_test.go:33-43` + `binding/form_mapping_benchmark_test.go:56-66` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `routes_test.go:339-355` + `routes_test.go:372-388` | 34/34 (100.0%) | 34/34 (100.0%) |
| gin | IN | `context_test.go:2757-2771` + `context_test.go:2742-2754` | 28/28 (100.0%) | 28/28 (100.0%) |
| gin | IN | `recovery_test.go:285-296` + `recovery_test.go:62-73` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `logger_test.go:22-84` + `logger_test.go:86-148` | 126/126 (100.0%) | 126/126 (100.0%) |
| gin | IN | `context.go:1259-1279` + `context.go:1288-1305` | 39/39 (100.0%) | 39/39 (100.0%) |
| gin | IN | `logger_test.go:196-204` + `logger_test.go:161-169` | 18/18 (100.0%) | 18/18 (100.0%) |
| gin | IN | `render/render_test.go:659-678` + `render/render_test.go:680-699` | 40/40 (100.0%) | 40/40 (100.0%) |
| gin | IN | `gin_test.go:417-435` + `gin_test.go:350-368` | 38/38 (100.0%) | 38/38 (100.0%) |
| gin | IN | `context_test.go:2076-2100` + `context_test.go:2050-2074` | 50/50 (100.0%) | 50/50 (100.0%) |
| gin | IN | `routes_test.go:880-906` + `routes_test.go:911-942` | 59/59 (100.0%) | 59/59 (100.0%) |
| gin | IN | `binding/msgpack.go:17-29` + `binding/yaml.go:15-27` | 22/26 (84.6%) | 22/26 (84.6%) |
| gin | IN | `binding/xml.go:24-34` + `binding/toml.go:25-35` | 14/22 (63.6%) | 14/22 (63.6%) |
| gin | IN | `routes_test.go:475-485` + `routes_test.go:440-450` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `gin_test.go:123-141` + `gin_test.go:143-161` | 38/38 (100.0%) | 38/38 (100.0%) |
| gin | IN | `tree.go:877-887` + `tree.go:861-871` | — | — |
| gin | IN | `render/render_test.go:87-99` + `render/render_test.go:124-136` | 26/26 (100.0%) | 26/26 (100.0%) |
| gin | IN | `context_test.go:1782-1792` + `context_test.go:1794-1804` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `tree.go:848-871` + `tree.go:764-799` | — | — |
| gin | IN | `context_test.go:2431-2451` + `context_test.go:2601-2621` | 42/42 (100.0%) | 42/42 (100.0%) |
| gin | IN | `render/render_test.go:669-677` + `render/render_test.go:622-630` | 18/18 (100.0%) | 18/18 (100.0%) |
| gin | IN | `tree_test.go:1067-1091` + `tree_test.go:1023-1052` | 55/55 (100.0%) | 55/55 (100.0%) |
| gin | IN | `binding/binding_test.go:1000-1016` + `binding/binding_test.go:982-998` | 34/34 (100.0%) | 34/34 (100.0%) |
| gin | IN | `response_writer_test.go:166-221` + `response_writer_test.go:225-280` | 112/112 (100.0%) | 112/112 (100.0%) |
| gin | IN | `gin_test.go:510-518` + `gin_test.go:589-597` | 18/18 (100.0%) | 18/18 (100.0%) |
| gin | IN | `gin_integration_test.go:69-80` + `gin_integration_test.go:213-224` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `binding/form_mapping_test.go:508-542` + `binding/form_mapping_test.go:756-787` | 67/67 (100.0%) | 67/67 (100.0%) |
| gin | IN | `middleware_test.go:135-155` + `middleware_test.go:95-114` | 41/41 (100.0%) | 41/41 (100.0%) |
| gin | IN | `recovery_test.go:203-217` + `recovery_test.go:273-287` | 30/30 (100.0%) | 30/30 (100.0%) |
| gin | IN | `logger_test.go:418-434` + `logger_test.go:403-416` | 31/31 (100.0%) | 31/31 (100.0%) |
| gin | IN | `recovery_test.go:242-253` + `recovery_test.go:54-65` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `routes_test.go:163-174` + `routes_test.go:189-200` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `routes_test.go:441-453` + `routes_test.go:476-487` | 25/25 (100.0%) | 25/25 (100.0%) |
| gin | IN | `routes_test.go:739-753` + `routes_test.go:759-773` | 30/30 (100.0%) | 30/30 (100.0%) |
| gin | IN | `binding/json_test.go:113-131` + `binding/json_test.go:169-187` | 38/38 (100.0%) | 38/38 (100.0%) |
| gin | IN | `gin_test.go:164-172` + `gin_test.go:124-132` | 18/18 (100.0%) | 18/18 (100.0%) |
| gin | IN | `binding/form_mapping_test.go:555-603` + `binding/form_mapping_test.go:802-850` | 98/98 (100.0%) | 98/98 (100.0%) |
| gin | IN | `gin_test.go:399-414` + `gin_test.go:172-187` | 32/32 (100.0%) | 32/32 (100.0%) |
| gin | IN | `gin_test.go:746-779` + `gin_test.go:781-816` | 70/70 (100.0%) | 70/70 (100.0%) |
| gin | IN | `codec/json/json.go:15-41` + `codec/json/go_json.go:16-42` | 54/54 (100.0%) | 54/54 (100.0%) |
| gin | IN | `render/render_test.go:734-745` + `render/render_test.go:747-758` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `binding/yaml.go:17-27` + `binding/xml.go:16-26` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `routes_test.go:458-468` + `routes_test.go:492-502` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `binding/form_mapping_test.go:942-988` + `binding/form_mapping_test.go:692-738` | 94/94 (100.0%) | 94/94 (100.0%) |
| gin | IN | `context_test.go:2933-2943` + `context_test.go:2869-2879` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `recovery_test.go:277-288` + `recovery_test.go:207-218` | 24/24 (100.0%) | 24/24 (100.0%) |
| gin | IN | `context_test.go:224-231` + `context_test.go:159-166` | 14/16 (87.5%) | 14/16 (87.5%) |
| gin | IN | `context_test.go:1922-1929` + `context_test.go:1863-1870` | 16/16 (100.0%) | 16/16 (100.0%) |
| gin | IN | `codec/json/sonic.go:16-44` + `codec/json/jsoniter.go:16-44` | 58/58 (100.0%) | 58/58 (100.0%) |
| gin | IN | `auth_test.go:106-118` + `auth_test.go:126-138` | 26/26 (100.0%) | 26/26 (100.0%) |
| gin | IN | `binding/binding_nomsgpack.go:12-87` + `binding/binding.go:12-91` | 156/156 (100.0%) | 156/156 (100.0%) |
| gin | IN | `binding/binding_test.go:1177-1187` + `binding/binding_test.go:895-905` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `binding/binding_test.go:531-554` + `binding/binding_test.go:556-579` | 48/48 (100.0%) | 48/48 (100.0%) |
| gin | IN | `context_test.go:3032-3105` + `context_test.go:2775-2837` | 137/137 (100.0%) | 137/137 (100.0%) |
| gin | IN | `render/toml.go:21-31` + `render/yaml.go:21-31` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `recovery_test.go:268-301` + `recovery_test.go:234-265` | 66/66 (100.0%) | 66/66 (100.0%) |
| gin | IN | `binding/binding_test.go:253-263` + `binding/binding_test.go:241-251` | 22/22 (100.0%) | 22/22 (100.0%) |
| gin | IN | `fs_test.go:13-49` + `internal/fs/fs_test.go:13-49` | 74/74 (100.0%) | 74/74 (100.0%) |
| gin | IN | `recovery_test.go:233-266` + `recovery_test.go:268-301` | 68/68 (100.0%) | 68/68 (100.0%) |
| gin | IN | `binding/binding.go:95-127` + `binding/binding_nomsgpack.go:91-121` | 64/64 (100.0%) | 64/64 (100.0%) |
| Giraffe | IN | `src/Giraffe/Routing.fs:310-331` + `src/Giraffe/Routing.fs:278-309` | 54/54 (100.0%) | 54/54 (100.0%) |
| Giraffe | OUT | `tests/Giraffe.Tests/EndpointRoutingTests.fs:18-24` + `tests/Giraffe.Tests/EndpointRoutingTests.fs:180-186` | — | — |
| guzzle | IN | `tests/ClientTest.php:1598-1631` + `tests/Handler/CurlMultiHandlerTest.php:4098-4131` | 68/68 (100.0%) | 68/68 (100.0%) |
| guzzle | IN | `src/Handler/TlsVersion.php:55-63` + `src/Handler/CurlFactory.php:2912-2922` | 20/20 (100.0%) | 20/20 (100.0%) |
| guzzle | IN | `src/Cookie/SetCookie.php:215-228` + `src/Cookie/SetCookie.php:197-210` | 28/28 (100.0%) | 28/28 (100.0%) |
| guzzle | IN | `tests/RedirectMiddlewareTest.php:359-392` + `tests/RedirectMiddlewareTest.php:1241-1273` | 67/67 (100.0%) | 67/67 (100.0%) |
| guzzle | IN | `tests/Handler/CurlHandlerTest.php:327-386` + `tests/Handler/CurlMultiHandlerTest.php:1671-1730` | 120/120 (100.0%) | 120/120 (100.0%) |
| guzzle | IN | `tests/Handler/CurlHandlerTest.php:194-233` + `tests/Handler/CurlMultiHandlerTest.php:1858-1897` | 80/80 (100.0%) | 80/80 (100.0%) |
| guzzle | IN | `src/Handler/StreamHandler.php:594-617` + `src/AuthMiddleware.php:819-842` | 48/48 (100.0%) | 48/48 (100.0%) |
| guzzle | IN | `tests/Cookie/SessionCookieJarTest.php:229-259` + `tests/Cookie/FileCookieJarTest.php:336-366` | 62/62 (100.0%) | 62/62 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:10182-10219` + `tests/Handler/ProxyEnvTest.php:198-235` | 76/76 (100.0%) | 76/76 (100.0%) |
| guzzle | IN | `tests/Handler/StreamHandlerTest.php:5140-5151` + `tests/Handler/StreamHandlerTest.php:5071-5082` | 24/24 (100.0%) | 24/24 (100.0%) |
| guzzle | IN | `tests/Cookie/FileCookieJarTest.php:271-278` + `tests/Cookie/SessionCookieJarTest.php:216-223` | 16/16 (100.0%) | 16/16 (100.0%) |
| guzzle | IN | `src/Handler/CurlMultiHandler.php:531-553` + `src/Handler/CurlFactory.php:602-624` | 46/46 (100.0%) | 46/46 (100.0%) |
| guzzle | IN | `tests/Handler/StreamHandlerTest.php:5063-5095` + `tests/Handler/StreamHandlerTest.php:5132-5164` | 66/66 (100.0%) | 66/66 (100.0%) |
| guzzle | IN | `tests/AuthMiddlewareTest.php:259-274` + `tests/AuthMiddlewareTest.php:614-629` | 32/32 (100.0%) | 32/32 (100.0%) |
| guzzle | IN | `tests/Cookie/CookieJarTest.php:528-541` + `tests/Cookie/CookieJarTest.php:566-579` | 28/28 (100.0%) | 28/28 (100.0%) |
| guzzle | IN | `tests/Handler/StreamHandlerTest.php:2206-2243` + `tests/Handler/ProxyEnvTest.php:198-235` | 76/76 (100.0%) | 76/76 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:6757-6769` + `tests/Handler/CurlFactoryTest.php:6475-6487` | 26/26 (100.0%) | 26/26 (100.0%) |
| guzzle | IN | `tests/AuthMiddlewareTest.php:218-233` + `tests/AuthMiddlewareTest.php:259-274` | 32/32 (100.0%) | 32/32 (100.0%) |
| guzzle | IN | `src/Handler/CurlFactory.php:864-874` + `src/Handler/CurlFactory.php:849-859` | 22/22 (100.0%) | 22/22 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:6447-6467` + `tests/Handler/CurlFactoryTest.php:6477-6496` | 41/41 (100.0%) | 41/41 (100.0%) |
| guzzle | IN | `tests/ClientTest.php:998-1005` + `tests/ClientTest.php:1137-1144` | 16/16 (100.0%) | 16/16 (100.0%) |
| guzzle | IN | `tests/Handler/CurlMultiHandlerTest.php:3661-3676` + `tests/Handler/CurlMultiHandlerTest.php:3693-3706` | 30/30 (100.0%) | 30/30 (100.0%) |
| guzzle | IN | `tests/Handler/CurlMultiHandlerTest.php:2144-2206` + `tests/Handler/CurlMultiHandlerTest.php:2208-2257` | 113/113 (100.0%) | 113/113 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:8968-8991` + `tests/Handler/CurlFactoryTest.php:9292-9323` | 56/56 (100.0%) | 56/56 (100.0%) |
| guzzle | IN | `tests/Handler/CurlMultiHandlerTest.php:3395-3404` + `tests/Handler/CurlMultiHandlerTest.php:3108-3117` | 20/20 (100.0%) | 20/20 (100.0%) |
| guzzle | IN | `tests/Exception/NetworkTimeoutExceptionTest.php:12-36` + `tests/Exception/ConnectExceptionTest.php:12-35` | 49/49 (100.0%) | 49/49 (100.0%) |
| guzzle | IN | `tests/Exception/ConnectTimeoutExceptionTest.php:21-35` + `tests/Exception/NetworkTimeoutExceptionTest.php:21-35` | 30/30 (100.0%) | 30/30 (100.0%) |
| guzzle | IN | `tests/Handler/StreamHandlerTest.php:4972-4981` + `tests/Handler/CurlFactoryTest.php:1119-1128` | 20/20 (100.0%) | 20/20 (100.0%) |
| guzzle | IN | `tests/Exception/ServerExceptionTest.php:20-40` + `tests/Exception/TooManyRedirectsExceptionTest.php:19-37` | 40/40 (100.0%) | 40/40 (100.0%) |
| guzzle | IN | `tests/Handler/StreamHandlerTest.php:3677-3682` + `tests/Handler/CurlFactoryTest.php:9437-9442` | 12/12 (100.0%) | 12/12 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:9173-9178` + `tests/Handler/CurlFactoryTest.php:8852-8857` | 12/12 (100.0%) | 12/12 (100.0%) |
| guzzle | IN | `tests/Exception/NetworkTimeoutExceptionTest.php:1-36` + `tests/Exception/ConnectExceptionTest.php:1-35` | 71/71 (100.0%) | 71/71 (100.0%) |
| guzzle | IN | `tests/Handler/CurlFactoryTest.php:9728-9735` + `tests/Handler/CurlFactoryTest.php:9172-9179` | 8/16 (50.0%) | 8/16 (50.0%) |
| guzzle | IN | `tests/AuthMiddlewareTest.php:482-513` + `tests/AuthMiddlewareTest.php:874-897` | 56/56 (100.0%) | 56/56 (100.0%) |
| guzzle | IN | `tests/Exception/ServerExceptionTest.php:20-40` + `tests/Exception/ClientExceptionTest.php:20-40` | 42/42 (100.0%) | 42/42 (100.0%) |
| guzzle | IN | `tests/PoolTest.php:429-442` + `tests/ClientTest.php:2109-2122` | 28/28 (100.0%) | 28/28 (100.0%) |
| guzzle | OUT | `tests/Handler/CurlFactoryTest.php:2684-2685` + `tests/Handler/CurlFactoryTest.php:326-327` | — | — |
| guzzle | OUT | `tests/ProxyOptionsTest.php:109-115` + `tests/ProxyOptionsTest.php:326-332` | — | — |
| guzzle | OUT | `src/Handler/CurlVersion.php:24-59` + `src/RequestOptions.php:225-370` | — | — |
| guzzle | OUT | `src/Multiplexing.php:19-23` + `src/TransportSharing.php:9-13` | — | — |
| guzzle | OUT | `tests/Exception/ServerExceptionTest.php:5-10` + `tests/Handler/StreamHandlerTest.php:5-10` | — | — |
| guzzle | OUT | `src/Handler/CurlVersion.php:18-59` + `src/RequestOptions.php:236-393` | — | — |
| guzzle | OUT | `src/Multiplexing.php:19-23` + `src/RequestOptions.php:236-356` | — | — |
| guzzle | OUT | `tests/ClientTest.php:2009-2027` + `tests/PoolTest.php:429-447` | — | — |
| guzzle | OUT | `src/RequestOptions.php:236-370` + `src/Handler/CurlVersion.php:35-64` | — | — |
| guzzle | OUT | `src/RequestOptions.php:236-356` + `src/Handler/CurlVersion.php:37-64` | — | — |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:226-240` + `pkgs/cronet_http/example/integration_test/client_profile_test.dart:331-345` | 30/30 (100.0%) | 30/30 (100.0%) |
| http | IN | `pkgs/http_profile/test/http_profile_request_data_test.dart:173-238` + `pkgs/http_profile/test/http_profile_response_data_test.dart:160-225` | 132/132 (100.0%) | 132/132 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_configuration_test.dart:97-114` + `pkgs/cupertino_http/test/url_session_configuration_test.dart:41-58` | 36/36 (100.0%) | 36/36 (100.0%) |
| http | IN | `pkgs/http_parser/test/http_date_test.dart:282-349` + `pkgs/http_parser/test/http_date_test.dart:71-138` | 136/136 (100.0%) | 136/136 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:19-94` + `pkgs/http_client_conformance_tests/lib/src/response_headers_tests.dart:26-65` | 116/116 (100.0%) | 116/116 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_tests.dart:31-57` + `pkgs/http_client_conformance_tests/lib/src/response_body_tests.dart:24-46` | 50/50 (100.0%) | 50/50 (100.0%) |
| http | IN | `pkgs/ok_http/example/lib/main.dart:5-93` + `pkgs/cronet_http/example/lib/main.dart:5-98` | 183/183 (100.0%) | 183/183 (100.0%) |
| http | IN | `pkgs/http2/test/client_pool_test.dart:216-230` + `pkgs/http2/test/client_pool_test.dart:261-275` | 30/30 (100.0%) | 30/30 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:173-197` + `pkgs/cronet_http/example/integration_test/client_profile_test.dart:48-72` | 6/50 (12.0%) | 6/50 (12.0%) |
| http | IN | `pkgs/http_profile/test/http_profile_response_data_test.dart:133-153` + `pkgs/http_profile/test/http_profile_response_data_test.dart:105-126` | 43/43 (100.0%) | 43/43 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:108-144` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:108-143` | 73/73 (100.0%) | 73/73 (100.0%) |
| http | IN | `pkgs/http_profile/test/http_profile_response_data_test.dart:295-330` + `pkgs/http_profile/test/http_profile_request_data_test.dart:252-287` | 72/72 (100.0%) | 72/72 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_tests.dart:27-40` + `pkgs/http_client_conformance_tests/lib/src/response_body_tests.dart:21-34` | 28/28 (100.0%) | 28/28 (100.0%) |
| http | IN | `pkgs/http_profile/lib/src/http_profile_request_data.dart:150-172` + `pkgs/http_profile/lib/src/http_profile_response_data.dart:192-214` | 46/46 (100.0%) | 46/46 (100.0%) |
| http | IN | `pkgs/http2/test/http2_client_test.dart:210-218` + `pkgs/http2/test/http2_client_test.dart:126-135` | 19/19 (100.0%) | 19/19 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/client_profile_test.dart:238-324` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:146-215` | 157/157 (100.0%) | 157/157 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:167-174` + `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:225-233` | — | — |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/request_cookies_test.dart:48-57` + `pkgs/http_client_conformance_tests/lib/src/request_cookies_test.dart:37-46` | 20/20 (100.0%) | 20/20 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_task_test.dart:104-117` + `pkgs/cupertino_http/test/url_session_task_test.dart:184-197` | 28/28 (100.0%) | 28/28 (100.0%) |
| http | IN | `pkgs/web_socket_conformance_tests/lib/src/peer_protocol_errors_tests.dart:15-55` + `pkgs/web_socket_conformance_tests/lib/src/no_upgrade_tests.dart:15-35` | 62/62 (100.0%) | 62/62 (100.0%) |
| http | IN | `pkgs/http2/test/client_test.dart:1289-1300` + `pkgs/http2/test/client_test.dart:1216-1227` | 24/24 (100.0%) | 24/24 (100.0%) |
| http | IN | `pkgs/ok_http/example/integration_test/client_profile_test.dart:93-144` + `pkgs/cupertino_http/test/client_profile_test.dart:171-236` | 118/118 (100.0%) | 118/118 (100.0%) |
| http | IN | `pkgs/http2/test/src/frames/frame_writer_reader_test.dart:70-76` + `pkgs/http2/test/src/frames/frame_writer_reader_test.dart:143-149` | 14/14 (100.0%) | 14/14 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:173-216` + `pkgs/cupertino_http/test/client_profile_test.dart:268-323` | 100/100 (100.0%) | 100/100 (100.0%) |
| http | IN | `pkgs/http2/example/main.dart:9-30` + `pkgs/cupertino_http/example/example.dart:9-30` | 44/44 (100.0%) | 44/44 (100.0%) |
| http | IN | `pkgs/http/lib/src/base_response.dart:122-184` + `pkgs/http_profile/lib/src/utils.dart:7-52` | 109/109 (100.0%) | 109/109 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:398-422` + `pkgs/http2/test/server_test.dart:440-471` | 57/57 (100.0%) | 57/57 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:71-81` + `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:34-44` | 22/22 (100.0%) | 22/22 (100.0%) |
| http | IN | `pkgs/http2/lib/src/streams/stream_handler.dart:837-847` + `pkgs/http2/lib/src/streams/stream_handler.dart:850-862` | 24/24 (100.0%) | 24/24 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/client_profile_test.dart:171-236` + `pkgs/cronet_http/example/integration_test/client_profile_test.dart:93-145` | 119/119 (100.0%) | 119/119 (100.0%) |
| http | IN | `pkgs/http_parser/test/http_date_test.dart:54-69` + `pkgs/http_parser/test/http_date_test.dart:71-86` | 32/32 (100.0%) | 32/32 (100.0%) |
| http | IN | `pkgs/http/test/response_test.dart:171-184` + `pkgs/http/test/response_test.dart:157-169` | 27/27 (100.0%) | 27/27 (100.0%) |
| http | IN | `pkgs/web_socket_conformance_tests/lib/src/protocol_tests.dart:23-43` + `pkgs/web_socket_conformance_tests/lib/src/close_local_tests.dart:24-44` | 42/42 (100.0%) | 42/42 (100.0%) |
| http | IN | `pkgs/http2/test/client_test.dart:1153-1161` + `pkgs/http2/test/client_test.dart:843-852` | 6/19 (31.6%) | 6/19 (31.6%) |
| http | IN | `pkgs/http_parser/test/http_date_test.dart:88-138` + `pkgs/http_parser/test/http_date_test.dart:187-237` | 102/102 (100.0%) | 102/102 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:441-472` + `pkgs/http2/test/server_test.dart:392-423` | 64/64 (100.0%) | 64/64 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:627-671` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:673-717` | 90/90 (100.0%) | 90/90 (100.0%) |
| http | IN | `pkgs/web_socket_channel/lib/adapter_web_socket_channel.dart:135-149` + `pkgs/web_socket_channel/lib/html.dart:167-181` | 30/30 (100.0%) | 30/30 (100.0%) |
| http | IN | `pkgs/http/example/main.dart:13-28` + `pkgs/http2/example/main.dart:9-30` | 38/38 (100.0%) | 38/38 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:137-153` + `pkgs/http2/test/client_test.dart:1385-1396` | 29/29 (100.0%) | 29/29 (100.0%) |
| http | IN | `pkgs/http_parser/lib/src/chunked_coding/decoder.dart:136-144` + `pkgs/http_parser/lib/src/chunked_coding/decoder.dart:126-134` | 18/18 (100.0%) | 18/18 (100.0%) |
| http | IN | `pkgs/http/test/request_test.dart:249-259` + `pkgs/http/test/request_test.dart:237-247` | 22/22 (100.0%) | 22/22 (100.0%) |
| http | IN | `pkgs/http2/test/src/streams/streams_test.dart:150-187` + `pkgs/http2/test/src/streams/streams_test.dart:113-148` | 74/74 (100.0%) | 74/74 (100.0%) |
| http | IN | `pkgs/http_parser/lib/src/authentication_challenge.dart:134-143` + `pkgs/http_parser/lib/src/authentication_challenge.dart:72-81` | 20/20 (100.0%) | 20/20 (100.0%) |
| http | IN | `pkgs/http2/test/client_test.dart:741-756` + `pkgs/http2/test/client_test.dart:411-426` | 32/32 (100.0%) | 32/32 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:20-217` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:18-215` | 396/396 (100.0%) | 396/396 (100.0%) |
| http | IN | `pkgs/http/test/io/client_test.dart:112-147` + `pkgs/http/test/io/client_test.dart:149-185` | 73/73 (100.0%) | 73/73 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:620-653` + `pkgs/http2/test/server_test.dart:829-854` | 60/60 (100.0%) | 60/60 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:34-44` + `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:46-56` | 22/22 (100.0%) | 22/22 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:478-525` + `pkgs/http2/test/server_test.dart:427-474` | 96/96 (100.0%) | 96/96 (100.0%) |
| http | IN | `pkgs/http2/test/server_test.dart:392-403` + `pkgs/http2/test/server_test.dart:769-780` | 24/24 (100.0%) | 24/24 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/client_profile_test.dart:43-120` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:146-215` | 148/148 (100.0%) | 148/148 (100.0%) |
| http | IN | `pkgs/http_parser/test/http_date_test.dart:88-143` + `pkgs/http_parser/test/http_date_test.dart:187-242` | 112/112 (100.0%) | 112/112 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_test.dart:15-46` + `pkgs/ok_http/example/integration_test/client_test.dart:18-56` | 71/71 (100.0%) | 71/71 (100.0%) |
| http | IN | `pkgs/ok_http/example/integration_test/certificate_test.dart:116-124` + `pkgs/ok_http/example/integration_test/certificate_test.dart:79-87` | 18/18 (100.0%) | 18/18 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:33-60` + `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:62-93` | 60/60 (100.0%) | 60/60 (100.0%) |
| http | IN | `pkgs/http2/test/client_test.dart:1292-1302` + `pkgs/http2/test/client_test.dart:1219-1229` | 22/22 (100.0%) | 22/22 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:93-103` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:163-173` | 22/22 (100.0%) | 22/22 (100.0%) |
| http | IN | `pkgs/http_profile/test/close_test.dart:76-113` + `pkgs/http_profile/test/close_test.dart:29-66` | 76/76 (100.0%) | 76/76 (100.0%) |
| http | IN | `pkgs/http2/test/client_test.dart:866-883` + `pkgs/http2/test/client_test.dart:768-785` | 36/36 (100.0%) | 36/36 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:46-52` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:125-130` | 13/13 (100.0%) | 13/13 (100.0%) |
| http | IN | `pkgs/http2/test/src/hpack/hpack_test.dart:473-491` + `pkgs/http2/test/src/hpack/hpack_test.dart:273-291` | 38/38 (100.0%) | 38/38 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/response_headers_server.dart:5-44` + `pkgs/http_client_conformance_tests/lib/src/response_status_line_server.dart:5-43` | 79/79 (100.0%) | 79/79 (100.0%) |
| http | IN | `pkgs/http_profile/test/http_profile_response_data_test.dart:12-27` + `pkgs/http_profile/test/http_profile_request_data_test.dart:12-27` | 32/32 (100.0%) | 32/32 (100.0%) |
| http | IN | `pkgs/web_socket_channel/test/io_test.dart:76-99` + `pkgs/web_socket_channel/test/io_test.dart:53-74` | 46/46 (100.0%) | 46/46 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_server_web.dart:3-12` + `pkgs/http_client_conformance_tests/lib/src/response_body_streamed_server_web.dart:3-12` | 20/20 (100.0%) | 20/20 (100.0%) |
| http | IN | `pkgs/http_profile/lib/src/http_profile_response_data.dart:228-297` + `pkgs/http_profile/lib/src/http_profile_request_data.dart:184-251` | 138/138 (100.0%) | 138/138 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:384-453` + `pkgs/cupertino_http/test/client_profile_test.dart:379-458` | 150/150 (100.0%) | 150/150 (100.0%) |
| http | IN | `pkgs/http2/test/transport_test.dart:408-443` + `pkgs/http2/test/transport_test.dart:445-478` | 70/70 (100.0%) | 70/70 (100.0%) |
| http | IN | `pkgs/http2/test/transport_test.dart:24-52` + `pkgs/http2/test/transport_test.dart:54-82` | 58/58 (100.0%) | 58/58 (100.0%) |
| http | IN | `pkgs/http2/test/client_pool_test.dart:263-277` + `pkgs/http2/test/client_pool_test.dart:103-117` | 30/30 (100.0%) | 30/30 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:788-793` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:800-805` | 12/12 (100.0%) | 12/12 (100.0%) |
| http | IN | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:18-91` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:18-91` | 148/148 (100.0%) | 148/148 (100.0%) |
| http | IN | `pkgs/http_client_conformance_tests/lib/src/response_cookies_server.dart:19-44` + `pkgs/http_client_conformance_tests/lib/src/response_status_line_server.dart:5-43` | 65/65 (100.0%) | 65/65 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:551-564` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:31-44` | 28/28 (100.0%) | 28/28 (100.0%) |
| http | IN | `pkgs/http_profile/lib/src/http_profile_response_data.dart:83-159` + `pkgs/http_profile/lib/src/http_profile_request_data.dart:102-157` | 133/133 (100.0%) | 133/133 (100.0%) |
| http | IN | `pkgs/http2/lib/src/hpack/hpack.dart:235-251` + `pkgs/http2/test/src/hpack/hpack_test.dart:887-906` | 37/37 (100.0%) | 37/37 (100.0%) |
| http | IN | `pkgs/http_profile/test/http_profile_response_data_test.dart:307-354` + `pkgs/http_profile/test/http_profile_request_data_test.dart:240-287` | 96/96 (100.0%) | 96/96 (100.0%) |
| http | IN | `pkgs/flutter_http_example/test/widget_test.dart:145-168` + `pkgs/flutter_http_example/test/widget_test.dart:208-233` | 50/50 (100.0%) | 50/50 (100.0%) |
| http | IN | `pkgs/web_socket_conformance_tests/lib/src/protocol_tests.dart:23-34` + `pkgs/web_socket_conformance_tests/lib/src/close_remote_tests.dart:21-32` | 24/24 (100.0%) | 24/24 (100.0%) |
| http | IN | `pkgs/http/test/io/http_test.dart:90-114` + `pkgs/http/test/io/http_test.dart:316-340` | 50/50 (100.0%) | 50/50 (100.0%) |
| http | IN | `pkgs/http_profile/test/http_client_request_profile_test.dart:99-107` + `pkgs/http_profile/test/http_client_request_profile_test.dart:125-133` | 18/18 (100.0%) | 18/18 (100.0%) |
| http | IN | `pkgs/ok_http/example/integration_test/client_test.dart:19-36` + `pkgs/ok_http/example/integration_test/client_test.dart:38-55` | 36/36 (100.0%) | 36/36 (100.0%) |
| http | IN | `pkgs/cupertino_http/test/url_session_delegate_test.dart:673-763` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:627-671` | 136/136 (100.0%) | 136/136 (100.0%) |
| http | OUT | `pkgs/http2/test/flow_control_test.dart:22-31` + `pkgs/web_socket_channel/lib/status.dart:43-90` | — | — |
| http | OUT | `pkgs/http2/lib/transport.dart:79-120` + `pkgs/http2/lib/src/settings/settings.dart:19-72` | — | — |
| http | OUT | `pkgs/http2/lib/transport.dart:79-128` + `pkgs/http2/lib/src/connection.dart:134-153` | — | — |
| hugo | IN | `resources/image_cache.go:86-91` + `resources/image_cache.go:61-66` | 12/12 (100.0%) | 12/12 (100.0%) |
| hugo | IN | `tpl/collections/collections.go:195-227` + `tpl/collections/collections.go:355-387` | 66/66 (100.0%) | 66/66 (100.0%) |
| hugo | IN | `resources/page/pages_sort.go:160-172` + `navigation/menu.go:215-233` | 32/32 (100.0%) | 32/32 (100.0%) |
| hugo | IN | `markup/goldmark/goldmark_integration_test.go:335-351` + `hugolib/cascade_test.go:73-89` | 34/34 (100.0%) | 34/34 (100.0%) |
| hugo | IN | `tpl/internal/go_templates/texttemplate/exec_test.go:360-364` + `tpl/internal/go_templates/htmltemplate/exec_test.go:354-358` | 10/10 (100.0%) | 10/10 (100.0%) |
| hugo | IN | `resources/page/pagegroup_test.go:350-358` + `resources/page/pagegroup_test.go:399-407` | 18/18 (100.0%) | 18/18 (100.0%) |
| hugo | IN | `resources/page/taxonomy.go:148-162` + `navigation/menu.go:179-193` | 30/30 (100.0%) | 30/30 (100.0%) |
| hugo | IN | `tpl/urls/urls.go:93-104` + `tpl/urls/urls.go:107-119` | 25/25 (100.0%) | 25/25 (100.0%) |
| hugo | IN | `resources/page/pagegroup_test.go:276-284` + `resources/page/pagegroup_test.go:447-455` | 18/18 (100.0%) | 18/18 (100.0%) |
| hugo | IN | `helpers/path_test.go:113-134` + `common/paths/path_test.go:55-76` | 44/44 (100.0%) | 44/44 (100.0%) |
| hugo | IN | `markup/markup_config/config_test.go:73-79` + `markup/markup_config/config_test.go:60-66` | 14/14 (100.0%) | 14/14 (100.0%) |
| hugo | IN | `tpl/collections/index.go:123-128` + `tpl/internal/go_templates/texttemplate/funcs.go:161-166` | 12/12 (100.0%) | 12/12 (100.0%) |
| hugo | OUT | `tpl/internal/go_templates/texttemplate/exec_test.go:605-611` + `tpl/internal/go_templates/texttemplate/exec_test.go:618-624` | — | — |
| hugo | OUT | `tpl/urls/init.go:47-58` + `tpl/collections/init.go:151-164` | — | — |
| hugo | OUT | `hugolib/hugo_sites_build_test.go:208-244` + `hugolib/menu_test.go:111-146` | — | — |
| hugo | OUT | `tpl/lang/init.go:35-52` + `tpl/strings/init.go:51-69` | — | — |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/exec_test.go:497-500` + `tpl/internal/go_templates/htmltemplate/exec_test.go:664-668` | — | — |
| hugo | OUT | `common/paths/path_test.go:136-143` + `tpl/internal/go_templates/htmltemplate/css_test.go:220-227` | — | — |
| hugo | OUT | `config/allconfig/allconfig_integration_test.go:135-166` + `tpl/transform/transform_integration_test.go:353-381` | — | — |
| hugo | OUT | `tpl/internal/go_templates/texttemplate/exec_test.go:443-452` + `tpl/internal/go_templates/htmltemplate/exec_test.go:532-539` | — | — |
| hugo | OUT | `htesting/test_helpers_test.go:25-27` + `hugofs/glob_test.go:75-78` | — | — |
| hugo | OUT | `hugolib/site_output_test.go:489-543` + `hugolib/pagebundler_test.go:381-418` | — | — |
| hugo | OUT | `tpl/partials/partials_integration_test.go:298-299` + `hugolib/site_url_test.go:114-115` | — | — |
| hugo | OUT | `hugolib/site_output_test.go:657-689` + `hugolib/page__meta_test.go:24-47` | 57/57 (100.0%) | 8/57 (14.0%) |
| hugo | OUT | `hugolib/testhelpers_test.go:531-533` + `langs/i18n/translationProvider.go:55-57` | — | — |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/exec_test.go:507-514` + `tpl/internal/go_templates/texttemplate/exec_test.go:615-622` | 16/16 (100.0%) | 16/16 (100.0%) |
| hugo | OUT | `resources/images/config_test.go:38-39` + `media/config_test.go:68-69` | — | — |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/css_test.go:220-227` + `resources/page/pagemeta/page_frontmatter_test.go:165-172` | — | — |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/escape_test.go:1141-1174` + `tpl/internal/go_templates/htmltemplate/css_test.go:93-99` | — | — |
| hugo | OUT | `commands/server.go:550-551` + `commands/server.go:545-546` | 4/4 (100.0%) | 4/4 (100.0%) |
| hugo | OUT | `tpl/tplimpl/templatestore_integration_test.go:1438-1461` + `tpl/tplimpl/templatestore_integration_test.go:367-394` | 52/52 (100.0%) | — |
| hugo | OUT | `hugolib/content_map_test.go:55-67` + `hugolib/content_map_test.go:79-82` | — | — |
| hugo | OUT | `resources/image_test.go:394-395` + `hugolib/pagebundler_test.go:285-286` | — | — |
| hugo | OUT | `common/urls/baseURL_test.go:79-80` + `resources/resource_transformers/tocss/dartsass/dartsass_integration_test.go:361-362` | — | — |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/html_test.go:60-67` + `tpl/internal/go_templates/htmltemplate/css_test.go:78-85` | — | — |
| hugo | OUT | `tpl/tplimpl/templatestore_integration_test.go:1076-1114` + `tpl/tplimpl/templatestore_integration_test.go:908-925` | 57/57 (100.0%) | 57/57 (100.0%) |
| hugo | OUT | `config/allconfig/allconfig_integration_test.go:76-84` + `config/allconfig/allconfig_integration_test.go:17-48` | — | — |
| hugo | OUT | `tpl/internal/go_templates/texttemplate/exec_test.go:1311-1313` + `resources/page/pagemeta/pagemeta_test.go:110-112` | — | — |
| hugo | OUT | `resources/image_test.go:304-306` + `hugolib/disableKinds_test.go:175-176` | 5/5 (100.0%) | 5/5 (100.0%) |
| hugo | OUT | `tpl/internal/go_templates/htmltemplate/exec_test.go:569-578` + `tpl/internal/go_templates/htmltemplate/exec_test.go:471-486` | — | — |
| hugo | OUT | `resources/page/permalinks_test.go:43-48` + `hugolib/site_test.go:958-967` | — | — |
| mocktail | IN | `packages/mocktail/test/mockito_compat/capture_test.dart:129-138` + `packages/mocktail/test/mockito_compat/capture_test.dart:113-122` | 20/20 (100.0%) | 20/20 (100.0%) |
| mocktail | IN | `packages/mocktail_image_network/test/mocktail_image_network_test.dart:183-201` + `packages/mocktail_image_network/test/mocktail_image_network_test.dart:163-181` | 38/38 (100.0%) | 38/38 (100.0%) |
| mocktail | IN | `packages/mocktail/lib/src/_register_matcher.dart:113-120` + `packages/mocktail/lib/src/_register_matcher.dart:103-110` | 16/16 (100.0%) | 16/16 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/until_called_test.dart:130-144` + `packages/mocktail/test/mockito_compat/until_called_test.dart:284-298` | 30/30 (100.0%) | 30/30 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/until_called_test.dart:351-367` + `packages/mocktail/test/mockito_compat/until_called_test.dart:185-201` | 34/34 (100.0%) | 34/34 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/capture_test.dart:109-123` + `packages/mocktail/test/mockito_compat/capture_test.dart:125-139` | 30/30 (100.0%) | 30/30 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/until_called_test.dart:274-299` + `packages/mocktail/test/mockito_compat/until_called_test.dart:340-368` | 55/55 (100.0%) | 55/55 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/verify_test.dart:232-247` + `packages/mocktail/test/mockito_compat/verify_test.dart:264-279` | 32/32 (100.0%) | 32/32 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mocktail_test.dart:326-351` + `packages/mocktail/test/mocktail_test.dart:308-323` | 42/42 (100.0%) | 42/42 (100.0%) |
| mocktail | IN | `packages/mocktail/test/mockito_compat/invocation_matcher_test.dart:95-106` + `packages/mocktail/test/mockito_compat/invocation_matcher_test.dart:113-124` | 24/24 (100.0%) | 24/24 (100.0%) |
| mocktail | IN | `packages/mocktail_image_network/test/mocktail_image_network_test.dart:55-65` + `packages/mocktail_image_network/test/mocktail_image_network_test.dart:100-110` | 22/22 (100.0%) | 22/22 (100.0%) |
| monolog | IN | `src/Monolog/Handler/FilterHandler.php:153-164` + `src/Monolog/Handler/SamplingHandler.php:81-92` | 24/24 (100.0%) | 24/24 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/FingersCrossedHandlerTest.php:225-230` + `tests/Monolog/Handler/FingersCrossedHandlerTest.php:209-214` | 12/12 (100.0%) | 12/12 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/LogmaticHandlerTest.php:37-46` + `tests/Monolog/Handler/LogEntriesHandlerTest.php:37-46` | 20/20 (100.0%) | 20/20 (100.0%) |
| monolog | IN | `src/Monolog/Handler/SlackWebhookHandler.php:70-80` + `src/Monolog/Handler/SlackHandler.php:81-89` | 20/20 (100.0%) | 20/20 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/RedisHandlerTest.php:25-67` + `tests/Monolog/Handler/RedisPubSubHandlerTest.php:21-68` | 91/91 (100.0%) | 91/91 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/GroupHandlerTest.php:121-152` + `tests/Monolog/Handler/FallbackGroupHandlerTest.php:142-173` | 64/64 (100.0%) | 64/64 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/GelfMessageFormatterTest.php:61-78` + `tests/Monolog/Formatter/GelfMessageFormatterTest.php:127-148` | 40/40 (100.0%) | 40/40 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/RotatingFileHandlerTest.php:177-214` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:253-290` | 76/76 (100.0%) | 76/76 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/GelfMessageFormatterTest.php:95-105` + `tests/Monolog/Formatter/GelfMessageFormatterTest.php:180-190` | 22/22 (100.0%) | 22/22 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/InsightOpsHandlerTest.php:70-80` + `tests/Monolog/Handler/LogmaticHandlerTest.php:74-84` | 22/22 (100.0%) | 22/22 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/GroupHandlerTest.php:17-153` + `tests/Monolog/Handler/WhatFailureGroupHandlerTest.php:17-172` | 293/293 (100.0%) | 293/293 (100.0%) |
| monolog | IN | `tests/Monolog/Processor/WebProcessorTest.php:63-70` + `tests/Monolog/Processor/WebProcessorTest.php:50-57` | 16/16 (100.0%) | 16/16 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/WhatFailureGroupHandlerTest.php:49-119` + `tests/Monolog/Handler/GroupHandlerTest.php:49-119` | 12/142 (8.5%) | 12/142 (8.5%) |
| monolog | IN | `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:146-160` + `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:162-178` | 32/32 (100.0%) | 32/32 (100.0%) |
| monolog | IN | `src/Monolog/Handler/FlowdockHandler.php:48-64` + `src/Monolog/Handler/SlackHandler.php:66-79` | 31/31 (100.0%) | 31/31 (100.0%) |
| monolog | IN | `src/Monolog/Handler/SlackWebhookHandler.php:115-129` + `src/Monolog/Handler/SlackHandler.php:177-191` | 30/30 (100.0%) | 30/30 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/ElasticaHandlerTest.php:88-106` + `tests/Monolog/Handler/ElasticsearchHandlerTest.php:61-80` | 39/39 (100.0%) | 39/39 (100.0%) |
| monolog | IN | `tests/Monolog/LoggerTest.php:458-467` + `tests/Monolog/LoggerTest.php:427-436` | 20/20 (100.0%) | 20/20 (100.0%) |
| monolog | IN | `src/Monolog/Handler/RollbarHandler.php:61-73` + `src/Monolog/Formatter/ChromePHPFormatter.php:29-41` | 26/26 (100.0%) | 26/26 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/SyslogUdpHandlerTest.php:34-54` + `tests/Monolog/Handler/SyslogUdpHandlerTest.php:87-106` | 41/41 (100.0%) | 41/41 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/ZendMonitorHandlerTest.php:44-54` + `tests/Monolog/Handler/FrankenPhpHandlerTest.php:79-89` | 22/22 (100.0%) | 22/22 (100.0%) |
| monolog | IN | `tests/Monolog/Processor/MemoryPeakUsageProcessorTest.php:1-40` + `tests/Monolog/Processor/MemoryUsageProcessorTest.php:1-40` | 80/80 (100.0%) | 80/80 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/LogstashFormatterTest.php:76-89` + `tests/Monolog/Formatter/LogstashFormatterTest.php:106-119` | 28/28 (100.0%) | 28/28 (100.0%) |
| monolog | IN | `tests/Monolog/LoggerTest.php:444-470` + `tests/Monolog/LoggerTest.php:412-439` | 55/55 (100.0%) | 55/55 (100.0%) |
| monolog | IN | `src/Monolog/Formatter/JsonFormatter.php:172-190` + `src/Monolog/Formatter/NormalizerFormatter.php:209-231` | 42/42 (100.0%) | 42/42 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/LogstashFormatterTest.php:104-129` + `tests/Monolog/Formatter/LogstashFormatterTest.php:74-99` | 52/52 (100.0%) | 52/52 (100.0%) |
| monolog | IN | `src/Monolog/Handler/InsightOpsHandler.php:23-74` + `src/Monolog/Handler/LogEntriesHandler.php:20-68` | 101/101 (100.0%) | 101/101 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/NormalizerFormatterTest.php:125-141` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:105-121` | 34/34 (100.0%) | 34/34 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/FlowdockHandlerTest.php:26-43` + `tests/Monolog/Handler/SlackHandlerTest.php:28-45` | 36/36 (100.0%) | 36/36 (100.0%) |
| monolog | IN | `src/Monolog/Handler/WhatFailureGroupHandler.php:23-80` + `src/Monolog/Handler/FallbackGroupHandler.php:24-68` | 103/103 (100.0%) | 103/103 (100.0%) |
| monolog | IN | `src/Monolog/Handler/FrankenPhpHandler.php:28-100` + `src/Monolog/Handler/ZendMonitorHandler.php:25-90` | 139/139 (100.0%) | 139/139 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/JsonFormatterTest.php:267-278` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:603-613` | 23/23 (100.0%) | 23/23 (100.0%) |
| monolog | IN | `src/Monolog/Formatter/GelfMessageFormatter.php:117-128` + `src/Monolog/Formatter/GelfMessageFormatter.php:130-141` | 24/24 (100.0%) | 24/24 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/NormalizerFormatterTest.php:200-227` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:238-261` | 52/52 (100.0%) | 52/52 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:49-63` + `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:33-47` | 30/30 (100.0%) | 30/30 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/LogmaticHandlerTest.php:20-86` + `tests/Monolog/Handler/LogEntriesHandlerTest.php:20-86` | 134/134 (100.0%) | 134/134 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/FlowdockHandlerTest.php:66-88` + `tests/Monolog/Handler/SlackHandlerTest.php:132-154` | 46/46 (100.0%) | 46/46 (100.0%) |
| monolog | IN | `src/Monolog/Processor/MemoryUsageProcessor.php:1-39` + `src/Monolog/Processor/MemoryPeakUsageProcessor.php:1-39` | 78/78 (100.0%) | 78/78 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/GelfHandlerTest.php:72-91` + `tests/Monolog/Handler/GelfHandlerTest.php:51-70` | 40/40 (100.0%) | 40/40 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/AmqpHandlerTest.php:105-132` + `tests/Monolog/Handler/AmqpHandlerTest.php:48-72` | 53/53 (100.0%) | 53/53 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/LogEntriesHandlerTest.php:67-75` + `tests/Monolog/Handler/FlowdockHandlerTest.php:69-76` | 17/17 (100.0%) | 17/17 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/LogmaticHandlerTest.php:25-62` + `tests/Monolog/Handler/LogEntriesHandlerTest.php:25-62` | 76/76 (100.0%) | 76/76 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/NormalizerFormatterTest.php:530-542` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:463-474` | 25/25 (100.0%) | 25/25 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/FirePHPHandlerTest.php:48-63` + `tests/Monolog/Handler/FirePHPHandlerTest.php:29-40` | 28/28 (100.0%) | 28/28 (100.0%) |
| monolog | IN | `src/Monolog/Formatter/HtmlFormatter.php:102-109` + `src/Monolog/Formatter/HtmlFormatter.php:94-101` | 16/16 (100.0%) | 16/16 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/SocketHandlerTest.php:121-131` + `tests/Monolog/Handler/SocketHandlerTest.php:95-105` | 22/22 (100.0%) | 22/22 (100.0%) |
| monolog | IN | `src/Monolog/Formatter/WildfireFormatter.php:42-54` + `src/Monolog/Formatter/ChromePHPFormatter.php:29-41` | 26/26 (100.0%) | 26/26 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/RotatingFileHandlerTest.php:224-251` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:148-175` | 56/56 (100.0%) | 56/56 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/NormalizerFormatterTest.php:263-293` + `tests/Monolog/Formatter/JsonFormatterTest.php:307-337` | 62/62 (100.0%) | 62/62 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/RotatingFileHandlerTest.php:367-373` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:162-169` | 6/15 (40.0%) | 6/15 (40.0%) |
| monolog | IN | `tests/Monolog/Handler/GroupHandlerTest.php:76-119` + `tests/Monolog/Handler/FallbackGroupHandlerTest.php:96-140` | 89/89 (100.0%) | 89/89 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/FingersCrossedHandlerTest.php:271-282` + `tests/Monolog/Handler/FingersCrossedHandlerTest.php:255-266` | 24/24 (100.0%) | 24/24 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/GroupHandlerTest.php:94-135` + `tests/Monolog/Handler/FallbackGroupHandlerTest.php:114-156` | 85/85 (100.0%) | 85/85 (100.0%) |
| monolog | IN | `src/Monolog/Handler/ElasticsearchHandler.php:82-152` + `src/Monolog/Handler/ElasticaHandler.php:66-125` | 131/131 (100.0%) | 131/131 (100.0%) |
| monolog | IN | `src/Monolog/Handler/SamplingHandler.php:81-120` + `src/Monolog/Handler/FingersCrossedHandler.php:202-241` | 24/80 (30.0%) | 24/80 (30.0%) |
| monolog | IN | `tests/Monolog/Handler/ElasticaHandlerTest.php:121-130` + `tests/Monolog/Handler/ElasticsearchHandlerTest.php:71-80` | 20/20 (100.0%) | 20/20 (100.0%) |
| monolog | IN | `tests/Monolog/Formatter/JsonFormatterTest.php:174-195` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:335-355` | 43/43 (100.0%) | 43/43 (100.0%) |
| monolog | IN | `tests/Monolog/LoggerTest.php:563-576` + `tests/Monolog/LoggerTest.php:585-599` | 29/29 (100.0%) | 29/29 (100.0%) |
| monolog | IN | `tests/Monolog/Handler/DeduplicationHandlerTest.php:27-34` + `tests/Monolog/Handler/BufferHandlerTest.php:131-136` | 14/14 (100.0%) | 14/14 (100.0%) |
| nest | IN | `integration/microservices/src/redis/redis.controller.ts:25-47` + `integration/microservices/src/tcp-tls/app.controller.ts:69-91` | 46/46 (100.0%) | 46/46 (100.0%) |
| nest | IN | `integration/versioning/e2e/media-type-versioning.spec.ts:399-441` + `integration/versioning/e2e/media-type-versioning-fastify.spec.ts:409-451` | 86/86 (100.0%) | 86/86 (100.0%) |
| nest | IN | `integration/microservices/e2e/sum-rpc-tls.spec.ts:61-129` + `integration/microservices/e2e/sum-rpc.spec.ts:46-114` | 138/138 (100.0%) | 138/138 (100.0%) |
| nest | IN | `integration/microservices/e2e/sum-rpc-tls.spec.ts:68-137` + `integration/microservices/e2e/sum-rpc.spec.ts:53-122` | 140/140 (100.0%) | 140/140 (100.0%) |
| nest | IN | `integration/versioning/e2e/uri-versioning-fastify.spec.ts:207-363` + `integration/versioning/e2e/uri-versioning.spec.ts:197-353` | 314/314 (100.0%) | 314/314 (100.0%) |
| nest | IN | `packages/core/test/errors/test/exceptions-zone.spec.ts:21-33` + `packages/core/test/errors/test/exceptions-zone.spec.ts:76-88` | 26/26 (100.0%) | 26/26 (100.0%) |
| nest | IN | `integration/versioning/e2e/media-type-versioning-fastify.spec.ts:31-343` + `integration/versioning/e2e/media-type-versioning.spec.ts:24-336` | 626/626 (100.0%) | 626/626 (100.0%) |
| nest | IN | `packages/core/test/hooks/on-module-init.hook.spec.ts:19-37` + `packages/core/test/hooks/before-app-shutdown.hook.spec.ts:19-37` | 38/38 (100.0%) | 38/38 (100.0%) |
| nest | IN | `sample/32-graphql-federation-schema-first/users-application/src/users/users.resolver.spec.ts:1-39` + `sample/31-graphql-federation-code-first/users-application/src/users/users.resolver.spec.ts:1-39` | 78/78 (100.0%) | 78/78 (100.0%) |
| nest | IN | `sample/32-graphql-federation-schema-first/posts-application/eslint.config.mjs:8-42` + `sample/31-graphql-federation-code-first/users-application/eslint.config.mjs:8-42` | 70/70 (100.0%) | 70/70 (100.0%) |
| nest | IN | `sample/31-graphql-federation-code-first/users-application/src/users/users.resolver.spec.ts:6-39` + `sample/32-graphql-federation-schema-first/users-application/src/users/users.resolver.spec.ts:6-39` | 68/68 (100.0%) | 68/68 (100.0%) |
| nest | IN | `integration/versioning/e2e/header-versioning-fastify.spec.ts:31-343` + `integration/versioning/e2e/header-versioning.spec.ts:24-336` | 626/626 (100.0%) | 626/626 (100.0%) |
| nest | IN | `sample/01-cats-app/e2e/cats/cats.e2e-spec.ts:1-34` + `sample/36-hmr-esm/e2e/cats/cats.e2e-spec.ts:1-34` | 68/68 (100.0%) | 68/68 (100.0%) |
| nest | IN | `integration/microservices/src/app.controller.ts:99-114` + `integration/microservices/src/tcp-tls/app.controller.ts:112-127` | 32/32 (100.0%) | 32/32 (100.0%) |
| nest | IN | `integration/inspector/src/users/users.controller.ts:14-42` + `integration/repl/src/users/users.controller.ts:14-42` | 58/58 (100.0%) | 58/58 (100.0%) |
| nest | IN | `integration/versioning/e2e/custom-versioning-fastify.spec.ts:780-798` + `integration/versioning/e2e/custom-versioning-fastify.spec.ts:299-317` | 38/38 (100.0%) | 38/38 (100.0%) |
| nest | IN | `integration/versioning/e2e/header-versioning-fastify.spec.ts:365-671` + `integration/versioning/e2e/header-versioning.spec.ts:355-661` | 614/614 (100.0%) | 614/614 (100.0%) |
| nest | IN | `sample/01-cats-app/src/cats/cats.controller.ts:15-32` + `sample/36-hmr-esm/src/cats/cats.controller.ts:15-32` | 36/36 (100.0%) | 36/36 (100.0%) |
| nest | IN | `integration/microservices/src/app.controller.ts:32-45` + `integration/microservices/src/tcp-tls/app.controller.ts:45-58` | 28/28 (100.0%) | 28/28 (100.0%) |
| nest | IN | `integration/microservices/src/app.controller.ts:99-117` + `integration/microservices/src/tcp-tls/app.controller.ts:112-130` | 38/38 (100.0%) | 38/38 (100.0%) |
| nest | IN | `packages/platform-express/multer/interceptors/file.interceptor.ts:1-67` + `packages/platform-express/multer/interceptors/files.interceptor.ts:1-70` | 137/137 (100.0%) | 137/137 (100.0%) |
| nest | OUT | `packages/common/decorators/core/index.ts:1-14` + `packages/microservices/index.ts:7-21` | — | — |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextWriterAsyncTests.cs:600-643` + `Src/Newtonsoft.Json.Tests/JsonTextWriterAsyncTests.cs:521-552` | 76/76 (100.0%) | 76/76 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:141-150` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:165-174` | 20/20 (100.0%) | 20/20 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:609-620` + `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:588-599` | — | — |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonValidatingReaderTests.cs:159-169` + `Src/Newtonsoft.Json.Tests/JsonValidatingReaderTests.cs:1056-1066` | 22/22 (100.0%) | 22/22 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Linq/JsonPath/JPathExecuteTests.cs:848-855` + `Src/Newtonsoft.Json.Tests/Linq/JsonPath/JPathExecuteTests.cs:831-838` | 16/16 (100.0%) | 16/16 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Documentation/Samples/Serializer/ErrorHandlingAttribute.cs:45-76` + `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:289-320` | 64/64 (100.0%) | 64/64 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Documentation/TraceWriterTests.cs:135-171` + `Src/Newtonsoft.Json.Tests/Serialization/TraceWriterTests.cs:285-305` | 58/58 (100.0%) | 58/58 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/JsonTextReader.Async.cs:1567-1675` + `Src/Newtonsoft.Json/JsonTextReader.cs:497-601` | 214/214 (100.0%) | 214/214 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Serialization/ContractResolverTests.cs:56-86` + `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:881-909` | 60/60 (100.0%) | 60/60 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/Linq/JValue.cs:303-312` + `Src/Newtonsoft.Json/Linq/JValue.cs:278-287` | 20/20 (100.0%) | 20/20 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:113-231` + `Src/Newtonsoft.Json.Tests/Documentation/Samples/Serializer/SerializationCallbackAttributes.cs:43-142` | 219/219 (100.0%) | 219/219 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:112-285` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:198-371` | 348/348 (100.0%) | 348/348 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:1013-1019` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1167-1173` | 14/14 (100.0%) | 14/14 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:757-845` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:740-828` | 178/178 (100.0%) | 178/178 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Linq/MergeTests.cs:306-353` + `Src/Newtonsoft.Json.Tests/Linq/MergeTests.cs:514-558` | 93/93 (100.0%) | 93/93 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/Converters/XmlNodeConverter.cs:174-186` + `Src/Newtonsoft.Json/Converters/XmlNodeConverter.cs:420-432` | 26/26 (100.0%) | 26/26 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/Linq/JContainer.cs:821-899` + `Src/Newtonsoft.Json/Linq/JContainer.Async.cs:82-160` | 158/158 (100.0%) | 158/158 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:468-477` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:323-332` | 20/20 (100.0%) | 20/20 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:81-88` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:115-122` | 16/16 (100.0%) | 16/16 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:589-600` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:526-537` | 24/24 (100.0%) | 24/24 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1064-1086` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1088-1109` | 22/45 (48.9%) | 22/45 (48.9%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Serialization/TimeOnlyTests.cs:64-98` + `Src/Newtonsoft.Json.Tests/Serialization/DateOnlyTests.cs:55-89` | 70/70 (100.0%) | 70/70 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Linq/JTokenTests.cs:886-899` + `Src/Newtonsoft.Json.Tests/Linq/JTokenTests.cs:836-849` | 28/28 (100.0%) | 28/28 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:387-395` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:324-332` | 18/18 (100.0%) | 18/18 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/Utilities/LinqBridge.cs:1752-1758` + `Src/Newtonsoft.Json/Utilities/LinqBridge.cs:1797-1803` | 14/14 (100.0%) | 14/14 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextWriterTest.cs:599-607` + `Src/Newtonsoft.Json.Tests/JsonTextWriterTest.cs:647-655` | 18/18 (100.0%) | 18/18 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Serialization/NullValueHandlingTests.cs:135-143` + `Src/Newtonsoft.Json.Tests/Serialization/NullValueHandlingTests.cs:156-164` | 18/18 (100.0%) | 18/18 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:508-530` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:574-596` | 46/46 (100.0%) | 46/46 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:209-219` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:902-912` | 22/22 (100.0%) | 22/22 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:568-595` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:539-566` | 56/56 (100.0%) | 56/56 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/TestObjects/EnumerableClass.cs:1-34` + `Src/Newtonsoft.Json.Tests/TestObjects/GenericIEnumerableWithImplicitConversion.cs:1-34` | 68/68 (100.0%) | 68/68 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:814-824` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:698-708` | 22/22 (100.0%) | 22/22 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json/JsonTextReader.cs:2601-2621` + `Src/Newtonsoft.Json/JsonTextWriter.cs:191-207` | 38/38 (100.0%) | 38/38 (100.0%) |
| Newtonsoft.Json | IN | `Src/Newtonsoft.Json.Tests/Serialization/JsonSerializerTest.cs:7316-7329` + `Src/Newtonsoft.Json.Tests/Serialization/JsonSerializerTest.cs:7333-7346` | 28/28 (100.0%) | 28/28 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2082-2107` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2472-2497` | 52/52 (100.0%) | 52/52 (100.0%) |
| Polly | IN | `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:467-479` + `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:804-816` | 26/26 (100.0%) | 26/26 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Retry/RetryAsyncSpecs.cs:378-382` + `src/Polly.SharedSpecs/Retry/RetryTResultSpecsAsync.cs:438-442` | 10/10 (100.0%) | 10/10 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2537-2561` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1901-1925` | 50/50 (100.0%) | 50/50 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/PolicyContextAndKeyAsyncSpecs.cs:206-283` + `src/Polly.SharedSpecs/PolicyContextAndKeySpecs.cs:202-280` | 157/157 (100.0%) | 157/157 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:1198-1203` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:1132-1137` | 12/12 (100.0%) | 12/12 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecsAsync.cs:38-109` + `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecs.cs:37-109` | 145/145 (100.0%) | 145/145 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:66-158` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:65-157` | 186/186 (100.0%) | 186/186 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:66-114` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:65-113` | 98/98 (100.0%) | 98/98 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2161-2184` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2223-2246` | 48/48 (100.0%) | 48/48 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1508-1532` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1470-1494` | 50/50 (100.0%) | 50/50 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1427-1450` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1280-1303` | 48/48 (100.0%) | 48/48 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Caching/SerializingCacheProviderAsyncSpecs.cs:14-46` + `src/Polly.SharedSpecs/Caching/SerializingCacheProviderSpecs.cs:12-44` | 66/66 (100.0%) | 66/66 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Retry/WaitAndRetrySpecs.cs:330-341` + `src/Polly.SharedSpecs/Retry/WaitAndRetrySpecs.cs:375-386` | 24/24 (100.0%) | 24/24 (100.0%) |
| Polly | IN | `src/Polly.Shared/Retry/RetrySyntaxAsync.cs:531-535` + `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:403-407` | 10/10 (100.0%) | 10/10 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2471-2494` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2571-2594` | 48/48 (100.0%) | 48/48 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecsAsync.cs:18-104` + `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecs.cs:17-104` | 175/175 (100.0%) | 175/175 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:998-1010` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:988-1000` | 24/26 (92.3%) | 24/26 (92.3%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultMixedResultExceptionSpecs.cs:387-398` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultMixedResultExceptionSpecs.cs:42-53` | 24/24 (100.0%) | 24/24 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/Caching/CacheAsyncSpecs.cs:17-48` + `src/Polly.SharedSpecs/Caching/CacheSpecs.cs:16-47` | 64/64 (100.0%) | 64/64 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:1027-1043` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:981-997` | 34/34 (100.0%) | 34/34 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:234-251` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:665-682` | 36/36 (100.0%) | 36/36 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:44-138` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:43-137` | 190/190 (100.0%) | 190/190 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:496-515` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:408-427` | 40/40 (100.0%) | 40/40 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:294-312` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:333-351` | 38/38 (100.0%) | 38/38 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:152-156` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:153-157` | 10/10 (100.0%) | 10/10 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:805-812` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:813-820` | 16/16 (100.0%) | 16/16 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:337-354` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:298-315` | 36/36 (100.0%) | 36/36 (100.0%) |
| Polly | IN | `src/Polly.Shared/Wrap/PolicyWrapSyntaxAsync.cs:8-42` + `src/Polly.Shared/Wrap/PolicyWrapSyntax.cs:8-42` | 70/70 (100.0%) | 70/70 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:404-429` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:501-526` | 52/52 (100.0%) | 52/52 (100.0%) |
| Polly | IN | `src/Polly.Shared/Caching/SerializingCacheProvider.cs:59-100` + `src/Polly.Shared/Caching/SerializingCacheProvider.cs:10-51` | 84/84 (100.0%) | 84/84 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1482-1497` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1521-1536` | 32/32 (100.0%) | 32/32 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2437-2461` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2537-2561` | 50/50 (100.0%) | 50/50 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1378-1401` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1280-1303` | 48/48 (100.0%) | 48/48 (100.0%) |
| Polly | IN | `src/Polly.Shared/Wrap/PolicyWrapSyntaxAsync.cs:44-77` + `src/Polly.Shared/Wrap/PolicyWrapSyntax.cs:44-77` | 68/68 (100.0%) | 68/68 (100.0%) |
| Polly | IN | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1583-1608` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:501-526` | 50/52 (96.2%) | 50/52 (96.2%) |
| Polly | OUT | `src/Polly.Shared/Bulkhead/BulkheadEngine.cs:15-21` + `src/Polly.Shared/Caching/CacheEngine.cs:13-18` | — | — |
| Polly | OUT | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:334-337` + `src/Polly.SharedSpecs/Fallback/FallbackAsyncSpecs.cs:577-604` | — | — |
| rayon | IN | `src/iter/skip_any_while.rs:10-30` + `src/iter/take_any_while.rs:10-30` | 42/42 (100.0%) | 42/42 (100.0%) |
| rayon | IN | `src/iter/flatten_iter.rs:7-75` + `src/iter/flatten.rs:7-76` | 139/139 (100.0%) | 139/139 (100.0%) |
| rayon | IN | `src/slice/mod.rs:978-1014` + `src/slice/mod.rs:940-974` | 72/72 (100.0%) | 72/72 (100.0%) |
| rayon | IN | `src/str.rs:586-596` + `src/str.rs:486-496` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `src/iter/skip_any_while.rs:55-110` + `src/iter/fold.rs:226-288` | 119/119 (100.0%) | 119/119 (100.0%) |
| rayon | IN | `src/slice/chunks.rs:38-59` + `src/slice/rchunks.rs:218-239` | 44/44 (100.0%) | 44/44 (100.0%) |
| rayon | IN | `src/iter/multizip.rs:151-165` + `src/iter/multizip.rs:167-181` | 30/30 (100.0%) | 30/30 (100.0%) |
| rayon | IN | `src/iter/try_reduce.rs:8-78` + `src/iter/reduce.rs:4-70` | 138/138 (100.0%) | 138/138 (100.0%) |
| rayon | IN | `src/iter/map.rs:162-202` + `src/iter/flat_map_iter.rs:51-91` | 82/82 (100.0%) | 82/82 (100.0%) |
| rayon | IN | `src/collections/btree_set.rs:18-45` + `src/result.rs:31-48` | 46/46 (100.0%) | 46/46 (100.0%) |
| rayon | IN | `rayon-demo/src/join_microbench.rs:36-45` + `rayon-demo/src/join_microbench.rs:47-56` | 20/20 (100.0%) | 20/20 (100.0%) |
| rayon | IN | `src/iter/update.rs:18-170` + `src/iter/inspect.rs:19-155` | 290/290 (100.0%) | 290/290 (100.0%) |
| rayon | IN | `tests/chars.rs:4-19` + `tests/chars.rs:22-38` | 33/33 (100.0%) | 33/33 (100.0%) |
| rayon | IN | `rayon-demo/src/str_split.rs:24-36` + `rayon-demo/src/str_split.rs:42-54` | 26/26 (100.0%) | 26/26 (100.0%) |
| rayon | IN | `src/iter/flatten.rs:48-133` + `src/iter/flat_map.rs:60-152` | 179/179 (100.0%) | 179/179 (100.0%) |
| rayon | IN | `src/iter/test.rs:1502-1515` + `src/iter/test.rs:1487-1500` | 28/28 (100.0%) | 28/28 (100.0%) |
| rayon | IN | `src/iter/fold.rs:5-59` + `src/iter/try_fold.rs:9-67` | 114/114 (100.0%) | 114/114 (100.0%) |
| rayon | IN | `src/iter/len.rs:137-220` + `src/iter/len.rs:7-90` | 154/168 (91.7%) | 154/168 (91.7%) |
| rayon | IN | `src/iter/test.rs:499-525` + `src/iter/test.rs:529-555` | 33/54 (61.1%) | 33/54 (61.1%) |
| rayon | IN | `src/iter/fold_chunks.rs:14-122` + `src/iter/fold_chunks_with.rs:14-121` | 217/217 (100.0%) | 217/217 (100.0%) |
| rayon | IN | `src/iter/fold_chunks.rs:157-194` + `src/iter/fold_chunks_with.rs:153-192` | 78/78 (100.0%) | 78/78 (100.0%) |
| rayon | IN | `rayon-demo/src/find/mod.rs:83-101` + `rayon-demo/src/find/mod.rs:130-148` | 38/38 (100.0%) | 38/38 (100.0%) |
| rayon | IN | `src/split_producer.rs:31-42` + `src/split_producer.rs:44-55` | 24/24 (100.0%) | 24/24 (100.0%) |
| rayon | IN | `tests/clones.rs:192-223` + `tests/debug.rs:202-234` | 65/65 (100.0%) | 65/65 (100.0%) |
| rayon | IN | `src/iter/skip_any_while.rs:17-127` + `src/iter/take_any_while.rs:17-127` | 222/222 (100.0%) | 222/222 (100.0%) |
| rayon | IN | `src/slice/rchunks.rs:74-87` + `src/slice/rchunks.rs:254-267` | 28/28 (100.0%) | 28/28 (100.0%) |
| rayon | IN | `src/iter/skip_any_while.rs:55-110` + `src/iter/take_any_while.rs:55-116` | 118/118 (100.0%) | 118/118 (100.0%) |
| rayon | IN | `src/iter/collect/test.rs:156-161` + `src/iter/collect/test.rs:192-198` | 13/13 (100.0%) | 13/13 (100.0%) |
| rayon | IN | `rayon-demo/src/sort.rs:139-149` + `rayon-demo/src/sort.rs:175-210` | 47/47 (100.0%) | 47/47 (100.0%) |
| rayon | IN | `tests/clones.rs:56-64` + `tests/debug.rs:29-37` | 18/18 (100.0%) | 18/18 (100.0%) |
| rayon | IN | `rayon-demo/src/find/mod.rs:131-150` + `rayon-demo/src/find/mod.rs:84-103` | 40/40 (100.0%) | 40/40 (100.0%) |
| rayon | IN | `src/iter/inspect.rs:14-102` + `src/iter/map_with.rs:340-438` | 188/188 (100.0%) | 188/188 (100.0%) |
| rayon | IN | `src/iter/update.rs:11-49` + `src/iter/map.rs:11-50` | 79/79 (100.0%) | 79/79 (100.0%) |
| rayon | IN | `src/iter/fold.rs:68-82` + `src/iter/try_fold.rs:76-90` | 30/30 (100.0%) | 30/30 (100.0%) |
| rayon | IN | `src/iter/test.rs:1333-1342` + `src/iter/test.rs:1316-1325` | 20/20 (100.0%) | 20/20 (100.0%) |
| rayon | IN | `src/iter/while_some.rs:60-70` + `src/iter/take_any.rs:57-67` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `src/iter/collect/test.rs:188-205` + `src/iter/collect/test.rs:207-224` | 36/36 (100.0%) | 36/36 (100.0%) |
| rayon | IN | `rayon-demo/src/nbody/nbody.rs:333-348` + `rayon-demo/src/nbody/nbody.rs:194-209` | 32/32 (100.0%) | 32/32 (100.0%) |
| rayon | IN | `rayon-demo/src/nbody/nbody.rs:440-471` + `rayon-demo/src/nbody/nbody.rs:297-328` | 64/64 (100.0%) | 64/64 (100.0%) |
| rayon | IN | `src/collections/vec_deque.rs:100-149` + `src/vec.rs:96-159` | 114/114 (100.0%) | 114/114 (100.0%) |
| rayon | IN | `src/iter/empty.rs:50-63` + `src/iter/once.rs:35-48` | 28/28 (100.0%) | 28/28 (100.0%) |
| rayon | IN | `src/iter/skip.rs:1-92` + `src/iter/take.rs:1-83` | 175/175 (100.0%) | 175/175 (100.0%) |
| rayon | IN | `src/slice/chunks.rs:192-245` + `src/slice/rchunks.rs:191-244` | 108/108 (100.0%) | 108/108 (100.0%) |
| rayon | IN | `rayon-core/src/scope/test.rs:302-342` + `rayon-core/src/thread_pool/test.rs:231-262` | 73/73 (100.0%) | 73/73 (100.0%) |
| rayon | IN | `src/collections/btree_set.rs:14-45` + `src/collections/linked_list.rs:14-50` | 69/69 (100.0%) | 69/69 (100.0%) |
| rayon | IN | `rayon-core/src/broadcast/test.rs:163-168` + `rayon-core/src/broadcast/test.rs:201-206` | — | — |
| rayon | IN | `src/iter/flat_map.rs:1-152` + `src/iter/positions.rs:1-132` | 284/284 (100.0%) | 284/284 (100.0%) |
| rayon | IN | `rayon-demo/src/sort.rs:132-148` + `rayon-demo/src/sort.rs:163-204` | 59/59 (100.0%) | 59/59 (100.0%) |
| rayon | IN | `src/range_inclusive.rs:45-148` + `src/range.rs:50-140` | 195/195 (100.0%) | 195/195 (100.0%) |
| rayon | IN | `src/iter/flatten.rs:8-90` + `src/iter/flatten_iter.rs:9-93` | 168/168 (100.0%) | 168/168 (100.0%) |
| rayon | IN | `src/iter/chunks.rs:40-87` + `src/iter/step_by.rs:40-87` | 96/96 (100.0%) | 96/96 (100.0%) |
| rayon | IN | `rayon-demo/src/tree.rs:47-62` + `rayon-demo/src/tree.rs:64-79` | 32/32 (100.0%) | 32/32 (100.0%) |
| rayon | IN | `rayon-core/src/scope/mod.rs:492-506` + `rayon-core/src/scope/mod.rs:434-448` | 30/30 (100.0%) | 30/30 (100.0%) |
| rayon | IN | `rayon-demo/src/find/mod.rs:84-104` + `rayon-demo/src/find/mod.rs:131-151` | 42/42 (100.0%) | 42/42 (100.0%) |
| rayon | IN | `src/iter/while_some.rs:46-99` + `src/iter/skip_any.rs:43-96` | 108/108 (100.0%) | 108/108 (100.0%) |
| rayon | IN | `src/iter/map_with.rs:442-500` + `src/iter/map_with.rs:117-175` | 118/118 (100.0%) | 118/118 (100.0%) |
| rayon | IN | `src/iter/map_with.rs:12-54` + `src/iter/fold_chunks.rs:14-61` | 91/91 (100.0%) | 91/91 (100.0%) |
| rayon | IN | `tests/octillion.rs:101-115` + `tests/octillion.rs:70-87` | 33/33 (100.0%) | 33/33 (100.0%) |
| rayon | IN | `src/str.rs:565-583` + `src/str.rs:609-627` | 38/38 (100.0%) | 38/38 (100.0%) |
| rayon | IN | `src/collections/vec_deque.rs:86-149` + `src/collections/binary_heap.rs:60-120` | 125/125 (100.0%) | 125/125 (100.0%) |
| rayon | IN | `src/slice/rchunks.rs:91-188` + `src/slice/chunks.rs:91-189` | 197/197 (100.0%) | 197/197 (100.0%) |
| rayon | IN | `src/iter/test.rs:450-467` + `src/iter/test.rs:432-449` | 36/36 (100.0%) | 36/36 (100.0%) |
| rayon | IN | `src/collections/vec_deque.rs:43-89` + `src/option.rs:83-137` | 102/102 (100.0%) | 102/102 (100.0%) |
| rayon | IN | `src/iter/map_with.rs:256-266` + `src/iter/fold.rs:258-268` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `tests/octillion.rs:141-151` + `tests/octillion.rs:130-140` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `src/iter/update.rs:31-200` + `src/iter/inspect.rs:52-199` | 318/318 (100.0%) | 318/318 (100.0%) |
| rayon | IN | `src/iter/map_with.rs:458-468` + `src/iter/map_with.rs:537-547` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `src/iter/update.rs:105-219` + `src/iter/inspect.rs:111-213` | 218/218 (100.0%) | 218/218 (100.0%) |
| rayon | IN | `src/iter/flat_map.rs:10-20` + `src/iter/filter_map.rs:10-22` | 24/24 (100.0%) | 24/24 (100.0%) |
| rayon | IN | `src/slice/chunks.rs:139-160` + `src/slice/chunks.rs:337-358` | 44/44 (100.0%) | 44/44 (100.0%) |
| rayon | IN | `src/iter/sum.rs:1-110` + `src/iter/product.rs:1-114` | 224/224 (100.0%) | 224/224 (100.0%) |
| rayon | IN | `src/iter/test.rs:1133-1156` + `src/iter/test.rs:1157-1180` | 48/48 (100.0%) | 48/48 (100.0%) |
| rayon | IN | `src/iter/while_some.rs:56-77` + `src/iter/skip_any.rs:53-74` | 44/44 (100.0%) | 44/44 (100.0%) |
| rayon | IN | `src/range.rs:84-195` + `src/range_inclusive.rs:92-205` | 226/226 (100.0%) | 226/226 (100.0%) |
| rayon | IN | `src/iter/cloned.rs:133-170` + `src/iter/copied.rs:133-170` | 76/76 (100.0%) | 76/76 (100.0%) |
| rayon | IN | `src/collections/vec_deque.rs:32-66` + `src/collections/binary_heap.rs:37-63` | 62/62 (100.0%) | 62/62 (100.0%) |
| rayon | IN | `src/iter/inspect.rs:160-218` + `src/iter/map.rs:162-222` | 120/120 (100.0%) | 120/120 (100.0%) |
| rayon | IN | `src/iter/collect/test.rs:49-59` + `src/iter/collect/test.rs:65-75` | 22/22 (100.0%) | 22/22 (100.0%) |
| rayon | IN | `src/iter/test.rs:1142-1164` + `src/iter/test.rs:1166-1188` | 46/46 (100.0%) | 46/46 (100.0%) |
| rayon | IN | `src/iter/map_with.rs:117-175` + `src/iter/panic_fuse.rs:121-171` | 110/110 (100.0%) | 110/110 (100.0%) |
| rayon | IN | `src/iter/cloned.rs:133-188` + `src/iter/flatten_iter.rs:38-93` | 112/112 (100.0%) | 112/112 (100.0%) |
| react | IN | `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/repro-dont-memoize-array-with-mutable-map-after-hook.js:1-26` + `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/repro-dont-memoize-array-with-capturing-map-after-hook.js:1-26` | 52/52 (100.0%) | 52/52 (100.0%) |
| react | IN | `packages/react-server-dom-webpack/src/__tests__/ReactFlightDOM-test.js:1268-1313` + `packages/react-server-dom-webpack/src/__tests__/ReactFlightDOM-test.js:1315-1372` | 104/104 (100.0%) | 104/104 (100.0%) |
| react | IN | `packages/react-dom/src/__tests__/ReactDOMServerIntegrationElements-test.js:17-36` + `packages/react-dom/src/__tests__/ReactDOMServerIntegrationAttributes-test.js:15-35` | 10/41 (24.4%) | 10/41 (24.4%) |
| react | IN | `compiler/packages/babel-plugin-react-compiler/src/HIR/HIR.ts:1554-1589` + `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingEffects.ts:2791-2823` | 69/69 (100.0%) | 69/69 (100.0%) |
| react | IN | `packages/react-reconciler/src/ReactFiberPerformanceTrack.js:421-486` + `packages/react-reconciler/src/ReactFiberPerformanceTrack.js:338-419` | 148/148 (100.0%) | 148/148 (100.0%) |
| react | OUT | `fixtures/attribute-behavior/src/attributes.js:1941-1952` + `fixtures/attribute-behavior/src/attributes.js:1019-1030` | — | — |
| react | OUT | `compiler/apps/playground/colors.js:15-22` + `packages/react-dom-bindings/src/shared/possibleStandardNames.js:439-446` | — | — |
| react | OUT | `packages/react-dom-bindings/src/shared/possibleStandardNames.js:236-243` + `packages/react-dom-bindings/src/shared/possibleStandardNames.js:458-465` | — | — |
| ripgrep | IN | `crates/searcher/src/searcher/core.rs:561-580` + `crates/searcher/src/searcher/core.rs:592-611` | 40/40 (100.0%) | 40/40 (100.0%) |
| ripgrep | IN | `tests/json.rs:210-229` + `tests/json.rs:159-178` | 38/40 (95.0%) | 38/40 (95.0%) |
| ripgrep | IN | `crates/printer/src/json.rs:1033-1039` + `crates/printer/src/json.rs:1049-1055` | 14/14 (100.0%) | 14/14 (100.0%) |
| ripgrep | IN | `crates/printer/src/summary.rs:1054-1064` + `crates/printer/src/summary.rs:921-931` | 22/22 (100.0%) | 22/22 (100.0%) |
| ripgrep | IN | `crates/globset/src/lib.rs:1043-1049` + `crates/globset/src/lib.rs:1035-1041` | 14/14 (100.0%) | 14/14 (100.0%) |
| ripgrep | OUT | `crates/ignore/src/default_types.rs:112-116` + `crates/ignore/src/default_types.rs:121-125` | — | — |
| serde | IN | `test_suite/tests/test_de.rs:1630-1647` + `test_suite/tests/test_ser.rs:316-333` | 36/36 (100.0%) | 36/36 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2632-2643` + `test_suite/tests/regression/issue2565.rs:37-46` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `serde/src/private/ser.rs:949-959` + `serde/src/private/ser.rs:979-989` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2918-2972` + `test_suite/tests/test_annotations.rs:2974-3072` | 154/154 (100.0%) | 154/154 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2227-2259` + `serde_core/src/de/impls.rs:2558-2589` | 65/65 (100.0%) | 65/65 (100.0%) |
| serde | IN | `serde/src/private/de.rs:1180-1234` + `serde/src/private/de.rs:2136-2190` | 110/110 (100.0%) | 110/110 (100.0%) |
| serde | IN | `serde_derive/src/ser.rs:502-573` + `serde_derive/src/ser.rs:575-639` | 137/137 (100.0%) | 137/137 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:86-97` + `test_suite/tests/test_enum_adjacently_tagged.rs:103-114` | 24/24 (100.0%) | 24/24 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:967-1023` + `test_suite/tests/test_de_error.rs:1024-1080` | 114/114 (100.0%) | 114/114 (100.0%) |
| serde | IN | `test_suite/tests/test_de.rs:1819-1843` + `test_suite/tests/test_de.rs:1792-1816` | 22/50 (44.0%) | 22/50 (44.0%) |
| serde | IN | `serde_derive/src/internals/attr.rs:1609-1631` + `serde_derive/src/internals/attr.rs:1633-1655` | 46/46 (100.0%) | 46/46 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:318-330` + `test_suite/tests/test_annotations.rs:168-180` | 26/26 (100.0%) | 26/26 (100.0%) |
| serde | IN | `test_suite/tests/test_identifier.rs:13-72` + `test_suite/tests/test_identifier.rs:80-139` | 120/120 (100.0%) | 120/120 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:128-138` + `test_suite/tests/test_annotations.rs:2506-2516` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:677-709` + `serde_core/src/de/value.rs:474-505` | 65/65 (100.0%) | 65/65 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2012-2049` + `test_suite/tests/test_annotations.rs:1974-2010` | 75/75 (100.0%) | 75/75 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2089-2096` + `serde/src/private/de.rs:1135-1142` | 16/16 (100.0%) | 16/16 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:584-600` + `test_suite/tests/test_enum_internally_tagged.rs:567-583` | 34/34 (100.0%) | 34/34 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_untagged.rs:370-410` + `test_suite/tests/test_enum_internally_tagged.rs:1164-1211` | 89/89 (100.0%) | 89/89 (100.0%) |
| serde | IN | `serde_core/src/ser/impls.rs:404-420` + `serde_core/src/de/impls.rs:1481-1497` | 34/34 (100.0%) | 34/34 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3488-3497` + `test_suite/tests/test_annotations.rs:3424-3433` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2741-2869` + `serde_core/src/de/impls.rs:2602-2730` | 258/258 (100.0%) | 258/258 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:1345-1360` + `test_suite/tests/test_de_error.rs:1329-1344` | 32/32 (100.0%) | 32/32 (100.0%) |
| serde | IN | `serde_derive/src/ser.rs:26-58` + `serde_derive/src/de.rs:39-76` | 71/71 (100.0%) | 71/71 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3134-3236` + `test_suite/tests/test_enum_adjacently_tagged.rs:523-570` | 151/151 (100.0%) | 151/151 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2793-2808` + `test_suite/tests/test_annotations.rs:2747-2762` | 32/32 (100.0%) | 32/32 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2494-2503` + `serde_core/src/de/impls.rs:2296-2305` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:762-775` + `test_suite/tests/test_roundtrip.rs:9-22` | 28/28 (100.0%) | 28/28 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:647-680` + `test_suite/tests/test_annotations.rs:2793-2824` | 66/66 (100.0%) | 66/66 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3139-3154` + `test_suite/tests/test_annotations.rs:3116-3129` | 30/30 (100.0%) | 30/30 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:600-643` + `test_suite/tests/test_enum_internally_tagged.rs:1395-1429` | 79/79 (100.0%) | 79/79 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2994-3072` + `test_suite/tests/test_annotations.rs:2954-2992` | 118/118 (100.0%) | 118/118 (100.0%) |
| serde | IN | `test_suite/tests/ui/with-variant/skip_de_struct_field.rs:1-13` + `test_suite/tests/ui/with-variant/skip_ser_struct_field.rs:1-13` | 22/26 (84.6%) | 22/26 (84.6%) |
| serde | IN | `serde_derive/src/lib.rs:121-127` + `serde_derive/src/lib.rs:113-119` | 14/14 (100.0%) | 14/14 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2129-2183` + `serde/src/private/de.rs:1173-1227` | 110/110 (100.0%) | 110/110 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:1254-1264` + `test_suite/tests/test_enum_internally_tagged.rs:1182-1192` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `test_suite/tests/test_de.rs:952-964` + `test_suite/tests/test_de.rs:965-977` | 26/26 (100.0%) | 26/26 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2842-2853` + `test_suite/tests/test_annotations.rs:3336-3347` | 24/24 (100.0%) | 24/24 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:67-75` + `test_suite/tests/test_enum_adjacently_tagged.rs:773-781` | 18/18 (100.0%) | 18/18 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:432-442` + `test_suite/tests/test_de.rs:1822-1832` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2718-2771` + `serde_core/src/de/value.rs:1475-1528` | 108/108 (100.0%) | 108/108 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:373-440` + `serde_core/src/de/value.rs:677-747` | 139/139 (100.0%) | 139/139 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2187-2262` + `serde_core/src/de/impls.rs:2326-2405` | 156/156 (100.0%) | 156/156 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:973-1000` + `test_suite/tests/test_enum_internally_tagged.rs:1001-1036` | 64/64 (100.0%) | 64/64 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:960-972` + `test_suite/tests/test_enum_internally_tagged.rs:973-985` | 26/26 (100.0%) | 26/26 (100.0%) |
| serde | IN | `serde_core/src/ser/impls.rs:885-902` + `serde_core/src/ser/impls.rs:869-883` | 33/33 (100.0%) | 33/33 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:227-262` + `test_suite/tests/test_enum_adjacently_tagged.rs:121-155` | 71/71 (100.0%) | 71/71 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3293-3301` + `test_suite/tests/test_enum_adjacently_tagged.rs:561-568` | 17/17 (100.0%) | 17/17 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:1261-1277` + `serde_core/src/de/value.rs:1027-1043` | 34/34 (100.0%) | 34/34 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:601-619` + `test_suite/tests/test_enum_untagged.rs:512-530` | 38/38 (100.0%) | 38/38 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2793-2801` + `serde/src/private/de.rs:1843-1851` | 18/18 (100.0%) | 18/18 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2809-2824` + `test_suite/tests/test_annotations.rs:2763-2778` | 32/32 (100.0%) | 32/32 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:550-569` + `test_suite/tests/test_macros.rs:193-208` | 36/36 (100.0%) | 36/36 (100.0%) |
| serde | IN | `serde/src/private/de.rs:1772-1865` + `serde_core/src/de/value.rs:1475-1577` | 197/197 (100.0%) | 197/197 (100.0%) |
| serde | IN | `serde_derive/src/de/enum_untagged.rs:62-109` + `serde_derive/src/de/enum_internally.rs:67-106` | 88/88 (100.0%) | 88/88 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:959-997` + `serde_core/src/de/value.rs:900-938` | 78/78 (100.0%) | 78/78 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:773-782` + `test_suite/tests/test_enum_adjacently_tagged.rs:189-198` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:38-68` + `test_suite/tests/test_de.rs:107-138` | 63/63 (100.0%) | 63/63 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2610-2645` + `test_suite/tests/test_enum_internally_tagged.rs:1395-1429` | 71/71 (100.0%) | 71/71 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_untagged.rs:412-425` + `test_suite/tests/test_enum_untagged.rs:397-410` | 28/28 (100.0%) | 28/28 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:743-773` + `test_suite/tests/test_de_error.rs:1025-1055` | 62/62 (100.0%) | 62/62 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2138-2185` + `serde_core/src/de/impls.rs:2475-2522` | 96/96 (100.0%) | 96/96 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:1353-1437` + `serde/src/private/de.rs:1690-1767` | 163/163 (100.0%) | 163/163 (100.0%) |
| serde | IN | `serde_core/src/lib.rs:46-90` + `serde/src/lib.rs:111-155` | 90/90 (100.0%) | 90/90 (100.0%) |
| serde | IN | `serde/src/private/de.rs:1618-1635` + `serde/src/private/de.rs:1520-1537` | 36/36 (100.0%) | 36/36 (100.0%) |
| serde | IN | `serde_derive/src/internals/attr.rs:383-390` + `serde_derive/src/internals/attr.rs:405-412` | 16/16 (100.0%) | 16/16 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3254-3263` + `test_suite/tests/test_annotations.rs:2959-2968` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2610-2645` + `test_suite/tests/test_enum_adjacently_tagged.rs:600-643` | 80/80 (100.0%) | 80/80 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:1385-1400` + `test_suite/tests/test_de_error.rs:1401-1416` | 32/32 (100.0%) | 32/32 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:748-805` + `test_suite/tests/test_enum_internally_tagged.rs:335-392` | 116/116 (100.0%) | 116/116 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:664-702` + `test_suite/tests/test_de_error.rs:703-741` | 78/78 (100.0%) | 78/78 (100.0%) |
| serde | IN | `test_suite/tests/test_de_error.rs:966-1021` + `test_suite/tests/test_de_error.rs:1080-1135` | 112/112 (100.0%) | 112/112 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:2921-2930` + `test_suite/tests/test_annotations.rs:3085-3094` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `serde/src/private/de.rs:1520-1562` + `serde_core/src/de/value.rs:1023-1067` | 88/88 (100.0%) | 88/88 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:227-275` + `test_suite/tests/test_annotations.rs:118-164` | 96/96 (100.0%) | 96/96 (100.0%) |
| serde | IN | `test_suite/tests/test_de.rs:1435-1448` + `test_suite/tests/test_de.rs:1406-1419` | 28/28 (100.0%) | 28/28 (100.0%) |
| serde | IN | `serde_core/src/ser/impls.rs:1009-1045` + `serde_core/src/de/impls.rs:3102-3133` | 69/69 (100.0%) | 69/69 (100.0%) |
| serde | IN | `serde_core/src/ser/mod.rs:1675-1738` + `serde_core/src/ser/mod.rs:1518-1623` | 170/170 (100.0%) | 170/170 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:311-353` + `test_suite/tests/test_de.rs:1619-1666` | 91/91 (100.0%) | 91/91 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2878-2899` + `serde/src/private/de.rs:2901-2926` | 48/48 (100.0%) | 48/48 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:824-831` + `test_suite/tests/test_de.rs:1518-1525` | 16/16 (100.0%) | 16/16 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:1142-1158` + `test_suite/tests/test_enum_internally_tagged.rs:1124-1140` | 34/34 (100.0%) | 34/34 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:187-195` + `test_suite/tests/test_enum_internally_tagged.rs:138-146` | 18/18 (100.0%) | 18/18 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3076-3237` + `test_suite/tests/test_annotations.rs:3240-3416` | 339/339 (100.0%) | 339/339 (100.0%) |
| serde | IN | `serde_core/src/ser/mod.rs:1901-1930` + `serde_core/src/ser/mod.rs:1971-2000` | 60/60 (100.0%) | 60/60 (100.0%) |
| serde | IN | `serde/src/private/ser.rs:876-883` + `serde/src/private/ser.rs:991-998` | 16/16 (100.0%) | 16/16 (100.0%) |
| serde | IN | `serde/src/private/de.rs:2478-2495` + `serde/src/private/de.rs:2564-2581` | 36/36 (100.0%) | 36/36 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:1418-1429` + `test_suite/tests/test_annotations.rs:1431-1442` | 24/24 (100.0%) | 24/24 (100.0%) |
| serde | IN | `serde_core/src/de/value.rs:473-540` + `serde_core/src/de/value.rs:563-630` | 136/136 (100.0%) | 136/136 (100.0%) |
| serde | IN | `serde_core/src/de/impls.rs:2469-2522` + `serde_core/src/de/impls.rs:2767-2817` | 105/105 (100.0%) | 105/105 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:25-59` + `test_suite/tests/test_enum_adjacently_tagged.rs:82-116` | 70/70 (100.0%) | 70/70 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:3189-3198` + `test_suite/tests/test_annotations.rs:3037-3046` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:117-159` + `test_suite/tests/test_enum_internally_tagged.rs:166-208` | 86/86 (100.0%) | 86/86 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:1730-1739` + `test_suite/tests/test_annotations.rs:676-685` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `serde_derive/src/de/unit.rs:19-51` + `serde_derive/src/de/enum_externally.rs:79-115` | 70/70 (100.0%) | 70/70 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:762-791` + `test_suite/tests/test_enum_internally_tagged.rs:349-378` | 60/60 (100.0%) | 60/60 (100.0%) |
| serde | IN | `test_suite/tests/regression/issue2371.rs:8-17` + `test_suite/tests/regression/issue2371.rs:44-53` | 20/20 (100.0%) | 20/20 (100.0%) |
| serde | IN | `test_suite/tests/ui/conflict/alias-enum.rs:7-17` + `test_suite/tests/ui/conflict/alias.rs:4-14` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_adjacently_tagged.rs:561-568` + `test_suite/tests/test_enum_adjacently_tagged.rs:480-487` | 16/16 (100.0%) | 16/16 (100.0%) |
| serde | IN | `test_suite/tests/test_annotations.rs:1277-1324` + `test_suite/tests/test_annotations.rs:1227-1274` | 96/96 (100.0%) | 96/96 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:896-906` + `test_suite/tests/test_ser.rs:884-894` | 22/22 (100.0%) | 22/22 (100.0%) |
| serde | IN | `test_suite/tests/test_ser.rs:714-720` + `test_suite/tests/test_ser.rs:721-727` | 14/14 (100.0%) | 14/14 (100.0%) |
| serde | IN | `serde/src/private/de.rs:1166-1220` + `serde/src/private/de.rs:2122-2176` | 110/110 (100.0%) | 110/110 (100.0%) |
| serde | IN | `serde_derive/src/de/struct_.rs:588-607` + `serde_derive/src/de/struct_.rs:293-311` | 39/39 (100.0%) | 39/39 (100.0%) |
| serde | IN | `test_suite/tests/test_de.rs:988-1000` + `test_suite/tests/test_de.rs:1093-1105` | 26/26 (100.0%) | 26/26 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_untagged.rs:513-525` + `test_suite/tests/test_annotations.rs:2612-2623` | 25/25 (100.0%) | 25/25 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:63-74` + `test_suite/tests/test_enum_internally_tagged.rs:51-62` | 24/24 (100.0%) | 24/24 (100.0%) |
| serde | IN | `test_suite/tests/test_macros.rs:559-589` + `test_suite/tests/test_enum_untagged.rs:28-52` | 56/56 (100.0%) | 56/56 (100.0%) |
| serde | IN | `test_suite/tests/test_enum_internally_tagged.rs:808-853` + `test_suite/tests/test_enum_internally_tagged.rs:475-520` | 92/92 (100.0%) | 92/92 (100.0%) |
| serilog | IN | `src/Serilog/Formatting/Json/JsonValueFormatter.cs:314-331` + `src/Serilog/Formatting/Json/JsonValueFormatter.cs:333-350` | 36/36 (100.0%) | 36/36 (100.0%) |
| serilog | IN | `src/Serilog/Configuration/LoggerFilterConfiguration.cs:22-62` + `src/Serilog/Configuration/LoggerEnrichmentConfiguration.cs:22-66` | 86/86 (100.0%) | 86/86 (100.0%) |
| serilog | IN | `test/Serilog.PerformanceTests/Support/DictionaryMessageTemplateCache.cs:4-50` + `test/Serilog.PerformanceTests/Support/ConcurrentDictionaryTemplateCache.cs:4-46` | 90/90 (100.0%) | 90/90 (100.0%) |
| serilog | IN | `test/Serilog.PerformanceTests/Support/DictionaryMessageTemplateCache.cs:4-50` + `src/Serilog/Core/Pipeline/MessageTemplateCache.cs:18-73` | 103/103 (100.0%) | 103/103 (100.0%) |
| serilog | IN | `src/Serilog/Core/Sinks/AggregateSink.cs:19-47` + `src/Serilog/Core/Sinks/DisposingAggregateSink.cs:22-49` | 57/57 (100.0%) | 57/57 (100.0%) |
| serilog | IN | `src/Serilog/Formatting/Json/JsonValueFormatter.cs:469-484` + `src/Serilog/Formatting/Json/JsonValueFormatter.cs:486-501` | 32/32 (100.0%) | 32/32 (100.0%) |
| serilog | IN | `src/Serilog/Policies/ProjectedDestructuringPolicy.cs:17-42` + `test/Serilog.Tests/Configuration/LoggerConfigurationTests.cs:136-161` | 52/52 (100.0%) | 52/52 (100.0%) |
| tokio | IN | `tokio/tests/dump.rs:35-62` + `tokio/tests/dump.rs:73-100` | 56/56 (100.0%) | 56/56 (100.0%) |
| tokio | IN | `tokio/tests/tcp_stream.rs:38-53` + `tokio/tests/tcp_stream.rs:297-312` | 32/32 (100.0%) | 32/32 (100.0%) |
| tokio | IN | `tokio/src/runtime/time_alt/wheel/level.rs:152-194` + `tokio/src/runtime/time/wheel/level.rs:150-192` | 86/86 (100.0%) | 86/86 (100.0%) |
| tokio | IN | `tokio/src/runtime/time/wheel/mod.rs:292-331` + `tokio-util/src/time/wheel/mod.rs:278-317` | 80/80 (100.0%) | 80/80 (100.0%) |
| tokio | IN | `tokio/tests/tcp_stream.rs:181-213` + `tokio/tests/uds_stream.rs:209-241` | 60/66 (90.9%) | 60/66 (90.9%) |
| tokio | IN | `tokio/src/net/tcp/socket.rs:655-675` + `tokio/src/net/tcp/socket.rs:595-615` | 42/42 (100.0%) | 42/42 (100.0%) |
| tokio | IN | `examples/connect-udp.rs:28-44` + `examples/connect-tcp.rs:28-44` | 34/34 (100.0%) | 34/34 (100.0%) |
| tokio | IN | `tokio/src/fs/file/tests.rs:617-635` + `tokio/src/fs/file/tests.rs:651-669` | 38/38 (100.0%) | 38/38 (100.0%) |
| tokio | IN | `tokio/src/runtime/io/scheduled_io.rs:39-100` + `tokio/src/runtime/task/core.rs:63-124` | 124/124 (100.0%) | 124/124 (100.0%) |
| viper | IN | `viper_test.go:176-187` + `viper_test.go:268-279` | 24/24 (100.0%) | 24/24 (100.0%) |
| viper | IN | `viper_test.go:1905-1915` + `viper_test.go:1955-1965` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper.go:964-974` + `viper.go:1069-1079` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper_test.go:1297-1306` + `flags_test.go:47-57` | 21/21 (100.0%) | 21/21 (100.0%) |
| viper | IN | `viper_test.go:1901-1906` + `viper_test.go:1951-1956` | 10/12 (83.3%) | 10/12 (83.3%) |
| viper | IN | `viper_test.go:218-227` + `viper_test.go:250-259` | 20/20 (100.0%) | 20/20 (100.0%) |
| viper | IN | `viper.go:1224-1253` + `viper.go:1326-1355` | 60/60 (100.0%) | 60/60 (100.0%) |
| viper | IN | `viper_test.go:1914-1922` + `viper_test.go:1964-1972` | 18/18 (100.0%) | 18/18 (100.0%) |
| viper | IN | `flags_test.go:46-65` + `viper_test.go:1295-1314` | 40/40 (100.0%) | 40/40 (100.0%) |
| viper | IN | `viper_test.go:1962-1970` + `viper_test.go:1912-1920` | 18/18 (100.0%) | 18/18 (100.0%) |
| viper | IN | `viper_test.go:684-694` + `viper_test.go:616-626` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper_test.go:869-880` + `viper_test.go:883-894` | 24/24 (100.0%) | 24/24 (100.0%) |
| viper | IN | `viper_test.go:218-226` + `viper_test.go:292-300` | 18/18 (100.0%) | 18/18 (100.0%) |
| viper | IN | `viper_test.go:175-193` + `viper_test.go:195-212` | 37/37 (100.0%) | 37/37 (100.0%) |
| viper | IN | `viper_test.go:1551-1558` + `viper_test.go:490-497` | 16/16 (100.0%) | 16/16 (100.0%) |
| viper | IN | `viper_test.go:1184-1210` + `viper_test.go:1140-1166` | 52/54 (96.3%) | 52/54 (96.3%) |
| viper | IN | `viper_test.go:1065-1089` + `viper_test.go:1029-1053` | 50/50 (100.0%) | 50/50 (100.0%) |
| viper | IN | `viper_test.go:1006-1026` + `viper_test.go:1069-1088` | 41/41 (100.0%) | 41/41 (100.0%) |
| viper | IN | `viper.go:1557-1574` + `viper.go:1535-1551` | 35/35 (100.0%) | 35/35 (100.0%) |
| viper | IN | `viper_test.go:1690-1712` + `viper_test.go:1667-1687` | 44/44 (100.0%) | 44/44 (100.0%) |
| viper | IN | `viper_test.go:1954-1964` + `viper_test.go:1904-1914` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `remote.go:134-166` + `remote.go:93-122` | 63/63 (100.0%) | 63/63 (100.0%) |
| viper | IN | `viper_test.go:446-460` + `viper_test.go:338-350` | 28/28 (100.0%) | 28/28 (100.0%) |
| viper | IN | `viper.go:539-553` + `viper.go:572-587` | — | — |
| viper | IN | `viper_test.go:1953-1963` + `viper_test.go:1903-1913` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `internal/encoding/dotenv/map_utils.go:12-41` + `viper.go:1996-2025` | 60/60 (100.0%) | 60/60 (100.0%) |
| viper | IN | `viper_test.go:309-327` + `viper_test.go:214-232` | 14/38 (36.8%) | 14/38 (36.8%) |
| viper | IN | `viper_test.go:1339-1362` + `viper_test.go:1382-1407` | 50/50 (100.0%) | 50/50 (100.0%) |
| viper | IN | `encoding_test.go:21-38` + `encoding_test.go:87-104` | 36/36 (100.0%) | 36/36 (100.0%) |
| viper | IN | `remote/remote.go:34-45` + `remote/remote.go:22-32` | 23/23 (100.0%) | 23/23 (100.0%) |
| viper | IN | `viper_test.go:1184-1210` + `viper_test.go:1141-1165` | 52/52 (100.0%) | 52/52 (100.0%) |
| viper | IN | `viper_test.go:1913-1921` + `viper_test.go:1963-1971` | 18/18 (100.0%) | 18/18 (100.0%) |
| viper | IN | `viper_test.go:1906-1916` + `viper_test.go:1956-1966` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper.go:1620-1628` + `viper.go:1583-1591` | 18/18 (100.0%) | 18/18 (100.0%) |
| viper | IN | `flags_test.go:12-43` + `viper_test.go:1092-1123` | 64/64 (100.0%) | 64/64 (100.0%) |
| viper | IN | `viper_test.go:1568-1574` + `viper_test.go:2127-2133` | 14/14 (100.0%) | 14/14 (100.0%) |
| viper | IN | `internal/encoding/dotenv/codec_test.go:24-55` + `internal/encoding/toml/codec_test.go:66-97` | — | — |
| viper | IN | `viper_test.go:1957-1967` + `viper_test.go:1907-1917` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper_test.go:902-913` + `viper_test.go:940-951` | 24/24 (100.0%) | 24/24 (100.0%) |
| viper | IN | `viper_test.go:410-421` + `viper_test.go:385-396` | 24/24 (100.0%) | 24/24 (100.0%) |
| viper | IN | `internal/encoding/yaml/codec_test.go:75-95` + `internal/encoding/yaml/codec_test.go:11-72` | 83/83 (100.0%) | 83/83 (100.0%) |
| viper | IN | `viper_test.go:270-280` + `viper_test.go:291-301` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper_test.go:1342-1362` + `viper_test.go:1386-1406` | 42/42 (100.0%) | 42/42 (100.0%) |
| viper | IN | `viper.go:1940-1952` + `viper.go:1922-1934` | 26/26 (100.0%) | 26/26 (100.0%) |
| viper | IN | `viper_test.go:1902-1912` + `viper_test.go:1952-1962` | 22/22 (100.0%) | 22/22 (100.0%) |
| viper | IN | `viper_test.go:359-382` + `viper_test.go:384-407` | 48/48 (100.0%) | 48/48 (100.0%) |
| viper | IN | `internal/encoding/yaml/codec_test.go:75-128` + `internal/encoding/toml/codec_test.go:44-97` | 108/108 (100.0%) | 108/108 (100.0%) |
| viper | IN | `viper_test.go:1144-1165` + `viper_test.go:1270-1291` | 44/44 (100.0%) | 44/44 (100.0%) |
| viper | IN | `internal/encoding/json/codec_test.go:65-87` + `internal/encoding/toml/codec_test.go:75-97` | 46/46 (100.0%) | 46/46 (100.0%) |
| viper | IN | `viper_test.go:409-432` + `viper_test.go:384-407` | 48/48 (100.0%) | 48/48 (100.0%) |
| vite | IN | `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:43-80` + `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:43-80` | 76/76 (100.0%) | 76/76 (100.0%) |
| vite | IN | `packages/vite/src/node/utils.ts:1412-1420` + `packages/vite/src/node/utils.ts:1307-1315` | 18/18 (100.0%) | 18/18 (100.0%) |
| vite | IN | `playground/js-sourcemap/dep-class-field-sourcemap-oxc/index.js:1-10` + `playground/js-sourcemap/dep-class-field-sourcemap-babel/index.js:1-10` | 20/20 (100.0%) | 20/20 (100.0%) |
| vite | IN | `packages/vite/src/node/server/hmr.ts:561-575` + `packages/vite/src/node/server/hmr.ts:577-591` | 30/30 (100.0%) | 30/30 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:733-821` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:625-709` | 174/174 (100.0%) | 174/174 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:823-898` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:711-779` | 145/145 (100.0%) | 145/145 (100.0%) |
| vite | IN | `playground/hmr-ssr/file-delete-restore/runtime.js:1-15` + `playground/hmr/file-delete-restore/runtime.js:1-16` | 31/31 (100.0%) | 31/31 (100.0%) |
| vite | IN | `playground/hmr-full-bundle-mode/__tests__/hmr-full-bundle-mode.spec.ts:289-317` + `playground/hmr-full-bundle-mode/__tests__/hmr-full-bundle-mode.spec.ts:257-285` | 58/58 (100.0%) | 58/58 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/http.spec.ts:107-125` + `packages/vite/src/node/__tests__/http.spec.ts:63-81` | 38/38 (100.0%) | 38/38 (100.0%) |
| vite | IN | `playground/hmr/hmr.ts:138-156` + `playground/hmr-ssr/hmr.ts:85-103` | 38/38 (100.0%) | 38/38 (100.0%) |
| vite | IN | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:119-192` + `playground/hmr/__tests__/hmr.spec.ts:121-202` | 156/156 (100.0%) | 156/156 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a1.js:29-36` + `playground/multiple-entrypoints/entrypoints/a2.js:28-35` | 16/16 (100.0%) | 16/16 (100.0%) |
| vite | IN | `packages/vite/src/node/preview.ts:233-247` + `packages/vite/src/node/server/index.ts:1016-1037` | 37/37 (100.0%) | 37/37 (100.0%) |
| vite | IN | `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:96-115` + `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:100-117` | 38/38 (100.0%) | 38/38 (100.0%) |
| vite | IN | `playground/fs-serve/root/vite.config-base.js:10-45` + `playground/fs-serve/root/vite.config.js:8-41` | 70/70 (100.0%) | 70/70 (100.0%) |
| vite | IN | `packages/create-vite/template-qwik-ts/src/app.tsx:30-99` + `packages/create-vite/template-preact-ts/src/app.tsx:33-102` | 140/140 (100.0%) | 140/140 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/utils.spec.ts:1228-1244` + `packages/vite/src/node/__tests__/utils.spec.ts:1210-1226` | 34/34 (100.0%) | 34/34 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/build.spec.ts:1647-1659` + `packages/vite/src/node/__tests__/build.spec.ts:1676-1688` | 26/26 (100.0%) | 26/26 (100.0%) |
| vite | IN | `packages/plugin-legacy/src/index.ts:1139-1150` + `packages/plugin-legacy/src/index.ts:1126-1137` | 24/24 (100.0%) | 24/24 (100.0%) |
| vite | IN | `playground/worker/__tests__/iife/worker-iife.spec.ts:117-124` + `playground/worker/__tests__/es/worker-es.spec.ts:139-146` | 16/16 (100.0%) | 16/16 (100.0%) |
| vite | IN | `playground/cli/__tests__/serve.ts:21-145` + `playground/cli-module/__tests__/serve.ts:16-156` | 266/266 (100.0%) | 266/266 (100.0%) |
| vite | IN | `packages/vite/src/node/server/middlewares/__tests__/indexHtml.spec.ts:76-94` + `packages/vite/src/node/server/middlewares/__tests__/indexHtml.spec.ts:41-62` | 41/41 (100.0%) | 41/41 (100.0%) |
| vite | IN | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:194-235` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:151-192` | 84/84 (100.0%) | 84/84 (100.0%) |
| vite | IN | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:885-923` + `playground/hmr/__tests__/hmr.spec.ts:1107-1156` | 89/89 (100.0%) | 89/89 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/utils.spec.ts:1230-1243` + `packages/vite/src/node/__tests__/utils.spec.ts:1194-1207` | 28/28 (100.0%) | 28/28 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a2.js:28-38` + `playground/multiple-entrypoints/entrypoints/a1.js:29-39` | 22/22 (100.0%) | 22/22 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/config.spec.ts:427-457` + `packages/vite/src/node/__tests__/config.spec.ts:264-294` | 62/62 (100.0%) | 62/62 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a3.js:27-40` + `playground/multiple-entrypoints/entrypoints/a4.js:26-39` | 28/28 (100.0%) | 28/28 (100.0%) |
| vite | IN | `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:178-205` + `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:191-218` | — | — |
| vite | IN | `playground/worker/__tests__/es/worker-es.spec.ts:6-28` + `playground/worker/__tests__/iife/worker-iife.spec.ts:16-37` | 45/45 (100.0%) | 45/45 (100.0%) |
| vite | IN | `playground/ssr/server.js:1-73` + `playground/optimize-missing-deps/server.js:2-66` | 138/138 (100.0%) | 138/138 (100.0%) |
| vite | IN | `playground/ssr-conditions/server.js:29-43` + `playground/ssr/server.js:22-36` | 30/30 (100.0%) | 30/30 (100.0%) |
| vite | IN | `playground/resolve/__tests__/resolve.spec.ts:250-289` + `playground/resolve/__tests__/resolve.spec.ts:31-71` | 81/81 (100.0%) | 81/81 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:530-658` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:379-506` | 257/257 (100.0%) | 257/257 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:1139-1150` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:825-834` | 22/22 (100.0%) | 22/22 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:182-198` + `playground/hmr/__tests__/hmr.spec.ts:160-176` | 34/34 (100.0%) | 34/34 (100.0%) |
| vite | IN | `packages/vite/src/node/plugins/html.ts:908-918` + `packages/vite/src/node/plugins/html.ts:896-906` | 22/22 (100.0%) | 22/22 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:122-137` + `playground/hmr/__tests__/hmr.spec.ts:139-154` | 32/32 (100.0%) | 32/32 (100.0%) |
| vite | IN | `playground/hmr-ssr/vite.config.ts:87-97` + `playground/hmr/vite.config.ts:150-160` | 22/22 (100.0%) | 22/22 (100.0%) |
| vite | IN | `playground/tailwind/__test__/tailwind.spec.ts:31-106` + `playground/tailwind-v3/__test__/tailwind-v3.spec.ts:15-90` | 152/152 (100.0%) | 152/152 (100.0%) |
| vite | IN | `playground/ssr-noexternal/__tests__/serve.ts:10-52` + `playground/ssr-conditions/__tests__/serve.ts:10-62` | 96/96 (100.0%) | 96/96 (100.0%) |
| vite | IN | `playground/worker/__tests__/iife/worker-iife.spec.ts:85-113` + `playground/worker/__tests__/es/worker-es.spec.ts:100-128` | 58/58 (100.0%) | 58/58 (100.0%) |
| vite | IN | `packages/create-vite/template-solid-ts/src/App.tsx:7-104` + `packages/create-vite/template-preact-ts/src/app.tsx:7-104` | 196/196 (100.0%) | 196/196 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/config.spec.ts:627-704` + `packages/vite/src/node/__tests__/config.spec.ts:706-783` | 156/156 (100.0%) | 156/156 (100.0%) |
| vite | IN | `packages/vite/client.d.ts:158-189` + `packages/vite/client.d.ts:244-277` | 66/66 (100.0%) | 66/66 (100.0%) |
| vite | IN | `playground/css-lightningcss/__tests__/css-lightningcss.spec.ts:34-49` + `playground/css-lightningcss/__tests__/css-lightningcss.spec.ts:17-32` | 32/32 (100.0%) | 32/32 (100.0%) |
| vite | IN | `playground/assets/__tests__/url-base/assets-url-base.spec.ts:1-271` + `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:1-269` | 540/540 (100.0%) | 540/540 (100.0%) |
| vite | IN | `playground/assets/__tests__/runtime-base/assets-runtime-base.spec.ts:156-239` + `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:160-232` | 157/157 (100.0%) | 157/157 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a11.js:19-30` + `playground/multiple-entrypoints/entrypoints/a10.js:20-31` | — | — |
| vite | IN | `playground/worker/vite.config-iife.js:5-35` + `playground/worker/vite.config-relative-base.js:5-33` | 60/60 (100.0%) | 60/60 (100.0%) |
| vite | IN | `packages/vite/src/node/plugins/resolve.ts:316-322` + `packages/vite/src/node/plugins/resolve.ts:855-861` | 14/14 (100.0%) | 14/14 (100.0%) |
| vite | IN | `packages/vite/src/node/server/index.ts:665-677` + `packages/vite/src/node/preview.ts:174-190` | 30/30 (100.0%) | 30/30 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/plugins/esbuild.spec.ts:116-129` + `packages/vite/src/node/__tests__/plugins/esbuild.spec.ts:148-161` | 28/28 (100.0%) | 28/28 (100.0%) |
| vite | IN | `playground/worker/vite.config-es.js:11-48` + `playground/worker/vite.config-iife.js:11-52` | 80/80 (100.0%) | 80/80 (100.0%) |
| vite | IN | `playground/tailwind/vite.config.ts:1-44` + `playground/tailwind-v3/vite.config.ts:1-42` | 86/86 (100.0%) | 86/86 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/config.spec.ts:707-763` + `packages/vite/src/node/__tests__/config.spec.ts:786-842` | 114/114 (100.0%) | 114/114 (100.0%) |
| vite | IN | `packages/vite/src/node/ssr/__tests__/ssrTransform.spec.ts:424-461` + `packages/vite/src/node/ssr/__tests__/ssrTransform.spec.ts:463-498` | 74/74 (100.0%) | 74/74 (100.0%) |
| vite | IN | `packages/vite/src/node/ssr/runtime/__tests__/server-runtime.spec.ts:364-372` + `packages/vite/src/node/ssr/runtime/__tests__/server-runtime.spec.ts:384-392` | 18/18 (100.0%) | 18/18 (100.0%) |
| vite | IN | `packages/vite/src/node/__tests__/plugins/css.spec.ts:385-396` + `packages/vite/src/node/__tests__/plugins/css.spec.ts:374-383` | 22/22 (100.0%) | 22/22 (100.0%) |
| vite | IN | `playground/hmr/__tests__/hmr.spec.ts:284-304` + `playground/hmr/__tests__/hmr.spec.ts:255-276` | 43/43 (100.0%) | 43/43 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a7.js:23-34` + `playground/multiple-entrypoints/entrypoints/a5.js:25-36` | 24/24 (100.0%) | 24/24 (100.0%) |
| vite | IN | `packages/create-vite/template-react-ts/src/App.tsx:7-120` + `packages/create-vite/template-preact-ts/src/app.tsx:7-104` | 212/212 (100.0%) | 212/212 (100.0%) |
| vite | IN | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:514-572` + `playground/hmr/__tests__/hmr.spec.ts:660-731` | 131/131 (100.0%) | 131/131 (100.0%) |
| vite | IN | `playground/assets/__tests__/url-base/assets-url-base.spec.ts:234-271` + `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:232-269` | 76/76 (100.0%) | 76/76 (100.0%) |
| vite | IN | `playground/resolve/__tests__/resolve.spec.ts:237-263` + `playground/resolve/__tests__/resolve.spec.ts:17-44` | 55/55 (100.0%) | 55/55 (100.0%) |
| vite | IN | `playground/worker/__tests__/iife/worker-iife.spec.ts:84-115` + `playground/worker/__tests__/es/worker-es.spec.ts:99-137` | 71/71 (100.0%) | 71/71 (100.0%) |
| vite | IN | `playground/ssr-pug/server.js:18-45` + `playground/ssr/server.js:12-40` | 57/57 (100.0%) | 57/57 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a12.js:18-28` + `playground/multiple-entrypoints/entrypoints/a8.js:22-32` | — | — |
| vite | IN | `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:9-30` + `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:9-30` | 44/44 (100.0%) | 44/44 (100.0%) |
| vite | IN | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:239-256` + `playground/hmr/__tests__/hmr.spec.ts:285-303` | 37/37 (100.0%) | 37/37 (100.0%) |
| vite | IN | `playground/worker/__tests__/iife/worker-iife.spec.ts:82-159` + `playground/worker/__tests__/relative-base/worker-relative-base.spec.ts:60-140` | 159/159 (100.0%) | 159/159 (100.0%) |
| vite | IN | `playground/multiple-entrypoints/entrypoints/a15.js:1-19` + `playground/multiple-entrypoints/entrypoints/a16.js:1-18` | 37/37 (100.0%) | 37/37 (100.0%) |
| vite | IN | `playground/css/__tests__/tests.ts:115-129` + `playground/css/__tests__/sass-tests.ts:28-44` | 32/32 (100.0%) | 32/32 (100.0%) |
| vite | IN | `packages/vite/src/node/server/hmr.ts:1078-1090` + `packages/vite/src/node/server/hmr.ts:1065-1077` | 26/26 (100.0%) | 26/26 (100.0%) |
| vite | IN | `playground/worker/__tests__/iife/worker-iife.spec.ts:33-65` + `playground/worker/__tests__/es/worker-es.spec.ts:34-68` | 68/68 (100.0%) | 68/68 (100.0%) |
| vite | OUT | `packages/vite/src/types/ws.d.ts:262-271` + `packages/vite/src/types/chokidar.d.ts:77-92` | 5/26 (19.2%) | 5/26 (19.2%) |
| zlib | IN | `gzwrite.c:666-673` + `gzwrite.c:543-550` | 16/16 (100.0%) | 16/16 (100.0%) |
| zlib | IN | `contrib/minizip/iowin32.c:161-182` + `contrib/minizip/iowin32.c:91-112` | 44/44 (100.0%) | 44/44 (100.0%) |
| zlib | IN | `crc32.h:7580-7631` + `crc32.h:3602-3653` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `contrib/iostream3/zfstream.h:234-313` + `contrib/iostream3/zfstream.h:323-402` | 160/160 (100.0%) | 160/160 (100.0%) |
| zlib | IN | `inflate.c:776-787` + `infback.c:281-288` | 20/20 (100.0%) | 20/20 (100.0%) |
| zlib | IN | `contrib/infback9/infback9.c:334-342` + `infback.c:294-302` | — | — |
| zlib | IN | `contrib/minizip/iowin32.c:163-184` + `contrib/minizip/iowin32.c:93-114` | 44/44 (100.0%) | 44/44 (100.0%) |
| zlib | IN | `examples/zpipe.c:40-48` + `examples/zpipe.c:96-104` | — | — |
| zlib | IN | `contrib/minizip/iowin32.c:160-185` + `contrib/minizip/iowin32.c:90-115` | 52/52 (100.0%) | 52/52 (100.0%) |
| zlib | IN | `crc32.h:9118-9169` + `crc32.h:3394-3445` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `contrib/minizip/ioapi.c:171-194` + `contrib/minizip/ioapi.c:148-169` | 46/46 (100.0%) | 46/46 (100.0%) |
| zlib | IN | `contrib/dotzlib/DotZLib/Deflater.cs:18-105` + `contrib/dotzlib/DotZLib/Inflater.cs:18-104` | 175/175 (100.0%) | 175/175 (100.0%) |
| zlib | IN | `infback.c:262-274` + `inflate.c:757-769` | 26/26 (100.0%) | 26/26 (100.0%) |
| zlib | IN | `deflate.c:1595-1609` + `deflate.c:1523-1538` | 31/31 (100.0%) | 31/31 (100.0%) |
| zlib | IN | `contrib/infback9/infback9.c:86-94` + `contrib/infback9/infback9.c:97-105` | 18/18 (100.0%) | 18/18 (100.0%) |
| zlib | IN | `contrib/minizip/unzip.c:762-775` + `contrib/minizip/unzip.c:790-803` | 28/28 (100.0%) | 28/28 (100.0%) |
| zlib | IN | `gzread.c:426-433` + `gzread.c:552-559` | — | — |
| zlib | IN | `gzwrite.c:616-630` + `gzwrite.c:495-508` | 29/29 (100.0%) | 29/29 (100.0%) |
| zlib | IN | `contrib/minizip/zip.c:665-675` + `contrib/minizip/zip.c:633-643` | 22/22 (100.0%) | 22/22 (100.0%) |
| zlib | IN | `crc32.h:1804-1855` + `crc32.h:5990-6041` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `gzwrite.c:620-631` + `gzwrite.c:498-508` | 23/23 (100.0%) | 23/23 (100.0%) |
| zlib | IN | `crc32.c:811-888` + `crc32.c:690-766` | 155/155 (100.0%) | 155/155 (100.0%) |
| zlib | IN | `crc32.h:1908-1959` + `crc32.h:6094-6145` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `contrib/minizip/zip.c:625-650` + `contrib/minizip/zip.c:606-623` | 44/44 (100.0%) | 44/44 (100.0%) |
| zlib | IN | `contrib/infback9/infback9.c:97-104` + `contrib/infback9/infback9.c:86-93` | 16/16 (100.0%) | 16/16 (100.0%) |
| zlib | IN | `contrib/minizip/unzip.c:1792-1798` + `contrib/minizip/unzip.c:1872-1878` | 14/14 (100.0%) | 14/14 (100.0%) |
| zlib | IN | `crc32.h:266-317` + `crc32.h:2914-2965` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `inflate.c:938-948` + `inflate.c:990-1000` | 22/22 (100.0%) | 22/22 (100.0%) |
| zlib | IN | `contrib/blast/blast.c:190-241` + `contrib/puff/puff.c:340-379` | 92/92 (100.0%) | 92/92 (100.0%) |
| zlib | IN | `inflate.c:812-829` + `infback.c:315-332` | 36/36 (100.0%) | 36/36 (100.0%) |
| zlib | IN | `zutil.c:105-118` + `zutil.c:89-102` | 28/28 (100.0%) | 28/28 (100.0%) |
| zlib | IN | `contrib/minizip/ioapi.c:100-116` + `contrib/minizip/ioapi.c:82-98` | 34/34 (100.0%) | 34/34 (100.0%) |
| zlib | IN | `contrib/infback9/inftree9.c:228-236` + `contrib/infback9/inftree9.c:302-310` | — | — |
| zlib | IN | `contrib/infback9/infback9.c:232-239` + `infback.c:198-206` | 17/17 (100.0%) | 17/17 (100.0%) |
| zlib | IN | `deflate.c:600-607` + `deflate.c:323-328` | 14/14 (100.0%) | 14/14 (100.0%) |
| zlib | IN | `infback.c:402-419` + `inflate.c:900-917` | 36/36 (100.0%) | 36/36 (100.0%) |
| zlib | IN | `contrib/dotzlib/DotZLib/GZipStream.cs:199-218` + `contrib/dotzlib/DotZLib/GZipStream.cs:154-175` | 42/42 (100.0%) | 42/42 (100.0%) |
| zlib | IN | `gzread.c:426-442` + `gzread.c:517-533` | — | — |
| zlib | IN | `contrib/dotzlib/DotZLib/UnitTests.cs:135-144` + `contrib/dotzlib/DotZLib/UnitTests.cs:101-110` | 20/20 (100.0%) | 20/20 (100.0%) |
| zlib | IN | `contrib/minizip/zip.c:654-666` + `contrib/minizip/zip.c:626-638` | 26/26 (100.0%) | 26/26 (100.0%) |
| zlib | IN | `contrib/blast/blast.c:131-143` + `contrib/puff/puff.c:266-278` | 26/26 (100.0%) | 26/26 (100.0%) |
| zlib | IN | `test/example.c:341-351` + `test/example.c:175-185` | 22/22 (100.0%) | 22/22 (100.0%) |
| zlib | IN | `contrib/minizip/unzip.c:628-653` + `contrib/minizip/zip.c:1104-1127` | 50/50 (100.0%) | 50/50 (100.0%) |
| zlib | IN | `contrib/dotzlib/DotZLib/GZipStream.cs:92-128` + `contrib/dotzlib/DotZLib/CodecBase.cs:114-134` | 58/58 (100.0%) | 58/58 (100.0%) |
| zlib | IN | `crc32.h:1960-2011` + `crc32.h:4400-4451` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `gzwrite.c:440-469` + `gzwrite.c:557-586` | 6/60 (10.0%) | 6/60 (10.0%) |
| zlib | IN | `inflate.c:717-755` + `infback.c:228-260` | 72/72 (100.0%) | 72/72 (100.0%) |
| zlib | IN | `inffast.c:132-143` + `inffast.c:101-112` | 24/24 (100.0%) | 24/24 (100.0%) |
| zlib | IN | `contrib/minizip/unzip.c:493-513` + `contrib/minizip/unzip.c:552-572` | 42/42 (100.0%) | 42/42 (100.0%) |
| zlib | IN | `gzread.c:705-713` + `gzwrite.c:703-713` | 20/20 (100.0%) | 20/20 (100.0%) |
| zlib | IN | `gzwrite.c:271-281` + `gzwrite.c:298-308` | — | — |
| zlib | IN | `test/example.c:436-442` + `test/example.c:400-406` | 14/14 (100.0%) | 14/14 (100.0%) |
| zlib | IN | `infback.c:572-579` + `contrib/infback9/infback9.c:596-603` | 16/16 (100.0%) | 16/16 (100.0%) |
| zlib | IN | `crc32.h:3654-3705` + `crc32.h:7632-7683` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `deflate.c:2002-2016` + `deflate.c:1902-1916` | 30/30 (100.0%) | 30/30 (100.0%) |
| zlib | IN | `contrib/minizip/unzip.h:133-152` + `contrib/minizip/unzip.h:112-131` | 40/40 (100.0%) | 40/40 (100.0%) |
| zlib | IN | `contrib/dotzlib/DotZLib/UnitTests.cs:102-111` + `contrib/dotzlib/DotZLib/UnitTests.cs:136-145` | 20/20 (100.0%) | 20/20 (100.0%) |
| zlib | IN | `infback.c:486-510` + `contrib/infback9/infback9.c:518-537` | 45/45 (100.0%) | 45/45 (100.0%) |
| zlib | IN | `crc32.h:6042-6093` + `crc32.h:1856-1907` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `crc32.h:422-473` + `crc32.h:1324-1375` | 104/104 (100.0%) | 104/104 (100.0%) |
| zlib | IN | `contrib/testzlib/testzlib.c:256-263` + `contrib/testzlib/testzlib.c:215-222` | 16/16 (100.0%) | 16/16 (100.0%) |
| zlib | IN | `gzwrite.c:429-437` + `gzwrite.c:542-550` | 18/18 (100.0%) | 18/18 (100.0%) |
| zod | IN | `packages/zod/src/v4/core/api.ts:162-177` + `packages/zod/src/v4/core/api.ts:180-195` | 32/32 (100.0%) | 32/32 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/codec.test.ts:88-186` + `packages/zod/src/v4/classic/tests/codec.test.ts:85-183` | 198/198 (100.0%) | 198/198 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/index.test.ts:480-489` + `packages/zod/src/v4/mini/tests/index.test.ts:556-565` | 20/20 (100.0%) | 20/20 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/partial.test.ts:79-147` + `packages/zod/src/v3/tests/partials.test.ts:165-233` | 138/138 (100.0%) | 138/138 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/map.test.ts:181-189` + `packages/zod/src/v3/tests/map.test.ts:71-79` | 18/18 (100.0%) | 18/18 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:84-204` + `packages/zod/src/v4/classic/tests/index.test.ts:84-204` | 242/242 (100.0%) | 242/242 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/index.test.ts:275-289` + `packages/zod/src/v4/mini/tests/index.test.ts:331-345` | 19/30 (63.3%) | 19/30 (63.3%) |
| zod | IN | `packages/zod/src/v4/classic/tests/record.test.ts:420-433` + `packages/zod/src/v3/tests/record.test.ts:149-162` | 28/28 (100.0%) | 28/28 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:900-905` + `packages/zod/src/v4/classic/tests/stringbool.test.ts:19-24` | 12/12 (100.0%) | 12/12 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:935-950` + `packages/zod/src/v4/classic/tests/index.test.ts:832-847` | 32/32 (100.0%) | 32/32 (100.0%) |
| zod | IN | `packages/zod/src/v3/tests/async-parsing.test.ts:53-110` + `packages/zod/src/v4/classic/tests/async-parsing.test.ts:52-109` | 116/116 (100.0%) | 116/116 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:155-222` + `packages/zod/src/v4/classic/tests/index.test.ts:155-222` | 136/136 (100.0%) | 136/136 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/union.test.ts:5-27` + `packages/zod/src/v3/tests/unions.test.ts:6-28` | 46/46 (100.0%) | 46/46 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/in-out.test.ts:147-150` + `packages/zod/src/v4/classic/tests/in-out.test.ts:6-9` | 8/8 (100.0%) | 8/8 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/recursive-types.test.ts:152-159` + `packages/zod/src/v4/classic/tests/recursive-types.test.ts:145-152` | 16/16 (100.0%) | 16/16 (100.0%) |
| zod | IN | `packages/zod/src/v4/core/checks.ts:1087-1095` + `packages/zod/src/v4/core/checks.ts:1043-1051` | 18/18 (100.0%) | 18/18 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:44-166` + `packages/zod/src/v4/classic/tests/index.test.ts:44-166` | 246/246 (100.0%) | 246/246 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/from-json-schema.ts:567-572` + `packages/zod/src/v4/classic/from-json-schema.ts:772-777` | 12/12 (100.0%) | 12/12 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/detached-methods.test.ts:75-80` + `packages/zod/src/v4/classic/tests/detached-methods.test.ts:63-68` | 12/12 (100.0%) | 12/12 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/index.test.ts:461-560` + `packages/zod/src/v4/mini/tests/index.test.ts:537-636` | 200/200 (100.0%) | 200/200 (100.0%) |
| zod | IN | `packages/zod/src/v3/tests/async-parsing.test.ts:68-125` + `packages/zod/src/v4/classic/tests/async-parsing.test.ts:67-124` | 116/116 (100.0%) | 116/116 (100.0%) |
| zod | IN | `vitest.compile.config.ts:13-22` + `vitest.config.ts:8-17` | 20/20 (100.0%) | 20/20 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:134-215` + `packages/zod/src/v4/classic/tests/index.test.ts:134-215` | 164/164 (100.0%) | 164/164 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:803-812` + `packages/zod/src/v4/classic/tests/index.test.ts:727-736` | 20/20 (100.0%) | 20/20 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/brand.test.ts:16-21` + `packages/zod/src/v3/tests/branded.test.ts:18-23` | 12/12 (100.0%) | 12/12 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/index.test.ts:44-67` + `packages/zod/src/v4/mini/tests/index.test.ts:44-67` | 48/48 (100.0%) | 48/48 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/index.test.ts:174-191` + `packages/zod/src/v4/classic/tests/index.test.ts:174-191` | 36/36 (100.0%) | 36/36 (100.0%) |
| zod | IN | `packages/zod/src/v3/tests/object.test.ts:186-199` + `packages/zod/src/v4/classic/tests/object.test.ts:195-208` | 28/28 (100.0%) | 28/28 (100.0%) |
| zod | IN | `packages/zod/src/v4/classic/tests/record.test.ts:266-280` + `packages/zod/src/v4/classic/tests/record.test.ts:179-193` | 30/30 (100.0%) | 30/30 (100.0%) |
| zod | IN | `packages/zod/src/v4/mini/tests/codec.test.ts:27-42` + `packages/zod/src/v4/classic/tests/codec.test.ts:24-39` | 32/32 (100.0%) | 32/32 (100.0%) |
| zod | IN | `packages/docs/app/og.png/route.tsx:15-106` + `packages/docs/pages/api/_og.tsx:32-125` | 186/186 (100.0%) | 186/186 (100.0%) |
| zod | OUT | `packages/docs/next.config.mjs:106-113` + `packages/docs/next.config.mjs:165-172` | — | — |
| zod | OUT | `packages/docs/next.config.mjs:107-116` + `packages/docs/next.config.mjs:166-173` | — | — |
| zod | OUT | `packages/docs/next.config.mjs:108-117` + `packages/docs/next.config.mjs:167-174` | — | — |
| zod | OUT | `packages/docs/next.config.mjs:143-150` + `packages/docs/next.config.mjs:163-170` | — | — |
| zod | OUT | `packages/docs/next.config.mjs:144-151` + `packages/docs/next.config.mjs:164-171` | — | — |

## Per repository — cost

Description, never scored. Reported beside the accuracy tables so a change in cost can never be mistaken for a change in what the engine found. `wall` is the scan's elapsed time; `CPU time` is user + system time across every core; `peak CPU` is the busiest 500 ms of the scan, in percent of one core (`400%` is four cores busy); `peak memory` is the peak resident set.

| repository | clusters `89751512d3a4` | clusters `fb68ce65f2d5` | wall `89751512d3a4` | wall `fb68ce65f2d5` | CPU time `89751512d3a4` | CPU time `fb68ce65f2d5` | peak CPU `89751512d3a4` | peak CPU `fb68ce65f2d5` | peak memory `89751512d3a4` | peak memory `fb68ce65f2d5` |
|---|---|---|---|---|---|---|---|---|---|---|
| axios (javascript) | 218 | 219 | 1.91 s | 2.31 s | 2.74 s | 2.61 s | 238% | 285% | 149 MB | 157 MB |
| BenchmarkDotNet (csharp) | 1666 | 1702 | 6.01 s | 4.38 s | 23.86 s | 22.41 s | 802% | 1090% | 845 MB | 885 MB |
| bloc (dart) | 984 | 998 | 3.51 s | 2.90 s | 14.09 s | 12.78 s | 862% | 620% | 532 MB | 544 MB |
| click (python) | 276 | 282 | 1.12 s | 0.94 s | 2.35 s | 2.01 s | 273% | 281% | 151 MB | 152 MB |
| cobra (go) | 457 | 455 | 1.78 s | 1.38 s | 4.27 s | 3.55 s | 304% | 312% | 209 MB | 211 MB |
| composer (php) | 2178 | 2188 | 5.28 s | 4.24 s | 22.19 s | 21.35 s | 917% | 976% | 835 MB | 911 MB |
| console (php) | 1162 | 1163 | 2.55 s | 2.23 s | 9.40 s | 9.18 s | 510% | 661% | 471 MB | 489 MB |
| date-fns (typescript) | 1930 | 1938 | 43.30 s | 44.59 s | 156.96 s | 155.71 s | 960% | 1225% | 1093 MB | 1093 MB |
| Deslop (rust) | 1083 | 1072 | 4.14 s | 5.32 s | 13.45 s | 13.46 s | 784% | 849% | 720 MB | 785 MB |
| dio (dart) | 301 | 303 | 1.23 s | 1.36 s | 2.89 s | 2.81 s | 295% | 210% | 178 MB | 179 MB |
| django (python) | 8947 | 9029 | 21.09 s | 20.16 s | 89.15 s | 88.88 s | 1126% | 1139% | 2465 MB | 2868 MB |
| express (javascript) | 473 | 476 | 2.23 s | 2.22 s | 8.06 s | 8.46 s | 483% | 492% | 376 MB | 379 MB |
| fastapi (python) | 2085 | 2093 | 207.15 s | 132.29 s | 388.54 s | 354.32 s | 1135% | 1195% | 1788 MB | 1755 MB |
| flask (python) | 206 | 211 | 1.57 s | 0.87 s | 1.70 s | 1.35 s | 162% | 178% | 109 MB | 107 MB |
| FsCheck (fsharp) | 279 | 282 | 2.06 s | 1.32 s | 4.28 s | 3.32 s | 283% | 325% | 201 MB | 203 MB |
| FSharp.Data (fsharp) | 823 | 842 | 4.25 s | 2.68 s | 11.29 s | 9.28 s | 513% | 462% | 457 MB | 469 MB |
| FSharpPlus (fsharp) | 1848 | 1888 | 10.48 s | 8.63 s | 45.54 s | 39.06 s | 875% | 1061% | 903 MB | 782 MB |
| gin (go) | 585 | 606 | 1.79 s | 1.94 s | 5.17 s | 5.19 s | 350% | 370% | 249 MB | 255 MB |
| Giraffe (fsharp) | 319 | 322 | 3.08 s | 3.50 s | 7.06 s | 6.97 s | 476% | 346% | 215 MB | 234 MB |
| guzzle (php) | 1480 | 1485 | 3.39 s | 4.73 s | 10.82 s | 13.95 s | 535% | 563% | 533 MB | 509 MB |
| http (dart) | 763 | 773 | 14.77 s | 24.67 s | 77.23 s | 95.00 s | 1215% | 815% | 1956 MB | 1608 MB |
| hugo (go) | 2515 | 2848 | 8.88 s | 13.43 s | 39.65 s | 50.91 s | 965% | 932% | 1236 MB | 1139 MB |
| mocktail (dart) | 73 | 73 | 0.34 s | 0.39 s | 0.50 s | 0.55 s | — | — | 51 MB | 49 MB |
| monolog (php) | 421 | 425 | 0.93 s | 1.05 s | 2.81 s | 3.05 s | 385% | 367% | 210 MB | 203 MB |
| nest (typescript) | 1598 | 1616 | 5.65 s | 7.10 s | 30.16 s | 34.99 s | 1214% | 998% | 829 MB | 912 MB |
| Newtonsoft.Json (csharp) | 4098 | 4170 | 9.67 s | 11.52 s | 49.17 s | 54.58 s | 1290% | 1134% | 1332 MB | 1558 MB |
| Polly (csharp) | 2100 | 2112 | 6.79 s | 7.78 s | 46.27 s | 48.76 s | 1321% | 1163% | 1044 MB | 1030 MB |
| rayon (rust) | 625 | 626 | 2.67 s | 2.73 s | 11.27 s | 11.49 s | 798% | 800% | 386 MB | 389 MB |
| react (javascript) | 11501 | 11788 | 41.57 s | 57.23 s | 288.18 s | 311.63 s | 1318% | 1264% | 6889 MB | 6805 MB |
| ripgrep (rust) | 630 | 726 | 2.07 s | 2.65 s | 8.29 s | 10.44 s | 687% | 676% | 441 MB | 443 MB |
| serde (rust) | 775 | 790 | 2.62 s | 2.63 s | 7.63 s | 8.04 s | 436% | 471% | 362 MB | 362 MB |
| serilog (csharp) | 270 | 275 | 1.03 s | 1.00 s | 2.69 s | 2.72 s | 281% | 299% | 197 MB | 201 MB |
| tokio (rust) | 1951 | 1961 | 4.38 s | 4.03 s | 20.80 s | 20.88 s | 1128% | 1245% | 821 MB | 901 MB |
| viper (go) | 145 | 146 | 0.63 s | 0.65 s | 0.80 s | 0.85 s | 121% | 120% | 70 MB | 70 MB |
| vite (typescript) | 976 | 980 | 4.84 s | 5.06 s | 18.13 s | 18.77 s | 955% | 899% | 621 MB | 648 MB |
| zlib (c) | 547 | 550 | 7.29 s | 11.41 s | 11.35 s | 16.35 s | 338% | 324% | 318 MB | 307 MB |
| zod (typescript) | 2055 | 2038 | 19.37 s | 22.63 s | 102.61 s | 109.53 s | 1176% | 1292% | 1305 MB | 1365 MB |
| osprey (osprey) | 14529 | 13354 | 151.80 s | 35.09 s | 373.28 s | 168.27 s | 1261% | 1338% | 5368 MB | 4029 MB |
| jq (c) | 370 | 371 | 1.70 s | 1.08 s | 2.66 s | 2.69 s | 250% | 325% | 169 MB | 171 MB |
| hiredis (c) | 313 | 316 | 1.48 s | 0.90 s | 1.81 s | 1.79 s | 151% | 234% | 133 MB | 136 MB |
| clap (rust) | 1363 | 1401 | 5.92 s | 3.82 s | 14.23 s | 16.49 s | 590% | 702% | 645 MB | 680 MB |
| trpc (typescript) | 1706 | 1716 | 11.47 s | 6.73 s | 31.22 s | 34.95 s | 619% | 1069% | 1344 MB | 1423 MB |
| riverpod (dart) | 2235 | 2256 | 58.24 s | 29.21 s | 168.50 s | 180.06 s | 645% | 1313% | 1803 MB | 2214 MB |
| MediatR (csharp) | 271 | 275 | 2.51 s | 1.35 s | 3.49 s | 3.23 s | 181% | 306% | 171 MB | 175 MB |
| httpx (python) | 339 | 341 | 1.59 s | 1.53 s | 3.05 s | 3.20 s | 205% | 196% | 159 MB | 162 MB |
| Argu (fsharp) | 171 | 172 | 1.11 s | 1.22 s | 1.08 s | 1.14 s | 147% | 144% | 80 MB | 79 MB |
| libuv (c) | 2536 | 2546 | 11.78 s | 15.81 s | 36.06 s | 37.05 s | 655% | 653% | 644 MB | 603 MB |

## Judged pair record

`corpus/register/outcomes.json` holds every judged pair's last recorded outcome. An unrecorded failure or a regression fails the run. Only explicitly accepted failures are tolerated.

**PASS** — every failed judged pair is explicitly accepted on record.

### Every judged pair

Each pair stands alone: one regression or unrecorded failure fails the run however many other pairs were fixed.

| repository | verdict | judged pair | result | standing |
|---|---|---|---|---|
| axios | clearly_in | `test/unit/adapters/http.js:1321-1341` + `test/unit/adapters/http.js:1364-1384` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:133-143` + `test/module/typings/esm/index.ts:156-166` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:141-173` + `test/module/typings/esm/index.ts:164-196` | ✅ pass | passing |
| axios | clearly_in | `test/specs/xsrf.spec.js:100-113` + `test/specs/xsrf.spec.js:115-128` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:153-163` + `test/module/typings/esm/index.ts:176-186` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:251-259` + `test/module/typings/esm/index.ts:274-282` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:76-106` + `test/module/typings/esm/index.ts:99-129` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:227-252` + `test/module/typings/esm/index.ts:250-275` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:254-275` + `test/module/typings/esm/index.ts:277-298` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:19-28` + `test/module/typings/esm/index.ts:42-51` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:132-152` + `test/unit/adapters/fetch.js:206-226` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:239-256` + `test/module/typings/esm/index.ts:262-279` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:137-169` + `test/module/typings/esm/index.ts:160-192` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:167-197` + `test/module/typings/esm/index.ts:190-220` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:149-181` + `test/module/typings/esm/index.ts:172-204` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:179-212` + `test/module/typings/esm/index.ts:202-235` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:174-188` + `test/unit/adapters/fetch.js:249-263` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:134-154` + `test/unit/adapters/fetch.js:208-228` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:72-102` + `test/module/typings/esm/index.ts:95-125` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:133-163` + `test/module/typings/esm/index.ts:156-186` | ✅ pass | passing |
| axios | clearly_in | `test/specs/utils/endsWith.js:1-12` + `test/specs/utils/kindOfTest.js:1-12` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:145-177` + `test/module/typings/esm/index.ts:168-200` | ✅ pass | passing |
| axios | clearly_in | `test/module/test.js:80-92` + `test/module/test.js:94-106` | ✅ pass | passing |
| axios | clearly_in | `test/specs/interceptors.spec.js:575-587` + `test/specs/interceptors.spec.js:589-601` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:167-173` + `test/module/typings/esm/index.ts:190-196` | ✅ pass | passing |
| axios | clearly_in | `test/specs/core/mergeConfig.spec.js:173-213` + `test/specs/core/mergeConfig.spec.js:229-269` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:129-159` + `test/module/typings/esm/index.ts:152-182` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:68-98` + `test/module/typings/esm/index.ts:91-121` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:223-251` + `test/module/typings/esm/index.ts:246-274` | ✅ pass | passing |
| axios | clearly_in | `test/specs/requests.spec.js:236-255` + `test/specs/requests.spec.js:257-276` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:1364-1384` + `test/unit/adapters/http.js:1400-1420` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:264-275` + `test/module/typings/esm/index.ts:287-298` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:157-189` + `test/module/typings/esm/index.ts:180-212` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:251-263` + `test/unit/adapters/http.js:2000-2012` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:171-201` + `test/module/typings/esm/index.ts:194-224` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:161-193` + `test/module/typings/esm/index.ts:184-216` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:80-111` + `test/module/typings/esm/index.ts:103-134` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:175-205` + `test/module/typings/esm/index.ts:198-228` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:345-354` + `test/module/typings/esm/index.ts:374-383` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:1321-1341` + `test/unit/adapters/http.js:1400-1420` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:1960-1979` + `test/unit/adapters/http.js:2018-2038` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:219-249` + `test/module/typings/esm/index.ts:242-272` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:219-225` + `test/module/typings/esm/index.ts:242-248` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:153-185` + `test/module/typings/esm/index.ts:176-208` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:183-193` + `test/module/typings/esm/index.ts:206-216` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:1057-1077` + `test/unit/adapters/http.js:1158-1178` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:223-233` + `test/module/typings/esm/index.ts:246-256` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:104-116` + `test/module/typings/esm/index.ts:127-139` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:84-116` + `test/module/typings/esm/index.ts:107-139` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:96-102` + `test/module/typings/esm/index.ts:119-125` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:141-151` + `test/module/typings/esm/index.ts:164-174` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:231-253` + `test/module/typings/esm/index.ts:254-276` | ✅ pass | passing |
| axios | clearly_in | `test/module/typings/cjs/index.ts:279-289` + `test/module/typings/cjs/index.ts:294-304` | ✅ pass | passing |
| axios | clearly_in | `test/specs/interceptors.spec.js:238-252` + `test/specs/interceptors.spec.js:267-281` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:251-263` + `test/unit/adapters/http.js:2060-2072` | ✅ pass | passing |
| axios | clearly_in | `test/helpers/server.js:13-52` + `test/unit/adapters/http.js:65-104` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/fetch.js:131-189` + `test/unit/adapters/fetch.js:205-264` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:1958-2014` + `test/unit/adapters/http.js:2016-2074` | ✅ pass | passing |
| axios | clearly_in | `test/unit/adapters/http.js:2196-2209` + `test/unit/adapters/http.js:969-982` | ✅ pass | passing |
| axios | clearly_out | `lib/axios.js:50-60` + `test/manual/promise.js:9-9` | ✅ pass | passing |
| axios | clearly_out | `index.d.ts:197-206` + `index.d.ts:218-227` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:156-168` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncStateMachineEmitter.cs:501-531` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Diagnostics.Windows/JitStatsDiagnoser.cs:59-72` + `src/BenchmarkDotNet.Diagnostics.Windows/JitStatsDiagnoser.cs:74-87` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Helpers/CodeAnnotations.cs:1307-1318` + `src/BenchmarkDotNet/Helpers/CodeAnnotations.cs:1344-1355` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:196-268` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/AsyncBenchmarkAnalyzerTests.cs:35-107` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Detectors/Cpu/LinuxCpuInfoParserTests.cs:46-61` + `tests/BenchmarkDotNet.Tests/Detectors/Cpu/LinuxCpuInfoParserTests.cs:63-78` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:147-156` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:554-561` | 🔴 fail | on record as failing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:131-141` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:296-306` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1013-1085` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1111-1189` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Analyzers/Attributes/GeneralParameterAttributesAnalyzer.cs:12-63` + `src/BenchmarkDotNet.Analyzers/General/BenchmarkClassAnalyzer.cs:12-63` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1716-1727` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:918-930` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:115-143` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:84-112` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1281-1358` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:676-753` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Running/BenchmarkRunnerDirty.cs:29-38` + `src/BenchmarkDotNet/Running/BenchmarkRunnerDirty.cs:65-74` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Diagnostics.dotMemory/DotMemoryDiagnoser.cs:1-79` + `src/BenchmarkDotNet.Diagnostics.dotTrace/DotTraceDiagnoser.cs:1-82` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/GithubMarkdownDisassemblyExporterMultiCorerunTest.cs:21-37` + `tests/BenchmarkDotNet.Tests/Disassemblers/GithubMarkdownDisassemblyExporterMultiCorerunTest.cs:43-55` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/BenchmarkCancellationAttributeAnalyzerTests.cs:80-120` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:1269-1313` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:13-48` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:56-100` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:130-169` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:346-379` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Reports/SummaryTableTests.cs:259-269` + `tests/BenchmarkDotNet.Tests/Reports/SummaryTableTests.cs:282-292` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1027-1037` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1329-1339` | 🔴 fail | on record as failing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Engine/EngineActualStageTests.cs:31-44` + `tests/BenchmarkDotNet.Tests/Engine/EngineWarmupStageTests.cs:80-93` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/AllSetupAndCleanupTest.cs:109-148` + `tests/BenchmarkDotNet.IntegrationTests/AllSetupAndCleanupTest.cs:171-210` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.Movz.cs:23-33` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenMovk.cs:23-33` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Validators/ExecutionValidatorTests.cs:620-698` + `tests/BenchmarkDotNet.Tests/Validators/ReturnValueValidatorTests.cs:331-419` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1912-1919` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:662-669` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:138-145` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:201-208` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/Mono/RoslynMonoAotToolchain.cs:1-53` + `src/BenchmarkDotNet/Toolchains/Mono/RoslynMonoToolchain.cs:1-53` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1211-1246` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:452-480` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Exporters.Plotting/ScottPlotExporter.cs:179-253` + `src/BenchmarkDotNet.Exporters.Plotting/ScottPlotExporter.cs:255-337` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/EventProcessorTests.cs:29-69` + `tests/BenchmarkDotNet.IntegrationTests/EventProcessorTests.cs:71-112` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:351-357` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:435-441` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:162-171` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:349-356` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Analyzers/AnalyzerHelper.cs:29-36` + `src/BenchmarkDotNet.Analyzers/AnalyzerHelper.cs:40-47` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:1637-1683` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/RequiredMemberAnalyzerTests.cs:36-75` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:1869-1902` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/General/BenchmarkClassAnalyzerTests.cs:966-997` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Analyzers/Attributes/ParamsAllValuesAttributeAnalyzer.cs:21-50` + `src/BenchmarkDotNet.Analyzers/BenchmarkRunner/RunAnalyzer.cs:47-82` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenLdr.cs:19-32` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64RegisterValueAccumulatorTests.MovzThenLdr.cs:95-108` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Detectors/OsDetector.cs:131-139` + `src/BenchmarkDotNet/Detectors/OsDetector.cs:140-147` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:711-809` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:811-909` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:114-163` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:383-443` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:115-152` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:67-104` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:310-318` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:552-559` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryResolvePreCode.cs:25-40` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryResolvePreCode.cs:59-74` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableParamsSourceTests.cs:19-35` + `tests/BenchmarkDotNet.IntegrationTests/ParamSourceTests.cs:17-33` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Mathematics/StatisticsTests.cs:69-76` + `tests/BenchmarkDotNet.Tests/Mathematics/StatisticsTests.cs:93-100` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:16-40` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:291-315` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/wwwroot/custom-main-node.mjs:85-110` + `tests/BenchmarkDotNet.IntegrationTests/wwwroot/custom-main-v8.mjs:110-140` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Exporters.Plotting.Tests/ScottPlotExporterTests.cs:189-206` + `tests/BenchmarkDotNet.Tests/Exporters/MarkdownExporterVerifyTests.cs:158-175` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableBenchmarksTests.cs:111-121` + `tests/BenchmarkDotNet.IntegrationTests/AsyncEnumerableBenchmarksTests.cs:79-91` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:756-825` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:897-965` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Detectors/Cpu/CpuInfoParserTests_Wmic.cs:44-53` + `tests/BenchmarkDotNet.Tests/Detectors/Cpu/CpuInfoParserTests_Wmic.cs:76-85` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/DotNetCli/DotNetCliExecutor.cs:87-125` + `src/BenchmarkDotNet/Toolchains/Wasm/WasmExecutor.cs:190-244` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/BenchmarkCancellationAttributeAnalyzerTests.cs:78-120` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:42-95` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Exporters/OpenMetricsExporterTests.cs:101-179` + `tests/BenchmarkDotNet.Tests/Exporters/OpenMetricsExporterTests.cs:24-99` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:756-794` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/GeneralParameterAttributesAnalyzerTests.cs:897-934` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:41-66` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/SetupCleanupAsyncEnumerableAnalyzerTests.cs:97-122` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryTranslateAddressToName.cs:237-260` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryTranslateAddressToName.cs:267-288` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:658-716` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:380-434` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Helpers/HashCode.cs:85-92` + `src/BenchmarkDotNet/Helpers/HashCode.cs:98-105` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:138-146` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:303-311` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:12-48` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/BenchmarkRunner/RunAnalyzerTests.cs:55-100` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:1461-1491` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1321-1359` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:232-242` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryFollowJumpTrampoline.cs:49-59` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Analyzers/Attributes/GeneralArgumentAttributesAnalyzer.cs:12-42` + `src/BenchmarkDotNet.Analyzers/General/BenchmarkClassAnalyzer.cs:73-110` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:243-264` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ArgumentsAttributeAnalyzerTests.cs:526-545` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:421-440` + `src/BenchmarkDotNet/Toolchains/InProcess/NoEmit/BenchmarkActionImpl.cs:573-592` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:11-45` + `tests/BenchmarkDotNet.Tests/Disassemblers/Arm64/Arm64DisassemblerTests.TryReadStubHead.cs:47-80` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/RuntimeAndToolchainCodeFixProviderTests.cs:12-39` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/RuntimeAndToolchainCodeFixProviderTests.cs:41-68` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `src/BenchmarkDotNet.Analyzers/Attributes/ArgumentsAttributeAnalyzer.cs:12-45` + `src/BenchmarkDotNet.Analyzers/Attributes/GeneralParameterAttributesAnalyzer.cs:57-90` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Tests/ConfigParserTests.cs:1103-1115` + `tests/BenchmarkDotNet.Tests/ConfigParserTests.cs:1117-1129` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.IntegrationTests/JitListenerTests.cs:139-149` + `tests/BenchmarkDotNet.IntegrationTests/JitListenerTests.cs:15-25` | ✅ pass | passing |
| benchmarkdotnet | clearly_in | `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:25-63` + `tests/BenchmarkDotNet.Analyzers.Tests/CodeFixTests/ArgumentsSourceElementTypeCodeFixProviderTests.cs:870-907` | ✅ pass | passing |
| benchmarkdotnet | clearly_out | `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:55-62` + `tests/BenchmarkDotNet.Tests/Environments/PhysicalMemoryInfoTests.cs:14-21` | ✅ pass | passing |
| benchmarkdotnet | clearly_out | `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:30-35` + `tests/BenchmarkDotNet.Tests/Environments/PhysicalMemoryInfoTests.cs:14-21` | ✅ pass | passing |
| benchmarkdotnet | clearly_out | `src/BenchmarkDotNet/Engines/JitListener.cs:57-63` + `src/BenchmarkDotNet/Helpers/Hashing/XxHashShared.cs:54-62` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_firebase_login/packages/form_inputs/lib/src/password.dart:19-28` + `examples/flutter_form_validation/lib/models/password.dart:9-18` | ✅ pass | passing |
| bloc | clearly_in | `extensions/vscode/src/commands/new-bloc.command.ts:20-74` + `extensions/vscode/src/commands/new-cubit.command.ts:16-70` | ✅ pass | passing |
| bloc | clearly_in | `packages/replay_bloc/test/replay_bloc_test.dart:271-306` + `packages/replay_bloc/test/replay_bloc_test.dart:529-564` | ✅ pass | passing |
| bloc | clearly_in | `extensions/vscode/src/commands/new-bloc.command.ts:110-146` + `extensions/vscode/src/commands/new-cubit.command.ts:96-132` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_todos/test/edit_todo/bloc/edit_todo_bloc_test.dart:170-186` + `examples/flutter_todos/test/edit_todo/bloc/edit_todo_bloc_test.dart:74-90` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_todos/test/stats/view/stats_page_test.dart:130-169` + `examples/flutter_todos/test/stats/view/stats_page_test.dart:89-128` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_login/lib/login/models/password.dart:1-14` + `examples/flutter_login/lib/login/models/username.dart:1-14` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_todos/lib/main_production.dart:1-13` + `examples/flutter_todos/lib/main_staging.dart:1-13` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_firebase_login/lib/login/view/login_form.dart:53-92` + `examples/flutter_firebase_login/lib/sign_up/view/sign_up_form.dart:42-81` | ✅ pass | passing |
| bloc | clearly_in | `packages/replay_bloc/test/replay_bloc_test.dart:202-240` + `packages/replay_bloc/test/replay_bloc_test.dart:489-527` | ✅ pass | passing |
| bloc | clearly_in | `packages/replay_bloc/test/replay_cubit_test.dart:188-245` + `packages/replay_bloc/test/replay_cubit_test.dart:433-490` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc/test/bloc_event_transformer_test.dart:5-41` + `packages/bloc/test/bloc_event_transformer_test_legacy.dart:5-41` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_login/test/login/view/login_form_test.dart:22-45` + `examples/flutter_login/test/login/view/login_form_test.dart:47-70` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc_lint/lib/src/text_document.dart:191-197` + `packages/bloc_tools/lib/src/lsp/text_document.dart:195-201` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_login/test/login/models/password_test.dart:2-38` + `examples/flutter_login/test/login/models/username_test.dart:2-38` | ✅ pass | passing |
| bloc | clearly_in | `packages/replay_bloc/test/replay_bloc_test.dart:423-443` + `packages/replay_bloc/test/replay_bloc_test.dart:71-91` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_timer/lib/timer/view/timer_page.dart:83-96` + `examples/flutter_timer/lib/timer/view/timer_page.dart:97-110` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc/test/bloc_test.dart:542-604` + `packages/bloc/test/bloc_test.dart:641-703` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc/test/bloc_test.dart:541-604` + `packages/bloc/test/bloc_test.dart:640-703` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_login/lib/login/bloc/login_event.dart:3-30` + `examples/flutter_todos/lib/edit_todo/bloc/edit_todo_event.dart:3-30` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc_lint/lib/src/text_document.dart:100-158` + `packages/bloc_tools/lib/src/lsp/text_document.dart:41-99` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_weather/lib/weather/cubit/weather_cubit.dart:25-36` + `examples/flutter_weather/lib/weather/cubit/weather_cubit.dart:49-60` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc/test/bloc_test.dart:303-316` + `packages/bloc/test/bloc_test.dart:352-365` | ✅ pass | passing |
| bloc | clearly_in | `extensions/vscode/src/templates/bloc-state.template.ts:4-55` + `extensions/vscode/src/templates/cubit-state.template.ts:4-55` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_todos/test/edit_todo/bloc/edit_todo_event_test.dart:26-42` + `examples/flutter_todos/test/edit_todo/bloc/edit_todo_event_test.dart:8-24` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:102-114` + `examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:224-236` | ✅ pass | passing |
| bloc | clearly_in | `examples/flutter_todos/lib/main_development.dart:1-13` + `examples/flutter_todos/lib/main_staging.dart:1-13` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc_lint/test/src/analysis_options_test.dart:135-147` + `packages/bloc_lint/test/src/linter_test.dart:49-61` | ✅ pass | passing |
| bloc | clearly_in | `packages/bloc/test/bloc_test.dart:303-315` + `packages/bloc/test/bloc_test.dart:352-364` | ✅ pass | passing |
| click | clearly_in | `tests/test_arguments.py:340-345` + `tests/test_arguments.py:394-399` | ✅ pass | passing |
| click | clearly_in | `src/click/_compat.py:176-191` + `src/click/_compat.py:194-209` | ✅ pass | passing |
| click | clearly_in | `tests/test_basic.py:550-562` + `tests/test_basic.py:608-620` | ✅ pass | passing |
| click | clearly_in | `tests/test_stream_lifecycle.py:340-347` + `tests/test_stream_lifecycle.py:498-505` | ✅ pass | passing |
| click | clearly_in | `src/click/_compat.py:287-300` + `src/click/_compat.py:303-316` | ✅ pass | passing |
| click | clearly_in | `src/click/decorators.py:324-349` + `src/click/decorators.py:352-377` | ✅ pass | passing |
| click | clearly_in | `src/click/shell_completion.py:432-442` + `src/click/shell_completion.py:530-540` | ✅ pass | passing |
| click | clearly_in | `src/click/exceptions.py:238-260` + `src/click/exceptions.py:274-296` | ✅ pass | passing |
| click | clearly_in | `src/click/shell_completion.py:451-464` + `src/click/shell_completion.py:527-540` | ✅ pass | passing |
| click | clearly_in | `src/click/decorators.py:133-165` + `src/click/decorators.py:258-290` | ✅ pass | passing |
| click | clearly_in | `src/click/shell_completion.py:454-464` + `src/click/shell_completion.py:530-540` | ✅ pass | passing |
| click | clearly_in | `tests/test_context.py:539-566` + `tests/test_context.py:593-620` | ✅ pass | passing |
| click | clearly_in | `tests/test_termui.py:977-985` + `tests/test_termui.py:997-1005` | ✅ pass | passing |
| click | clearly_in | `tests/test_testing.py:18-26` + `tests/test_testing.py:36-44` | ✅ pass | passing |
| click | clearly_in | `tests/test_options.py:939-946` + `tests/test_options.py:972-979` | ✅ pass | passing |
| click | clearly_in | `tests/test_context.py:52-59` + `tests/test_context.py:75-82` | ✅ pass | passing |
| click | clearly_in | `src/click/decorators.py:550-556` + `src/click/decorators.py:592-598` | ✅ pass | passing |
| click | clearly_in | `tests/test_basic.py:207-213` + `tests/test_basic.py:230-236` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3059-3083` + `completions_test.go:3159-3183` | ✅ pass | passing |
| cobra | clearly_in | `doc/md_docs.go:125-129` + `doc/yaml_docs.go:53-57` | ✅ pass | passing |
| cobra | clearly_in | `fish_completions.go:28-34` + `powershell_completions.go:31-37` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3060-3071` + `completions_test.go:3160-3171` | ✅ pass | passing |
| cobra | clearly_in | `command.go:563-570` + `command.go:573-580` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3142-3155` + `completions_test.go:3568-3581` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:1404-1433` + `completions_test.go:1504-1533` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:1337-1354` + `completions_test.go:2303-2320` | ✅ pass | passing |
| cobra | clearly_in | `command.go:1344-1355` + `command.go:1414-1425` | ✅ pass | passing |
| cobra | clearly_in | `command.go:412-420` + `command.go:422-430` | ✅ pass | passing |
| cobra | clearly_in | `bash_completions.go:567-573` + `bash_completions.go:581-587` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:391-423` + `completions_test.go:442-474` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:1326-1354` + `completions_test.go:2292-2320` | ✅ pass | passing |
| cobra | clearly_in | `args_test.go:190-206` + `args_test.go:240-256` | ✅ pass | passing |
| cobra | clearly_in | `command.go:464-473` + `command.go:505-515` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3122-3139` + `completions_test.go:3220-3237` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:1385-1402` + `completions_test.go:1718-1735` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:1753-1763` + `completions_test.go:1797-1807` | ✅ pass | passing |
| cobra | clearly_in | `doc/rest_docs_test.go:80-96` + `doc/yaml_docs_test.go:57-73` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3060-3071` + `completions_test.go:3258-3269` | ✅ pass | passing |
| cobra | clearly_in | `completions_test.go:3240-3253` + `completions_test.go:3334-3347` | ✅ pass | passing |
| cobra | clearly_in | `doc/md_docs.go:32-49` + `doc/rest_docs.go:30-49` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Util/ConfigValidator.php:190-197` + `src/Composer/Util/ConfigValidator.php:201-208` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/DownloadManagerTest.php:827-834` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:915-922` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Util/ForgejoTest.php:18-132` + `tests/Composer/Test/Util/GitHubTest.php:21-131` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/FossilDownloaderTest.php:98-161` + `tests/Composer/Test/Downloader/HgDownloaderTest.php:97-153` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Command/DiagnoseCommand.php:326-360` + `src/Composer/Command/DiagnoseCommand.php:365-399` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Command/RemoveCommand.php:50-61` + `src/Composer/Command/RequireCommand.php:87-98` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Command/DependsCommand.php:23-58` + `src/Composer/Command/ProhibitsCommand.php:23-59` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Command/RemoveCommand.php:51-56` + `src/Composer/Command/UpdateCommand.php:65-70` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/GitDownloaderTest.php:541-587` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:589-629` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/GitDownloaderTest.php:105-116` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:141-152` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Plugin/Fixtures/plugin-v2/Installer/Plugin2.php:9-27` + `tests/Composer/Test/Plugin/Fixtures/plugin-v9/Installer/Plugin.php:11-29` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:1637-1687` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:1787-1837` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/GitDownloaderTest.php:253-261` + `tests/Composer/Test/Downloader/GitDownloaderTest.php:390-398` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/DownloadManagerTest.php:1059-1066` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:871-878` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/Console/Input/InputArgument.php:31-69` + `src/Composer/Console/Input/InputOption.php:31-72` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Repository/ComposerRepositoryTest.php:1093-1104` + `tests/Composer/Test/Repository/ComposerRepositoryTest.php:734-745` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Downloader/DownloadManagerTest.php:260-332` + `tests/Composer/Test/Downloader/DownloadManagerTest.php:820-903` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:298-305` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:327-334` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Autoload/Fixtures/autoload_psr4.php:1-11` + `tests/Composer/Test/Autoload/Fixtures/autoload_psr4_2.php:1-11` | ✅ pass | passing |
| composer | clearly_in | `src/Composer/FilterList/FilterListEntryBuilder.php:69-77` + `src/Composer/FilterList/FilterListProvider/FilterListProviderSet.php:146-156` | ✅ pass | passing |
| composer | clearly_in | `tests/Composer/Test/Autoload/Fixtures/autoload_static_functions.php:1-28` + `tests/Composer/Test/Autoload/Fixtures/autoload_static_functions_with_include_paths.php:1-28` | ✅ pass | passing |
| composer | clearly_out | `tests/Composer/Test/EventDispatcher/EventDispatcherTest.php:25-29` + `tests/Composer/Test/Package/Loader/ValidatingArrayLoaderTest.php:15-19` | ✅ pass | passing |
| composer | clearly_out | `tests/Composer/Test/DependencyResolver/PoolTest.php:1-19` + `tests/Composer/Test/Policy/MalwarePolicyConfigTest.php:1-19` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Plugin/PrePoolCreateEvent.php:30-62` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:56-94` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Plugin/PrePoolCreateEvent.php:34-62` + `src/Composer/Util/RemoteFilesystem.php:34-46` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Command/RequireCommand.php:56-70` + `src/Composer/Util/RemoteFilesystem.php:34-48` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Installer/PackageEvent.php:41-78` + `src/Composer/Plugin/PreFileDownloadEvent.php:62-74` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Repository/Vcs/GitHubDriver.php:267-281` + `src/Composer/Repository/Vcs/GitHubDriver.php:282-296` | 🔴 fail | on record as failing |
| composer | clearly_out | `src/Composer/Command/RequireCommand.php:92-94` + `src/Composer/Command/UpdateCommand.php:65-67` | 🔴 fail | on record as failing |
| composer | clearly_out | `tests/Composer/Test/Downloader/HgDownloaderTest.php:67-69` + `tests/Composer/Test/Installer/SuggestedPackagesReporterTest.php:226-228` | 🔴 fail | on record as failing |
| composer | clearly_out | `src/Composer/Command/CheckPlatformReqsCommand.php:32-33` + `src/Composer/Command/LicensesCommand.php:41-42` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Command/CreateProjectCommand.php:90-91` + `src/Composer/Command/InitCommand.php:56-57` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Package/Dumper/ArrayDumper.php:116-126` + `tests/Composer/Test/Util/PerforceTest.php:131-131` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Command/BumpCommand.php:48-49` + `src/Composer/Command/SearchCommand.php:40-41` | ✅ pass | passing |
| composer | clearly_out | `tests/Composer/Test/DependencyResolver/SolverTest.php:741-742` + `tests/Composer/Test/DependencyResolver/SolverTest.php:833-838` | 🔴 fail | on record as failing |
| composer | clearly_out | `src/Composer/FilterList/FilterListEntry.php:27-57` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:61-94` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/DependencyResolver/PoolBuilder.php:47-78` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:56-94` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/DependencyResolver/Solver.php:32-45` + `src/Composer/Package/Loader/ValidatingArrayLoader.php:39-49` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Advisory/SecurityAdvisory.php:24-54` + `tests/Composer/Test/Autoload/AutoloadGeneratorTest.php:66-94` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Command/BaseCommand.php:34-39` + `src/Composer/IO/BufferIO.php:16-21` | ✅ pass | passing |
| composer | clearly_out | `tests/Composer/Test/Util/FilesystemTest.php:68-70` + `tests/Composer/Test/Util/FilesystemTest.php:98-100` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Command/InitCommand.php:56-57` + `src/Composer/Command/ValidateCommand.php:44-45` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Installer.php:104-140` + `src/Composer/Repository/VcsRepository.php:46-60` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/DependencyResolver/PoolOptimizer.php:34-59` + `src/Composer/DependencyResolver/Request.php:44-54` | ✅ pass | passing |
| composer | clearly_out | `src/Composer/Config.php:115-129` + `src/Composer/Repository/VcsRepository.php:58-72` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:135-147` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:97-109` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:214-230` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:267-283` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/InputArgumentTest.php:149-180` + `Tests/Input/InputOptionTest.php:319-350` | ✅ pass | passing |
| console | clearly_in | `Tests/ApplicationTest.php:2348-2399` + `Tests/ApplicationTest.php:2401-2452` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:136-149` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:169-182` | ✅ pass | passing |
| console | clearly_in | `Tests/ApplicationTest.php:1792-1803` + `Tests/ApplicationTest.php:2036-2047` | ✅ pass | passing |
| console | clearly_in | `Tests/CommandLoader/ContainerCommandLoaderTest.php:1-60` + `Tests/CommandLoader/FactoryCommandLoaderTest.php:1-59` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:236-282` + `Tests/ArgumentResolver/ValueResolver/VariadicValueResolverTest.php:98-138` | ✅ pass | passing |
| console | clearly_in | `Descriptor/MarkdownDescriptor.php:31-78` + `Descriptor/ReStructuredTextDescriptor.php:40-92` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/FormatterHelperTest.php:56-67` + `Tests/Helper/FormatterHelperTest.php:69-79` | ✅ pass | passing |
| console | clearly_in | `Tests/Style/SymfonyStyleTest.php:282-296` + `Tests/Style/SymfonyStyleTest.php:396-411` | ✅ pass | passing |
| console | clearly_in | `Tests/Tester/CommandTesterTest.php:212-239` + `Tests/Tester/CommandTesterTest.php:241-267` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/QuestionHelperTest.php:740-751` + `Tests/Helper/QuestionHelperTest.php:759-770` | ✅ pass | passing |
| console | clearly_in | `Exception/InvalidArgumentException.php:1-40` + `Exception/InvalidOptionException.php:1-42` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/TreeHelperTest.php:262-271` + `Tests/Helper/TreeHelperTest.php:66-75` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:36-49` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:362-375` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:263-287` + `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:289-317` | ✅ pass | passing |
| console | clearly_in | `Tests/Terminal/Image/ITerm2ProtocolTest.php:81-108` + `Tests/Terminal/Image/KittyGraphicsProtocolTest.php:70-106` | ✅ pass | passing |
| console | clearly_in | `Tests/Completion/Output/FishCompletionOutputTest.php:1-33` + `Tests/Completion/Output/ZshCompletionOutputTest.php:1-33` | 🔴 fail | on record as failing |
| console | clearly_in | `Tests/Helper/TableTest.php:1853-1879` + `Tests/Helper/TableTest.php:1881-1914` | ✅ pass | passing |
| console | clearly_in | `Tests/ApplicationTest.php:2314-2327` + `Tests/ApplicationTest.php:2810-2824` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/TableTest.php:1328-1345` + `Tests/Helper/TableTest.php:1396-1413` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:502-510` + `Tests/Helper/ProgressBarTest.php:911-919` | ✅ pass | passing |
| console | clearly_in | `Completion/Output/FishCompletionOutput.php:1-36` + `Completion/Output/ZshCompletionOutput.php:1-36` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/DefaultValueResolverTest.php:51-79` + `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:60-94` | ✅ pass | passing |
| console | clearly_in | `Event/ConsoleAlarmEvent.php:1-47` + `Event/ConsoleSignalEvent.php:1-56` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/QuestionHelperTest.php:1117-1131` + `Tests/Helper/QuestionHelperTest.php:1158-1168` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:237-352` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:211-300` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/TableTest.php:1532-1540` + `Tests/Helper/TableTest.php:1855-1863` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/AddConsoleCommandPassTest.php:427-437` + `Tests/DependencyInjection/AddConsoleCommandPassTest.php:475-485` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:195-209` + `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:220-234` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:118-131` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:165-178` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:161-173` + `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:36-48` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/InputFileValueResolverTest.php:50-109` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:260-326` | ✅ pass | passing |
| console | clearly_in | `Tests/SignalRegistry/SignalRegistryTest.php:185-207` + `Tests/SignalRegistry/SignalRegistryTest.php:209-231` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/File/InputFileTest.php:137-147` + `Tests/Input/File/InputFileTest.php:149-159` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:1236-1243` + `Tests/Helper/ProgressBarTest.php:834-841` | ✅ pass | passing |
| console | clearly_in | `Tests/Terminal/Image/ITerm2ProtocolTest.php:19-47` + `Tests/Terminal/Image/KittyGraphicsProtocolTest.php:19-48` | ✅ pass | passing |
| console | clearly_in | `Tests/Descriptor/MarkdownDescriptorTest.php:1-45` + `Tests/Descriptor/ReStructuredTextDescriptorTest.php:1-45` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/DateTimeValueResolverTest.php:245-265` + `Tests/ArgumentResolver/ValueResolver/DateTimeValueResolverTest.php:97-117` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:140-161` + `Tests/ArgumentResolver/ValueResolver/UidValueResolverTest.php:163-184` | ✅ pass | passing |
| console | clearly_in | `Tests/ApplicationTest.php:2335-2345` + `Tests/ApplicationTest.php:2851-2861` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/ArgvInputTest.php:664-755` + `Tests/Input/ArrayInputTest.php:342-449` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:1817-1825` + `Tests/Helper/ProgressBarTest.php:1840-1850` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/InputOptionTest.php:91-97` + `Tests/Input/InputOptionTest.php:98-104` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:116-125` + `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:59-68` | 🔴 fail | on record as failing |
| console | clearly_in | `Tests/Helper/TreeHelperTest.php:119-131` + `Tests/Helper/TreeHelperTest.php:133-145` | ✅ pass | passing |
| console | clearly_in | `Tests/ColorTest.php:36-49` + `Tests/ColorTest.php:51-64` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/TreeHelperTest.php:103-116` + `Tests/Helper/TreeHelperTest.php:50-60` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/RegisterCommandArgumentLocatorsPassTest.php:347-354` + `Tests/DependencyInjection/RegisterCommandArgumentLocatorsPassTest.php:365-372` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/InputArgumentTest.php:139-170` + `Tests/Input/InputOptionTest.php:301-340` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:587-594` + `Tests/Helper/ProgressBarTest.php:612-619` | ✅ pass | passing |
| console | clearly_in | `Tests/DependencyInjection/ConsoleArgumentValueResolverPassTest.php:25-54` + `Tests/DependencyInjection/ConsoleArgumentValueResolverPassTest.php:56-91` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/TreeHelperTest.php:259-282` + `Tests/Helper/TreeHelperTest.php:63-86` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/ArgvInputTest.php:848-907` + `Tests/Input/ArrayInputTest.php:522-595` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/BackedEnumValueResolverTest.php:262-352` + `Tests/ArgumentResolver/ValueResolver/VariadicValueResolverTest.php:50-138` | ✅ pass | passing |
| console | clearly_in | `Attribute/Argument.php:86-112` + `Attribute/Option.php:113-146` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:1682-1690` + `Tests/Helper/ProgressBarTest.php:1867-1875` | ✅ pass | passing |
| console | clearly_in | `Tests/Input/ArrayInputTest.php:542-550` + `Tests/Input/ArrayInputTest.php:552-560` | ✅ pass | passing |
| console | clearly_in | `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:241-257` + `Tests/ArgumentResolver/ValueResolver/MapInputValueResolverTest.php:267-283` | ✅ pass | passing |
| console | clearly_in | `Tester/Constraint/CommandFailed.php:1-43` + `Tester/Constraint/CommandIsInvalid.php:1-43` | ✅ pass | passing |
| console | clearly_in | `Tests/Helper/ProgressBarTest.php:330-339` + `Tests/Helper/ProgressBarTest.php:348-357` | ✅ pass | passing |
| console | clearly_out | `Helper/Table.php:34-39` + `Output/OutputInterface.php:23-28` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/ru/_lib/formatDistance/index.ts:116-252` + `pkgs/core/src/locale/uk/_lib/formatDistance/index.ts:110-246` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/differenceInQuarters/test.ts:82-149` + `pkgs/core/src/differenceInWeeks/test.ts:83-150` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/getWeek/test.ts:47-83` + `pkgs/core/src/getWeekOfMonth/test.ts:71-102` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/ar-EG/_lib/match/index.ts:94-107` + `pkgs/core/src/locale/ja/_lib/match/index.ts:77-90` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/tz/playground.mjs:112-120` + `pkgs/tz/playground.mjs:59-67` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/bs/_lib/match/index.ts:47-60` + `pkgs/core/src/locale/de/_lib/match/index.ts:48-61` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/gl/_lib/formatDistance/index.ts:10-110` + `pkgs/core/src/locale/mk/_lib/formatDistance/index.ts:10-110` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/pt/_lib/formatDistance/index.ts:10-87` + `pkgs/core/src/locale/ro/_lib/formatDistance/index.ts:10-87` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/addDays/test.ts:48-59` + `pkgs/core/src/subISOWeekYears/test.ts:63-74` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/isBefore/test.ts:37-50` + `pkgs/core/src/isSameHour/test.ts:31-44` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/cy/_lib/match/index.ts:1-143` + `pkgs/core/src/locale/pt-BR/_lib/match/index.ts:1-135` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/ja/_lib/match/index.ts:76-120` + `pkgs/core/src/locale/tr/_lib/match/index.ts:107-151` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/be/_lib/formatRelative/index.ts:1-107` + `pkgs/core/src/locale/bg/_lib/formatRelative/index.ts:21-113` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/_lib/parsers/Hour0to23Parser.ts:8-26` + `pkgs/core/src/parse/_lib/parsers/SecondParser.ts:8-26` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/ar-TN/_lib/match/index.ts:33-82` + `pkgs/core/src/locale/ar/_lib/match/index.ts:33-82` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/getDaysInMonth/test.ts:23-58` + `pkgs/core/src/getISOWeek/test.ts:44-75` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/tz/src/date/tests.ts:1696-1707` + `pkgs/tz/src/date/tests.ts:1709-1720` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/test.ts:530-593` + `pkgs/core/src/parse/test.ts:595-658` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/_lib/utils.ts:77-93` + `pkgs/core/src/parse/_lib/utils.ts:95-111` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/test.ts:1016-1039` + `pkgs/core/src/parse/test.ts:944-967` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/_lib/format/formatters/index.ts:232-258` + `pkgs/core/src/_lib/format/formatters/index.ts:261-290` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/az/_lib/match/index.ts:27-82` + `pkgs/core/src/locale/ca/_lib/match/index.ts:29-85` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/de/_lib/match/index.ts:80-135` + `pkgs/core/src/locale/fr/_lib/match/index.ts:79-134` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/da/_lib/match/index.ts:18-89` + `pkgs/core/src/locale/en-US/_lib/match/index.ts:18-89` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/tz/playground.mjs:112-118` + `pkgs/tz/playground.mjs:28-34` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/_lib/parsers/MonthParser.ts:23-90` + `pkgs/core/src/parse/_lib/parsers/StandAloneMonthParser.ts:8-75` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/zh-HK/test.ts:1-33` + `pkgs/core/src/locale/zh-TW/test.ts:1-33` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/setISOWeekYear/test.ts:36-91` + `pkgs/core/src/subISOWeekYears/test.ts:29-82` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/lt/_lib/localize/index.ts:111-233` + `pkgs/core/src/locale/lv/_lib/localize/index.ts:119-249` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/test.ts:290-297` + `pkgs/core/src/parse/test.ts:631-638` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/differenceInCalendarMonths/test.ts:7-98` + `pkgs/core/src/differenceInCalendarYears/test.ts:7-98` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/differenceInQuarters/index.ts:36-43` + `pkgs/core/src/differenceInWeeks/index.tp.ts:6-13` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/bg/_lib/match/index.ts:1-128` + `pkgs/core/src/locale/mk/_lib/match/index.ts:1-126` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/areIntervalsOverlapping/test.ts:33-64` + `pkgs/core/src/getOverlappingDaysInIntervals/test.ts:42-76` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/tz/src/date/tests.ts:1365-1376` + `pkgs/tz/src/date/tests.ts:1378-1393` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/eu/_lib/match/index.ts:128-162` + `pkgs/core/src/locale/fr/_lib/match/index.ts:99-133` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/endOfHour/index.ts:1-50` + `pkgs/core/src/startOfHour/index.ts:1-43` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/endOfQuarter/index.ts:7-53` + `pkgs/core/src/lastDayOfQuarter/index.ts:7-46` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/setMonth/test.ts:7-84` + `pkgs/core/src/setQuarter/test.ts:7-84` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/isSameISOWeek/test.ts:7-59` + `pkgs/core/src/isSameQuarter/test.ts:7-59` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/_lib/parsers/ISOWeekParser.ts:13-22` + `pkgs/core/src/parse/_lib/parsers/LocalWeekParser.ts:13-22` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/examples/cts/test.ts:1-20` + `pkgs/core/examples/ts/test.ts:5-20` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/is/_lib/match/index.ts:14-79` + `pkgs/core/src/locale/mt/_lib/match/index.ts:14-86` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/lt/_lib/localize/index.ts:198-218` + `pkgs/core/src/locale/vi/_lib/localize/index.ts:270-290` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/examples/cdn/legacy.js:1-36` + `pkgs/core/examples/cdn/legacyStrict.js:1-47` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/constructFrom/test.ts:10-17` + `pkgs/core/src/constructFrom/test.ts:32-39` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/ca/_lib/formatDistance/index.ts:105-116` + `pkgs/core/src/locale/cy/_lib/formatDistance/index.ts:102-113` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/differenceInCalendarQuarters/test.ts:31-98` + `pkgs/core/src/differenceInCalendarYears/test.ts:31-98` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/format/test.ts:132-151` + `pkgs/core/src/format/test.ts:244-263` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/nextTuesday/test.ts:42-52` + `pkgs/core/src/roundToNearestMinutes/test.ts:264-274` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/zh-HK/_lib/localize/index.ts:1-197` + `pkgs/core/src/locale/zh-TW/_lib/localize/index.ts:1-197` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/pt-BR/_lib/localize/index.ts:10-19` + `pkgs/core/src/locale/pt/_lib/localize/index.ts:10-19` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/zh-HK/_lib/formatDistance/index.ts:73-86` + `pkgs/core/src/locale/zh-TW/_lib/formatDistance/index.ts:73-86` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/isLastDayOfMonth/test.ts:7-15` + `pkgs/core/src/isTomorrow/test.ts:11-19` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/test.ts:399-427` + `pkgs/core/src/parse/test.ts:531-559` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/test.ts:1011-1044` + `pkgs/core/src/parse/test.ts:1090-1123` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/lastDayOfWeek/index.ts:41-46` + `pkgs/core/src/parse/index.ts:374-379` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/endOfISOWeekYear/test.ts:13-40` + `pkgs/core/src/startOfWeekYear/test.ts:13-40` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/parse/_lib/parsers/ISOTimezoneParser.ts:1-50` + `pkgs/core/src/parse/_lib/parsers/ISOTimezoneWithZParser.ts:1-50` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/eachWeekendOfMonth/test.ts:41-54` + `pkgs/core/src/eachWeekendOfMonth/test.ts:55-68` | ✅ pass | passing |
| date-fns | clearly_in | `pkgs/core/src/locale/kk/_lib/formatRelative/index.ts:52-64` + `pkgs/core/src/locale/mk/_lib/formatRelative/index.ts:82-94` | 🔴 fail | on record as failing |
| deslop | clearly_in | `crates/deslop/tests/python_issue_104_module_preamble.rs:24-33` + `crates/deslop/tests/python_issue_133_constant_table.rs:45-54` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:243-251` + `crates/deslop-lsp/tests/observability_heartbeat.rs:237-245` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:253-258` + `crates/deslop-lsp/tests/observability_heartbeat.rs:248-253` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-test-support/src/skip_policy.rs:394-400` + `crates/deslop-test-support/src/test_target_parity/suite_scan.rs:190-196` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-core/src/pipeline/corpus.rs:128-133` + `crates/deslop-core/src/pipeline/session/mod.rs:459-464` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:218-222` + `crates/deslop-lsp/tests/observability_heartbeat.rs:231-235` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop/tests/common/corpora.rs:79-84` + `crates/deslop/tests/common/corpora.rs:93-98` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-core/tests/content_gate_admits.rs:63-76` + `crates/deslop-core/tests/content_gate_rejects.rs:47-60` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-lsp/tests/cpu_throttle_knob.rs:260-268` + `crates/deslop-lsp/tests/observability_heartbeat.rs:255-263` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-mcp/tests/cli.rs:1801-1808` + `crates/deslop-mcp/tests/cli.rs:2761-2768` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-core/src/cluster_filters/structural_families/tests.rs:305-314` + `crates/deslop-core/src/cluster_filters/structural_families/tests.rs:357-366` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-core/src/refactor/emit.rs:108-113` + `crates/deslop-core/src/refactor/merge/mod.rs:245-250` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop/tests/cli/thresholds.rs:131-137` + `crates/deslop/tests/cli/thresholds.rs:161-167` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-core/src/live/session.rs:753-759` + `crates/deslop-core/src/live/session.rs:779-785` | ✅ pass | passing |
| deslop | clearly_in | `crates/deslop-lsp/tests/cli.rs:324-330` + `crates/deslop/tests/cli/invocation.rs:29-35` | ✅ pass | passing |
| deslop | clearly_out | `crates/deslop-core/src/cluster_filters/calls/args.rs:134-139` + `crates/deslop-core/src/cluster_filters/calls/asserts.rs:164-168` | ✅ pass | passing |
| dio | clearly_in | `dio/test/interceptor_test.dart:1541-1583` + `dio/test/interceptor_test.dart:1585-1628` | ✅ pass | passing |
| dio | clearly_out | `dio_test/lib/src/test/headers_tests.dart:49-51` + `plugins/web_adapter/test/cors_preflight_test.dart:203-206` | ✅ pass | passing |
| dio | clearly_out | `dio/lib/src/dio_exception.dart:214-228` + `dio/lib/src/options.dart:508-529` | ✅ pass | passing |
| dio | clearly_out | `dio/lib/src/options.dart:680-694` + `dio/lib/src/response.dart:33-49` | ✅ pass | passing |
| django | clearly_in | `tests/timezones/tests.py:212-230` + `tests/timezones/tests.py:493-511` | ✅ pass | passing |
| django | clearly_in | `tests/model_meta/results.py:356-403` + `tests/model_meta/results.py:482-529` | ✅ pass | passing |
| django | clearly_in | `tests/migrations/test_operations.py:4319-4343` + `tests/migrations/test_operations.py:4388-4412` | ✅ pass | passing |
| django | clearly_in | `django/core/management/commands/sqlflush.py:12-24` + `django/core/management/commands/sqlsequencereset.py:10-22` | ✅ pass | passing |
| django | clearly_in | `tests/flatpages_tests/test_csrf.py:72-105` + `tests/flatpages_tests/test_middleware.py:71-104` | ✅ pass | passing |
| django | clearly_in | `tests/flatpages_tests/test_csrf.py:32-67` + `tests/flatpages_tests/test_middleware.py:17-52` | ✅ pass | passing |
| django | clearly_in | `tests/flatpages_tests/test_templatetags.py:13-47` + `tests/flatpages_tests/test_views.py:15-49` | ✅ pass | passing |
| django | clearly_in | `django/db/models/fields/related_descriptors.py:297-309` + `django/db/models/fields/related_descriptors.py:578-590` | ✅ pass | passing |
| django | clearly_in | `tests/migrations/test_commands.py:1895-1909` + `tests/migrations/test_commands.py:1973-1987` | ✅ pass | passing |
| django | clearly_in | `tests/admin_widgets/tests.py:1442-1452` + `tests/admin_widgets/tests.py:1455-1465` | ✅ pass | passing |
| django | clearly_in | `tests/raw_query/tests.py:249-256` + `tests/raw_query/tests.py:260-267` | ✅ pass | passing |
| django | clearly_in | `tests/backends/tests.py:638-645` + `tests/backends/tests.py:719-726` | ✅ pass | passing |
| django | clearly_in | `tests/multiple_database/tests.py:922-929` + `tests/multiple_database/tests.py:939-946` | ✅ pass | passing |
| django | clearly_in | `tests/model_meta/results.py:546-563` + `tests/model_meta/results.py:588-605` | ✅ pass | passing |
| django | clearly_in | `tests/servers/tests.py:265-271` + `tests/servers/tests.py:273-279` | ✅ pass | passing |
| django | clearly_out | `tests/admin_filters/tests.py:1721-1723` + `tests/forms_tests/widget_tests/test_choicewidget.py:53-56` | ✅ pass | passing |
| django | clearly_out | `tests/requests_tests/tests.py:1472-1474` + `tests/test_client_regress/tests.py:1410-1412` | ✅ pass | passing |
| django | clearly_out | `tests/runtests.py:548-558` + `tests/runtests.py:650-660` | ✅ pass | passing |
| django | clearly_out | `django/contrib/gis/management/commands/ogrinspect.py:100-109` + `tests/runtests.py:669-679` | ✅ pass | passing |
| django | clearly_out | `tests/validators/tests.py:343-347` + `tests/validators/tests.py:367-371` | ✅ pass | passing |
| django | clearly_out | `tests/responses/test_cookie.py:132-135` + `tests/template_backends/test_jinja2.py:129-132` | ✅ pass | passing |
| django | clearly_out | `tests/auth_tests/test_views.py:168-170` + `tests/middleware/tests.py:405-407` | ✅ pass | passing |
| django | clearly_out | `tests/forms_tests/field_tests/test_decimalfield.py:64-81` + `tests/humanize_tests/tests.py:64-81` | ✅ pass | passing |
| django | clearly_out | `django/db/migrations/operations/models.py:818-830` + `tests/utils_tests/test_dateformat.py:200-200` | ✅ pass | passing |
| django | clearly_out | `tests/invalid_models_tests/test_relative_fields.py:313-318` + `tests/migrations/migrations_test_apps/lookuperror_a/models.py:8-18` | 🔴 fail | on record as failing |
| django | clearly_out | `tests/migrations/test_commands.py:1363-1365` + `tests/test_utils/tests.py:725-735` | ✅ pass | passing |
| django | clearly_out | `tests/admin_views/tests.py:827-828` + `tests/custom_lookups/tests.py:656-657` | ✅ pass | passing |
| django | clearly_out | `django/contrib/staticfiles/management/commands/collectstatic.py:40-90` + `tests/runtests.py:644-688` | ✅ pass | passing |
| express | clearly_in | `test/res.cookie.js:189-201` + `test/res.cookie.js:217-229` | ✅ pass | passing |
| express | clearly_in | `test/app.response.js:111-141` + `test/app.response.js:79-109` | ✅ pass | passing |
| express | clearly_in | `test/express.raw.js:179-185` + `test/express.raw.js:466-472` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:143-173` + `test/express.urlencoded.js:263-293` | ✅ pass | passing |
| express | clearly_in | `test/express.static.js:265-325` + `test/express.static.js:340-400` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:95-169` + `test/express.urlencoded.js:241-314` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:242-252` + `test/express.urlencoded.js:111-121` | ✅ pass | passing |
| express | clearly_in | `test/app.router.js:1071-1093` + `test/app.router.js:966-983` | ✅ pass | passing |
| express | clearly_in | `test/req.subdomains.js:12-19` + `test/req.subdomains.js:84-91` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:60-70` + `test/express.json.js:77-87` | ✅ pass | passing |
| express | clearly_in | `test/req.hostname.js:47-71` + `test/req.hostname.js:8-32` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:617-626` + `test/res.sendFile.js:637-646` | ✅ pass | passing |
| express | clearly_in | `test/express.raw.js:86-187` + `test/express.text.js:95-199` | ✅ pass | passing |
| express | clearly_in | `test/res.status.js:106-116` + `test/res.status.js:34-44` | ✅ pass | passing |
| express | clearly_in | `test/app.param.js:306-320` + `test/app.param.js:71-85` | ✅ pass | passing |
| express | clearly_in | `test/express.urlencoded.js:317-370` + `test/express.urlencoded.js:372-425` | ✅ pass | passing |
| express | clearly_in | `test/app.router.js:437-449` + `test/app.router.js:491-503` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:186-193` + `test/express.urlencoded.js:306-313` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:476-487` + `test/express.raw.js:285-296` | ✅ pass | passing |
| express | clearly_in | `test/express.urlencoded.js:139-153` + `test/express.urlencoded.js:93-107` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:196-224` + `test/express.text.js:171-199` | ✅ pass | passing |
| express | clearly_in | `test/req.hostname.js:60-71` + `test/req.hostname.js:8-19` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:490-495` + `test/app.use.js:497-502` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:496-545` + `test/express.urlencoded.js:735-784` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:10-20` + `test/express.urlencoded.js:11-21` | ✅ pass | passing |
| express | clearly_in | `test/res.append.js:109-118` + `test/res.append.js:17-26` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:578-594` + `test/res.sendFile.js:596-612` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:297-391` + `test/express.text.js:201-293` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:506-534` + `test/express.raw.js:330-359` | ✅ pass | passing |
| express | clearly_in | `test/app.render.js:418-446` + `test/app.render.js:448-476` | ✅ pass | passing |
| express | clearly_in | `test/acceptance/mvc.js:103-114` + `test/acceptance/mvc.js:33-44` | ✅ pass | passing |
| express | clearly_in | `test/res.redirect.js:131-144` + `test/res.redirect.js:179-192` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:400-450` + `test/express.urlencoded.js:646-696` | ✅ pass | passing |
| express | clearly_in | `test/express.raw.js:141-146` + `test/express.text.js:151-156` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:361-419` + `test/express.urlencoded.js:607-665` | ✅ pass | passing |
| express | clearly_in | `test/req.ip.js:10-38` + `test/req.ips.js:10-38` | ✅ pass | passing |
| express | clearly_in | `test/req.fresh.js:9-49` + `test/req.stale.js:8-48` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:255-294` + `test/express.urlencoded.js:111-145` | ✅ pass | passing |
| express | clearly_in | `test/req.is.js:149-191` + `test/req.is.js:193-235` | ✅ pass | passing |
| express | clearly_in | `test/req.query.js:65-73` + `test/req.query.js:75-83` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:504-574` + `test/express.urlencoded.js:607-676` | ✅ pass | passing |
| express | clearly_in | `test/req.host.js:79-87` + `test/req.host.js:95-103` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:367-389` + `test/express.raw.js:253-275` | ✅ pass | passing |
| express | clearly_in | `test/app.request.js:111-141` + `test/app.request.js:79-109` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:473-481` + `test/app.use.js:92-100` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:229-255` + `test/app.use.js:420-446` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:634-646` + `test/express.text.js:458-470` | ✅ pass | passing |
| express | clearly_in | `test/express.static.js:598-610` + `test/express.static.js:619-631` | ✅ pass | passing |
| express | clearly_in | `test/express.raw.js:195-207` + `test/express.raw.js:224-236` | ✅ pass | passing |
| express | clearly_in | `test/req.accepts.js:20-31` + `test/req.accepts.js:33-44` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:394-445` + `test/express.urlencoded.js:525-576` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:230-240` + `test/app.use.js:297-307` | ✅ pass | passing |
| express | clearly_in | `test/express.raw.js:299-306` + `test/express.urlencoded.js:545-552` | ✅ pass | passing |
| express | clearly_in | `test/res.location.js:183-214` + `test/res.location.js:260-291` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:78-84` + `test/express.text.js:86-92` | ✅ pass | passing |
| express | clearly_in | `test/app.router.js:119-129` + `test/app.router.js:95-105` | 🔴 fail | on record as failing |
| express | clearly_in | `test/res.sendFile.js:541-555` + `test/res.sendFile.js:765-779` | ✅ pass | passing |
| express | clearly_in | `test/res.attachment.js:78-90` + `test/res.attachment.js:92-104` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:522-538` + `test/res.sendFile.js:693-709` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:607-647` + `test/express.text.js:453-493` | ✅ pass | passing |
| express | clearly_in | `test/app.router.js:704-740` + `test/app.router.js:742-778` | ✅ pass | passing |
| express | clearly_in | `test/res.attachment.js:106-117` + `test/res.attachment.js:24-35` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:144-149` + `test/app.use.js:193-198` | 🔴 fail | on record as failing |
| express | clearly_in | `test/req.accepts.js:73-84` + `test/req.accepts.js:99-110` | ✅ pass | passing |
| express | clearly_in | `test/app.router.js:861-868` + `test/app.router.js:892-899` | ✅ pass | passing |
| express | clearly_in | `test/res.render.js:245-273` + `test/res.render.js:91-119` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:495-546` + `test/express.urlencoded.js:734-785` | ✅ pass | passing |
| express | clearly_in | `test/acceptance/vhost.js:25-34` + `test/acceptance/vhost.js:36-45` | ✅ pass | passing |
| express | clearly_in | `test/res.send.js:151-162` + `test/res.send.js:194-205` | ✅ pass | passing |
| express | clearly_in | `test/req.range.js:21-32` + `test/req.range.js:8-19` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:365-380` + `test/app.use.js:393-408` | ✅ pass | passing |
| express | clearly_in | `test/res.render.js:189-205` + `test/res.render.js:207-223` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:172-184` + `test/express.urlencoded.js:226-238` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:106-123` + `test/express.urlencoded.js:295-313` | ✅ pass | passing |
| express | clearly_in | `test/app.param.js:275-282` + `test/app.param.js:99-106` | 🔴 fail | on record as failing |
| express | clearly_in | `test/req.host.js:74-119` + `test/req.hostname.js:74-119` | ✅ pass | passing |
| express | clearly_in | `test/app.use.js:372-382` + `test/app.use.js:400-410` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:515-540` + `test/express.text.js:372-397` | 🔴 fail | on record as failing |
| express | clearly_in | `test/express.json.js:394-428` + `test/express.urlencoded.js:525-559` | ✅ pass | passing |
| express | clearly_in | `test/express.json.js:90-194` + `test/express.urlencoded.js:211-314` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:693-709` + `test/res.sendFile.js:711-727` | ✅ pass | passing |
| express | clearly_in | `test/res.append.js:65-83` + `test/res.append.js:9-27` | ✅ pass | passing |
| express | clearly_in | `test/app.param.js:182-193` + `test/app.param.js:203-214` | ✅ pass | passing |
| express | clearly_in | `test/res.jsonp.js:164-171` + `test/res.jsonp.js:335-342` | 🔴 fail | on record as failing |
| express | clearly_in | `test/express.text.js:207-221` + `test/express.urlencoded.js:434-448` | ✅ pass | passing |
| express | clearly_in | `test/app.request.js:111-141` + `test/app.request.js:25-49` | ✅ pass | passing |
| express | clearly_in | `test/express.text.js:432-450` + `test/express.urlencoded.js:678-696` | ✅ pass | passing |
| express | clearly_in | `test/express.urlencoded.js:31-41` + `test/express.urlencoded.js:66-76` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:952-964` + `test/res.sendFile.js:966-978` | ✅ pass | passing |
| express | clearly_in | `test/res.sendFile.js:471-484` + `test/res.sendFile.js:504-517` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_form/test_nullable_and_defaults.py:127-139` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:64-76` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_response_model_include_exclude.py:116-124` + `tests/test_response_model_include_exclude.py:147-153` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_file/test_optional_list.py:27-56` + `tests/test_request_params/test_file/test_optional_list.py:296-325` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_form/test_optional_str.py:163-214` + `tests/test_request_params/test_form/test_optional_str.py:87-134` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_api_key_cookie.py:7-23` + `tests/test_security_api_key_query.py:7-23` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_body/test_optional_list.py:115-177` + `tests/test_request_params/test_body/test_optional_list.py:27-86` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_path.py:527-561` + `tests/test_path.py:703-737` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_cookie/test_required_str.py:297-378` + `tests/test_request_params/test_cookie/test_required_str.py:92-158` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_http_basic_realm.py:10-49` + `tests/test_security_http_basic_realm_description.py:8-49` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_path_operation_configurations/test_tutorial001.py:88-188` + `tests/test_tutorial/test_path_operation_configurations/test_tutorial003_tutorial004.py:109-209` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_security/test_tutorial004.py:186-372` + `tests/test_tutorial/test_security/test_tutorial005.py:230-432` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_include_router_defaults_overrides.py:160-178` + `tests/test_include_router_defaults_overrides.py:181-199` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_multipart_installation.py:102-112` + `tests/test_multipart_installation.py:89-99` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_body/test_optional_list.py:104-188` + `tests/test_request_params/test_body/test_optional_list.py:206-299` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_cookie/test_optional_str.py:165-215` + `tests/test_request_params/test_cookie/test_optional_str.py:250-306` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_path.py:290-325` + `tests/test_path.py:505-540` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_oauth2.py:36-54` + `tests/test_security_openid_connect.py:21-38` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/param_functions.py:1066-1383` + `fastapi/param_functions.py:375-731` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_path.py:625-635` + `tests/test_path.py:721-731` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/security/oauth2.py:442-534` + `fastapi/security/oauth2.py:553-640` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_query_params_str_validations/test_tutorial007.py:10-54` + `tests/test_tutorial/test_query_params_str_validations/test_tutorial008.py:10-54` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_file/test_list.py:269-306` + `tests/test_request_params/test_file/test_list.py:416-453` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_body/test_list.py:410-419` + `tests/test_request_params/test_form/test_list.py:413-422` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_strict_content_type_nested.py:8-19` + `tests/test_strict_content_type_router_level.py:4-18` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_dependencies/test_tutorial002_tutorial003_tutorial004.py:10-79` + `tests/test_tutorial/test_dependencies/test_tutorial011.py:8-45` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_path.py:558-592` + `tests/test_path.py:644-678` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_form/test_nullable_and_defaults.py:22-48` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:54-80` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_body/test_required_str.py:26-67` + `tests/test_request_params/test_body/test_required_str.py:299-353` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_query_params_str_validations/test_tutorial002.py:63-147` + `tests/test_tutorial/test_query_params_str_validations/test_tutorial006c.py:67-148` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_cookie/test_optional_str.py:60-69` + `tests/test_request_params/test_cookie/test_required_str.py:67-76` | ✅ pass | passing |
| fastapi | clearly_in | `scripts/tests/test_translation_fixer/test_code_blocks/test_code_blocks_lines_number_mismatch.py:13-34` + `scripts/tests/test_translation_fixer/test_code_blocks/test_code_blocks_wrong_lang_code.py:37-58` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_api_key_query_optional.py:45-73` + `tests/test_security_http_basic_realm_description.py:52-84` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_include_router_defaults_overrides.py:1757-1931` + `tests/test_include_router_defaults_overrides.py:654-828` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_include_router_defaults_overrides.py:377-394` + `tests/test_include_router_defaults_overrides.py:414-431` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_query/test_optional_list.py:215-237` + `tests/test_request_params/test_query/test_optional_list.py:318-348` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_dependency_wrapped.py:267-317` + `tests/test_dependency_wrapped.py:349-401` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_nullable_and_defaults.py:263-277` + `tests/test_request_params/test_nullable_and_defaults.py:633-647` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_header/test_required_str.py:259-271` + `tests/test_request_params/test_header/test_required_str.py:399-411` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/security/tutorial007_an_py310.py:12-36` + `docs_src/security/tutorial007_py310.py:11-33` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_header_param_models/test_tutorial001.py:26-36` + `tests/test_tutorial/test_header_param_models/test_tutorial003.py:47-57` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_dependency_overrides.py:51-63` + `tests/test_query.py:8-20` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_api_key_cookie_description.py:7-23` + `tests/test_security_api_key_query_description.py:7-23` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_response_model/test_tutorial005.py:10-26` + `tests/test_tutorial/test_response_model/test_tutorial006.py:10-26` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/settings/app02_an_py310/main.py:1-22` + `docs_src/settings/app02_py310/main.py:1-21` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_header/test_optional_list.py:215-239` + `tests/test_request_params/test_header/test_optional_list.py:320-352` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_router_events.py:277-292` + `tests/test_router_events.py:96-111` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_header/test_required_str.py:133-167` + `tests/test_request_params/test_header/test_required_str.py:372-411` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_application.py:105-122` + `tests/test_tutorial/test_request_forms/test_tutorial001.py:110-127` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/security/tutorial005_an_py310.py:151-170` + `docs_src/security/tutorial005_py310.py:150-167` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_dependency_yield_scope.py:59-70` + `tests/test_dependency_yield_scope_websockets.py:61-72` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_oauth2_password_bearer_optional.py:11-30` + `tests/test_security_oauth2_password_bearer_optional_description.py:15-34` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_http_bearer_description.py:6-16` + `tests/test_security_http_digest_description.py:6-16` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/settings/app03_an_py310/config.py:1-9` + `docs_src/settings/app03_py310/config.py:1-9` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/param_functions.py:2355-2388` + `fastapi/params.py:284-317` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_header_params/test_tutorial001.py:38-116` + `tests/test_tutorial/test_header_params/test_tutorial002.py:49-127` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/additional_responses/tutorial001_py310.py:6-15` + `docs_src/additional_responses/tutorial003_py310.py:6-15` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_form/test_nullable_and_defaults.py:601-615` + `tests/test_request_params/test_form/test_nullable_and_defaults.py:627-641` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_nullable_and_defaults.py:128-152` + `tests/test_request_params/test_nullable_and_defaults.py:75-99` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_header/test_list.py:195-246` + `tests/test_request_params/test_header/test_list.py:302-356` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/templates/tutorial001_py310.py:1-18` + `docs_src/templates/tutorial002_py310.py:1-18` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/app_testing/app_b_an_py310/main.py:1-38` + `docs_src/app_testing/app_b_py310/main.py:1-36` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_api_key_query.py:12-39` + `tests/test_security_api_key_query_description.py:12-39` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/param_functions.py:1694-1728` + `fastapi/params.py:665-699` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_dependency_overrides.py:111-168` + `tests/test_dependency_overrides.py:51-108` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_stream_bare_type.py:48-53` + `tests/test_stream_bare_type.py:56-61` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_security_http_base.py:39-64` + `tests/test_security_openid_connect_optional.py:52-82` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/routing.py:4945-5320` + `fastapi/routing.py:5322-5702` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_header/test_list.py:143-177` + `tests/test_request_params/test_header/test_list.py:388-427` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/body_multiple_params/tutorial004_an_py310.py:1-33` + `docs_src/body_multiple_params/tutorial004_py310.py:1-31` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_path_params_numeric_validations/test_tutorial001.py:10-49` + `tests/test_tutorial/test_path_params_numeric_validations/test_tutorial002_tutorial003.py:8-49` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_form/test_required_str.py:185-235` + `tests/test_request_params/test_form/test_required_str.py:26-66` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_header_params/test_tutorial001.py:10-35` + `tests/test_tutorial/test_header_params/test_tutorial002.py:10-46` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_router_events.py:150-168` + `tests/test_router_events.py:67-83` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_default_response_class_router.py:111-122` + `tests/test_default_response_class_router.py:139-150` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_request_params/test_cookie/test_optional_str.py:154-169` + `tests/test_request_params/test_query/test_optional_str.py:151-166` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_tutorial/test_header_param_models/test_tutorial001.py:102-209` + `tests/test_tutorial/test_header_param_models/test_tutorial002.py:97-204` | ✅ pass | passing |
| fastapi | clearly_in | `docs_src/security/tutorial004_an_py310.py:113-150` + `docs_src/security/tutorial005_an_py310.py:143-177` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_path.py:44-77` + `tests/test_path.py:95-128` | ✅ pass | passing |
| fastapi | clearly_in | `fastapi/dependencies/models.py:139-148` + `fastapi/dependencies/models.py:169-182` | ✅ pass | passing |
| fastapi | clearly_in | `tests/test_dependency_wrapped.py:188-229` + `tests/test_dependency_wrapped.py:334-381` | ✅ pass | passing |
| flask | clearly_in | `tests/test_user_error_handler.py:229-235` + `tests/test_user_error_handler.py:47-53` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:142-149` + `tests/test_basic.py:180-187` | ✅ pass | passing |
| flask | clearly_in | `src/flask/sansio/app.py:714-719` + `src/flask/sansio/app.py:773-778` | ✅ pass | passing |
| flask | clearly_in | `src/flask/app.py:55-76` + `src/flask/sansio/app.py:37-57` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:145-152` + `tests/test_basic.py:183-190` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:433-440` + `tests/test_blueprints.py:483-490` | ✅ pass | passing |
| flask | clearly_in | `tests/test_testing.py:309-320` + `tests/test_testing.py:326-337` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:622-633` + `tests/test_blueprints.py:639-650` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:923-941` + `tests/test_blueprints.py:943-961` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:146-153` + `tests/test_basic.py:184-191` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:281-289` + `tests/test_basic.py:303-311` | ✅ pass | passing |
| flask | clearly_in | `tests/test_templating.py:305-315` + `tests/test_templating.py:332-342` | ✅ pass | passing |
| flask | clearly_in | `examples/tutorial/tests/test_blog.py:46-54` + `examples/tutorial/tests/test_blog.py:57-65` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:1435-1443` + `tests/test_basic.py:1446-1454` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:143-150` + `tests/test_basic.py:181-188` | ✅ pass | passing |
| flask | clearly_in | `tests/test_appctx.py:115-124` + `tests/test_appctx.py:92-101` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:586-600` + `tests/test_blueprints.py:636-650` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:927-945` + `tests/test_blueprints.py:947-965` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:935-953` + `tests/test_blueprints.py:955-973` | ✅ pass | passing |
| flask | clearly_in | `src/flask/ctx.py:528-540` + `src/flask/globals.py:65-77` | ✅ pass | passing |
| flask | clearly_in | `tests/test_helpers.py:230-240` + `tests/test_helpers.py:274-284` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:931-949` + `tests/test_blueprints.py:951-969` | ✅ pass | passing |
| flask | clearly_in | `src/flask/sansio/blueprints.py:18-26` + `src/flask/sansio/scaffold.py:28-38` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:1745-1755` + `tests/test_blueprints.py:286-296` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:625-633` + `tests/test_blueprints.py:659-667` | ✅ pass | passing |
| flask | clearly_in | `tests/test_appctx.py:58-67` + `tests/test_reqctx.py:30-39` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:148-153` + `tests/test_basic.py:186-192` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:1022-1038` + `tests/test_blueprints.py:1041-1060` | ✅ pass | passing |
| flask | clearly_in | `tests/test_cli.py:397-409` + `tests/test_cli.py:412-424` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:459-473` + `tests/test_blueprints.py:493-507` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:778-793` + `tests/test_basic.py:796-811` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:426-440` + `tests/test_blueprints.py:476-490` | ✅ pass | passing |
| flask | clearly_in | `tests/test_cli.py:63-70` + `tests/test_cli.py:81-88` | ✅ pass | passing |
| flask | clearly_in | `tests/test_user_error_handler.py:166-179` + `tests/test_user_error_handler.py:187-200` | ✅ pass | passing |
| flask | clearly_in | `tests/test_cli.py:400-409` + `tests/test_cli.py:415-424` | ✅ pass | passing |
| flask | clearly_in | `tests/test_cli.py:504-509` + `tests/test_cli.py:513-518` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:153-172` + `tests/test_blueprints.py:839-859` | ✅ pass | passing |
| flask | clearly_in | `src/flask/app.py:363-443` + `src/flask/blueprints.py:55-128` | ✅ pass | passing |
| flask | clearly_in | `src/flask/sansio/blueprints.py:413-420` + `src/flask/sansio/scaffold.py:376-441` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:157-172` + `tests/test_blueprints.py:843-859` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:765-793` + `tests/test_blueprints.py:796-832` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:144-151` + `tests/test_basic.py:182-189` | ✅ pass | passing |
| flask | clearly_in | `tests/test_views.py:218-234` + `tests/test_views.py:237-253` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:619-633` + `tests/test_blueprints.py:653-667` | ✅ pass | passing |
| flask | clearly_in | `tests/test_templating.py:71-88` + `tests/test_templating.py:91-110` | ✅ pass | passing |
| flask | clearly_in | `tests/test_blueprints.py:919-937` + `tests/test_blueprints.py:939-957` | ✅ pass | passing |
| flask | clearly_in | `tests/test_basic.py:818-828` + `tests/test_basic.py:830-840` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Arbitrary.fs:1018-1025` + `tests/FsCheck.Test/Arbitrary.fs:1027-1034` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test.Xunit.v3/PropertyAttributeTests.fs:1-94` + `tests/FsCheck.Test.Xunit/PropertyAttributeTests.fs:1-93` | ✅ pass | passing |
| fscheck | clearly_in | `docs/TestData.fsx:133-142` + `tests/FsCheck.Test/Runner.fs:95-104` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Property.fs:315-338` + `tests/FsCheck.Test/Property.fs:352-388` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/CheckExtensions.fs:25-30` + `src/FsCheck.Xunit/CheckExtensions.fs:23-28` | ✅ pass | passing |
| fscheck | clearly_in | `docs/StatefulTesting.fsx:16-61` + `tests/FsCheck.Test/StateMachine.fs:184-222` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/Internals.Numeric.fs:34-56` + `src/FsCheck/Internals.Numeric.fs:9-31` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:16-169` + `src/FsCheck.Xunit/PropertyAttribute.fs:13-162` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit/AssemblyInfo.fs:3-23` + `src/FsCheck/AssemblyInfo.fs:3-23` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/DefaultArbTypes.fs:84-92` + `src/FsCheck/DefaultArbTypes.fs:93-101` | ✅ pass | passing |
| fscheck | clearly_in | `examples/FsCheck.Docs.CSharpSnippets/Properties.cs:107-112` + `examples/FsCheck.Examples.MsTest/SampleTests.cs:106-114` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.NUnit/FsCheckPropertyAttribute.fs:53-123` + `src/FsCheck.Xunit.v3/PropertyAttribute.fs:48-118` | ✅ pass | passing |
| fscheck | clearly_in | `examples/FsCheck.Examples.CSharp/Program.cs:13-26` + `examples/FsCheck.Examples.MsTest/ClassesToTest/EnumerableExtensions.cs:10-23` | 🔴 fail | on record as failing |
| fscheck | clearly_in | `examples/FsCheck.Examples.NUnit.CSharp/SyncVersusAsyncExamples.cs:1-58` + `examples/FsCheck.Examples.Xunit.CSharp/SyncVersusAsyncExamples.cs:1-57` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/Random.fs:143-178` + `src/FsCheck/Random.fs:179-205` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:137-160` + `src/FsCheck.Xunit/PropertyAttribute.fs:134-157` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Arbitrary.fs:859-865` + `tests/FsCheck.Test/Arbitrary.fs:869-875` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.NUnit/AssemblyInfo.fs:3-22` + `src/FsCheck.Xunit.v3/AssemblyInfo.fs:3-22` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Runner.fs:241-257` + `tests/FsCheck.Test/Runner.fs:260-276` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/TypeClass.fs:34-41` + `tests/FsCheck.Test/TypeClass.fs:49-55` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/Internals.DefaultArbs.fs:119-131` + `src/FsCheck/Internals.DefaultArbs.fs:89-101` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/Fluent.Gen.fs:114-127` + `src/FsCheck/Fluent.Gen.fs:128-141` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/DefaultArbTypes.fs:102-110` + `src/FsCheck/DefaultArbTypes.fs:93-101` | ✅ pass | passing |
| fscheck | clearly_in | `examples/FsCheck.Examples.MsTest/ClassesToTest/Doc.cs:45-60` + `examples/FsCheck.Examples.MsTest/ClassesToTest/Doc.cs:62-77` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/Internals.DefaultArbs.fs:1013-1023` + `src/FsCheck/Internals.DefaultArbs.fs:999-1009` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Arbitrary.fs:642-650` + `tests/FsCheck.Test/Arbitrary.fs:652-660` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/CheckExtensions.fs:1-62` + `src/FsCheck.Xunit/CheckExtensions.fs:1-60` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.NUnit/FsCheckPropertyAttribute.fs:20-51` + `src/FsCheck.Xunit.v3/PropertyAttribute.fs:24-46` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/Runner.fs:1-20` + `src/FsCheck.Xunit/Runner.fs:1-20` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test.NUnit/PropertiesAttributeTests.fs:6-68` + `tests/FsCheck.Test.Xunit.v3/PropertyAttributeTests.fs:8-70` | ✅ pass | passing |
| fscheck | clearly_in | `examples/FsCheck.Docs.CSharpSnippets/Properties.cs:116-123` + `examples/FsCheck.Examples.CSharp/Program.cs:112-119` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck.Xunit.v3/PropertyAttribute.fs:197-216` + `src/FsCheck.Xunit/PropertyAttribute.fs:178-197` | ✅ pass | passing |
| fscheck | clearly_in | `tests/FsCheck.Test/Property.fs:352-363` + `tests/FsCheck.Test/Property.fs:365-376` | ✅ pass | passing |
| fscheck | clearly_in | `src/FsCheck/FSharp.Gen.fs:116-126` + `src/FsCheck/FSharp.Gen.fs:128-139` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:526-608` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:675-757` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Json.Core/JsonSchema.fs:181-187` + `src/FSharp.Data.Json.Core/JsonSchema.fs:197-203` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:587-619` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:736-768` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:576-608` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:725-757` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:597-630` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:746-779` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:515-547` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:664-696` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Http/Http.fs:2333-2368` + `src/FSharp.Data.Http/Http.fs:2369-2404` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:564-575` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:713-724` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:555-586` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:704-735` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:155-168` + `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:90-103` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Html.Core/HtmlOperations.fs:620-652` + `src/FSharp.Data.Html.Core/HtmlOperations.fs:769-801` | ✅ pass | passing |
| fsharp.data | clearly_in | `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:442-447` + `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:449-454` | ✅ pass | passing |
| fsharp.data | clearly_in | `src/FSharp.Data.Json.Core/JsonSchema.fs:404-409` + `src/FSharp.Data.Json.Core/JsonSchema.fs:420-425` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:57-73` + `tests/FSharp.Data.Core.Tests/WorldBankRuntime.fs:23-173` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.DesignTime/CommonProviderImplementation/Helpers.fs:12-17` + `src/FSharp.Data.DesignTime/Xml/XmlGenerator.fs:7-12` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:418-445` + `src/FSharp.Data.Http/Http.fs:697-724` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:248-278` + `src/FSharp.Data.Http/Http.fs:690-720` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.DesignTime/Json/JsonGenerator.fs:7-12` + `tests/FSharp.Data.DesignTime.Tests/InferenceTests.fs:8-13` | ✅ pass | passing |
| fsharp.data | clearly_out | `docs/library/HtmlProvider.fsx:105-113` + `docs/library/HtmlProvider.fsx:370-404` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:2043-2045` + `src/FSharp.Data.Http/Http.fs:2046-2048` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.DesignTime/Json/JsonGenerator.fs:10-20` + `src/FSharp.Data.DesignTime/Xml/XmlGenerator.fs:10-22` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:2044-2046` + `src/FSharp.Data.Http/Http.fs:2047-2049` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.Http/Http.fs:359-390` + `src/FSharp.Data.Http/Http.fs:418-449` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.DesignTime/Json/JsonProvider.fs:6-13` + `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:7-14` | ✅ pass | passing |
| fsharp.data | clearly_out | `src/FSharp.Data.DesignTime/Json/JsonProvider.fs:8-18` + `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:9-19` | ✅ pass | passing |
| fsharp.data | clearly_out | `tests/FSharp.Data.Core.Tests/HtmlRuntimeTypes.fs:1-9` + `tests/FSharp.Data.Tests/JsonProvider.fs:1-9` | ✅ pass | passing |
| fsharp.data | clearly_out | `docs/library/TypeInference.fsx:185-210` + `tests/FSharp.Data.Tests/JsonProvider.fs:832-839` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:18-51` + `tests/FSharpPlus.Tests/ValueTask.fs:42-78` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Internals.fs:351-441` + `src/FSharpPlus/Internals.fs:443-532` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus.TypeLevel/Providers/Matrix.Provider.fs:163-181` + `src/FSharpPlus.TypeLevel/Providers/Matrix.Provider.fs:73-91` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:56-122` + `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:59-126` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/NonEmptyList.fs:57-76` + `src/FSharpPlus/Data/NonEmptySet.fs:66-83` | 🔴 fail | on record as failing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:194-238` + `tests/FSharpPlus.Tests/ValueTask.fs:155-202` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Builders.fs:20-28` + `tests/FSharpPlusFable.Tests/FSharpTests/General.fs:19-32` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Traversable.fs:201-212` + `src/FSharpPlus/Control/Traversable.fs:400-407` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlusFable.Tests/FSharpTests/Extensions.fs:163-170` + `tests/FSharpPlusFable.Tests/FSharpTests/Extensions.fs:199-206` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Error.fs:137-149` + `src/FSharpPlus/Data/Error.fs:226-238` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Collection.fs:166-173` + `src/FSharpPlus/Control/Collection.fs:182-189` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Task.fs:267-273` + `src/FSharpPlus/Extensions/ValueTask.fs:251-257` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Seq.fs:1006-1024` + `src/FSharpPlus/Data/Seq.fs:1026-1058` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Option.fs:1-101` + `src/FSharpPlus/Data/ValueOption.fs:1-102` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Indexable.fs:182-188` + `src/FSharpPlus/Control/Indexable.fs:201-207` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Helpers.fs:22-77` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:67-116` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:28-105` + `src/FSharpPlus/Extensions/Dictionary.fs:30-107` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Extensions.fs:313-318` + `tests/benchmarks/ExtensionsBenchmarks.fs:27-32` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Traversable.fs:160-166` + `src/FSharpPlus/Control/Traversable.fs:360-366` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Numeric.fs:541-590` + `src/FSharpPlus/Control/Numeric.fs:592-640` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Traversable.fs:135-151` + `src/FSharpPlus/Control/Traversable.fs:335-351` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Seq.fs:398-440` + `src/FSharpPlus/Data/Seq.fs:442-484` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Task.fs:405-456` + `src/FSharpPlus/Extensions/ValueTask.fs:378-418` | ✅ pass | passing |
| fsharpplus | clearly_in | `docsrc/content/type-free.fsx:72-87` + `tests/FSharpPlus.Tests/Free.fs:107-122` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/General.fs:15-68` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:2-64` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Option.fs:1-89` + `src/FSharpPlus/Extensions/ValueOption.fs:1-103` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Error.fs:184-211` + `src/FSharpPlus/Data/Error.fs:95-122` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Seq.fs:398-440` + `src/FSharpPlus/Data/Seq.fs:486-528` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/ValueTask.fs:163-206` + `src/FSharpPlus/Extensions/ValueTask.fs:207-249` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:227-309` + `src/FSharpPlus/Extensions/Dictionary.fs:228-310` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:546-567` + `tests/FSharpPlus.Tests/ValueTask.fs:439-460` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Error.fs:156-166` + `src/FSharpPlus/Data/Error.fs:63-73` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:137-204` + `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:160-242` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Free.fs:161-167` + `tests/FSharpPlus.Tests/Free.fs:79-85` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Validations.fs:151-198` + `tests/FSharpPlus.Tests/Validations.fs:204-251` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Collection.fs:13-78` + `src/FSharpPlus/Control/Collection.fs:81-134` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Traversals.fs:116-171` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Traversable.fs:134-191` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/General.fs:168-215` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:211-255` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:17-55` + `src/FSharpPlus/Extensions/Dictionary.fs:18-57` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Foldable.fs:362-374` + `src/FSharpPlus/Control/Foldable.fs:392-404` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Collection.fs:267-299` + `src/FSharpPlus/Control/Collection.fs:376-408` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Task.fs:103-132` + `src/FSharpPlus/Extensions/Task.fs:263-286` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:302-1106` + `tests/FSharpPlus.Tests/ValueTask.fs:203-999` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Applicative.fs:167-199` + `src/FSharpPlus/Control/ZipApplicative.fs:221-253` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Control/Numeric.fs:571-580` + `src/FSharpPlus/Control/Numeric.fs:621-630` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:190-226` + `src/FSharpPlus/Extensions/Dictionary.fs:192-227` | ✅ pass | passing |
| fsharpplus | clearly_in | `docsrc/content/type-free.fsx:4-110` + `tests/FSharpPlus.Tests/Free.fs:75-145` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Seq.fs:267-316` + `src/FSharpPlus/Extensions/Seq.fs:317-351` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Monads.fs:14-35` + `tests/FSharpPlusFable.Tests/FSharpTests/General.fs:42-67` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/ValueTask.fs:121-153` + `tests/FSharpPlus.Tests/ValueTask.fs:155-187` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/ResizeArray.fs:106-138` + `src/FSharpPlus/Extensions/ResizeArray.fs:139-161` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Option.fs:83-91` + `src/FSharpPlus/Data/ValueOption.fs:84-92` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Asyncs.fs:38-67` + `tests/FSharpPlus.Tests/Asyncs.fs:69-98` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Extensions.fs:349-418` + `src/FSharpPlus/Extensions/Extensions.fs:436-505` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/General.fs:101-185` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:152-225` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Collections.fs:34-52` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Collections.fs:18-39` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:123-189` + `src/FSharpPlus/Extensions/IReadOnlyDictionary.fs:127-231` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:160-192` + `tests/FSharpPlus.Tests/Task.fs:194-226` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Task.fs:31-102` + `src/FSharpPlus/Extensions/ValueTask.fs:28-105` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Extensions/Dict.fs:190-204` + `src/FSharpPlus/Extensions/Dictionary.fs:192-205` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/Task.fs:3-12` + `tests/FSharpPlus.Tests/ValueTask.fs:3-12` | ✅ pass | passing |
| fsharpplus | clearly_in | `src/FSharpPlus/Data/Error.fs:102-124` + `src/FSharpPlus/Data/Error.fs:191-213` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlus.Tests/General.fs:51-63` + `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:46-58` | 🔴 fail | on record as failing |
| fsharpplus | clearly_in | `docsrc/content/type-all.fsx:5-45` + `docsrc/content/type-any.fsx:5-46` | ✅ pass | passing |
| fsharpplus | clearly_in | `tests/FSharpPlusFable.Tests/FSharpTests/General/Monoid.fs:5-38` + `tests/FSharpPlusFable.Tests/FSharpTests/General/MonoidCompile.fs:5-38` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:1178-1188` + `binding/binding_test.go:876-886` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:227-237` + `routes_test.go:296-303` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:203-214` + `recovery_test.go:272-284` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:914-922` + `binding/binding_test.go:935-943` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:1037-1046` + `binding/binding_test.go:1205-1214` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:858-872` + `binding/binding_test.go:874-892` | ✅ pass | passing |
| gin | clearly_in | `middleware_test.go:118-157` + `middleware_test.go:81-116` | ✅ pass | passing |
| gin | clearly_in | `binding/form_mapping_test.go:616-663` + `binding/form_mapping_test.go:865-912` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:334-363` + `routes_test.go:366-396` | ✅ pass | passing |
| gin | clearly_in | `gin_integration_test.go:326-336` + `gin_integration_test.go:359-369` | ✅ pass | passing |
| gin | clearly_in | `binding/form_mapping_benchmark_test.go:33-43` + `binding/form_mapping_benchmark_test.go:56-66` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:339-355` + `routes_test.go:372-388` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:2742-2754` + `context_test.go:2757-2771` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:285-296` + `recovery_test.go:62-73` | ✅ pass | passing |
| gin | clearly_in | `logger_test.go:22-84` + `logger_test.go:86-148` | ✅ pass | passing |
| gin | clearly_in | `context.go:1259-1279` + `context.go:1288-1305` | ✅ pass | passing |
| gin | clearly_in | `logger_test.go:161-169` + `logger_test.go:196-204` | ✅ pass | passing |
| gin | clearly_in | `render/render_test.go:659-678` + `render/render_test.go:680-699` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:350-368` + `gin_test.go:417-435` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:2050-2074` + `context_test.go:2076-2100` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:880-906` + `routes_test.go:911-942` | ✅ pass | passing |
| gin | clearly_in | `binding/msgpack.go:17-29` + `binding/yaml.go:15-27` | ✅ pass | passing |
| gin | clearly_in | `binding/toml.go:25-35` + `binding/xml.go:24-34` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:440-450` + `routes_test.go:475-485` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:123-141` + `gin_test.go:143-161` | ✅ pass | passing |
| gin | clearly_in | `tree.go:861-871` + `tree.go:877-887` | 🔴 fail | on record as failing |
| gin | clearly_in | `render/render_test.go:124-136` + `render/render_test.go:87-99` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:1782-1792` + `context_test.go:1794-1804` | ✅ pass | passing |
| gin | clearly_in | `tree.go:764-799` + `tree.go:848-871` | 🔴 fail | on record as failing |
| gin | clearly_in | `context_test.go:2431-2451` + `context_test.go:2601-2621` | ✅ pass | passing |
| gin | clearly_in | `render/render_test.go:622-630` + `render/render_test.go:669-677` | ✅ pass | passing |
| gin | clearly_in | `tree_test.go:1023-1052` + `tree_test.go:1067-1091` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:1000-1016` + `binding/binding_test.go:982-998` | ✅ pass | passing |
| gin | clearly_in | `response_writer_test.go:166-221` + `response_writer_test.go:225-280` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:510-518` + `gin_test.go:589-597` | ✅ pass | passing |
| gin | clearly_in | `gin_integration_test.go:213-224` + `gin_integration_test.go:69-80` | ✅ pass | passing |
| gin | clearly_in | `binding/form_mapping_test.go:508-542` + `binding/form_mapping_test.go:756-787` | ✅ pass | passing |
| gin | clearly_in | `middleware_test.go:135-155` + `middleware_test.go:95-114` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:203-217` + `recovery_test.go:273-287` | ✅ pass | passing |
| gin | clearly_in | `logger_test.go:403-416` + `logger_test.go:418-434` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:242-253` + `recovery_test.go:54-65` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:163-174` + `routes_test.go:189-200` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:441-453` + `routes_test.go:476-487` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:739-753` + `routes_test.go:759-773` | ✅ pass | passing |
| gin | clearly_in | `binding/json_test.go:113-131` + `binding/json_test.go:169-187` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:124-132` + `gin_test.go:164-172` | ✅ pass | passing |
| gin | clearly_in | `binding/form_mapping_test.go:555-603` + `binding/form_mapping_test.go:802-850` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:172-187` + `gin_test.go:399-414` | ✅ pass | passing |
| gin | clearly_in | `gin_test.go:746-779` + `gin_test.go:781-816` | ✅ pass | passing |
| gin | clearly_in | `codec/json/go_json.go:16-42` + `codec/json/json.go:15-41` | ✅ pass | passing |
| gin | clearly_in | `render/render_test.go:734-745` + `render/render_test.go:747-758` | ✅ pass | passing |
| gin | clearly_in | `binding/xml.go:16-26` + `binding/yaml.go:17-27` | ✅ pass | passing |
| gin | clearly_in | `routes_test.go:458-468` + `routes_test.go:492-502` | ✅ pass | passing |
| gin | clearly_in | `binding/form_mapping_test.go:692-738` + `binding/form_mapping_test.go:942-988` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:2869-2879` + `context_test.go:2933-2943` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:207-218` + `recovery_test.go:277-288` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:159-166` + `context_test.go:224-231` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:1863-1870` + `context_test.go:1922-1929` | ✅ pass | passing |
| gin | clearly_in | `codec/json/jsoniter.go:16-44` + `codec/json/sonic.go:16-44` | ✅ pass | passing |
| gin | clearly_in | `auth_test.go:106-118` + `auth_test.go:126-138` | ✅ pass | passing |
| gin | clearly_in | `binding/binding.go:12-91` + `binding/binding_nomsgpack.go:12-87` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:1177-1187` + `binding/binding_test.go:895-905` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:531-554` + `binding/binding_test.go:556-579` | ✅ pass | passing |
| gin | clearly_in | `context_test.go:2775-2837` + `context_test.go:3032-3105` | ✅ pass | passing |
| gin | clearly_in | `render/toml.go:21-31` + `render/yaml.go:21-31` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:234-265` + `recovery_test.go:268-301` | ✅ pass | passing |
| gin | clearly_in | `binding/binding_test.go:241-251` + `binding/binding_test.go:253-263` | ✅ pass | passing |
| gin | clearly_in | `fs_test.go:13-49` + `internal/fs/fs_test.go:13-49` | ✅ pass | passing |
| gin | clearly_in | `recovery_test.go:233-266` + `recovery_test.go:268-301` | ✅ pass | passing |
| gin | clearly_in | `binding/binding.go:95-127` + `binding/binding_nomsgpack.go:91-121` | ✅ pass | passing |
| giraffe | clearly_in | `src/Giraffe/Routing.fs:278-309` + `src/Giraffe/Routing.fs:310-331` | ✅ pass | passing |
| giraffe | clearly_out | `tests/Giraffe.Tests/EndpointRoutingTests.fs:18-24` + `tests/Giraffe.Tests/EndpointRoutingTests.fs:180-186` | ✅ pass | passing |
| guzzle | clearly_in | `tests/ClientTest.php:1598-1631` + `tests/Handler/CurlMultiHandlerTest.php:4098-4131` | ✅ pass | passing |
| guzzle | clearly_in | `src/Handler/CurlFactory.php:2912-2922` + `src/Handler/TlsVersion.php:55-63` | ✅ pass | passing |
| guzzle | clearly_in | `src/Cookie/SetCookie.php:197-210` + `src/Cookie/SetCookie.php:215-228` | ✅ pass | passing |
| guzzle | clearly_in | `tests/RedirectMiddlewareTest.php:1241-1273` + `tests/RedirectMiddlewareTest.php:359-392` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlHandlerTest.php:327-386` + `tests/Handler/CurlMultiHandlerTest.php:1671-1730` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlHandlerTest.php:194-233` + `tests/Handler/CurlMultiHandlerTest.php:1858-1897` | ✅ pass | passing |
| guzzle | clearly_in | `src/AuthMiddleware.php:819-842` + `src/Handler/StreamHandler.php:594-617` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Cookie/FileCookieJarTest.php:336-366` + `tests/Cookie/SessionCookieJarTest.php:229-259` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:10182-10219` + `tests/Handler/ProxyEnvTest.php:198-235` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/StreamHandlerTest.php:5071-5082` + `tests/Handler/StreamHandlerTest.php:5140-5151` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Cookie/FileCookieJarTest.php:271-278` + `tests/Cookie/SessionCookieJarTest.php:216-223` | ✅ pass | passing |
| guzzle | clearly_in | `src/Handler/CurlFactory.php:602-624` + `src/Handler/CurlMultiHandler.php:531-553` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/StreamHandlerTest.php:5063-5095` + `tests/Handler/StreamHandlerTest.php:5132-5164` | ✅ pass | passing |
| guzzle | clearly_in | `tests/AuthMiddlewareTest.php:259-274` + `tests/AuthMiddlewareTest.php:614-629` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Cookie/CookieJarTest.php:528-541` + `tests/Cookie/CookieJarTest.php:566-579` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/ProxyEnvTest.php:198-235` + `tests/Handler/StreamHandlerTest.php:2206-2243` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:6475-6487` + `tests/Handler/CurlFactoryTest.php:6757-6769` | ✅ pass | passing |
| guzzle | clearly_in | `tests/AuthMiddlewareTest.php:218-233` + `tests/AuthMiddlewareTest.php:259-274` | ✅ pass | passing |
| guzzle | clearly_in | `src/Handler/CurlFactory.php:849-859` + `src/Handler/CurlFactory.php:864-874` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:6447-6467` + `tests/Handler/CurlFactoryTest.php:6477-6496` | ✅ pass | passing |
| guzzle | clearly_in | `tests/ClientTest.php:1137-1144` + `tests/ClientTest.php:998-1005` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlMultiHandlerTest.php:3661-3676` + `tests/Handler/CurlMultiHandlerTest.php:3693-3706` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlMultiHandlerTest.php:2144-2206` + `tests/Handler/CurlMultiHandlerTest.php:2208-2257` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:8968-8991` + `tests/Handler/CurlFactoryTest.php:9292-9323` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlMultiHandlerTest.php:3108-3117` + `tests/Handler/CurlMultiHandlerTest.php:3395-3404` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Exception/ConnectExceptionTest.php:12-35` + `tests/Exception/NetworkTimeoutExceptionTest.php:12-36` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Exception/ConnectTimeoutExceptionTest.php:21-35` + `tests/Exception/NetworkTimeoutExceptionTest.php:21-35` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:1119-1128` + `tests/Handler/StreamHandlerTest.php:4972-4981` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Exception/ServerExceptionTest.php:20-40` + `tests/Exception/TooManyRedirectsExceptionTest.php:19-37` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:9437-9442` + `tests/Handler/StreamHandlerTest.php:3677-3682` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:8852-8857` + `tests/Handler/CurlFactoryTest.php:9173-9178` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Exception/ConnectExceptionTest.php:1-35` + `tests/Exception/NetworkTimeoutExceptionTest.php:1-36` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Handler/CurlFactoryTest.php:9172-9179` + `tests/Handler/CurlFactoryTest.php:9728-9735` | ✅ pass | passing |
| guzzle | clearly_in | `tests/AuthMiddlewareTest.php:482-513` + `tests/AuthMiddlewareTest.php:874-897` | ✅ pass | passing |
| guzzle | clearly_in | `tests/Exception/ClientExceptionTest.php:20-40` + `tests/Exception/ServerExceptionTest.php:20-40` | ✅ pass | passing |
| guzzle | clearly_in | `tests/ClientTest.php:2109-2122` + `tests/PoolTest.php:429-442` | ✅ pass | passing |
| guzzle | clearly_out | `tests/Handler/CurlFactoryTest.php:2684-2685` + `tests/Handler/CurlFactoryTest.php:326-327` | ✅ pass | passing |
| guzzle | clearly_out | `tests/ProxyOptionsTest.php:109-115` + `tests/ProxyOptionsTest.php:326-332` | ✅ pass | passing |
| guzzle | clearly_out | `src/Handler/CurlVersion.php:24-59` + `src/RequestOptions.php:225-370` | ✅ pass | passing |
| guzzle | clearly_out | `src/Multiplexing.php:19-23` + `src/TransportSharing.php:9-13` | ✅ pass | passing |
| guzzle | clearly_out | `tests/Exception/ServerExceptionTest.php:5-10` + `tests/Handler/StreamHandlerTest.php:5-10` | ✅ pass | passing |
| guzzle | clearly_out | `src/Handler/CurlVersion.php:18-59` + `src/RequestOptions.php:236-393` | ✅ pass | passing |
| guzzle | clearly_out | `src/Multiplexing.php:19-23` + `src/RequestOptions.php:236-356` | ✅ pass | passing |
| guzzle | clearly_out | `tests/ClientTest.php:2009-2027` + `tests/PoolTest.php:429-447` | ✅ pass | passing |
| guzzle | clearly_out | `src/Handler/CurlVersion.php:35-64` + `src/RequestOptions.php:236-370` | ✅ pass | passing |
| guzzle | clearly_out | `src/Handler/CurlVersion.php:37-64` + `src/RequestOptions.php:236-356` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:226-240` + `pkgs/cronet_http/example/integration_test/client_profile_test.dart:331-345` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_profile_request_data_test.dart:173-238` + `pkgs/http_profile/test/http_profile_response_data_test.dart:160-225` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_configuration_test.dart:41-58` + `pkgs/cupertino_http/test/url_session_configuration_test.dart:97-114` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/test/http_date_test.dart:282-349` + `pkgs/http_parser/test/http_date_test.dart:71-138` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:19-94` + `pkgs/http_client_conformance_tests/lib/src/response_headers_tests.dart:26-65` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_tests.dart:31-57` + `pkgs/http_client_conformance_tests/lib/src/response_body_tests.dart:24-46` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/lib/main.dart:5-98` + `pkgs/ok_http/example/lib/main.dart:5-93` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_pool_test.dart:216-230` + `pkgs/http2/test/client_pool_test.dart:261-275` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:173-197` + `pkgs/cronet_http/example/integration_test/client_profile_test.dart:48-72` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_profile_response_data_test.dart:105-126` + `pkgs/http_profile/test/http_profile_response_data_test.dart:133-153` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:108-144` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:108-143` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_profile_request_data_test.dart:252-287` + `pkgs/http_profile/test/http_profile_response_data_test.dart:295-330` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_tests.dart:27-40` + `pkgs/http_client_conformance_tests/lib/src/response_body_tests.dart:21-34` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/lib/src/http_profile_request_data.dart:150-172` + `pkgs/http_profile/lib/src/http_profile_response_data.dart:192-214` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/http2_client_test.dart:126-135` + `pkgs/http2/test/http2_client_test.dart:210-218` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/client_profile_test.dart:238-324` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:146-215` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:167-174` + `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:225-233` | 🔴 fail | on record as failing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/request_cookies_test.dart:37-46` + `pkgs/http_client_conformance_tests/lib/src/request_cookies_test.dart:48-57` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_task_test.dart:104-117` + `pkgs/cupertino_http/test/url_session_task_test.dart:184-197` | ✅ pass | passing |
| http | clearly_in | `pkgs/web_socket_conformance_tests/lib/src/no_upgrade_tests.dart:15-35` + `pkgs/web_socket_conformance_tests/lib/src/peer_protocol_errors_tests.dart:15-55` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:1216-1227` + `pkgs/http2/test/client_test.dart:1289-1300` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/client_profile_test.dart:171-236` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:93-144` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/src/frames/frame_writer_reader_test.dart:143-149` + `pkgs/http2/test/src/frames/frame_writer_reader_test.dart:70-76` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:173-216` + `pkgs/cupertino_http/test/client_profile_test.dart:268-323` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/example/example.dart:9-30` + `pkgs/http2/example/main.dart:9-30` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/lib/src/base_response.dart:122-184` + `pkgs/http_profile/lib/src/utils.dart:7-52` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/server_test.dart:398-422` + `pkgs/http2/test/server_test.dart:440-471` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:34-44` + `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:71-81` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/lib/src/streams/stream_handler.dart:837-847` + `pkgs/http2/lib/src/streams/stream_handler.dart:850-862` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:93-145` + `pkgs/cupertino_http/test/client_profile_test.dart:171-236` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/test/http_date_test.dart:54-69` + `pkgs/http_parser/test/http_date_test.dart:71-86` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/test/response_test.dart:157-169` + `pkgs/http/test/response_test.dart:171-184` | ✅ pass | passing |
| http | clearly_in | `pkgs/web_socket_conformance_tests/lib/src/close_local_tests.dart:24-44` + `pkgs/web_socket_conformance_tests/lib/src/protocol_tests.dart:23-43` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:1153-1161` + `pkgs/http2/test/client_test.dart:843-852` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/test/http_date_test.dart:187-237` + `pkgs/http_parser/test/http_date_test.dart:88-138` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/server_test.dart:392-423` + `pkgs/http2/test/server_test.dart:441-472` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:627-671` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:673-717` | ✅ pass | passing |
| http | clearly_in | `pkgs/web_socket_channel/lib/adapter_web_socket_channel.dart:135-149` + `pkgs/web_socket_channel/lib/html.dart:167-181` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/example/main.dart:13-28` + `pkgs/http2/example/main.dart:9-30` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:1385-1396` + `pkgs/http2/test/server_test.dart:137-153` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/lib/src/chunked_coding/decoder.dart:126-134` + `pkgs/http_parser/lib/src/chunked_coding/decoder.dart:136-144` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/test/request_test.dart:237-247` + `pkgs/http/test/request_test.dart:249-259` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/src/streams/streams_test.dart:113-148` + `pkgs/http2/test/src/streams/streams_test.dart:150-187` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/lib/src/authentication_challenge.dart:134-143` + `pkgs/http_parser/lib/src/authentication_challenge.dart:72-81` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:411-426` + `pkgs/http2/test/client_test.dart:741-756` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:20-217` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:18-215` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/test/io/client_test.dart:112-147` + `pkgs/http/test/io/client_test.dart:149-185` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/server_test.dart:620-653` + `pkgs/http2/test/server_test.dart:829-854` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:34-44` + `pkgs/http_client_conformance_tests/lib/src/redirect_tests.dart:46-56` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/server_test.dart:427-474` + `pkgs/http2/test/server_test.dart:478-525` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/server_test.dart:392-403` + `pkgs/http2/test/server_test.dart:769-780` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/client_profile_test.dart:43-120` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:146-215` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_parser/test/http_date_test.dart:187-242` + `pkgs/http_parser/test/http_date_test.dart:88-143` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_test.dart:15-46` + `pkgs/ok_http/example/integration_test/client_test.dart:18-56` | ✅ pass | passing |
| http | clearly_in | `pkgs/ok_http/example/integration_test/certificate_test.dart:116-124` + `pkgs/ok_http/example/integration_test/certificate_test.dart:79-87` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:33-60` + `pkgs/http_client_conformance_tests/lib/src/response_cookies_test.dart:62-93` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:1219-1229` + `pkgs/http2/test/client_test.dart:1292-1302` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:163-173` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:93-103` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/close_test.dart:29-66` + `pkgs/http_profile/test/close_test.dart:76-113` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_test.dart:768-785` + `pkgs/http2/test/client_test.dart:866-883` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:125-130` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:46-52` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/src/hpack/hpack_test.dart:273-291` + `pkgs/http2/test/src/hpack/hpack_test.dart:473-491` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/response_headers_server.dart:5-44` + `pkgs/http_client_conformance_tests/lib/src/response_status_line_server.dart:5-43` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_profile_request_data_test.dart:12-27` + `pkgs/http_profile/test/http_profile_response_data_test.dart:12-27` | ✅ pass | passing |
| http | clearly_in | `pkgs/web_socket_channel/test/io_test.dart:53-74` + `pkgs/web_socket_channel/test/io_test.dart:76-99` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/compressed_response_body_server_web.dart:3-12` + `pkgs/http_client_conformance_tests/lib/src/response_body_streamed_server_web.dart:3-12` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/lib/src/http_profile_request_data.dart:184-251` + `pkgs/http_profile/lib/src/http_profile_response_data.dart:228-297` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:384-453` + `pkgs/cupertino_http/test/client_profile_test.dart:379-458` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/transport_test.dart:408-443` + `pkgs/http2/test/transport_test.dart:445-478` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/transport_test.dart:24-52` + `pkgs/http2/test/transport_test.dart:54-82` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/test/client_pool_test.dart:103-117` + `pkgs/http2/test/client_pool_test.dart:263-277` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:788-793` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:800-805` | ✅ pass | passing |
| http | clearly_in | `pkgs/cronet_http/example/integration_test/client_profile_test.dart:18-91` + `pkgs/ok_http/example/integration_test/client_profile_test.dart:18-91` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_client_conformance_tests/lib/src/response_cookies_server.dart:19-44` + `pkgs/http_client_conformance_tests/lib/src/response_status_line_server.dart:5-43` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:31-44` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:551-564` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/lib/src/http_profile_request_data.dart:102-157` + `pkgs/http_profile/lib/src/http_profile_response_data.dart:83-159` | ✅ pass | passing |
| http | clearly_in | `pkgs/http2/lib/src/hpack/hpack.dart:235-251` + `pkgs/http2/test/src/hpack/hpack_test.dart:887-906` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_profile_request_data_test.dart:240-287` + `pkgs/http_profile/test/http_profile_response_data_test.dart:307-354` | ✅ pass | passing |
| http | clearly_in | `pkgs/flutter_http_example/test/widget_test.dart:145-168` + `pkgs/flutter_http_example/test/widget_test.dart:208-233` | ✅ pass | passing |
| http | clearly_in | `pkgs/web_socket_conformance_tests/lib/src/close_remote_tests.dart:21-32` + `pkgs/web_socket_conformance_tests/lib/src/protocol_tests.dart:23-34` | ✅ pass | passing |
| http | clearly_in | `pkgs/http/test/io/http_test.dart:316-340` + `pkgs/http/test/io/http_test.dart:90-114` | ✅ pass | passing |
| http | clearly_in | `pkgs/http_profile/test/http_client_request_profile_test.dart:125-133` + `pkgs/http_profile/test/http_client_request_profile_test.dart:99-107` | ✅ pass | passing |
| http | clearly_in | `pkgs/ok_http/example/integration_test/client_test.dart:19-36` + `pkgs/ok_http/example/integration_test/client_test.dart:38-55` | ✅ pass | passing |
| http | clearly_in | `pkgs/cupertino_http/test/url_session_delegate_test.dart:627-671` + `pkgs/cupertino_http/test/url_session_delegate_test.dart:673-763` | ✅ pass | passing |
| http | clearly_out | `pkgs/http2/test/flow_control_test.dart:22-31` + `pkgs/web_socket_channel/lib/status.dart:43-90` | ✅ pass | passing |
| http | clearly_out | `pkgs/http2/lib/src/settings/settings.dart:19-72` + `pkgs/http2/lib/transport.dart:79-120` | ✅ pass | passing |
| http | clearly_out | `pkgs/http2/lib/src/connection.dart:134-153` + `pkgs/http2/lib/transport.dart:79-128` | ✅ pass | passing |
| hugo | clearly_in | `resources/image_cache.go:61-66` + `resources/image_cache.go:86-91` | ✅ pass | passing |
| hugo | clearly_in | `tpl/collections/collections.go:195-227` + `tpl/collections/collections.go:355-387` | ✅ pass | passing |
| hugo | clearly_in | `navigation/menu.go:215-233` + `resources/page/pages_sort.go:160-172` | ✅ pass | passing |
| hugo | clearly_in | `hugolib/cascade_test.go:73-89` + `markup/goldmark/goldmark_integration_test.go:335-351` | ✅ pass | passing |
| hugo | clearly_in | `tpl/internal/go_templates/htmltemplate/exec_test.go:354-358` + `tpl/internal/go_templates/texttemplate/exec_test.go:360-364` | ✅ pass | passing |
| hugo | clearly_in | `resources/page/pagegroup_test.go:350-358` + `resources/page/pagegroup_test.go:399-407` | ✅ pass | passing |
| hugo | clearly_in | `navigation/menu.go:179-193` + `resources/page/taxonomy.go:148-162` | ✅ pass | passing |
| hugo | clearly_in | `tpl/urls/urls.go:107-119` + `tpl/urls/urls.go:93-104` | ✅ pass | passing |
| hugo | clearly_in | `resources/page/pagegroup_test.go:276-284` + `resources/page/pagegroup_test.go:447-455` | ✅ pass | passing |
| hugo | clearly_in | `common/paths/path_test.go:55-76` + `helpers/path_test.go:113-134` | ✅ pass | passing |
| hugo | clearly_in | `markup/markup_config/config_test.go:60-66` + `markup/markup_config/config_test.go:73-79` | ✅ pass | passing |
| hugo | clearly_in | `tpl/collections/index.go:123-128` + `tpl/internal/go_templates/texttemplate/funcs.go:161-166` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/texttemplate/exec_test.go:605-611` + `tpl/internal/go_templates/texttemplate/exec_test.go:618-624` | ✅ pass | passing |
| hugo | clearly_out | `tpl/collections/init.go:151-164` + `tpl/urls/init.go:47-58` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/hugo_sites_build_test.go:208-244` + `hugolib/menu_test.go:111-146` | ✅ pass | passing |
| hugo | clearly_out | `tpl/lang/init.go:35-52` + `tpl/strings/init.go:51-69` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/exec_test.go:497-500` + `tpl/internal/go_templates/htmltemplate/exec_test.go:664-668` | ✅ pass | passing |
| hugo | clearly_out | `common/paths/path_test.go:136-143` + `tpl/internal/go_templates/htmltemplate/css_test.go:220-227` | ✅ pass | passing |
| hugo | clearly_out | `config/allconfig/allconfig_integration_test.go:135-166` + `tpl/transform/transform_integration_test.go:353-381` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/exec_test.go:532-539` + `tpl/internal/go_templates/texttemplate/exec_test.go:443-452` | ✅ pass | passing |
| hugo | clearly_out | `htesting/test_helpers_test.go:25-27` + `hugofs/glob_test.go:75-78` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/pagebundler_test.go:381-418` + `hugolib/site_output_test.go:489-543` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/site_url_test.go:114-115` + `tpl/partials/partials_integration_test.go:298-299` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/page__meta_test.go:24-47` + `hugolib/site_output_test.go:657-689` | 🔴 fail | on record as failing |
| hugo | clearly_out | `hugolib/testhelpers_test.go:531-533` + `langs/i18n/translationProvider.go:55-57` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/exec_test.go:507-514` + `tpl/internal/go_templates/texttemplate/exec_test.go:615-622` | 🔴 fail | on record as failing |
| hugo | clearly_out | `media/config_test.go:68-69` + `resources/images/config_test.go:38-39` | ✅ pass | passing |
| hugo | clearly_out | `resources/page/pagemeta/page_frontmatter_test.go:165-172` + `tpl/internal/go_templates/htmltemplate/css_test.go:220-227` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/css_test.go:93-99` + `tpl/internal/go_templates/htmltemplate/escape_test.go:1141-1174` | ✅ pass | passing |
| hugo | clearly_out | `commands/server.go:545-546` + `commands/server.go:550-551` | 🔴 fail | on record as failing |
| hugo | clearly_out | `tpl/tplimpl/templatestore_integration_test.go:1438-1461` + `tpl/tplimpl/templatestore_integration_test.go:367-394` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/content_map_test.go:55-67` + `hugolib/content_map_test.go:79-82` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/pagebundler_test.go:285-286` + `resources/image_test.go:394-395` | ✅ pass | passing |
| hugo | clearly_out | `common/urls/baseURL_test.go:79-80` + `resources/resource_transformers/tocss/dartsass/dartsass_integration_test.go:361-362` | ✅ pass | passing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/css_test.go:78-85` + `tpl/internal/go_templates/htmltemplate/html_test.go:60-67` | ✅ pass | passing |
| hugo | clearly_out | `tpl/tplimpl/templatestore_integration_test.go:1076-1114` + `tpl/tplimpl/templatestore_integration_test.go:908-925` | 🔴 fail | on record as failing |
| hugo | clearly_out | `config/allconfig/allconfig_integration_test.go:17-48` + `config/allconfig/allconfig_integration_test.go:76-84` | ✅ pass | passing |
| hugo | clearly_out | `resources/page/pagemeta/pagemeta_test.go:110-112` + `tpl/internal/go_templates/texttemplate/exec_test.go:1311-1313` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/disableKinds_test.go:175-176` + `resources/image_test.go:304-306` | 🔴 fail | on record as failing |
| hugo | clearly_out | `tpl/internal/go_templates/htmltemplate/exec_test.go:471-486` + `tpl/internal/go_templates/htmltemplate/exec_test.go:569-578` | ✅ pass | passing |
| hugo | clearly_out | `hugolib/site_test.go:958-967` + `resources/page/permalinks_test.go:43-48` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/capture_test.dart:113-122` + `packages/mocktail/test/mockito_compat/capture_test.dart:129-138` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail_image_network/test/mocktail_image_network_test.dart:163-181` + `packages/mocktail_image_network/test/mocktail_image_network_test.dart:183-201` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/lib/src/_register_matcher.dart:103-110` + `packages/mocktail/lib/src/_register_matcher.dart:113-120` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/until_called_test.dart:130-144` + `packages/mocktail/test/mockito_compat/until_called_test.dart:284-298` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/until_called_test.dart:185-201` + `packages/mocktail/test/mockito_compat/until_called_test.dart:351-367` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/capture_test.dart:109-123` + `packages/mocktail/test/mockito_compat/capture_test.dart:125-139` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/until_called_test.dart:274-299` + `packages/mocktail/test/mockito_compat/until_called_test.dart:340-368` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/verify_test.dart:232-247` + `packages/mocktail/test/mockito_compat/verify_test.dart:264-279` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mocktail_test.dart:308-323` + `packages/mocktail/test/mocktail_test.dart:326-351` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail/test/mockito_compat/invocation_matcher_test.dart:113-124` + `packages/mocktail/test/mockito_compat/invocation_matcher_test.dart:95-106` | ✅ pass | passing |
| mocktail | clearly_in | `packages/mocktail_image_network/test/mocktail_image_network_test.dart:100-110` + `packages/mocktail_image_network/test/mocktail_image_network_test.dart:55-65` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/FilterHandler.php:153-164` + `src/Monolog/Handler/SamplingHandler.php:81-92` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FingersCrossedHandlerTest.php:209-214` + `tests/Monolog/Handler/FingersCrossedHandlerTest.php:225-230` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/LogEntriesHandlerTest.php:37-46` + `tests/Monolog/Handler/LogmaticHandlerTest.php:37-46` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/SlackHandler.php:81-89` + `src/Monolog/Handler/SlackWebhookHandler.php:70-80` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/RedisHandlerTest.php:25-67` + `tests/Monolog/Handler/RedisPubSubHandlerTest.php:21-68` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FallbackGroupHandlerTest.php:142-173` + `tests/Monolog/Handler/GroupHandlerTest.php:121-152` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/GelfMessageFormatterTest.php:127-148` + `tests/Monolog/Formatter/GelfMessageFormatterTest.php:61-78` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/RotatingFileHandlerTest.php:177-214` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:253-290` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/GelfMessageFormatterTest.php:180-190` + `tests/Monolog/Formatter/GelfMessageFormatterTest.php:95-105` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/InsightOpsHandlerTest.php:70-80` + `tests/Monolog/Handler/LogmaticHandlerTest.php:74-84` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/GroupHandlerTest.php:17-153` + `tests/Monolog/Handler/WhatFailureGroupHandlerTest.php:17-172` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Processor/WebProcessorTest.php:50-57` + `tests/Monolog/Processor/WebProcessorTest.php:63-70` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/GroupHandlerTest.php:49-119` + `tests/Monolog/Handler/WhatFailureGroupHandlerTest.php:49-119` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:146-160` + `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:162-178` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/FlowdockHandler.php:48-64` + `src/Monolog/Handler/SlackHandler.php:66-79` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/SlackHandler.php:177-191` + `src/Monolog/Handler/SlackWebhookHandler.php:115-129` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/ElasticaHandlerTest.php:88-106` + `tests/Monolog/Handler/ElasticsearchHandlerTest.php:61-80` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/LoggerTest.php:427-436` + `tests/Monolog/LoggerTest.php:458-467` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Formatter/ChromePHPFormatter.php:29-41` + `src/Monolog/Handler/RollbarHandler.php:61-73` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/SyslogUdpHandlerTest.php:34-54` + `tests/Monolog/Handler/SyslogUdpHandlerTest.php:87-106` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FrankenPhpHandlerTest.php:79-89` + `tests/Monolog/Handler/ZendMonitorHandlerTest.php:44-54` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Processor/MemoryPeakUsageProcessorTest.php:1-40` + `tests/Monolog/Processor/MemoryUsageProcessorTest.php:1-40` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/LogstashFormatterTest.php:106-119` + `tests/Monolog/Formatter/LogstashFormatterTest.php:76-89` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/LoggerTest.php:412-439` + `tests/Monolog/LoggerTest.php:444-470` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Formatter/JsonFormatter.php:172-190` + `src/Monolog/Formatter/NormalizerFormatter.php:209-231` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/LogstashFormatterTest.php:104-129` + `tests/Monolog/Formatter/LogstashFormatterTest.php:74-99` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/InsightOpsHandler.php:23-74` + `src/Monolog/Handler/LogEntriesHandler.php:20-68` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/NormalizerFormatterTest.php:105-121` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:125-141` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FlowdockHandlerTest.php:26-43` + `tests/Monolog/Handler/SlackHandlerTest.php:28-45` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/FallbackGroupHandler.php:24-68` + `src/Monolog/Handler/WhatFailureGroupHandler.php:23-80` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/FrankenPhpHandler.php:28-100` + `src/Monolog/Handler/ZendMonitorHandler.php:25-90` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/JsonFormatterTest.php:267-278` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:603-613` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Formatter/GelfMessageFormatter.php:117-128` + `src/Monolog/Formatter/GelfMessageFormatter.php:130-141` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/NormalizerFormatterTest.php:200-227` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:238-261` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:33-47` + `tests/Monolog/Handler/BrowserConsoleHandlerTest.php:49-63` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/LogEntriesHandlerTest.php:20-86` + `tests/Monolog/Handler/LogmaticHandlerTest.php:20-86` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FlowdockHandlerTest.php:66-88` + `tests/Monolog/Handler/SlackHandlerTest.php:132-154` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Processor/MemoryPeakUsageProcessor.php:1-39` + `src/Monolog/Processor/MemoryUsageProcessor.php:1-39` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/GelfHandlerTest.php:51-70` + `tests/Monolog/Handler/GelfHandlerTest.php:72-91` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/AmqpHandlerTest.php:105-132` + `tests/Monolog/Handler/AmqpHandlerTest.php:48-72` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FlowdockHandlerTest.php:69-76` + `tests/Monolog/Handler/LogEntriesHandlerTest.php:67-75` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/LogEntriesHandlerTest.php:25-62` + `tests/Monolog/Handler/LogmaticHandlerTest.php:25-62` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/NormalizerFormatterTest.php:463-474` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:530-542` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FirePHPHandlerTest.php:29-40` + `tests/Monolog/Handler/FirePHPHandlerTest.php:48-63` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Formatter/HtmlFormatter.php:102-109` + `src/Monolog/Formatter/HtmlFormatter.php:94-101` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/SocketHandlerTest.php:121-131` + `tests/Monolog/Handler/SocketHandlerTest.php:95-105` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Formatter/ChromePHPFormatter.php:29-41` + `src/Monolog/Formatter/WildfireFormatter.php:42-54` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/RotatingFileHandlerTest.php:148-175` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:224-251` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/JsonFormatterTest.php:307-337` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:263-293` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/RotatingFileHandlerTest.php:162-169` + `tests/Monolog/Handler/RotatingFileHandlerTest.php:367-373` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FallbackGroupHandlerTest.php:96-140` + `tests/Monolog/Handler/GroupHandlerTest.php:76-119` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FingersCrossedHandlerTest.php:255-266` + `tests/Monolog/Handler/FingersCrossedHandlerTest.php:271-282` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/FallbackGroupHandlerTest.php:114-156` + `tests/Monolog/Handler/GroupHandlerTest.php:94-135` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/ElasticaHandler.php:66-125` + `src/Monolog/Handler/ElasticsearchHandler.php:82-152` | ✅ pass | passing |
| monolog | clearly_in | `src/Monolog/Handler/FingersCrossedHandler.php:202-241` + `src/Monolog/Handler/SamplingHandler.php:81-120` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/ElasticaHandlerTest.php:121-130` + `tests/Monolog/Handler/ElasticsearchHandlerTest.php:71-80` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Formatter/JsonFormatterTest.php:174-195` + `tests/Monolog/Formatter/NormalizerFormatterTest.php:335-355` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/LoggerTest.php:563-576` + `tests/Monolog/LoggerTest.php:585-599` | ✅ pass | passing |
| monolog | clearly_in | `tests/Monolog/Handler/BufferHandlerTest.php:131-136` + `tests/Monolog/Handler/DeduplicationHandlerTest.php:27-34` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/src/redis/redis.controller.ts:25-47` + `integration/microservices/src/tcp-tls/app.controller.ts:69-91` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/media-type-versioning-fastify.spec.ts:409-451` + `integration/versioning/e2e/media-type-versioning.spec.ts:399-441` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/e2e/sum-rpc-tls.spec.ts:61-129` + `integration/microservices/e2e/sum-rpc.spec.ts:46-114` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/e2e/sum-rpc-tls.spec.ts:68-137` + `integration/microservices/e2e/sum-rpc.spec.ts:53-122` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/uri-versioning-fastify.spec.ts:207-363` + `integration/versioning/e2e/uri-versioning.spec.ts:197-353` | ✅ pass | passing |
| nest | clearly_in | `packages/core/test/errors/test/exceptions-zone.spec.ts:21-33` + `packages/core/test/errors/test/exceptions-zone.spec.ts:76-88` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/media-type-versioning-fastify.spec.ts:31-343` + `integration/versioning/e2e/media-type-versioning.spec.ts:24-336` | ✅ pass | passing |
| nest | clearly_in | `packages/core/test/hooks/before-app-shutdown.hook.spec.ts:19-37` + `packages/core/test/hooks/on-module-init.hook.spec.ts:19-37` | ✅ pass | passing |
| nest | clearly_in | `sample/31-graphql-federation-code-first/users-application/src/users/users.resolver.spec.ts:1-39` + `sample/32-graphql-federation-schema-first/users-application/src/users/users.resolver.spec.ts:1-39` | ✅ pass | passing |
| nest | clearly_in | `sample/31-graphql-federation-code-first/users-application/eslint.config.mjs:8-42` + `sample/32-graphql-federation-schema-first/posts-application/eslint.config.mjs:8-42` | ✅ pass | passing |
| nest | clearly_in | `sample/31-graphql-federation-code-first/users-application/src/users/users.resolver.spec.ts:6-39` + `sample/32-graphql-federation-schema-first/users-application/src/users/users.resolver.spec.ts:6-39` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/header-versioning-fastify.spec.ts:31-343` + `integration/versioning/e2e/header-versioning.spec.ts:24-336` | ✅ pass | passing |
| nest | clearly_in | `sample/01-cats-app/e2e/cats/cats.e2e-spec.ts:1-34` + `sample/36-hmr-esm/e2e/cats/cats.e2e-spec.ts:1-34` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/src/app.controller.ts:99-114` + `integration/microservices/src/tcp-tls/app.controller.ts:112-127` | ✅ pass | passing |
| nest | clearly_in | `integration/inspector/src/users/users.controller.ts:14-42` + `integration/repl/src/users/users.controller.ts:14-42` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/custom-versioning-fastify.spec.ts:299-317` + `integration/versioning/e2e/custom-versioning-fastify.spec.ts:780-798` | ✅ pass | passing |
| nest | clearly_in | `integration/versioning/e2e/header-versioning-fastify.spec.ts:365-671` + `integration/versioning/e2e/header-versioning.spec.ts:355-661` | ✅ pass | passing |
| nest | clearly_in | `sample/01-cats-app/src/cats/cats.controller.ts:15-32` + `sample/36-hmr-esm/src/cats/cats.controller.ts:15-32` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/src/app.controller.ts:32-45` + `integration/microservices/src/tcp-tls/app.controller.ts:45-58` | ✅ pass | passing |
| nest | clearly_in | `integration/microservices/src/app.controller.ts:99-117` + `integration/microservices/src/tcp-tls/app.controller.ts:112-130` | ✅ pass | passing |
| nest | clearly_in | `packages/platform-express/multer/interceptors/file.interceptor.ts:1-67` + `packages/platform-express/multer/interceptors/files.interceptor.ts:1-70` | ✅ pass | passing |
| nest | clearly_out | `packages/common/decorators/core/index.ts:1-14` + `packages/microservices/index.ts:7-21` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextWriterAsyncTests.cs:521-552` + `Src/Newtonsoft.Json.Tests/JsonTextWriterAsyncTests.cs:600-643` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:141-150` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:165-174` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:588-599` + `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:609-620` | 🔴 fail | on record as failing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonValidatingReaderTests.cs:1056-1066` + `Src/Newtonsoft.Json.Tests/JsonValidatingReaderTests.cs:159-169` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Linq/JsonPath/JPathExecuteTests.cs:831-838` + `Src/Newtonsoft.Json.Tests/Linq/JsonPath/JPathExecuteTests.cs:848-855` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Documentation/Samples/Serializer/ErrorHandlingAttribute.cs:45-76` + `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:289-320` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Documentation/TraceWriterTests.cs:135-171` + `Src/Newtonsoft.Json.Tests/Serialization/TraceWriterTests.cs:285-305` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/JsonTextReader.Async.cs:1567-1675` + `Src/Newtonsoft.Json/JsonTextReader.cs:497-601` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:881-909` + `Src/Newtonsoft.Json.Tests/Serialization/ContractResolverTests.cs:56-86` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/Linq/JValue.cs:278-287` + `Src/Newtonsoft.Json/Linq/JValue.cs:303-312` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Documentation/Samples/Serializer/SerializationCallbackAttributes.cs:43-142` + `Src/Newtonsoft.Json.Tests/Documentation/SerializationTests.cs:113-231` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:112-285` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:198-371` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:1013-1019` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1167-1173` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:740-828` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:757-845` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Linq/MergeTests.cs:306-353` + `Src/Newtonsoft.Json.Tests/Linq/MergeTests.cs:514-558` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/Converters/XmlNodeConverter.cs:174-186` + `Src/Newtonsoft.Json/Converters/XmlNodeConverter.cs:420-432` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/Linq/JContainer.Async.cs:82-160` + `Src/Newtonsoft.Json/Linq/JContainer.cs:821-899` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:323-332` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:468-477` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:115-122` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:81-88` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:526-537` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadAsyncTests.cs:589-600` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1064-1086` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:1088-1109` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Serialization/DateOnlyTests.cs:55-89` + `Src/Newtonsoft.Json.Tests/Serialization/TimeOnlyTests.cs:64-98` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Linq/JTokenTests.cs:836-849` + `Src/Newtonsoft.Json.Tests/Linq/JTokenTests.cs:886-899` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:324-332` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:387-395` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/Utilities/LinqBridge.cs:1752-1758` + `Src/Newtonsoft.Json/Utilities/LinqBridge.cs:1797-1803` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextWriterTest.cs:599-607` + `Src/Newtonsoft.Json.Tests/JsonTextWriterTest.cs:647-655` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Serialization/NullValueHandlingTests.cs:135-143` + `Src/Newtonsoft.Json.Tests/Serialization/NullValueHandlingTests.cs:156-164` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:508-530` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/ReadTests.cs:574-596` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:209-219` + `Src/Newtonsoft.Json.Tests/JsonTextReaderTests/MiscTests.cs:902-912` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:539-566` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderAsyncTests.cs:568-595` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/TestObjects/EnumerableClass.cs:1-34` + `Src/Newtonsoft.Json.Tests/TestObjects/GenericIEnumerableWithImplicitConversion.cs:1-34` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:698-708` + `Src/Newtonsoft.Json.Tests/Bson/BsonReaderTests.cs:814-824` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json/JsonTextReader.cs:2601-2621` + `Src/Newtonsoft.Json/JsonTextWriter.cs:191-207` | ✅ pass | passing |
| newtonsoft.json | clearly_in | `Src/Newtonsoft.Json.Tests/Serialization/JsonSerializerTest.cs:7316-7329` + `Src/Newtonsoft.Json.Tests/Serialization/JsonSerializerTest.cs:7333-7346` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2082-2107` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2472-2497` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:467-479` + `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:804-816` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Retry/RetryAsyncSpecs.cs:378-382` + `src/Polly.SharedSpecs/Retry/RetryTResultSpecsAsync.cs:438-442` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1901-1925` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2537-2561` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/PolicyContextAndKeyAsyncSpecs.cs:206-283` + `src/Polly.SharedSpecs/PolicyContextAndKeySpecs.cs:202-280` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:1198-1203` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:1132-1137` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecs.cs:37-109` + `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecsAsync.cs:38-109` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:65-157` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:66-158` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:65-113` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:66-114` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2161-2184` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2223-2246` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1470-1494` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1508-1532` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1280-1303` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1427-1450` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Caching/SerializingCacheProviderAsyncSpecs.cs:14-46` + `src/Polly.SharedSpecs/Caching/SerializingCacheProviderSpecs.cs:12-44` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Retry/WaitAndRetrySpecs.cs:330-341` + `src/Polly.SharedSpecs/Retry/WaitAndRetrySpecs.cs:375-386` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.Shared/Retry/RetrySyntaxAsync.cs:531-535` + `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:403-407` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2471-2494` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:2571-2594` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecs.cs:17-104` + `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecsAsync.cs:18-104` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:998-1010` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:988-1000` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultMixedResultExceptionSpecs.cs:387-398` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultMixedResultExceptionSpecs.cs:42-53` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/Caching/CacheAsyncSpecs.cs:17-48` + `src/Polly.SharedSpecs/Caching/CacheSpecs.cs:16-47` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:1027-1043` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:981-997` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:234-251` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:665-682` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:43-137` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:44-138` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:408-427` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:496-515` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:294-312` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:333-351` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:152-156` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:153-157` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:813-820` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:805-812` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:298-315` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:337-354` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.Shared/Wrap/PolicyWrapSyntax.cs:8-42` + `src/Polly.Shared/Wrap/PolicyWrapSyntaxAsync.cs:8-42` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:404-429` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:501-526` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.Shared/Caching/SerializingCacheProvider.cs:10-51` + `src/Polly.Shared/Caching/SerializingCacheProvider.cs:59-100` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1482-1497` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1521-1536` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2437-2461` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2537-2561` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1280-1303` + `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:1378-1401` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.Shared/Wrap/PolicyWrapSyntax.cs:44-77` + `src/Polly.Shared/Wrap/PolicyWrapSyntaxAsync.cs:44-77` | ✅ pass | passing |
| polly | clearly_in | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1583-1608` + `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:501-526` | ✅ pass | passing |
| polly | clearly_out | `src/Polly.Shared/Bulkhead/BulkheadEngine.cs:15-21` + `src/Polly.Shared/Caching/CacheEngine.cs:13-18` | ✅ pass | passing |
| polly | clearly_out | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:334-337` + `src/Polly.SharedSpecs/Fallback/FallbackAsyncSpecs.cs:577-604` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip_any_while.rs:10-30` + `src/iter/take_any_while.rs:10-30` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/flatten.rs:7-76` + `src/iter/flatten_iter.rs:7-75` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/mod.rs:940-974` + `src/slice/mod.rs:978-1014` | ✅ pass | passing |
| rayon | clearly_in | `src/str.rs:486-496` + `src/str.rs:586-596` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold.rs:226-288` + `src/iter/skip_any_while.rs:55-110` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/chunks.rs:38-59` + `src/slice/rchunks.rs:218-239` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/multizip.rs:151-165` + `src/iter/multizip.rs:167-181` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/reduce.rs:4-70` + `src/iter/try_reduce.rs:8-78` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/flat_map_iter.rs:51-91` + `src/iter/map.rs:162-202` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/btree_set.rs:18-45` + `src/result.rs:31-48` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/join_microbench.rs:36-45` + `rayon-demo/src/join_microbench.rs:47-56` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/inspect.rs:19-155` + `src/iter/update.rs:18-170` | ✅ pass | passing |
| rayon | clearly_in | `tests/chars.rs:22-38` + `tests/chars.rs:4-19` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/str_split.rs:24-36` + `rayon-demo/src/str_split.rs:42-54` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/flat_map.rs:60-152` + `src/iter/flatten.rs:48-133` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:1487-1500` + `src/iter/test.rs:1502-1515` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold.rs:5-59` + `src/iter/try_fold.rs:9-67` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/len.rs:137-220` + `src/iter/len.rs:7-90` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:499-525` + `src/iter/test.rs:529-555` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold_chunks.rs:14-122` + `src/iter/fold_chunks_with.rs:14-121` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold_chunks.rs:157-194` + `src/iter/fold_chunks_with.rs:153-192` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/find/mod.rs:130-148` + `rayon-demo/src/find/mod.rs:83-101` | ✅ pass | passing |
| rayon | clearly_in | `src/split_producer.rs:31-42` + `src/split_producer.rs:44-55` | ✅ pass | passing |
| rayon | clearly_in | `tests/clones.rs:192-223` + `tests/debug.rs:202-234` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip_any_while.rs:17-127` + `src/iter/take_any_while.rs:17-127` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/rchunks.rs:254-267` + `src/slice/rchunks.rs:74-87` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip_any_while.rs:55-110` + `src/iter/take_any_while.rs:55-116` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/collect/test.rs:156-161` + `src/iter/collect/test.rs:192-198` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/sort.rs:139-149` + `rayon-demo/src/sort.rs:175-210` | ✅ pass | passing |
| rayon | clearly_in | `tests/clones.rs:56-64` + `tests/debug.rs:29-37` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/find/mod.rs:131-150` + `rayon-demo/src/find/mod.rs:84-103` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/inspect.rs:14-102` + `src/iter/map_with.rs:340-438` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/map.rs:11-50` + `src/iter/update.rs:11-49` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold.rs:68-82` + `src/iter/try_fold.rs:76-90` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:1316-1325` + `src/iter/test.rs:1333-1342` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/take_any.rs:57-67` + `src/iter/while_some.rs:60-70` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/collect/test.rs:188-205` + `src/iter/collect/test.rs:207-224` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/nbody/nbody.rs:194-209` + `rayon-demo/src/nbody/nbody.rs:333-348` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/nbody/nbody.rs:297-328` + `rayon-demo/src/nbody/nbody.rs:440-471` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/vec_deque.rs:100-149` + `src/vec.rs:96-159` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/empty.rs:50-63` + `src/iter/once.rs:35-48` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip.rs:1-92` + `src/iter/take.rs:1-83` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/chunks.rs:192-245` + `src/slice/rchunks.rs:191-244` | ✅ pass | passing |
| rayon | clearly_in | `rayon-core/src/scope/test.rs:302-342` + `rayon-core/src/thread_pool/test.rs:231-262` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/btree_set.rs:14-45` + `src/collections/linked_list.rs:14-50` | ✅ pass | passing |
| rayon | clearly_in | `rayon-core/src/broadcast/test.rs:163-168` + `rayon-core/src/broadcast/test.rs:201-206` | 🔴 fail | on record as failing |
| rayon | clearly_in | `src/iter/flat_map.rs:1-152` + `src/iter/positions.rs:1-132` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/sort.rs:132-148` + `rayon-demo/src/sort.rs:163-204` | ✅ pass | passing |
| rayon | clearly_in | `src/range.rs:50-140` + `src/range_inclusive.rs:45-148` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/flatten.rs:8-90` + `src/iter/flatten_iter.rs:9-93` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/chunks.rs:40-87` + `src/iter/step_by.rs:40-87` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/tree.rs:47-62` + `rayon-demo/src/tree.rs:64-79` | ✅ pass | passing |
| rayon | clearly_in | `rayon-core/src/scope/mod.rs:434-448` + `rayon-core/src/scope/mod.rs:492-506` | ✅ pass | passing |
| rayon | clearly_in | `rayon-demo/src/find/mod.rs:131-151` + `rayon-demo/src/find/mod.rs:84-104` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip_any.rs:43-96` + `src/iter/while_some.rs:46-99` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/map_with.rs:117-175` + `src/iter/map_with.rs:442-500` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold_chunks.rs:14-61` + `src/iter/map_with.rs:12-54` | ✅ pass | passing |
| rayon | clearly_in | `tests/octillion.rs:101-115` + `tests/octillion.rs:70-87` | ✅ pass | passing |
| rayon | clearly_in | `src/str.rs:565-583` + `src/str.rs:609-627` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/binary_heap.rs:60-120` + `src/collections/vec_deque.rs:86-149` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/chunks.rs:91-189` + `src/slice/rchunks.rs:91-188` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:432-449` + `src/iter/test.rs:450-467` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/vec_deque.rs:43-89` + `src/option.rs:83-137` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/fold.rs:258-268` + `src/iter/map_with.rs:256-266` | ✅ pass | passing |
| rayon | clearly_in | `tests/octillion.rs:130-140` + `tests/octillion.rs:141-151` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/inspect.rs:52-199` + `src/iter/update.rs:31-200` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/map_with.rs:458-468` + `src/iter/map_with.rs:537-547` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/inspect.rs:111-213` + `src/iter/update.rs:105-219` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/filter_map.rs:10-22` + `src/iter/flat_map.rs:10-20` | ✅ pass | passing |
| rayon | clearly_in | `src/slice/chunks.rs:139-160` + `src/slice/chunks.rs:337-358` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/product.rs:1-114` + `src/iter/sum.rs:1-110` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:1133-1156` + `src/iter/test.rs:1157-1180` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/skip_any.rs:53-74` + `src/iter/while_some.rs:56-77` | ✅ pass | passing |
| rayon | clearly_in | `src/range.rs:84-195` + `src/range_inclusive.rs:92-205` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/cloned.rs:133-170` + `src/iter/copied.rs:133-170` | ✅ pass | passing |
| rayon | clearly_in | `src/collections/binary_heap.rs:37-63` + `src/collections/vec_deque.rs:32-66` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/inspect.rs:160-218` + `src/iter/map.rs:162-222` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/collect/test.rs:49-59` + `src/iter/collect/test.rs:65-75` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/test.rs:1142-1164` + `src/iter/test.rs:1166-1188` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/map_with.rs:117-175` + `src/iter/panic_fuse.rs:121-171` | ✅ pass | passing |
| rayon | clearly_in | `src/iter/cloned.rs:133-188` + `src/iter/flatten_iter.rs:38-93` | ✅ pass | passing |
| react | clearly_in | `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/repro-dont-memoize-array-with-capturing-map-after-hook.js:1-26` + `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/repro-dont-memoize-array-with-mutable-map-after-hook.js:1-26` | ✅ pass | passing |
| react | clearly_in | `packages/react-server-dom-webpack/src/__tests__/ReactFlightDOM-test.js:1268-1313` + `packages/react-server-dom-webpack/src/__tests__/ReactFlightDOM-test.js:1315-1372` | ✅ pass | passing |
| react | clearly_in | `packages/react-dom/src/__tests__/ReactDOMServerIntegrationAttributes-test.js:15-35` + `packages/react-dom/src/__tests__/ReactDOMServerIntegrationElements-test.js:17-36` | ✅ pass | passing |
| react | clearly_in | `compiler/packages/babel-plugin-react-compiler/src/HIR/HIR.ts:1554-1589` + `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingEffects.ts:2791-2823` | ✅ pass | passing |
| react | clearly_in | `packages/react-reconciler/src/ReactFiberPerformanceTrack.js:338-419` + `packages/react-reconciler/src/ReactFiberPerformanceTrack.js:421-486` | ✅ pass | passing |
| react | clearly_out | `fixtures/attribute-behavior/src/attributes.js:1019-1030` + `fixtures/attribute-behavior/src/attributes.js:1941-1952` | ✅ pass | passing |
| react | clearly_out | `compiler/apps/playground/colors.js:15-22` + `packages/react-dom-bindings/src/shared/possibleStandardNames.js:439-446` | ✅ pass | passing |
| react | clearly_out | `packages/react-dom-bindings/src/shared/possibleStandardNames.js:236-243` + `packages/react-dom-bindings/src/shared/possibleStandardNames.js:458-465` | ✅ pass | passing |
| ripgrep | clearly_in | `crates/searcher/src/searcher/core.rs:561-580` + `crates/searcher/src/searcher/core.rs:592-611` | ✅ pass | passing |
| ripgrep | clearly_in | `tests/json.rs:159-178` + `tests/json.rs:210-229` | ✅ pass | passing |
| ripgrep | clearly_in | `crates/printer/src/json.rs:1033-1039` + `crates/printer/src/json.rs:1049-1055` | ✅ pass | passing |
| ripgrep | clearly_in | `crates/printer/src/summary.rs:1054-1064` + `crates/printer/src/summary.rs:921-931` | ✅ pass | passing |
| ripgrep | clearly_in | `crates/globset/src/lib.rs:1035-1041` + `crates/globset/src/lib.rs:1043-1049` | ✅ pass | passing |
| ripgrep | clearly_out | `crates/ignore/src/default_types.rs:112-116` + `crates/ignore/src/default_types.rs:121-125` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1630-1647` + `test_suite/tests/test_ser.rs:316-333` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/regression/issue2565.rs:37-46` + `test_suite/tests/test_annotations.rs:2632-2643` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/ser.rs:949-959` + `serde/src/private/ser.rs:979-989` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2918-2972` + `test_suite/tests/test_annotations.rs:2974-3072` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2227-2259` + `serde_core/src/de/impls.rs:2558-2589` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1180-1234` + `serde/src/private/de.rs:2136-2190` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/ser.rs:502-573` + `serde_derive/src/ser.rs:575-639` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:103-114` + `test_suite/tests/test_enum_adjacently_tagged.rs:86-97` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:1024-1080` + `test_suite/tests/test_de_error.rs:967-1023` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1792-1816` + `test_suite/tests/test_de.rs:1819-1843` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/internals/attr.rs:1609-1631` + `serde_derive/src/internals/attr.rs:1633-1655` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:168-180` + `test_suite/tests/test_ser.rs:318-330` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_identifier.rs:13-72` + `test_suite/tests/test_identifier.rs:80-139` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:128-138` + `test_suite/tests/test_annotations.rs:2506-2516` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/value.rs:474-505` + `serde_core/src/de/value.rs:677-709` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:1974-2010` + `test_suite/tests/test_annotations.rs:2012-2049` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1135-1142` + `serde/src/private/de.rs:2089-2096` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:567-583` + `test_suite/tests/test_enum_internally_tagged.rs:584-600` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:1164-1211` + `test_suite/tests/test_enum_untagged.rs:370-410` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:1481-1497` + `serde_core/src/ser/impls.rs:404-420` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3424-3433` + `test_suite/tests/test_annotations.rs:3488-3497` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2602-2730` + `serde_core/src/de/impls.rs:2741-2869` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:1329-1344` + `test_suite/tests/test_de_error.rs:1345-1360` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/de.rs:39-76` + `serde_derive/src/ser.rs:26-58` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3134-3236` + `test_suite/tests/test_enum_adjacently_tagged.rs:523-570` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2747-2762` + `test_suite/tests/test_annotations.rs:2793-2808` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2296-2305` + `serde_core/src/de/impls.rs:2494-2503` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_roundtrip.rs:9-22` + `test_suite/tests/test_ser.rs:762-775` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2793-2824` + `test_suite/tests/test_enum_internally_tagged.rs:647-680` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3116-3129` + `test_suite/tests/test_annotations.rs:3139-3154` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:600-643` + `test_suite/tests/test_enum_internally_tagged.rs:1395-1429` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2954-2992` + `test_suite/tests/test_annotations.rs:2994-3072` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/ui/with-variant/skip_de_struct_field.rs:1-13` + `test_suite/tests/ui/with-variant/skip_ser_struct_field.rs:1-13` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/lib.rs:113-119` + `serde_derive/src/lib.rs:121-127` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1173-1227` + `serde/src/private/de.rs:2129-2183` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:1182-1192` + `test_suite/tests/test_enum_internally_tagged.rs:1254-1264` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:952-964` + `test_suite/tests/test_de.rs:965-977` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2842-2853` + `test_suite/tests/test_annotations.rs:3336-3347` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:67-75` + `test_suite/tests/test_enum_adjacently_tagged.rs:773-781` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1822-1832` + `test_suite/tests/test_ser.rs:432-442` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:2718-2771` + `serde_core/src/de/value.rs:1475-1528` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/value.rs:373-440` + `serde_core/src/de/value.rs:677-747` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2187-2262` + `serde_core/src/de/impls.rs:2326-2405` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:1001-1036` + `test_suite/tests/test_enum_internally_tagged.rs:973-1000` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:960-972` + `test_suite/tests/test_enum_internally_tagged.rs:973-985` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/ser/impls.rs:869-883` + `serde_core/src/ser/impls.rs:885-902` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:121-155` + `test_suite/tests/test_enum_adjacently_tagged.rs:227-262` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3293-3301` + `test_suite/tests/test_enum_adjacently_tagged.rs:561-568` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/value.rs:1027-1043` + `serde_core/src/de/value.rs:1261-1277` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:601-619` + `test_suite/tests/test_enum_untagged.rs:512-530` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1843-1851` + `serde/src/private/de.rs:2793-2801` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2763-2778` + `test_suite/tests/test_annotations.rs:2809-2824` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:550-569` + `test_suite/tests/test_macros.rs:193-208` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1772-1865` + `serde_core/src/de/value.rs:1475-1577` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/de/enum_internally.rs:67-106` + `serde_derive/src/de/enum_untagged.rs:62-109` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/value.rs:900-938` + `serde_core/src/de/value.rs:959-997` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:189-198` + `test_suite/tests/test_enum_adjacently_tagged.rs:773-782` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:107-138` + `test_suite/tests/test_de_error.rs:38-68` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2610-2645` + `test_suite/tests/test_enum_internally_tagged.rs:1395-1429` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_untagged.rs:397-410` + `test_suite/tests/test_enum_untagged.rs:412-425` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:1025-1055` + `test_suite/tests/test_de_error.rs:743-773` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2138-2185` + `serde_core/src/de/impls.rs:2475-2522` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1690-1767` + `serde_core/src/de/value.rs:1353-1437` | ✅ pass | passing |
| serde | clearly_in | `serde/src/lib.rs:111-155` + `serde_core/src/lib.rs:46-90` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1520-1537` + `serde/src/private/de.rs:1618-1635` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/internals/attr.rs:383-390` + `serde_derive/src/internals/attr.rs:405-412` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2959-2968` + `test_suite/tests/test_annotations.rs:3254-3263` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2610-2645` + `test_suite/tests/test_enum_adjacently_tagged.rs:600-643` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:1385-1400` + `test_suite/tests/test_de_error.rs:1401-1416` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:335-392` + `test_suite/tests/test_enum_internally_tagged.rs:748-805` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:664-702` + `test_suite/tests/test_de_error.rs:703-741` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de_error.rs:1080-1135` + `test_suite/tests/test_de_error.rs:966-1021` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2921-2930` + `test_suite/tests/test_annotations.rs:3085-3094` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1520-1562` + `serde_core/src/de/value.rs:1023-1067` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:118-164` + `test_suite/tests/test_annotations.rs:227-275` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1406-1419` + `test_suite/tests/test_de.rs:1435-1448` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:3102-3133` + `serde_core/src/ser/impls.rs:1009-1045` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/ser/mod.rs:1518-1623` + `serde_core/src/ser/mod.rs:1675-1738` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1619-1666` + `test_suite/tests/test_ser.rs:311-353` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:2878-2899` + `serde/src/private/de.rs:2901-2926` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:824-831` + `test_suite/tests/test_de.rs:1518-1525` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:1124-1140` + `test_suite/tests/test_enum_internally_tagged.rs:1142-1158` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:138-146` + `test_suite/tests/test_enum_internally_tagged.rs:187-195` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3076-3237` + `test_suite/tests/test_annotations.rs:3240-3416` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/ser/mod.rs:1901-1930` + `serde_core/src/ser/mod.rs:1971-2000` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/ser.rs:876-883` + `serde/src/private/ser.rs:991-998` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:2478-2495` + `serde/src/private/de.rs:2564-2581` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:1418-1429` + `test_suite/tests/test_annotations.rs:1431-1442` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/value.rs:473-540` + `serde_core/src/de/value.rs:563-630` | ✅ pass | passing |
| serde | clearly_in | `serde_core/src/de/impls.rs:2469-2522` + `serde_core/src/de/impls.rs:2767-2817` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:25-59` + `test_suite/tests/test_enum_adjacently_tagged.rs:82-116` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:3037-3046` + `test_suite/tests/test_annotations.rs:3189-3198` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:117-159` + `test_suite/tests/test_enum_internally_tagged.rs:166-208` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:1730-1739` + `test_suite/tests/test_annotations.rs:676-685` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/de/enum_externally.rs:79-115` + `serde_derive/src/de/unit.rs:19-51` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:349-378` + `test_suite/tests/test_enum_internally_tagged.rs:762-791` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/regression/issue2371.rs:44-53` + `test_suite/tests/regression/issue2371.rs:8-17` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/ui/conflict/alias-enum.rs:7-17` + `test_suite/tests/ui/conflict/alias.rs:4-14` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_adjacently_tagged.rs:480-487` + `test_suite/tests/test_enum_adjacently_tagged.rs:561-568` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:1227-1274` + `test_suite/tests/test_annotations.rs:1277-1324` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_ser.rs:884-894` + `test_suite/tests/test_ser.rs:896-906` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_ser.rs:714-720` + `test_suite/tests/test_ser.rs:721-727` | ✅ pass | passing |
| serde | clearly_in | `serde/src/private/de.rs:1166-1220` + `serde/src/private/de.rs:2122-2176` | ✅ pass | passing |
| serde | clearly_in | `serde_derive/src/de/struct_.rs:293-311` + `serde_derive/src/de/struct_.rs:588-607` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_de.rs:1093-1105` + `test_suite/tests/test_de.rs:988-1000` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_annotations.rs:2612-2623` + `test_suite/tests/test_enum_untagged.rs:513-525` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:51-62` + `test_suite/tests/test_enum_internally_tagged.rs:63-74` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_untagged.rs:28-52` + `test_suite/tests/test_macros.rs:559-589` | ✅ pass | passing |
| serde | clearly_in | `test_suite/tests/test_enum_internally_tagged.rs:475-520` + `test_suite/tests/test_enum_internally_tagged.rs:808-853` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Formatting/Json/JsonValueFormatter.cs:314-331` + `src/Serilog/Formatting/Json/JsonValueFormatter.cs:333-350` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Configuration/LoggerEnrichmentConfiguration.cs:22-66` + `src/Serilog/Configuration/LoggerFilterConfiguration.cs:22-62` | ✅ pass | passing |
| serilog | clearly_in | `test/Serilog.PerformanceTests/Support/ConcurrentDictionaryTemplateCache.cs:4-46` + `test/Serilog.PerformanceTests/Support/DictionaryMessageTemplateCache.cs:4-50` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Core/Pipeline/MessageTemplateCache.cs:18-73` + `test/Serilog.PerformanceTests/Support/DictionaryMessageTemplateCache.cs:4-50` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Core/Sinks/AggregateSink.cs:19-47` + `src/Serilog/Core/Sinks/DisposingAggregateSink.cs:22-49` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Formatting/Json/JsonValueFormatter.cs:469-484` + `src/Serilog/Formatting/Json/JsonValueFormatter.cs:486-501` | ✅ pass | passing |
| serilog | clearly_in | `src/Serilog/Policies/ProjectedDestructuringPolicy.cs:17-42` + `test/Serilog.Tests/Configuration/LoggerConfigurationTests.cs:136-161` | ✅ pass | passing |
| tokio | clearly_in | `tokio/tests/dump.rs:35-62` + `tokio/tests/dump.rs:73-100` | ✅ pass | passing |
| tokio | clearly_in | `tokio/tests/tcp_stream.rs:297-312` + `tokio/tests/tcp_stream.rs:38-53` | ✅ pass | passing |
| tokio | clearly_in | `tokio/src/runtime/time/wheel/level.rs:150-192` + `tokio/src/runtime/time_alt/wheel/level.rs:152-194` | ✅ pass | passing |
| tokio | clearly_in | `tokio-util/src/time/wheel/mod.rs:278-317` + `tokio/src/runtime/time/wheel/mod.rs:292-331` | ✅ pass | passing |
| tokio | clearly_in | `tokio/tests/tcp_stream.rs:181-213` + `tokio/tests/uds_stream.rs:209-241` | ✅ pass | passing |
| tokio | clearly_in | `tokio/src/net/tcp/socket.rs:595-615` + `tokio/src/net/tcp/socket.rs:655-675` | ✅ pass | passing |
| tokio | clearly_in | `examples/connect-tcp.rs:28-44` + `examples/connect-udp.rs:28-44` | ✅ pass | passing |
| tokio | clearly_in | `tokio/src/fs/file/tests.rs:617-635` + `tokio/src/fs/file/tests.rs:651-669` | ✅ pass | passing |
| tokio | clearly_in | `tokio/src/runtime/io/scheduled_io.rs:39-100` + `tokio/src/runtime/task/core.rs:63-124` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:176-187` + `viper_test.go:268-279` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1905-1915` + `viper_test.go:1955-1965` | ✅ pass | passing |
| viper | clearly_in | `viper.go:1069-1079` + `viper.go:964-974` | ✅ pass | passing |
| viper | clearly_in | `flags_test.go:47-57` + `viper_test.go:1297-1306` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1901-1906` + `viper_test.go:1951-1956` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:218-227` + `viper_test.go:250-259` | ✅ pass | passing |
| viper | clearly_in | `viper.go:1224-1253` + `viper.go:1326-1355` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1914-1922` + `viper_test.go:1964-1972` | ✅ pass | passing |
| viper | clearly_in | `flags_test.go:46-65` + `viper_test.go:1295-1314` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1912-1920` + `viper_test.go:1962-1970` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:616-626` + `viper_test.go:684-694` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:869-880` + `viper_test.go:883-894` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:218-226` + `viper_test.go:292-300` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:175-193` + `viper_test.go:195-212` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1551-1558` + `viper_test.go:490-497` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1140-1166` + `viper_test.go:1184-1210` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1029-1053` + `viper_test.go:1065-1089` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1006-1026` + `viper_test.go:1069-1088` | ✅ pass | passing |
| viper | clearly_in | `viper.go:1535-1551` + `viper.go:1557-1574` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1667-1687` + `viper_test.go:1690-1712` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1904-1914` + `viper_test.go:1954-1964` | ✅ pass | passing |
| viper | clearly_in | `remote.go:134-166` + `remote.go:93-122` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:338-350` + `viper_test.go:446-460` | ✅ pass | passing |
| viper | clearly_in | `viper.go:539-553` + `viper.go:572-587` | 🔴 fail | on record as failing |
| viper | clearly_in | `viper_test.go:1903-1913` + `viper_test.go:1953-1963` | ✅ pass | passing |
| viper | clearly_in | `internal/encoding/dotenv/map_utils.go:12-41` + `viper.go:1996-2025` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:214-232` + `viper_test.go:309-327` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1339-1362` + `viper_test.go:1382-1407` | ✅ pass | passing |
| viper | clearly_in | `encoding_test.go:21-38` + `encoding_test.go:87-104` | ✅ pass | passing |
| viper | clearly_in | `remote/remote.go:22-32` + `remote/remote.go:34-45` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1141-1165` + `viper_test.go:1184-1210` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1913-1921` + `viper_test.go:1963-1971` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1906-1916` + `viper_test.go:1956-1966` | ✅ pass | passing |
| viper | clearly_in | `viper.go:1583-1591` + `viper.go:1620-1628` | ✅ pass | passing |
| viper | clearly_in | `flags_test.go:12-43` + `viper_test.go:1092-1123` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1568-1574` + `viper_test.go:2127-2133` | ✅ pass | passing |
| viper | clearly_in | `internal/encoding/dotenv/codec_test.go:24-55` + `internal/encoding/toml/codec_test.go:66-97` | 🔴 fail | on record as failing |
| viper | clearly_in | `viper_test.go:1907-1917` + `viper_test.go:1957-1967` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:902-913` + `viper_test.go:940-951` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:385-396` + `viper_test.go:410-421` | ✅ pass | passing |
| viper | clearly_in | `internal/encoding/yaml/codec_test.go:11-72` + `internal/encoding/yaml/codec_test.go:75-95` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:270-280` + `viper_test.go:291-301` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1342-1362` + `viper_test.go:1386-1406` | ✅ pass | passing |
| viper | clearly_in | `viper.go:1922-1934` + `viper.go:1940-1952` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1902-1912` + `viper_test.go:1952-1962` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:359-382` + `viper_test.go:384-407` | ✅ pass | passing |
| viper | clearly_in | `internal/encoding/toml/codec_test.go:44-97` + `internal/encoding/yaml/codec_test.go:75-128` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:1144-1165` + `viper_test.go:1270-1291` | ✅ pass | passing |
| viper | clearly_in | `internal/encoding/json/codec_test.go:65-87` + `internal/encoding/toml/codec_test.go:75-97` | ✅ pass | passing |
| viper | clearly_in | `viper_test.go:384-407` + `viper_test.go:409-432` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:43-80` + `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:43-80` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/utils.ts:1307-1315` + `packages/vite/src/node/utils.ts:1412-1420` | ✅ pass | passing |
| vite | clearly_in | `playground/js-sourcemap/dep-class-field-sourcemap-babel/index.js:1-10` + `playground/js-sourcemap/dep-class-field-sourcemap-oxc/index.js:1-10` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/server/hmr.ts:561-575` + `packages/vite/src/node/server/hmr.ts:577-591` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:625-709` + `playground/hmr/__tests__/hmr.spec.ts:733-821` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:711-779` + `playground/hmr/__tests__/hmr.spec.ts:823-898` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/file-delete-restore/runtime.js:1-15` + `playground/hmr/file-delete-restore/runtime.js:1-16` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-full-bundle-mode/__tests__/hmr-full-bundle-mode.spec.ts:257-285` + `playground/hmr-full-bundle-mode/__tests__/hmr-full-bundle-mode.spec.ts:289-317` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/http.spec.ts:107-125` + `packages/vite/src/node/__tests__/http.spec.ts:63-81` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/hmr.ts:85-103` + `playground/hmr/hmr.ts:138-156` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:119-192` + `playground/hmr/__tests__/hmr.spec.ts:121-202` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a1.js:29-36` + `playground/multiple-entrypoints/entrypoints/a2.js:28-35` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/preview.ts:233-247` + `packages/vite/src/node/server/index.ts:1016-1037` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:96-115` + `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:100-117` | ✅ pass | passing |
| vite | clearly_in | `playground/fs-serve/root/vite.config-base.js:10-45` + `playground/fs-serve/root/vite.config.js:8-41` | ✅ pass | passing |
| vite | clearly_in | `packages/create-vite/template-preact-ts/src/app.tsx:33-102` + `packages/create-vite/template-qwik-ts/src/app.tsx:30-99` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/utils.spec.ts:1210-1226` + `packages/vite/src/node/__tests__/utils.spec.ts:1228-1244` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/build.spec.ts:1647-1659` + `packages/vite/src/node/__tests__/build.spec.ts:1676-1688` | ✅ pass | passing |
| vite | clearly_in | `packages/plugin-legacy/src/index.ts:1126-1137` + `packages/plugin-legacy/src/index.ts:1139-1150` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/es/worker-es.spec.ts:139-146` + `playground/worker/__tests__/iife/worker-iife.spec.ts:117-124` | ✅ pass | passing |
| vite | clearly_in | `playground/cli-module/__tests__/serve.ts:16-156` + `playground/cli/__tests__/serve.ts:21-145` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/server/middlewares/__tests__/indexHtml.spec.ts:41-62` + `packages/vite/src/node/server/middlewares/__tests__/indexHtml.spec.ts:76-94` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:151-192` + `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:194-235` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:885-923` + `playground/hmr/__tests__/hmr.spec.ts:1107-1156` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/utils.spec.ts:1194-1207` + `packages/vite/src/node/__tests__/utils.spec.ts:1230-1243` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a1.js:29-39` + `playground/multiple-entrypoints/entrypoints/a2.js:28-38` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/config.spec.ts:264-294` + `packages/vite/src/node/__tests__/config.spec.ts:427-457` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a3.js:27-40` + `playground/multiple-entrypoints/entrypoints/a4.js:26-39` | ✅ pass | passing |
| vite | clearly_in | `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:178-205` + `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:191-218` | 🔴 fail | on record as failing |
| vite | clearly_in | `playground/worker/__tests__/es/worker-es.spec.ts:6-28` + `playground/worker/__tests__/iife/worker-iife.spec.ts:16-37` | ✅ pass | passing |
| vite | clearly_in | `playground/optimize-missing-deps/server.js:2-66` + `playground/ssr/server.js:1-73` | ✅ pass | passing |
| vite | clearly_in | `playground/ssr-conditions/server.js:29-43` + `playground/ssr/server.js:22-36` | ✅ pass | passing |
| vite | clearly_in | `playground/resolve/__tests__/resolve.spec.ts:250-289` + `playground/resolve/__tests__/resolve.spec.ts:31-71` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:379-506` + `playground/hmr/__tests__/hmr.spec.ts:530-658` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:825-834` + `playground/hmr/__tests__/hmr.spec.ts:1139-1150` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr/__tests__/hmr.spec.ts:160-176` + `playground/hmr/__tests__/hmr.spec.ts:182-198` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/plugins/html.ts:896-906` + `packages/vite/src/node/plugins/html.ts:908-918` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr/__tests__/hmr.spec.ts:122-137` + `playground/hmr/__tests__/hmr.spec.ts:139-154` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/vite.config.ts:87-97` + `playground/hmr/vite.config.ts:150-160` | ✅ pass | passing |
| vite | clearly_in | `playground/tailwind-v3/__test__/tailwind-v3.spec.ts:15-90` + `playground/tailwind/__test__/tailwind.spec.ts:31-106` | ✅ pass | passing |
| vite | clearly_in | `playground/ssr-conditions/__tests__/serve.ts:10-62` + `playground/ssr-noexternal/__tests__/serve.ts:10-52` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/es/worker-es.spec.ts:100-128` + `playground/worker/__tests__/iife/worker-iife.spec.ts:85-113` | ✅ pass | passing |
| vite | clearly_in | `packages/create-vite/template-preact-ts/src/app.tsx:7-104` + `packages/create-vite/template-solid-ts/src/App.tsx:7-104` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/config.spec.ts:627-704` + `packages/vite/src/node/__tests__/config.spec.ts:706-783` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/client.d.ts:158-189` + `packages/vite/client.d.ts:244-277` | ✅ pass | passing |
| vite | clearly_in | `playground/css-lightningcss/__tests__/css-lightningcss.spec.ts:17-32` + `playground/css-lightningcss/__tests__/css-lightningcss.spec.ts:34-49` | ✅ pass | passing |
| vite | clearly_in | `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:1-269` + `playground/assets/__tests__/url-base/assets-url-base.spec.ts:1-271` | ✅ pass | passing |
| vite | clearly_in | `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:160-232` + `playground/assets/__tests__/runtime-base/assets-runtime-base.spec.ts:156-239` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a10.js:20-31` + `playground/multiple-entrypoints/entrypoints/a11.js:19-30` | 🔴 fail | on record as failing |
| vite | clearly_in | `playground/worker/vite.config-iife.js:5-35` + `playground/worker/vite.config-relative-base.js:5-33` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/plugins/resolve.ts:316-322` + `packages/vite/src/node/plugins/resolve.ts:855-861` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/preview.ts:174-190` + `packages/vite/src/node/server/index.ts:665-677` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/plugins/esbuild.spec.ts:116-129` + `packages/vite/src/node/__tests__/plugins/esbuild.spec.ts:148-161` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/vite.config-es.js:11-48` + `playground/worker/vite.config-iife.js:11-52` | ✅ pass | passing |
| vite | clearly_in | `playground/tailwind-v3/vite.config.ts:1-42` + `playground/tailwind/vite.config.ts:1-44` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/config.spec.ts:707-763` + `packages/vite/src/node/__tests__/config.spec.ts:786-842` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/ssr/__tests__/ssrTransform.spec.ts:424-461` + `packages/vite/src/node/ssr/__tests__/ssrTransform.spec.ts:463-498` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/ssr/runtime/__tests__/server-runtime.spec.ts:364-372` + `packages/vite/src/node/ssr/runtime/__tests__/server-runtime.spec.ts:384-392` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/__tests__/plugins/css.spec.ts:374-383` + `packages/vite/src/node/__tests__/plugins/css.spec.ts:385-396` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr/__tests__/hmr.spec.ts:255-276` + `playground/hmr/__tests__/hmr.spec.ts:284-304` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a5.js:25-36` + `playground/multiple-entrypoints/entrypoints/a7.js:23-34` | ✅ pass | passing |
| vite | clearly_in | `packages/create-vite/template-preact-ts/src/app.tsx:7-104` + `packages/create-vite/template-react-ts/src/App.tsx:7-120` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:514-572` + `playground/hmr/__tests__/hmr.spec.ts:660-731` | ✅ pass | passing |
| vite | clearly_in | `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:232-269` + `playground/assets/__tests__/url-base/assets-url-base.spec.ts:234-271` | ✅ pass | passing |
| vite | clearly_in | `playground/resolve/__tests__/resolve.spec.ts:17-44` + `playground/resolve/__tests__/resolve.spec.ts:237-263` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/es/worker-es.spec.ts:99-137` + `playground/worker/__tests__/iife/worker-iife.spec.ts:84-115` | ✅ pass | passing |
| vite | clearly_in | `playground/ssr-pug/server.js:18-45` + `playground/ssr/server.js:12-40` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a12.js:18-28` + `playground/multiple-entrypoints/entrypoints/a8.js:22-32` | 🔴 fail | on record as failing |
| vite | clearly_in | `playground/worker/__tests__/sourcemap-hidden/worker-sourcemap-hidden.spec.ts:9-30` + `playground/worker/__tests__/sourcemap/worker-sourcemap.spec.ts:9-30` | ✅ pass | passing |
| vite | clearly_in | `playground/hmr-ssr/__tests__/hmr-ssr.spec.ts:239-256` + `playground/hmr/__tests__/hmr.spec.ts:285-303` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/iife/worker-iife.spec.ts:82-159` + `playground/worker/__tests__/relative-base/worker-relative-base.spec.ts:60-140` | ✅ pass | passing |
| vite | clearly_in | `playground/multiple-entrypoints/entrypoints/a15.js:1-19` + `playground/multiple-entrypoints/entrypoints/a16.js:1-18` | ✅ pass | passing |
| vite | clearly_in | `playground/css/__tests__/sass-tests.ts:28-44` + `playground/css/__tests__/tests.ts:115-129` | ✅ pass | passing |
| vite | clearly_in | `packages/vite/src/node/server/hmr.ts:1065-1077` + `packages/vite/src/node/server/hmr.ts:1078-1090` | ✅ pass | passing |
| vite | clearly_in | `playground/worker/__tests__/es/worker-es.spec.ts:34-68` + `playground/worker/__tests__/iife/worker-iife.spec.ts:33-65` | ✅ pass | passing |
| vite | clearly_out | `packages/vite/src/types/chokidar.d.ts:77-92` + `packages/vite/src/types/ws.d.ts:262-271` | 🔴 fail | on record as failing |
| zlib | clearly_in | `gzwrite.c:543-550` + `gzwrite.c:666-673` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/iowin32.c:161-182` + `contrib/minizip/iowin32.c:91-112` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:3602-3653` + `crc32.h:7580-7631` | ✅ pass | passing |
| zlib | clearly_in | `contrib/iostream3/zfstream.h:234-313` + `contrib/iostream3/zfstream.h:323-402` | ✅ pass | passing |
| zlib | clearly_in | `infback.c:281-288` + `inflate.c:776-787` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/infback9.c:334-342` + `infback.c:294-302` | 🔴 fail | on record as failing |
| zlib | clearly_in | `contrib/minizip/iowin32.c:163-184` + `contrib/minizip/iowin32.c:93-114` | ✅ pass | passing |
| zlib | clearly_in | `examples/zpipe.c:40-48` + `examples/zpipe.c:96-104` | 🔴 fail | on record as failing |
| zlib | clearly_in | `contrib/minizip/iowin32.c:160-185` + `contrib/minizip/iowin32.c:90-115` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:3394-3445` + `crc32.h:9118-9169` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/ioapi.c:148-169` + `contrib/minizip/ioapi.c:171-194` | ✅ pass | passing |
| zlib | clearly_in | `contrib/dotzlib/DotZLib/Deflater.cs:18-105` + `contrib/dotzlib/DotZLib/Inflater.cs:18-104` | ✅ pass | passing |
| zlib | clearly_in | `infback.c:262-274` + `inflate.c:757-769` | ✅ pass | passing |
| zlib | clearly_in | `deflate.c:1523-1538` + `deflate.c:1595-1609` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/infback9.c:86-94` + `contrib/infback9/infback9.c:97-105` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/unzip.c:762-775` + `contrib/minizip/unzip.c:790-803` | ✅ pass | passing |
| zlib | clearly_in | `gzread.c:426-433` + `gzread.c:552-559` | 🔴 fail | on record as failing |
| zlib | clearly_in | `gzwrite.c:495-508` + `gzwrite.c:616-630` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/zip.c:633-643` + `contrib/minizip/zip.c:665-675` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:1804-1855` + `crc32.h:5990-6041` | ✅ pass | passing |
| zlib | clearly_in | `gzwrite.c:498-508` + `gzwrite.c:620-631` | ✅ pass | passing |
| zlib | clearly_in | `crc32.c:690-766` + `crc32.c:811-888` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:1908-1959` + `crc32.h:6094-6145` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/zip.c:606-623` + `contrib/minizip/zip.c:625-650` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/infback9.c:86-93` + `contrib/infback9/infback9.c:97-104` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/unzip.c:1792-1798` + `contrib/minizip/unzip.c:1872-1878` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:266-317` + `crc32.h:2914-2965` | ✅ pass | passing |
| zlib | clearly_in | `inflate.c:938-948` + `inflate.c:990-1000` | ✅ pass | passing |
| zlib | clearly_in | `contrib/blast/blast.c:190-241` + `contrib/puff/puff.c:340-379` | ✅ pass | passing |
| zlib | clearly_in | `infback.c:315-332` + `inflate.c:812-829` | ✅ pass | passing |
| zlib | clearly_in | `zutil.c:105-118` + `zutil.c:89-102` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/ioapi.c:100-116` + `contrib/minizip/ioapi.c:82-98` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/inftree9.c:228-236` + `contrib/infback9/inftree9.c:302-310` | 🔴 fail | on record as failing |
| zlib | clearly_in | `contrib/infback9/infback9.c:232-239` + `infback.c:198-206` | ✅ pass | passing |
| zlib | clearly_in | `deflate.c:323-328` + `deflate.c:600-607` | ✅ pass | passing |
| zlib | clearly_in | `infback.c:402-419` + `inflate.c:900-917` | ✅ pass | passing |
| zlib | clearly_in | `contrib/dotzlib/DotZLib/GZipStream.cs:154-175` + `contrib/dotzlib/DotZLib/GZipStream.cs:199-218` | ✅ pass | passing |
| zlib | clearly_in | `gzread.c:426-442` + `gzread.c:517-533` | 🔴 fail | on record as failing |
| zlib | clearly_in | `contrib/dotzlib/DotZLib/UnitTests.cs:101-110` + `contrib/dotzlib/DotZLib/UnitTests.cs:135-144` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/zip.c:626-638` + `contrib/minizip/zip.c:654-666` | ✅ pass | passing |
| zlib | clearly_in | `contrib/blast/blast.c:131-143` + `contrib/puff/puff.c:266-278` | ✅ pass | passing |
| zlib | clearly_in | `test/example.c:175-185` + `test/example.c:341-351` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/unzip.c:628-653` + `contrib/minizip/zip.c:1104-1127` | ✅ pass | passing |
| zlib | clearly_in | `contrib/dotzlib/DotZLib/CodecBase.cs:114-134` + `contrib/dotzlib/DotZLib/GZipStream.cs:92-128` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:1960-2011` + `crc32.h:4400-4451` | ✅ pass | passing |
| zlib | clearly_in | `gzwrite.c:440-469` + `gzwrite.c:557-586` | ✅ pass | passing |
| zlib | clearly_in | `infback.c:228-260` + `inflate.c:717-755` | ✅ pass | passing |
| zlib | clearly_in | `inffast.c:101-112` + `inffast.c:132-143` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/unzip.c:493-513` + `contrib/minizip/unzip.c:552-572` | ✅ pass | passing |
| zlib | clearly_in | `gzread.c:705-713` + `gzwrite.c:703-713` | ✅ pass | passing |
| zlib | clearly_in | `gzwrite.c:271-281` + `gzwrite.c:298-308` | 🔴 fail | on record as failing |
| zlib | clearly_in | `test/example.c:400-406` + `test/example.c:436-442` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/infback9.c:596-603` + `infback.c:572-579` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:3654-3705` + `crc32.h:7632-7683` | ✅ pass | passing |
| zlib | clearly_in | `deflate.c:1902-1916` + `deflate.c:2002-2016` | ✅ pass | passing |
| zlib | clearly_in | `contrib/minizip/unzip.h:112-131` + `contrib/minizip/unzip.h:133-152` | ✅ pass | passing |
| zlib | clearly_in | `contrib/dotzlib/DotZLib/UnitTests.cs:102-111` + `contrib/dotzlib/DotZLib/UnitTests.cs:136-145` | ✅ pass | passing |
| zlib | clearly_in | `contrib/infback9/infback9.c:518-537` + `infback.c:486-510` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:1856-1907` + `crc32.h:6042-6093` | ✅ pass | passing |
| zlib | clearly_in | `crc32.h:1324-1375` + `crc32.h:422-473` | ✅ pass | passing |
| zlib | clearly_in | `contrib/testzlib/testzlib.c:215-222` + `contrib/testzlib/testzlib.c:256-263` | ✅ pass | passing |
| zlib | clearly_in | `gzwrite.c:429-437` + `gzwrite.c:542-550` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/core/api.ts:162-177` + `packages/zod/src/v4/core/api.ts:180-195` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/codec.test.ts:85-183` + `packages/zod/src/v4/mini/tests/codec.test.ts:88-186` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:480-489` + `packages/zod/src/v4/mini/tests/index.test.ts:556-565` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/partials.test.ts:165-233` + `packages/zod/src/v4/classic/tests/partial.test.ts:79-147` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/map.test.ts:71-79` + `packages/zod/src/v4/classic/tests/map.test.ts:181-189` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:84-204` + `packages/zod/src/v4/mini/tests/index.test.ts:84-204` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:275-289` + `packages/zod/src/v4/mini/tests/index.test.ts:331-345` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/record.test.ts:149-162` + `packages/zod/src/v4/classic/tests/record.test.ts:420-433` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/stringbool.test.ts:19-24` + `packages/zod/src/v4/mini/tests/index.test.ts:900-905` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:832-847` + `packages/zod/src/v4/mini/tests/index.test.ts:935-950` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/async-parsing.test.ts:53-110` + `packages/zod/src/v4/classic/tests/async-parsing.test.ts:52-109` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:155-222` + `packages/zod/src/v4/mini/tests/index.test.ts:155-222` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/unions.test.ts:6-28` + `packages/zod/src/v4/classic/tests/union.test.ts:5-27` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/in-out.test.ts:147-150` + `packages/zod/src/v4/classic/tests/in-out.test.ts:6-9` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/recursive-types.test.ts:145-152` + `packages/zod/src/v4/mini/tests/recursive-types.test.ts:152-159` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/core/checks.ts:1043-1051` + `packages/zod/src/v4/core/checks.ts:1087-1095` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:44-166` + `packages/zod/src/v4/mini/tests/index.test.ts:44-166` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/from-json-schema.ts:567-572` + `packages/zod/src/v4/classic/from-json-schema.ts:772-777` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/detached-methods.test.ts:63-68` + `packages/zod/src/v4/classic/tests/detached-methods.test.ts:75-80` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:461-560` + `packages/zod/src/v4/mini/tests/index.test.ts:537-636` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/async-parsing.test.ts:68-125` + `packages/zod/src/v4/classic/tests/async-parsing.test.ts:67-124` | ✅ pass | passing |
| zod | clearly_in | `vitest.compile.config.ts:13-22` + `vitest.config.ts:8-17` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:134-215` + `packages/zod/src/v4/mini/tests/index.test.ts:134-215` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:727-736` + `packages/zod/src/v4/mini/tests/index.test.ts:803-812` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/branded.test.ts:18-23` + `packages/zod/src/v4/classic/tests/brand.test.ts:16-21` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:44-67` + `packages/zod/src/v4/mini/tests/index.test.ts:44-67` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/index.test.ts:174-191` + `packages/zod/src/v4/mini/tests/index.test.ts:174-191` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v3/tests/object.test.ts:186-199` + `packages/zod/src/v4/classic/tests/object.test.ts:195-208` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/record.test.ts:179-193` + `packages/zod/src/v4/classic/tests/record.test.ts:266-280` | ✅ pass | passing |
| zod | clearly_in | `packages/zod/src/v4/classic/tests/codec.test.ts:24-39` + `packages/zod/src/v4/mini/tests/codec.test.ts:27-42` | ✅ pass | passing |
| zod | clearly_in | `packages/docs/app/og.png/route.tsx:15-106` + `packages/docs/pages/api/_og.tsx:32-125` | ✅ pass | passing |
| zod | clearly_out | `packages/docs/next.config.mjs:106-113` + `packages/docs/next.config.mjs:165-172` | ✅ pass | passing |
| zod | clearly_out | `packages/docs/next.config.mjs:107-116` + `packages/docs/next.config.mjs:166-173` | ✅ pass | passing |
| zod | clearly_out | `packages/docs/next.config.mjs:108-117` + `packages/docs/next.config.mjs:167-174` | ✅ pass | passing |
| zod | clearly_out | `packages/docs/next.config.mjs:143-150` + `packages/docs/next.config.mjs:163-170` | ✅ pass | passing |
| zod | clearly_out | `packages/docs/next.config.mjs:144-151` + `packages/docs/next.config.mjs:164-171` | ✅ pass | passing |

## Judged pairs `deslop@fb68ce65f2d5` gets wrong

- **FALSE NEGATIVE** BenchmarkDotNet — `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncEnumerableCoreEmitter.cs:554-561` + `src/BenchmarkDotNet/Toolchains/InProcess/Emit/Implementation/Emitters/AsyncCoreEmitter.cs:147-156` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `ilBuilder.EmitLdloc(isCompleteAwaiterLocal);` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** BenchmarkDotNet — `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1027-1037` + `tests/BenchmarkDotNet.Analyzers.Tests/AnalyzerTests/Attributes/ParamsAttributeAnalyzerTests.cs:1329-1339` (Type I; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `string[] nameColonUsages =` is repeated unchanged apart from comments or formatting.
- **FALSE POSITIVE** composer — `src/Composer/Repository/Vcs/GitHubDriver.php:267-281` + `src/Composer/Repository/Vcs/GitHubDriver.php:282-296` — on record as failing since 2026-11-02T01:31:12.995Z
  - These are different adjacent cases of one funding-provider switch, not two copies of a region.
- **FALSE POSITIVE** composer — `src/Composer/Command/RequireCommand.php:92-94` + `src/Composer/Command/UpdateCommand.php:65-67` — on record as failing since 2026-11-02T01:31:12.995Z
  - Two three-line runs of InputOption definitions listing entirely different options (no-security-blocking/no-blocking/update-no-dev vs lock/no-install/no-audit); only the constructor shape is shared.
- **FALSE POSITIVE** composer — `tests/Composer/Test/Downloader/HgDownloaderTest.php:67-69` + `tests/Composer/Test/Installer/SuggestedPackagesReporterTest.php:226-228` — on record as failing since 2026-11-02T01:31:12.995Z
  - Two PHPUnit mock chains with nothing in common: different test files, different mocked methods (getSourceUrls vs getNames), different return values. Only the expects/method/will skeleton matches.
- **FALSE POSITIVE** composer — `tests/Composer/Test/DependencyResolver/SolverTest.php:741-742` + `tests/Composer/Test/DependencyResolver/SolverTest.php:833-838` — on record as failing since 2026-11-02T01:31:12.995Z
  - setRequires test data for different packages with different links, constraints and formatting; only the call shape is shared.
- **FALSE NEGATIVE** console — `Tests/Completion/Output/FishCompletionOutputTest.php:1-33` + `Tests/Completion/Output/ZshCompletionOutputTest.php:1-33` (Type II; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `<?php` is repeated with the same operations in the same order and only consistent name or literal substitutions.
- **FALSE NEGATIVE** console — `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:116-125` + `Tests/DependencyInjection/RemoveEmptyCommandArgumentLocatorsPassTest.php:59-68` (Type I; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `$container->setDefinition('command_locator', $commandLocator);` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** date-fns — `pkgs/core/src/locale/kk/_lib/formatRelative/index.ts:52-64` + `pkgs/core/src/locale/mk/_lib/formatRelative/index.ts:82-94` (Type I; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `nextWeek: <DateType extends Date>(` is repeated unchanged apart from comments or formatting.
- **FALSE POSITIVE** django — `tests/invalid_models_tests/test_relative_fields.py:313-318` + `tests/migrations/migrations_test_apps/lookuperror_a/models.py:8-18` — on record as failing since 2026-11-02T01:31:12.995Z
  - Model definitions with unrelated fields: GroupMember's person/group FKs versus A3's lookuperror FKs; only the class-with-ForeignKeys skeleton is shared.
- **FALSE NEGATIVE** express — `test/app.router.js:119-129` + `test/app.router.js:95-105` (Type II; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `it('should not decode spaces', function (done) {` is repeated with the same operations in the same order and only consistent name or literal substitutions.
- **FALSE NEGATIVE** express — `test/app.use.js:144-149` + `test/app.use.js:193-198` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `request(app)` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** express — `test/app.param.js:99-106` + `test/app.param.js:275-282` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `app.get('/:user/bob', function(req, res, next) {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** express — `test/express.text.js:372-397` + `test/express.json.js:515-540` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `app.use(function (req, res, next) {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** express — `test/res.jsonp.js:164-171` + `test/res.jsonp.js:335-342` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `app.use(function (req, res) {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** FsCheck — `examples/FsCheck.Examples.MsTest/ClassesToTest/EnumerableExtensions.cs:10-23` + `examples/FsCheck.Examples.CSharp/Program.cs:13-26` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `public static IEnumerable<int> Insert(this IEnumerable<int> cs, int x)` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** FSharpPlus — `src/FSharpPlus/Data/NonEmptySet.fs:66-83` + `src/FSharpPlus/Data/NonEmptyList.fs:57-76` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `let ofArray (array : _ array) =` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** FSharpPlus — `tests/FSharpPlusFable.Tests/FSharpTests/General/Util.fs:46-58` + `tests/FSharpPlus.Tests/General.fs:51-63` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `type WrappedListB<'s> = WrappedListB of 's list with` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** gin — `tree.go:877-887` + `tree.go:861-871` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `idxc := rb[0]` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** gin — `tree.go:848-871` + `tree.go:764-799` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `var rv rune` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** http — `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:167-174` + `pkgs/http_client_conformance_tests/lib/src/abort_tests.dart:225-233` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `final abortTrigger = Completer<void>();` is repeated unchanged apart from comments or formatting.
- **FALSE POSITIVE** hugo — `hugolib/site_output_test.go:657-689` + `hugolib/page__meta_test.go:24-47` — on record as failing since 2026-11-02T01:31:12.995Z
  - Two integration tests sharing only the t.Parallel/files/Test/AssertFileContent convention; content, layouts and assertions differ throughout.
- **FALSE POSITIVE** hugo — `tpl/internal/go_templates/htmltemplate/exec_test.go:507-514` + `tpl/internal/go_templates/texttemplate/exec_test.go:615-622` — on record as failing since 2026-11-02T01:31:12.995Z
  - Exec-test rows for different features from the two vendored files: index tests paired against range tests — the paired slices do not correspond to each other.
- **FALSE POSITIVE** hugo — `commands/server.go:550-551` + `commands/server.go:545-546` — on record as failing since 2026-11-02T01:31:12.995Z
  - These flag declarations configure unrelated server options.
- **FALSE POSITIVE** hugo — `tpl/tplimpl/templatestore_integration_test.go:1076-1114` + `tpl/tplimpl/templatestore_integration_test.go:908-925` — on record as failing since 2026-11-02T01:31:12.995Z
  - Two integration tests sharing only the files/Test/AssertFileContent convention; goat-diagram fixtures versus layout-selection fixtures.
- **FALSE POSITIVE** hugo — `resources/image_test.go:304-306` + `hugolib/disableKinds_test.go:175-176` — on record as failing since 2026-11-02T01:31:12.995Z
  - Assert triples about unrelated subjects (image resize success vs disabled categories permalink).
- **FALSE NEGATIVE** Newtonsoft.Json — `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:609-620` + `Src/Newtonsoft.Json.Tests/Serialization/ShouldSerializeTests.cs:588-599` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `private string nameField;` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** rayon — `rayon-core/src/broadcast/test.rs:163-168` + `rayon-core/src/broadcast/test.rs:201-206` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `let (panic_tx, panic_rx) = channel();` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** viper — `viper.go:539-553` + `viper.go:572-587` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `if pathIndex == len(path) {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** viper — `internal/encoding/dotenv/codec_test.go:24-55` + `internal/encoding/toml/codec_test.go:66-97` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `func TestCodec_Encode(t *testing.T) {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** vite — `playground/assets/__tests__/encoded-base/assets-encoded-base.spec.ts:178-205` + `playground/assets/__tests__/relative-base/assets-relative-base.spec.ts:191-218` (Type I; reported as loosely_similar) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `describe('svg fragments', () => {` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** vite — `playground/multiple-entrypoints/entrypoints/a11.js:19-30` + `playground/multiple-entrypoints/entrypoints/a10.js:20-31` — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `a12() +` is repeated with the same operations in the same order and only consistent name or literal substitutions.
- **FALSE NEGATIVE** vite — `playground/multiple-entrypoints/entrypoints/a12.js:18-28` + `playground/multiple-entrypoints/entrypoints/a8.js:22-32` — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `a13() +` is repeated with the same operations in the same order and only consistent name or literal substitutions.
- **FALSE POSITIVE** vite — `packages/vite/src/types/ws.d.ts:262-271` + `packages/vite/src/types/chokidar.d.ts:77-92` — on record as failing since 2026-11-08T06:47:26.057Z
  - WebSocket event overloads and filesystem-watcher event overloads have unrelated parameter lists and event payloads. Their shared shape is the event-listener API declaration syntax.
- **FALSE NEGATIVE** zlib — `contrib/infback9/infback9.c:334-342` + `infback.c:294-302` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `case TABLE:` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** zlib — `examples/zpipe.c:40-48` + `examples/zpipe.c:96-104` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `unsigned have;` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** zlib — `gzread.c:426-433` + `gzread.c:552-559` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `gz_statep state;` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** zlib — `contrib/infback9/inftree9.c:228-236` + `contrib/infback9/inftree9.c:302-310` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `incr = 1U << (len - 1);` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** zlib — `gzread.c:426-442` + `gzread.c:517-533` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `gz_statep state;` is repeated unchanged apart from comments or formatting.
- **FALSE NEGATIVE** zlib — `gzwrite.c:271-281` + `gzwrite.c:298-308` (Type I; reported as nearly_identical) — on record as failing since 2026-11-08T06:47:26.057Z
  - The code beginning with `gz_statep state;` is repeated unchanged apart from comments or formatting.
