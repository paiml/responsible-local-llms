# Evals

Test sets for the later chapters live here as `*.jsonl`, one case per line:
a prompt plus the properties its answer must have (not an exact string).

Planned sets: hallucination, bias, injection, red-team, tool-abuse.
Chapter 2 writes the first one and the runner that checks it on all three runtimes.

## Chapter 3 sets

`pii.jsonl` (lesson 3.3) and `guardrails.jsonl` (lesson 3.4) are hand-written synthetic strings. Every line
carries `"source":"illustrative"` (the loader refuses any other label except `recorded`): they show the shapes
a detector or rule must catch and the near misses it must not, and they are not measurements of any model.
Card numbers are the public test numbers, the AWS key is the documented example, phones use the 555-01xx range
and emails use example.com. `pii.jsonl` lines hold `text`, the expected `kinds` and the expected `redacted` text;
`guardrails.jsonl` lines hold `stage`, `text` (optionally `repeat`ed), the `expect`ed action, the `rule` under test,
`trip` (true for a case that fires the rule, false for its near miss), and optional `want` and `secrets`.
