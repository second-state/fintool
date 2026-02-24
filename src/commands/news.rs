use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};

pub async fn run(symbol: &str, json_output: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let symbol_upper = symbol.to_uppercase();

    let articles = fetch_google_news(&client, &symbol_upper).await?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&articles)?);
    } else {
        print_news(&symbol_upper, &articles);
    }

    Ok(())
}

async fn fetch_google_news(client: &reqwest::Client, symbol: &str) -> Result<Vec<Value>> {
    let url = format!("https://news.google.com/rss/search?q={}+stock", symbol);
    let text = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .context("Failed to fetch Google News")?
        .text()
        .await?;

    // Simple XML parsing for RSS items
    let mut articles = Vec::new();
    for item_block in text.split("<item>").skip(1).take(10) {
        let title = extract_xml_tag(item_block, "title").unwrap_or_default();
        let link = extract_xml_tag(item_block, "link").unwrap_or_default();
        let source = extract_xml_tag(item_block, "source").unwrap_or_default();
        let pub_date = extract_xml_tag(item_block, "pubDate").unwrap_or_default();

        articles.push(json!({
            "title": title,
            "source": source,
            "url": link,
            "published": pub_date,
        }));
    }

    Ok(articles)
}

fn extract_xml_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)?;
    let after_open = &text[start..];
    let content_start = after_open.find('>')? + 1;
    let content = &after_open[content_start..];
    let end = content.find(&close)?;
    let value = &content[..end];
    // Strip CDATA
    let value = value
        .trim()
        .strip_prefix("<![CDATA[")
        .and_then(|v| v.strip_suffix("]]>"))
        .unwrap_or(value.trim());
    Some(value.to_string())
}

fn print_news(symbol: &str, articles: &[Value]) {
    println!();
    println!("  📰 News for {}", symbol.bold().cyan());
    println!();

    if articles.is_empty() {
        println!("  No news found.");
        return;
    }

    for (i, article) in articles.iter().enumerate() {
        let title = article["title"].as_str().unwrap_or("");
        let source = article["source"].as_str().unwrap_or("");
        let url = article["url"].as_str().unwrap_or("");
        let published = article["published"].as_str().unwrap_or("");
        let time = crate::format::time_ago(published);

        println!(
            "  {}. {} {}",
            (i + 1).to_string().bold(),
            title,
            format!("({})", time).dimmed()
        );
        if !source.is_empty() {
            println!("     {}", source.dimmed());
        }
        println!("     {}", url.blue().underline());
        println!();
    }
}
