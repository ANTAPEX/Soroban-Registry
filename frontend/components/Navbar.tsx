'use client';

import { Package, ChevronDown, BarChart2, Users, Menu, X, Layers, Search, ArrowUpRight, Columns2, ShieldCheck, PieChart, TrendingUp, Settings, Zap, Code2, Star, GitBranch, Store } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import React, { useState, useRef, useEffect, useCallback } from 'react';
import ThemeToggle from './ThemeToggle';
import NotificationBell from './NotificationBell';
import { useTranslation } from '@/lib/i18n/client';
import LanguageSelector from './LanguageSelector';
import { useFavorites } from '@/hooks/useFavorites';

/* ─── nav links ──────────────────────────────────────────── */
const NAV_LINKS = [
    { href: '/contracts',       label: 'Browse'  },
    { href: '/compare',         label: 'Compare' },
    { href: '/marketplace',     label: 'Market'  },
    { href: '/verify-contract', label: 'Verify'  },
    { href: '/developer',       label: 'IDE'     },
] as const;

const EXPLORE_LINKS = [
    { href: '/stats',      label: 'Statistics', icon: BarChart2 },
    { href: '/analytics',  label: 'Analytics',  icon: PieChart  },
    { href: '/templates',  label: 'Templates',  icon: Layers    },
] as const;

/* ─── helpers ─────────────────────────────────────────────── */
function useScrolled(threshold = 8) {
    const [scrolled, setScrolled] = useState(false);
    useEffect(() => {
        const onScroll = () => setScrolled(window.scrollY > threshold);
        window.addEventListener('scroll', onScroll, { passive: true });
        return () => window.removeEventListener('scroll', onScroll);
    }, [threshold]);
    return scrolled;
}

function useTrapFocus(ref: React.RefObject<HTMLElement | null>, active: boolean) {
    const previouslyFocused = useRef<HTMLElement | null>(null);

    useEffect(() => {
        if (!active || !ref.current) return;
        previouslyFocused.current = document.activeElement as HTMLElement | null;

        const el = ref.current;
        const focusable = el.querySelectorAll<HTMLElement>(
            'a[href], button:not([disabled]), input, [tabindex]:not([tabindex="-1"])'
        );
        const first = focusable[0];
        const last  = focusable[focusable.length - 1];
        const onKey = (e: KeyboardEvent) => {
            if (e.key !== 'Tab') return;
            if (e.shiftKey ? document.activeElement === first : document.activeElement === last) {
                e.preventDefault();
                (e.shiftKey ? last : first).focus();
            }
        };
        el.addEventListener('keydown', onKey);
        first?.focus();
        return () => {
            el.removeEventListener('keydown', onKey);
            previouslyFocused.current?.focus();
        };
    }, [active, ref]);
}

/* ─── Search Modal ─────────────────────────────────────────── */
function SearchModal({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
    const inputRef = useRef<HTMLInputElement>(null);
    const [query, setQuery] = useState('');

    useEffect(() => {
        if (isOpen) {
            setTimeout(() => {
                inputRef.current?.focus();
                setQuery('');
            }, 0);
        }
    }, [isOpen]);

    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onClose();
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, [onClose]);

    const suggestions = [
        { label: 'Token Contract',     href: '/contracts?q=token',   icon: Zap    },
        { label: 'NFT Marketplace',    href: '/contracts?q=nft',     icon: Layers },
        { label: 'DeFi Protocols',     href: '/contracts?q=defi',    icon: TrendingUp},
        { label: 'DAO Governance',     href: '/contracts?q=dao',     icon: Users  },
        { label: 'Smart Contract SDK', href: '/contracts?q=sdk',     icon: Code2  },
    ];

    return (
        <div
            className={`fixed inset-0 z-[200] flex items-start justify-center pt-16 sm:pt-24 px-4 transition-all duration-200 ${
                isOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
            }`}
            role="dialog"
            aria-modal="true"
            aria-label="Search"
        >
            {/* Backdrop */}
            <div
                className="absolute inset-0 bg-background/70 backdrop-blur-md"
                onClick={onClose}
            />

            {/* Modal panel */}
            <div
                className={`relative w-full max-w-lg bg-card border border-border rounded-2xl shadow-2xl overflow-hidden transition-all duration-200 ${
                    isOpen ? 'translate-y-0 scale-100' : '-translate-y-4 scale-95'
                }`}
            >
                {/* Search Input */}
                <div className="flex items-center gap-3 px-4 py-3 border-b border-border">
                    <Search className="w-5 h-5 text-muted-foreground flex-shrink-0" />
                    <input
                        ref={inputRef}
                        type="search"
                        value={query}
                        onChange={e => setQuery(e.target.value)}
                        placeholder="Search contracts, publishers, templates…"
                        className="flex-1 bg-transparent text-foreground text-sm focus:outline-none placeholder:text-muted-foreground"
                    />
                    <button
                        onClick={onClose}
                        className="flex items-center gap-1 px-2 py-1 rounded-md border border-border text-xs text-muted-foreground hover:border-primary/50 transition-colors"
                        aria-label="Close search"
                    >
                        <span>ESC</span>
                    </button>
                </div>

                {/* Suggestions */}
                <div className="p-2">
                    <p className="px-3 py-1.5 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
                        {query ? 'Results' : 'Popular searches'}
                    </p>
                    <ul>
                        {suggestions.map(({ label, href, icon: Icon }) => (
                            <li key={href}>
                                <Link
                                    href={`${href}${query ? `&q=${encodeURIComponent(query)}` : ''}`}
                                    onClick={onClose}
                                    className="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors group"
                                >
                                    <span className="w-7 h-7 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0 group-hover:bg-primary/20 transition-colors">
                                        <Icon className="w-3.5 h-3.5 text-primary" />
                                    </span>
                                    {label}
                                </Link>
                            </li>
                        ))}
                    </ul>
                </div>

                {/* Footer */}
                <div className="px-4 py-2.5 border-t border-border bg-accent/30 flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">
                        Press <kbd className="px-1.5 py-0.5 rounded border border-border bg-card text-foreground font-mono text-[10px]">↑↓</kbd> to navigate
                    </span>
                    <Link href={`/contracts${query ? `?q=${encodeURIComponent(query)}` : ''}`} onClick={onClose} className="text-xs text-primary hover:underline">
                        Browse all →
                    </Link>
                </div>
            </div>
        </div>
    );
}

/* ─── Logo mark — an abstract soroban (abacus) glyph. "Soroban" is the
   Japanese word for abacus, which is also why Stellar named its smart-
   contracts platform Soroban — so the rods + beads are a literal nod to
   the name, not just a generic icon. The bead color reuses --primary,
   the same gold used for the nav's active underline and the Publish
   CTA's badge, so the mark and the rest of the chrome read as one
   system rather than a logo bolted onto an unrelated palette. ─── */
function LogoMark({ className = 'w-8 h-8' }: { className?: string }) {
    return (
        <span className={`relative flex items-center justify-center rounded-lg bg-foreground flex-shrink-0 ${className}`}>
            <svg viewBox="0 0 24 24" className="w-[62%] h-[62%]" fill="none" aria-hidden="true">
                <line x1="3.5" y1="8" x2="20.5" y2="8" strokeWidth="2" strokeLinecap="round" className="stroke-background/40" />
                <line x1="3.5" y1="16" x2="20.5" y2="16" strokeWidth="2" strokeLinecap="round" className="stroke-background/40" />
                <circle cx="9" cy="8" r="2.75" className="fill-primary" />
                <circle cx="15" cy="16" r="2.75" className="fill-primary" />
            </svg>
        </span>
    );
}

/* ─── Publish CTA — black pill with a trailing gold arrow badge ─── */
function PublishCta({ onClick, size = 'sm' }: { onClick?: () => void; size?: 'sm' | 'lg' }) {
    const isLg = size === 'lg';
    return (
        <Link
            href="/publish"
            onClick={onClick}
            className={`group inline-flex items-center rounded-full bg-foreground text-background font-semibold hover:opacity-90 transition-opacity ${
                isLg ? 'justify-center gap-2.5 pl-5 pr-2 py-2 text-sm w-full' : 'gap-2 pl-4 pr-1.5 py-1.5 text-[13px]'
            }`}
        >
            Publish
            <span
                className={`flex items-center justify-center rounded-full bg-primary text-primary-foreground flex-shrink-0 motion-safe:group-hover:scale-105 transition-transform ${
                    isLg ? 'w-7 h-7' : 'w-6 h-6'
                }`}
            >
                <ArrowUpRight className={isLg ? 'w-4 h-4' : 'w-3.5 h-3.5'} strokeWidth={2.5} />
            </span>
        </Link>
    );
}

/* ─── Navbar ────────────────────────────────────────────────── */
export default function Navbar() {
    const { t, i18n } = useTranslation('en');
    const lng = i18n.resolvedLanguage || 'en';
    const pathname = usePathname() ?? '';
    const scrolled = useScrolled();
    const { favoritesCount } = useFavorites();

    const [mobileOpen,    setMobileOpen]    = useState(false);
    const [exploreOpen,   setExploreOpen]   = useState(false);
    const [searchOpen,    setSearchOpen]    = useState(false);

    const exploreTimeout = useRef<NodeJS.Timeout | null>(null);
    const exploreRef     = useRef<HTMLDivElement>(null);
    const drawerRef      = useRef<HTMLDivElement>(null);

    // Close mobile menu on route change
    useEffect(() => { setTimeout(() => setMobileOpen(false), 0); }, [pathname]);

    // Prevent body scroll when mobile drawer open
    useEffect(() => {
        document.body.style.overflow = mobileOpen ? 'hidden' : '';
        return () => { document.body.style.overflow = ''; };
    }, [mobileOpen]);

    // ESC closes mobile drawer, trap focus while it's open
    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setMobileOpen(false);
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                setSearchOpen(true);
            }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, []);
    useTrapFocus(drawerRef, mobileOpen);

    // Explore is opened by hover for pointer users, but must also work by
    // click/keyboard (Enter/Space on the button) — close on outside click
    // or Escape so it doesn't get stuck open for keyboard users either.
    useEffect(() => {
        if (!exploreOpen) return;
        const onClickOutside = (e: MouseEvent) => {
            if (exploreRef.current && !exploreRef.current.contains(e.target as Node)) {
                setExploreOpen(false);
            }
        };
        const onEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setExploreOpen(false);
        };
        document.addEventListener('mousedown', onClickOutside);
        document.addEventListener('keydown', onEscape);
        return () => {
            document.removeEventListener('mousedown', onClickOutside);
            document.removeEventListener('keydown', onEscape);
        };
    }, [exploreOpen]);

    const isActive = useCallback((href: string) => pathname === href, [pathname]);
    const isExploreActive = EXPLORE_LINKS.some(l => pathname.startsWith(l.href));

    /* hover helpers */
    const onExploreEnter = () => { if (exploreTimeout.current) clearTimeout(exploreTimeout.current); setExploreOpen(true);  };
    const onExploreLeave = () => { exploreTimeout.current = setTimeout(() => setExploreOpen(false), 150); };
    const onExploreToggle = () => { if (exploreTimeout.current) clearTimeout(exploreTimeout.current); setExploreOpen(v => !v); };

    const navLinkClass = (active: boolean) =>
        `pb-[3px] border-b-2 text-[14px] font-medium transition-colors ${
            active
                ? 'text-foreground border-primary'
                : 'text-foreground/65 border-transparent hover:text-foreground'
        }`;

    return (
        <>
            {/* ── Main nav bar ───────────────────────────────────────── */}
            <nav
                className={`sticky top-0 z-50 w-full bg-background/95 backdrop-blur-xl border-b border-border transition-shadow duration-300 ${
                    scrolled ? 'shadow-sm shadow-black/5' : ''
                }`}
                aria-label="Main navigation"
            >
                <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div className="flex items-center justify-between h-16">

                        {/* Logo — abacus mark + name as one lockup, Stellar-style */}
                        <Link href="/" className="group flex items-center gap-2.5 flex-shrink-0" aria-label="Soroban Registry home">
                            <LogoMark className="w-8 h-8 transition-transform motion-safe:group-hover:scale-105" />
                            <span className="text-lg font-bold text-foreground tracking-tight hidden sm:block">
                                Soroban
                            </span>
                        </Link>

                        {/* ── Desktop nav links ─────────────────────────── */}
                        <div className="hidden lg:flex items-center gap-5 xl:gap-7" role="menubar" aria-label="Site navigation">

                            {NAV_LINKS.map(({ href, label }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    role="menuitem"
                                    className={navLinkClass(isActive(href))}
                                    aria-current={isActive(href) ? 'page' : undefined}
                                >
                                    {t(`navbar.${label.toLowerCase()}`, label)}
                                </Link>
                            ))}

                            {/* Explore dropdown */}
                            <div
                                ref={exploreRef}
                                className="relative"
                                onMouseEnter={onExploreEnter}
                                onMouseLeave={onExploreLeave}
                                role="none"
                            >
                                <button
                                    type="button"
                                    onClick={onExploreToggle}
                                    role="menuitem"
                                    aria-haspopup="true"
                                    aria-expanded={exploreOpen}
                                    className={`flex items-center gap-1 focus:outline-none ${navLinkClass(isExploreActive || exploreOpen)}`}
                                >
                                    Explore
                                    <ChevronDown className={`w-3.5 h-3.5 transition-transform duration-200 ${exploreOpen ? 'rotate-180' : ''}`} />
                                </button>

                                <div
                                    role="menu"
                                    className={`absolute top-full left-1/2 -translate-x-1/2 pt-4 transition-all duration-150 ${
                                        exploreOpen
                                            ? 'opacity-100 translate-y-0 pointer-events-auto'
                                            : 'opacity-0 -translate-y-2 pointer-events-none'
                                    }`}
                                >
                                    <div className="w-64 rounded-xl border border-border bg-card shadow-xl shadow-black/10 overflow-hidden p-3">
                                        <p className="px-2 pt-1 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
                                            Explore
                                        </p>
                                        {EXPLORE_LINKS.map(({ href, label, icon: Icon }) => (
                                            <Link
                                                key={href}
                                                href={href}
                                                role="menuitem"
                                                aria-current={isActive(href) ? 'page' : undefined}
                                                className={`flex items-center gap-3 px-2 py-2 rounded-lg transition-colors ${
                                                    isActive(href) ? 'bg-accent' : 'hover:bg-accent'
                                                }`}
                                            >
                                                <span className="w-9 h-9 rounded-lg border-2 border-border-strong flex items-center justify-center flex-shrink-0">
                                                    <Icon className="w-4 h-4 text-primary" />
                                                </span>
                                                <span className={`text-sm font-semibold ${isActive(href) ? 'text-primary' : 'text-foreground'}`}>
                                                    {label}
                                                </span>
                                            </Link>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            <Link
                                href="/graph"
                                role="menuitem"
                                className={navLinkClass(isActive('/graph'))}
                                aria-current={isActive('/graph') ? 'page' : undefined}
                            >
                                Graph
                            </Link>
                        </div>

                        {/* ── Desktop right actions ────────────────────── */}
                        <div className="hidden lg:flex items-center gap-1.5 xl:gap-2">
                            {/* Search */}
                            <button
                                onClick={() => setSearchOpen(true)}
                                className="flex items-center justify-center w-9 h-9 rounded-full bg-muted hover:bg-accent transition-colors"
                                aria-label="Open search (⌘K)"
                            >
                                <Search className="w-4 h-4 text-foreground" />
                            </button>

                            <LanguageSelector lng={lng} />
                            <ThemeToggle />
                            <NotificationBell />

                            {/* Favorites link */}
                            <Link
                                href="/favorites"
                                aria-label="Your favorites"
                                className={`relative flex items-center justify-center w-9 h-9 rounded-full transition-colors ${
                                    isActive('/favorites')
                                        ? 'text-primary bg-accent'
                                        : 'text-foreground/70 hover:text-foreground hover:bg-accent'
                                }`}
                            >
                                <Star className="w-[18px] h-[18px]" />
                                {favoritesCount > 0 && (
                                    <span className="absolute top-0.5 right-0.5 flex items-center justify-center min-w-[1rem] h-4 px-0.5 text-[10px] font-bold text-primary-foreground bg-primary rounded-full">
                                        {favoritesCount > 99 ? '99+' : favoritesCount}
                                    </span>
                                )}
                            </Link>

                            {/* Settings link */}
                            <Link
                                href="/settings"
                                aria-label="Settings"
                                className={`flex items-center justify-center w-9 h-9 rounded-full transition-colors ${
                                    isActive('/settings')
                                        ? 'text-primary bg-accent'
                                        : 'text-foreground/70 hover:text-foreground hover:bg-accent'
                                }`}
                            >
                                <Settings className="w-[18px] h-[18px]" />
                            </Link>

                            <div className="ml-1">
                                <PublishCta />
                            </div>
                        </div>

                        {/* ── Mobile actions row ───────────────────────── */}
                        <div className="flex lg:hidden items-center gap-1">
                            {/* Mobile search button */}
                            <button
                                onClick={() => setSearchOpen(true)}
                                className="flex items-center justify-center w-9 h-9 rounded-full hover:bg-accent transition-colors"
                                aria-label="Open search"
                            >
                                <Search className="w-[18px] h-[18px] text-foreground" />
                            </button>

                            <LanguageSelector lng={lng} />
                            <ThemeToggle />

                            {/* Hamburger / close */}
                            <button
                                onClick={() => setMobileOpen(v => !v)}
                                className="flex items-center justify-center w-9 h-9 rounded-full hover:bg-accent transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50"
                                aria-label={mobileOpen ? 'Close menu' : 'Open menu'}
                                aria-expanded={mobileOpen}
                                aria-controls="mobile-nav-drawer"
                            >
                                <span className="relative w-5 h-5 flex items-center justify-center">
                                    <Menu
                                        className={`w-5 h-5 absolute transition-all duration-200 ${
                                            mobileOpen ? 'opacity-0 rotate-90 scale-75' : 'opacity-100 rotate-0 scale-100'
                                        }`}
                                    />
                                    <X
                                        className={`w-5 h-5 absolute transition-all duration-200 ${
                                            mobileOpen ? 'opacity-100 rotate-0 scale-100' : 'opacity-0 -rotate-90 scale-75'
                                        }`}
                                    />
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            </nav>

            {/* ── Mobile drawer overlay + panel ──────────────────────── */}
            <div
                id="mobile-nav-drawer"
                className={`fixed inset-0 z-[100] lg:hidden transition-all duration-300 ${
                    mobileOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
                }`}
                aria-hidden={!mobileOpen}
                inert={!mobileOpen}
            >
                {/* Backdrop */}
                <div
                    className="absolute inset-0 bg-background/60 backdrop-blur-sm"
                    onClick={() => setMobileOpen(false)}
                    aria-hidden="true"
                />

                {/* Slide-out panel */}
                <div
                    ref={drawerRef}
                    role="dialog"
                    aria-modal="true"
                    aria-label="Mobile navigation menu"
                    className={`absolute inset-y-0 right-0 w-[80vw] max-w-sm flex flex-col bg-card border-l border-border shadow-2xl transition-transform duration-300 ease-in-out ${
                        mobileOpen ? 'translate-x-0' : 'translate-x-full'
                    }`}
                >
                    {/* Drawer header */}
                    <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                        <Link href="/" className="flex items-center gap-2.5" onClick={() => setMobileOpen(false)}>
                            <LogoMark className="w-7 h-7" />
                            <span className="text-base font-bold text-foreground tracking-tight">
                                Soroban
                            </span>
                        </Link>
                        <button
                            onClick={() => setMobileOpen(false)}
                            className="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors focus:outline-none"
                            aria-label="Close menu"
                        >
                            <X className="w-5 h-5" />
                        </button>
                    </div>

                    {/* Quick links section */}
                    <div className="px-3 pt-3 pb-2">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Quick Links</p>
                        <div className="grid grid-cols-2 gap-1">
                            {[
                                { href: '/contracts', label: 'Browse', icon: Search },
                                { href: '/compare', label: 'Compare', icon: Columns2 },
                                { href: '/marketplace', label: 'Market', icon: Store },
                                { href: '/verify-contract', label: 'Verify', icon: ShieldCheck },
                                { href: '/stats', label: 'Statistics', icon: BarChart2 },
                                { href: '/analytics', label: 'Analytics', icon: PieChart },
                                { href: '/templates', label: 'Templates', icon: Layers },
                                { href: '/graph', label: 'Dependency Graph', icon: GitBranch },
                                { href: '/developer', label: 'IDE', icon: Code2 },
                            ].map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={`${href}-${label}`}
                                    href={href}
                                    onClick={() => setMobileOpen(false)}
                                    className={`flex items-center gap-2 px-3 py-2.5 rounded-lg text-sm font-medium border transition-all ${
                                        isActive(href)
                                            ? 'text-foreground border-border-strong bg-accent'
                                            : 'text-foreground/70 border-transparent hover:text-foreground hover:bg-accent'
                                    }`}
                                >
                                    <Icon className="w-4 h-4" />
                                    {label}
                                </Link>
                            ))}
                        </div>
                    </div>

                    {/* Divider */}
                    <div className="mx-3 border-t border-border" />

                    {/* All nav links */}
                    {/* Navigation links */}
                    <div className="flex-1 overflow-y-auto py-3 px-3">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Navigation</p>
                        <nav className="flex flex-col gap-0.5" aria-label="Mobile navigation links">
                            {[
                                { href: '/contracts',       label: 'Browse',            icon: Package    },
                                { href: '/compare',         label: 'Compare',           icon: Columns2   },
                                { href: '/verify-contract', label: 'Verify',            icon: ShieldCheck},
                                { href: '/stats',           label: 'Statistics',        icon: BarChart2  },
                                { href: '/analytics',       label: 'Analytics',         icon: PieChart   },
                                { href: '/templates',       label: 'Templates',         icon: Layers     },
                                { href: '/graph',           label: 'Dependency Graph',  icon: GitBranch  },
                                { href: '/favorites',       label: 'My Favorites',      icon: Star       },
                            ].map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    onClick={() => setMobileOpen(false)}
                                    className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
                                        pathname === href
                                            ? 'text-foreground'
                                            : 'text-foreground/70 hover:text-foreground hover:bg-accent'
                                    }`}
                                    aria-current={pathname === href ? 'page' : undefined}
                                >
                                    <span className={`w-7 h-7 rounded-lg border-2 flex items-center justify-center flex-shrink-0 transition-colors ${
                                        pathname === href ? 'border-primary text-primary' : 'border-border-strong/25 text-foreground/60'
                                    }`}>
                                        <Icon className="w-3.5 h-3.5" />
                                    </span>
                                    {label}
                                    {pathname === href && (
                                        <span className="ml-auto w-1.5 h-1.5 rounded-full bg-primary" />
                                    )}
                                </Link>
                            ))}
                        </nav>
                    </div>

                    {/* Footer actions — open to everyone, no account/sign-in concept */}
                    <div className="border-t border-border p-4">
                        <div className="flex items-center gap-2">
                            <PublishCta size="lg" onClick={() => setMobileOpen(false)} />
                            <Link
                                href="/settings"
                                onClick={() => setMobileOpen(false)}
                                aria-label="Settings"
                                className="flex items-center justify-center w-11 h-11 rounded-full border-2 border-border-strong text-foreground hover:bg-accent transition-colors flex-shrink-0"
                            >
                                <Settings className="w-4 h-4" />
                            </Link>
                        </div>
                    </div>
                </div>
            </div>

            {/* ── Global search modal ────────────────────────────────── */}
            <SearchModal isOpen={searchOpen} onClose={() => setSearchOpen(false)} />
        </>
    );
}
