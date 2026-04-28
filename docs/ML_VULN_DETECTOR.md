# ML-based Vulnerability Detector — Design Doc

Status: Draft

## Goal

Provide an automated machine-learning based detector that scans smart contract source (and related artifacts) to surface likely vulnerabilities, risk signals, and high-confidence findings to assist reviewers and automated CI. The detector should be incremental (scores + categories), explainable (high-level reasons), and safe (low false-positive impact on automation).

## Scope
- Input artifacts: contract source (Rust/TX/other), compiled WASM/bytecode, ABI, contract metadata, OpenAPI/ABI if available, historical audit records, test results, and runtime metrics.
- Output: vulnerability score (0-1), top-k vulnerability labels (e.g. reentrancy, integer overflow, access control), supporting evidence (code spans, tokens, features), confidence, and suggested remediation links.

## Data sources & labeling
- Harvest existing public vulnerability datasets (Research datasets for smart contracts), internal audit logs, `frontend`/`backend` historical security issues, CVE-like records if present.
- Labeling approach:
  - Start with rule-based labels (heuristics + static analyzers) to bootstrap weak labels.
  - Curate a small gold standard labeled set from audits (10s-100s) for validation and fine-tuning.
  - Use active learning and human-in-the-loop labeling to expand dataset.

## Feature engineering / representations
- Multi-input representations: raw source tokens, AST/CFG extracts, compiled bytecode features, symbolic features (uses of unsafe ops), call graphs, dependency graphs, and metadata (author, compiler version).
- Candidate features:
  - Token n-grams, embeddings (CodeBERT/Starcoder embeddings)
  - AST path features (code2vec/code2seq style)
  - Graph features via GNN on call graph or CFG
  - Binary features from static analyzers (linters, slither-like results)

## Model candidates
- Baseline: logistic regression / XGBoost on engineered features.
- Sequence model: fine-tune code model (CodeBERT, StarCoder) for classification.
- Graph model: GNN over AST/CFG for structural vulnerabilities.
- Ensemble: combine static-analyzer signals + learned model for improved precision.

## Training & evaluation
- Metrics: precision@k (high importance), recall, F1, ROC-AUC, calibrated confidence. Prefer high-precision operating points for automated gating.
- Cross-validation, holdout test set from gold-labeled audits, and adversarial robustness checks (obfuscated code, renamed identifiers).

## Pipeline & infra
- Data ingestion: collectors that pull from repository sources, `backend` artefacts, and past audit logs. Store raw artifacts in S3-like storage.
- Preprocessing: tokenization, AST extraction (language-specific), bytecode parsing, static analysis runs.
- Training: use GPU instances; containerized training jobs (Docker/TF/PyTorch). Store models in an artifact registry.
- Inference: provide a REST endpoint (fast HTTP + batching) or serverless function; also provide a background batch scanner (cron) for repository scans.
- CI integration: add a `security-scan` job that calls the inference endpoint and fails or annotates PRs based on thresholds.

## Backend integration points
- New backend route: `/api/security/scan` — submit contract artifacts; returns report.
- Batch job: background scanner that enqueues contracts for scanning (topic/queue) and stores results in DB and emits events.
- UI: display vulnerability reports on contract pages and review workflows; provide quick triage to mark findings as true/false and feed back labels.

## Explainability & triage
- Return code spans or AST node references as evidence.
- Provide provenance: which features produced the score (e.g., flagged by static analyzer X, high token similarity to known vulnerable pattern Y).
- Allow human reviewers to accept/reject and send feedback to retraining pipeline.

## Monitoring & retraining
- Monitor model drift (data distribution), false-positive rate, and label drift. Expose metrics: scans/sec, avg score, FP rate.
- Retraining cadence: weekly/biweekly depending on data volume; support on-demand retrain after major label additions.

## Privacy, safety, and security
- Use access control on model APIs. Sanitize artifacts before sending to third-party services. Ensure models do not exfiltrate sensitive keys (strip secrets in preprocessing).

## Minimal viable product (MVP)
1. Define taxonomy and collect 500-2k weakly labeled examples (heuristics + static analyzer).
2. Build data pipeline and implement baseline XGBoost classifier over engineered features.
3. Expose `/api/security/scan` for single-contract inference and add UI report page.
4. Human-in-the-loop labeling workflow to collect feedback.

## Dependencies & cost estimate
- Libraries: PyTorch/TensorFlow, HuggingFace transformers, scikit-learn, networkx, tree-sitter (for parsing), Docker, S3 or object storage, Redis/RabbitMQ for queueing.
- Estimated initial infra: one GPU training node, small inference service (2-3 replicas), storage for artifacts and dataset.

## Timeline & milestones (rough)
- Week 0: finalize taxonomy, collect initial dataset, prototype feature extraction.
- Week 1-2: baseline model & evaluation; create inference endpoint and UI integration.
- Week 3-4: active learning loop + labeling, improve model (transformer/GNN prototype).
- Week 5+: hardening, monitoring, CI integration, and rollout.

## Next immediate steps (I can take now)
1. Inventory available labeled data and static-analyzer outputs in the repo (search `security`/`audit` logs).
2. Prototype a small extractor (use `tree-sitter` or language parser) and create 100 example feature rows.
3. Implement the `/api/security/scan` backend stub and a minimal UI page to display results.

If you want, I can start with step 1 (inventory data) and then prototype the extractor.
