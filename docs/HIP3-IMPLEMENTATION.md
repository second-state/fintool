# HIP-3 Implementation Notes

## What is HIP-3?

[HIP-3](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals) enables third-party builders to deploy their own perpetual futures markets on Hyperliquid. These are called **builder-deployed perps** and run on separate "dexes" alongside the main Hyperliquid perp exchange.

Notable HIP-3 dexes include:
- **cash** (dreamcash) — commodities (SILVER, GOLD), US stocks (TSLA, NVDA, GOOGL, etc.), and indices (USA500)
- **xyz** (XYZ) — similar asset coverage
- **flx** (Felix Exchange) — commodities and stocks
- **km** (Markets by Kinetiq) — stocks and commodities

The `cash` dex is the most liquid, with SILVER alone doing hundreds of millions in daily volume.

## Why Custom Signing?

The official [hyperliquid_rust_sdk](https://crates.io/crates/hyperliquid_rust_sdk) (v0.6) **does not support HIP-3 dexes**. It only loads assets from the main perp universe and spot markets:

```rust
// SDK's ExchangeClient::new()
let meta = info.meta().await?;          // main perps only
let spot = info.spot_meta().await?;     // spot only
// coin_to_asset map = main perps + spot — no HIP-3
```

The Python SDK supports HIP-3 natively via the `perp_dexs` parameter, but the Rust SDK doesn't expose this. So fintool implements the full signing flow from scratch in `src/hip3.rs`.

## Asset Index Resolution

Every asset on Hyperliquid has a global numeric index used in the wire protocol:

| Asset Type | Index Range | Example |
|-----------|-------------|---------|
| Main perps | 0 – 999 | BTC=0, ETH=1, SOL=2, ... |
| Spot pairs | 10000 + pair_index | PURR/USDC=10000, ... |
| HIP-3 dex 1 | 110000 + asset_index | xyz:XYZ100=110000, ... |
| HIP-3 dex 2 | 120000 + asset_index | flx:COIN=120000, ... |
| HIP-3 dex N | 110000 + N*10000 + asset_index | cash:USA500=1X0000, ... |

To resolve a HIP-3 asset index:

1. **Query `perpDexs`** to get the ordered list of all builder dexes
2. **Find the dex position** (0-indexed, skipping the null main dex entry)
3. **Compute offset**: `110000 + dex_position * 10000`
4. **Query `meta` with `dex` param** to get the dex's universe
5. **Find asset position** within the dex universe
6. **Global index** = offset + position

```rust
// Example: cash:SILVER
// 1. perpDexs returns: [null, {name:"xyz"}, {name:"flx"}, ..., {name:"cash"}]
// 2. cash is at position N (0-indexed among non-null entries)
// 3. offset = 110000 + N * 10000
// 4. meta(dex="cash") → universe = [USA500, TSLA, NVDA, ..., SILVER, ...]
// 5. SILVER is at position M in the universe
// 6. global_index = offset + M
```

## Order Wire Format

Orders use MessagePack (msgpack) for hashing, not JSON. The wire format:

```rust
struct OrderWire {
    a: u32,      // global asset index
    b: bool,     // is_buy
    p: String,   // price (formatted)
    s: String,   // size (formatted)
    r: bool,     // reduce_only
    t: OrderType, // {limit: {tif: "Gtc"}}
}

struct OrderAction {
    type: "order",
    orders: Vec<OrderWire>,
    grouping: "na",
}
```

## Signing Flow

The signing is identical to the main SDK, just with different asset indices:

```
1. Build OrderAction struct
2. Serialize to msgpack bytes (rmp_serde::to_vec_named)
3. Append timestamp as 8 big-endian bytes
4. Append 0x00 (no vault) or 0x01 + 20-byte vault address
5. Keccak256 hash → connection_id (H256)
6. EIP-712 sign Agent { source: "a"|"b", connection_id }
   - source = "a" for mainnet, "b" for testnet
   - Domain: { name: "Exchange", version: "1", chainId: 1337 }
7. POST to /exchange with:
   {
     action: { type: "order", orders: [...], grouping: "na" },
     nonce: timestamp,
     signature: { r, s, v },
     vaultAddress: null
   }
```

Note: The API receives the action as **JSON** (not msgpack), but the **hash for signing** uses msgpack. This is a critical detail — the JSON and msgpack representations of the same order produce different bytes.

## Price/Size Formatting

Prices and sizes must be formatted to match the SDK's precision:
- **Price**: 5 significant figures, then rounded to `6 - szDecimals` decimal places
- **Size**: `szDecimals` decimal places (from the asset's metadata)

`szDecimals` varies by asset (typically 3-4 for commodities/stocks).

## API Endpoints Used

| Endpoint | Method | Body | Purpose |
|----------|--------|------|---------|
| `/info` | POST | `{"type":"perpDexs"}` | List all HIP-3 dexes |
| `/info` | POST | `{"type":"meta","dex":"cash"}` | Get dex universe + asset metadata |
| `/info` | POST | `{"type":"metaAndAssetCtxs","dex":"cash"}` | Get universe + live market data |
| `/info` | POST | `{"type":"allMids","dex":"cash"}` | Get all mid prices for a dex |
| `/info` | POST | `{"type":"clearinghouseState","user":"0x...","dex":"cash"}` | User positions on a dex |
| `/exchange` | POST | `{action, nonce, signature}` | Submit signed order |

## Dependencies

- `rmp-serde` — msgpack serialization (matching the SDK's wire format)
- `ethers` — EIP-712 signing, keccak256, wallet management
- `reqwest` — HTTP client for API calls

## Files

- `src/hip3.rs` — HIP-3 signing and order execution
- `src/commands/quote.rs` — HIP-3 perp quote (queries dex meta)
- `src/commands/perp.rs` — Routes HIP-3 symbols to `hip3::place_order()`

## References

- [HIP-3 Documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals)
- [Perpetuals API (with dex param)](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals)
- [Exchange Endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint)
- [Python SDK (reference implementation)](https://github.com/hyperliquid-dex/hyperliquid-python-sdk)
