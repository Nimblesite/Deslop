# Verdict disagreements

| field | value |
| --- | --- |
| generator | `scripts/corpus/merge-verdicts.mjs` |
| spec | `docs/specs/corpus.md` [CORPUS-REGISTER-MERGE] |
| judges | clone-judging-claude, clone-judging-codex, register |
| repositories | 6 |
| pairs_imported | 108 |
| pairs_left_out | 12 |
| types_unspecified | 0 |

## by_kind

| kind | pairs |
| --- | --- |
| clearly_in/not_clear | 8 |
| clearly_out/not_clear | 4 |

## by_repository

| repository | pairs |
| --- | --- |
| serilog | 4 |
| giraffe | 3 |
| dio | 2 |
| composer | 1 |
| hugo | 1 |
| react | 1 |

## disagreements

| kind | repository | candidate | candidate_occurrences | by_source | stated_reasons |
| --- | --- | --- | --- | --- | --- |
| clearly_in/not_clear | dio | 5 | `plugins/cookie_manager/test/cookies_persistance_test.dart:123-153`<br>`plugins/cookie_manager/test/cookies_persistance_test.dart:88-118` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The whole cookie-redirect test body was copied for the non-relative-redirect case; only the redirect location (relative Uri -> absolute Uri.parse) and the expected saved URL were changed.<br>clone-judging-codex: The cookie tests share extensive mock-response setup, but exercise absolute and relative redirect handling with a different URI expression. This remains a related-test pattern with some doubt. |
| clearly_in/not_clear | dio | 18 | `plugins/native_dio_adapter/test/conversion_layer_adapter_test.dart:269-289`<br>`plugins/native_dio_adapter/test/conversion_layer_adapter_test.dart:291-312` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The receiveTimeout test was copied to write the combined connect+receive timeout test: same mock, same fetch call, same catch block, with one extra connectTimeout argument and the expected message digit changed.<br>clone-judging-codex: The timeout tests reuse a delayed-client harness and exception assertions, while the combined case adds a timeout argument and changes the expected message. |
| clearly_in/not_clear | giraffe | 18 | `tests/Giraffe.Tests/RoutingTests.fs:598-620`<br>`tests/Giraffe.Tests/RoutingTests.fs:622-644` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The whole routeBind test was copied to write the next one: same app, same mock setup, same assertion block; only the test name, the request path and the expected string were changed.<br>clone-judging-codex: These are the matching and nonmatching cases of a route-binding test, using the same framework setup and assertion pattern. |
| clearly_in/not_clear | giraffe | 20 | `tests/Giraffe.Tests/RoutingTests.fs:129-205`<br>`tests/Giraffe.Tests/RoutingTests.fs:15-95` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The three `route` tests (GET /, GET /foo, GET /FOO -> 404) were copied wholesale to make the three `routex` tests, with route renamed to routex throughout and nothing else touched.<br>clone-judging-codex: The three routing tests repeat with routex changed to route, but the second range also includes the next section heading. The selected boundaries are not matched. |
| clearly_in/not_clear | serilog | 2 | `src/Serilog/ILogger.cs:1102-1124`<br>`src/Serilog/ILogger.cs:1261-1283` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The Error overloads <T> and <T0,T1> with exception are copied for Fatal with Error renamed to Fatal throughout; only the doc example sentence differs.<br>clone-judging-codex: These are pairs of one-call logging overloads with Error changed to Fatal, surrounded mostly by API documentation. The tiny forwarding implementations leave doubt about a substantive clone. |
| clearly_in/not_clear | serilog | 6 | `src/Serilog/ILogger.cs:1119-1142`<br>`src/Serilog/ILogger.cs:1278-1301` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: Same as the Error/Fatal pair one overload further along: the <T0,T1> and <T0,T1,T2> exception overloads of Error copied for Fatal with Error renamed to Fatal.<br>clone-judging-codex: The selected overloads repeat the same short forwarding pattern for Error and Fatal. Most of the span is documentation; the substantive implementation is only two calls. |
| clearly_in/not_clear | serilog | 7 | `test/Serilog.Tests/Core/LoggerTests.cs:546-560`<br>`test/Serilog.Tests/Core/LoggerTests.cs:530-544` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The async-sink disposal test is the sync-sink disposal test copied with the second sink's type and the flag checked on it switched to the async variant.<br>clone-judging-codex: The disposal tests share logger construction and two assertions while changing the sink kind. This is related-case test scaffolding. |
| clearly_in/not_clear | serilog | 18 | `test/Serilog.Tests/Settings/KeyValuePairSettingsTests.cs:52-64`<br>`test/Serilog.Tests/Settings/KeyValuePairSettingsTests.cs:66-79` | clone-judging-claude=clearly_in<br>clone-judging-codex=not_clear | clone-judging-claude: The count-of-matched-arguments test is the callable-methods test copied with a second supplied argument added and the expected parameter type changed.<br>clone-judging-codex: The related configuration-method tests share setup and selection assertions, while one adds an argument and expects a different overload. This is test-case scaffolding with doubt. |
| clearly_out/not_clear | composer | 13 | `src/Composer/Policy/PolicyConfig.php:185-186`<br>`src/Composer/Policy/PolicyConfig.php:183-184` | clone-judging-claude=clearly_out<br>clone-judging-codex=not_clear | clone-judging-claude: Four consecutive, different statements in one sequence: each loads a different policy config (advisories, malware, abandoned, cooldown). Calling different factories with the same arguments is not the same code written twice.<br>clone-judging-codex: These are two-line factory-call idioms for related policy classes, with no substantive routine selected. |
| clearly_out/not_clear | giraffe | 10 | `tests/Giraffe.Tests/RoutingTests.Props.fs:54-55`<br>`tests/Giraffe.Tests/RoutingTests.Props.fs:52-53` | clone-judging-claude=clearly_out<br>clone-judging-codex=not_clear | clone-judging-claude: Two different pairs of entries in the same route table (/guid + /short-guid vs /big-age + /price). Different paths, format specifiers, parameter types and labels; the only thing in common is the routef syntax every route line uses.<br>clone-judging-codex: These are two-line examples of the same route-formatting idiom, exercising different parameter types. |
| clearly_out/not_clear | hugo | 16 | `hugolib/disableKinds_test.go:336-338`<br>`hugolib/pagebundler_test.go:284-286` | clone-judging-claude=clearly_out<br>clone-judging-codex=not_clear | clone-judging-claude: Assertions from two unrelated tests on two different objects (a no-publish-resources bundle's permalink and resource count vs a headless bundle's title and empty permalink). Only the not-nil-then-check-properties assertion idiom is shared.<br>clone-judging-codex: These are three ordinary page-property assertions about related bundle objects; the checked properties and expected values differ. |
| clearly_out/not_clear | react | 18 | `packages/react-reconciler/src/__tests__/ReactDeferredValue-test.js:535-549`<br>`packages/react-reconciler/src/__tests__/ReactUse-test.js:1812-1815` | clone-judging-claude=clearly_out<br>clone-judging-codex=not_clear | clone-judging-claude: A 4-line create-root/render/expect/resolve step from a use() test set against a 15-line block of a useDeferredValue test that merely starts with the same create-root-and-render idiom; the assertions and resolution differ. 'This region is a copy of that one' is false.<br>clone-judging-codex: The shared root/render/null-output setup is test scaffolding. The longer range also includes a suspension log sequence and ends at a different resolution step. |

## types_left_unspecified

(none)
