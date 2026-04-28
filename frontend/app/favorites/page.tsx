'use client';

import { useQueries } from '@tanstack/react-query';
import { Star, BookmarkX, ArrowRight, Share2, FolderPlus } from 'lucide-react';
import Link from 'next/link';
import { useState } from 'react';
import { api } from '@/lib/api';
import ContractCard from '@/components/ContractCard';
import ContractCardSkeleton from '@/components/ContractCardSkeleton';
import Navbar from '@/components/Navbar';
import { useFavorites } from '@/hooks/useFavorites';
import { useCopy } from '@/hooks/useCopy';

export default function FavoritesPage() {
  const { favorites, isLoading: favoritesLoading, clearAllFavorites } = useFavorites();
  const [confirmingClear, setConfirmingClear] = useState(false);
  const { copy, copied } = useCopy();

  // Fetch contract data for each favorited UUID in parallel
  const contractQueries = useQueries({
    queries: favorites.map((id) => ({
      queryKey: ['contract', id],
      queryFn: () => api.getContract(id),
      retry: false,
    })),
  });

  const isLoading = favoritesLoading || contractQueries.some((q) => q.isLoading);

  // Only include successfully loaded contracts (silently omit 404s / errors)
  const loadedContracts = contractQueries
    .filter((q) => q.isSuccess && q.data)
    .map((q) => q.data!);

  const handleClearAll = () => {
    if (confirmingClear) {
      clearAllFavorites();
      setConfirmingClear(false);
    } else {
      setConfirmingClear(true);
    }
  };

  const handleCancelClear = () => setConfirmingClear(false);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        {/* Hero / heading */}
        <div className="mb-10 flex items-start justify-between gap-4 flex-wrap">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-yellow-500/10 border border-yellow-500/20 flex items-center justify-center">
                <Star className="w-5 h-5 text-yellow-500" fill="currentColor" />
              </div>
              <h1 className="text-3xl font-bold text-foreground">
                Your Favorites
                {!isLoading && (
                  <span className="ml-2 text-xl font-normal text-muted-foreground">
                    ({loadedContracts.length})
                  </span>
                )}
              </h1>
            </div>
            <p className="text-muted-foreground">
              Contracts you&apos;ve saved for quick access.
            </p>
          </div>

          {favorites.length > 0 && !isLoading && (
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-50"
                disabled
                title="Coming soon"
              >
                <FolderPlus className="w-4 h-4" />
                Collections
              </button>
              <button
                type="button"
                onClick={() => {
                  if (typeof window !== 'undefined') {
                    const url = new URL(window.location.href);
                    url.searchParams.set('list', favorites.join(','));
                    copy(url.toString(), {
                      successMessage: 'Favorites list link copied!',
                      failureMessage: 'Failed to copy link',
                    });
                  }
                }}
                className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                <Share2 className="w-4 h-4" />
                {copied ? 'Copied Link' : 'Share'}
              </button>
              <div className="w-px h-6 bg-border mx-1 hidden sm:block"></div>
              {confirmingClear ? (
                <>
                  <span className="text-sm text-muted-foreground">Remove all {favorites.length} favorites?</span>
                  <button
                    type="button"
                    onClick={handleClearAll}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-sm font-medium text-red-500 transition-colors hover:bg-red-500/20"
                  >
                    <BookmarkX className="w-4 h-4" />
                    Confirm
                  </button>
                  <button
                    type="button"
                    onClick={handleCancelClear}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-foreground transition-colors hover:bg-accent"
                  >
                    Cancel
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  onClick={handleClearAll}
                  className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  <BookmarkX className="w-4 h-4" />
                  Remove all favorites
                </button>
              )}
            </div>
          )}
        </div>

        {/* Loading state */}
        {isLoading && (
          <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
            {Array.from({ length: Math.max(favorites.length, 3) }).map((_, i) => (
              <ContractCardSkeleton key={i} />
            ))}
          </div>
        )}

        {/* Empty state */}
        {!isLoading && favorites.length === 0 && (
          <div className="flex flex-col items-center justify-center py-24 text-center">
            <div className="w-16 h-16 rounded-2xl bg-yellow-500/10 border border-yellow-500/20 flex items-center justify-center mb-6">
              <Star className="w-8 h-8 text-yellow-500/50" />
            </div>
            <h2 className="text-xl font-semibold text-foreground mb-2">No favorites saved yet</h2>
            <p className="text-muted-foreground max-w-sm mb-8">
              Browse contracts and click the <strong>Save</strong> button on any card to add it here.
            </p>
            <Link
              href="/contracts"
              className="inline-flex items-center gap-2 rounded-xl bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-all hover:brightness-110 btn-glow"
            >
              Browse contracts
              <ArrowRight className="w-4 h-4" />
            </Link>
          </div>
        )}

        {/* Contracts grid */}
        {!isLoading && loadedContracts.length > 0 && (
          <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
            {loadedContracts.map((contract) => (
              <ContractCard key={contract.id} contract={contract} />
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
