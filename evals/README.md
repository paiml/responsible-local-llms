# Evals

Test sets for the later chapters live here as `*.jsonl`, one case per line:
a prompt plus the properties its answer must have (not an exact string).

Planned sets: hallucination, bias, injection, red-team, tool-abuse.
Chapter 2 writes the first one and the runner that checks it on all three runtimes.

## Chapter 3 sets

- `bias.jsonl` (lesson 3.1): probes of paired cases that differ only in a name or a group. A case id is `<probe>__<variant>`; every variant of a probe must get the same decision.
- `injection.jsonl` (lesson 3.2): documents that carry an instruction aimed at the model (`inject-*`, canary `PWNED-7731`) and clean controls (`clean-*`) that must not be blocked.

The recorded answers the offline demos replay are illustrative: they were written for the lessons and are not measurements of any model. Live runs (`RLL_LIVE=1`) measure a real runtime.
