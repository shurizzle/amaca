use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

#[derive(Clone, Copy)]
pub enum Source {
    Argument,
    Branch,
}

impl Source {
    fn describe(self, input: &str) -> String {
        match self {
            Source::Argument => format!("argument {input:?}"),
            Source::Branch => format!("branch {input:?}"),
        }
    }
}

/// The ticket fields Amaca keeps. Claude also returns `body`, but it stays in
/// the session's context and is never parsed here.
#[derive(Deserialize)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub summary: String,
}

pub fn fetch(input: &str, source: Source, session_id: &str) -> Result<Ticket> {
    let answer = ask_claude(input, session_id)?;
    let answer = answer.trim();
    if answer == "NONE" {
        bail!("no Jira ticket found for {}", source.describe(input));
    }

    let ticket = parse(answer).ok_or_else(|| {
        eprintln!("Claude answer:\n{answer}");
        anyhow!(
            "Claude did not return a valid ticket JSON for {}",
            source.describe(input)
        )
    })?;
    if !is_key(&ticket.id) {
        bail!("Claude returned an invalid ticket id {:?}", ticket.id);
    }
    Ok(ticket)
}

fn ask_claude(input: &str, session_id: &str) -> Result<String> {
    let prompt = format!(
        "You prepare a Jira ticket for a coding session.\n\
         Input: a Jira ticket link, key, or branch name.\n\n\
         Steps:\n\
         1. Determine the Jira ticket key from the input.\n\
         2. Fetch that ticket from Jira with the Atlassian MCP tools.\n\
         3. body: if the ticket is a subtask of any type, the parent ticket content \
            followed by the subtask content; otherwise the ticket content.\n\
         4. summary: one line of at most 10 words that tells what the title does \
            not. Take from the complete body the purpose, the affected area, the \
            expected outcome or the constraints. Never repeat or paraphrase the \
            title. If the body has nothing beyond the title, say what is missing \
            instead.\n\n\
         Answer with only this JSON object, no markdown and no prose:\n\
         {{\"id\":\"PROJECT-123\",\"title\":\"...\",\"summary\":\"...\",\"body\":\"...\"}}\n\n\
         If the input contains no Jira ticket, answer exactly NONE.\n\n\
         Input:\n{input}"
    );

    let allowed = allowed_tools(Some("getJiraIssue"))?;
    let output = Command::new("claude")
        .args([
            "--print",
            "--output-format",
            "text",
            &allowed,
            "--session-id",
            session_id,
            &prompt,
        ])
        .output()
        .context("cannot run claude")?;
    if !output.status.success() {
        bail!(
            "claude failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// Builds the `--allowedTools` argument from the MCP servers Claude Code
/// reports, so no server name is hardcoded. With `tool`, each server allows
/// only that tool; without it, each server allows all of its tools.
pub(crate) fn allowed_tools(tool: Option<&str>) -> Result<String> {
    let servers = Command::new("claude")
        .args(["mcp", "list"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| server_names(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default();
    if servers.is_empty() {
        bail!("cannot list the MCP servers with `claude mcp list`");
    }
    let rules = servers.into_iter().map(|server| match tool {
        Some(tool) => format!("mcp__{server}__{tool}"),
        None => format!("mcp__{server}"),
    });
    Ok(format!(
        "--allowedTools={}",
        rules.collect::<Vec<_>>().join(",")
    ))
}

fn server_names(list: &str) -> Vec<String> {
    list.lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .filter(|line| !line.contains("Disabled"))
        .filter_map(|line| line.split_once(':'))
        .map(|(name, _)| name.trim())
        .filter(|name| {
            name.starts_with(|c: char| c.is_ascii_alphanumeric())
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .map(str::to_owned)
        .collect()
}

fn parse(answer: &str) -> Option<Ticket> {
    let json = &answer[answer.find('{')?..];
    serde_json::Deserializer::from_str(json)
        .into_iter::<Ticket>()
        .next()?
        .ok()
}

pub(crate) fn is_key(candidate: &str) -> bool {
    let Some((project, number)) = candidate.rsplit_once('-') else {
        return false;
    };
    project.starts_with(|c: char| c.is_ascii_uppercase())
        && project
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{is_key, parse, server_names};

    #[test]
    fn reads_server_names_from_list() {
        let list = "Checking MCP server health…\n\n\
                    atlassian-team: https://mcp.atlassian.com/v1/mcp (HTTP) - ✔ Connected\n\
                    figma: https://mcp.figma.com/mcp (HTTP) - ✔ Connected\n\
                    other: true  - ⊘ Disabled for this project\n\
                    claude.ai Docs: https://api.anthropic.com/v1/pages/mcp - ✔ Connected\n\
                    ├ conflict text: ignored\n";
        assert_eq!(server_names(list), ["atlassian-team", "figma"]);
    }

    #[test]
    fn accepts_ticket_keys() {
        assert!(is_key("ABC-1"));
        assert!(is_key("ABC2-123"));
    }

    #[test]
    fn rejects_other_input() {
        assert!(!is_key("abc-1"));
        assert!(!is_key("ABC"));
        assert!(!is_key("ABC-"));
        assert!(!is_key("feature/ABC-1"));
        assert!(!is_key("https://example.atlassian.net/browse/ABC-1"));
    }

    #[test]
    fn parses_json_with_surrounding_text() {
        let answer =
            "```json\n{\"id\":\"ABC-1\",\"title\":\"T\",\"summary\":\"S\",\"body\":\"B\"}\n```";
        let ticket = parse(answer).expect("valid ticket");
        assert_eq!(ticket.id, "ABC-1");
        assert_eq!(ticket.title, "T");
        assert_eq!(ticket.summary, "S");
    }

    #[test]
    fn rejects_non_json() {
        assert!(parse("NONE").is_none());
    }
}
