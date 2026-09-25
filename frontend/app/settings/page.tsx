'use client';

import Navbar from '@/components/Navbar';
import { useEffect, useState } from 'react';
import { useTheme, Theme } from '@/hooks/useTheme';
import { Sun, Moon, Monitor, Globe, Palette } from 'lucide-react';
import { useTranslation } from '@/lib/i18n/client';
import { languages } from '@/lib/i18n/settings';

const LANGUAGE_LABELS: Record<string, string> = {
    en: 'English',
    es: 'Español',
    fr: 'Français',
    ar: 'العربية',
};

export default function SettingsPage() {
    const { theme, setTheme } = useTheme();
    const { i18n } = useTranslation('common');
    const currentLng = i18n.resolvedLanguage || 'en';
    const [reducedMotion, setReducedMotion] = useState(false);

    // The class is set before paint by the script in app/layout.tsx.
    useEffect(() => {
        setReducedMotion(document.documentElement.classList.contains('reduce-motion'));
    }, []);

    const toggleReducedMotion = () => {
        const next = !reducedMotion;
        setReducedMotion(next);
        document.documentElement.classList.toggle('reduce-motion', next);
        try {
            if (next) localStorage.setItem('soroban-registry-reduced-motion', '1');
            else localStorage.removeItem('soroban-registry-reduced-motion');
        } catch {
            // Storage may be unavailable; the setting still applies for this visit.
        }
    };

    const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
        { value: 'light', label: 'Light', icon: Sun },
        { value: 'dark', label: 'Dark', icon: Moon },
        { value: 'system', label: 'System', icon: Monitor },
    ];

    return (
        <div className="min-h-screen bg-background text-foreground">
            <Navbar />

            <main className="max-w-4xl mx-auto px-4 py-12 sm:px-6 lg:px-8">
                <div className="mb-10">
                    <h1 className="text-3xl font-semibold tracking-tight mb-2">Settings</h1>
                    <p className="text-muted-foreground text-lg">Manage your registry experience and preferences.</p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-[240px_1fr] gap-10">
                    {/* Sidebar Nav */}
                    <aside className="space-y-1">
                        {[
                            { label: 'Appearance', icon: Palette, href: '#appearance' },
                            { label: 'Language', icon: Globe, href: '#language' },
                        ].map((item) => (
                            <a
                                key={item.label}
                                href={item.href}
                                className="w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-accent"
                            >
                                <item.icon className="w-4 h-4" />
                                {item.label}
                            </a>
                        ))}
                    </aside>

                    {/* Content */}
                    <div className="space-y-8">
                        {/* Appearance Section */}
                        <section id="appearance" className="scroll-mt-24 bg-card border border-border rounded-lg overflow-hidden">
                            <div className="px-6 py-5 border-b border-border">
                                <h2 className="text-lg font-semibold flex items-center gap-2">
                                    <Palette className="w-5 h-5 text-primary" />
                                    Appearance
                                </h2>
                                <p className="text-sm text-muted-foreground mt-1">Customize how the registry looks for you.</p>
                            </div>

                            <div className="p-6 space-y-6">
                                <div>
                                    <label className="text-sm font-medium mb-4 block">Color theme</label>
                                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                                        {themeOptions.map((option) => (
                                            <button
                                                key={option.value}
                                                onClick={() => setTheme(option.value)}
                                                className={`flex flex-col items-center gap-3 p-4 rounded-md border-2 transition-colors group ${
                                                    theme === option.value
                                                        ? 'border-primary bg-primary/5'
                                                        : 'border-border hover:border-primary/50 hover:bg-accent'
                                                }`}
                                            >
                                                <div className={`p-3 rounded-full transition-colors ${
                                                    theme === option.value ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground group-hover:text-foreground'
                                                }`}>
                                                    <option.icon className="w-6 h-6" />
                                                </div>
                                                <span className="font-medium">{option.label}</span>
                                                {theme === option.value && (
                                                    <div className="absolute top-2 right-2 w-2 h-2 rounded-full bg-primary" />
                                                )}
                                            </button>
                                        ))}
                                    </div>
                                </div>

                                <div className="pt-4 border-t border-border">
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p id="reduced-motion-label" className="font-medium">Reduced motion</p>
                                            <p id="reduced-motion-desc" className="text-sm text-muted-foreground">Turn off animations and transitions across the interface.</p>
                                        </div>
                                        <button
                                            type="button"
                                            role="switch"
                                            aria-checked={reducedMotion}
                                            aria-labelledby="reduced-motion-label"
                                            aria-describedby="reduced-motion-desc"
                                            onClick={toggleReducedMotion}
                                            className={`w-12 h-6 shrink-0 rounded-full relative p-1 transition-colors ${reducedMotion ? 'bg-primary' : 'bg-muted-foreground/35 hover:bg-muted-foreground/50'}`}
                                        >
                                            <span
                                                className={`block w-4 h-4 bg-background rounded-full shadow-sm transition-transform ${reducedMotion ? 'translate-x-6' : 'translate-x-0'}`}
                                                aria-hidden="true"
                                            />
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </section>

                        {/* Language Section */}
                        <section id="language" className="scroll-mt-24 bg-card border border-border rounded-lg overflow-hidden">
                            <div className="px-6 py-5 border-b border-border">
                                <h2 className="text-lg font-semibold flex items-center gap-2">
                                    <Globe className="w-5 h-5 text-secondary" />
                                    Language and region
                                </h2>
                            </div>
                            <div className="p-6">
                                <p className="text-sm text-muted-foreground mb-4">Select your preferred language for the interface.</p>
                                <select
                                    value={currentLng}
                                    onChange={(e) => i18n.changeLanguage(e.target.value)}
                                    className="w-full sm:w-64 bg-background border border-border rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-primary outline-none"
                                >
                                    {languages.map((l) => (
                                        <option key={l} value={l}>
                                            {LANGUAGE_LABELS[l] || l}
                                        </option>
                                    ))}
                                </select>
                            </div>
                        </section>

                        <p className="pt-2 text-right text-sm text-muted-foreground">
                            Changes apply and save as soon as you make them.
                        </p>
                    </div>
                </div>
            </main>
        </div>
    );
}
