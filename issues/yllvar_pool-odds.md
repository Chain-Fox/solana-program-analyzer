## Buy-side trades fail because `market` PDA signer seeds do not match initialization seeds


repo: https://github.com/yllvar/pool-odds

commit: 37241d85ef60664ae7e02f9dfcd5f5d22d63e519

The `market` PDA is initialized in `create_market.rs` with:

https://github.com/yllvar/pool-odds/blob/37241d85ef60664ae7e02f9dfcd5f5d22d63e519/programs/pool-odds/src/instructions/create_market.rs#L29-L40

```rust
seeds = [
    b"market",
    creator.key().as_ref(),
    &global_state.total_markets.to_le_bytes()
]
```

So the canonical market PDA is derived from:

```text
["market", creator, global_state.total_markets]
```

The same instruction also creates the YES/NO share mints with `mint::authority = market`, so later `MintTo` CPIs must be signed by this exact `market` PDA. ([GitHub][1])

However, in `trade.rs`, when `params.is_buy == true`, the program mints shares to the trader using `market` as the mint authority, but signs with:

https://github.com/yllvar/pool-odds/blob/37241d85ef60664ae7e02f9dfcd5f5d22d63e519/programs/pool-odds/src/instructions/trade.rs#L161-L179

```rust
let seeds = &[
    b"market",
    market.creator.as_ref(),
    &market.created_at.to_le_bytes(),
    &[market.bump],
];
```

That reconstructs:

```text
["market", market.creator, market.created_at]
```

not:

```text
["market", market.creator, market_id / global_state.total_markets_at_creation]
```

The buy branch is externally reachable because `TradeParams` includes user-controlled `is_buy`, and when `params.is_buy` is true the handler transfers base tokens, constructs the above signer seeds, and calls `mint_to` with `authority: market.to_account_info()`. ([GitHub][2])

Since `created_at` is set later as a timestamp in `create_market`, it will not equal the `global_state.total_markets` seed used to derive the PDA. As a result, the `market` PDA signature should not be valid for the `MintTo` CPI.

### Impact

Buy-side trading should fail at `mint_to`, making the core buy path unusable.

This does **not** look like direct fund loss, because the failed CPI should revert the whole Solana transaction, including the prior base-token transfer. But it is still a functional DoS / broken core flow.

### Suggested fix

Store the market index used during initialization, for example:

```rust
market.market_id = global_state.total_markets;
```

and use that same value for later PDA signing:

```rust
let seeds = &[
    b"market",
    market.creator.as_ref(),
    &market.market_id.to_le_bytes(),
    &[market.bump],
];
```

Alternatively, derive the market PDA from `created_at` during initialization too, but both sides must use the exact same seed tuple.


Severity: **High functional bug** if buy trading is core, **not asset theft**.
