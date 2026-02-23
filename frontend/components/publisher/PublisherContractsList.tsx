import React, { useState, useMemo } from "react";
import { ContractSummary } from "@/types/publisher";
import { Tag } from "@/types/tag";
import TagAutocomplete from "@/components/tags/TagAutocomplete";
import { Search, Filter, ArrowUpRight } from "lucide-react";
import { VerificationBadge } from "./VerificationBadge";
import Link from "next/link";

interface PublisherContractsListProps {
  contracts: ContractSummary[];
}

type FilterStatus = "all" | "verified" | "failed" | "pending";

export function PublisherContractsList({ contracts }: PublisherContractsListProps) {
  const [statusFilter, setStatusFilter] = useState<FilterStatus>("all");
  const [searchTerm, setSearchTerm] = useState("");

  const filteredContracts = useMemo(() => {
    return contracts
      .filter((contract) => {
        const matchesStatus = statusFilter === "all" || contract.verificationStatus === statusFilter;
        const matchesSearch = contract.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          contract.description.toLowerCase().includes(searchTerm.toLowerCase());
        return matchesStatus && matchesSearch;
      })
      .sort((a, b) => new Date(b.deployedAt).getTime() - new Date(a.deployedAt).getTime());
  }, [contracts, statusFilter, searchTerm]);

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 mb-6">
        <h2 className="text-xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
          Published Contracts
          <span className="text-sm font-normal text-gray-500 bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded-full">
            {filteredContracts.length}
          </span>
        </h2>
        
        <div className="flex flex-col sm:flex-row gap-3 w-full md:w-auto">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search contracts..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 pr-4 py-2 w-full sm:w-64 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none transition-all"
            />
          </div>
          
          <div className="relative">
            <Filter className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as FilterStatus)}
              className="pl-9 pr-8 py-2 w-full sm:w-40 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg text-sm appearance-none focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none cursor-pointer"
            >
              <option value="all">All Status</option>
              <option value="verified">Verified</option>
              <option value="pending">Pending</option>
              <option value="failed">Failed</option>
            </select>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
        {filteredContracts.length > 0 ? (
          filteredContracts.map((contract) => (
            <Link
              key={contract.id}
              href={`/contracts/${contract.id}`}
              className="group block bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-5 hover:border-blue-500 dark:hover:border-blue-500 transition-colors"
            >
              <div className="flex justify-between items-start mb-3">
                <VerificationBadge status={contract.verificationStatus} />
                <ArrowUpRight className="w-4 h-4 text-gray-400 group-hover:text-blue-500 transition-colors" />
              </div>
              
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate">
                {contract.name}
              </h3>
              
              <p className="text-sm text-gray-500 dark:text-gray-400 line-clamp-2 mb-4 h-10">
                {contract.description}
              </p>
              
              {contract.tags && contract.tags.length > 0 && (
                <div className="flex flex-wrap gap-2 mb-4">
                  {contract.tags.slice(0, 3).map((tag) => (
                    <span
                      key={tag}
                      className="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300"
                    >
                      {tag}
                    </span>
                  ))}
                  {contract.tags.length > 3 && (
                    <span className="text-xs text-gray-500 flex items-center">+{contract.tags.length - 3}</span>
                  )}
                </div>
              )}
              
              <div className="flex items-center justify-between text-xs text-gray-400 pt-3 border-t border-gray-200 dark:border-gray-700">
                <span>ID: {contract.id.substring(0, 8)}...</span>
                <span>{new Date(contract.deployedAt).toLocaleDateString()}</span>
              </div>
            </Link>
          ))
        ) : (
          <div className="col-span-full py-12 text-center text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-dashed border-gray-200 dark:border-gray-700">
            <p>No contracts found matching your filters.</p>
          </div>
        )}
      </div>
    </div>
  );
}
