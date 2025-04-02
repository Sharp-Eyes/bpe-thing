use anyhow::Result;
use bpe_thing::*;
use std::{
    collections::HashMap,
    env,
    fs::{self},
};

// TODO: actual usable cli

fn main() -> Result<()> {
    let token_count: u32 = env::args()
        .skip(1)
        .next()
        .unwrap_or_else(|| "2".to_string())
        .parse()
        .unwrap();

    let data = fs::read_to_string("./data/wiki.txt")?;

    // Byte to use for non-char BPE tokens, incremented on demand.
    let mut char_count = 0;

    // Input tokens to be encoded with BPE.
    let mut tokens_in: Vec<Token> = data.chars().map(|c| Token::Char(c)).collect();

    // Finalised token 'grammar' definition of (left-token, right-token, frequency).
    let mut token_grammar: TokenGrammar = vec![];

    // Hashmap of {(left-token, right-token): frequency} for the entire token stream.
    let mut freqs: HashMap<(Token, Token), usize> = HashMap::new();
    for i in 0..tokens_in.len() - 1 {
        freqs
            .entry((tokens_in[i].clone(), tokens_in[i + 1].clone()))
            .and_modify(|freq| *freq += 1)
            .or_insert(1);
    }

    for _ in 0..token_count {
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
        while i < tokens_in.len() - 1 {
            if max_pair.0 == tokens_in[i] && max_pair.1 == tokens_in[i + 1] {
                // Replace most common pair bc at index i with new token Z: abcd -> aZd
                tokens_in.push(Token::Pair(char_count)); // abcdZ
                tokens_in.remove(i); // acdZ
                tokens_in.swap_remove(i); // aZd

                if i > 0 {
                    // Decrement ac freq, increment aZ freq or create it if it does not exist.
                    freqs
                        .entry((tokens_in[i - 1].clone(), max_pair.0.clone()))
                        .and_modify(|freq| *freq -= 1);
                    freqs
                        .entry((tokens_in[i - 1].clone(), Token::Pair(char_count)))
                        .and_modify(|freq| *freq += 1)
                        .or_insert(1);
                }

                if i < tokens_in.len() - 2 {
                    // Decrement cd freq, increment Zd freq or create it if it does not exist.
                    freqs
                        .entry((max_pair.1.clone(), tokens_in[i + 1].clone()))
                        .and_modify(|freq| *freq -= 1);
                    freqs
                        .entry((Token::Pair(char_count), tokens_in[i + 1].clone()))
                        .and_modify(|freq| *freq += 1)
                        .or_insert(1);
                }
            }

            i += 1;
        }

        char_count += 1;
    }

    println!("{:?}", tokens_in);
    let _: Vec<_> = token_grammar
        .iter()
        .enumerate()
        .inspect(|(i, (l, r, f))| println!("{}, {} -> T{} ({})", l, r, i, f))
        .collect();

    write_tokens(&token_grammar, "./data/wiki.bpe")?;
    let pls = load_tokens("./data/wiki.bpe")?;

    let _: Vec<_> = pls
        .iter()
        .enumerate()
        .inspect(|(i, (l, r, f))| println!("{}, {} -> T{} ({})", l, r, i, f))
        .collect();

    let mut ok = tokenize("BPE ".to_string(), &pls).into_iter();
    let last = ok.next_back();

    println!(
        "{}{}",
        tokens_to_string(&ok.collect(), &pls)?,
        generate_gibberish(&last.unwrap(), &pls, 20)?
    );

    Ok(())
}
