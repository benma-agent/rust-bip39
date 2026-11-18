bip39
=====

A Rust implementation of [BIP-39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
mnemonic codes.


## Word lists (languages)

We support all languages
[specified in the BIP-39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md)
as of writing.

The English language is always loaded and other languages can be loaded using the corresponding feature.

Use the `all-languages` feature to enable all languages.

- English (always enabled)
- Simplified Chinese (`chinese-simplified`)
- Traditional Chinese (`chinese-traditional`)
- Czech (`czech`)
- French (`french`)
- Italian (`italian`)
- Japanese (`japanese`)
- Korean (`korean`)
- Portuguese (`portuguese`)
- Spanish (`spanish`)


## Compact word storage

If binary size matters for your app, enable the `compact-wordlist` feature on your `bip39`
dependency in `Cargo.toml`. It applies to every language you enable. One English-only ARM
test program saved about 15 KiB. Mnemonic generation, parsing, and seed derivation use compact
storage automatically.

If you access words directly, use `Mnemonic::words` to iterate over a phrase, `Language::word_at`
to get a word by index, or `Language::words_by_prefix_iter` for autocomplete. Calling
`Language::word_list` or the deprecated `Language::words_by_prefix` also keeps the original
word table in your binary and can cancel out the size savings.

Looking up a word by name can take longer. An x86-64 benchmark looked up all 2,048 words in
each language and compared the average time per lookup with the default word lists in v3.0.0.

| Language | Lookup time with `compact-wordlist` |
| --- | --- |
| English | 14% longer |
| Korean, Portuguese | 25–28% longer |
| Italian | 47% longer |
| Spanish | 1.6× as long |
| Japanese | 1.9× as long |
| French | 2.1× as long |
| Czech | 3× as long |
| Simplified Chinese | 56% shorter |
| Traditional Chinese | 14% shorter |


## MSRV

This crate supports Rust v1.41.1 and up and works with `no_std`.

The `bitcoin_hashes` range dependency effects the MSRV as follows

- `bitcoin_hashes v0.12`: MSRV v1.41.1
- `bitcoin_hashes v0.13`: MSRV v1.48.0
- `bitcoin_hashes v0.14`: MSRV v1.56.0

When using older version of Rust, you might have to pin the versions of several crates, for an up-to-date list refer to [`contrib/test.sh`](contrib/test.sh):

```bash
cargo update --package "bitcoin_hashes" --precise "0.12.0"
cargo update --package "rand" --precise "0.7.0"
cargo update --package "libc" --precise "0.2.151"
cargo update --package "tinyvec" --precise "1.6.0"
cargo update --package "unicode-normalization" --precise "0.1.22"
cargo update --package "ppv-lite86" --precise "0.2.17"
```

If you enable the `zeroize` feature the MSRV becomes 1.51.

The `compact-wordlist` feature requires Rust 1.46 or newer for compile-time table generation.
