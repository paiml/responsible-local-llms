# li-c1-rllm assembly inventory (2026-11-23)

Course: Responsible AI and Local LLMs. 6 chapters (0 to 5), 35 lessons, 137 narrated pieces.

| item | state | where |
|---|---|---|
| outline (35 lessons, chapters, titles, files) | EXISTS | catalogue `courses/li-c1-rllm/edx-outline.jsonl`; copied to `course/li-c1-rllm/outline.jsonl` |
| course record, edX run (`course-v1:AI+rllm1x+3T2026`, unpublished), about, sink edx | EXISTS | catalogue `courses/li-c1-rllm/`; edX quiz (58 MCQs) in catalogue history, not on main tree list |
| narration wavs, final pieces (137, 1.1 GB) | EXISTS (staging) | `/mnt/nvme-raid0/tmp/rmedia-cop/lib-rll/raw/li-c1-rllm/*.wav`; sha256 list committed as `audio.sha256`; NOT on NAS RecordedCourses |
| raw takes (148 takes + processed wavs + json) | EXISTS (staging) | `lib-rll/raw/li-c1-rllm/narration-takes` (444 wav files total) |
| teleprompter (137 pieces, status ready) | EXISTS | committed under `course/li-c1-rllm/teleprompter/` |
| demo bins (24 bins, lessons 1.2 to 5.4) | EXISTS, merged, gated | crates/*/src/bin; `evidence/demo-bins.jsonl`; lesson map `lesson-demos.tsv` |
| lessons with no demo bin | by design | ch0 (0.0 to 0.7), 1.1, 1.5 (shell script `whisper-pipeline.sh`), 1.6, 1.7 |
| course config | ADDED here | `course/li-c1-rllm/course.toml` |
| per-lesson manifest (35 rows) | ADDED here | `course/li-c1-rllm/manifest.jsonl`, derived and checked by `scripts/course-manifest.sh` |
| plates / animations (35 lessons) | MISSING, all 35 | none in the demo repo, rmedia repo, catalogue, NAS or staging |
| SRT per lesson | MISSING, all 35 | no transcription has been run (rmedia: whisper never at render time, `--transcribe` prepares the SRT) |
| rendered mp4 (35) | MISSING | outline rows say `rendered: false` |
| LinkedIn Drive folder | not started | no render to deliver |
| edX upload | not started | run exists, unpublished, no videos |

Per-lesson state: every lesson has pieces and teleprompter (35 of 35); 24 have a demo bin; 0 have a plate; 0 have an SRT; 0 rendered.

Doubles checked: the 137 wav names equal the 137 teleprompter names; the outline lists the same 35 lesson ids as the pieces.
