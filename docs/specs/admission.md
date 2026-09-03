# The admission algebra, explained

How Deslop decides that a pair of code occurrences is a duplicate, and how clusters are weighed. This is the plain-language companion to the owning spec, [fused.md](fused.md) — the spec ids, pins, provenance, and config surface live there, not here.

Everything happens at the level of a **pair** of occurrences. Pairs are admitted or rejected one at a time; admitted pairs are then transitively closed into clusters; clusters are weighed by mass alone. No pair score ever survives onto a cluster.

## How to read the notation

- $a$, $b$ — the two code occurrences being compared. A subscript picks which one a value belongs to: $n_a$ is "the node count of occurrence $a$".
- $p$ — a candidate pair (the two occurrences together). $c$ — a cluster.
- $|X|$ — the number of elements in set $X$. $\cap$ — elements in both sets. $\cup$ — elements in either set.
- $\max$ / $\min$ — the larger / smaller of the listed values.
- $\operatorname{clamp}(x, lo, hi)$ — $x$, pinned into the range $[lo, hi]$.
- $\mathbf{1}[\text{condition}]$ — the indicator: $1$ if the condition holds, $0$ if it doesn't.
- A hat, as in $\hat{J}$ — an *estimate* of the un-hatted quantity.
- $\land$ and, $\lor$ or, $\neg$ not, $\iff$ "exactly when", $\implies$ "implies".

## The structural signal

**Symbols:**

- $\mathrm{TED}(a,b)$ — **tree edit distance**: the minimum number of single-node edits — insert a node, delete a node, or relabel a node — needed to turn $a$'s normalised syntax tree into $b$'s. Identical trees cost $0$; unrelated trees cost a lot. Computed with the Zhang–Shasha algorithm over normalised node kinds, unit cost per edit.
- $n_a$, $n_b$ — the node counts of the two normalised trees.
- $S(a,b)$ — structural similarity, in $[0,1]$.

$$
S(a,b) = 1 - \frac{\mathrm{TED}(a,b)}{\max(n_a,\, n_b)}
$$

In words: the share of the bigger tree that survives the alignment. Edit nothing and $S = 1$; edit everything and $S = 0$.

Every normalised subtree also carries an exact fingerprint — a hash built from its children's hashes (a Merkle hash) — so two subtrees share a fingerprint exactly when their entire normalised structure is identical.

**Symbols:**

- $M(p)$ — fingerprint equality: $1$ when the pair's two fingerprints are equal, $0$ otherwise.

Fingerprint-equal pairs skip the alignment walk entirely:

$$
M(p) = 1 \;\implies\; S(p) = 1
$$

## The token signal

**Symbols:**

- **k-gram** — a run of $k$ consecutive normalised node kinds (a small sliding window over the tree's token stream).
- $G_a$, $G_b$ — the **sets of all k-grams** in occurrence $a$ and occurrence $b$. ($G_b$ reads "$G$ of $b$" — a set named $G$, with a subscript picking the occurrence.)
- $J(a,b)$ — Jaccard similarity: what fraction of all the k-grams either occurrence has are shared by both. $1$ means identical sets, $0$ means nothing in common.

$$
J(a,b) = \frac{|G_a \cap G_b|}{|G_a \cup G_b|}
$$

Computing that exactly for every pair is too slow, so Deslop estimates it with MinHash.

**Symbols:**

- $m$ — the signature length: how many hash slots each occurrence gets (`representation.minhash_signature_len`, default 128).
- $\sigma_a(i)$ — slot $i$ of $a$'s signature: the smallest hash value of any k-gram in $G_a$ under the $i$-th hash function.
- $\hat{J}$ — the estimate of $J$: the fraction of slots where the two signatures agree. Slot agreement happens with probability equal to the true Jaccard, so averaging many slots converges on it.

$$
\hat{J}(a,b) = \frac{1}{m}\sum_{i=1}^{m} \mathbf{1}\bigl[\sigma_a(i) = \sigma_b(i)\bigr]
$$

LSH banding turns those signatures into candidate discovery.

**Symbols:**

- $b$ — the number of bands the signature is cut into; $r$ — rows (slots) per band, so $m = b \times r$.
- $s$ — the pair's true Jaccard similarity.
- $P(s)$ — the probability that the pair collides in at least one band and so becomes a candidate at all.

$$
P(s) = 1 - (1 - s^r)^b
$$

Finally, fingerprint equality corrects the token signal: identical normalised trees necessarily have identical k-gram sets, so a lower estimate is a signature artifact.

$$
M(p) = 1 \;\implies\; J(p) = 1
$$

## The score and the bar

**Symbols:**

- $f(p)$ — the pair's shape score: the **stronger** of the two signals. Never a sum, never an average — both axes look at the same normalised tree, so adding them double-counts, and neither is a calibrated probability.

$$
f(p) = \max\bigl(S(p),\, J(p)\bigr)
$$

**Symbols:**

- $t(p)$ — the bar this pair must clear.
- $t_{\text{xlang}}$ — the lower cross-language threshold (`admission.cross_language_fused_threshold`).
- $0.85$ — the ordinary threshold (`admission.fused_threshold`, configurable).

$$
t(p) =
\begin{cases}
t_{\text{xlang}} & \text{cross-language and } M(p) = 0 \\
0.85 & \text{otherwise}
\end{cases}
$$

## Content evidence

$S$ and $J$ are computed on the *normalised* tree — identifiers and literals collapsed away — so a perfect shape score says nothing about what the code actually says. Content evidence measures exactly what normalisation erased, by comparing the raw source bytes at every collapsed position.

**Symbols:**

- **collapsed position** — a spot where normalisation erased authored text: an identifier or a literal.
- $k_{a,i}$ — the raw source bytes at collapsed position $i$ of occurrence $a$.
- $K_a$, $K_b$ — the full sets of content keys of $a$ and $b$.
- $F$ — the keys both occurrences share that are *non-authored* (grammar scaffolding, not something a person typed); these are removed so they can't pad the score.
- **scored positions** — the positions that either disagree or carry authored content.
- **operator contradiction** — a position where both occurrences have a behaviour-bearing operator and they disagree ($+$ vs $-$). This is a hard contradiction: the surrounding matches cannot outvote the operation that changed.
- $A(a,b)$ — **agreement**: the fraction of authored positions whose raw bytes match, in $[0,1]$.

$$
A(a,b) =
\begin{cases}
0 & \text{operator contradiction} \\[4pt]
\dfrac{|\{\,i : k_{a,i} = k_{b,i}\,\}|}{|\text{scored positions}|} & \text{positions align one-to-one} \\[10pt]
\dfrac{|K_a \cap K_b| - |F|}{|K_a \cup K_b| - |F|} & \text{otherwise}
\end{cases}
$$

An empty denominator in either branch yields $1$: there is no authored content on which the pair disagrees.

The second piece of content evidence asks: is every renamed identifier renamed the *same way*? That is what separates a legitimate Type-2 clone (systematic rename) from unrelated code that merely shares a shape.

**Symbols:**

- **anchors** — repeated identifier occurrences that pin the rename mapping down. A symbol seen once matches anything and proves nothing; only repetition carries binding proof.
- $h$ — the anchor count at which the discount factor reaches one half (`content_gate.rename_evidence_half_anchors`).
- **consistency** — how uniformly the literal substitutions follow one mapping, in $[0,1]$.
- **coverage** — how much of the rename mapping the evidence actually explains, in $[0,1]$.
- $q$ — the anchor discount: smoothly reduces trust in evidence supported by few anchors. Airtight evidence — perfect consistency, perfect coverage, enough anchors — is certified and takes $q = 1$ instead.
- $R$ — **rename consistency**: the weaker of consistency and coverage, discounted by $q$.

$$
q = \frac{\text{anchors}}{\text{anchors} + h}
\qquad
R =
\begin{cases}
0 & \text{operator contradiction} \\
\min(\text{consistency},\, \text{coverage}) \times q & \text{otherwise}
\end{cases}
$$

**Symbols:**

- $C(p)$ — **content support**: whichever population vouches harder. Never a mean — averaging would let two lukewarm signals impersonate one strong one.

$$
C(p) = \max\bigl(A(p),\, R(p)\bigr)
$$

## The gates

**Size coherence.**

**Symbols:**

- $n_l$, $n_r$ — the node counts of the pair's left and right endpoints.
- $\rho$ — the maximum allowed size ratio between them (`admission.max_endpoint_node_ratio`, default 4).

$$
\mathrm{size\_ok}(p) \iff M(p) = 1 \;\lor\; \max(n_l, n_r) \le \rho \cdot \min(n_l, n_r)
$$

In words: without an exact fingerprint match, a tiny snippet may not pair with a huge one.

**The LSH-only guard.** A pair carried by tokens alone must clear higher floors, because tiny scaffolding windows hit $J \approx 1$ by accident.

**Symbols:**

- $J_{\min}$ — the LSH-only Jaccard floor (`admission.lsh_only_min_jaccard`, default 0.90).
- $n_{\min}$ — the LSH-only minimum endpoint node count (`admission.lsh_only_min_node_count`, default 40).
- $\mathrm{rescue}(p)$ — defined next.

$$
\mathrm{lsh\_ok}(p) \iff \bigl(M(p) = 0 \land \neg\mathrm{rescue}(p)\bigr) \implies \bigl(J(p) \ge J_{\min} \land \min(n_l, n_r) \ge n_{\min}\bigr)
$$

(Explicit cross-language mode waives the node-count half of this guard.)

**The rescue.** A cross-file pair below the bar can still be admitted when structure, tokens, size, and raw content *all* corroborate — no axis rescues alone.

**Symbols:**

- $S_{\text{resc}}$ — the rescue's structural floor (`admission.shared_subtree_min_overlap`, default 0.75).
- $J_{\text{resc}}$ — the rescue's Jaccard floor (`admission.shared_subtree_min_jaccard`, default 0.65).
- $n_{\text{resc}}$ — the rescue's minimum endpoint node count (`admission.shared_subtree_min_node_count`, default 30).
- $A_{\text{resc}}$ — the rescue's raw-content agreement floor (`rescue.content_agreement_floor`, default 0.10).

$$
\mathrm{rescue}(p) \iff
\mathrm{cross\_file}(p)
\land f(p) < t(p)
\land S(p) \ge S_{\text{resc}}
\land J(p) \ge J_{\text{resc}}
\land \min(n_l, n_r) \ge n_{\text{resc}}
\land A(p) \ge A_{\text{resc}}
$$

**The content gate.** When the shape evidence saturates, the pair must additionally prove its raw content agrees, because saturated shape says nothing an echo couldn't say.

**Symbols:**

- $S_{\text{sat}}$ — the structural saturation floor (`routing.shape_identical_floor`).
- $J_{\text{sat}}$ — the token saturation floor (`content_gate.saturating_token_floor`, default 0.95).
- $u(p)$ — the content-support floor this pair must meet: `content_gate.support_floor` (default 0.70), in every scope; an unanchored LSH-only pair pays `content_gate.promote_floor` (default 0.85) instead ([FUSED-CONTENT-GATE]).

$$
\mathrm{required}(p) \iff M(p) = 1 \lor S(p) \ge S_{\text{sat}} \lor J(p) \ge J_{\text{sat}}
$$

$$
\mathrm{content\_ok}(p) \iff \neg\mathrm{required}(p) \lor C(p) \ge u(p),
\qquad
u(p) =
\begin{cases}
0.85 & \text{lsh\_only}(p) \\
0.70 & \text{otherwise}
\end{cases}
$$

## Admission

Every symbol here was defined above: $\mathrm{size\_ok}$ (endpoint sizes are coherent), $\mathrm{lsh\_ok}$ (token-only pairs cleared their higher floors), $f(p) \ge t(p)$ (the score cleared the bar), $\mathrm{rescue}(p)$ (the corroborated near-miss path), $\mathrm{content\_ok}$ (raw content agreed where it had to).

$$
\mathrm{admit}(p) \iff
\mathrm{size\_ok}(p)
\land \mathrm{lsh\_ok}(p)
\land \bigl(f(p) \ge t(p) \lor \mathrm{rescue}(p)\bigr)
\land \mathrm{content\_ok}(p)
$$

Admitted pairs form clusters by transitive closure. No score, agreement, or verdict crosses that boundary.

## Weight

**Symbols:**

- $\mathrm{nodes}(c)$ — the node count of the cluster's canonical extent (how big the duplicated thing is).
- $\mathrm{visible}(c)$ — how many visible occurrences the cluster has (how many copies exist).
- $\mathrm{mass}(c)$ — the cluster's weight: each copy beyond the first adds the whole extent again. Fewer than two visible occurrences means zero mass — one of something is not duplication.

$$
\mathrm{mass}(c) = \mathrm{nodes}(c) \times \max\bigl(\mathrm{visible}(c) - 1,\; 0\bigr)
$$

Reports sort by mass descending, ties broken by cluster id ascending so ordering is stable. Nothing else — no pair evidence, no policy multiplier — touches mass or order.

## The headline percentage

**Symbols:**

- $\mathrm{analysed\_loc}$ — total lines of code analysed.
- $\mathrm{duplicated\_loc}$ — lines that belong to at least one admitted duplicate. Each line counts once, no matter how many pairs reach it.

$$
\mathrm{duplication\_percent} =
\begin{cases}
0 & \text{if } \mathrm{analysed\_loc} = 0 \\[4pt]
\operatorname{clamp}\!\left(\dfrac{100 \times \mathrm{duplicated\_loc}}{\mathrm{analysed\_loc}},\; 0,\; 100\right) & \text{otherwise}
\end{cases}
$$

*Pair evidence decides pair admission. Closure forms the cluster. Mass alone weighs it.*
