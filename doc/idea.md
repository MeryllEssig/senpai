# Senpai - Ideas to Explore

Parking lot for design ideas that are not yet decided. Unlike [notes.md](notes.md)
(open questions and cross-cutting decisions), entries here are speculative
directions to investigate later. Nothing here is committed to the spec until it
is promoted into [technical-considerations.md](technical-considerations.md) and
the [reference manifest](reference-manifest.jsonc).

---

## A dedicated secret configuration (secrets + hidden wrapped commands)

Raised 2026-07-10. **Resolved 2026-07-13** as the **capsule** model (named
"contract" until the 2026-07-17 rename) and promoted into the spec, so this
entry is closed.

- The resolution lives in [technical-considerations.md](technical-considerations.md)
  **1.10 Capsules**, with the related updates to 1.6, 1.7, 2.1, and 3.1,
  and a worked example in the [reference manifest](reference-manifest.jsonc)
  (a `capsules` block plus the shape of the never-committed
  `.senpai/capsules.local.json` values file).
- Shape of the resolution: a capsule is a bounded, non-interactive command AI
  Manager runs in its own process. Commands requiring no local values and private
  templates share this one primitive. For a private template, the manifest
  holds the `command` template; a gitignored local values file holds the value
  of each non-supplied `{variable}` (a literal or a `$ENV` reference); the agent
  fills `supplied` variables at call time. This settles the crux question (real
  secret values may live in the local values file, never in the manifest) and
  the local `doctor` / `validate` cross-file checks.
- The two points left open in 2026-07-13 were closed on 2026-07-17 (see 1.10):
  output scrubbing (literal replacement of resolved values in stdout and
  stderr) and the final name ("capsule"). The execution verb was simplified on
  2026-07-20 to `senpai run`, which also returns the declared template. The
  2026-07-22 decision made capsules the sole bounded project-operation abstraction
  before implementation. Interactive and unbounded commands remain explicitly
  unsupported.
