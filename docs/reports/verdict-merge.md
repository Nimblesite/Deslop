# Verdicts that disagree

**293 pairs. Nothing was merged.**

Judges read: `/Users/christianfindlay/clone-judging-codex`, `/Users/christianfindlay/clone-judging-glm5.3`, plus this repository's existing registers. A pair enters a register only when every judge who ruled on it said the same thing, so every pair below was left out. Written by `scripts/corpus/merge-verdicts.mjs`; spec `docs/specs/corpus.md` [CORPUS-REGISTER-MERGE].

## Opposite conclusions — one judge CLEARLY IN, the other CLEARLY OUT

**1.** These cannot both be true of the same lines. Someone is wrong about the source.

### cobra — candidate 10

- `doc/rest_docs.go:138-170`
- `doc/yaml_docs.go:30-85`

**clone-judging-codex — clearly_out**

> One side is ReST tree-generation functions, the other is the yaml cmdOption/cmdDoc struct block — nothing shared but the file package.

**clone-judging-glm5.3 — clearly_in**

> GenReSTTreeCustom and GenYamlTreeCustom are the same 33-line tree walker with the format name systematically swapped

## A judge's ranges are not the candidate's

**2.** The judge filed a verdict on lines the candidate never showed them, so it is a ruling on something else.

| repository | candidate | judge | what happened |
| --- | --- | --- | --- |
| polly | 51 | clone-judging-glm5.3 | ranges are not the candidate's |
| guzzle | 19 | clone-judging-glm5.3 | ranges are not the candidate's |

## Contradicts a verdict the register already holds

**1.** An earlier pass and this one read the same lines differently.

| repository | candidate | ranges | verdicts |
| --- | --- | --- | --- |
| polly | 2 | `src/Polly.SharedSpecs/Retry/RetryAsyncSpecs.cs:378-382`<br>`src/Polly.SharedSpecs/Retry/RetryTResultSpecsAsync.cs:438-442` | register: **clearly_in**<br>clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **not_clear** |

## The same pair judged twice, differently, in one pass

**4.** The draw showed one pair of regions under two numbers, and the judges answered them differently.

| repository | candidates | ranges | verdicts |
| --- | --- | --- | --- |
| bloc | 14 then 98 | `packages/replay_bloc/test/replay_bloc_test.dart:271-306`<br>`packages/replay_bloc/test/replay_bloc_test.dart:529-564` | **clearly_in** then **not_clear** |
| bloc | 51 then 173 | `examples/flutter_login/lib/login/models/password.dart:1-14`<br>`examples/flutter_login/lib/login/models/username.dart:1-14` | **clearly_in** then **not_clear** |
| click | 49 then 179 | `src/click/_compat.py:303-316`<br>`src/click/_compat.py:287-300` | **clearly_in** then **not_clear** |
| guzzle | 22 then 99 | `tests/Handler/CurlFactoryTest.php:4040-4081`<br>`tests/Handler/CurlFactoryTest.php:3943-3984` | **clearly_in** then **not_clear** |

## One judge committed, the other would not

**285.** A firm verdict against NOT CLEAR. Weaker than an opposite conclusion, but still not agreement.

| repository | candidate | ranges | verdicts |
| --- | --- | --- | --- |
| fsharp.data | 10 | `tests/FSharp.Data.Core.Tests/HtmlCharRefs.fs:41-47`<br>`tests/FSharp.Data.Core.Tests/TextConversions.fs:266-271` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 19 | `src/FSharp.Data.Html.Core/HtmlParser.fs:572-581`<br>`src/FSharp.Data.Html.Core/HtmlParser.fs:583-592` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 22 | `src/FSharp.Data.Html.Core/HtmlOperations.fs:490-525`<br>`src/FSharp.Data.Html.Core/HtmlDocumentOperations.fs:129-163` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 23 | `tests/FSharp.Data.Core.Tests/JsonSchema.fs:84-88`<br>`tests/FSharp.Data.Core.Tests/JsonSchema.fs:78-82` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 25 | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:212-233`<br>`src/FSharp.Data.DesignTime/Json/JsonProvider.fs:172-193` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 34 | `src/FSharp.Data.Json.Core/JsonExtensions.fs:364-386`<br>`src/FSharp.Data.Json.Core/JsonExtensions.fs:335-355` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 39 | `tests/FSharp.Data.Core.Tests/JsonRuntime.fs:179-182`<br>`tests/FSharp.Data.Core.Tests/JsonRuntime.fs:169-172` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 40 | `tests/FSharp.Data.Core.Tests/StructuralInference.fs:110-114`<br>`tests/FSharp.Data.Core.Tests/StructuralInference.fs:118-122` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 43 | `tests/FSharp.Data.Core.Tests/XmlInference.fs:311-327`<br>`tests/FSharp.Data.Core.Tests/XmlInference.fs:49-65` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 46 | `src/FSharp.Data.Html.Core/HtmlParser.fs:573-583`<br>`src/FSharp.Data.Html.Core/HtmlParser.fs:584-594` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 54 | `src/FSharp.Data.Html.Core/HtmlOperations.fs:360-364`<br>`src/FSharp.Data.Html.Core/HtmlOperations.fs:354-358` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 57 | `tests/FSharp.Data.Benchmarks/JsonBenchmarks.fs:15-46`<br>`tests/FSharp.Data.Benchmarks/HtmlBenchmarks.fs:15-47` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 64 | `tests/FSharp.Data.DesignTime.Tests/TypeProviderInstantiation.fs:301-307`<br>`tests/FSharp.Data.DesignTime.Tests/TypeProviderInstantiation.fs:320-326` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 69 | `src/FSharp.Data.Html.Core/HtmlParser.fs:524-540`<br>`src/FSharp.Data.Html.Core/HtmlParser.fs:431-447` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 75 | `src/FSharp.Data.Xml.Core/XmlExtensions.fs:39-39`<br>`src/FSharp.Data.Xml.Core/XmlExtensions.fs:19-19` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 80 | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:212-270`<br>`src/FSharp.Data.DesignTime/Json/JsonProvider.fs:172-228` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 83 | `tests/FSharp.Data.Core.Tests/NameUtils.fs:202-223`<br>`tests/FSharp.Data.Core.Tests/NameUtils.fs:174-195` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 100 | `tests/FSharp.Data.Benchmarks/CsvBenchmarks.fs:25-31`<br>`tests/FSharp.Data.Benchmarks/CsvBenchmarks.fs:41-47` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 103 | `tests/FSharp.Data.Core.Tests/XmlInference.fs:41-47`<br>`tests/FSharp.Data.Core.Tests/XmlInference.fs:59-65` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 110 | `tests/FSharp.Data.Core.Tests/JsonValue.fs:742-749`<br>`tests/FSharp.Data.Core.Tests/JsonValue.fs:734-741` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 113 | `tests/FSharp.Data.Core.Tests/IOTests.fs:15-19`<br>`tests/FSharp.Data.Core.Tests/TextConversions.fs:43-45` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 114 | `src/FSharp.Data.DesignTime/Xml/XmlGenerator.fs:30-32`<br>`src/FSharp.Data.Http/Http.fs:672-675` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| fsharp.data | 119 | `tests/FSharp.Data.Core.Tests/WorldBankRuntime.fs:23-173`<br>`src/FSharp.Data.Http/Http.fs:57-73` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| fsharp.data | 125 | `src/FSharp.Data.Html.Core/HtmlOperations.fs:769-801`<br>`src/FSharp.Data.Html.Core/HtmlOperations.fs:620-652` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 126 | `tests/FSharp.Data.Core.Tests/CsvParserProperties.fs:98-112`<br>`tests/FSharp.Data.Core.Tests/CsvParserProperties.fs:74-88` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 145 | `tests/FSharp.Data.Core.Tests/JsonSchema.fs:201-206`<br>`tests/FSharp.Data.Core.Tests/JsonSchema.fs:240-245` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 147 | `tests/FSharp.Data.Core.Tests/HtmlCharRefs.fs:41-52`<br>`tests/FSharp.Data.Core.Tests/NameUtils.fs:27-40` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 150 | `src/FSharp.Data.Html.Core/HtmlOperations.fs:725-807`<br>`src/FSharp.Data.Html.Core/HtmlOperations.fs:576-663` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| fsharp.data | 151 | `tests/FSharp.Data.Core.Tests/NameUtilsProperties.fs:105-120`<br>`tests/FSharp.Data.Core.Tests/NameUtilsProperties.fs:45-56` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 153 | `tests/FSharp.Data.Core.Tests/CsvParserProperties.fs:74-88`<br>`tests/FSharp.Data.Core.Tests/CsvParserProperties.fs:98-112` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 155 | `src/FSharp.Data.Json.Core/JsonExtensions.fs:95-103`<br>`src/FSharp.Data.Json.Core/JsonExtensions.fs:144-152` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 156 | `src/FSharp.Data.Html.Core/HtmlParser.fs:475-501`<br>`src/FSharp.Data.Html.Core/HtmlParser.fs:327-353` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 158 | `src/FSharp.Data.Html.Core/HtmlDocumentOperations.fs:154-224`<br>`src/FSharp.Data.Html.Core/HtmlDocumentOperations.fs:225-291` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 166 | `tests/FSharp.Data.Core.Tests/JsonRuntime.fs:97-106`<br>`tests/FSharp.Data.Core.Tests/JsonRuntime.fs:108-117` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 174 | `tests/FSharp.Data.Core.Tests.CSharp/CsvExtensionsTests.cs:8-64`<br>`tests/FSharp.Data.Core.Tests.CSharp/JsonExtensionsTests.cs:65-120` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 180 | `tests/FSharp.Data.Tests/CsvProvider.fs:38-43`<br>`tests/FSharp.Data.Tests/CsvProvider.fs:45-50` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 184 | `src/FSharp.Data.Csv.Core/CsvFile.fs:137-141`<br>`src/FSharp.Data.Csv.Core/CsvFile.fs:107-111` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 185 | `tests/FSharp.Data.Core.Tests/StructuralInference.fs:320-322`<br>`tests/FSharp.Data.Core.Tests/HtmlOperations.fs:28-29` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 191 | `src/FSharp.Data.DesignTime/CommonProviderImplementation/Helpers.fs:693-709`<br>`src/FSharp.Data.DesignTime/CommonProviderImplementation/Helpers.fs:636-650` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| fsharp.data | 193 | `tests/FSharp.Data.DesignTime.Tests/TypeProviderInstantiation.fs:35-52`<br>`tests/FSharp.Data.DesignTime.Tests/TypeProviderInstantiation.fs:54-71` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 196 | `src/FSharp.Data.DesignTime/Xml/XmlProvider.fs:212-270`<br>`src/FSharp.Data.DesignTime/Json/JsonProvider.fs:172-228` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 197 | `tests/FSharp.Data.Benchmarks/CsvBenchmarks.fs:6-55`<br>`tests/FSharp.Data.Benchmarks/JsonBenchmarks.fs:6-56` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 199 | `src/FSharp.Data.Html.Core/HtmlOperations.fs:354-358`<br>`src/FSharp.Data.Html.Core/HtmlOperations.fs:360-364` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| fsharp.data | 200 | `tests/FSharp.Data.Core.Tests/TextConversions.fs:23-33`<br>`tests/FSharp.Data.Core.Tests/TextConversions.fs:35-45` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 4 | `src/Polly.Shared/PolicyAsync.TResult.cs:282-313`<br>`src/Polly.Shared/Policy.cs:138-161` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 19 | `src/Polly.SharedSpecs/Fallback/FallbackTResultSpecs.cs:559-568`<br>`src/Polly.SharedSpecs/Fallback/FallbackTResultSpecs.cs:582-591` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 24 | `src/Polly.Shared/Caching/SerializingCacheProvider.cs:59-100`<br>`src/Polly.Shared/Caching/SerializingCacheProvider.cs:10-51` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 28 | `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:401-407`<br>`src/Polly.Shared/Retry/RetrySyntaxAsync.cs:529-535` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 31 | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:641-799`<br>`src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:1108-1265` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 32 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:348-368`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:317-337` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 33 | `src/Polly.SharedSpecs/Fallback/FallbackAsyncSpecs.cs:405-424`<br>`src/Polly.SharedSpecs/Fallback/FallbackAsyncSpecs.cs:427-446` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 35 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:813-815`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:781-783` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 36 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:967-981`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:851-866` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 41 | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerSpecs.cs:24-26`<br>`src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:23-25` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 66 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:35-37`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:34-36` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 67 | `src/Polly.Shared/PolicyAsync.cs:109-112`<br>`src/Polly.Shared/PolicyAsync.cs:675-678` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 69 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:348-364`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerSpecs.cs:317-333` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 78 | `src/Polly.SharedSpecs/Wrap/PolicyWrapSpecs.cs:10-432`<br>`src/Polly.SharedSpecs/Wrap/PolicyWrapSpecsAsync.cs:1-435` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 95 | `src/Polly.Shared/ISyncPolicyTResult.cs:103-130`<br>`src/Polly.Shared/IAsyncPolicy.cs:200-225` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 107 | `src/Polly.SharedSpecs/Timeout/TimeoutSpecs.cs:17-83`<br>`src/Polly.SharedSpecs/Timeout/TimeoutAsyncSpecs.cs:18-84` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 108 | `src/Polly.Shared/ISyncPolicy.cs:17-69`<br>`src/Polly.Shared/IAsyncPolicy.cs:18-61` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 119 | `src/Polly.Shared/Wrap/PolicyWrapSyntaxAsync.cs:44-77`<br>`src/Polly.Shared/Wrap/PolicyWrapSyntax.cs:44-77` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 121 | `src/Polly.SharedSpecs/Retry/RetryForeverAsyncSpecs.cs:41-85`<br>`src/Polly.SharedSpecs/Retry/RetryAsyncSpecs.cs:99-143` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 122 | `src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2628-2666`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:1254-1283` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 136 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultSpecs.cs:193-196`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerTResultAsyncSpecs.cs:194-197` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 152 | `src/Polly.SharedSpecs/Fallback/FallbackSpecs.cs:432-432`<br>`src/Polly.SharedSpecs/Timeout/TimeoutTResultSpecs.cs:250-251` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| polly | 160 | `src/Polly.Shared/NoOp/NoOpPolicyAsync.cs:9-27`<br>`src/Polly.Shared/Timeout/TimeoutPolicyAsync.cs:7-26` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 162 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:203-218`<br>`src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:320-328` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 165 | `src/Polly.SharedSpecs/Retry/RetryAsyncSpecs.cs:309-313`<br>`src/Polly.SharedSpecs/Retry/RetrySpecs.cs:330-334` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 172 | `src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:460-465`<br>`src/Polly.Shared/Retry/RetryTResultSyntax.cs:251-256` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 173 | `src/Polly.Shared/Retry/RetryTResultSyntax.cs:534-540`<br>`src/Polly.Shared/Retry/RetryTResultSyntaxAsync.cs:843-849` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 179 | `src/Polly.Shared/ISyncPolicy.cs:69-115`<br>`src/Polly.Shared/ISyncPolicyTResult.cs:27-69` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| polly | 188 | `src/Polly.Shared/CircuitBreaker/AdvancedCircuitBreakerTResultSyntax.cs:198-224`<br>`src/Polly.Shared/CircuitBreaker/CircuitBreakerTResultSyntaxAsync.cs:170-199` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 193 | `src/Polly.SharedSpecs/Registry/IReadOnlyPolicyRegistrySpecs.cs:205-221`<br>`src/Polly.SharedSpecs/Registry/PolicyRegistrySpecs.cs:351-367` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| polly | 197 | `src/Polly.SharedSpecs/CircuitBreaker/CircuitBreakerAsyncSpecs.cs:1343-1521`<br>`src/Polly.SharedSpecs/CircuitBreaker/AdvancedCircuitBreakerAsyncSpecs.cs:2722-2900` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 7 | `test/specs/adapter.spec.js:41-49`<br>`test/specs/adapter.spec.js:15-23` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 27 | `test/specs/progress.spec.js:15-22`<br>`test/specs/progress.spec.js:64-71` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 44 | `bin/contributors.js:182-186`<br>`bin/contributors.js:222-226` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 50 | `test/specs/requests.spec.js:311-336`<br>`test/specs/requests.spec.js:339-364` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 51 | `examples/server.js:69-76`<br>`sandbox/server.js:7-15` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 69 | `test/unit/adapters/http.js:361-365`<br>`test/unit/adapters/http.js:551-555` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 70 | `bin/contributors.js:182-186`<br>`bin/contributors.js:222-226` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 80 | `test/specs/interceptors.spec.js:323-337`<br>`test/specs/interceptors.spec.js:293-307` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 86 | `test/module/ts-require/index.ts:13-21`<br>`test/module/ts/index.ts:12-20` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 99 | `test/unit/adapters/http.js:65-104`<br>`test/helpers/server.js:20-51` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 104 | `test/specs/adapter.spec.js:75-83`<br>`test/specs/adapter.spec.js:41-49` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 120 | `test/specs/utils/endsWith.js:1-12`<br>`test/specs/utils/kindOfTest.js:1-12` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 126 | `test/unit/adapters/http.js:1351-1355`<br>`test/unit/adapters/http.js:1433-1437` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 131 | `test/specs/interceptors.spec.js:112-120`<br>`test/specs/interceptors.spec.js:95-103` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 136 | `test/module/typings/esm/index.ts:272-277`<br>`test/module/typings/cjs/index.ts:249-254` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 142 | `test/module/ts-require/index.js:4-11`<br>`test/module/ts-require-default/index.js:4-11` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 154 | `test/module/ts/index.ts:13-21`<br>`test/module/ts-require-default/index.ts:14-22` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 160 | `test/specs/options.spec.js:22-33`<br>`test/specs/options.spec.js:49-60` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 180 | `test/module/typings/esm/index.ts:275-279`<br>`test/module/typings/cjs/index.ts:252-256` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| axios | 183 | `test/module/ts-require/index.ts:17-24`<br>`test/module/ts/index.ts:16-23` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| axios | 184 | `test/specs/helpers/toFormData.spec.js:70-77`<br>`test/specs/helpers/toFormData.spec.js:34-41` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 2 | `packages/bloc/test/bloc_observer_test.dart:79-86`<br>`packages/bloc/test/bloc_observer_test.dart:133-140` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 7 | `examples/flutter_shopping_cart/test/cart/bloc/cart_bloc_test.dart:86-102`<br>`examples/flutter_shopping_cart/test/cart/bloc/cart_bloc_test.dart:124-140` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 11 | `examples/flutter_weather/packages/weather_repository/test/weather_repository_test.dart:91-117`<br>`examples/flutter_weather/packages/weather_repository/test/weather_repository_test.dart:119-145` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 28 | `examples/flutter_firebase_login/lib/sign_up/cubit/sign_up_state.dart:50-57`<br>`examples/flutter_firebase_login/lib/sign_up/cubit/sign_up_state.dart:59-66` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 29 | `examples/bloc_concurrency_visualizer/lib/timeline/view/timeline_page.dart:274-279`<br>`packages/bloc_lint/lib/src/diagnostic.dart:34-49` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 34 | `packages/bloc/test/bloc_test.dart:353-364`<br>`packages/bloc/test/bloc_test.dart:378-389` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 39 | `examples/flutter_todos/test/edit_todo/view/edit_todo_page_test.dart:156-193`<br>`examples/flutter_todos/test/edit_todo/view/edit_todo_page_test.dart:195-234` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 40 | `examples/flutter_timer/test/timer/view/timer_page_test.dart:125-131`<br>`examples/flutter_timer/test/timer/view/timer_page_test.dart:117-123` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 43 | `packages/replay_bloc/test/replay_cubit_test.dart:396-419`<br>`packages/replay_bloc/test/replay_cubit_test.dart:151-174` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 46 | `examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:102-114`<br>`examples/flutter_weather/test/weather/cubit/weather_cubit_test.dart:224-236` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 64 | `packages/hydrated_bloc/lib/src/hydrated_bloc.dart:110-116`<br>`packages/hydrated_bloc/lib/src/hydrated_bloc.dart:64-70` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 68 | `examples/flutter_firebase_login/test/login/view/login_form_test.dart:108-149`<br>`examples/flutter_firebase_login/test/sign_up/view/sign_up_form_test.dart:110-154` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 85 | `packages/bloc_lint/test/src/linter_test.dart:49-61`<br>`packages/bloc_lint/test/src/lint_test_helper.dart:63-75` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 89 | `examples/flutter_firebase_login/lib/sign_up/cubit/sign_up_state.dart:39-78`<br>`examples/flutter_firebase_login/lib/login/cubit/login_state.dart:17-44` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 94 | `packages/bloc_lint/lib/src/rules/prefer_cubit.dart:1-51`<br>`packages/bloc_lint/lib/src/rules/prefer_bloc.dart:1-51` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 97 | `packages/flutter_bloc/test/bloc_listener_test.dart:470-479`<br>`packages/flutter_bloc/test/bloc_listener_test.dart:459-468` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 102 | `examples/flutter_weather/test/weather/widgets/weather_populated_test.dart:46-59`<br>`examples/flutter_weather/test/weather/widgets/weather_populated_test.dart:76-89` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 103 | `packages/bloc_tools/test/src/commands/lint/lint_command_test.dart:159-163`<br>`packages/bloc_tools/test/src/commands/language_server/language_server_command_test.dart:47-51` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 105 | `extensions/vscode/src/commands/new-cubit.command.ts:107-158`<br>`extensions/vscode/src/commands/new-bloc.command.ts:148-194` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 107 | `examples/flutter_firebase_login/packages/authentication_repository/test/authentication_repository_test.dart:215-264`<br>`examples/flutter_firebase_login/packages/authentication_repository/test/authentication_repository_test.dart:94-134` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 110 | `packages/bloc_lint/lib/src/env.dart:42-48`<br>`packages/bloc_lint/lib/src/env.dart:31-37` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 134 | `examples/flutter_todos/test/edit_todo/view/edit_todo_page_test.dart:156-193`<br>`examples/flutter_todos/test/edit_todo/view/edit_todo_page_test.dart:195-234` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 139 | `packages/replay_bloc/lib/src/replay_bloc.dart:115-127`<br>`packages/replay_bloc/lib/src/replay_bloc.dart:102-114` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 141 | `packages/bloc/test/blocs/complex/complex_state.dart:1-64`<br>`packages/bloc/test/blocs/complex/complex_event.dart:1-64` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 145 | `packages/bloc_lint/lib/src/env.dart:86-93`<br>`packages/bloc_lint/lib/src/env.dart:76-83` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 148 | `packages/hydrated_bloc/test/hydrated_bloc_test.dart:33-39`<br>`packages/hydrated_bloc/test/hydrated_cubit_test.dart:113-119` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 154 | `packages/bloc/test/bloc_test.dart:703-765`<br>`packages/bloc/test/bloc_test.dart:640-702` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 158 | `packages/flutter_bloc/test/bloc_provider_test.dart:530-592`<br>`packages/flutter_bloc/test/bloc_provider_test.dart:594-653` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| bloc | 184 | `examples/flutter_timer/test/timer/bloc/timer_state_test.dart:16-23`<br>`examples/flutter_todos/packages/todos_api/test/todos_api_test.dart:13-19` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 188 | `examples/flutter_firebase_login/lib/sign_up/view/sign_up_page.dart:6-26`<br>`examples/flutter_login/lib/login/view/login_page.dart:6-27` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 192 | `examples/flutter_complex_list/lib/complex_list/models/item.dart:4-11`<br>`examples/bloc_concurrency_visualizer/lib/timeline/view/timeline_page.dart:227-236` | clone-judging-codex: **clearly_out**<br>clone-judging-glm5.3: **not_clear** |
| bloc | 196 | `packages/replay_bloc/test/replay_cubit_test.dart:137-174`<br>`packages/replay_bloc/test/replay_cubit_test.dart:382-419` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| click | 7 | `tests/test_options.py:422-430`<br>`tests/test_options.py:734-742` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 12 | `tests/test_options.py:1155-1165`<br>`tests/test_options.py:1167-1177` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 23 | `tests/test_formatting.py:148-161`<br>`tests/test_formatting.py:236-249` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 28 | `tests/test_termui.py:970-975`<br>`tests/test_termui.py:1018-1023` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 30 | `src/click/_compat.py:326-330`<br>`src/click/_compat.py:333-337` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 32 | `tests/test_custom_classes.py:66-81`<br>`tests/test_custom_classes.py:84-99` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 34 | `tests/test_formatting.py:91-112`<br>`tests/test_formatting.py:56-77` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 35 | `tests/test_basic.py:581-589`<br>`tests/test_basic.py:554-562` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 40 | `tests/test_formatting.py:55-87`<br>`tests/test_formatting.py:90-122` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 45 | `tests/test_basic.py:275-287`<br>`tests/test_basic.py:201-213` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 46 | `tests/test_utils/test_echo.py:143-152`<br>`tests/test_utils/test_echo.py:115-126` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 52 | `tests/test_chain.py:68-85`<br>`tests/test_chain.py:107-124` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 57 | `tests/test_options.py:878-888`<br>`tests/test_arguments.py:271-281` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 65 | `tests/test_utils/test_echo.py:139-149`<br>`tests/test_utils/test_echo.py:111-123` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 74 | `tests/test_options.py:648-663`<br>`tests/test_options.py:631-645` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 77 | `tests/test_utils/test_echo.py:116-129`<br>`tests/test_utils/test_echo.py:144-155` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 81 | `tests/test_utils/test_confirm.py:13-22`<br>`tests/test_utils/test_confirm.py:32-41` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 93 | `tests/test_stream_lifecycle.py:408-417`<br>`tests/test_testing.py:360-370` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 97 | `tests/test_chain.py:107-124`<br>`tests/test_chain.py:68-85` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 116 | `tests/test_formatting.py:91-97`<br>`tests/test_formatting.py:56-62` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 123 | `tests/test_context.py:715-720`<br>`tests/test_context.py:722-727` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 129 | `tests/test_termui.py:977-985`<br>`tests/test_termui.py:997-1005` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 130 | `tests/test_options.py:1276-1286`<br>`tests/test_termui.py:1447-1456` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 133 | `tests/test_termui.py:970-975`<br>`tests/test_termui.py:1018-1023` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 145 | `src/click/testing.py:301-310`<br>`src/click/testing.py:282-292` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| click | 151 | `src/click/core.py:1093-1100`<br>`src/click/core.py:1233-1240` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 153 | `tests/test_utils/test_echo.py:117-130`<br>`tests/test_utils/test_echo.py:145-156` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 154 | `src/click/types.py:1210-1216`<br>`src/click/types.py:1201-1207` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 156 | `tests/test_utils/test_echo.py:141-150`<br>`tests/test_utils/test_echo.py:113-124` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| click | 176 | `src/click/testing.py:301-310`<br>`src/click/testing.py:294-299` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| click | 182 | `tests/test_formatting.py:90-122`<br>`tests/test_formatting.py:55-87` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 5 | `args_test.go:176-180`<br>`args_test.go:136-140` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 9 | `completions_test.go:2094-2100`<br>`completions_test.go:2061-2067` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 15 | `command_test.go:2098-2106`<br>`command_test.go:2118-2126` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 32 | `command_test.go:309-314`<br>`command_test.go:439-444` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 36 | `command.go:1643-1653`<br>`command.go:1657-1669` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 44 | `flag_groups_test.go:131-133`<br>`flag_groups_test.go:120-122` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 46 | `command_test.go:74-78`<br>`bash_completions_test.go:33-37` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 54 | `completions_test.go:1986-2000`<br>`completions_test.go:1970-1984` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 60 | `completions_test.go:1500-1530`<br>`completions_test.go:1400-1430` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 71 | `doc/rest_docs_test.go:98-112`<br>`doc/yaml_docs_test.go:86-100` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 91 | `flag_groups_test.go:130-135`<br>`flag_groups_test.go:125-130` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 97 | `command_test.go:445-451`<br>`command_test.go:121-127` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 126 | `completions_test.go:1189-1204`<br>`completions_test.go:1258-1273` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 132 | `doc/yaml_docs_test.go:86-100`<br>`doc/rest_docs_test.go:98-112` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 138 | `command_test.go:1996-2003`<br>`command_test.go:2016-2023` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 147 | `completions_test.go:462-480`<br>`completions_test.go:1118-1134` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 149 | `doc/yaml_docs.go:88-90`<br>`doc/md_docs.go:52-54` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 153 | `completions_test.go:2061-2076`<br>`completions_test.go:2094-2109` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 155 | `completions_test.go:1371-1381`<br>`completions_test.go:1474-1485` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 164 | `command.go:1712-1720`<br>`command.go:1740-1748` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| cobra | 167 | `completions_test.go:2726-2744`<br>`completions_test.go:1132-1148` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 168 | `command_test.go:942-950`<br>`command_test.go:2752-2760` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 176 | `command_test.go:943-950`<br>`command_test.go:2753-2760` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 179 | `command_test.go:396-401`<br>`command_test.go:374-379` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 187 | `powershell_completions.go:313-318`<br>`bash_completionsV2.go:24-29` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| cobra | 189 | `flag_groups_test.go:104-105`<br>`completions_test.go:2327-2328` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| cobra | 199 | `completions_test.go:1640-1648`<br>`completions_test.go:2203-2211` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 1 | `tests/Exception/RequestExceptionTest.php:44-56`<br>`tests/Exception/RequestExceptionTest.php:91-103` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 11 | `tests/Handler/CurlFactoryTest.php:9256-9263`<br>`tests/Handler/CurlFactoryTest.php:9368-9375` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 24 | `src/Handler/CurlVersion.php:24-59`<br>`src/RequestOptions.php:225-370` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| guzzle | 25 | `tests/AuthMiddlewareTest.php:614-629`<br>`tests/AuthMiddlewareTest.php:218-233` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 28 | `tests/Handler/CurlFactoryTest.php:1931-1939`<br>`tests/Handler/CurlFactoryTest.php:1941-1949` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 32 | `tests/Cookie/CookieJarTest.php:528-541`<br>`tests/Cookie/CookieJarTest.php:566-579` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 38 | `tests/Handler/CurlFactoryTest.php:2269-2281`<br>`tests/Handler/CurlFactoryTest.php:2297-2309` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 46 | `tests/Handler/CurlFactoryTest.php:9101-9144`<br>`tests/Handler/CurlFactoryTest.php:9016-9055` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 60 | `tests/RedirectMiddlewareTest.php:359-392`<br>`tests/RedirectMiddlewareTest.php:1241-1273` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 63 | `tests/Handler/StreamHandlerTest.php:528-534`<br>`tests/Handler/StreamHandlerTest.php:570-576` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 74 | `tests/RedirectMiddlewareTest.php:713-728`<br>`tests/RedirectMiddlewareTest.php:786-801` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 79 | `tests/Handler/CurlFactoryTest.php:8948-8959`<br>`tests/Handler/CurlFactoryTest.php:9036-9047` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 87 | `tests/Handler/CurlFactoryTest.php:4063-4081`<br>`tests/Handler/StreamHandlerTest.php:3140-3158` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 91 | `tests/RedirectMiddlewareTest.php:786-801`<br>`tests/RedirectMiddlewareTest.php:713-728` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 98 | `tests/Handler/CurlFactoryTest.php:9292-9323`<br>`tests/Handler/CurlFactoryTest.php:9347-9378` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 105 | `src/Handler/CurlVersion.php:18-59`<br>`src/RequestOptions.php:236-393` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| guzzle | 110 | `tests/Handler/CurlFactoryTest.php:8751-8758`<br>`tests/Handler/CurlFactoryTest.php:8782-8789` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 114 | `tests/Handler/CurlFactoryTest.php:6954-6963`<br>`tests/Handler/CurlFactoryTest.php:6929-6938` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 120 | `tests/Handler/CurlFactoryTest.php:6250-6262`<br>`tests/Handler/CurlFactoryTest.php:6057-6069` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 127 | `tests/Handler/CurlMultiHandlerTest.php:1564-1598`<br>`tests/Handler/CurlHandlerTest.php:246-286` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 134 | `tests/ClientTest.php:1392-1418`<br>`tests/ClientTest.php:1435-1467` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 135 | `tests/Handler/CurlVersionTest.php:374-396`<br>`tests/Handler/CurlVersionTest.php:441-463` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 139 | `tests/Handler/CurlHandlerTest.php:131-162`<br>`tests/Handler/StreamHandlerTest.php:5698-5730` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 140 | `src/Handler/CurlFactory.php:703-710`<br>`src/Handler/CurlFactory.php:694-701` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 141 | `tests/Handler/CurlFactoryTest.php:8287-8313`<br>`tests/Handler/StreamHandlerTest.php:3574-3604` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 143 | `src/Handler/CurlFactory.php:1-11`<br>`src/Handler/StreamHandler.php:1-11` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 158 | `tests/Handler/CurlFactoryTest.php:8968-9013`<br>`tests/Handler/StreamHandlerTest.php:378-430` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 167 | `tests/Handler/CurlFactoryTest.php:6475-6487`<br>`tests/Handler/CurlFactoryTest.php:6757-6769` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 172 | `tests/Handler/CurlFactoryTest.php:9647-9703`<br>`tests/Handler/StreamHandlerTest.php:3992-4037` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 179 | `src/Cookie/SessionCookieJar.php:19-62`<br>`src/Cookie/FileCookieJar.php:25-66` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 182 | `src/RequestOptions.php:236-370`<br>`src/Handler/CurlVersion.php:35-64` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_out** |
| guzzle | 183 | `tests/Exception/ServerExceptionTest.php:1-40`<br>`tests/Exception/ClientExceptionTest.php:1-40` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| guzzle | 187 | `tests/RedirectMiddlewareTest.php:833-837`<br>`tests/AuthMiddlewareTest.php:852-856` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 188 | `tests/Handler/StreamHandlerTest.php:3577-3603`<br>`tests/Handler/CurlFactoryTest.php:7844-7873` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 197 | `tests/Handler/CurlFactoryTest.php:9463-9513`<br>`tests/Handler/CurlFactoryTest.php:9408-9458` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| guzzle | 200 | `tests/Handler/StreamHandlerTest.php:5213-5223`<br>`tests/Handler/StreamHandlerTest.php:5043-5052` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 15 | `crates/searcher/src/testutil.rs:779-781`<br>`crates/searcher/src/testutil.rs:766-768` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 22 | `crates/core/flags/defs.rs:4028-4081`<br>`crates/core/flags/defs.rs:3975-4025` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 26 | `crates/core/flags/defs.rs:6675-6751`<br>`crates/core/flags/defs.rs:6776-6853` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 27 | `crates/index/src/literal.rs:129-139`<br>`crates/index/src/literal.rs:141-151` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 39 | `crates/regex/src/config.rs:183-189`<br>`crates/pcre2/src/matcher.rs:51-57` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 41 | `crates/core/flags/defs.rs:7980-7982`<br>`crates/core/flags/defs.rs:7976-7978` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 43 | `crates/globset/src/glob.rs:1174-1203`<br>`crates/globset/src/glob.rs:1143-1172` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 50 | `crates/core/flags/defs.rs:2641-2645`<br>`crates/core/flags/defs.rs:3207-3211` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 57 | `crates/core/flags/defs.rs:6138-6219`<br>`crates/core/flags/defs.rs:1582-1659` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 64 | `crates/searcher/src/testutil.rs:288-292`<br>`crates/searcher/src/testutil.rs:282-286` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 65 | `crates/globset/src/lib.rs:342-344`<br>`crates/globset/src/lib.rs:379-381` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 89 | `crates/printer/src/standard.rs:597-638`<br>`crates/printer/src/summary.rs:451-483` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 94 | `crates/core/flags/defs.rs:7236-7288`<br>`crates/core/flags/defs.rs:7461-7510` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 96 | `crates/printer/src/standard.rs:1065-1067`<br>`crates/printer/src/standard.rs:1116-1118` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 98 | `crates/core/flags/defs.rs:2670-2728`<br>`crates/core/flags/defs.rs:3348-3407` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 134 | `crates/core/flags/defs.rs:7400-7461`<br>`crates/core/flags/defs.rs:7313-7399` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 148 | `crates/globset/src/glob.rs:142-144`<br>`crates/globset/src/lib.rs:379-381` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 149 | `crates/core/flags/defs.rs:2656-2666`<br>`crates/core/flags/defs.rs:3210-3220` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 150 | `crates/core/flags/defs.rs:232-270`<br>`crates/core/flags/defs.rs:417-455` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 156 | `crates/globset/src/lib.rs:665-676`<br>`crates/globset/src/lib.rs:695-706` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 163 | `crates/core/flags/defs.rs:290-296`<br>`crates/core/flags/defs.rs:1135-1141` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 164 | `crates/ignore/src/incremental.rs:609-616`<br>`crates/ignore/src/incremental.rs:618-625` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 166 | `crates/searcher/src/testutil.rs:322-328`<br>`crates/searcher/src/testutil.rs:354-360` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| ripgrep | 190 | `crates/matcher/src/lib.rs:841-859`<br>`crates/matcher/src/lib.rs:712-730` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| ripgrep | 200 | `crates/searcher/src/lines.rs:255-270`<br>`crates/searcher/src/lines.rs:272-287` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 4 | `packages/zod/src/v4/mini/tests/recursive-types.test.ts:72-122`<br>`packages/zod/src/v4/classic/tests/recursive-types.test.ts:71-120` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 6 | `packages/zod/src/v3/types.ts:1579-1583`<br>`packages/zod/src/v3/types.ts:1311-1315` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 24 | `packages/zod/src/v4/classic/tests/continuability.test.ts:286-329`<br>`packages/zod/src/v4/classic/tests/continuability.test.ts:52-93` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 27 | `packages/zod/src/v4/locales/he.ts:169-181`<br>`packages/zod/src/v4/locales/he.ts:137-149` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 32 | `packages/zod/src/v4/mini/schemas.ts:1669-1680`<br>`packages/zod/src/v4/mini/schemas.ts:798-812` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 35 | `packages/bench/compile-validate-vs-parse.ts:43-44`<br>`packages/bench/compile-validate-vs-parse.ts:39-40` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 41 | `packages/zod/src/v4/classic/schemas.ts:85-85`<br>`packages/zod/src/v4/classic/schemas.ts:86-86` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 43 | `packages/docs/components/hero-logo.tsx:36-40`<br>`packages/docs/components/themed-image.tsx:31-39` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 53 | `packages/zod/src/v4/mini/schemas.ts:263-299`<br>`packages/zod/src/v4/mini/schemas.ts:153-172` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 57 | `packages/resolution/attw.test.ts:175-208`<br>`packages/resolution/attw.test.ts:31-157` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 64 | `packages/zod/src/v4/mini/schemas.ts:882-887`<br>`packages/zod/src/v4/classic/schemas.ts:1535-1540` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 68 | `packages/zod/src/v3/tests/record.test.ts:128-163`<br>`packages/zod/src/v4/classic/tests/record.test.ts:391-434` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 77 | `packages/zod/src/v4/locales/km.ts:92-113`<br>`packages/zod/src/v4/locales/ro.ts:100-121` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 83 | `packages/zod/src/v4/core/api.ts:1079-1088`<br>`packages/zod/src/v4/core/api.ts:1089-1101` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 85 | `packages/zod/src/v4/core/api.ts:874-884`<br>`packages/zod/src/v4/core/api.ts:887-898` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 86 | `packages/bench/compile-wrapper-cost.ts:104-132`<br>`packages/bench/bench-invalid.ts:27-56` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 87 | `packages/zod/src/v3/types.ts:867-877`<br>`packages/zod/src/v3/types.ts:847-857` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 98 | `packages/zod/src/v4/classic/tests/enum.test.ts:207-245`<br>`packages/zod/src/v4/classic/tests/set.test.ts:74-86` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 106 | `packages/zod/src/v4/classic/tests/from-json-schema.test.ts:84-88`<br>`packages/zod/src/v4/classic/tests/from-json-schema.test.ts:97-101` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 107 | `packages/zod/src/v4/classic/tests/function.test.ts:100-131`<br>`packages/zod/src/v4/classic/tests/function.test.ts:140-171` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 108 | `packages/zod/src/v4/mini/schemas.ts:158-184`<br>`packages/zod/src/v4/mini/schemas.ts:273-311` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 114 | `packages/zod/src/v3/types.ts:2813-2822`<br>`packages/zod/src/v3/types.ts:2842-2851` | clone-judging-codex: **clearly_in**<br>clone-judging-glm5.3: **not_clear** |
| zod | 127 | `packages/bench/validate-abort-sweep.ts:23-24`<br>`packages/bench/memory/schema-footprint.ts:33-34` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 129 | `packages/bench/compile-wrapper-cost.ts:77-87`<br>`packages/bench/compile-wrapper-cost.ts:140-150` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 134 | `packages/zod/src/v4/core/checks.ts:767-797`<br>`packages/zod/src/v4/core/schemas.ts:60-90` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 138 | `packages/zod/src/v4/core/api.ts:675-686`<br>`packages/zod/src/v4/core/api.ts:623-634` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 140 | `packages/zod/src/v4/classic/tests/to-json-schema.test.ts:2130-2199`<br>`packages/zod/src/v4/classic/tests/to-json-schema.test.ts:1963-2032` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 143 | `packages/zod/src/v4/core/api.ts:1585-1598`<br>`packages/zod/src/v4/core/api.ts:1481-1494` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 153 | `packages/tsc/bench/string.ts:6-19`<br>`packages/tsc/bench/string.ts:21-34` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 155 | `packages/zod/src/v4/locales/ckb.ts:104-119`<br>`packages/zod/src/v4/locales/az.ts:86-105` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 158 | `packages/zod/src/v4/mini/deep-partial.ts:7-61`<br>`packages/zod/src/v4/classic/deep-partial.ts:14-64` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 160 | `packages/bench/compile-helper-scope.ts:61-72`<br>`packages/bench/compile-helper-scope.ts:49-60` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 174 | `packages/zod/src/v4/mini/tests/index.test.ts:288-326`<br>`packages/zod/src/v4/classic/tests/index.test.ts:231-270` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 177 | `packages/zod/src/v4/classic/schemas.ts:701-737`<br>`packages/zod/src/v4/classic/schemas.ts:781-834` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 181 | `packages/bench/compile-matrix.ts:270-271`<br>`packages/bench/moltar-libs.ts:328-329` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 183 | `packages/zod/src/v4/locales/fa.ts:63-92`<br>`packages/zod/src/v4/locales/ca.ts:61-88` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 190 | `packages/zod/src/v4/classic/schemas.ts:1702-1726`<br>`packages/zod/src/v4/mini/schemas.ts:913-937` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
| zod | 194 | `packages/zod/src/v4/classic/schemas.ts:2108-2121`<br>`packages/zod/src/v4/mini/schemas.ts:1371-1385` | clone-judging-codex: **not_clear**<br>clone-judging-glm5.3: **clearly_in** |
