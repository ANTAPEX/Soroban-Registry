# Search Ranking Methodology & Transparency

## 1. Overview
The Soroban Registry search engine ranks smart contracts using a deterministic multi-signal scoring model designed to surface high-quality, verified, and actively maintained contracts while giving developers transparent insight into ranking outcomes.

Through the `--explain` CLI flag and API explainability parameters, developers and publishers can inspect the exact breakdown of contributing signals for each search result.

---

## 2. Ranking Formula

The final search score is computed as the sum of five distinct ranking factors:

$$\text{Final Score} = \text{Text Relevance} + \text{Verification Bonus} + \text{Popularity Signal} + \text{Recency Decay} + \text{Deprecation Penalty}$$

$$\text{Score} = \max\left(0.00, \; R_{\text{text}} + B_{\text{ver}} + P_{\text{pop}} + D_{\text{rec}} + P_{\text{dep}}\right)$$

All factors and composite scores are rounded to two decimal places, guaranteeing that the reported factors sum exactly to the reported score:

$$\text{Score} \equiv \sum \text{Factors}$$

---

## 3. Contributing Ranking Factors

### 3.1. Text Relevance ($R_{\text{text}}$)
- **Range**: `0.10` to `5.00`
- **Purpose**: Measures term match density, field weighting, and proximity between user query terms and contract metadata.
- **Weights & Matching**:
  - **Exact Name / Slug Match**: `+3.10`
  - **Name Contains Full Query**: `+2.20`
  - **Token Match in Contract Name**: `+1.20` per term
  - **Token Match in Description**: `+0.50` per term
  - **Token Match in Category**: `+0.40` per term
  - **Default Query Match Baseline**: `0.50` (clamped to $[0.10, 5.00]$)
- When backed by PostgreSQL `ts_rank` or Elasticsearch BM25, the raw engine relevance score is normalized directly into this range.

### 3.2. Verification Bonus ($B_{\text{ver}}$)
- **Range**: `0.00` or `+2.00`
- **Purpose**: Elevates contracts whose source code has been verified on-chain against compiled WASM artifacts.
- **Rule**:
  $$B_{\text{ver}} = \begin{cases} +2.00 & \text{if } \text{is\_verified} = \text{true} \lor \text{status} = \text{Verified} \\ 0.00 & \text{otherwise} \end{cases}$$

### 3.3. Popularity Signal ($P_{\text{pop}}$)
- **Range**: `0.00` to `2.50`
- **Purpose**: Recognizes proven adoption based on cumulative usage and API access frequency.
- **Formula**:
  $$P_{\text{pop}} = \min\left(2.50, \; 0.45 \times \ln(\text{usage\_count} + 1)\right)$$
- **Sample Outputs**:
  - `0` accesses $\to 0.00$
  - `10` accesses $\to +1.08$
  - `50` accesses $\to +1.77$
  - `100` accesses $\to +2.08$
  - `500+` accesses $\to +2.50$ (saturation cap)

### 3.4. Recency Decay ($D_{\text{rec}}$)
- **Range**: `0.05` to `1.00`
- **Purpose**: Rewards actively maintained contracts and prevents stale or abandoned code from permanently dominating search rankings.
- **Formula**:
  $$D_{\text{rec}} = \max\left(0.05, \; \frac{1.0}{1.0 + \frac{\text{days\_since\_update}}{60.0}}\right)$$
- **Sample Outputs**:
  - Updated today ($0$ days) $\to +1.00$
  - Updated $30$ days ago $\to +0.67$
  - Updated $60$ days ago $\to +0.50$
  - Updated $180$ days ago $\to +0.25$
  - Updated $365$ days ago $\to +0.14$

### 3.5. Deprecation Penalty ($P_{\text{dep}}$)
- **Range**: `-1.00` or `0.00`
- **Purpose**: Deprioritizes contracts that publishers have flagged as deprecated or superseded in favor of modern implementations.
- **Rule**:
  $$P_{\text{dep}} = \begin{cases} -1.00 & \text{if } \text{is\_deprecated} = \text{true} \lor \text{deprecation\_status} \in \{\text{Deprecated}, \text{Superseded}\} \\ 0.00 & \text{otherwise (Active)} \end{cases}$$

---

## 4. CLI Usage Examples

### 4.1. Structured JSON Output (`--explain --json`)
```bash
soroban-registry search "token contract" --explain --json
```

Output:
```json
{
  "results": [
    {
      "contract_id": "CA3D5KRYMCMUG7JWQYYM2C72W45Z5L6TNR57NHRFFZHR4G5HRPVPM3M7",
      "score": 8.42,
      "factors": {
        "text_relevance": 3.10,
        "verification_bonus": 2.00,
        "popularity": 1.80,
        "recency": 0.52,
        "deprecation_penalty": -1.00
      }
    }
  ]
}
```

### 4.2. Human Tree Breakdown
```bash
soroban-registry search "token contract" --explain
```

Output:
```text
Search Ranking Transparency & Explainability (--explain):
================================================================================
Formula: Score = Text Relevance + Verification Bonus + Popularity + Recency + Deprecation Penalty

1. Soroban Token Contract (CA3D5KRYMCMUG7JWQYYM2C72W45Z5L6TNR57NHRFFZHR4G5HRPVPM3M7)
   Total Score: 8.42
   ├── Text Relevance:      +3.10  (query term match density)
   ├── Verification Bonus:  +2.00  (verified contract)
   ├── Popularity Signal:   +1.80  (usage count: 54)
   ├── Recency Decay:       +0.52  (updated 55d ago)
   └── Deprecation Penalty: -1.00  (deprecated)
```

---

## 5. Performance Guarantees
- **Zero Overhead on Default Search**: When `--explain` is omitted, factor computation is completely bypassed, preserving low-latency query throughput.
- **Predictable Attribution**: Factors strictly sum to the total score with no hidden constants or stochastic weights.
