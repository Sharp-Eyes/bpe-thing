use anyhow::{Context, Result};
use bpe_thing::*;
use std::env;

macro_rules! pos_args {
    ($cmd:literal, $iter:ident, $n:literal) => {
        {
            let arg = $iter.next().context(format!("Please provide a value for '{}'.", $n))?;
            if arg.as_str() == "--help" || arg.as_str() == "-h" {
                print_help(Some($cmd));
                return Ok(());
            } else {
                arg
            }
        }
    };
    ($cmd:literal, $iter:ident, $($n:literal),*) => {
        (
            $(
                pos_args!($cmd, $iter, $n),
            )*
        )
    };
}

macro_rules! flag_args {
    ($cmd:literal, $iter:ident, $(($i:ident, $p:pat)),*) => {
        while let Some(arg) = $iter.next() {
            match arg.as_ref() {
                $(
                    $p => {
                        $i = $iter
                            .next()
                            .context(format!("Please provide a value for flag '{}'", arg))?
                            .parse()?
                    },
                )*
                "--help" | "-h" => {
                    print_help(Some($cmd));
                    return Ok(());
                },
                unknown => anyhow::bail!("Unknown flag: {}.", unknown),
            }
        }
    };
}

fn print_help(command: Option<&str>) {
    match command {
        Some("generate") => {
            println!("Usage: bpe-thing generate <SEED> <BPE_PATH> [OPTIONS]");
            println!("");
            println!("Arguments:");
            println!("  SEED      The seed from which to generate new text");
            println!("  BPE_PATH  The path to a file that contains BPE grammar");
            println!("");
            println!("Options:");
            println!("  -h, --help         Display concise help for this command");
            println!("  -t, --max-tokens   The maximum number of tokens to generate");
            println!("  -f, --freq-weight  The weight to apply to token randomisation based on the token frequency");
            println!("  -i, --idx-weight   The weight to apply to token randomisation based on the token index (bigger = longer phrases)");
            println!("");
        }
        Some("parse") => {
            println!("Usage: bpe-thing parse <TXT_PATH> <BPE_PATH> [OPTIONS]");
            println!("");
            println!("Arguments:");
            println!("  TXT_PATH  The path to the sauce");
            println!("  BPE_PATH  The path to the output BPE grammar file");
            println!("");
            println!("Options:");
            println!("  -h, --help        Display concise help for this command");
            println!("  -t, --max-tokens  The maximum number of tokens to use for the BPE grammar");
            println!("");
        }
        _ => {
            println!("Usage: bpe-thing [COMMAND] [OPTIONS]");
            println!("");
            println!("Commands:");
            println!("  generate  Generate text from a seed and a BPE file");
            println!("  parse     Parse a text file into a BPE file");
            println!("");
            println!("Options:");
            println!("  -h, --help  Display concise help for this command");
            println!("");
        }
    }
}

fn main() -> Result<()> {
    let mut args_iter = env::args().peekable().skip(1);
    match args_iter.next() {
        Some(cmd) => match cmd.to_lowercase().as_ref() {
            "generate" | "gen" | "g" => {
                let (seed, bpe_path) = pos_args!("generate", args_iter, "seed", "bpe_path");

                let mut max_token_count = 32;
                let mut freq_weight = 1.0;
                let mut idx_weight = 1.0;
                flag_args!(
                    "generate",
                    args_iter,
                    (max_token_count, "--max-tokens" | "-t"),
                    (freq_weight, "--freq_weight" | "-f"),
                    (idx_weight, "--idx_weight" | "-i")
                );

                println!(
                    "{}",
                    generate_from_seed(seed, bpe_path, max_token_count, freq_weight, idx_weight)?
                );
            }
            "parse" | "p" => {
                let (txt_path, bpe_path) = pos_args!("parse", args_iter, "txt_path", "bpe_path");

                let mut max_token_count = u32::MAX;
                flag_args!("parse", args_iter, (max_token_count, "--max-tokens" | "-t"));

                parse_bpe(txt_path, bpe_path, max_token_count)?;
            }
            "debug" | "d" => {
                let bpe_path = pos_args!("debug", args_iter, "bpe_path");

                let token_grammar = load_tokens(bpe_path)?;
                debug_grammar(&token_grammar);
            }
            "--help" | "-h" => print_help(None),
            unknown => {
                println!("error: unrecognized subcommand '{}'", unknown);
                println!("");
                print_help(None);
            }
        },
        None => {
            println!("A weird little BPE implementation in Rust.");
            println!("");
            print_help(None)
        }
    };

    Ok(())
}
