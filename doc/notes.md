# AI Manager - Open Questions and Notes

Questions raised during design that are not yet answered, plus topics deliberately deferred. To be resolved with the project owner before or during implementation. Resolved entries stay in place with their decision, so numbering never shifts.

## Naming and format

1. **Manifest file name.** RESOLVED (2026-07-10): `.aimanager.jsonc`.
2. **Dotted (hidden) or visible file?** RESOLVED (2026-07-10): hidden dotted file, implied by the `.aimanager.jsonc` decision.

## Manifest semantics

3. **Galaxy composition.** The working assumption is nearest-wins (technical considerations 1.2). When a sub-repo has its own manifest and an outer galaxy manifest also exists, is nearest-wins enough, or should the nearer manifest merge with or explicitly `extends` the outer one? Galaxies may want "my own context plus the galaxy graph".
4. **How far does orchestration go?** RESOLVED (2026-07-10): no orchestration engine, no scripted actions. The manifest declares enough facts (galaxy members, dependencies, code hosting instances and their roles, trackers) for the agent to derive cross-repo actions by itself, such as creating one merge request per modified repo on the right instance. See technical considerations 1.4 and 1.5.
5. **Bare ticket numbers with several trackers.** "#1234" is ambiguous when a project declares our Redmine and the client's Jira. Options: try sources in declared order, ask the user, or per-source ID patterns (Jira keys like `ACME-123` are self-disambiguating; two Redmines are not).
6. **Write-target granularity.** Per-category routing (bug to the client's Jira, internal chore to our Redmine) is deferred in the technical considerations (1.3). Does any current real project need it from day one?
7. **Local overrides.** Is a gitignored `.ai-manager.local.jsonc` overlay (personal paths, personal preferences) needed, or is the parent-directory placement already covering every private case? Deferred until a real need appears.
8. **Environments taxonomy.** Free-form environment names, or a suggested convention (dev, staging/preprod, prod) that connectors attach to?

## CLI

9. **Implementation language and distribution.** Single static binary (Rust, Go) versus scripting runtime (Node, Python). Constraint stated: friction-free install on `$PATH`. No decision yet.
10. **Output format.** Compact markdown, JSON, or both behind a flag? LLM-optimized output was requested; the exact shape should be benchmarked against real agent consumption.
11. **Query interface.** Freeform natural-language-ish query ("logs prod") versus strict subcommands (`get logs --env prod`). Strict is more testable; freeform is more agent-friendly. Possibly both: strict subcommands plus a `match` helper.

## Skills and ecosystems

12. **Auto-trigger portability.** Automatic activation "when the question needs external data" depends on each ecosystem's skill-selection mechanism. Acceptable degraded mode for ecosystems without one (always-on short instructions file? manual invocation only?) to be defined per ecosystem.
13. **Skill distribution.** How are the generic skills installed per machine and per ecosystem (copy, symlink, package manager of each ecosystem)? Out of scope of the manifest itself, but needed for the "installed once on the machine" promise.
14. **Automation-discovery scope.** Should the discovery skill only propose manifest improvements, or also propose ecosystem-level automations (hooks, scheduled jobs)? The latter is powerful but drifts from the declarative core.

## Security and trust

15. **Trust model for shared manifests.** A manifest committed by others feeds instructions (rules, comments) to the user's agent. Is a confirmation-on-first-use (with change detection) wanted, or is that overkill for the current usage context (mostly self-authored manifests)?
16. **Write operations guardrails.** Should the manifest be able to declare operation-level permissions (for example "read-only on prod database", "never create tickets in the client tracker without asking")? The IF-THEN rules can express this informally; a dedicated field would be checkable by tooling.

## Raised during review, not yet discussed with the owner

17. **Team sharing.** If a colleague clones a repo with a committed manifest, credentials setup on their machine is undocumented territory (which variable names, which tools to install). The setup skill could offer a "join existing manifest" mode that reads the manifest and walks the newcomer through the machine-side setup.
18. **Manifest evolution feedback loop.** When the usage skill hits a gap (a needed connector missing), should it proactively propose a manifest edit, or stay silent and let the user invoke setup? Proposal: propose, never apply silently.
19. **Success criteria / v1 cut.** RESOLVED (2026-07-10): no version-based prioritization. Tasks are separated but executed in their natural dependency order, so that nothing built early has to be undone later.
20. **QA skill transcript handling.** Conversation transcripts differ per ecosystem (formats to inventory: file locations, JSONL vs other) and may contain sensitive client material. Should the QA skill redact or anonymize before analysis, and where do its reports live?
