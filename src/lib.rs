use anyhow::{Context, Result};
use rand::{distr::weighted::WeightedIndex, prelude::*};
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    Char(char), // A token that refers to a single (usually input) char.
    Pair(u32),  // A token that refers to a previously combined pair of chars or other tokens.
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Char(c) => write!(f, "\"{}\"", c),
            Token::Pair(i) => write!(f, "T{}", i),
        }
    }
}

pub type TokenGrammar = Vec<(Token, Token, u32)>;

/// Read a token grammar from a slice of bytes (grouped into u32s).
pub fn read_tokens(bytes: &[u32]) -> Result<TokenGrammar> {
    let mut iter = bytes.into_iter();
    let mut tokens = vec![];

    loop {
        let l = match iter.next() {
            Some(bytes) if *bytes == 0 => Token::Pair(*iter.next().context("be nice")?),
            Some(bytes) => Token::Char(char::from_u32(*bytes).unwrap()),
            None => break,
        };
        let r = match iter.next() {
            Some(bytes) if *bytes == 0 => Token::Pair(*iter.next().context("be nice")?),
            Some(bytes) => Token::Char(char::from_u32(*bytes).unwrap()),
            None => break,
        };
        tokens.push((l, r, *iter.next().context("be nicer")?));
    }

    Ok(tokens)
}

/// Load a token grammar from a file.
pub fn load_tokens<P: AsRef<Path>>(path: P) -> Result<TokenGrammar> {
    let mut file = File::open(path)?;
    let mut buffer = vec![];

    file.read_to_end(&mut buffer)?;

    let mut bytes = vec![];
    for chunk in buffer.chunks_exact(4) {
        bytes.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    read_tokens(&bytes)
}

/// Convert multiple tokens back to text with the provided token grammar.
pub fn tokens_to_string(tokens: &Vec<Token>, token_grammar: &TokenGrammar) -> Result<String> {
    let mut result = String::new();

    for token in tokens {
        result.push_str(&token_to_chars(&token, token_grammar)?);
    }

    Ok(result)
}

/// Convert a token back to text with the provided token grammar.
pub fn token_to_chars(token: &Token, token_grammar: &TokenGrammar) -> Result<String> {
    Ok(match token {
        Token::Char(c) => c.to_string(),
        Token::Pair(i) => {
            let (l, r, _) = token_grammar.get(*i as usize).context("no existy")?;
            format!(
                "{}{}",
                token_to_chars(l, token_grammar)?,
                token_to_chars(r, token_grammar)?
            )
        }
    })
}

/// Write token grammar to a file.
///
/// [`token::Char`]s are written as-is,
/// [`token::Token`]s are prefixed with a null-byte.
pub fn write_tokens<P: AsRef<Path>>(token_grammar: &TokenGrammar, path: P) -> Result<()> {
    let mut file = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    // TODO: Check if moving null byte prefix to token::Char makes a meaningful difference.
    for (l, r, freq) in token_grammar {
        match l {
            Token::Char(c) => {
                file.write(&(*c as u32).to_le_bytes())?;
            }
            Token::Pair(i) => {
                file.write(&[0; 4])?;
                file.write(&i.to_le_bytes())?;
            }
        };
        match r {
            Token::Char(c) => {
                file.write(&(*c as u32).to_le_bytes())?;
            }
            Token::Pair(i) => {
                file.write(&[0; 4])?;
                file.write(&i.to_le_bytes())?;
            }
        };
        file.write(&freq.to_le_bytes())?;
    }

    Ok(())
}

/// Find valid followup tokens for a given root token and pre-existing token grammar.
pub fn find_valid_tokens(token: &Token, token_grammar: &TokenGrammar) -> Vec<Token> {
    let valid_tokens: Vec<Token> = token_grammar
        .iter()
        .filter(|(l, r, _)| {
            l == token
                && if let Token::Pair(_) = token {
                    l != r
                } else {
                    true
                }
        })
        .map(|(_, r, _)| r.clone())
        .collect();

    if valid_tokens.is_empty() {
        match token {
            Token::Pair(i) => find_valid_tokens(&token_grammar[*i as usize].1, &token_grammar),
            Token::Char(_) => valid_tokens,
        }
    } else {
        valid_tokens
    }
}

/// Generate a new string from a token and pre-existing token grammar, up to a maximum depth.
pub fn generate_gibberish(
    token: &Token,
    token_grammar: &TokenGrammar,
    depth: u32,
) -> Result<String> {
    let mut token = token.clone();
    let mut rng = rand::rng();

    let mut gibberish = token_to_chars(&token, token_grammar)?;
    for _ in 0..depth {
        let valid_tokens = find_valid_tokens(&token, token_grammar);

        if valid_tokens.is_empty() {
            break;
        }

        let weights: Vec<u32> = valid_tokens
            .iter()
            .map(|token| match token {
                Token::Char(_) => 1,
                Token::Pair(i) => token_grammar[*i as usize].2 + i, // frequency + index
            })
            .collect();

        let dist = WeightedIndex::new(&weights).unwrap();

        token = valid_tokens[dist.sample(&mut rng)].clone();
        gibberish.push_str(&token_to_chars(&token, token_grammar)?);
    }

    Ok(gibberish)
}

/// Interpret a string as tokens present in a pre-computed token grammar.
pub fn tokenize(string: String, token_grammar: &TokenGrammar) -> Vec<Token> {
    let mut tokens: Vec<Token> = string.chars().map(|c| Token::Char(c)).collect();

    for (token_idx, token) in token_grammar.iter().enumerate() {
        let mut i = 0;
        while i < tokens.len() - 1 {
            if token.0 == tokens[i] && token.1 == tokens[i + 1] {
                tokens.push(Token::Pair(token_idx as u32));
                tokens.remove(i);
                tokens.swap_remove(i);
            }
            i += 1;
        }
    }

    tokens
}
