# Domain docs

This repo uses a single-context layout:

- `CONTEXT.md` at the repo root: domain vocabulary and context.
- `docs/adr/`: architecture decision records.

## Before exploring

Read `CONTEXT.md` and ADRs relevant to the area being explored.

If these files do not exist, proceed silently. The domain-modeling
skill creates them lazily when terms or decisions are resolved.

## Use the glossary's vocabulary

Use terms defined in `CONTEXT.md` when naming domain concepts in
issues, proposals, hypotheses, and tests.

If a needed concept is absent, reconsider whether the term fits
the project, or note the gap for domain-modeling.

## Flag ADR conflicts

Explicitly identify any existing ADR that a proposal contradicts,
and explain why reopening that decision may be warranted.
