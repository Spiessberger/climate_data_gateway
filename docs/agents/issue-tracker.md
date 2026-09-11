# Issue tracker: Local Markdown

Issues and specs live in `.scratch/`, which is ignored by Git.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`.
- Specs: `.scratch/<feature-slug>/spec.md`.
- Implementation tickets: `.scratch/<feature-slug>/issues/<NN>-<slug>.md`,
  numbered from `01`, with one file per ticket.
- Triage state: a `Status:` line near the top of each issue.
  Use the role strings in `triage-labels.md`.
- Append comments under a `## Comments` heading.

## Publish and fetch

When a skill says to publish to the issue tracker, create the
appropriate file above, creating directories as needed.

When a skill says to fetch a ticket, read its referenced path.
Resolve a bare issue number within the relevant feature directory;
ask for the feature if the number is ambiguous.

## Wayfinding operations

- Map: `.scratch/<effort>/map.md`, containing Notes,
  Decisions-so-far, and Fog.
- Child ticket: `.scratch/<effort>/issues/NN-<slug>.md`,
  numbered from `01`, with the question in the body.
- Record ticket type in `Type:`: research, prototype, grilling, or task.
- Wayfinding tickets use `Status: open`, `claimed`, or `resolved`.
  These are workflow states separate from triage labels.
- Record dependencies in `Blocked by: NN, NN`.
  A ticket is unblocked when every listed dependency is resolved.
- Frontier: choose the lowest-numbered open, unblocked ticket.
- Claim: save `Status: claimed` before starting work.
- Resolve: append the answer under `## Answer`, save
  `Status: resolved`, and append a gist and ticket link to
  Decisions-so-far in the map.
