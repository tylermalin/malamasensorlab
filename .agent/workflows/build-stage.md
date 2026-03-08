---
description: build and test a single Odyssey stage
---

1. Ask the user which stage number (1–7) to build if not specified.
2. Run `cargo test --lib stage{N}` to see current status.
3. Implement any missing functionality from `docs/prompts.md` for that stage.
4. Run `cargo test --lib stage{N}` again — all tests must pass.
5. Run `cargo clippy -- -D warnings` and fix any warnings.
6. Commit with message: `feat(stage{N}): <short description>`
7. Push to GitHub.
8. Report: test count, pass/fail, and commit hash.
