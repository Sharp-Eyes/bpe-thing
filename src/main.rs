use anyhow::{Context, Result};
use bpe_thing::*;
use std::{
    collections::HashMap,
    env,
    fs::{self},
    path::Path,
};

// TODO: actual usable cli

fn parse_bpe<P: AsRef<Path>>(
    txt_path: P,
    bpe_path: P,
    max_token_count: u32,
) -> Result<TokenGrammar> {
    let data = fs::read_to_string(txt_path)?;

    // Byte to use for non-char BPE tokens, incremented on demand.
    let mut char_count = 0;

    // Input tokens to be encoded with BPE.
    let mut tokens: Vec<Token> = data.chars().map(|c| Token::Char(c)).collect();

    // Finalised token 'grammar' definition of (left-token, right-token, frequency).
    let mut token_grammar: TokenGrammar = vec![];

    // Hashmap of {(left-token, right-token): frequency} for the entire token stream.
    let mut freqs: HashMap<(Token, Token), usize> = HashMap::new();
    for i in 0..tokens.len() - 1 {
        freqs
            .entry((tokens[i].clone(), tokens[i + 1].clone()))
            .and_modify(|freq| *freq += 1)
            .or_insert(1);
    }

    for _ in 0..max_token_count {
        let (max_pair, max_freq) = freqs
            .iter()
            .max_by(|(_, l_freq), (_, r_freq)| l_freq.cmp(r_freq))
            .map(|(k, v)| ((*k).clone(), *v))
            .unwrap();

        if max_freq == 1 {
            break;
        };

        freqs.remove(&max_pair);
        token_grammar.push((max_pair.0.clone(), max_pair.1.clone(), max_freq as u32));

        let mut i = 0;
        while i < tokens.len() - 1 {
            if max_pair.0 == tokens[i] && max_pair.1 == tokens[i + 1] {
                // Replace most common pair bc at index i with new token Z: abcd -> aZd
                tokens.push(Token::Pair(char_count)); // abcdZ
                tokens.remove(i); // acdZ
                tokens.swap_remove(i); // aZd

                if i > 0 {
                    // Decrement ac freq, increment aZ freq or create it if it does not exist.
                    freqs
                        .entry((tokens[i - 1].clone(), max_pair.0.clone()))
                        .and_modify(|freq| *freq -= 1);
                    freqs
                        .entry((tokens[i - 1].clone(), Token::Pair(char_count)))
                        .and_modify(|freq| *freq += 1)
                        .or_insert(1);
                }

                if i < tokens.len() - 2 {
                    // Decrement cd freq, increment Zd freq or create it if it does not exist.
                    freqs
                        .entry((max_pair.1.clone(), tokens[i + 1].clone()))
                        .and_modify(|freq| *freq -= 1);
                    freqs
                        .entry((Token::Pair(char_count), tokens[i + 1].clone()))
                        .and_modify(|freq| *freq += 1)
                        .or_insert(1);
                }
            }

            i += 1;
        }

        char_count += 1;
    }

    // println!("{:?}", tokens);
    // let _: Vec<_> = token_grammar
    //     .iter()
    //     .enumerate()
    //     .inspect(|(i, (l, r, f))| println!("{}, {} -> T{} ({})", l, r, i, f))
    //     .collect();

    write_tokens(&token_grammar, bpe_path)?;

    Ok(token_grammar)
}

fn generate_from_seed<P: AsRef<Path>>(
    seed: String,
    bpe_path: P,
    max_token_count: u32,
) -> Result<String> {
    let token_grammar = load_tokens(bpe_path)?;

    let mut seed_token_iter = tokenize(seed, &token_grammar).into_iter();
    let last = seed_token_iter.next_back();

    Ok(format!(
        "{}{}",
        tokens_to_string(&seed_token_iter.collect(), &token_grammar)?,
        generate_gibberish(&last.unwrap(), &token_grammar, max_token_count)?
    ))
}

macro_rules! pos_args {
    ($cmd:literal, $iter:ident, $($n:literal),*) => {
        (
            $(
                {
                    let arg = $iter.next().context(format!("Please provide a value for '{}'.", $n))?;
                    if arg.as_str() == "--help" || arg.as_str() == "-h" {
                        print_help(Some($cmd));
                        return Ok(());
                    } else {
                        arg
                    }
                },
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
            println!("  -h, --help        Display concise help for this command");
            println!("  -t, --max-tokens  The maximum number of tokens to generate");
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
                flag_args!(
                    "generate",
                    args_iter,
                    (max_token_count, "--max-tokens" | "-t")
                );

                println!("{}", generate_from_seed(seed, bpe_path, max_token_count)?);
            }
            "parse" | "p" => {
                let (txt_path, bpe_path) = pos_args!("parse", args_iter, "txt_path", "bpe_path");

                let mut max_token_count = u32::MAX;
                flag_args!("parse", args_iter, (max_token_count, "--max-tokens" | "-t"));

                parse_bpe(txt_path, bpe_path, max_token_count)?;
                return Ok(());
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
