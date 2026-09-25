'use client';

import ContractCard from '@/components/ContractCard';
import ContractCardSkeleton from '@/components/ContractCardSkeleton';
import LoadingSkeleton from '@/components/LoadingSkeleton';
import { Search, Package, CheckCircle, Users, ArrowRight, Shield, GitBranch, Upload, Terminal, Github, MessageCircle, BookOpen, Zap } from 'lucide-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import { useAnalytics } from '@/hooks/useAnalytics';
import { useTranslation } from '@/lib/i18n/client';
import Navbar from '@/components/Navbar';
import ActivityFeed from '@/components/ActivityFeed';
import { useCopy } from '@/hooks/useCopy';
import CodeCopyButton from '@/components/CodeCopyButton';
import { buttonVariants } from '@/components/ui/button';
import { useRecentContracts, useRegistryStats } from "@/hooks/queries";
import { useReveal } from '@/hooks/useReveal';

export default function Home() {
  const { t } = useTranslation('common');
  const router = useRouter();
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);
  const { logEvent } = useAnalytics();
  const { copy, copied, isCopying } = useCopy();
  useReveal();

  const { data: stats, isLoading: statsLoading } = useRegistryStats();

  const { data: recentContracts, isLoading: contractsLoading } =
    useRecentContracts(6);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      logEvent('search_performed', {
        keyword: searchQuery.trim(),
        source: 'home_hero',
      });
      router.push(`/contracts?query=${encodeURIComponent(searchQuery)}`);
    }
  };

  const handleCopyCode = async () => {
    const code = `cargo install soroban-registry-cli\nsoroban-registry search token\nsoroban-registry install my-token-contract`;
    await copy(code, {
      successEventName: 'landing_cli_code_copied',
      failureEventName: 'landing_cli_code_copy_failed',
      successMessage: 'CLI example copied',
      failureMessage: 'Unable to copy CLI example',
      analyticsParams: { source: 'home_cli_block' },
    });
  };

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const isSlashShortcut = event.key === '/' || event.code === 'Slash';
      if (!isSlashShortcut || event.ctrlKey || event.metaKey || event.altKey) return;

      const activeElement = document.activeElement as HTMLElement | null;
      const isTypingField = Boolean(
        activeElement &&
        (activeElement.tagName === 'INPUT' ||
          activeElement.tagName === 'TEXTAREA' ||
          activeElement.tagName === 'SELECT' ||
          activeElement.isContentEditable),
      );

      if (isTypingField) return;

      event.preventDefault();
      searchInputRef.current?.focus();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      {/* Hero Section */}
      <section className="relative overflow-hidden ledger-grid">
        <div className="absolute inset-0 bg-gradient-to-b from-accent/60 via-transparent to-transparent" />
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24 relative">
          <div className="text-center max-w-3xl mx-auto">
            <p className="eyebrow mb-6 inline-flex items-center gap-2 rounded-full border border-border bg-card/60 px-4 py-1.5 animate-fade-in-up">
              <span className="w-1.5 h-1.5 rounded-full bg-primary" aria-hidden="true" />
              Soroban smart contract registry
            </p>

            <h1 className="text-6xl sm:text-7xl lg:text-8xl font-semibold mb-6 leading-[0.92] tracking-[-0.04em] animate-fade-in-up-delay-1">
              {t('home.title_part1')}
              <br />
              <span className="text-gradient">
                {t('home.title_part2')}
              </span>
            </h1>

            <p className="text-xl sm:text-2xl text-muted-foreground mb-12 animate-fade-in-up-delay-2">
              {t('home.subtitle')}
            </p>

            {/* Search Bar */}
            <form onSubmit={handleSearch} className="max-w-2xl mx-auto mb-12 animate-fade-in-up-delay-3">
              <div className="relative">
                <Search className="absolute left-5 top-1/2 -translate-y-1/2 w-5 h-5 text-muted-foreground" />
                <input
                  ref={searchInputRef}
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search contracts by name, category, or tag..."
                  aria-label="Search contracts"
                  aria-keyshortcuts="/"
                  className="w-full pl-14 pr-28 py-4 rounded-lg border border-border-strong bg-card text-foreground placeholder-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary shadow-lg"
                />
                <button
                  type="submit"
                  className={buttonVariants({ lift: false, className: 'absolute right-2 top-1/2 -translate-y-1/2' })}
                >
                  Search
                </button>
              </div>
            </form>

            {/* Stats */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-3xl mx-auto">
              {statsLoading ? (
                <>
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="bg-card rounded-lg p-6 border border-border">
                      <div className="flex items-center justify-center gap-2 mb-2">
                        <LoadingSkeleton width="3rem" height="2.25rem" />
                      </div>
                      <LoadingSkeleton width="7rem" height="0.875rem" className="mx-auto" />
                    </div>
                  ))}
                </>
              ) : stats ? (
                <>
                  <div className="bg-card rounded-lg p-6 border border-border">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <Package className="w-5 h-5 text-primary" />
                      <span className="text-3xl font-semibold font-mono tabular-nums">
                        {stats.total_contracts}
                      </span>
                    </div>
                    <p className="eyebrow">Total contracts</p>
                  </div>

                  <div className="bg-card rounded-lg p-6 border border-border">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <CheckCircle className="w-5 h-5 text-success" />
                      <span className="text-3xl font-semibold font-mono tabular-nums">
                        {stats.verified_contracts}
                      </span>
                    </div>
                    <p className="eyebrow">Verified</p>
                  </div>

                  <div className="bg-card rounded-lg p-6 border border-border">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <Users className="w-5 h-5 text-muted-foreground" />
                      <span className="text-3xl font-semibold font-mono tabular-nums">
                        {stats.total_publishers}
                      </span>
                    </div>
                    <p className="eyebrow">Publishers</p>
                  </div>
                </>
              ) : null}
            </div>
          </div>
        </div>
      </section>

      {/* Recently published ticker */}
      {recentContracts && (recentContracts.items?.length ?? 0) > 0 && (
        <section aria-label="Recently published contracts" className="border-y border-border bg-card">
          <div className="marquee py-4">
            <div className="marquee-track">
              {[0, 1].map((copyIndex) => (
                <ul
                  key={copyIndex}
                  aria-hidden={copyIndex === 1 || undefined}
                  className="flex shrink-0 items-center"
                >
                  {(recentContracts.items ?? []).map((contract) => (
                    <li key={contract.id} className="flex items-center">
                      <Link
                        href={`/contracts/${contract.id}`}
                        tabIndex={copyIndex === 1 ? -1 : undefined}
                        className="flex items-center gap-2 px-6 text-sm text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {contract.is_verified && (
                          <CheckCircle className="w-4 h-4 text-primary" aria-label="Verified" />
                        )}
                        <span className="font-medium text-foreground">{contract.name}</span>
                        <span className="font-mono text-xs uppercase">{contract.network}</span>
                      </Link>
                      <span className="w-1 h-1 rounded-full bg-border-strong/40" aria-hidden="true" />
                    </li>
                  ))}
                </ul>
              ))}
            </div>
          </div>
        </section>
      )}

      {/* Why Soroban Registry — Feature Cards */}
      <section className="reveal max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-semibold mb-4">
            {t('home.whyTitle')} <span className="text-gradient">Registry</span>
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            {t('home.whySubtitle')}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-md border border-border-strong flex items-center justify-center mb-6">
              <Shield className="w-6 h-6 text-foreground" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.verified.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.verified.desc')}
            </p>
          </div>

          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-md border border-border-strong flex items-center justify-center mb-6">
              <GitBranch className="w-6 h-6 text-primary" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.graph.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.graph.desc')}
            </p>
          </div>

          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-md border border-border-strong flex items-center justify-center mb-6">
              <Upload className="w-6 h-6 text-foreground" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.easy.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.easy.desc')}
            </p>
          </div>
        </div>
      </section>

      {/* Recent Contracts */}
      <section className="reveal max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="flex items-center justify-between mb-8">
          <h2 className="text-3xl font-semibold">
            {t('home.recent')}
          </h2>
          <Link
            href="/contracts"
            className="flex items-center gap-2 text-primary hover:opacity-80 font-medium transition-opacity"
          >
            {t('home.viewAll')}
            <ArrowRight className="w-4 h-4" />
          </Link>
        </div>

        {contractsLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <ContractCardSkeleton key={i} />
            ))}
          </div>
        ) : recentContracts && (recentContracts.items?.length ?? 0) > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {(recentContracts.items ?? []).map((contract) => (
              <ContractCard key={contract.id} contract={contract} />
            ))}
          </div>
        ) : (
          <div className="text-center py-12 rounded-lg border border-dashed border-border bg-card">
            <Package className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
            <p className="text-muted-foreground">No contracts published yet</p>
          </div>
        )}
      </section>

    {/* Activity Feed Section */}
    <section className="reveal bg-muted/30 border-y border-border">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-12">
          <div className="lg:col-span-2">
            <ActivityFeed />
          </div>
          <div className="space-y-8">
            <div className="bg-card border border-border rounded-lg p-6">
              <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-success" aria-hidden="true" />
                Live insights
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                The registry is alive with activity. Watch as developers publish, verify, and deploy contracts in real-time.
              </p>
              <div className="space-y-4">
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-md bg-primary/10 text-primary">
                    <Upload className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Publishing</h4>
                    <p className="text-xs text-muted-foreground">New contracts added to the registry</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-md bg-success/10 text-success">
                    <CheckCircle className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Verification</h4>
                    <p className="text-xs text-muted-foreground">Source code validated by our nodes</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-md bg-muted text-muted-foreground">
                    <Zap className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Deployments</h4>
                    <p className="text-xs text-muted-foreground">Contracts going live on Stellar networks</p>
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-card border border-primary/40 rounded-lg p-6">
              <h3 className="text-lg font-semibold mb-2">Build together</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Share your contracts with the ecosystem and help other builders.
              </p>
              <Link
                href="/publish"
                className={buttonVariants({ size: 'sm', className: 'w-full' })}
              >
                Publish your contract
                <ArrowRight className="w-4 h-4" />
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>

      {/* Install & Learn — Code Section */}
      <section className="reveal max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <h2 className="text-3xl sm:text-4xl font-semibold mb-6">
              Install & start <span className="text-gradient">building</span>
            </h2>
            <p className="text-lg text-muted-foreground mb-8 leading-relaxed">
              Get up and running in minutes. Install the CLI, search the registry,
              and integrate verified contracts into your Soroban project.
            </p>
            <div className="flex flex-col sm:flex-row gap-4">
              <Link
                href="/contracts"
                className={buttonVariants({ size: 'lg' })}
              >
                Browse contracts
                <ArrowRight className="w-4 h-4" />
              </Link>
              <Link
                href="/templates"
                className={buttonVariants({ variant: 'outline', size: 'lg' })}
              >
                View templates
              </Link>
            </div>
          </div>

          <div className="sweep-line rounded-lg overflow-hidden border border-border bg-deep text-deep-foreground">
            <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
              <div className="flex items-center gap-2">
                <Terminal className="w-4 h-4 text-deep-foreground/60" />
                <span className="text-xs text-deep-foreground/60 font-mono">Terminal</span>
              </div>
              <CodeCopyButton
                onCopy={handleCopyCode}
                copied={copied}
                disabled={isCopying}
                idleLabel="Copy"
                copiedLabel="Copied"
                tone="terminal"
              />
            </div>
            <div className="p-6 font-mono text-sm leading-relaxed">
              <div className="text-deep-foreground/45 mb-1"># Install the CLI</div>
              <div className="text-deep-foreground mb-4">$ cargo install soroban-registry-cli</div>
              <div className="text-deep-foreground/45 mb-1"># Search for contracts</div>
              <div className="text-deep-foreground mb-4">$ soroban-registry search token</div>
              <div className="text-deep-foreground/45 mb-1"># Install a contract</div>
              <div className="text-deep-foreground mb-4">$ soroban-registry install my-token-contract</div>
              <div className="text-deep-foreground">
                $<span className="terminal-cursor" aria-hidden="true" />
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Community / CTA Section */}
      <section className="reveal border-t border-border bg-accent/50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
          <div className="text-center mb-12">
            <h2 className="text-3xl sm:text-4xl font-semibold mb-4">
              Join the <span className="text-gradient">community</span>
            </h2>
            <p className="text-lg text-muted-foreground max-w-xl mx-auto">
              Connect with developers building the future of DeFi on Stellar.
            </p>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-3xl mx-auto">
            <a
              href="https://github.com/stellar"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-md bg-background flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <Github className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">GitHub</h3>
              <p className="text-sm text-muted-foreground">Contribute to the codebase</p>
            </a>

            <a
              href="https://discord.com/invite/stellardev"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-md bg-background flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <MessageCircle className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">Discord</h3>
              <p className="text-sm text-muted-foreground">Chat with developers</p>
            </a>

            <a
              href="https://developers.stellar.org/docs/smart-contracts"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-md bg-background flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <BookOpen className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">Documentation</h3>
              <p className="text-sm text-muted-foreground">Read the Soroban docs</p>
            </a>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border bg-card" aria-label="Site footer">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8 mb-12">
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.registry')}</h4>
              <ul className="space-y-3 text-sm">
                <li><Link href="/contracts" className="text-muted-foreground hover:text-foreground transition-colors">{t('home.viewContracts')}</Link></li>
                <li><Link href="/templates" className="text-muted-foreground hover:text-foreground transition-colors">Templates</Link></li>
                <li><Link href="/publish" className="text-muted-foreground hover:text-foreground transition-colors">Publish</Link></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.explore')}</h4>
              <ul className="space-y-3 text-sm">
                <li><Link href="/graph" className="text-muted-foreground hover:text-foreground transition-colors">Dependency Graph</Link></li>
                <li><Link href="/stats" className="text-muted-foreground hover:text-foreground transition-colors">Statistics</Link></li>
                <li><Link href="/publishers" className="text-muted-foreground hover:text-foreground transition-colors">Publishers</Link></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.developers')}</h4>
              <ul className="space-y-3 text-sm">
                <li><a href="https://developers.stellar.org/docs/smart-contracts" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Soroban Docs</a></li>
                <li><a href="https://stellar.org/soroban" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">About Soroban</a></li>
                <li><a href="https://github.com/stellar" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">GitHub</a></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.community')}</h4>
              <ul className="space-y-3 text-sm">
                <li><a href="https://discord.com/invite/stellardev" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Discord</a></li>
                <li><a href="https://twitter.com/BuildOnStellar" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Twitter</a></li>
                <li><a href="https://stellar.org/community" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Stellar Community</a></li>
              </ul>
            </div>
          </div>

          <div className="border-t border-border pt-8 flex flex-col sm:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2 text-muted-foreground text-sm">
              <Package className="w-4 h-4 text-primary" />
              <span>{t('footer.builtFor')}</span>
            </div>
            <p className="text-sm text-muted-foreground">{t('footer.poweredBy')}</p>
          </div>
        </div>
      </footer>
    </div>
  );
}
