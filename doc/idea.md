# AI Manager - Ideas to Explore

Parking lot for design ideas that are not yet decided. Unlike [notes.md](notes.md)
(open questions and cross-cutting decisions), entries here are speculative
directions to investigate later. Nothing here is committed to the spec until it
is promoted into [technical-considerations.md](technical-considerations.md) and
the [reference manifest](reference-manifest.jsonc).

---

## A dedicated secret configuration (secrets + hidden wrapped commands)

Raised 2026-07-10. To explore later.

### The idea

Instead of scattering credential references and executable wrapper strings across
the (shareable, committable) `.aimanager.jsonc`, introduce a **separate secret
configuration** that holds:

- **secrets / credential references**, and
- **hidden wrapped commands** (for example an SSH + one-shot command wrapper the
  agent is allowed to invoke without ever forming the SSH call itself).

This secret config could live **inside the project folder** (gitignored, like the
current personal overlay) or **outside it entirely** (a machine-level / per-user
location referenced by the manifest). Location is an open question.

The manifest stays the clean, declarative, committable description of *what
exists*; anything sensitive or executable-and-hidden moves into the secret config.

### What motivates it

0. **Env-variable sprawl is the root pain.** The number of environment variables
   an agent must have defined can explode and becomes annoying to manage: one per
   credential, and (see below) potentially one per `user`/`host`/`port`/`database`
   field across every data store, times every environment. Spread across shell
   profiles they are scattered, easy to get wrong, and hard to hand off. A single
   secret config gathers them in one managed place instead of N loose env vars.

1. **More fields want to be env-referenced, not just `password_env`.** For
   `data_stores`, the intent is that `user`, `host`, `port`, and `database` should
   also be environment variables, not only the password. Under the current model
   that means an inline `*_env` reference per field, multiplying references across
   every store. A dedicated secret store keeps the manifest readable and puts all
   of these in one place.
   - Tension to resolve: this contradicts the currently-decided
     "usernames are plain (not secrets)" and the "everything referenced inline by
     env-var name" rule in [technical-considerations.md](technical-considerations.md)
     section 1.7. If the secret config wins, 1.7 is reshaped rather than extended.

2. **Wrapped commands are considered safe.** The `local_commands` trust-gate
   (confirmation-on-first-use for a committed block, section 1.10) is not a concern
   in this direction: wrapper commands are treated as safe. That removes the reason
   to keep them out of a dedicated, possibly-committed-or-not secret config, and
   makes "hidden wrapped commands live next to the secrets they use" a natural fit
   (for example a DB-query wrapper and its credentials in one place).

### Open questions (for later)

- **In-folder vs out-of-folder**, and how the manifest points at an out-of-folder
  config (path? name convention? machine-level default location?).
- **Relationship to the existing personal overlay** (`.aimanager.local.jsonc`,
  section 1.9). Is the secret config the same mechanism generalized, a distinct
  file, or does it replace the overlay for the secret/command concerns?
- **What exactly moves out of the manifest**: only secret *references*, or also
  connector fields like `host`/`port`/`database`/`user` for `data_stores`?
- **Argument passing for wrapped commands**, still unspecified in the
  `{ run, label }` shape; a one-shot parametric wrapper (`wrapper "<SQL>"`) needs
  a defined parameter slot if it becomes a first-class citizen here.
- **Secret-value policy**: does "no secret values in the manifest" extend
  verbatim to this secret config (references only), or is the secret config the
  one place real values may live? This is the crux: it changes the whole
  no-secrets-stored non-goal ([goal.md](goal.md)).
- **`doctor` / `validate` impact**: keep the credential-name check and referential
  coherence working across two files.
