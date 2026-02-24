use anyhow::{bail, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{config, polymarket};

const POLYMARKET_BASE: &str = "https://gamma-api.polymarket.com";
const KALSHI_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";

// --- Unified market type ---

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Market {
    platform: String,
    id: String,
    question: String,
    yes_price: f64,
    no_price: f64,
    volume: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    liquidity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcomes: Option<Vec<String>>,
    url: String,
}

// --- Polymarket types ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PolyMarket {
    question: Option<String>,
    slug: Option<String>,
    #[serde(rename = "outcomePrices")]
    outcome_prices: Option<String>,
    outcomes: Option<String>,
    volume: Option<serde_json::Value>,
    liquidity: Option<serde_json::Value>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

impl PolyMarket {
    fn to_market(&self) -> Option<Market> {
        let slug = self.slug.as_deref()?;
        let question = self.question.as_deref().unwrap_or("").to_string();
        let (yes, no) = self.parse_prices();
        let volume = val_to_string(self.volume.as_ref());
        let liquidity = self.liquidity.as_ref().map(|v| val_to_string(Some(v)));
        let outcomes: Option<Vec<String>> = self
            .outcomes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        Some(Market {
            platform: "polymarket".into(),
            id: format!("polymarket:{}", slug),
            question,
            yes_price: yes,
            no_price: no,
            volume,
            liquidity,
            end_date: self.end_date.clone(),
            outcomes,
            url: format!("https://polymarket.com/event/{}", slug),
        })
    }

    fn parse_prices(&self) -> (f64, f64) {
        if let Some(ref s) = self.outcome_prices {
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
                let yes = arr
                    .first()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let no = arr
                    .get(1)
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                return (yes, no);
            }
        }
        (0.0, 0.0)
    }
}

// --- Kalshi types ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiMarket {
    ticker: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    yes_sub_title: Option<String>,
    yes_bid: Option<i64>,
    yes_ask: Option<i64>,
    no_bid: Option<i64>,
    no_ask: Option<i64>,
    last_price: Option<i64>,
    volume: Option<i64>,
    open_interest: Option<i64>,
    close_time: Option<String>,
    status: Option<String>,
    event_ticker: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KalshiMarketsResponse {
    markets: Option<Vec<KalshiMarket>>,
}

#[derive(Debug, Deserialize)]
struct KalshiMarketResponse {
    market: Option<KalshiMarket>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KalshiEvent {
    event_ticker: Option<String>,
    title: Option<String>,
    category: Option<String>,
    volume: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct KalshiEventsResponse {
    events: Option<Vec<KalshiEvent>>,
}

impl KalshiMarket {
    fn to_market(&self) -> Option<Market> {
        let ticker = self.ticker.as_deref()?;
        let title = self.title.as_deref().unwrap_or("");
        let subtitle = self
            .subtitle
            .as_deref()
            .or(self.yes_sub_title.as_deref())
            .unwrap_or("");

        // Build a readable question from title + subtitle
        let question = if !subtitle.is_empty() && !title.contains(subtitle) {
            format!("{} — {}", title, subtitle)
        } else {
            title.to_string()
        };

        // Use mid of yes_bid/yes_ask, fall back to last_price (all in cents)
        let yes_bid = self.yes_bid.unwrap_or(0) as f64;
        let yes_ask = self.yes_ask.unwrap_or(100) as f64;
        let last = self.last_price.unwrap_or(0) as f64;

        let yes_cents = if yes_bid > 0.0 && yes_ask < 100.0 {
            (yes_bid + yes_ask) / 2.0
        } else if last > 0.0 {
            last
        } else {
            yes_bid
        };
        let yes = yes_cents / 100.0;
        let no = 1.0 - yes;

        let volume = self.volume.unwrap_or(0).to_string();

        Some(Market {
            platform: "kalshi".into(),
            id: format!("kalshi:{}", ticker),
            question,
            yes_price: yes,
            no_price: no,
            volume,
            liquidity: None,
            end_date: self.close_time.clone(),
            outcomes: Some(vec!["Yes".into(), "No".into()]),
            url: format!("https://kalshi.com/markets/{}", ticker),
        })
    }
}

fn val_to_string(v: Option<&serde_json::Value>) -> String {
    v.map(|v| match v {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => "0".into(),
    })
    .unwrap_or_else(|| "0".into())
}

// --- API fetchers ---

async fn fetch_polymarket_list(
    client: &reqwest::Client,
    limit: usize,
    query: Option<&str>,
) -> Vec<Market> {
    // Fetch more than needed to allow client-side filtering
    let fetch_limit = if query.is_some() {
        100.min(limit * 10)
    } else {
        limit
    };
    let url = format!(
        "{}/markets?limit={}&active=true&closed=false&order=volume&ascending=false",
        POLYMARKET_BASE, fetch_limit
    );
    match client.get(&url).send().await {
        Ok(r) => {
            let markets: Vec<PolyMarket> = r.json().await.unwrap_or_default();
            let mut result: Vec<Market> = markets.iter().filter_map(|m| m.to_market()).collect();
            // Client-side text search since gamma API ignores _q
            if let Some(q) = query {
                let q_lower = q.to_lowercase();
                result.retain(|m| m.question.to_lowercase().contains(&q_lower));
            }
            result.truncate(limit);
            result
        }
        Err(e) => {
            eprintln!("Polymarket API error: {}", e);
            vec![]
        }
    }
}

async fn fetch_kalshi_list(
    client: &reqwest::Client,
    limit: usize,
    query: Option<&str>,
) -> Vec<Market> {
    if let Some(q) = query {
        // Search: try as series ticker first, then fetch events and search titles
        let q_upper = q.to_uppercase();
        let q_lower = q.to_lowercase();

        // Try direct series ticker match (e.g. "BTC" -> series_ticker=KXBTC)
        let series_url = format!(
            "{}/markets?limit={}&status=open&series_ticker=KX{}",
            KALSHI_BASE, limit, q_upper
        );
        if let Ok(resp) = client.get(&series_url).send().await {
            if let Ok(body) = resp.json::<KalshiMarketsResponse>().await {
                let markets: Vec<Market> = body
                    .markets
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|m| m.to_market())
                    .filter(|m| m.volume.parse::<i64>().unwrap_or(0) > 0 || m.yes_price > 0.0)
                    .collect();
                if !markets.is_empty() {
                    return markets;
                }
            }
        }

        // Fall back: fetch events and search by title
        let events_url = format!("{}/events?limit=100&status=open", KALSHI_BASE);
        if let Ok(resp) = client.get(&events_url).send().await {
            if let Ok(body) = resp.json::<KalshiEventsResponse>().await {
                let matching_events: Vec<&KalshiEvent> = body
                    .events
                    .as_ref()
                    .map(|events| {
                        events
                            .iter()
                            .filter(|e| {
                                let title = e.title.as_deref().unwrap_or("").to_lowercase();
                                let cat = e.category.as_deref().unwrap_or("").to_lowercase();
                                title.contains(&q_lower) || cat.contains(&q_lower)
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let mut all_markets = Vec::new();
                for event in matching_events.iter().take(5) {
                    if let Some(ticker) = &event.event_ticker {
                        let markets_url = format!(
                            "{}/markets?limit={}&status=open&event_ticker={}",
                            KALSHI_BASE, limit, ticker
                        );
                        if let Ok(resp) = client.get(&markets_url).send().await {
                            if let Ok(body) = resp.json::<KalshiMarketsResponse>().await {
                                let markets: Vec<Market> = body
                                    .markets
                                    .unwrap_or_default()
                                    .iter()
                                    .filter_map(|m| m.to_market())
                                    .collect();
                                all_markets.extend(markets);
                            }
                        }
                    }
                }
                if !all_markets.is_empty() {
                    return all_markets;
                }
            }
        }

        vec![]
    } else {
        // List: fetch popular events and get their top markets
        let events_url = format!("{}/events?limit=20&status=open", KALSHI_BASE);
        match client.get(&events_url).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.json::<KalshiEventsResponse>().await {
                    let mut all_markets = Vec::new();
                    let events = body.events.unwrap_or_default();

                    // Get 1-2 markets per event, skip sports parlays
                    for event in events.iter().take(limit) {
                        let title = event.title.as_deref().unwrap_or("");
                        // Skip sports multi-game parlays
                        if title.starts_with("yes ") || title.contains("wins by over") {
                            continue;
                        }
                        if let Some(ticker) = &event.event_ticker {
                            let markets_url = format!(
                                "{}/markets?limit=2&status=open&event_ticker={}",
                                KALSHI_BASE, ticker
                            );
                            if let Ok(resp) = client.get(&markets_url).send().await {
                                if let Ok(body) = resp.json::<KalshiMarketsResponse>().await {
                                    let markets: Vec<Market> = body
                                        .markets
                                        .unwrap_or_default()
                                        .iter()
                                        .filter_map(|m| m.to_market())
                                        .filter(|m| !m.question.starts_with("yes "))
                                        .collect();
                                    all_markets.extend(markets);
                                }
                            }
                        }
                        if all_markets.len() >= limit {
                            break;
                        }
                    }
                    all_markets.truncate(limit);
                    all_markets
                } else {
                    vec![]
                }
            }
            Err(e) => {
                eprintln!("Kalshi API error: {}", e);
                vec![]
            }
        }
    }
}

fn sort_by_volume(markets: &mut [Market]) {
    markets.sort_by(|a, b| {
        let va: f64 = a.volume.parse().unwrap_or(0.0);
        let vb: f64 = b.volume.parse().unwrap_or(0.0);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn print_market_list(markets: &[Market]) {
    println!();
    println!("  🔮 Prediction Markets");
    println!();
    if markets.is_empty() {
        println!("  No markets found.");
        println!();
        return;
    }
    for (i, m) in markets.iter().enumerate() {
        let tag = match m.platform.as_str() {
            "polymarket" => "[PM]".magenta(),
            "kalshi" => "[KA]".blue(),
            _ => "[??]".white(),
        };
        let yes_str = format!("{:.0}%", m.yes_price * 100.0);
        let no_str = format!("{:.0}%", m.no_price * 100.0);
        println!(
            "  {}. {} {}",
            (i + 1).to_string().bold(),
            tag,
            m.question.cyan()
        );
        println!(
            "     Yes: {}  No: {}  Vol: {}",
            yes_str.green(),
            no_str.red(),
            m.volume
        );
        println!("     ID: {}", m.id.dimmed());
        if let Some(ref end) = m.end_date {
            println!("     Ends: {}", end.dimmed());
        }
        println!();
    }
}

fn print_market_detail(m: &Market) {
    println!();
    println!("  🔮 Market Quote");
    println!();
    println!("  Platform:  {}", m.platform.cyan());
    println!("  ID:        {}", m.id);
    println!("  Question:  {}", m.question.bold());
    println!();
    let yes_str = format!("{:.1}%", m.yes_price * 100.0);
    let no_str = format!("{:.1}%", m.no_price * 100.0);
    println!("  Yes Price: {}", yes_str.green().bold());
    println!("  No Price:  {}", no_str.red().bold());
    println!("  Volume:    {}", m.volume);
    if let Some(ref liq) = m.liquidity {
        println!("  Liquidity: {}", liq);
    }
    if let Some(ref end) = m.end_date {
        println!("  End Date:  {}", end);
    }
    if let Some(ref outcomes) = m.outcomes {
        println!("  Outcomes:  {}", outcomes.join(", "));
    }
    println!("  URL:       {}", m.url.underline());
    println!();
}

// --- Public command handlers ---

pub async fn list(platform: &str, limit: usize, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let mut markets = match platform {
        "polymarket" => fetch_polymarket_list(&client, limit, None).await,
        "kalshi" => fetch_kalshi_list(&client, limit, None).await,
        _ => {
            let (pm, ka) = tokio::join!(
                fetch_polymarket_list(&client, limit, None),
                fetch_kalshi_list(&client, limit, None)
            );
            let mut all = pm;
            all.extend(ka);
            all
        }
    };
    sort_by_volume(&mut markets);
    markets.truncate(limit);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&markets)?);
    } else {
        print_market_list(&markets);
    }
    Ok(())
}

pub async fn search(query: &str, platform: &str, limit: usize, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let mut markets = match platform {
        "polymarket" => fetch_polymarket_list(&client, limit, Some(query)).await,
        "kalshi" => fetch_kalshi_list(&client, limit, Some(query)).await,
        _ => {
            let (pm, ka) = tokio::join!(
                fetch_polymarket_list(&client, limit, Some(query)),
                fetch_kalshi_list(&client, limit, Some(query))
            );
            let mut all = pm;
            all.extend(ka);
            all
        }
    };
    sort_by_volume(&mut markets);
    markets.truncate(limit);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&markets)?);
    } else {
        print_market_list(&markets);
    }
    Ok(())
}

pub async fn quote(market_id: &str, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let (platform, id) = market_id
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Format: polymarket:<slug> or kalshi:<TICKER>"))?;

    let market = match platform {
        "polymarket" => {
            let url = format!(
                "{}/markets?slug={}",
                POLYMARKET_BASE,
                urlencoding::encode(id)
            );
            let markets: Vec<PolyMarket> = client.get(&url).send().await?.json().await?;
            markets.first().and_then(|m| m.to_market())
        }
        "kalshi" => {
            let url = format!("{}/markets/{}", KALSHI_BASE, id);
            let resp: KalshiMarketResponse = client.get(&url).send().await?.json().await?;
            resp.market.and_then(|m| m.to_market())
        }
        _ => bail!("Unknown platform '{}'. Use polymarket or kalshi.", platform),
    };

    match market {
        Some(m) => {
            if json_output {
                println!("{}", serde_json::to_string_pretty(&m)?);
            } else {
                print_market_detail(&m);
            }
        }
        None => bail!("Market '{}' not found.", market_id),
    }
    Ok(())
}

pub async fn buy(
    market: &str,
    side: &str,
    amount: &str,
    max_price: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let (platform, slug) = market
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Format: polymarket:<slug> or kalshi:<TICKER>"))?;

    match platform {
        "polymarket" => polymarket_buy(slug, side, amount, max_price, json_output).await,
        "kalshi" => {
            // Kalshi trading still stubbed
            if json_output {
                println!(
                    "{}",
                    json!({
                        "action": "predict_buy", "market": market, "side": side,
                        "amount": amount, "maxPrice": max_price,
                        "status": "not_implemented",
                        "note": "Kalshi trading requires additional configuration."
                    })
                );
            } else {
                println!();
                println!("  🔮 Prediction Buy (Preview)");
                println!("  Market:    {}", market.cyan());
                println!("  Side:      {}", side);
                println!("  Amount:    {}", amount);
                if let Some(mp) = max_price {
                    println!("  Max Price: {}¢", mp);
                }
                println!();
                print_trading_config_hint(platform);
            }
            Ok(())
        }
        _ => bail!("Unknown platform '{}'", platform),
    }
}

pub async fn sell(
    market: &str,
    side: &str,
    amount: &str,
    min_price: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let (platform, slug) = market
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Format: polymarket:<slug> or kalshi:<TICKER>"))?;

    match platform {
        "polymarket" => polymarket_sell(slug, side, amount, min_price, json_output).await,
        "kalshi" => {
            // Kalshi trading still stubbed
            if json_output {
                println!(
                    "{}",
                    json!({
                        "action": "predict_sell", "market": market, "side": side,
                        "amount": amount, "minPrice": min_price,
                        "status": "not_implemented",
                        "note": "Kalshi trading requires additional configuration."
                    })
                );
            } else {
                println!();
                println!("  🔮 Prediction Sell (Preview)");
                println!("  Market:    {}", market.cyan());
                println!("  Side:      {}", side);
                println!("  Amount:    {}", amount);
                if let Some(mp) = min_price {
                    println!("  Min Price: {}¢", mp);
                }
                println!();
                print_trading_config_hint(platform);
            }
            Ok(())
        }
        _ => bail!("Unknown platform '{}'", platform),
    }
}

// --- Polymarket Trading Helpers ---

async fn polymarket_buy(
    slug: &str,
    side: &str,
    amount_str: &str,
    max_price_str: Option<&str>,
    json_output: bool,
) -> Result<()> {
    // Load wallet (supports both private_key and wallet_json + passcode)
    let hl_cfg = config::load_hl_config()?;
    let private_key = hl_cfg.private_key.clone();
    let address = hl_cfg.address.clone();

    let client = reqwest::Client::new();

    // Derive API credentials
    let (api_key, secret, passphrase) =
        polymarket::derive_api_credentials(&client, &private_key).await?;

    // Fetch market info
    let (token_ids, neg_risk) = polymarket::get_market_info(&client, slug).await?;

    // Determine which token to buy: YES = index 0, NO = index 1
    let token_idx = match side.to_lowercase().as_str() {
        "yes" => 0,
        "no" => 1,
        _ => bail!("Side must be 'yes' or 'no'"),
    };
    let token_id = token_ids
        .get(token_idx)
        .ok_or_else(|| anyhow::anyhow!("Token ID not found for side {}", side))?;

    // Get tick size
    let tick_size = polymarket::get_tick_size(&client, token_id).await?;

    // Parse amount (USDC to spend) and max_price (0-1 range, like 0.50)
    let amount_usdc: f64 = amount_str.parse()?;
    let max_price: f64 = max_price_str.map(|s| s.parse()).transpose()?.unwrap_or(1.0);

    // Round price to tick
    let limit_price = polymarket::round_to_tick(max_price, tick_size);

    // Calculate sizes in token decimals (6 decimals for both USDC and conditional tokens)
    // For a BUY order:
    // - makerAmount = USDC to spend (in 6 decimals)
    // - takerAmount = outcome tokens to receive (in 6 decimals)
    // - takerAmount = makerAmount / price
    let maker_amount_raw = (amount_usdc * 1_000_000.0).round() as u64;
    let taker_amount_raw = if limit_price > 0.0 {
        (amount_usdc / limit_price * 1_000_000.0).round() as u64
    } else {
        bail!("Price must be > 0");
    };

    // Build order
    let salt = uuid::Uuid::new_v4().as_u128().to_string();
    let order = polymarket::OrderData {
        salt,
        maker: address.clone(),
        signer: address.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: "0".to_string(),
        side: 0, // BUY
        signature_type: 0,
    };

    // Sign order
    let signature = polymarket::sign_order(&private_key, &order, neg_risk).await?;

    // Submit order
    let result = polymarket::post_order(
        &client,
        &api_key,
        &secret,
        &passphrase,
        &address,
        &order,
        &signature,
    )
    .await?;

    if json_output {
        println!(
            "{}",
            json!({
                "action": "predict_buy",
                "market": format!("polymarket:{}", slug),
                "side": side,
                "amount": amount_str,
                "maxPrice": limit_price,
                "orderId": result.order_id,
                "success": result.success,
                "error": result.error
            })
        );
    } else {
        println!();
        println!("  🔮 Polymarket Buy Order");
        println!("  Market:     polymarket:{}", slug.cyan());
        println!("  Side:       {}", side);
        println!("  Amount:     ${}", amount_str);
        println!("  Limit:      {:.4}", limit_price);
        println!("  Token ID:   {}", token_id.dimmed());
        println!();
        if let Some(true) = result.success {
            println!("  {} Order submitted!", "✓".green().bold());
            if let Some(ref oid) = result.order_id {
                println!("  Order ID:   {}", oid);
            }
        } else {
            println!("  {} Order failed", "✗".red().bold());
            if let Some(ref err) = result.error {
                println!("  Error:      {}", err);
            }
        }
        println!();
    }

    Ok(())
}

async fn polymarket_sell(
    slug: &str,
    side: &str,
    amount_str: &str,
    min_price_str: Option<&str>,
    json_output: bool,
) -> Result<()> {
    // Load wallet (supports both private_key and wallet_json + passcode)
    let hl_cfg = config::load_hl_config()?;
    let private_key = hl_cfg.private_key.clone();
    let address = hl_cfg.address.clone();

    let client = reqwest::Client::new();

    // Derive API credentials
    let (api_key, secret, passphrase) =
        polymarket::derive_api_credentials(&client, &private_key).await?;

    // Fetch market info
    let (token_ids, neg_risk) = polymarket::get_market_info(&client, slug).await?;

    // Determine which token to sell
    let token_idx = match side.to_lowercase().as_str() {
        "yes" => 0,
        "no" => 1,
        _ => bail!("Side must be 'yes' or 'no'"),
    };
    let token_id = token_ids
        .get(token_idx)
        .ok_or_else(|| anyhow::anyhow!("Token ID not found for side {}", side))?;

    // Get tick size
    let tick_size = polymarket::get_tick_size(&client, token_id).await?;

    // Parse amount (outcome tokens to sell) and min_price
    let amount_tokens: f64 = amount_str.parse()?;
    let min_price: f64 = min_price_str.map(|s| s.parse()).transpose()?.unwrap_or(0.0);

    // Round price to tick
    let limit_price = polymarket::round_to_tick(min_price, tick_size);

    // For a SELL order:
    // - makerAmount = outcome tokens to sell (in 6 decimals)
    // - takerAmount = USDC to receive (in 6 decimals)
    // - takerAmount = makerAmount * price
    let maker_amount_raw = (amount_tokens * 1_000_000.0).round() as u64;
    let taker_amount_raw = (amount_tokens * limit_price * 1_000_000.0).round() as u64;

    // Build order
    let salt = uuid::Uuid::new_v4().as_u128().to_string();
    let order = polymarket::OrderData {
        salt,
        maker: address.clone(),
        signer: address.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: "0".to_string(),
        side: 1, // SELL
        signature_type: 0,
    };

    // Sign order
    let signature = polymarket::sign_order(&private_key, &order, neg_risk).await?;

    // Submit order
    let result = polymarket::post_order(
        &client,
        &api_key,
        &secret,
        &passphrase,
        &address,
        &order,
        &signature,
    )
    .await?;

    if json_output {
        println!(
            "{}",
            json!({
                "action": "predict_sell",
                "market": format!("polymarket:{}", slug),
                "side": side,
                "amount": amount_str,
                "minPrice": limit_price,
                "orderId": result.order_id,
                "success": result.success,
                "error": result.error
            })
        );
    } else {
        println!();
        println!("  🔮 Polymarket Sell Order");
        println!("  Market:     polymarket:{}", slug.cyan());
        println!("  Side:       {}", side);
        println!("  Amount:     {} tokens", amount_str);
        println!("  Limit:      {:.4}", limit_price);
        println!("  Token ID:   {}", token_id.dimmed());
        println!();
        if let Some(true) = result.success {
            println!("  {} Order submitted!", "✓".green().bold());
            if let Some(ref oid) = result.order_id {
                println!("  Order ID:   {}", oid);
            }
        } else {
            println!("  {} Order failed", "✗".red().bold());
            if let Some(ref err) = result.error {
                println!("  Error:      {}", err);
            }
        }
        println!();
    }

    Ok(())
}

fn print_trading_config_hint(platform: &str) {
    match platform {
        "polymarket" => {
            println!(
                "  {} Trading on Polymarket requires wallet config.",
                "⚠️".yellow()
            );
            println!("  Set private_key in ~/.fintool/config.toml (trades on Polygon).");
        }
        "kalshi" => {
            println!(
                "  {} Trading on Kalshi requires API credentials.",
                "⚠️".yellow()
            );
            println!("  Set kalshi_api_key and kalshi_api_secret in ~/.fintool/config.toml");
        }
        _ => println!("  {} Unknown platform.", "⚠️".yellow()),
    }
    println!();
}
