//! skill の本文と CLI のずれの検査: 本文のコードに出てくる `fune …` のコマンドと `--flag` が clap の定義に実在する。
//! コマンドを改名・削除したのに本文を直し忘れると落ちる。

use crate::args::Cli;
use clap::{Command, CommandFactory};

/// 本文の中の `fune …` の行（コードブロックの行と、行内の `…` の中身）。
fn commands_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
        } else if in_block {
            let code = line.split(" #").next().unwrap_or(line).trim();
            if code.starts_with("fune ") {
                found.push(code.to_owned());
            }
        } else {
            // 行内: 奇数番目の「`」で囲まれた部分
            found.extend(
                line.split('`')
                    .skip(1)
                    .step_by(2)
                    .filter(|span| span.starts_with("fune "))
                    .map(str::to_owned),
            );
        }
    }
    found
}

/// 引用符の中を除いた語。
fn words_of(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c if !quoted => current.push(c),
            _ => {}
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// 1 行の検査。`fune` のあとのコマンドの道筋が実在し、`--flag` がそのコマンド（か全体の引数）にある。問題の説明を返す。
fn problem_in(root: &Command, line: &str) -> Option<String> {
    let words = words_of(line);
    let mut cmd = root;
    let mut flags = Vec::new();
    let mut path_open = true;
    for word in &words[1..] {
        if let Some(flag) = word.strip_prefix("--") {
            flags.push(flag.split('=').next().unwrap_or(flag).to_owned());
        } else if word.starts_with('-') || word.starts_with('<') || !path_open {
            continue;
        } else if let Some(sub) = cmd.find_subcommand(word) {
            cmd = sub;
        } else if cmd.has_subcommands() && word != "help" {
            return Some(format!(
                "`{line}`: `{word}` is not a subcommand of `{}`",
                cmd.get_name()
            ));
        } else {
            path_open = false;
        }
    }
    flags.into_iter().find_map(|f| {
        let known = f == "help"
            || cmd
                .get_arguments()
                .any(|a| a.get_long() == Some(f.as_str()));
        (!known).then(|| format!("`{line}`: `{}` has no flag --{f}", cmd.get_name()))
    })
}

fn cli() -> Command {
    let mut cmd = Cli::command();
    cmd.build();
    cmd
}

#[test]
fn every_command_in_the_skill_text_exists() {
    let root = cli();
    let lines = commands_in(&fune_service::skill::render());
    assert!(lines.len() > 20, "found too few commands: {lines:?}");
    let problems: Vec<_> = lines.iter().filter_map(|l| problem_in(&root, l)).collect();
    assert!(
        problems.is_empty(),
        "the skill text has drifted from the CLI:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_check_notices_renamed_commands_and_flags() {
    let root = cli();
    assert!(problem_in(&root, "fune part ls").is_none());
    assert!(problem_in(&root, "fune analyze power.energy_balance --case x --json").is_none());
    assert!(problem_in(&root, "fune <command> --help").is_none());
    assert!(problem_in(&root, "fune part list").is_some());
    assert!(problem_in(&root, "fune partz ls").is_some());
    assert!(problem_in(&root, "fune part add X --catalogue y").is_some());
    assert!(problem_in(&root, "fune requirement add R --budget mass").is_none());
}

#[test]
fn extraction_reads_blocks_and_inline_code_only() {
    let text = "Run `fune check` now; fune fmt is not code.\n```\nfune status # note\nls\n```\n";
    assert_eq!(commands_in(text), ["fune check", "fune status"]);
}
