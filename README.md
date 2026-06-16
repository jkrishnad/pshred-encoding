# encoder

A small demo of Reed-Solomon erasure coding in Rust, modeled on how a Solana
MCP proposer spreads a block across many attesters.

## What it does

We take some data (a transaction list), split it into 64 pieces, and use
erasure coding to turn those into 256 pieces called shreds. The trick: any 64
of those 256 shreds are enough to rebuild the original data. The other 192 are
redundancy.

So we then throw away 192 shreds at random — simulating packets lost over a
real network — and reconstruct the exact original data from the 64 that
survived. Whichever 64 you keep, it works.

That 64-of-256 split is a 4x redundancy: you can lose three quarters of
everything and still recover, which is why it's useful for getting blocks to
attesters reliably.

## Running it

```
cargo run
```

Prints the data sent, a map of which shreds survived and which were lost, and
the recovered data, then confirms it matches the original.

## Testing

```
cargo test --release
```

Runs 100 trials, each dropping a different random set of 192 shreds, and checks
the data always comes back intact. Use `--release` — the field math is slow in
debug mode (~16s vs under a second).

## Dependencies

- `reed-solomon-erasure` — the actual erasure coding
- `rand` — picking which shreds to drop
- `anyhow` — error handling
