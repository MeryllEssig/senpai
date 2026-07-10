# AI Manager - User Stories

Format: **As** (who), **I want** (what), **so that** (why).

## Epic 1 - Declare project context (the manifest)

**US-1.1 - Declare external tools in one place**
As a developer, I want a single declarative manifest file in my project describing its external ecosystem (trackers, code hosting, environments, logs, data stores, documentation), so that any AI agent I use knows how to reach them without me re-explaining in every session.

**US-1.2 - Comment the manifest**
As a developer, I want the manifest to support comments (JSONC), so that I can document inline what each entry is and why it exists, for teammates and for the AI.

**US-1.3 - Declare several ticket trackers**
As a developer working for clients, I want to declare several trackers on one project (our Redmine plus the client's Redmine, Jira, or Linear), so that the AI consults the correct system depending on the situation.

**US-1.4 - Configure precisely where tickets go**
As a developer whose projects change tracker over their lifecycle (a dedicated project space inside the tracker during the build, then a shared general-maintenance space after go-live), I want to configure precisely which tracker and project space is the current write target, so that tickets are always created in the right place.

**US-1.5 - Declare data sources**
As a developer, I want to declare typed data-source connectors (SSH targets, preprod and prod log locations, databases, Elasticsearch, Solr, Redis, documentation locations in local or remote repositories), so that the AI knows how to fetch the data a request needs.

**US-1.6 - Declare intent rules (IF-THEN)**
As a developer, I want to declare rules such as "IF the task needs database access THEN use this skill", so that the AI picks the right approach by itself.
Constraint: the manifest stays purely declarative; it never installs the commands or skills it references.

**US-1.7 - Cover a multi-repo galaxy**
As a developer maintaining a main repository that contains a galaxy of interdependent sub-repositories, I want the manifest to describe the sub-repos and their dependencies, so that the AI can navigate and orchestrate work across repos that depend on each other.

**US-1.8 - Keep the manifest out of the client's repo**
As a developer on repositories where committing tooling files is not appropriate, I want to place the manifest in a parent directory of the repo, so that I get the full feature set without touching the client's codebase.

**US-1.9 - Work outside git**
As a developer, I want the manifest to work in any folder, not only git repositories, so that non-repo workspaces are covered too.

## Epic 2 - Query the context (the CLI)

**US-2.1 - Scoped answers, not dumps**
As an AI agent, I want a CLI that parses the manifest and returns only the slice relevant to the current request (for example just the log access for prod), so that I answer the user without wasting tokens on irrelevant context.

**US-2.2 - Validate the manifest**
As a developer, I want the CLI to validate a manifest (syntax, schema, coherence), so that mistakes are caught when I edit it rather than mid-task.

**US-2.3 - Resolve from anywhere in the project**
As an AI agent invoked in any subdirectory, I want the CLI to find the manifest by walking up the directory tree, so that resolution works regardless of the current working directory.

## Epic 3 - Use the context (usage skills)

**US-3.1 - Automatic activation**
As a user, I want the usage skill to activate automatically when my question requires external data (tickets, logs, database, documentation) on ecosystems that support automatic activation, and to remain manually invocable everywhere, so that I get project-aware answers without thinking about the plumbing.

**US-3.2 - Progressive discovery**
As a user, I want skills designed with progressive discovery (a minimal always-loaded entry point, details fetched only when needed), so that the AI understands immediately what to do and no tokens are consumed when the context is not needed.

**US-3.3 - Any agent ecosystem**
As a user of several agentic tools (Claude Code, Codex, Gemini CLI, OpenCode, and others), I want the skills and CLI to be agent-agnostic, so that one project setup serves every ecosystem.

## Epic 4 - Set up and maintain (management skills)

**US-4.1 - Guided setup**
As a user, I want a setup skill that interviews me about where my project's information lives (trackers, repos, environments, data sources), lets me describe complex cases in detail, and writes the manifest for me, so that setup requires no knowledge of the schema.

**US-4.2 - Prior analysis of the folder**
As a user, I want the setup skill to study the current folder first to assess the project's complexity (single repo, galaxy of repos, plain folder), so that the interview is informed and shorter.

**US-4.3 - No secret values read**
As a user, I want setup to never read the values of secret environment variables (it may ask me for variable names and where they are defined), so that secrets never enter the conversation or the manifest.

**US-4.4 - Tool installation assistance**
As a user, I want the setup skill to offer help installing the external CLIs my manifest relies on (Redmine CLI, GitLab CLI, GitHub CLI), so that declared capabilities are actually usable.

**US-4.5 - Didactic guidance**
As a user, I want every setup and management skill to explain what it does and what I must do (for example restarting the agent so new environment variables are picked up, or restarting a service), in my own language, so that I understand and trust the setup instead of executing it blindly.

**US-4.6 - Automation discovery**
As a user, I want a skill that reviews my project and my habits to suggest things that could be automated but are not yet, so that the setup keeps improving over time.

## Epic 5 - Project-wide constraints

**US-5.1 - English repository**
As a maintainer, I want everything in this project (code, docs, skills, schemas) written in English, so that it is publishable and universally readable.

**US-5.2 - User-language responses**
As a user, I want runtime interactions to happen in my language with no default language assumed, so that the tool adapts to me and not the reverse.

**US-5.3 - Tool-agnostic core, optional connectors**
As a maintainer, I want the core to stay agnostic of any specific tracker or tool, with connectors provided only when necessary, so that the system survives tool churn.
