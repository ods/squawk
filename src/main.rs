mod ast;
mod error;
mod github;
mod parse;
mod reporter;
mod rules;
mod subcommand;
#[macro_use]
extern crate lazy_static;
use crate::reporter::{
    check_files, dump_ast_for_paths, explain_rule, list_rules, print_violations, DumpAstOption,
    Reporter,
};
use crate::subcommand::{check_and_comment_on_pr, Command};
use atty::Stream;
use std::io;
use std::process;
use structopt::StructOpt;

fn handle_exit_err<E: std::fmt::Debug>(res: Result<(), E>) -> ! {
    match res {
        Ok(_) => process::exit(0),
        Err(err) => {
            eprintln!("{:#?}", err);
            process::exit(1)
        }
    }
}

/// Find problems in your SQL
#[derive(StructOpt, Debug)]
struct Opt {
    /// Paths to search
    paths: Vec<String>,
    /// Exclude specific warnings
    ///
    /// For example:
    /// --exclude=require-concurrent-index-creation,ban-drop-database
    #[structopt(short, long, use_delimiter = true)]
    exclude: Option<Vec<String>>,
    /// List all available rules
    #[structopt(long)]
    list_rules: bool,
    /// Provide documentation on the given rule
    #[structopt(long)]
    explain: Option<String>,
    /// Output AST in JSON
    #[structopt(long, possible_values = &DumpAstOption::variants(), case_insensitive = true)]
    dump_ast: Option<DumpAstOption>,
    /// Style of error reporting
    #[structopt(long, possible_values = &Reporter::variants(), case_insensitive = true)]
    reporter: Option<Reporter>,
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

fn main() {
    let opts = Opt::from_args();
    let mut clap_app = Opt::clap();
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let is_stdin = !atty::is(Stream::Stdin);
    if let Some(subcommand) = opts.cmd {
        match check_and_comment_on_pr(subcommand, is_stdin) {
            Ok(exit_code) => {
                process::exit(exit_code);
            }
            Err(err) => {
                eprintln!("{:#?}", err);
                process::exit(1);
            }
        }
    } else if !opts.paths.is_empty() || is_stdin {
        if let Some(dump_ast_kind) = opts.dump_ast {
            handle_exit_err(dump_ast_for_paths(
                &mut handle,
                &opts.paths,
                is_stdin,
                dump_ast_kind,
            ));
        } else {
            match check_files(&opts.paths, is_stdin, opts.exclude) {
                Ok(violations) => {
                    let reporter = opts.reporter.unwrap_or(Reporter::Tty);
                    let exit_code = if !violations.is_empty() { 1 } else { 0 };
                    match print_violations(&mut handle, violations, &reporter) {
                        Ok(_) => {
                            process::exit(exit_code);
                        }
                        Err(e) => {
                            eprintln!("{:#?}", e);
                            process::exit(1);
                        }
                    }
                }
                e => {
                    eprintln!("{:#?}", e);
                    process::exit(1)
                }
            }
        }
    } else if opts.list_rules {
        handle_exit_err(list_rules(&mut handle));
    } else if let Some(rule_name) = opts.explain {
        handle_exit_err(explain_rule(&mut handle, &rule_name));
    } else {
        clap_app.print_long_help().expect("problem printing help");
    }
}
