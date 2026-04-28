#!/usr/bin/env python3
"""
Export security scan results, contract snapshots, and verified source code to JSONL for ML dataset building.

Usage:
  DATABASE_URL=postgresql://user:pass@host:5432/dbname python3 tools/export_security_dataset.py --out dataset.jsonl

    The script queries `security_scans`, `security_issues`, `contract_snapshots`, and `verifications` and emits one JSON object
    per issue (or per scan if no issues) with attached snapshot_data and source_code when available.
"""
import os
import argparse
import json
import sys

try:
    import psycopg2
    import psycopg2.extras
except Exception:
    print("psycopg2 is required. Install with: pip install psycopg2-binary", file=sys.stderr)
    raise


def fetch_and_export(db_url, out_path, limit=None):
    conn = psycopg2.connect(db_url)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    # Fetch scans that have raw results or issues
    scan_q = "SELECT * FROM security_scans ORDER BY created_at DESC"
    if limit:
        scan_q += f" LIMIT {int(limit)}"

    cur.execute(scan_q)
    scans = cur.fetchall()

    def extract_source(snapshot):
        if isinstance(snapshot, dict):
            for key in ("source_code", "source", "code", "contract_source", "rust_source", "content"):
                value = snapshot.get(key)
                if isinstance(value, str) and len(value.strip()) > 32:
                    return value
            for value in snapshot.values():
                if isinstance(value, dict):
                    nested = extract_source(value)
                    if nested:
                        return nested
                elif isinstance(value, str) and len(value.strip()) > 120 and any(tok in value for tok in ("pub fn", "#[contract", "require_auth", "contractimpl")):
                    return value
        return None

    def normalize_label(issue):
        category = (issue.get("category") or "").lower()
        title = (issue.get("title") or "").lower()
        description = (issue.get("description") or "").lower()
        text = f"{category} {title} {description}"
        if any(token in text for token in ("auth", "access", "permission", "role")):
            return "access-control"
        if any(token in text for token in ("overflow", "underflow", "panic", "reentrancy", "unsafe", "runtime")):
            return "runtime-safety"
        if any(token in text for token in ("loop", "gas", "dos", "resource", "budget")):
            return "resource-usage"
        if any(token in text for token in ("storage", "key", "collision", "state")):
            return "storage"
        if any(token in text for token in ("event", "observability", "logging", "trace")):
            return "observability"
        if any(token in text for token in ("optimiz", "maintain", "hardcoded", "magic")):
            return "maintainability"
        return "optimization"

    with open(out_path, "w", encoding="utf-8") as out:
        for scan in scans:
            scan_id = scan["id"]
            contract_id = scan["contract_id"]
            contract_version_id = scan.get("contract_version_id")

            # Try to fetch a snapshot for this version (if present)
            snapshot = None
            if contract_version_id:
                cur.execute(
                    "SELECT snapshot_data FROM contract_snapshots WHERE contract_id = %s AND version_number = (SELECT version_number FROM contract_versions WHERE id = %s) LIMIT 1",
                    (contract_id, contract_version_id),
                )
                row = cur.fetchone()
                if row:
                    snapshot = row["snapshot_data"]

            cur.execute(
                "SELECT source_code FROM verifications WHERE contract_id = %s AND status = 'verified' ORDER BY created_at DESC LIMIT 1",
                (contract_id,),
            )
            verification_row = cur.fetchone()
            source_code = verification_row["source_code"] if verification_row and verification_row.get("source_code") else None

            if not source_code:
                source_code = extract_source(snapshot)
            if not source_code and snapshot:
                source_code = json.dumps(snapshot, default=str)
            # Fallback: latest snapshot for contract
            if not snapshot:
                cur.execute(
                    "SELECT snapshot_data FROM contract_snapshots WHERE contract_id = %s ORDER BY version_number DESC LIMIT 1",
                    (contract_id,)
                )
                row = cur.fetchone()
                if row:
                    snapshot = row["snapshot_data"]

            # Fetch issues for this scan
            cur.execute("SELECT * FROM security_issues WHERE scan_id = %s", (scan_id,))
            issues = cur.fetchall()

            if issues:
                for issue in issues:
                    out_obj = {
                        "scan": dict(scan),
                        "issue": dict(issue),
                        "snapshot": snapshot,
                        "source_code": source_code,
                        "label": normalize_label(issue),
                    }
                    out.write(json.dumps(out_obj, default=str) + "\n")
            else:
                # Emit one record per scan if no issues
                out_obj = {
                    "scan": dict(scan),
                    "issue": None,
                    "snapshot": snapshot,
                    "source_code": source_code,
                    "label": "benign",
                }
                out.write(json.dumps(out_obj, default=str) + "\n")

    cur.close()
    conn.close()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--out", required=True, help="Output JSONL file")
    p.add_argument("--limit", required=False, help="Limit number of scans")
    args = p.parse_args()

    db_url = os.environ.get("DATABASE_URL")
    if not db_url:
        print("Set DATABASE_URL environment variable", file=sys.stderr)
        sys.exit(2)

    fetch_and_export(db_url, args.out, limit=args.limit)


if __name__ == "__main__":
    main()
