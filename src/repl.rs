//! # REPL Module
//!
//! Provides an interactive Read-Eval-Print Loop for Xenith with advanced features.

use crate::run;
use crate::utils::value_to_string;
use crate::values::Value;
use colored::*;
use rustyline::completion::{Completer, Pair};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator};
use rustyline::{Cmd, Context, Editor, EventHandler, KeyCode, KeyEvent, Modifiers};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};

// Keywords for completion
const KEYWORDS: &[&str] = &[
    "let",
    "const",
    "method",
    "release",
    "return",
    "when",
    "or",
    "otherwise",
    "for",
    "to",
    "step",
    "while",
    "skip",
    "stop",
    "match",
    "in",
    "try",
    "catch",
    "panic",
    "grab",
    "export",
    "as",
    "from",
    "struct",
    "impl",
    "type",
    "true",
    "false",
    "null",
    "format",
    "echo",
    "input",
    "len",
];

// Built-in types
const TYPES: &[&str] = &[
    "int", "float", "string", "bool", "list", "map", "json", "null",
];

// Built-in functions
const BUILTINS: &[&str] = &[
    "echo",
    "ret",
    "input",
    "input_int",
    "clear",
    "is_num",
    "is_str",
    "is_list",
    "is_fun",
    "append",
    "pop",
    "extend",
    "len",
    "run",
    "format",
];

// Command completions
const COMMANDS: &[&str] = &[":help", ":exit", ":quit", ":clear", ":vars", ":load"];

/// REPL helper that provides completion, validation, and highlighting
struct ReplHelper {
    completer: ReplCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl ReplHelper {
    fn new(colored_prompt: String) -> Self {
        Self {
            completer: ReplCompleter,
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
            colored_prompt,
        }
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Cow::Owned(self.colored_prompt.clone())
        } else {
            Cow::Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(hint.truecolor(100, 100, 100).to_string())
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        // Simple syntax highlighting
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();

        while i < chars.len() {
            // Keywords and types
            let rest: String = chars[i..].iter().collect();
            let mut matched = false;

            for kw in KEYWORDS {
                if rest.starts_with(kw)
                    && (i + kw.len() >= chars.len() || !is_identifier_char(chars[i + kw.len()]))
                {
                    result.push_str(&kw.bright_cyan().to_string());
                    i += kw.len();
                    matched = true;
                    break;
                }
            }

            if !matched {
                for typ in TYPES {
                    if rest.starts_with(typ)
                        && (i + typ.len() >= chars.len()
                            || !is_identifier_char(chars[i + typ.len()]))
                    {
                        result.push_str(&typ.bright_yellow().to_string());
                        i += typ.len();
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                for builtin in BUILTINS {
                    if rest.starts_with(builtin)
                        && (i + builtin.len() >= chars.len()
                            || !is_identifier_char(chars[i + builtin.len()]))
                    {
                        result.push_str(&builtin.bright_green().to_string());
                        i += builtin.len();
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // Strings
                if chars[i] == '"' {
                    result.push('"');
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' {
                            result.push('\\');
                            i += 1;
                            if i < chars.len() {
                                result.push(chars[i]);
                                i += 1;
                            }
                        } else {
                            result.push(chars[i]);
                            i += 1;
                        }
                    }
                    if i < chars.len() && chars[i] == '"' {
                        result.push('"');
                        i += 1;
                    }
                } else if chars[i] == '`' {
                    result.push('`');
                    i += 1;
                    while i < chars.len() && chars[i] != '`' {
                        result.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '`' {
                        result.push('`');
                        i += 1;
                    }
                } else if chars[i].is_ascii_digit() {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    result.push_str(&line[start..i].bright_magenta().to_string());
                } else {
                    result.push(chars[i]);
                    i += 1;
                }
            }
        }

        Cow::Owned(result)
    }

    fn highlight_char(&self, line: &str, pos: usize, _: bool) -> bool {
        let ch = line.chars().nth(pos).unwrap_or('\0');
        matches!(ch, '(' | ')' | '[' | ']' | '{' | '}')
    }
}

impl Validator for ReplHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult, ReadlineError> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        true
    }
}

impl rustyline::Helper for ReplHelper {}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

struct ReplCompleter;

impl Completer for ReplCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let line = &line[..pos];

        if line.starts_with(':') {
            let candidates: Vec<Pair> = COMMANDS
                .iter()
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            return Ok((0, candidates));
        }

        let last_word = line.split_whitespace().last().unwrap_or("");
        let start = line.rfind(last_word).unwrap_or(0);

        let mut candidates = Vec::new();

        for kw in KEYWORDS {
            if kw.starts_with(last_word) {
                candidates.push(Pair {
                    display: kw.to_string(),
                    replacement: kw.to_string(),
                });
            }
        }

        for typ in TYPES {
            if typ.starts_with(last_word) {
                candidates.push(Pair {
                    display: typ.to_string(),
                    replacement: typ.to_string(),
                });
            }
        }

        for builtin in BUILTINS {
            if builtin.starts_with(last_word) {
                candidates.push(Pair {
                    display: builtin.to_string(),
                    replacement: builtin.to_string(),
                });
            }
        }

        Ok((start, candidates))
    }
}

impl Hinter for ReplCompleter {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

/// Main REPL loop
pub fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let history_file = dirs::home_dir()
        .unwrap_or_else(|| ".".into())
        .join(".xenith_history");

    let mut rl = Editor::<ReplHelper, rustyline::history::DefaultHistory>::new()?;

    rl.set_auto_add_history(true);
    // rl.set_keymap(rustyline::config::EditMode::Emacs);

    if history_file.exists() {
        let _ = rl.load_history(&history_file);
    }

    rl.bind_sequence(
        KeyEvent(KeyCode::Char('l'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::ClearScreen),
    );

    let prompt = format!("{} ", "xenith>".bright_cyan().bold());

    let helper = ReplHelper::new(prompt.clone());
    rl.set_helper(Some(helper));

    let title = "Xenith Interactive Shell"; // 24 chars
    let box_width = 58; // inner width between ║ and ║
    let padding = (box_width - title.len()) / 2;
    let left = " ".repeat(padding);
    let right = " ".repeat(box_width - title.len() - padding);

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".bright_blue()
    );
    println!(
        "{}{}{}{}{}",
        "║".bright_blue(),
        left,
        title.bright_cyan().bold(),
        right,
        "║".bright_blue()
    );
    println!(
        "{}",
        "╠══════════════════════════════════════════════════════════╣".bright_blue()
    );
    println!(
        "{}",
        "║  • Type :help for available commands                     ║".bright_blue()
    );
    println!(
        "{}",
        "║  • Type :exit or press Ctrl+D/Ctrl+C to quit             ║".bright_blue()
    );
    println!(
        "{}",
        "║  • Use Ctrl+L to clear screen                            ║".bright_blue()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".bright_blue()
    );
    println!();

    let mut variables: HashMap<String, String> = HashMap::new();

    loop {
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line.starts_with(':') {
                    handle_command(line, &mut rl, &history_file, &mut variables)?;
                    continue;
                }

                let needs_more = needs_more_input(line);
                let mut full_line = line.to_string();

                if needs_more {
                    let continuation_prompt = "    ...> ".truecolor(100, 100, 100).to_string();
                    loop {
                        let next_line = rl.readline(&continuation_prompt);
                        match next_line {
                            Ok(next) => {
                                let next = next.trim();
                                if next.is_empty() {
                                    continue;
                                }
                                full_line.push_str("\n");
                                full_line.push_str(next);
                                if !needs_more_input(&full_line) {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }

                // Execute the code
                match run("<repl>", &full_line) {
                    Ok(value) => {
                        // Don't print null/0 values (they're usually from echo or statements)
                        match &value {
                            Value::Number(n) if n.value == 0.0 => {
                                // Skip printing 0 (null)
                            }
                            _ => {
                                let output = value_to_string(&value);
                                if !output.is_empty() && output != "null" && output != "0" {
                                    println!("{}", output.bright_green());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e.as_string().bright_red());
                    }
                }

                let _ = rl.add_history_entry(line);
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("\n{}", "Goodbye!".bright_yellow());
                break;
            }
            Err(err) => {
                eprintln!("{}: {}", "Error".bright_red(), err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_file);
    Ok(())
}

fn handle_command(
    cmd: &str,
    rl: &mut Editor<ReplHelper, rustyline::history::DefaultHistory>,
    _history_file: &std::path::PathBuf,
    variables: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    match parts[0] {
        ":help" | ":h" => {
            println!("{}", "\nAvailable Commands:".bright_cyan().bold());
            println!("  {}     - Show this help message", ":help".bright_yellow());
            println!("  {}     - Exit the REPL", ":exit".bright_yellow());
            println!("  {}     - Also exit", ":quit".bright_yellow());
            println!(
                "  {}     - Clear screen (or Ctrl+L)",
                ":clear".bright_yellow()
            );
            println!(
                "  {}     - Show all variables in current scope",
                ":vars".bright_yellow()
            );
            println!(
                "  {}    - Load and execute a Xenith file",
                ":load <filename>".bright_yellow()
            );
            println!();
            println!("{}", "Keyboard Shortcuts:".bright_cyan().bold());
            println!("  ↑/↓      - Navigate command history");
            println!("  Ctrl+L   - Clear screen");
            println!("  Ctrl+D   - Exit (on empty line)");
            println!("  Ctrl+C   - Exit");
            println!("  Tab      - Auto-completion");
            println!("  Home/End - Move to beginning/end of line");
            println!();
        }
        ":exit" | ":quit" | ":q" => {
            println!("{}", "Goodbye!".bright_yellow());
            std::process::exit(0);
        }
        ":clear" | ":cls" => {
            print!("\x1B[2J\x1B[1;1H");
            let _ = io::stdout().flush();
        }
        ":vars" | ":variables" => {
            if variables.is_empty() {
                println!(
                    "{}",
                    "No variables in current scope".truecolor(100, 100, 100)
                );
            } else {
                println!("{}", "\nVariables:".bright_cyan().bold());
                for (name, value) in variables {
                    println!("  {} = {}", name.bright_yellow(), value);
                }
                println!();
            }
        }
        ":load" => {
            if parts.len() < 2 {
                println!("{}", "Usage: :load <filename.xen>".bright_red());
            } else {
                let filename = parts[1];
                match std::fs::read_to_string(filename) {
                    Ok(source) => match run(filename, &source) {
                        Ok(value) => {
                            let output = value_to_string(&value);
                            if !output.is_empty() && output != "null" {
                                println!("{}", output.bright_green());
                            }
                            println!(
                                "{}",
                                format!("Loaded successfully: {}", filename).bright_green()
                            );
                        }
                        Err(e) => eprintln!("{}", e.as_string().bright_red()),
                    },
                    Err(e) => println!("{}", format!("Failed to load file: {}", e).bright_red()),
                }
            }
        }
        _ => {
            println!(
                "{}",
                format!(
                    "Unknown command: {}. Type :help for available commands.",
                    cmd
                )
                .bright_red()
            );
        }
    }

    Ok(())
}

fn needs_more_input(line: &str) -> bool {
    let mut brace_count = 0;
    let mut paren_count = 0;
    let mut bracket_count = 0;
    let mut in_string = false;
    let mut in_backtick = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape {
            escape = false;
            continue;
        }

        match ch {
            '"' if !in_backtick => in_string = !in_string,
            '`' if !in_string => in_backtick = !in_backtick,
            '\\' if in_string || in_backtick => escape = true,
            '{' if !in_string && !in_backtick => brace_count += 1,
            '}' if !in_string && !in_backtick => brace_count -= 1,
            '(' if !in_string && !in_backtick => paren_count += 1,
            ')' if !in_string && !in_backtick => paren_count -= 1,
            '[' if !in_string && !in_backtick => bracket_count += 1,
            ']' if !in_string && !in_backtick => bracket_count -= 1,
            _ => {}
        }
    }

    brace_count > 0 || paren_count > 0 || bracket_count > 0 || in_string || in_backtick
}
