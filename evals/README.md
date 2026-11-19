# Evals

Test sets live here as `*.jsonl`, one case per line: a prompt plus the properties its
answer must have (not an exact string). A case holds `id`, `prompt`, `must_contain` and
`must_not_contain`; matching is case-insensitive.

## The six risks of lesson 1.7, and the set that covers each

| Risk | Set | Status |
|------|-----|--------|
| Hallucination | `hallucination` | planned, chapter 2 |
| Bias | `bias` | planned |
| Prompt injection | `injection`, `red-team` | planned |
| Leaks (secrets and personal data in a prompt or answer) | `leaks` | planned |
| Excessive agency | `tool-abuse` | planned |
| Compliance (licenses, regulation) | `compliance` | planned |

Every risk has a planned set. Planned sets: hallucination, bias, injection, red-team,
tool-abuse, leaks, compliance. Chapter 2 writes the first one and the runner that checks it
on all three runtimes.

## Written now

| Set | Lesson | What it scores |
|-----|--------|----------------|
| `quantization-quality.jsonl` | 1.3 | Whether a smaller quantization (q4_k_m, q8_0, f16) still answers right. |

`cargo run -p ch1-sovereign --bin lesson3_quality` scores it. Offline it reads
`fixtures/quantization-quality.recorded.jsonl`, answers recorded per level for the demo; those
are illustrative, not a measurement. With `RLL_LIVE=1` it asks the real runtime and scores the
one level that runtime serves (`RLL_QUANT`, default `q4_k_m`). The contract is
`contracts/quantization-quality-v1.yaml`.

## Chapter 2 sets (W3)

- `hallucination.jsonl`: questions with source passages; an answer must cite the right passage and quote it, or refuse with `NOT IN SOURCE` (lesson 2.2).
- `structured.jsonl` and `ticket.schema.json`: messages to turn into a ticket that validates against the schema (lesson 2.3).
- `elo.jsonl`: eval cases that models are ranked on, head to head (lesson 2.5).

The passages are invented ("Orchard Co-op"). The replay fixtures in `crates/ch2-evals/fixtures/` are labelled `illustrative` in every row: hand-written for the lesson, not model output, and no number from them is a measurement. Pull requests replay them; a live run is `RLL_LIVE=1` outside the PR path.

## Chapter 3 sets

- `bias.jsonl` (lesson 3.1): probes of paired cases that differ only in a name or a group. A case id is `<probe>__<variant>`; every variant of a probe must get the same decision.
- `injection.jsonl` (lesson 3.2): documents that carry an instruction aimed at the model (`inject-*`, canary `PWNED-7731`) and clean controls (`clean-*`) that must not be blocked.

The recorded answers the offline demos replay are illustrative: they were written for the lessons and are not measurements of any model. Live runs (`RLL_LIVE=1`) measure a real runtime.

## Chapter 3 sets (3.3, 3.4)

`pii.jsonl` (lesson 3.3) and `guardrails.jsonl` (lesson 3.4) are hand-written synthetic strings. Every line
carries `"source":"illustrative"` (the loader refuses any other label except `recorded`): they show the shapes
a detector or rule must catch and the near misses it must not, and they are not measurements of any model.
Card numbers are the public test numbers, the AWS key is the documented example, phones use the 555-01xx range
and emails use example.com. `pii.jsonl` lines hold `text`, the expected `kinds` and the expected `redacted` text;
`guardrails.jsonl` lines hold `stage`, `text` (optionally `repeat`ed), the `expect`ed action, the `rule` under test,
`trip` (true for a case that fires the rule, false for its near miss), and optional `want` and `secrets`.

- `red-team.jsonl` (lesson 3.5): ten adversarial prompts in five categories (injection, leak, jailbreak, pii, encoding). Each names a goal token or secret the answer must not contain. Its offline answers are recorded and illustrative, hand-written to show the harness working; they are not measurements of any model.
