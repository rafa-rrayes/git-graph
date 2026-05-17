use clap::Parser;
use std::collections::HashSet;
use std::env;
use std::io::IsTerminal;
use std::process::{exit, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use terminal_size::{terminal_size, Width};

const HASH_W: usize = 7;
const PM_W: usize = 6;
const AUTHOR_W: usize = 12;
const TIME_W: usize = 4;
const BAR_MAX: usize = 40;
const BAR_MIN: usize = 10;
const SUBJECT_MIN: usize = 15;

#[derive(clap::ValueEnum, Clone, PartialEq, Eq)]
enum SortBy {
    Date,
    Churn,
}

/// Show recent git commits with a visual +/- bar graph.
#[derive(Parser)]
#[command(
    version,
    about,
    long_about = None,
    override_usage = "git-graph [options] [<n>] [<ref>...] [-- <path>...]",
)]
struct Args {
    /// Number of commits (default: 10; bare integer also works).
    #[arg(short = 'n', long, default_value_t = 10)]
    count: usize,

    /// Filter to commits by author (substring match).
    #[arg(long)]
    author: Option<String>,

    /// Only commits after this date (any git date string).
    #[arg(long)]
    since: Option<String>,

    /// Only commits before this date (any git date string).
    #[arg(long)]
    until: Option<String>,

    /// Include merge commits (excluded by default).
    #[arg(long)]
    merges: bool,

    /// Scale bars logarithmically (useful when one commit dwarfs the rest).
    #[arg(long = "log-scale")]
    log_scale: bool,

    /// Sort order.
    #[arg(long, value_enum, default_value_t = SortBy::Date)]
    sort: SortBy,

    /// Disable color output.
    #[arg(long = "no-color")]
    no_color: bool,

    /// Hide the +/- numeric columns.
    #[arg(long = "no-stats")]
    no_stats: bool,

    /// Hide the author column.
    #[arg(long = "no-author")]
    no_author: bool,

    /// Hide the when (relative time) column.
    #[arg(long = "no-when")]
    no_when: bool,

    /// Hide the footer summary lines.
    #[arg(long = "no-summary")]
    no_summary: bool,

    /// Optional refs or revision ranges (e.g. main, v1.0..HEAD).
    refs: Vec<String>,

    /// Paths to limit by (after `--`).
    #[arg(last = true)]
    paths: Vec<String>,
}

fn has_count_flag(args_list: &[String]) -> bool {
    for a in args_list {
        if a == "-n" || a == "--count" {
            return true;
        }
        if a.starts_with("--count=") {
            return true;
        }
        if let Some(rest) = a.strip_prefix("-n") {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

struct Commit {
    hash: String,
    author: String,
    ts: u64,
    date: String,
    subject: String,
    add: u32,
    rem: u32,
}

fn trunc(s: &str, w: usize) -> String {
    if w == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= w {
        return s.to_string();
    }
    let mut out: String = chars[..w - 1].iter().collect();
    out.push('…');
    out
}

fn ljust(s: &str, w: usize) -> String {
    let t = trunc(s, w);
    let len = t.chars().count();
    if len < w {
        format!("{}{}", t, " ".repeat(w - len))
    } else {
        t
    }
}

fn rjust(s: &str, w: usize) -> String {
    let t = trunc(s, w);
    let len = t.chars().count();
    if len < w {
        format!("{}{}", " ".repeat(w - len), t)
    } else {
        t
    }
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

fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

fn relative_time(ts: u64, now: u64) -> String {
    let d = now.saturating_sub(ts);
    if d < 60 {
        format!("{d}s")
    } else if d < 3600 {
        format!("{}m", d / 60)
    } else if d < 86400 {
        format!("{}h", d / 3600)
    } else if d < 86400 * 7 {
        format!("{}d", d / 86400)
    } else if d < 86400 * 30 {
        format!("{}w", d / (86400 * 7))
    } else if d < 86400 * 365 {
        format!("{}mo", d / (86400 * 30))
    } else {
        format!("{}y", d / (86400 * 365))
    }
}

fn main() {
    let raw: Vec<String> = env::args().collect();
    let pre_for_check: &[String] = match raw.iter().skip(1).position(|a| a == "--") {
        Some(i) => &raw[1..(1 + i)],
        None => &raw[raw.len().min(1)..],
    };
    let explicit_count = has_count_flag(pre_for_check);
    let mut args = Args::parse();
    if !explicit_count
        && !args.refs.is_empty()
        && !args.refs[0].is_empty()
        && args.refs[0].chars().all(|c| c.is_ascii_digit())
    {
        if let Ok(n) = args.refs[0].parse::<usize>() {
            args.count = n;
            args.refs.remove(0);
        }
    }

    let use_color = std::io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none()
        && !args.no_color;

    let (green, red, dim, reset, bold, ins_ch, del_ch) = if use_color {
        (
            "\x1b[32m", "\x1b[31m", "\x1b[2m", "\x1b[0m", "\x1b[1m", "█", "█",
        )
    } else {
        ("", "", "", "", "", "+", "-")
    };

    let mut cmd = Command::new("git");
    cmd.args([
        "log",
        "--pretty=format:COMMIT|%h|%an|%at|%ad|%s",
        "--date=format-local:%b %d",
        "--shortstat",
    ]);
    if !args.merges {
        cmd.arg("--no-merges");
    }
    if let Some(a) = &args.author {
        cmd.arg(format!("--author={a}"));
    }
    if let Some(s) = &args.since {
        cmd.arg(format!("--since={s}"));
    }
    if let Some(u) = &args.until {
        cmd.arg(format!("--until={u}"));
    }
    cmd.arg(format!("-{}", args.count));
    for r in &args.refs {
        cmd.arg(r);
    }
    if !args.paths.is_empty() {
        cmd.arg("--");
        for p in &args.paths {
            cmd.arg(p);
        }
    }

    let output = match cmd.output() {
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
            let parts: Vec<&str> = rest.splitn(5, '|').collect();
            if parts.len() < 5 {
                continue;
            }
            let Ok(ts) = parts[2].parse::<u64>() else {
                continue;
            };
            commits.push(Commit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                ts,
                date: parts[3].to_string(),
                subject: parts[4].to_string(),
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

    if args.sort == SortBy::Churn {
        commits.sort_by(|a, b| (b.add + b.rem).cmp(&(a.add + a.rem)));
    }

    let show_stats = !args.no_stats;
    let show_author = !args.no_author;
    let show_when = !args.no_when;
    let show_summary = !args.no_summary;

    let hash_col_w = HASH_W.max(
        commits
            .iter()
            .map(|c| c.hash.chars().count())
            .max()
            .unwrap_or(0),
    );
    let mut meta_w = 0usize;
    if show_author {
        meta_w = AUTHOR_W;
    }
    if show_when {
        if meta_w > 0 {
            meta_w += 1;
        }
        meta_w += TIME_W;
    }

    let mut fixed = hash_col_w;
    if show_stats {
        fixed += 1 + PM_W + 1 + PM_W;
    }
    fixed += 2;
    if meta_w > 0 {
        fixed += 2 + meta_w;
    }
    fixed += 2;

    let term_w = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80);
    let avail = term_w.saturating_sub(fixed).max(BAR_MIN + SUBJECT_MIN);
    let bar_w = avail.saturating_sub(SUBJECT_MIN).clamp(BAR_MIN, BAR_MAX);
    let subject_w = avail.saturating_sub(bar_w).max(SUBJECT_MIN);

    let scale = |v: f64| -> f64 {
        if args.log_scale {
            (v + 1.0).ln()
        } else {
            v
        }
    };
    let max_val: u64 = commits
        .iter()
        .map(|c| (c.add + c.rem) as u64)
        .max()
        .unwrap_or(1)
        .max(1);
    let scaled_max = {
        let s = scale(max_val as f64);
        if s == 0.0 {
            1.0
        } else {
            s
        }
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut header_group1 = format!("{}{}", bold, ljust("hash", hash_col_w));
    if show_stats {
        header_group1.push(' ');
        header_group1.push_str(&ljust("+", PM_W));
        header_group1.push(' ');
        header_group1.push_str(&ljust("-", PM_W));
    }
    let mut header_parts: Vec<String> = vec![header_group1, ljust("graph", bar_w)];
    if meta_w > 0 {
        let mut meta_h: Vec<String> = Vec::new();
        if show_author {
            meta_h.push(ljust("author", AUTHOR_W));
        }
        if show_when {
            meta_h.push(rjust("when", TIME_W));
        }
        header_parts.push(meta_h.join(" "));
    }
    header_parts.push(format!("subject{}", reset));
    println!("{}", header_parts.join("  "));
    println!("{}", "─".repeat(term_w));

    for c in &commits {
        let total = c.add + c.rem;
        let bar = if total == 0 {
            " ".repeat(bar_w)
        } else {
            let mut bar_total =
                (scale(total as f64) / scaled_max * bar_w as f64).round_ties_even() as usize;
            bar_total = bar_total.clamp(1, bar_w);
            let add_w = ((bar_total as f64 * c.add as f64 / total as f64).round_ties_even()
                as usize)
                .min(bar_total);
            let rem_w = bar_total - add_w;
            let pad_w = bar_w - bar_total;
            format!(
                "{green}{}{reset}{red}{}{reset}{}",
                ins_ch.repeat(add_w),
                del_ch.repeat(rem_w),
                " ".repeat(pad_w),
                green = green,
                red = red,
                reset = reset,
            )
        };

        let mut row_group1 = format!("{}{}{}", dim, ljust(&c.hash, hash_col_w), reset);
        if show_stats {
            row_group1.push_str(&format!(
                " {green}+{add:<5}{reset} {red}-{rem:<5}{reset}",
                green = green,
                reset = reset,
                red = red,
                add = c.add,
                rem = c.rem,
            ));
        }
        let mut row_parts: Vec<String> = vec![row_group1, bar];
        if meta_w > 0 {
            let mut meta_d: Vec<String> = Vec::new();
            if show_author {
                meta_d.push(ljust(&c.author, AUTHOR_W));
            }
            if show_when {
                meta_d.push(rjust(&relative_time(c.ts, now), TIME_W));
            }
            row_parts.push(meta_d.join(" "));
        }
        row_parts.push(trunc(&c.subject, subject_w));
        println!("{}", row_parts.join("  "));
    }

    if !show_summary {
        return;
    }

    let total_add: u64 = commits.iter().map(|c| c.add as u64).sum();
    let total_rem: u64 = commits.iter().map(|c| c.rem as u64).sum();
    let authors: HashSet<&str> = commits.iter().map(|c| c.author.as_str()).collect();
    let oldest = commits.iter().min_by_key(|c| c.ts).unwrap();
    let newest = commits.iter().max_by_key(|c| c.ts).unwrap();
    let span = if newest.ts.saturating_sub(oldest.ts) < 86400 {
        newest.date.clone()
    } else {
        let days = (newest.ts - oldest.ts) / 86400;
        format!("{} – {} ({}d)", oldest.date, newest.date, days)
    };

    println!();
    let scale_note = if args.log_scale { " [log scale]" } else { "" };
    let author_word = if authors.len() == 1 { "author" } else { "authors" };
    let commit_word = if commits.len() == 1 { "commit" } else { "commits" };
    println!(
        "{dim}Bar scaled to max total ({} lines in one commit){scale_note}. {} {commit_word} shown.{reset}",
        format_thousands(max_val),
        commits.len(),
        dim = dim, reset = reset,
    );
    println!(
        "{dim}+{} / -{} across {} {author_word}, {span}.{reset}",
        format_thousands(total_add),
        format_thousands(total_rem),
        authors.len(),
        dim = dim, reset = reset,
    );
}
