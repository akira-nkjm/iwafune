//! `fune skill …` の引数。

use clap::Subcommand;
use std::path::PathBuf;

/// `fune skill …`
#[derive(Debug, Subcommand)]
pub enum SkillCmd {
    /// Print the agent skill text (SKILL.md) that this fune ships
    Show,
    /// Install the agent skill (<skills dir>/fune/SKILL.md)
    ///
    /// Default: <project root>/.claude/skills/fune/SKILL.md (the current directory if not in a project; -C
    /// points at the project). An existing file is left alone if it is identical, replaced if another fune
    /// version wrote it, and refused if it was edited (use --force to overwrite).
    Install {
        /// Skills directory to install into instead of <project root>/.claude/skills (for other agents' locations)
        #[arg(long, value_name = "PATH")]
        skills_dir: Option<PathBuf>,
        /// Overwrite the file even if it was edited by hand
        #[arg(long)]
        force: bool,
    },
}
