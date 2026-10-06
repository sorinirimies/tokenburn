//! Client-side row filtering: free text and a tool subset.
//!
//! Applied to rows that are already loaded, so changing it is instant (no log
//! re-read). Used by the TUI's `/` and `f` filters; reusable by any front-end.

use std::collections::BTreeSet;

use crate::features::usage::{Row, Tool};

/// What to keep.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowFilter {
    /// Space-separated terms, **all** must match (AND), case-insensitively,
    /// against the tool (`claude` / `Claude Code`), the project or model and the
    /// session id. A term starting with `-` excludes rows that match it:
    /// `tokenburn -zed` = rows mentioning *tokenburn* but not *zed*.
    pub text: String,
    /// Keep only these tools. `None` keeps every tool.
    pub tools: Option<BTreeSet<Tool>>,
}

impl RowFilter {
    /// `true` when the filter keeps everything.
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.tools.is_none()
    }

    /// Does `row` pass?
    pub fn matches(&self, row: &Row) -> bool {
        if let Some(tools) = &self.tools {
            if !tools.contains(&row.tool) {
                return false;
            }
        }
        let text = self.text.trim();
        if text.is_empty() {
            return true;
        }
        let hay = haystack(row);
        text.split_whitespace()
            .all(|term| match term.strip_prefix('-') {
                // A lone "-" (still being typed) excludes nothing.
                Some("") => true,
                Some(neg) => !hay.contains(&neg.to_lowercase()),
                None => hay.contains(&term.to_lowercase()),
            })
    }

    /// The rows that pass, cloned.
    pub fn apply(&self, rows: &[Row]) -> Vec<Row> {
        if self.is_empty() {
            return rows.to_vec();
        }
        rows.iter().filter(|r| self.matches(r)).cloned().collect()
    }

    /// Is `tool` currently kept?
    pub fn allows(&self, tool: Tool) -> bool {
        self.tools.as_ref().is_none_or(|t| t.contains(&tool))
    }

    /// Flip one tool. Re-selecting every tool collapses back to `None`.
    pub fn toggle_tool(&mut self, tool: Tool) {
        let mut set: BTreeSet<Tool> = self
            .tools
            .take()
            .unwrap_or_else(|| Tool::ALL.into_iter().collect());
        if !set.remove(&tool) {
            set.insert(tool);
        }
        self.tools = (set.len() < Tool::ALL.len()).then_some(set);
    }

    /// Keep every tool.
    pub fn select_all_tools(&mut self) {
        self.tools = None;
    }

    /// Keep no tool at all (everything is hidden until one is re-enabled).
    pub fn select_no_tools(&mut self) {
        self.tools = Some(BTreeSet::new());
    }

    /// How many tools are kept.
    pub fn tool_count(&self) -> usize {
        self.tools.as_ref().map_or(Tool::ALL.len(), BTreeSet::len)
    }
}

fn haystack(r: &Row) -> String {
    format!(
        "{} {} {} {}",
        r.tool.as_str(),
        r.tool.display_name(),
        r.project,
        r.id
    )
    .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(tool: Tool, project: &str) -> Row {
        Row {
            tool,
            project: project.into(),
            id: "sess-42".into(),
            ts: "2026-05-01T12:00:00Z".parse().unwrap(),
            input: 1,
            output: 1,
            cache_read: 0,
            cache_write: 0,
            cost: 0.0,
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            row(Tool::Pi, "Users-me-tokenburn"),
            row(Tool::Pi, "Users-me-other"),
            row(Tool::Claude, "Users-me-tokenburn"),
            row(Tool::Zed, "claude-sonnet"),
        ]
    }

    fn text(t: &str) -> RowFilter {
        RowFilter {
            text: t.into(),
            tools: None,
        }
    }

    #[test]
    fn empty_filter_keeps_everything() {
        let f = RowFilter::default();
        assert!(f.is_empty());
        assert_eq!(f.apply(&rows()).len(), 4);
        assert!(text("   ").is_empty(), "whitespace-only text is empty");
    }

    #[test]
    fn text_matches_project_tool_and_display_name_case_insensitively() {
        assert_eq!(text("TOKENBURN").apply(&rows()).len(), 2);
        assert_eq!(text("pi").apply(&rows()).len(), 2);
        // "Claude Code" is the display name of the Claude tool; the Zed row only has "claude".
        assert_eq!(text("claude code").apply(&rows()).len(), 1);
        assert_eq!(
            text("sonnet").apply(&rows()).len(),
            1,
            "model names (the project of Zed rows) match"
        );
        assert_eq!(text("sess-42").apply(&rows()).len(), 4, "session ids match");
    }

    #[test]
    fn all_terms_must_match() {
        let r = text("tokenburn claude").apply(&rows());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tool, Tool::Claude);
    }

    #[test]
    fn a_leading_dash_excludes() {
        let r = text("tokenburn -claude").apply(&rows());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tool, Tool::Pi);
        assert_eq!(text("-pi").apply(&rows()).len(), 2);
    }

    #[test]
    fn a_lone_dash_excludes_nothing_while_it_is_being_typed() {
        assert_eq!(text("-").apply(&rows()).len(), 4);
        assert_eq!(text("tokenburn -").apply(&rows()).len(), 2);
    }

    #[test]
    fn tool_subset_filters_by_tool() {
        let mut f = RowFilter::default();
        f.toggle_tool(Tool::Pi); // starts from "all", so this removes pi
        assert!(!f.allows(Tool::Pi) && f.allows(Tool::Claude));
        assert_eq!(f.apply(&rows()).len(), 2);
        assert_eq!(f.tool_count(), Tool::ALL.len() - 1);
        f.toggle_tool(Tool::Pi); // re-select every tool → back to "no filter"
        assert!(f.tools.is_none() && f.is_empty());
    }

    #[test]
    fn none_and_all_helpers() {
        let mut f = RowFilter::default();
        f.select_no_tools();
        assert_eq!(f.apply(&rows()).len(), 0);
        assert_eq!(f.tool_count(), 0);
        f.toggle_tool(Tool::Zed);
        assert_eq!(f.apply(&rows()).len(), 1);
        f.select_all_tools();
        assert!(f.is_empty());
    }

    #[test]
    fn text_and_tools_combine() {
        let mut f = text("tokenburn");
        f.select_no_tools();
        f.toggle_tool(Tool::Claude);
        let r = f.apply(&rows());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tool, Tool::Claude);
    }
}
