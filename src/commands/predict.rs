use std::str::FromStr;

use anyhow::{bail, Context, Result};
use chrono::{Days, Utc};
use colored::Colorize;
use serde_json::json;

use crate::polymarket;

pub async fn list(
    query: Option<&str>,
    limit: i32,
    _active: Option<bool>,
    sort: Option<&str>,
    min_end_days: i64,
    json: bool,
) -> Result<()> {
    let client = polymarket::create_gamma_client();
    let min_end_date = Utc::now()
        .checked_add_days(Days::new(min_end_days.max(0) as u64))
        .unwrap();

    if let Some(q) = query {
        use polymarket_client_sdk::gamma::types::request::SearchRequest;
        let results = if let Some(s) = sort {
            let req = SearchRequest::builder().q(q).sort(s.to_string()).build();
            client.search(&req).await?
        } else {
            let req = SearchRequest::builder().q(q).build();
            client.search(&req).await?
        };

        // Extract markets from events, filtering out closed and soon-ending markets
        let min_end_naive = min_end_date.date_naive();
        let mut markets = Vec::new();
        if let Some(events) = &results.events {
            for event in events {
                if let Some(ref mks) = event.markets {
                    for m in mks {
                        if m.closed == Some(true) {
                            continue;
                        }
                        if let Some(end) = m.end_date_iso {
                            if end < min_end_naive {
                                continue;
                            }
                        }
                        markets.push(m);
                    }
                }
            }
        }
        let markets: Vec<_> = markets.into_iter().take(limit as usize).collect();

        if json {
            let items: Vec<serde_json::Value> = markets.iter().map(|m| market_to_json(m)).collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"markets": items}))?
            );
        } else {
            if markets.is_empty() {
                println!("{}", "No markets found.".yellow());
                return Ok(());
            }
            for m in &markets {
                print_market_human(m);
            }
        }
    } else {
        use polymarket_client_sdk::gamma::types::request::MarketsRequest;
        let mut req = MarketsRequest::default();
        req.limit = Some(limit);
        req.end_date_min = Some(min_end_date);
        req.closed = Some(false);
        if let Some(s) = sort {
            req.order = Some(s.to_string());
        }
        let markets = client.markets(&req).await?;

        if json {
            let items: Vec<serde_json::Value> = markets.iter().map(market_to_json).collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"markets": items}))?
            );
        } else {
            if markets.is_empty() {
                println!("{}", "No markets found.".yellow());
                return Ok(());
            }
            for m in &markets {
                print_market_human(m);
            }
        }
    }
    Ok(())
}

pub async fn quote(market: &str, json: bool) -> Result<()> {
    let client = polymarket::create_gamma_client();
    let m = resolve_market(&client, market).await?;

    if json {
        let mut v = market_to_json(&m);
        if let Some(obj) = v.as_object_mut() {
            obj.insert("id".to_string(), json!(m.id));
            obj.insert(
                "clob_token_ids".to_string(),
                json!(m
                    .clob_token_ids
                    .as_ref()
                    .map(|ids| ids.iter().map(|id| format!("{}", id)).collect::<Vec<_>>())),
            );
        }
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        print_market_human(&m);
        if let Some(ref ids) = m.clob_token_ids {
            let id_strs: Vec<String> = ids.iter().map(|id| format!("{}", id)).collect();
            println!("  Token IDs: {}", id_strs.join(", "));
        }
        if let Some(ref desc) = m.description {
            println!("  Description: {}", desc);
        }
    }
    Ok(())
}

pub async fn buy(market: &str, outcome: &str, amount: &str, price: &str, json: bool) -> Result<()> {
    use polymarket_client_sdk::auth::Signer as _;
    use polymarket_client_sdk::clob::types::Side;

    let clob = polymarket::create_clob_client().await?;
    let gamma_client = polymarket::create_gamma_client();
    let (token_id, _m) = resolve_market_token(&gamma_client, market, outcome).await?;

    let price_dec = rust_decimal::Decimal::from_str(price).context("Invalid price")?;
    let size_dec = rust_decimal::Decimal::from_str(amount).context("Invalid amount")?;

    let signable = clob
        .limit_order()
        .token_id(token_id)
        .side(Side::Buy)
        .price(price_dec)
        .size(size_dec)
        .build()
        .await?;

    let (key, _) = crate::config::polymarket_credentials()?;
    let signer = polymarket_client_sdk::auth::LocalSigner::from_str(&key)
        .context("Invalid Polymarket private key")?
        .with_chain_id(Some(polymarket_client_sdk::POLYGON));
    let signed = clob.sign(&signer, signable).await?;
    let response = clob.post_order(signed).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "predict_buy",
                "market": market,
                "outcome": outcome,
                "amount": amount,
                "price": price,
                "response": format!("{:?}", response),
            }))?
        );
    } else {
        println!("{}", "Buy order placed!".green().bold());
        println!("  Market: {}", market);
        println!("  Outcome: {}", outcome);
        println!("  Amount: {} USDC", amount);
        println!("  Price: {}", price);
        println!("  Response: {:?}", response);
    }
    Ok(())
}

pub async fn sell(
    market: &str,
    outcome: &str,
    amount: &str,
    price: &str,
    json: bool,
) -> Result<()> {
    use polymarket_client_sdk::auth::Signer as _;
    use polymarket_client_sdk::clob::types::Side;

    let clob = polymarket::create_clob_client().await?;
    let gamma_client = polymarket::create_gamma_client();
    let (token_id, _m) = resolve_market_token(&gamma_client, market, outcome).await?;

    let price_dec = rust_decimal::Decimal::from_str(price).context("Invalid price")?;
    let size_dec = rust_decimal::Decimal::from_str(amount).context("Invalid amount")?;

    let signable = clob
        .limit_order()
        .token_id(token_id)
        .side(Side::Sell)
        .price(price_dec)
        .size(size_dec)
        .build()
        .await?;

    let (key, _) = crate::config::polymarket_credentials()?;
    let signer = polymarket_client_sdk::auth::LocalSigner::from_str(&key)
        .context("Invalid Polymarket private key")?
        .with_chain_id(Some(polymarket_client_sdk::POLYGON));
    let signed = clob.sign(&signer, signable).await?;
    let response = clob.post_order(signed).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "action": "predict_sell",
                "market": market,
                "outcome": outcome,
                "amount": amount,
                "price": price,
                "response": format!("{:?}", response),
            }))?
        );
    } else {
        println!("{}", "Sell order placed!".green().bold());
        println!("  Market: {}", market);
        println!("  Outcome: {}", outcome);
        println!("  Amount: {} shares", amount);
        println!("  Price: {}", price);
        println!("  Response: {:?}", response);
    }
    Ok(())
}

pub async fn positions(json: bool) -> Result<()> {
    let address = polymarket::get_polymarket_address()?;
    let client = polymarket::create_data_client();

    use polymarket_client_sdk::data::types::request::PositionsRequest;
    let addr = alloy::primitives::Address::from_str(&address).context("Invalid address")?;
    let req = PositionsRequest::builder().user(addr).build();
    let positions = client.positions(&req).await?;

    if json {
        let items: Vec<serde_json::Value> = positions
            .iter()
            .map(|p| json!({ "position": format!("{:?}", p) }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "address": address,
                "positions": items,
            }))?
        );
    } else {
        println!("{} {}", "Positions for:".green().bold(), address);
        if positions.is_empty() {
            println!("  No open positions.");
        } else {
            for p in &positions {
                println!("  {:?}", p);
            }
        }
    }
    Ok(())
}

// ---- helpers ----

fn market_to_json(m: &polymarket_client_sdk::gamma::types::response::Market) -> serde_json::Value {
    json!({
        "question": m.question,
        "slug": m.slug,
        "condition_id": m.condition_id.map(|c| format!("{:?}", c)),
        "active": m.active,
        "closed": m.closed,
        "volume": m.volume.map(|v| v.to_string()),
        "liquidity": m.liquidity.map(|v| v.to_string()),
        "outcomes": m.outcomes,
        "outcome_prices": m.outcome_prices.as_ref().map(|ps| ps.iter().map(|p| p.to_string()).collect::<Vec<String>>()),
        "end_date": m.end_date_iso.map(|d| d.to_string()),
        "description": m.description,
    })
}

fn print_market_human(m: &polymarket_client_sdk::gamma::types::response::Market) {
    println!(
        "{} {}",
        "Market:".green().bold(),
        m.question.as_deref().unwrap_or("(untitled)")
    );
    if let Some(ref slug) = m.slug {
        println!("  Slug: {}", slug);
    }
    if let Some(ref outcomes) = m.outcomes {
        println!("  Outcomes: {}", outcomes.join(", "));
    }
    if let Some(ref prices) = m.outcome_prices {
        let price_strs: Vec<String> = prices.iter().map(|p| p.to_string()).collect();
        println!("  Prices: {}", price_strs.join(", "));
    }
    if let Some(ref vol) = m.volume {
        println!("  Volume: ${}", vol);
    }
    if let Some(ref liq) = m.liquidity {
        println!("  Liquidity: ${}", liq);
    }
    if let Some(end) = m.end_date_iso {
        println!("  End date: {}", end);
    }
    println!();
}

async fn resolve_market(
    client: &polymarket_client_sdk::gamma::Client,
    market: &str,
) -> Result<polymarket_client_sdk::gamma::types::response::Market> {
    use polymarket_client_sdk::gamma::types::request::{MarketByIdRequest, MarketBySlugRequest};
    let req = MarketBySlugRequest::builder().slug(market).build();
    match client.market_by_slug(&req).await {
        Ok(m) => Ok(m),
        Err(_) => {
            let req = MarketByIdRequest::builder().id(market).build();
            client
                .market_by_id(&req)
                .await
                .context("Market not found by slug or ID")
        }
    }
}

async fn resolve_market_token(
    client: &polymarket_client_sdk::gamma::Client,
    market: &str,
    outcome: &str,
) -> Result<(
    polymarket_client_sdk::types::U256,
    polymarket_client_sdk::gamma::types::response::Market,
)> {
    let m = resolve_market(client, market).await?;

    let outcome_lower = outcome.to_lowercase();
    let outcomes = m
        .outcomes
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Market has no outcomes"))?;
    let token_ids = m
        .clob_token_ids
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Market has no CLOB token IDs"))?;

    if outcomes.len() != token_ids.len() {
        bail!(
            "Outcomes count ({}) doesn't match token IDs count ({})",
            outcomes.len(),
            token_ids.len()
        );
    }

    for (i, o) in outcomes.iter().enumerate() {
        if o.to_lowercase() == outcome_lower {
            return Ok((token_ids[i], m));
        }
    }

    bail!("Outcome '{}' not found. Available: {:?}", outcome, outcomes)
}
