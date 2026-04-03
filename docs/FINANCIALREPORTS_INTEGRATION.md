# Data Source Suggestion: FinancialReports.eu for Global Regulatory Filings

## Overview

[FinancialReports.eu](https://financialreports.eu) provides API access to **14M+ filings** from data sources across 30+ countries. Since fintool already supports SEC filings, adding FinancialReports.eu extends coverage to 30+ countries globally.

## Why This Fits fintool

fintool already has US filing support. FinancialReports.eu adds:

- **30+ additional countries** including UK, EU, Japan, South Korea, Switzerland
- **33,000+ companies** globally with ISIN identifiers
- **Standardized categories** — 11 filing categories across all countries (Financial Reporting, ESG, M&A, Debt/Equity, etc.)
- **Markdown endpoint** — `GET /filings/{id}/markdown/` for LLM-ready text
- **Simple REST API** — easy to integrate from Rust (HTTP + JSON)
- **MCP server** — for AI agent integration

## Integration Approaches

### 1. REST API Integration

Simple HTTP requests with API key auth:

```bash
# Search for a company
curl -H "X-API-Key: your-api-key" \
  "https://api.financialreports.eu/companies/?search=Samsung&page_size=5"

# Fetch filings
curl -H "X-API-Key: your-api-key" \
  "https://api.financialreports.eu/filings/?company_isin=KR7005930003&categories=2&page_size=10"

# Get filing as Markdown
curl -H "X-API-Key: your-api-key" \
  "https://api.financialreports.eu/filings/12345/markdown/"
```

### 2. MCP Server

FinancialReports.eu offers an [MCP server](https://financialreports.eu) for AI agent integration — complementing fintool's agentic trading capabilities.

### 3. Python SDK (for tooling/scripts)

```bash
pip install financial-reports-generated-client
```

```python
from financial_reports_client import Client
from financial_reports_client.api.filings import filings_list

client = Client(base_url="https://api.financialreports.eu")
client = client.with_headers({"X-API-Key": "your-api-key"})

filings = filings_list.sync(client=client, company_isin="KR7005930003", categories="2")
```

## API Details

| Property | Value |
|---|---|
| **Base URL** | `https://api.financialreports.eu` |
| **API Docs** | [docs.financialreports.eu](https://docs.financialreports.eu/) |
| **Authentication** | API key via `X-API-Key` header |
| **Rate Limiting** | Burst limit + monthly quota |
| **Format** | REST JSON (Markdown for filing content) |
| **Companies** | 33,230+ |
| **Total Filings** | 14,135,359+ |
| **Coverage** | 30+ countries |

## Coverage Comparison

| fintool (current) | + FinancialReports.eu |
|---|---|
| US filings | 30+ countries |
| US companies | 33,000+ global companies |
| — | 11 standardized filing categories |
| — | Markdown text extraction |
| — | ESG, M&A, management filings |
| — | MCP server for AI agents |
