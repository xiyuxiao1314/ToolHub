# MCP behavior hints and assessment notes

`tools/list` exposes the four MCP ToolAnnotations booleans for every one of ToolHub's 19 meta-tools (beta.10 adds metadata-only `offer_skill_sync`; beta.12 adds observation-only `register_service` and `list_services`). See the [MCP schema](https://modelcontextprotocol.io/specification/2025-11-25/schema#toolannotations). These optional protocol hints help clients describe operations; they never override executable trust, exact-invocation grants, caller ownership, controller verification or artifact retention.

| Tools | readOnlyHint | destructiveHint | idempotentHint | openWorldHint |
| --- | --- | --- | --- | --- |
| execute_tool | false | true | false | true |
| cancel_task, complete_artifact_package, review_artifact_package | false | true | false | false |
| register_service, list_services, offer_skill_sync, get_task, search_programs, propose_program, search_tools, resolve_capability, inspect_tool, list_environments, search_skills, inspect_skill, register_artifact_package, list_artifact_packages, request_execution_approval | false | false | false | false |

The inventory/query handlers do not launch tools or modify user files. However, each successful MCP call also records the self-reported client's last activity in ToolHub's local agent inventory. This persistent timestamp means the complete operation is conservatively **not strictly read-only or idempotent**. The record is bounded display history, not an authorization identity. Approval/status, program proposals and artifact registration additionally update local bookkeeping; approval alone does not execute. Calls may also append audit/denial records.

`execute_tool` can run a policy-authorized local executable whose arguments may cause filesystem changes or network access, so it is destructive and open-world. Cancellation can interrupt work and leave partial outputs. Artifact completion/review can schedule later system-Trash recycling under existing selected-root, ownership, priority, grace-period and retention checks; the hint includes that deferred effect. These calls cannot permanently delete files or grant themselves controller authority. The other tools operate on the configured local registry, explicitly selected roots and declared files, rather than unbounded network entities.

## October 2026 external snapshot

M8ven's public listing inspected commit `0183648fce3236152207b7c8fecb1c62d730aa04` (beta.2), not the current release source. Missing behavior hints was a real current issue and is corrected in beta.6. Its displayed tool list contains 16 tools while suggestions use a denominator of 17; the beta.6 catalog/unique-name/protocol tests enforced 16 (beta.10 deliberately adds a seventeenth tool), so this discrepancy needs scanner clarification. "10/17 names referenced in tests" is a heuristic, not instrumented Rust coverage. The current conformance suite calls all nineteen tools through a real isolated daemon, including guarded mutations; existing crate/policy/executor/artifact tests remain required. We do not claim complete path coverage or a security certification.

A publisher claim, README badge, connected GitHub App, code score and execution/sandbox verification are separate states. Publishing newer source does not prove the third party has re-scanned it. ToolHub does not require M8ven access or contact any assessment service at runtime.

中文：所有 MCP 接口提供四项真实行为标注。查询会保存智能体活动时间，因此保守标记为非严格只读、非幂等；执行/取消及可能安排回收的完成/审核操作标记为有破坏性。标注不是授权，不提升信任、不绕过审批。外部报告是旧版本快照，16/17 数量不一致及按名称计算的测试比例需与扫描方区分确认；不能把分数当作安全认证。
