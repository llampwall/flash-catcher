use crate::event::{Classification, ProcessInfo};
use serde::Serialize;

/// A single classification rule. Order matters — rules are matched top-down.
#[derive(Debug, Clone, Serialize)]
pub struct ClassifyRule {
    pub name: &'static str,
    pub classification: Classification,
    /// Match against `process.name` (case-insensitive, exact).
    pub name_eq: Option<&'static str>,
    /// Match against `process.command_line` (substring, case-insensitive).
    pub cmdline_contains: Option<&'static str>,
    /// Match against any ancestor exe name in the blame chain.
    pub ancestor_name_eq: Option<&'static str>,
}

/// Built-in rules. Adjust here as new patterns are discovered.
pub const BUILTIN_RULES: &[ClassifyRule] = &[
    ClassifyRule {
        name: "claude-code-reg-machineguid",
        classification: Classification::ClaudeCodeProbe,
        name_eq: Some("reg.exe"),
        cmdline_contains: Some("MachineGuid"),
        ancestor_name_eq: None,
    },
    ClassifyRule {
        name: "claude-code-tasklist-probe",
        classification: Classification::ClaudeCodeProbe,
        name_eq: Some("tasklist.exe"),
        cmdline_contains: None,
        ancestor_name_eq: Some("claude.exe"),
    },
    ClassifyRule {
        name: "claude-code-cim-process",
        classification: Classification::ClaudeCodeProbe,
        name_eq: Some("powershell.exe"),
        cmdline_contains: Some("Get-CimInstance Win32_Process"),
        ancestor_name_eq: None,
    },
    ClassifyRule {
        name: "chinvex-gateway",
        classification: Classification::Chinvex,
        name_eq: None,
        cmdline_contains: None,
        ancestor_name_eq: Some("chinvex.exe"),
    },
];

/// Apply rules to a process + blame chain. Returns (classification, rule_name).
pub fn classify(_info: &ProcessInfo, _ancestor_names: &[String]) -> (Classification, Option<&'static str>) {
    unimplemented!("walk BUILTIN_RULES in order, return first match or (Unknown, None)")
}

/// Dump the active rules as JSON (for `flash-watcher classify-rules`).
pub fn dump_rules(pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(BUILTIN_RULES).expect("rules serialize")
    } else {
        serde_json::to_string(BUILTIN_RULES).expect("rules serialize")
    }
}
