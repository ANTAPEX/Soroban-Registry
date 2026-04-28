# ML Vulnerability Detector

This directory contains the baseline ML vulnerability detector used by the backend ML scan path.

## What it does

- Trains a lightweight Naive Bayes-style classifier from JSONL samples exported by `tools/export_security_dataset.py`.
- Scores contract source code and returns an explainable report with:
  - score and grade
  - top predicted vulnerability labels
  - line-level evidence hints
  - suggested remediation

## Files

- `bin/train.js` - train a model from JSONL samples.
- `bin/scan.js` - score one source file or a JSONL dataset.
- `lib/model.js` - shared training and inference logic.

## Train

```bash
node bin/train.js /path/to/dataset.jsonl /path/to/model.json
```

## Scan

```bash
node bin/scan.js /path/to/model.json /path/to/source.rs
```

## Data export

Export training data from Postgres first:

```bash
DATABASE_URL=postgresql://user:pass@host:5432/dbname python3 tools/export_security_dataset.py --out dataset.jsonl
```

## Notes

- The detector is intentionally simple and explainable so it can bootstrap the ML pipeline immediately.
- It is designed to consume verified source code and scan/issue labels from the existing security scanning tables.
