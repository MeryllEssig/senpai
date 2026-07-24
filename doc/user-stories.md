# User stories — v2

- As a developer, I declare every external system in one `integrations` inventory so ticketing and forges follow one model. A forge is a code-development platform such as GitHub or GitLab.
- As a developer, I distinguish what an adapter can do (`provides`) from what my project routes to it (`handles`) and what it authorizes (`workflow.policy`).
- As an agent, I resolve exactly one declared integration for a ticket or repository operation and stop on an unresolved ambiguity.
- As a developer, I can mirror a repository on several forges and select the correct one per operation.
- As a user, I can migrate a V1 manifest as a reviewable draft without a silent write grant or secret exposure.
- As a user, I can run only finite, declared capsules without shell execution.
