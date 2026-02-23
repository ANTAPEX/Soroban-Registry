'use client';

import { Package, GitBranch, ChevronDown, BarChart2, Users } from 'lucide-react';
import Link from 'next/link';
import { useState } from 'react';
import ThemeToggle from './ThemeToggle';

export default function Navbar() {
    const [isDropdownOpen, setIsDropdownOpen] = useState(false);

    return (
        <nav className="border-b border-border bg-background/80 backdrop-blur-sm sticky top-0 z-50">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div className="flex items-center justify-between h-16">
                    <Link href="/" className="flex items-center gap-2">
                        <Package className="w-8 h-8 text-primary" />
                        <span className="text-xl font-bold bg-linear-to-r from-blue-600 to-purple-600 dark:from-blue-400 dark:to-purple-400 bg-clip-text text-transparent">
                            Soroban Registry
                        </span>
                    </Link>
                    <div className="flex items-center gap-4">
                        <Link
                            href="/contracts"
                            className="text-muted-foreground hover:text-foreground transition-colors text-sm font-medium"
                        >
                            Browse
                        </Link>
                        
                        {/* Contracts Dropdown */}
                        <div 
                            className="relative"
                            onMouseEnter={() => setIsDropdownOpen(true)}
                            onMouseLeave={() => setIsDropdownOpen(false)}
                        >
                            <button
                                className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors text-sm font-medium focus:outline-none py-4"
                            >
                                Contracts
                                <ChevronDown className={`w-4 h-4 transition-transform duration-200 ${isDropdownOpen ? 'rotate-180' : ''}`} />
                            </button>
                            
                            {/* Dropdown Menu */}
                            {isDropdownOpen && (
                                <div className="absolute top-full left-0 w-48 z-50 pt-2">
                                    <div className="bg-background rounded-md shadow-lg border border-border overflow-hidden">
                                        <div className="py-1">
                                            <Link
                                                href="/publishers/GBSX...2J4K"
                                                className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                                            >
                                                <Users className="w-4 h-4" />
                                                Publisher
                                            </Link>
                                            <Link
                                                href="/stats"
                                                className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                                            >
                                                <BarChart2 className="w-4 h-4" />
                                                Statistics
                                            </Link>
                                        </div>
                                    </div>
                                </div>
                            )}
                        </div>

                        <Link
                            href="/graph"
                            className="flex items-center gap-1.5 text-muted-foreground hover:text-foreground transition-colors text-sm font-medium"
                        >
                            <GitBranch className="w-4 h-4" />
                            Graph
                        </Link>
                        <Link
                            href="/publish"
                            className="px-4 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 transition-opacity font-medium text-sm"
                        >
                            Publish Contract
                        </Link>
                        <div className="border-l border-border pl-4 ml-2">
                            <ThemeToggle />
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    );
}
