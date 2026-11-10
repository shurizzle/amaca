use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::ticket;

/// Resolves the head branch of a GitHub pull request with Claude.
pub fn branch(pr: &str) -> Result<String> {
    let prompt = format!(
        "You resolve the head branch of a GitHub pull request.\n\
         Input: a GitHub pull request link or number.\n\n\
         Steps:\n\
         1. Find the pull request with the GitHub MCP tools.\n\
         2. Read its head branch name.\n\n\
         Answer with only the branch name, no markdown and no prose.\n\
         If the input is not a GitHub pull request, answer exactly NONE.\n\n\
         Input:\n{pr}"
    );
    let answer = ask(&[], &prompt)?;
    let branch = answer
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .trim_matches(['`', '"', '\'']);
    if branch.is_empty() || branch == "NONE" {
        bail!("no GitHub pull request found for {pr:?}");
    }
    Ok(branch.to_owned())
}

/// Loads the whole pull request conversation into the run's session, so the
/// review prompt finds it in context.
pub fn load_conversation(pr: &str, session_id: &str) -> Result<()> {
    let prompt = format!(
        "You load a GitHub pull request into this session's context.\n\
         Input: a GitHub pull request link or number.\n\n\
         Steps:\n\
         1. Read the pull request with the GitHub MCP tools: description, commits, \
            changed files, reviews, review comments and issue comments.\n\
         2. Do not change anything.\n\n\
         Answer exactly OK.\n\n\
         Input:\n{pr}"
    );
    ask(&["--resume", session_id], &prompt)?;
    Ok(())
}

/// Extracts the Jira ticket key from a branch name such as
/// `AA-1234-ticket-title`. Tokens are split on every non-alphanumeric
/// character, so `feature/AA-1234-title` works too.
pub fn ticket_key(branch: &str) -> Option<String> {
    let tokens: Vec<&str> = branch
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
    tokens.windows(2).find_map(|pair| {
        let key = format!("{}-{}", pair[0], pair[1]);
        ticket::is_key(&key).then_some(key)
    })
}

fn ask(extra: &[&str], prompt: &str) -> Result<String> {
    let allowed = ticket::allowed_tools(None)?;
    let output = Command::new("claude")
        .args(["--print", "--output-format", "text", &allowed])
        .args(extra)
        .arg(prompt)
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

#[cfg(test)]
mod tests {
    use super::ticket_key;

    #[test]
    fn extracts_the_ticket_from_a_branch() {
        assert_eq!(ticket_key("AA-1234").as_deref(), Some("AA-1234"));
        assert_eq!(
            ticket_key("AA-1234-ticket-title").as_deref(),
            Some("AA-1234")
        );
        assert_eq!(
            ticket_key("feature/AA-1234-title").as_deref(),
            Some("AA-1234")
        );
        assert_eq!(ticket_key("ABC2-12").as_deref(), Some("ABC2-12"));
    }

    #[test]
    fn rejects_branches_without_a_key() {
        assert_eq!(ticket_key("main"), None);
        assert_eq!(ticket_key("feature/abc-12-title"), None);
    }
}
