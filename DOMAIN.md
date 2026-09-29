# Domain baseline

## Core entities

| Entity | Meaning |
| --- | --- |
| ToolDefinition | What a software product is; identity, vendor, and supported categories |
| ToolInstance | One installed copy; version, platform, architecture, canonical path, availability |
| Interface | A concrete access method: CLI, GUI, API, MCP, library, automation, or other supported type |
| Capability | A namespaced function that a tool can provide and a skill can require |
| Environment | Installation or execution context, such as system, user, venv, conda, WSL, or application bundle |
| Owner / Origin | Who likely manages the instance and how it was acquired; these are separate concepts |
| Evidence | Source and confidence supporting recognition, ownership, availability, or trust |
| Trust | Verified, known, user-trusted, unknown, or blocked classification, with explicit policy behavior |
| ScanCandidate | Discovered item awaiting recognition; unknown candidates must not pollute tool definitions |
| Agent / Skill | Integration identity and capability requirements |
| ExecutionRecord | Redacted record of an authorized invocation and its outcome |

## Model rules

- Definition, installation instance, interface, capability, and environment are separate entities.
- Preserve multiple installations; duplicate analysis reports findings without merging or deleting copies.
- Preserve original and canonical paths, including link relationships and environment association.
- Ownership may be known, probable, or unknown. Directory names alone do not prove which agent installed a tool.
- AI classifications and user decisions carry provenance and confidence rather than becoming unsupported facts.
- Resolve capability requirements to candidate instances while retaining alternatives and the selection rationale.

## Decisions required in foundation work

Finalize identity formats, relationship cardinality, evidence provenance, confidence meaning, timestamps, availability/trust state transitions, path normalization, and version constraint handling. Agree on capability naming before populating shared resources.

The source design uses multiple illustrative spellings for video-frame extraction. Review and select canonical capability identifiers and any alias strategy; do not silently treat examples as an approved taxonomy.

Database schema and manifest/wire representations must share the same reviewed meanings. This document intentionally does not introduce a second set of implementation structs.
