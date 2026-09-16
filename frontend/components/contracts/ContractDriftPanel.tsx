"use client";

import React, { useEffect, useState } from "react";
import { AlertCircle, AlertTriangle, ArrowRightLeft, CheckCircle2, Clock, Database, ExternalLink, HelpCircle, Layers, RefreshCw } from "lucide-react";
import DriftBadge, { DriftStatus } from "@/components/verification/DriftBadge";

export interface ThreeWayDriftData {
  contract_id: string;
  lockfile_hash?: string;
  registry_hash: string;
  onchain_hash?: string;
  status: DriftStatus;
  diverged_leg?: string;
  first_detected_at: string;
  last_checked_at: string;
  observed_at_ledger?: number;
}

interface ContractDriftPanelProps {
  contractId: string;
  network?: string;
  initialDriftData?: ThreeWayDriftData;
}

export default function ContractDriftPanel({
  contractId,
  network = "testnet",
  initialDriftData,
}: ContractDriftPanelProps) {
  const [driftData, setDriftData] = useState<ThreeWayDriftData | null>(
    initialDriftData || null
  );
  const [loading, setLoading] = useState(!initialDriftData);
  const [error, setError] = useState<string | null>(null);

  const fetchDriftState = async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await fetch(`/api/contracts/${contractId}/drift?network=${network}`);
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}: Failed to fetch drift state`);
      }
      const data: ThreeWayDriftData = await res.json();
      setDriftData(data);
    } catch (err: any) {
      setError(err?.message || "Failed to load drift status");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!initialDriftData) {
      fetchDriftState();
    }
  }, [contractId, network]);

  if (loading) {
    return (
      <div className="bg-card rounded-2xl border border-border p-6 animate-pulse">
        <div className="h-6 w-48 bg-muted rounded mb-4" />
        <div className="h-20 bg-muted rounded" />
      </div>
    );
  }

  if (error || !driftData) {
    return (
      <div className="bg-card rounded-2xl border border-border p-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-muted-foreground text-sm">
            <HelpCircle className="w-4 h-4" />
            <span>Three-way WASM drift status could not be verified.</span>
          </div>
          <button
            onClick={fetchDriftState}
            className="inline-flex items-center gap-1.5 text-xs text-primary hover:underline"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Retry
          </button>
        </div>
      </div>
    );
  }

  const isDrifted = driftData.status === "drift";
  const isMatch = driftData.status === "match";

  return (
    <div
      className={`rounded-2xl border p-6 transition-all ${
        isDrifted
          ? "border-red-500/40 bg-red-500/5 shadow-sm"
          : isMatch
          ? "border-green-500/30 bg-card"
          : "border-border bg-card"
      }`}
    >
      <div className="flex flex-wrap items-center justify-between gap-4 pb-4 border-b border-border/60">
        <div>
          <div className="flex items-center gap-3">
            <h3 className="text-lg font-bold text-foreground flex items-center gap-2">
              <ArrowRightLeft className="w-5 h-5 text-primary" />
              Three-Way WASM Drift Analysis
            </h3>
            <DriftBadge status={driftData.status} size="md" />
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            Continuous parity check between local lockfile, registry entry, and live Stellar on-chain code.
          </p>
        </div>

        <button
          onClick={fetchDriftState}
          className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground border border-border bg-background px-3 py-1.5 rounded-lg hover:bg-accent transition-colors"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          Refresh
        </button>
      </div>

      {isDrifted && (
        <div className="my-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-xs text-red-600 dark:text-red-400 flex items-start gap-2.5">
          <AlertTriangle className="w-4 h-4 flex-shrink-0 mt-0.5" />
          <div>
            <span className="font-semibold">Cryptographic divergence detected:</span> The on-chain deployed WASM code differs from the registry catalog. Verification guarantees are overridden until the contract is upgraded or re-published.
          </div>
        </div>
      )}

      {/* Hashes & Legs */}
      <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
        <div className="p-3 rounded-xl bg-background border border-border">
          <div className="text-muted-foreground mb-1 flex items-center justify-between">
            <span className="font-sans font-medium text-foreground flex items-center gap-1.5">
              <Database className="w-3.5 h-3.5 text-primary" />
              Registry WASM Hash
            </span>
            <span className="text-[10px] text-muted-foreground">Catalog</span>
          </div>
          <div className="break-all select-all font-semibold text-foreground">
            {driftData.registry_hash}
          </div>
        </div>

        <div className="p-3 rounded-xl bg-background border border-border">
          <div className="text-muted-foreground mb-1 flex items-center justify-between">
            <span className="font-sans font-medium text-foreground flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-primary" />
              Live On-Chain WASM Hash
            </span>
            <span className="text-[10px] text-muted-foreground">Chain</span>
          </div>
          <div
            className={`break-all select-all font-semibold ${
              isDrifted
                ? "text-red-500 dark:text-red-400"
                : driftData.onchain_hash
                ? "text-foreground"
                : "text-muted-foreground italic"
            }`}
          >
            {driftData.onchain_hash || "Not deployed / entry missing"}
          </div>
        </div>
      </div>

      {/* Observation Metadata */}
      <div className="mt-4 pt-4 border-t border-border/60 grid grid-cols-2 sm:grid-cols-4 gap-4 text-xs">
        <div>
          <dt className="text-muted-foreground">Diverged Leg</dt>
          <dd className="font-semibold text-foreground mt-0.5">
            {driftData.diverged_leg || "None (All match)"}
          </dd>
        </div>

        <div>
          <dt className="text-muted-foreground">First Detected At</dt>
          <dd className="font-medium text-foreground mt-0.5">
            {new Date(driftData.first_detected_at).toLocaleString()}
          </dd>
        </div>

        <div>
          <dt className="text-muted-foreground">Last Checked</dt>
          <dd className="font-medium text-foreground mt-0.5">
            {new Date(driftData.last_checked_at).toLocaleString()}
          </dd>
        </div>

        <div>
          <dt className="text-muted-foreground">Observed Ledger</dt>
          <dd className="font-semibold text-foreground mt-0.5">
            {driftData.observed_at_ledger ? `#${driftData.observed_at_ledger}` : "—"}
          </dd>
        </div>
      </div>
    </div>
  );
}
