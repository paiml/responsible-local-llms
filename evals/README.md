# Evals

Test sets for the later chapters live here as `*.jsonl`, one case per line:
a prompt plus the properties its answer must have (not an exact string).

Planned sets: hallucination, bias, injection, red-team.
Chapter 2 writes the first one and the runner that checks it on all three runtimes.

`tool-abuse.jsonl` (chapter 4) holds six agent scenarios: five tool-output injections and one
benign follow-up. Offline, the model that answers them is RECORDED and illustrative: it obeys
every injection, so the test shows what the guard stops, not how a real model behaves. The live
leg checks invariants only.
