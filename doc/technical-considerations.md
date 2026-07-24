# Technical considerations — manifest v2

`version: 2` is a breaking format: V1 is rejected by normal commands. The schema accepts only `ticketing` and `forge` integrations. A forge is a code-development platform such as GitHub or GitLab, covering repository, review, and pipeline operations. Their operation vocabularies are closed in this version, so unknown operations and policy keys fail validation.

An integration's effective adapter is its `adapter`, then the root `adapter_overrides[kind][platform]`, then a shipped adapter. Adapter selection is technical only. Its scope is platform-defined, while repository paths live in `repos.<id>.integrations`.

Ticket patterns and priorities are optional routing hints. Multiple candidates are allowed; the lowest priority wins, while a tie requires an explicit integration. Forge selection additionally requires the integration to be mapped on the selected repository.

## Repository galaxies

A project may declare a galaxy of repositories in `repos`. Each repository has a project-relative `path`, optional `labels`, direct `depends_on` repository ids, and an `integrations` map from declared forge integrations to their platform repository paths. The dependency declarations keep multi-repository relationships available to the agent, while the integration map permits the same repository to be mirrored on several forges and routes each code operation only to a mapped integration. Validation rejects a dependency that names an unknown repository.

An omitted local `read` policy is `allow`; all other omitted capabilities are `deny`. This is evaluated again for each workflow-initiated call. Auth is secret-safe metadata only: adapters receive environment-variable names and read values themselves.

`senpai migrate v1` produces a non-writing proposal and review report. It flags role mappings, patterns/priorities, time fallbacks, mirrors, global workflow splits, and free-text rules; it does not invent cross-system links or write permissions.
