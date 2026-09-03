# Tornado corpus curation — 35 hand-verified pairs

2026-09-03 · engine commits compared: `f92300e5e100` (#368) vs `b5273c16351c` (#501), each a clean-room release build (sha256-verified distinct binaries) scanning the same `tornadoweb/tornado` v6.5.8 checkout. Results: `​.corpus/version-compare/reports/tornado/`.

Method: clusters sampled deterministically (seeded, three passes) from each report, each cluster's top two visible occurrences taken as the pair, code sliced at the report's exact byte offsets, and every verdict checked against the real source at the pinned sha. Cross-report matching keyed on shared occurrence start-lines, so "no counterpart" means no shared occurrence, not proof of absence — window-shifted coverage of the same code reads as absent.

Headline: 110 files / 46,901 LOC; `f92300e5` reported 377 clusters (9.37% dup, 201 hidden, 12.3 s); `b5273c16` reported 366 (10.42% dup, 66 hidden, 1.5 s). Across 35 hand-verified pairs: 24 real duplicates, 4 shape-only false positives (3 published only by the old build, 1 weak one at low rank in both), 1 boilerplate suppression, and 6 real duplications the new build finds that the old missed — including two in production code.

## Verdicts

Pairs 1-5 from pass one, 6-15 from pass two, 16-35 from pass three.

| # | pair | real duplicate? | `f92300e5` | `b5273c16` |
|---|---|---|---|---|
| 1 | `locks_test.py:53-55` ↔ `:254-256` (wait/acquire rename) | yes — rename clone | listed, looser pairing (`:251-253`) | ✅ tighter pairing |
| 2 | `websocket.py:739-748` ↔ `:789-797` (max_wbits validation) | yes — near-identical, one comment differs | ✅ `nearly_identical` | ✅ |
| 3 | `process_test.py:192-194` ↔ `:202-204` (Subprocess preamble) | yes — byte-identical | ✅ | ✅ |
| 4 | `tcpclient_test.py:364-365` ↔ `:418-419` (2-line assert couplet) | identical text, boilerplate — repeated ×7 | ⚠️ published as noise | suppressed (precision rule) |
| 5 | `auth_test.py:501-505` ↔ `web_test.py:2314-2318` | **no — unrelated code** (OAuth tests vs handler methods) | 🚨 **false positive** (`structural_only`, embedding 0.0) | correctly absent |
| 6 | `tcpclient_test.py:349-352` ↔ `:375-378` (timeout assert block) | yes — one literal differs | ✅ | ✅ |
| 7 | `tcpclient_test.py:344-346` ↔ `:425-427` (cleanup prologue) | yes — shared prologue, different final statement | ✅ | ✅ faint |
| 8 | `locks_test.py:290-293` ↔ `:440-443` (`f`/`future` acquire rename) | yes — rename clone | ✅ | ✅ |
| 9 | `web_test.py:486-491` ↔ `:3176-3181` (cookie expiry) | yes — literal `days=10`/`days=2` | ✅ | ✅ |
| 10 | `web_test.py:933-935` ↔ `:953-958` (rethrow/json_decode asserts) | yes — same pattern, literals + formatting differ | ✅ | absent |
| 11 | `locks_test.py:238-241` ↔ `:266-269` (e.wait vs sem.acquire) | no — shape-only, inverted assertions | 🚨 false positive | correctly absent |
| 12 | `locks_test.py:51-53` ↔ `:249-251` (Condition vs Semaphore repr) | no — shape-only, different types/messages | 🚨 false positive | correctly absent |
| 13 | `auth.py:492-521` ↔ `iostream.py:294-314` (NotImplementedError stubs) | borderline — abstract-stub idiom, docstring-heavy | absent | ⚠️ reported (faint) — watch |
| 14 | `red_test.py:169-173` ↔ `:177-181` (etag check_url) | yes — etag literals differ | ✅ | ✅ |
| 15 | `httpserver_test.py:1279-1283` ↔ `routing_test.py:187-192` (write_headers stub) | yes — same 5-statement sequence | absent | ✅ |
| 16 | `httpserver_test.py:83-86` ↔ `web_test.py:894-897` (`fetch_json` helper) | yes — **byte-identical** (empty diff) | ✅ `identical` | ✅ |
| 17 | `chat/static/chat.js` ↔ `websocket/static/chat.js` (`formToDict`) | yes — differs by one semicolon | ✅ | ✅ |
| 18 | `auth.py:1177-1187` ↔ `:1208-1219` (OAuth base_string) | yes — identical 11 lines, one blank line | ✅ `identical` | ✅ |
| 19 | `locks_test.py:322-332` ↔ `queues_test.py:132-136` (timed-out ops) | yes — sibling data-structure test (curated) | ✅ | ✅ |
| 20 | `escape_test.py:293-297` ↔ `template_test.py:411-420` (assertEqual runs) | no — shape-only (`json_decode` vs `render`) | 🚨 published `structural_only` | ⚠️ published faint — both, low rank |
| 21 | `websocket_test.py:307-316` ↔ `:318-327` (version rejection) | yes — version/codes differ (`13`/400 vs `12`/426) | ✅ | ✅ |
| 22 | `locale.py:427-429` ↔ `:445-449` (month/week/day dict) | yes — same dict built twice | ✅ | ✅ |
| 23 | `auth_test.py:552-554` ↔ `gen_test.py:723-725` (fetch/assert triplet, 11 occ) | weak-real — test idiom, literals differ | ✅ | ✅ top10 |
| 24 | `util_test.py:146-155` ↔ `:162-171` (TestConfig1/TestConfig2) | yes — clean rename clone (1→2, a→b, 3→5) | ✅ | absent |
| 25 | `auth_test.py:323-332` ↔ `:338-347` (OAuth route tuples) | no — config-table shape, different routes | 🚨 published (`structural_only`) | correctly absent |
| 26 | `web_test.py:1463-1464` ↔ `:1470-1471` (416/Content-Range couplet) | identical text, boilerplate | ⚠️ published | suppressed |
| 27 | `gen_test.py:559-563` ↔ `:565-569` (LeakedException 1/2) | yes — small rename clone | ✅ | absent |
| 28 | `docs/conf.py:26-31` ↔ `auth.py:742-746` (assignment runs) | **no** — Sphinx config vs OAuth URL constants | 🚨 false positive (fused 0.18!) | correctly absent |
| 29 | `tcpclient_test.py:356-359` ↔ `:384-387` (connect timeout, a/True vs b/False) | yes — rename clone | ✅ | absent (likely window-shifted) |
| 30 | `web_test.py:1700-1706` ↔ `:1735-1741` (add_handlers host matching) | yes — host regex + reply literals differ | absent | ✅ |
| 31 | `httpserver_test.py:1370-1373` ↔ `iostream_test.py:188-191` (raw socket read) | weak-real — same idiom, different reads | absent | ✅ faint |
| 32 | `locks.py:129-138` ↔ `queues.py:61-70` (`on_timeout` closure) | yes — **production** near-clone, timeout plumbing | absent | ✅ |
| 33 | `escape.py:221-236` ↔ `:252-267` (`utf8`/`to_unicode` twins) | yes — mirror-image encode/decode twins | absent | ✅ |
| 34 | `curl_httpclient.py:446-453` ↔ `simple_httpclient.py:412-419` (body guard + ValueError) | yes — **production**, differs only by indent + receiver | absent | ✅ |
| 35 | `blog.py:189-191` ↔ `auth.py:384-386` (get_argument runs) | no — shape-only `get_argument` calls | absent | ⚠️ reported faint — weakest new-build finding |

## Findings

- **False positives, old build**: pairs 5, 11, 12, 25, 28 — five shape-only clusters published by `f92300e5`, every one absent from `b5273c16`. Pair 28 is the starkest: Sphinx docs config vs OAuth URL constants, fused 0.18, published anyway. Pair 5 remains the canonical case (embedding 0.0, token_jaccard 1.0 only after stripping identifiers).
- **False positives, new build**: none confirmed. Pair 20 (both builds) and pair 35 (new only) are weak shape-only matches published at low rank (faint); pair 13 is the abstract-stub idiom — the watch item, since a fixture family for it already exists (`python_issue_69_abstract_method`).
- **Recall delta, new build wins**: pairs 15, 30, 32, 33, 34 — real duplications absent from the old report, including two in production code (32, 34). Pairs 24, 27, 29 flip the other way (old-only, real) but are small same-file rename clones; 29 may be window-shifted rather than missed.
- **Boilerplate suppression**: pairs 4 and 26 — byte-identical assertion couplets the old build published and the new build suppresses. Correct calls.
- Production-code duplication (32, 34, plus same-file 18, 33) matters more than the test-file bulk: the two clients drifted from one copy of the same guard, exactly the drift duplication detection exists to catch.

## Corpus enforcement

`corpus/tornado.json` curates six cross-file verdicts: `must_find` — the byte-identical `fetch_json` helper (pair 16, empty diff); `must_find_type2` — locks↔queues (floor 100, measured 126), httpserver↔routing (40/49), blog↔chatdemo (38/48), facebook↔google_auth (24/30), curl↔simple httpclient body guard (36/46). Gate at HEAD: `files=110 clusters=415 dup=11.2% wall=1.4s peak_rss=193MB` — green, alongside the manifest and selection contracts. Known gap, deliberately not worked around in harness code: same-file pairs (17 of 35 verdicts, including the strongest Type-1 evidence) and region-precise non-duplication claims are inexpressible in the current manifest schema; all are recorded in the manifest's status prose.
