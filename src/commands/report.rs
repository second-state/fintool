use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

const USER_AGENT: &str = "fintool contact@fintool.dev";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Filing {
    form: String,
    filing_date: String,
    report_date: String,
    accession_number: String,
    primary_document: String,
    url: String,
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent(USER_AGENT).build()?)
}

async fn resolve_cik(symbol: &str) -> Result<(u64, String)> {
    let c = client()?;
    let body: HashMap<String, Value> = c
        .get("https://www.sec.gov/files/company_tickers.json")
        .send()
        .await?
        .json()
        .await?;

    let sym_upper = symbol.to_uppercase();
    for v in body.values() {
        if v.get("ticker").and_then(|t| t.as_str()) == Some(sym_upper.as_str()) {
            let cik = v["cik_str"].as_u64().unwrap();
            let title = v["title"].as_str().unwrap_or("").to_string();
            return Ok((cik, title));
        }
    }
    Err(anyhow!("Ticker '{}' not found in SEC EDGAR", symbol))
}

async fn get_filings(cik: u64, form_type: Option<&str>, limit: usize) -> Result<Vec<Filing>> {
    let c = client()?;
    let url = format!("https://data.sec.gov/submissions/CIK{:010}.json", cik);
    let body: Value = c.get(&url).send().await?.json().await?;

    let recent = &body["filings"]["recent"];
    let forms = recent["form"]
        .as_array()
        .ok_or_else(|| anyhow!("No filings found"))?;
    let filing_dates = recent["filingDate"].as_array().unwrap();
    let accessions = recent["accessionNumber"].as_array().unwrap();
    let primary_docs = recent["primaryDocument"].as_array().unwrap();
    let report_dates = recent["reportDate"].as_array().unwrap();

    let mut filings = Vec::new();
    for i in 0..forms.len() {
        let form = forms[i].as_str().unwrap_or("");
        if let Some(ft) = form_type {
            if form != ft {
                continue;
            }
        }
        let accession = accessions[i].as_str().unwrap_or("");
        let primary_doc = primary_docs[i].as_str().unwrap_or("");
        let acc_no_dashes = accession.replace('-', "");
        let url = format!(
            "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
            cik, acc_no_dashes, primary_doc
        );
        filings.push(Filing {
            form: form.to_string(),
            filing_date: filing_dates[i].as_str().unwrap_or("").to_string(),
            report_date: report_dates[i].as_str().unwrap_or("").to_string(),
            accession_number: accession.to_string(),
            primary_document: primary_doc.to_string(),
            url,
        });
        if filings.len() >= limit {
            break;
        }
    }
    Ok(filings)
}

async fn fetch_filing_text(cik: u64, accession: &str, primary_doc: &str) -> Result<String> {
    let c = client()?;
    let acc_no_dashes = accession.replace('-', "");
    let url = format!(
        "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
        cik, acc_no_dashes, primary_doc
    );
    let html = c.get(&url).send().await?.text().await?;
    Ok(html_to_text(&html))
}

fn html_to_text(html: &str) -> String {
    let mut s = html.to_string();
    // Remove script/style blocks
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    s = re_script.replace_all(&s, "").to_string();
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    s = re_style.replace_all(&s, "").to_string();

    // Newlines for block elements
    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    s = re_br.replace_all(&s, "\n").to_string();
    for tag in &["p", "div", "tr", "h1", "h2", "h3", "h4", "h5", "h6", "li"] {
        let re = Regex::new(&format!(r"(?i)</?{}\b[^>]*>", tag)).unwrap();
        s = re.replace_all(&s, "\n").to_string();
    }
    // Tabs for cells
    for tag in &["td", "th"] {
        let re = Regex::new(&format!(r"(?i)</?{}\b[^>]*>", tag)).unwrap();
        s = re.replace_all(&s, "\t").to_string();
    }
    // Strip remaining tags
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    s = re_tags.replace_all(&s, "").to_string();

    // Decode entities
    s = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ");

    // Collapse blank lines
    let re_blanks = Regex::new(r"\n{3,}").unwrap();
    s = re_blanks.replace_all(&s, "\n\n").to_string();
    s.trim().to_string()
}

async fn fetch_and_output(
    symbol: &str,
    form_type: &str,
    output: Option<&str>,
    json: bool,
) -> Result<()> {
    let (cik, company) = resolve_cik(symbol).await?;
    let filings = get_filings(cik, Some(form_type), 1).await?;
    let filing = filings
        .first()
        .ok_or_else(|| anyhow!("No {} filing found for {}", form_type, symbol))?;
    let text = fetch_filing_text(cik, &filing.accession_number, &filing.primary_document).await?;

    if json {
        let out = json!({
            "symbol": symbol.to_uppercase(),
            "company": company,
            "form": filing.form,
            "filingDate": filing.filing_date,
            "reportDate": filing.report_date,
            "accessionNumber": filing.accession_number,
            "url": filing.url,
            "text": text,
            "textLength": text.len(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        use colored::Colorize;
        println!(
            "{} {} ({})",
            form_type.bold(),
            company.bold(),
            symbol.to_uppercase()
        );
        println!(
            "Filed: {}  Period: {}",
            filing.filing_date, filing.report_date
        );
        println!("Accession: {}", filing.accession_number);
        println!("URL: {}\n", filing.url);

        if let Some(path) = output {
            std::fs::write(path, &text)?;
            println!("Full report saved to: {}", path.green());
        } else {
            let truncated = if text.len() > 5000 {
                format!(
                    "{}...\n\n[Truncated — {} total chars. Use --output to save full report]",
                    &text[..5000],
                    text.len()
                )
            } else {
                text
            };
            println!("{}", truncated);
        }
    }
    Ok(())
}

pub async fn annual(symbol: &str, output: Option<&str>, json: bool) -> Result<()> {
    fetch_and_output(symbol, "10-K", output, json).await
}

pub async fn quarterly(symbol: &str, output: Option<&str>, json: bool) -> Result<()> {
    fetch_and_output(symbol, "10-Q", output, json).await
}

pub async fn list(symbol: &str, limit: usize, json: bool) -> Result<()> {
    let (cik, company) = resolve_cik(symbol).await?;
    let filings = get_filings(cik, None, limit).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&filings)?);
    } else {
        use colored::Colorize;
        println!(
            "{} recent filings for {} ({}):\n",
            limit,
            company.bold(),
            symbol.to_uppercase()
        );
        println!("Form     Filed        Period       Accession Number");
        println!("{}", "-".repeat(70));
        for f in &filings {
            println!(
                "{:<8} {:<12} {:<12} {}",
                f.form, f.filing_date, f.report_date, f.accession_number
            );
        }
    }
    Ok(())
}

pub async fn get(symbol: &str, accession: &str, output: Option<&str>, json: bool) -> Result<()> {
    let (cik, company) = resolve_cik(symbol).await?;
    let filings = get_filings(cik, None, 100).await?;
    let filing = filings
        .iter()
        .find(|f| f.accession_number == accession)
        .ok_or_else(|| anyhow!("Filing with accession '{}' not found", accession))?;
    let text = fetch_filing_text(cik, accession, &filing.primary_document).await?;

    if json {
        let out = json!({
            "symbol": symbol.to_uppercase(),
            "company": company,
            "form": filing.form,
            "filingDate": filing.filing_date,
            "reportDate": filing.report_date,
            "accessionNumber": filing.accession_number,
            "url": filing.url,
            "text": text,
            "textLength": text.len(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        use colored::Colorize;
        println!(
            "{} {} ({})",
            filing.form.bold(),
            company.bold(),
            symbol.to_uppercase()
        );
        println!(
            "Filed: {}  Period: {}",
            filing.filing_date, filing.report_date
        );
        println!("Accession: {}", filing.accession_number);
        println!("URL: {}\n", filing.url);

        if let Some(path) = output {
            std::fs::write(path, &text)?;
            println!("Full report saved to: {}", path.green());
        } else {
            let truncated = if text.len() > 5000 {
                format!(
                    "{}...\n\n[Truncated — {} total chars. Use --output to save full report]",
                    &text[..5000],
                    text.len()
                )
            } else {
                text
            };
            println!("{}", truncated);
        }
    }
    Ok(())
}
