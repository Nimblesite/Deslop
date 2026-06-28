# Reading list

<a id="read-list-deduped"></a>

**Deduplicated reading list.**

Canonical:
- [Baxter et al. 1998 — AST clone detection](https://leodemoura.github.io/files/ICSM98.pdf)
- [Chilowicz et al. 2009 — Syntax tree fingerprinting](https://igm.univ-mlv.fr/~chilowi/research/syntax_tree_fingerprinting/syntax_tree_fingerprinting_ICPC09.pdf)
- [SourcererCC — Scaling clone detection (Semantic Scholar)](https://www.semanticscholar.org/paper/SourcererCC:-Scaling-Code-Clone-Detection-to-Sajnani-Saini/e1abe96610cb3bc989e727f0b59cebedb14260f1)
- [NiCad clone detector](https://www.researchgate.net/publication/221219568_The_NiCad_clone_detector)

Recent (2024–2026):
- [SSCD — BERT + ANN scalable clone detection (Wiley 2024, gated)](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355)
- [Selecting & Combining LLMs for Clone Detection (arXiv 2510.15480)](https://arxiv.org/abs/2510.15480)
- [HyClone — LLM + execution validation (arXiv 2508.01357)](https://arxiv.org/abs/2508.01357)
- [Rator — tree encoding via node DoF (Springer 2025)](https://link.springer.com/article/10.1186/s42400-025-00456-4)
- [Empirical Study of LLM-Based Clone Detection (arXiv 2511.01176)](https://arxiv.org/abs/2511.01176)
- [Evaluating Small-Scale Code Models (arXiv 2506.10995)](https://arxiv.org/pdf/2506.10995)
- [CloReCo benchmarking platform, 2025](https://www.scitepress.org/Papers/2025/136449/136449.pdf)
- [Hybrid IR + BiLSTM semantic clone detection, PLOS ONE 2025](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0340971)
- [Multilingual Clone Detector Benchmark (arXiv 2409.06176)](https://arxiv.org/pdf/2409.06176)

Primitives:
- [MinHash](https://en.wikipedia.org/wiki/MinHash) · [SimHash](https://en.wikipedia.org/wiki/SimHash) · [LSH](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)
- [In Defense of MinHash Over SimHash (PMLR)](http://proceedings.mlr.press/v33/shrivastava14.pdf)

Surveys:
- [A Survey of Software Clone Detection from Security Perspective](https://www.semanticscholar.org/paper/A-Survey-of-Software-Clone-Detection-From-Security-Zhang-Sakurai/c834d313a2dca5747245c895b1a7c53e503ca8f6)
- [Survey of Clone Detection Techniques, Types I–IV](https://www.semanticscholar.org/paper/The-Survey-of-the-Code-Clone-Detection-Techniques-(-Kaur-Sharma/f5600f495f863fd9f62ed29873d509939cd09ca0)

<a id="read-list-literals"></a>

**Micro-clones, magic values, and constant drift.**

Grounds [literals.md](literals.md), [RANK-LITERAL-FAMILY], and [DECISION-LITERALS] — the
value-level lineage the canonical fragment-clone list above does not cover.

Micro-clones (why the size floor must not be lowered, and why sub-floor findings still matter):
- [Mondal, Roy & Schneider 2018 — Micro-clones in evolving software (SANER)](https://doi.org/10.1109/SANER.2018.8330196)
- [Islam, Mondal & Roy 2019 — Comparing bug replication in regular and micro clones (SANER)](https://doi.org/10.1109/SANER.2019.8667974) — micro-clones carry ~6× the consistent bug-fix changes of regular clones
- [van Tonder & Le Goues 2016 — Defending against the attack of the micro-clones (ICPC)](https://doi.org/10.1109/ICPC.2016.7503738) — 95% of micro-clone-fixing PRs merged uncontested
- [Li, Lu, Myagmar & Zhou 2004/2006 — CP-Miner: copy-paste and related bugs (OSDI/TSE)](https://doi.org/10.1109/TSE.2006.28) — value-indexed token mining + the unchanged-ratio forgotten-update heuristic
- [Beller, Zaidman & Karpov 2017 — The last line effect explained (EMSE)](https://doi.org/10.1007/s10664-016-9489-6)

Inconsistency & drift (the `constant_drift` grounding — transferred by inference, no direct study):
- [Juergens et al. 2009 — Do code clones matter? (ICSE)](https://doi.org/10.1109/ICSE.2009.5070547) — ~52% of clone classes change inconsistently; inconsistent change is fault-indicating
- [Engler, Chen, Hallem, Chou & Chelf 2001 — Bugs as deviant behavior (SOSP)](https://doi.org/10.1145/502034.502041) — belief-consistency z-ranking: the majority value outranks the deviant

Magic values & smells:
- [Eghbali & Pradel 2020 — No strings attached: an empirical study of string-related software bugs (ASE)](https://doi.org/10.1145/3324884.3416576)
- [Fowler — Replace Magic Literal](https://refactoring.com/catalog/replaceMagicLiteral.html)

Industrial rule defaults (the source-verified threshold provenance for [LITERAL-NOISE]):
- [SonarSource RSPEC S1192 — string literals should not be duplicated](https://rules.sonarsource.com/java/RSPEC-1192/) (threshold 3, min content length 5; in the default profile for Java/C#/Python and shipped for Dart)
- [SonarSource RSPEC S109 — magic numbers](https://rules.sonarsource.com/java/RSPEC-109/) (in **no** default profile — the opt-in precedent)
- [PMD — AvoidDuplicateLiterals](https://pmd.github.io/pmd/pmd_rules_java_errorprone.html#avoidduplicateliterals) · [Checkstyle — MagicNumber](https://checkstyle.org/checks/coding/magicnumber.html) (`constantWaiverParentToken` — the fix site never re-triggers)
- [ESLint — no-magic-numbers](https://eslint.org/docs/latest/rules/no-magic-numbers) · [goconst](https://github.com/jgautheron/goconst) (`match-constant` default-on — the `shadowed_constant` precedent) · [go-mnd](https://github.com/tommy-muehle/go-mnd)
- [rust-clippy #1539 — declined magic-number lint](https://github.com/rust-lang/rust-clippy/issues/1539) (the false-positive-rate argument) · [clippy approx_constant](https://rust-lang.github.io/rust-clippy/master/index.html#approx_constant) (per-constant `min_digits` gating)

Unused-symbol detection (the [LITERAL-UNUSED-MARKER] grounding):
- [vulture — confidence-scored dead-code detection for Python](https://github.com/jendrikseipp/vulture) (the 60/90/100 confidence model)
- [Eder et al. 2012 — How much does unused code matter for maintenance? (ICSE)](https://doi.org/10.1109/ICSE.2012.6227109)
- [Romano et al. 2018 — A multi-study investigation into dead code (TSE)](https://doi.org/10.1109/TSE.2018.2842781) — developers distrust deletion of possibly-dynamically-used code
- [rust-lang/rust #120079 — dead_code does not cover pub items across a workspace](https://github.com/rust-lang/rust/issues/120079) · [Knip — includeEntryExports](https://knip.dev/reference/configuration#includeentryexports)

<a id="read-list-merge"></a>

**Mechanical merge & behaviour-preserving refactoring.**

Grounds `[AUTOFIX-MERGE]` / `[AUTOFIX-CONSOLIDATE]` in [autofix-extract.md](autofix-extract.md). Every link below was verified to resolve with a faithful claim. (Baxter et al. 1998 — the differing-leaf `Similarity = 2S/(2S+L+R)` and leaf-ignoring hash — is already listed under *Canonical* above.)

Anti-unification (least general generalisation — the maths of turning differences into parameters):
- [Plotkin 1970 — A Note on Inductive Generalization (lgg)](https://homepages.inf.ed.ac.uk/gdp/publications/MI5_note_ind_gen.pdf)
- [Reynolds 1970 — Transformational Systems and the Algebraic Structure of Atomic Formulas](https://www.cs.cmu.edu/afs/cs/user/jcr/ftp/transysalg.pdf)
- [Cerna & Kutsia 2023 — Anti-unification and Generalization: A Survey (IJCAI)](https://arxiv.org/abs/2302.00277)
- [Bulychev & Minea 2008 — Duplicate Code Detection Using Anti-Unification (Clone Digger)](https://doi.org/10.15514/syrcose-2008-2-22)
- [Li & Thompson 2009 — Clone Detection and Removal for Erlang/OTP (Wrangler, PEPM)](https://doi.org/10.1145/1480945.1480971)

Clone refactoring / procedure extraction (the per-site argument lists + parameterisation):
- [Komondoor & Horwitz 2000 — Semantics-Preserving Procedure Extraction (POPL)](https://doi.org/10.1145/325694.325713)
- [Komondoor & Horwitz 2001 — Using Slicing to Identify Duplication (SAS)](https://doi.org/10.1007/3-540-47764-0_3)
- [Komondoor & Horwitz 2003 — Effective, Automatic Procedure Extraction (IWPC)](https://www.csa.iisc.ac.in/~raghavan/iwpc03-paper.pdf)
- [Krishnan & Tsantalis 2014 — Unification and Refactoring of Clones (CSMR-WCRE)](https://users.encs.concordia.ca/~nikolaos/publications/CSMR-WCRE_2014.pdf)
- [Tsantalis, Mazinanian & Krishnan 2015 — Assessing the Refactorability of Software Clones (TSE)](https://doi.org/10.1109/TSE.2015.2448531)
- [Tairas & Gray 2012 — Unifying clone detection and refactoring (CeDAR, IST)](https://doi.org/10.1016/j.infsof.2012.06.011)
- [Hotta, Higo & Kusumoto 2012 — Form Template Method via PDG (CSMR)](https://doi.org/10.1109/CSMR.2012.16)
- [Juillerat & Hirsbrunner 2007 — Toward Form Template Method (SCAM)](https://doi.org/10.1109/SCAM.2007.10)
- [Fowler — Refactoring catalog](https://refactoring.com/catalog/) · [Parameterize Function](https://refactoring.com/catalog/parameterizeFunction.html)
- [Code Clone Refactoring in C# with Lambda Expressions 2025 — value-vs-thunk (arXiv 2512.21511)](https://arxiv.org/abs/2512.21511)

Behaviour-preserving refactoring (the "zero-risk" definition + the binding invariant):
- [Opdyke 1992 — Refactoring Object-Oriented Frameworks (PhD thesis)](https://www.laputan.org/pub/papers/opdyke-thesis.pdf)
- [Schäfer, Ekman & de Moor 2008 — Sound and Extensible Renaming for Java (OOPSLA)](https://dl.acm.org/doi/10.1145/1449764.1449787)
- [Schäfer & de Moor 2010 — Specifying and Implementing Refactorings (OOPSLA)](https://dl.acm.org/doi/10.1145/1869459.1869485)
- [Schäfer 2010 — Specification, implementation and verification of refactorings (DPhil, Coq-verified)](https://ora.ox.ac.uk/objects/uuid:1a027679-1e2b-4fb5-a6ff-3270f15154a1)
- [Steimann 2018 — Constraint-Based Refactoring (TOPLAS)](https://doi.org/10.1145/3156016)
- [Steimann, Kollee & von Pilgrim 2011 — A Refactoring Constraint Language (ECOOP)](https://link.springer.com/chapter/10.1007/978-3-642-22655-7_13)

Clone-type theory (which clusters are mechanically mergeable):
- [Baker 1995 — On Finding Duplication and Near-Duplication (dup / p-match, WCRE)](https://plg.uwaterloo.ca/~migod/846/papers/wcre95-baker.pdf)
- [Baker 1996 — Parameterized Pattern Matching (prev-encoding, JCSS)](https://www.sciencedirect.com/science/article/pii/S0022000096900033)
- [Roy, Cordy & Koschke 2009 — Type-1..4 taxonomy (Sci. Comput. Program.)](https://doi.org/10.1016/j.scico.2009.02.007)
- [Bellon et al. 2007 — Clone-detection benchmark (TSE)](https://doi.org/10.1109/TSE.2007.70725)

IDE refactoring delivery + trust (LSP, previews, HCI):
- [LSP 3.17 — Code Action](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeAction) · [WorkspaceEdit](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspaceEdit) · [workspace/applyEdit](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_applyEdit)
- [rust-analyzer — assists architecture](https://rust-analyzer.github.io/book/)
- [IntelliJ — Structural Search and Replace](https://www.jetbrains.com/help/idea/structural-search-and-replace.html)
- [Eclipse LTK — Language Toolkit for refactorings](https://www.eclipse.org/articles/Article-LTK/ltk.html) · [JDT refactoring-wizard preview](https://help.eclipse.org/latest/topic/org.eclipse.jdt.doc.user/reference/ref-wizard-refactorings.htm)
- [Murphy-Hill & Black 2008 — Refactoring Tools: Fitness for Purpose (IEEE Software)](https://pdxscholar.library.pdx.edu/compsci_fac/109/)
- [Vakilian et al. 2012 — Use, disuse, and misuse of automated refactorings (ICSE)](https://dl.acm.org/doi/10.5555/2337223.2337251) ([open access](https://www.ideals.illinois.edu/items/27956))
