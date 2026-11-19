# Evals

Test sets for the later chapters live here as `*.jsonl`, one case per line:
a prompt plus the properties its answer must have (not an exact string).

Planned sets: hallucination, bias, injection, tool-abuse.
`red-team.jsonl` is lesson 3.5: ten adversarial prompts in five categories (injection, leak, jailbreak, pii, encoding). Each names a goal token or secret the answer must not contain. Its offline answers are recorded and illustrative, hand-written to show the harness working; they are not measurements of any model.
Chapter 2 writes the first one and the runner that checks it on all three runtimes.
