# Trading_agents

The architecture of `trading_agents.py` follows `TradingAgents.md`.

### Phase Hierarchy

| Phase | Designation | Agents | Model | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Phase I** | **Eyes** | Fundamental, Technical, Sentiment, Macro | `claude-haiku-4-5` | Fast, structured data-to-text |
| **Phase II** | **Brain** | Bull, Bear, (Facilitator loop) | `claude-opus-4-6` + Adaptive Thinking | Dialectical debate, N rounds |
| **Phase III** | **Hands** | Trader, Risk ×3, Fund Manager | `claude-opus-4-6` + Adaptive Thinking | Weighted vote, self-reflection |

---

### Key Design Decisions

* **Global State Management**: State is maintained as a Python `dataclass`, JSON-serialized as context for each agent to prevent hallucination drift.
* **Reasoning Framework**: ReAct is embedded in every system prompt ("reason before concluding").
* **Stress Testing**: Facilitator's mandatory question (*"What makes this bullish signal a trap?"*) is hardcoded into the Bear prompt for each round.
* **Execution Safety**: 
    * Fund Manager self-reflection is mandatory in the system prompt.
    * A `no_trade_reason` field surfaces automatically when self-reflection triggers a halt.
    * Confidence threshold set at **0.35**; any value below this defaults to "No Trade" regardless of suggested action.
* **Model Optimization**: `Haiku 4.5` for Phase I (high speed/no thinking); `Opus 4.6` + Adaptive Thinking for high-stakes reasoning in Phases II/III.

---

### Usage

**Environment Setup:**
```bash
export ANTHROPIC_API_KEY=sk-...
pip install anthropic yfinance pandas
```

**Dry Run (Default):**
```bash
python trading_agents.py BTC --exchange hyperliquid --portfolio 1000
```

**Full Pipeline & Trade Submission:**
```bash
python trading_agents.py ETH --exchange binance --market perp --portfolio 5000 --rounds 3 --execute
```

**Save State:**
```bash
python trading_agents.py HYPE --exchange hyperliquid --output state.json
```

---

### Run Simulation

`cd trading_agents`

#### Single scenario
```bash
python simulate.py btc # BTC/Hyperliquid spot → BUY
  
python simulate.py eth # ETH/Binance perp → HOLD
  
python simulate.py hype # HYPE/Hyperliquid spot → SELL
```

#### All three in sequence
```bash
python simulate.py all
```

#### Skip streaming delays (instant output)
```bash
python simulate.py btc --fast

python simulate.py all --fast
```

No API keys or network access required — everything runs on mock data.