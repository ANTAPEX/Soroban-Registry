import { Suspense } from 'react';
import { ContractsContent } from './contracts-content';
import Navbar from '@/components/Navbar';

export const dynamic = 'force-dynamic';

export default function ContractsPage() {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-950">
      <Navbar />

      <Suspense
        fallback={
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
            <div className="text-center py-12">
              <div className="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin" />
            </div>
          </div>
        }
      >
        <ContractsContent />
      </Suspense>
    </div>
  );
}
