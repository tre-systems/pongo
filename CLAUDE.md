# Claude Code Notes

Read `AGENTS.md` first — it is the source of truth for workflow, verification commands, architecture rules, and the code map in this repo.

Key reflexes for Pongo:

- Work on `main` and push there. A push to `main` auto-deploys to Cloudflare, so after a code change confirm CI is green and smoke-test <https://pongo.tre.systems>. Docs-only changes just need commit + push.
- The gate before pushing is `npm run test:all` (prettier, fmt, check, clippy, tests).
- `game_core` must stay deterministic and platform-agnostic — the same crate runs on both the client (prediction) and the authoritative server.
