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
