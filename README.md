Weird little BPE implementation in Rust.

## Usage:

To get started, try running
```bash
cargo run --release -- parse ./data/wiki.txt ./data/wiki.bpe
cargo run --release -- generate seed ./data/wiki.bpe -t 10
```

Note: given how small the wiki.txt sample data is, text generated from its bpe will not be particularly sensible.
Provide a larger dataset of your own to get... somewhat sensible output. maybe. sometimes.

## See Also:

- https://en.wikipedia.org/wiki/Byte_pair_encoding
- https://github.com/tsoding/bpe
