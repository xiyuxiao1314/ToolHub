# B02-R3 repair map (R3-F01–15 + carryover)

HEAD: see bundle / MANIFEST.

| ID | Status | Evidence |
| --- | --- | --- |
| R3-F01 | I/V | policy weakening requires controller; approve requires admin |
| R3-F02 | I/P | Tauri desktop + pipe principal; Unix socket ownership still simplified |
| R3-F03 | I/V | length-prefix digests; fail-closed empty; policy/trust digest |
| R3-F04 | I/V | ownership-checked cancel; nonexistent = not_found_or_not_owned |
| R3-F05 | I/V | Full-only Missing; Quick preserves untouched |
| R3-F06 | I/V | PE signature gate (junk MZ unknown) |
| R3-F07 | I/V | invalid version reject; Eq prerelease match |
| R3-F08 | I/V | strict confidence; candidate must exist |
| R3-F09 | I/V | SkillManifest validation at register |
| R3-F10 | I/V | MCP conformance 6/6 |
| R3-F11 | I/V | Tauri/React/TS nine pages, typecheck/build, release bin |
| R3-F12 | P | temp creds + extension trait; real agent launch U |
| R3-F13 | I/V | update rollback on copy failure |
| R3-F14 | I/V | report schema v1 export |
| R3-F15 | I/V | recursive package checksums + ui-dist + sdk |

## R2 carryover (mapped)

R2-B01→F01/F03; B02→F02; B03→F04; B04→F05; B05→F06; B06→F07; B07→F10; B08→F02/F08; B09→F09; B10→F14/F15.

## Original targets

B01-01–18 and B02-01–22 remain required; this packet implements the R3 blocking subset with Tauri desktop. Honest U: macOS, external agent launch, full UI screenshot set.
