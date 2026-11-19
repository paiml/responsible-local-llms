# Evals

Test sets for the later chapters live here as `*.jsonl`, one case per line:
a prompt plus the properties its answer must have (not an exact string).

Planned sets: hallucination, bias, injection, red-team, tool-abuse.
Chapter 2 writes the first one and the runner that checks it on all three runtimes.

## Chapter 2 sets (W3)

- `hallucination.jsonl`: questions with source passages; an answer must cite the right passage and quote it, or refuse with `NOT IN SOURCE` (lesson 2.2).
- `structured.jsonl` and `ticket.schema.json`: messages to turn into a ticket that validates against the schema (lesson 2.3).
- `elo.jsonl`: eval cases that models are ranked on, head to head (lesson 2.5).

The passages are invented ("Orchard Co-op"). The replay fixtures in `crates/ch2-evals/fixtures/` are labelled `illustrative` in every row: hand-written for the lesson, not model output, and no number from them is a measurement. Pull requests replay them; a live run is `RLL_LIVE=1` outside the PR path.
