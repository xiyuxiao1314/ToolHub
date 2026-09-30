//! Frozen method names for THP JSON-RPC.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Ping,
    Status,
    ScanStart,
    ScanStatus,
    SearchTools,
    InspectTool,
    InspectInstance,
    ListEnvironments,
    ListDuplicates,
    ResolveCapability,
    ExecuteTool,
    ExecuteCancel,
    ApproveExecution,
    RevokeApproval,
    SkillList,
    SkillInspect,
    SkillResolve,
    DiscoveryStart,
    DiscoveryList,
    DiscoveryInspect,
    DiscoveryClassify,
    DiscoveryRevoke,
    AgentList,
    PolicyGet,
    PolicySet,
    ActivityList,
    ExportReport,
}

pub const METHOD_NAMES: &[(&str, Method)] = &[
    ("ping", Method::Ping),
    ("status", Method::Status),
    ("scan.start", Method::ScanStart),
    ("scan.status", Method::ScanStatus),
    ("registry.search", Method::SearchTools),
    ("registry.inspect_tool", Method::InspectTool),
    ("registry.inspect_instance", Method::InspectInstance),
    ("environment.list", Method::ListEnvironments),
    ("environment.duplicates", Method::ListDuplicates),
    ("resolve.capability", Method::ResolveCapability),
    ("execute.tool", Method::ExecuteTool),
    ("execute.cancel", Method::ExecuteCancel),
    ("execute.approve", Method::ApproveExecution),
    ("execute.revoke", Method::RevokeApproval),
    ("skill.list", Method::SkillList),
    ("skill.inspect", Method::SkillInspect),
    ("skill.resolve", Method::SkillResolve),
    ("discovery.start", Method::DiscoveryStart),
    ("discovery.list", Method::DiscoveryList),
    ("discovery.inspect", Method::DiscoveryInspect),
    ("discovery.classify", Method::DiscoveryClassify),
    ("discovery.revoke", Method::DiscoveryRevoke),
    ("agent.list", Method::AgentList),
    ("policy.get", Method::PolicyGet),
    ("policy.set", Method::PolicySet),
    ("activity.list", Method::ActivityList),
    ("export.report", Method::ExportReport),
];

impl Method {
    pub fn from_name(name: &str) -> Option<Method> {
        METHOD_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, m)| *m)
    }
}
