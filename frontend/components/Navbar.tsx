'use client';

import React from 'react';
import Link from 'next/link';
import { Package, Search, Plus, Star, Menu, X, Home, ChevronRight } from 'lucide-react';
import { useTranslation } from '@/lib/i18n/client';
import ThemeToggle from './ThemeToggle';
import NotificationBell from './NotificationBell';
import LanguageSelector from './LanguageSelector';
import { useNavbar } from './layout/Navbar/useNavbar';
import { NavLinks } from './layout/Navbar/NavLinks';
import { SearchModal } from './layout/Navbar/SearchModal';
import { MobileDrawer } from './layout/Navbar/MobileDrawer';
import styles from './layout/Navbar/Navbar.module.css';

const formatBreadcrumbLabel = (segment: string) =>
    decodeURIComponent(segment)
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, (c) => c.toUpperCase());

export default function Navbar() {
    const { t, i18n } = useTranslation('');
    const lng = i18n.resolvedLanguage || 'en';
    
    const {
        pathname,
        scrolled,
        favoritesCount,
        mobileOpen,
        exploreOpen,
        searchOpen,
        isActive,
        onExploreEnter,
        onExploreLeave,
        toggleMobile,
        closeMobile,
        openSearch,
        closeSearch
    } = useNavbar();

    const isExploreActive = ['/publishers', '/stats', '/analytics', '/templates'].some(href => pathname.startsWith(href));
    const mobileCrumbs = pathname.split('/').filter(Boolean).slice(0, 3);

    return (
        <>
            <nav
                className={`${styles.navBar} ${scrolled ? styles.navBarScrolled : styles.navBarDefault}`}
                aria-label="Main navigation"
            >
                <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div className="flex items-center justify-between h-14">
                        {/* Logo */}
                        <Link href="/" className="flex items-center gap-2 group flex-shrink-0">
                            <div className="w-7 h-7 rounded-md bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-sm shadow-primary/25 group-hover:shadow-primary/50 transition-shadow">
                                <Package className="w-4 h-4 text-white" />
                            </div>
                            <span className="text-base font-bold text-foreground tracking-tight hidden sm:block">
                                Soroban<span className="text-primary">Registry</span>
                            </span>
                        </Link>

                        {/* Desktop Nav */}
                        <NavLinks 
                            isActive={isActive}
                            exploreOpen={exploreOpen}
                            onExploreEnter={onExploreEnter}
                            onExploreLeave={onExploreLeave}
                            isExploreActive={isExploreActive}
                        />

                        {/* Desktop Actions */}
                        <div className="hidden md:flex items-center gap-1.5">
                            <button
                                onClick={openSearch}
                                className="hidden lg:flex items-center gap-2 h-8 w-44 px-3 rounded-md border border-border bg-background hover:bg-accent text-[13px] text-muted-foreground transition-all hover:border-primary/40 mr-1 group"
                            >
                                <Search className="w-3.5 h-3.5 flex-shrink-0" />
                                <span className="flex-1 text-left">Search…</span>
                                <kbd className="hidden xl:block px-1 py-0.5 rounded border border-border bg-accent text-[10px] font-mono text-muted-foreground">⌘K</kbd>
                            </button>

                            <LanguageSelector lng={lng} />
                            <ThemeToggle />
                            <NotificationBell />

                            <Link
                                href="/favorites"
                                className={`relative p-1.5 rounded-md transition-colors ${isActive('/favorites') ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:bg-accent'}`}
                            >
                                <Star className="w-5 h-5" />
                                {favoritesCount > 0 && (
                                    <span className="absolute top-0.5 right-0.5 flex items-center justify-center min-w-[1rem] h-4 px-0.5 text-[10px] font-bold text-primary-foreground bg-primary rounded-full">
                                        {favoritesCount > 99 ? '99+' : favoritesCount}
                                    </span>
                                )}
                            </Link>

                            <Link
                                href="/publish"
                                className="flex items-center gap-1.5 px-3.5 py-1.5 ml-1 rounded-md bg-primary text-primary-foreground text-[13px] font-semibold btn-glow transition-all hover:brightness-110"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                Publish
                            </Link>
                        </div>

                        {/* Mobile Menu Button */}
                        <div className="flex md:hidden items-center gap-1">
                            <button onClick={openSearch} className="p-2 rounded-md text-muted-foreground hover:bg-accent">
                                <Search className="w-5 h-5" />
                            </button>
                            <LanguageSelector lng={lng} />
                            <ThemeToggle />
                            <button
                                onClick={toggleMobile}
                                className="p-2 rounded-md text-muted-foreground hover:bg-accent"
                            >
                                {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
                            </button>
                        </div>
                    </div>

                    {/* Mobile Breadcrumbs */}
                    <div className="md:hidden h-10 flex items-center overflow-x-auto no-scrollbar border-t border-border/60">
                        <nav className="flex items-center gap-1 text-xs text-muted-foreground whitespace-nowrap">
                            <Link href="/" className={`inline-flex items-center gap-1 px-1.5 py-1 rounded ${pathname === '/' ? 'text-primary font-medium' : ''}`}>
                                <Home className="w-3.5 h-3.5" /> Home
                            </Link>
                            {mobileCrumbs.map((segment, index) => {
                                const href = `/${mobileCrumbs.slice(0, index + 1).join('/')}`;
                                return (
                                    <React.Fragment key={href}>
                                        <ChevronRight className="w-3 h-3 opacity-60" />
                                        <Link href={href} className={`px-1.5 py-1 rounded ${pathname === href ? 'text-primary font-medium' : ''}`}>
                                            {formatBreadcrumbLabel(segment)}
                                        </Link>
                                    </React.Fragment>
                                );
                            })}
                        </nav>
                    </div>
                </div>
            </nav>

            <MobileDrawer 
                isOpen={mobileOpen} 
                onClose={closeMobile} 
                pathname={pathname}
                favoritesCount={favoritesCount}
                isActive={isActive}
            />

            <SearchModal isOpen={searchOpen} onClose={closeSearch} />
        </>
    );
}
