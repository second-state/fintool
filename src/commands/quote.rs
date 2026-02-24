use anyhow::{Result, Context};
use colored::Colorize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::config;

/// Aliases for common index/ETF symbols → Yahoo Finance tickers
fn symbol_aliases() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    // Indices
    map.insert("SP500", "^GSPC");
    map.insert("SPX", "^GSPC");
    map.insert("NASDAQ", "^IXIC");
    map.insert("NDX", "^NDX");
    map.insert("QQQ", "QQQ");
    map.insert("DOW", "^DJI");
    map.insert("DJI", "^DJI");
    map.insert("DJIA", "^DJI");
    map.insert("RUSSELL", "^RUT");
    map.insert("RUT", "^RUT");
    map.insert("VIX", "^VIX");
    // International
    map.insert("NIKKEI", "^N225");
    map.insert("FTSE", "^FTSE");
    map.insert("DAX", "^GDAXI");
    map.insert("HSI", "^HSI");
    map.insert("HANGSENG", "^HSI");
    // Commodities
    map.insert("GOLD", "GC=F");
    map.insert("SILVER", "SI=F");
    map.insert("OIL", "CL=F");
    map.insert("CRUDE", "CL=F");
    map.insert("NATGAS", "NG=F");
    // Treasury
    map.insert("TNX", "^TNX");
    map.insert("10Y", "^TNX");
    map.insert("TYX", "^TYX");
    map.insert("30Y", "^TYX");
    map
}

/// CoinGecko symbol to ID mapping for top cryptos
fn coingecko_symbol_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("BTC", "bitcoin");
    map.insert("ETH", "ethereum");
    map.insert("SOL", "solana");
    map.insert("BNB", "binancecoin");
    map.insert("XRP", "ripple");
    map.insert("ADA", "cardano");
    map.insert("DOGE", "dogecoin");
    map.insert("DOT", "polkadot");
    map.insert("MATIC", "matic-network");
    map.insert("AVAX", "avalanche-2");
    map.insert("UNI", "uniswap");
    map.insert("LINK", "chainlink");
    map.insert("LTC", "litecoin");
    map.insert("BCH", "bitcoin-cash");
    map.insert("ATOM", "cosmos");
    map.insert("XLM", "stellar");
    map.insert("ALGO", "algorand");
    map.insert("VET", "vechain");
    map.insert("ICP", "internet-computer");
    map.insert("FIL", "filecoin");
    map.insert("TRX", "tron");
    map.insert("ETC", "ethereum-classic");
    map.insert("HBAR", "hedera-hashgraph");
    map.insert("NEAR", "near");
    map.insert("APT", "aptos");
    map.insert("ARB", "arbitrum");
    map.insert("OP", "optimism");
    map.insert("SUI", "sui");
    map.insert("SEI", "sei-network");
    map.insert("PEPE", "pepe");
    map.insert("USD1", "usd1-wlfi");
    map.insert("USDC", "usd-coin");
    map.insert("USDT", "tether");
    map.insert("DAI", "dai");
    map
}

/// Spot price quote: Fetch from ALL sources in parallel, optionally use OpenAI for enrichment
pub async fn run_spot(symbol: &str, json_output: bool) -> Result<()> {
    let raw_upper = symbol.to_uppercase();
    let aliases = symbol_aliases();
    let symbol_upper = aliases.get(raw_upper.as_str()).map(|s| s.to_string()).unwrap_or(raw_upper);
    let client = reqwest::Client::new();

    // Fetch all sources in parallel
    let (hl_result, yf_result, cg_result) = tokio::join!(
        fetch_hl_spot(&client, &symbol_upper),
        fetch_yahoo_quote(&client, &symbol_upper),
        fetch_coingecko(&client, &symbol_upper)
    );

    // Collect successful sources
    let mut sources = Vec::new();
    if let Ok(ref data) = hl_result {
        sources.push(("Hyperliquid", data.clone()));
    }
    if let Ok(ref data) = yf_result {
        sources.push(("Yahoo Finance", data.clone()));
    }
    if let Ok(ref data) = cg_result {
        sources.push(("CoinGecko", data.clone()));
    }

    // If no sources succeeded, bail
    if sources.is_empty() {
        anyhow::bail!("No data sources returned results for {}", symbol);
    }

    // Try OpenAI enrichment if key is configured
    if let Some(api_key) = config::openai_api_key() {
        match enrich_with_openai(&client, &api_key, &symbol_upper, &sources).await {
            Ok(enriched) => {
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&enriched)?);
                } else {
                    print_enriched_quote(&enriched);
                }
                return Ok(());
            }
            Err(e) => {
                eprintln!("OpenAI enrichment failed: {}, falling back to basic output", e);
            }
        }
    }

    // Fallback: merge data from available sources
    let merged = merge_sources(&symbol_upper, &sources);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&merged)?);
    } else {
        print_basic_quote(&merged);
    }

    Ok(())
}

/// Perp price quote: funding rate, OI, mark price, etc.
pub async fn run_perp(symbol: &str, json_output: bool) -> Result<()> {
    let symbol_upper = symbol.to_uppercase();
    let client = reqwest::Client::new();
    let data = fetch_hl_perp(&client, &symbol_upper).await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print_perp_quote(&data);
    }

    Ok(())
}

async fn fetch_hl_spot(client: &reqwest::Client, symbol: &str) -> Result<Value> {
    let url = config::info_url();

    let mids: HashMap<String, String> = client
        .post(&url)
        .json(&json!({"type": "allMids"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse allMids")?;

    // Resolve spot @index
    let spot_meta: Value = client
        .post(&url)
        .json(&json!({"type": "spotMeta"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse spotMeta")?;

    let mut idx_to_name = HashMap::new();
    if let Some(tokens) = spot_meta.get("tokens").and_then(|t| t.as_array()) {
        for token in tokens {
            if let (Some(idx), Some(name)) = (
                token.get("index").and_then(|i| i.as_u64()),
                token.get("name").and_then(|n| n.as_str()),
            ) {
                idx_to_name.insert(idx, name.to_string());
            }
        }
    }

    let mut spot_key = None;
    let mut spot_index = None;
    if let Some(universe) = spot_meta.get("universe").and_then(|u| u.as_array()) {
        for pair in universe {
            if let (Some(tokens), Some(index)) = (
                pair.get("tokens").and_then(|t| t.as_array()),
                pair.get("index").and_then(|i| i.as_u64()),
            ) {
                if tokens.len() == 2 {
                    let t1 = tokens[0].as_u64().unwrap_or(0);
                    let name1 = idx_to_name.get(&t1).map(|s| s.as_str()).unwrap_or("");
                    if name1.eq_ignore_ascii_case(symbol) {
                        spot_key = Some(format!("@{}", index));
                        spot_index = Some(index);
                        break;
                    }
                }
            }
        }
    }

    let key = spot_key.ok_or_else(|| anyhow::anyhow!("Symbol {} not found on Hyperliquid spot", symbol))?;
    let price = mids.get(&key).ok_or_else(|| anyhow::anyhow!("No mid price for {}", symbol))?;

    // Get spot context for volume/prevDayPx
    let mut volume = String::new();
    let mut mark_px = String::new();
    let mut prev_day_px = String::new();

    let spot_resp: Value = client
        .post(&url)
        .json(&json!({"type": "spotMetaAndAssetCtxs"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse spotMetaAndAssetCtxs")?;

    if let Some(ctxs) = spot_resp.get(1).and_then(|c| c.as_array()) {
        let expected = format!("@{}", spot_index.unwrap());
        for ctx in ctxs {
            if ctx.get("coin").and_then(|c| c.as_str()) == Some(&expected) {
                volume = ctx.get("dayNtlVlm").and_then(|f| f.as_str()).unwrap_or("").to_string();
                mark_px = ctx.get("markPx").and_then(|f| f.as_str()).unwrap_or("").to_string();
                prev_day_px = ctx.get("prevDayPx").and_then(|f| f.as_str()).unwrap_or("").to_string();
                break;
            }
        }
    }

    let display_price = if !mark_px.is_empty() { mark_px.clone() } else { price.clone() };
    let change_24h = calc_change(&display_price, &prev_day_px);

    Ok(json!({
        "symbol": symbol,
        "price": display_price,
        "change24h": change_24h,
        "volume24h": volume,
        "prevDayPx": prev_day_px,
        "source": "Hyperliquid"
    }))
}

async fn fetch_hl_perp(client: &reqwest::Client, symbol: &str) -> Result<Value> {
    let url = config::info_url();

    let meta_resp: Value = client
        .post(&url)
        .json(&json!({"type": "metaAndAssetCtxs"}))
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse metaAndAssetCtxs")?;

    if let (Some(universe), Some(ctxs)) = (
        meta_resp.get(0).and_then(|m| m.get("universe")).and_then(|u| u.as_array()),
        meta_resp.get(1).and_then(|c| c.as_array()),
    ) {
        for (i, asset) in universe.iter().enumerate() {
            if asset.get("name").and_then(|n| n.as_str()) == Some(symbol) {
                if let Some(ctx) = ctxs.get(i) {
                    let funding = ctx.get("funding").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let open_interest = ctx.get("openInterest").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let volume = ctx.get("dayNtlVlm").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let mark_px = ctx.get("markPx").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let oracle_px = ctx.get("oraclePx").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let prev_day_px = ctx.get("prevDayPx").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let premium = ctx.get("premium").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    let max_leverage = asset.get("maxLeverage").and_then(|l| l.as_u64()).unwrap_or(0);
                    let change_24h = calc_change(&mark_px, &prev_day_px);

                    return Ok(json!({
                        "symbol": symbol,
                        "markPx": mark_px,
                        "oraclePx": oracle_px,
                        "change24h": change_24h,
                        "funding": funding,
                        "premium": premium,
                        "openInterest": open_interest,
                        "volume24h": volume,
                        "prevDayPx": prev_day_px,
                        "maxLeverage": max_leverage,
                        "source": "Hyperliquid"
                    }));
                }
            }
        }
    }

    anyhow::bail!("Symbol {} not found in Hyperliquid perps", symbol)
}

async fn fetch_yahoo_quote(client: &reqwest::Client, symbol: &str) -> Result<Value> {
    // If symbol is a known crypto, try -USD first (avoid stock ticker collisions like BTC=Grayscale)
    let is_crypto = coingecko_symbol_map().contains_key(symbol);
    let symbols_to_try = if is_crypto {
        vec![format!("{}-USD", symbol), symbol.to_string()]
    } else {
        vec![symbol.to_string(), format!("{}-USD", symbol)]
    };
    
    for ticker in symbols_to_try {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=5d",
            ticker
        );
        
        let resp_result = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await;
        
        if let Ok(resp) = resp_result {
            if let Ok(json_resp) = resp.json::<Value>().await {
                if let Some(result) = json_resp["chart"]["result"].get(0) {
                    let meta = &result["meta"];
                    if let Some(price) = meta["regularMarketPrice"].as_f64() {
                        if price > 0.0 {
                            let prev_close = meta["chartPreviousClose"].as_f64().unwrap_or(price);
                            let change_pct = if prev_close > 0.0 {
                                ((price - prev_close) / prev_close) * 100.0
                            } else {
                                0.0
                            };

                            // Try to get 52-week range, market cap, volume
                            let market_cap = meta["marketCap"].as_f64();
                            let volume = meta["regularMarketVolume"].as_f64();
                            let fifty_two_week_high = meta["fiftyTwoWeekHigh"].as_f64();
                            let fifty_two_week_low = meta["fiftyTwoWeekLow"].as_f64();

                            let mut data = json!({
                                "symbol": symbol,
                                "price": format!("{:.2}", price),
                                "change24h": format!("{:.2}", change_pct),
                                "currency": meta["currency"].as_str().unwrap_or("USD"),
                                "exchange": meta["exchangeName"].as_str().unwrap_or(""),
                                "source": "Yahoo Finance"
                            });

                            if let Some(mc) = market_cap {
                                data["marketCap"] = json!(mc);
                            }
                            if let Some(vol) = volume {
                                data["volume24h"] = json!(vol);
                            }
                            if let Some(high) = fifty_two_week_high {
                                data["fiftyTwoWeekHigh"] = json!(high);
                            }
                            if let Some(low) = fifty_two_week_low {
                                data["fiftyTwoWeekLow"] = json!(low);
                            }

                            return Ok(data);
                        }
                    }
                }
            }
        }
    }
    
    anyhow::bail!("Could not fetch valid data from Yahoo Finance for {}", symbol)
}

async fn fetch_coingecko(client: &reqwest::Client, symbol: &str) -> Result<Value> {
    // Try to map symbol to CoinGecko ID
    let map = coingecko_symbol_map();
    let mut cg_id = map.get(symbol.to_uppercase().as_str()).map(|s| s.to_string());

    // If not in hardcoded map, try search
    if cg_id.is_none() {
        let search_url = format!("https://api.coingecko.com/api/v3/search?query={}", symbol);
        if let Ok(search_resp) = client.get(&search_url).send().await {
            if let Ok(search_json) = search_resp.json::<Value>().await {
                if let Some(coins) = search_json["coins"].as_array() {
                    if let Some(first) = coins.first() {
                        if let Some(id) = first["id"].as_str() {
                            cg_id = Some(id.to_string());
                        }
                    }
                }
            }
        }
    }

    let id = cg_id.ok_or_else(|| anyhow::anyhow!("Could not find CoinGecko ID for {}", symbol))?;

    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&market_data=true&community_data=false&developer_data=false",
        id
    );
    let raw_resp = client
        .get(&url)
        .header("User-Agent", "fintool/0.1")
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to reach CoinGecko")?;
    
    if !raw_resp.status().is_success() {
        anyhow::bail!("CoinGecko returned status {}", raw_resp.status());
    }
    
    let resp: Value = raw_resp.json().await.context("Failed to parse CoinGecko response")?;

    let market_data = &resp["market_data"];
    let price = market_data["current_price"]["usd"].as_f64();
    let change_24h = market_data["price_change_percentage_24h"].as_f64();
    let change_7d = market_data["price_change_percentage_7d"].as_f64();
    let change_30d = market_data["price_change_percentage_30d"].as_f64();
    let market_cap = market_data["market_cap"]["usd"].as_f64();
    let volume = market_data["total_volume"]["usd"].as_f64();
    let ath = market_data["ath"]["usd"].as_f64();
    let atl = market_data["atl"]["usd"].as_f64();

    let mut data = json!({
        "symbol": symbol,
        "name": resp["name"].as_str().unwrap_or(symbol),
        "source": "CoinGecko"
    });

    // Only include price if valid
    if let Some(p) = price {
        if p > 0.0 {
            data["price"] = json!(p);
        }
    }

    if let Some(ch) = change_24h {
        data["change_24h_pct"] = json!(ch);
    }
    if let Some(ch) = change_7d {
        data["change_7d_pct"] = json!(ch);
    }
    if let Some(ch) = change_30d {
        data["change_30d_pct"] = json!(ch);
    }
    if let Some(mc) = market_cap {
        data["market_cap"] = json!(mc);
    }
    if let Some(vol) = volume {
        data["volume_24h"] = json!(vol);
    }
    if let Some(a) = ath {
        data["ath"] = json!(a);
    }
    if let Some(a) = atl {
        data["atl"] = json!(a);
    }

    Ok(data)
}

/// Call OpenAI API to enrich the data
async fn enrich_with_openai(
    client: &reqwest::Client,
    api_key: &str,
    symbol: &str,
    sources: &[(&str, Value)],
) -> Result<Value> {
    let sources_json: Vec<Value> = sources
        .iter()
        .map(|(name, data)| {
            json!({
                "source": name,
                "data": data
            })
        })
        .collect();

    let system_prompt = r#"You are a financial data analyst. Given raw market data from multiple sources for the SAME asset, merge them into one structured JSON analysis.

CRITICAL RULES for merging:
1. PRICE: All sources should report roughly the same price for the same asset. If prices differ wildly (e.g. $28 vs $96000), one source is returning the WRONG asset (e.g. a stock ticker collision). DISCARD the outlier and do NOT include that source in sources_used.
2. CHANGE: CoinGecko's "change_24h_pct", "change_7d_pct", "change_30d_pct" are the most reliable percentage changes. Yahoo's "change24h" is computed from chartPreviousClose which may differ. Prefer CoinGecko for crypto % changes. For stocks, prefer Yahoo.
3. VOLUME: Use the largest credible volume figure. CoinGecko "volume_24h" is global crypto volume. Yahoo "volume24h" may be exchange-specific.
4. MARKET CAP: Prefer CoinGecko for crypto, Yahoo for stocks.
5. TREND: Base trend assessment on the actual percentage changes (24h, 7d, 30d), not on the absolute price. A -2% day is mildly bearish, -10% is strongly bearish, +5% is moderately bullish, etc.
6. Be concise in momentum/volume_analysis/summary — no filler words.
7. ALWAYS extract change_7d_pct and change_30d_pct from CoinGecko if available — never return null when the data exists in the input.
8. List ALL sources whose data you actually used in sources_used — if you used price from one and changes from another, list both.
9. Prefer CoinGecko's change_24h_pct over Yahoo's change24h for crypto assets."#;

    let user_prompt = format!(
        r#"Merge the following raw data for symbol "{}" into a single JSON object:

{{
  "symbol": "string",
  "name": "string (full name if known)",
  "price": number (best/most reliable current price in USD),
  "price_currency": "USD",
  "change_24h_pct": number or null,
  "change_7d_pct": number or null,
  "change_30d_pct": number or null,
  "volume_24h": number or null,
  "market_cap": number or null,
  "trend": "bullish" | "bearish" | "neutral",
  "trend_strength": "strong" | "moderate" | "weak",
  "momentum": "string (1-2 sentences, factual, based on actual % changes)",
  "volume_analysis": "string (1 sentence putting volume in context)",
  "summary": "string (2-3 sentences, concise market overview)",
  "sources_used": ["only sources whose data was actually used (exclude wrong-asset outliers)"],
  "confidence": "high" | "medium" | "low"
}}

Raw source data:
{}
"#,
        symbol,
        serde_json::to_string_pretty(&sources_json)?
    );

    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "response_format": {"type": "json_object"},
        "temperature": 0.3
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to reach OpenAI API")?;

    let resp_json: Value = resp.json().await.context("Failed to parse OpenAI response")?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?;

    let enriched: Value = serde_json::from_str(content)
        .context("Failed to parse OpenAI JSON response")?;

    Ok(enriched)
}

/// Merge data from multiple sources (fallback when OpenAI not available)
fn merge_sources(symbol: &str, sources: &[(&str, Value)]) -> Value {
    let mut merged = json!({
        "symbol": symbol,
        "sources_used": sources.iter().map(|(name, _)| *name).collect::<Vec<_>>()
    });

    // Pick best price (prefer Hyperliquid > CoinGecko > Yahoo)
    for (name, data) in sources {
        if *name == "Hyperliquid" {
            if let Some(price) = data.get("price") {
                merged["price"] = price.clone();
                merged["source"] = json!("Hyperliquid");
            }
            if let Some(change) = data.get("change24h") {
                merged["change24h"] = change.clone();
            }
            if let Some(vol) = data.get("volume24h") {
                merged["volume24h"] = vol.clone();
            }
        }
    }

    // Add CoinGecko data if available
    for (name, data) in sources {
        if *name == "CoinGecko" {
            if merged.get("price").is_none() {
                if let Some(price) = data.get("price") {
                    merged["price"] = price.clone();
                }
            }
            if let Some(mc) = data.get("market_cap") {
                merged["market_cap"] = mc.clone();
            }
            if let Some(ch) = data.get("change_7d_pct") {
                merged["change_7d_pct"] = ch.clone();
            }
            if let Some(ch) = data.get("change_30d_pct") {
                merged["change_30d_pct"] = ch.clone();
            }
        }
    }

    // Add Yahoo data if available
    for (name, data) in sources {
        if *name == "Yahoo Finance" {
            if merged.get("price").is_none() {
                if let Some(price) = data.get("price") {
                    merged["price"] = price.clone();
                }
            }
            if merged.get("change24h").is_none() {
                if let Some(change) = data.get("change24h") {
                    merged["change24h"] = change.clone();
                }
            }
            if let Some(mc) = data.get("marketCap") {
                merged["market_cap"] = mc.clone();
            }
            if let Some(vol) = data.get("volume24h") {
                merged["volume24h"] = vol.clone();
            }
        }
    }

    merged
}

fn calc_change(current: &str, previous: &str) -> String {
    if let (Ok(cur), Ok(prev)) = (current.parse::<f64>(), previous.parse::<f64>()) {
        if prev > 0.0 {
            return format!("{:.2}", ((cur - prev) / prev) * 100.0);
        }
    }
    String::new()
}

fn print_enriched_quote(data: &Value) {
    let symbol = data["symbol"].as_str().unwrap_or("");
    let name = data["name"].as_str().unwrap_or(symbol);
    let price = data["price"].as_f64().unwrap_or(0.0);
    let change_24h = data["change_24h_pct"].as_f64();
    let trend = data["trend"].as_str().unwrap_or("neutral");
    let trend_strength = data["trend_strength"].as_str().unwrap_or("unknown");
    let momentum = data["momentum"].as_str().unwrap_or("");
    let volume_analysis = data["volume_analysis"].as_str().unwrap_or("");
    let summary = data["summary"].as_str().unwrap_or("");
    let confidence = data["confidence"].as_str().unwrap_or("unknown");

    let trend_arrow = match trend {
        "bullish" => "📈",
        "bearish" => "📉",
        _ => "➡️",
    };

    let change_str = if let Some(ch) = change_24h {
        crate::format::color_change(&format!("{:.2}", ch))
    } else {
        "N/A".to_string()
    };

    println!();
    println!("  {} {} ({})", "📊", symbol.bold().cyan(), name.dimmed());
    println!("  Price:      {}", format!("${:.2}", price).green().bold());
    println!("  24h Change: {}", change_str);
    println!();
    println!("  {} Trend:  {} ({})", trend_arrow, trend, trend_strength);
    println!();
    if !momentum.is_empty() {
        println!("  💫 Momentum: {}", momentum);
    }
    if !volume_analysis.is_empty() {
        println!("  📊 Volume:   {}", volume_analysis);
    }
    println!();
    if !summary.is_empty() {
        println!("  📝 Summary:");
        println!("     {}", summary);
        println!();
    }

    if let Some(sources) = data["sources_used"].as_array() {
        let source_names: Vec<String> = sources
            .iter()
            .filter_map(|s| s.as_str())
            .map(|s| s.to_string())
            .collect();
        println!("  Sources: {} | Confidence: {}", source_names.join(", ").yellow(), confidence);
    }
    println!();
}

fn print_basic_quote(data: &Value) {
    let symbol = data["symbol"].as_str().unwrap_or("");
    
    // Handle price as either string or number
    let price_str = if let Some(p) = data["price"].as_str() {
        p.to_string()
    } else if let Some(p) = data["price"].as_f64() {
        format!("{:.2}", p)
    } else {
        "?".to_string()
    };
    
    let change = data["change24h"].as_str().unwrap_or("0");
    let change_str = crate::format::color_change(change);

    println!();
    println!("  {} {}", "📊", symbol.bold().cyan());
    println!("  Price:      {}", format!("${}", price_str).green().bold());
    println!("  24h Change: {}", change_str);

    if let Some(vol) = data.get("volume24h") {
        println!("  24h Volume: {}", vol);
    }
    if let Some(mc) = data.get("market_cap") {
        println!("  Market Cap: ${}", mc);
    }

    if let Some(sources) = data["sources_used"].as_array() {
        let source_names: Vec<String> = sources
            .iter()
            .filter_map(|s| s.as_str())
            .map(|s| s.to_string())
            .collect();
        println!("  Sources:    {}", source_names.join(", ").yellow());
    }
    println!();
}

fn print_perp_quote(data: &Value) {
    let symbol = data["symbol"].as_str().unwrap_or("");
    let mark = data["markPx"].as_str().unwrap_or("?");
    let oracle = data["oraclePx"].as_str().unwrap_or("-");
    let change = data["change24h"].as_str().unwrap_or("-");
    let funding = data["funding"].as_str().unwrap_or("-");
    let premium = data["premium"].as_str().unwrap_or("-");
    let oi = data["openInterest"].as_str().unwrap_or("-");
    let vol = data["volume24h"].as_str().unwrap_or("-");
    let max_lev = data["maxLeverage"].as_u64().unwrap_or(0);
    let change_str = crate::format::color_change(change);

    println!();
    println!("  {} {} (perp)", "📊".to_string(), symbol.bold().cyan());
    println!("  Mark Price:    {}", format!("${}", mark).green().bold());
    println!("  Oracle Price:  ${}", oracle);
    println!("  24h Change:    {}", change_str);
    println!("  Funding Rate:  {}", funding);
    println!("  Premium:       {}", premium);
    println!("  Open Interest: {}", oi);
    println!("  24h Volume:    ${}", vol);
    println!("  Max Leverage:  {}x", max_lev);
    println!("  Source:        {}", "Hyperliquid".yellow());
    println!();
}
