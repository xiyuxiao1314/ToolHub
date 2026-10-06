# Portable Skill library — 2026-10-04

Owner approved a shared library of generic, reusable workflows. Preserve host Skills, existing records, tool trust and user-selected program collection. This change is local implementation and verification; it is not an extra formal review round or a GitHub publication.

| Work | Acceptance | State |
|---|---|---|
| Contract | Explicit platform/input/output/operation/host-dependency declaration; old v1 records readable | Complete |
| Import and inspection | Reject host-specific imports before upsert; retain old entries; recompute after content edits | Complete |
| MCP | No-query search also excludes incompatible entries; inspection explains incompatibility | Complete |
| Desktop | Declaration template, compatibility labels, readable requirements and scoped copying | Complete |
| Examples and integration | Generic media/PDF examples; Agent guidance describes portability and separate authorization | Complete |
| Verification | Unit/SDK checks and private native Desktop/MCP tests | Complete |
| Local delivery | Versioned package, preserved production data, installed Skill, production native checks | Complete |

Scope is declaration validation and a bounded known-host-reference assessment. It cannot prove arbitrary text portable. A Skill is task guidance and does not grant permission, tool trust or access to a host's exclusive APIs.
