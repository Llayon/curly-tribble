# Bevy 0.18.1 Development Constitution

## AI Entry & Loop Prevention (Mandatory)
- **First Step**: Every session MUST start by reading `llms.txt`.
- **Loop Prevention**: If any operation fails 3 times, agent MUST:
  1. Announce "LOOP DETECTED".
  2. Perform `read_file` on the target file.
  3. Propose an alternative strategy to the user.
- **Self-Evolution**: Upon establishing a new high-level pattern, agent MUST:
  1. Create/Update a Satellite SOP in `docs/superpowers/patterns/`.
  2. Update `llms.txt`. Keep `GEMINI.md` lean.

## Core Mandates
- **Stability**: Windows DX12 (`WGPU_BACKEND="dx12"`). No `unwrap()` on queries.
- **Modularity**: Everything is a Plugin. `main.rs` is for init only.
- **Encapsulation**: Max 300 lines per file (Guard #21). Logic in sub-modules.
- **Reactivity**: Prefer `Observer` for picking. No polling in `Update` for state changes.
- **Verification**: 21+ Architectural Guards in `tests/architecture.rs`.

## Context Efficiency & Compression (Mandatory)
- **Summarization**: Always summarize tool outputs exceeding 20 lines. Focus on: Status, Key Error/Change, and Conclusion.
- **Reporting Deltas**: For every file modification, explicitly report the number of lines added (+) and removed (-).
- **Silent Tools**: Use quiet flags (e.g., `cargo test -q`).
- **Surgical Actions**: Use `grep_search` and targeted `read_file` (line ranges). Never read full files > 100 lines.
- **Efficiency**: Use `replace` (Surgical Edits) for large files. Perform a surgical read first.
- **SOPs**: Use `llms.txt` to find specific technical standards (SOPs). Load them only when needed.
