use clap::Parser;
use owo_colors::OwoColorize;
use std::process::{exit, Command};

const BAR_WIDTH: usize = 40;
const SUBJECT_WIDTH: usize = 50;

/// Show recent git commits with a visual +/- bar graph.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of commits to show.
    #[arg(short = 'n', long, default_value_t = 3)]
    count: usize,
}

struct Commit {
    hash: String,
    subject: String,
    add: u32,
    rem: u32,
}

fn trunc(s: &str, w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= w {
        return s.to_string();
    }
    let mut out: String = chars[..w - 1].iter().collect();
    out.push('…');
    out
}

fn parse_shortstat(line: &str) -> (u32, u32) {
    let mut add = 0u32;
    let mut rem = 0u32;
    for part in line.split(',') {
        let part = part.trim();
        let Some(num_str) = part.split_whitespace().next() else {
            continue;
        };
        let Ok(n) = num_str.parse::<u32>() else {
            continue;
        };
        if part.contains("insertion") {
            add = n;
        } else if part.contains("deletion") {
            rem = n;
        }
    }
    (add, rem)
}

fn format_thousands(n: u32) -> String {
    let s = n.to_string();
    let bytes: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

fn main() {
    let args = Args::parse();

    let output = match Command::new("git")
        .args([
            "log",
            "--pretty=format:COMMIT|%h|%s",
            "--shortstat",
            "--no-merges",
        ])
        .arg(format!("-{}", args.count))
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("failed to run git: {e}");
            exit(127);
        }
    };

    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        exit(output.status.code().unwrap_or(1));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits: Vec<Commit> = Vec::new();

    for line in stdout.split('\n') {
        if let Some(rest) = line.strip_prefix("COMMIT|") {
            let mut parts = rest.splitn(2, '|');
            let hash = parts.next().unwrap_or("").to_string();
            let subject = parts.next().unwrap_or("").to_string();
            commits.push(Commit {
                hash,
                subject,
                add: 0,
                rem: 0,
            });
        } else if line.contains("insertion") || line.contains("deletion") {
            if let Some(c) = commits.last_mut() {
                let (a, r) = parse_shortstat(line);
                c.add = a;
                c.rem = r;
            }
        }
    }

    if commits.is_empty() {
        println!("No commits found.");
        return;
    }

    let max_val = commits
        .iter()
        .map(|c| c.add + c.rem)
        .max()
        .unwrap_or(1)
        .max(1);

    let header = format!(
        "{:<8} {:>6} {:>6}  {:<41}  subject",
        "hash", "   +", "   -", "graph"
    );
    println!("{}", header.bold());
    println!("{}", "─".repeat(110));

    for c in &commits {
        let total = c.add + c.rem;
        let bar = if total == 0 {
            " ".repeat(BAR_WIDTH)
        } else {
            let add_w =
                ((c.add as f64 / max_val as f64) * BAR_WIDTH as f64).round() as usize;
            let rem_w =
                ((c.rem as f64 / max_val as f64) * BAR_WIDTH as f64).round() as usize;
            let pad = BAR_WIDTH.saturating_sub(add_w + rem_w);
            format!(
                "{}{}{}",
                "█".repeat(add_w).green(),
                "█".repeat(rem_w).red(),
                " ".repeat(pad)
            )
        };

        println!(
            "{} {} {}  {}  {}",
            c.hash.dimmed(),
            format!("+{:<5}", c.add).green(),
            format!("-{:<5}", c.rem).red(),
            bar,
            trunc(&c.subject, SUBJECT_WIDTH)
        );
    }

    println!();
    println!(
        "{}",
        format!(
            "Bar scaled to max total ({} lines in one commit). {} commits shown.",
            format_thousands(max_val),
            commits.len()
        )
        .dimmed()
    );
}
