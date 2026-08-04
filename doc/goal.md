# SenpAI goals — v2

SenpAI is a project-scoped declarative context layer for coding agents. A manifest declares integrations, repositories, environments, documentation and bounded capsules without placing secrets in agent-visible configuration.

V2 has one external-system model. An integration declares its technical capabilities (`provides`), project routing (`handles`), local authorization policy and procedure (`workflow`), and adapter selection. The CLI resolves one operation before an agent contacts ticketing or a forge. A forge is a code-development platform such as GitHub or GitLab. A route, policy grant, and adapter capability must all exist for every call.

SenpAI does not contact a remote service during resolution, execute arbitrary commands, store secret values, or make workflows able to broaden policy. Its explicit `pipeline job-log` read command resolves policy first and is bounded, non-interactive, and limited to the shipped GitHub and GitLab adapters.
