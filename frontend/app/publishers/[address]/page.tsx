"use client";

import { Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "next/navigation";
import { getPublisher } from "@/lib/api/publishers";
import { PublisherHeader } from "@/components/publisher/PublisherHeader";
import { PublisherStats } from "@/components/publisher/PublisherStats";
import { PublisherContractsList } from "@/components/publisher/PublisherContractsList";
import { PublisherActivityTimeline } from "@/components/publisher/PublisherActivityTimeline";
import Navbar from "@/components/Navbar";
import { AlertCircle } from "lucide-react";

function PublisherProfileContent() {
  const params = useParams();
  const address = params.address as string;

  const { data: publisher, isLoading, error } = useQuery({
    queryKey: ["publisher", address],
    queryFn: () => getPublisher(address),
  });

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-pulse">
        <div className="bg-gray-200 dark:bg-gray-800 h-64 rounded-xl mb-8"></div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="bg-gray-200 dark:bg-gray-800 h-32 rounded-xl"></div>
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-2 bg-gray-200 dark:bg-gray-800 h-96 rounded-xl"></div>
          <div className="bg-gray-200 dark:bg-gray-800 h-96 rounded-xl"></div>
        </div>
      </div>
    );
  }

  if (error || !publisher) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 text-center">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 text-red-600 mb-4">
          <AlertCircle className="w-8 h-8" />
        </div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">Publisher Not Found</h2>
        <p className="text-gray-500 dark:text-gray-400 max-w-md mx-auto">
          We couldn't find a publisher with address <span className="font-mono bg-gray-100 dark:bg-gray-800 px-1 py-0.5 rounded">{address}</span>.
        </p>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8 animate-in fade-in duration-500">
      <PublisherHeader publisher={publisher} />
      
      <PublisherStats publisher={publisher} />
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2">
          <PublisherContractsList contracts={publisher.contracts} />
        </div>
        
        <div className="lg:col-span-1">
          <PublisherActivityTimeline activity={publisher.activity} />
        </div>
      </div>
    </div>
  );
}

export default function PublisherPage() {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-black text-gray-900 dark:text-gray-100 pb-12">
      <Navbar />
      <Suspense fallback={null}>
        <PublisherProfileContent />
      </Suspense>
    </div>
  );
}
