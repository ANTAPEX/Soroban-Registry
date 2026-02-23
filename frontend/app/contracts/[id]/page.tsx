"use client";

import { Suspense, useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { Network } from "@/lib/api";
import ExampleGallery from "@/components/ExampleGallery";
import DependencyGraph from "@/components/DependencyGraph";
import {
  ArrowLeft,
  CheckCircle2,
  Globe,
  Tag,
  GitCompare,
} from "lucide-react";
import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import { useAnalytics } from "@/hooks/useAnalytics";
import FormalVerificationPanel from "@/components/FormalVerificationPanel";
import InteractionHistorySection from "@/components/InteractionHistorySection";
import Navbar from "@/components/Navbar";
import MaintenanceBanner from "@/components/MaintenanceBanner";
import CustomMetricsPanel from "@/components/CustomMetricsPanel";
import DeprecationBanner from "@/components/DeprecationBanner";

const NETWORKS: Network[] = ["mainnet", "testnet", "futurenet"];

// Mock for maintenance status since it was missing in the original file view but used in code
const maintenanceStatus = { is_maintenance: false, current_window: null };
function ContractDetailsContent() {
  const params = useParams();
  const searchParams = useSearchParams();
  const id = params.id as string;
  const networkFromUrl = searchParams.get("network") as Network | null;
  const [selectedNetwork, setSelectedNetwork] = useState<Network>(
    networkFromUrl && NETWORKS.includes(networkFromUrl) ? networkFromUrl : "mainnet"
  );

  const {
    data: contract,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["contract", id],
    queryFn: () => api.getContract(id),
  });

  const { data: dependencies, isLoading: depsLoading } = useQuery({
    queryKey: ["contract-dependencies", id],
    queryFn: () => api.getContractDependencies(id),
    enabled: !!contract,
  });
  const { logEvent } = useAnalytics();

  useEffect(() => {
    if (!error) return;
    logEvent("error_event", {
      source: "contract_details",
      contract_id: id,
      message: "Failed to load contract details",
    });
  }, [error, id, logEvent]);

  const { data: deprecationInfo } = useQuery({
    queryKey: ["contract-deprecation", id],
    queryFn: () => api.getDeprecationInfo(id),
    enabled: !!contract,
  });

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="animate-pulse space-y-8">
          <div className="h-8 bg-gray-200 dark:bg-gray-800 rounded w-1/3" />
          <div className="h-4 bg-gray-200 dark:bg-gray-800 rounded w-1/2" />
          <div className="h-64 bg-gray-200 dark:bg-gray-800 rounded-xl" />
        </div>
      </div>
    );
  }

  if (error || !contract) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="p-4 bg-red-50 text-red-600 rounded-lg">
          Failed to load contract details
        </div>
      </div>
    );
  }

  const configForNetwork = contract.network_configs?.[selectedNetwork];
  const displayContractId = configForNetwork?.contract_id ?? contract.contract_id;
  const displayVerified = configForNetwork?.is_verified ?? contract.is_verified;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-500">
      <Link
        href="/contracts"
        className="inline-flex items-center gap-2 text-gray-500 hover:text-gray-900 dark:hover:text-white mb-8 transition-colors"
      >
        <ArrowLeft className="w-4 h-4" />
        Back to contracts
      </Link>

      {/* Maintenance Banner */}
      {maintenanceStatus?.is_maintenance && maintenanceStatus.current_window && (
        <MaintenanceBanner window={maintenanceStatus.current_window} />
      )}

      {/* Deprecation Banner */}
      {deprecationInfo && <DeprecationBanner info={deprecationInfo} />}

      {/* Header */}
      <div className="mb-12">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-2">
              {contract.name}
            </h1>
            <div className="flex items-center gap-3 text-gray-500 dark:text-gray-400">
              <span className="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-sm">
                {displayContractId}
              </span>
              {displayVerified && (
                <span className="flex items-center gap-1 text-green-600 dark:text-green-400 text-sm font-medium">
                  <CheckCircle2 className="w-4 h-4" />
                  Verified
                </span>
              )}
            </div>
          </div>

          {/* Network tabs (Issue #43) */}
          <div className="flex gap-1 p-1 bg-gray-100 dark:bg-gray-800 rounded-lg w-fit">
            {NETWORKS.map((net) => {
              const hasConfig = !!contract.network_configs?.[net];
              return (
                <button
                  key={net}
                  type="button"
                  onClick={() => setSelectedNetwork(net)}
                  className={`px-4 py-2 rounded-md text-sm font-medium capitalize transition-colors ${
                    selectedNetwork === net
                      ? "bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm"
                      : "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                  } ${!hasConfig ? "opacity-60" : ""}`}
                >
                  {net}
                </button>
              );
            })}
          </div>

          <div className="flex gap-2">
            {/* Publisher actions/links could go here */}
          </div>
        </div>

        {contract.description && (
          <p className="text-xl text-gray-600 dark:text-gray-300 max-w-3xl mb-6">
            {contract.description}
          </p>
        )}

        <div className="flex flex-wrap gap-2">
          {contract.tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-sm font-medium"
            >
              <Tag className="w-3 h-3" />
              {tag}
            </span>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-12">
          {/* Dependency Graph */}
          {depsLoading ? (
            <section className="bg-white dark:bg-slate-900 rounded-lg p-8">
              <div className="animate-pulse space-y-4">
                <div className="h-8 bg-gray-200 dark:bg-gray-800 rounded w-1/3" />
                <div className="h-96 bg-gray-200 dark:bg-gray-800 rounded-lg" />
              </div>
            </section>
          ) : dependencies ? (
            <section>
              <DependencyGraph
                nodes={[]}
                edges={[]}
              />
            </section>
          ) : null}

          {/* Examples Gallery */}
          <section>
            <ExampleGallery contractId={contract.id} />
          </section>

          {/* Interaction History (Issue #46) */}
          <InteractionHistorySection contractId={contract.id} />
          {/* Custom Metrics */}
          <CustomMetricsPanel contractId={contract.id} />
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-4">
              Contract Details
            </h3>

            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500 dark:text-gray-400">Network</dt>
                <dd className="font-medium text-gray-900 dark:text-white capitalize">
                  {selectedNetwork}
                </dd>
              </div>
              {configForNetwork && (
                <>
                  <div>
                    <dt className="text-gray-500 dark:text-gray-400">Contract address</dt>
                    <dd className="font-mono text-xs text-gray-900 dark:text-white break-all">
                      {displayContractId}
                    </dd>
                  </div>
                  {(configForNetwork.min_version ?? configForNetwork.max_version) && (
                    <div>
                      <dt className="text-gray-500 dark:text-gray-400">Version range</dt>
                      <dd className="font-medium text-gray-900 dark:text-white">
                        {[configForNetwork.min_version, configForNetwork.max_version]
                          .filter(Boolean)
                          .join(" – ") || "—"}
                      </dd>
                    </div>
                  )}
                </>
              )}
              <div>
                <dt className="text-gray-500 dark:text-gray-400">Published</dt>
                <dd className="font-medium text-gray-900 dark:text-white">
                  {new Date(contract.created_at).toLocaleDateString()}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500 dark:text-gray-400">
                  Last Updated
                </dt>
                <dd className="font-medium text-gray-900 dark:text-white">
                  {new Date(contract.updated_at).toLocaleDateString()}
                </dd>
              </div>
            </dl>
          </div>

          {/* API Documentation (OpenAPI / Swagger) */}
          <Link
            href={`/contracts/${contract.id}/api-docs`}
            className="flex items-center gap-3 w-full px-4 py-3 rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 hover:bg-blue-50 dark:hover:bg-blue-900/20 hover:border-blue-300 dark:hover:border-blue-700 text-gray-700 dark:text-gray-300 hover:text-blue-700 dark:hover:text-blue-300 transition-all group"
          >
            <Globe className="w-5 h-5 text-gray-400 group-hover:text-blue-500 transition-colors" />
            <div>
              <div className="text-sm font-medium">API Docs</div>
              <div className="text-xs text-gray-400 dark:text-gray-500">OpenAPI / Swagger UI</div>
            </div>
          </Link>

          {/* Compatibility Matrix link */}
          <Link
            href={`/contracts/${contract.id}/compatibility`}
            className="flex items-center gap-3 w-full px-4 py-3 rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 hover:bg-blue-50 dark:hover:bg-blue-900/20 hover:border-blue-300 dark:hover:border-blue-700 text-gray-700 dark:text-gray-300 hover:text-blue-700 dark:hover:text-blue-300 transition-all group"
          >
            <GitCompare className="w-5 h-5 text-gray-400 group-hover:text-blue-500 transition-colors" />
            <div>
              <div className="text-sm font-medium">Compatibility Matrix</div>
              <div className="text-xs text-gray-400 dark:text-gray-500">View version compatibility</div>
            </div>
          </Link>

          {/* Formal Verification Panel */}
          <FormalVerificationPanel contractId={contract.id} />
        </div>
      </div>
    </div>
  );
}

export default function ContractPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />
      <Suspense fallback={null}>
        <ContractDetailsContent />
      </Suspense>
    </div>
  );
}
