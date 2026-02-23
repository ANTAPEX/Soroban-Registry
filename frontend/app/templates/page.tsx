'use client';

import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api';
import TemplateGallery from '@/components/TemplateGallery';
import { Sparkles, Terminal } from 'lucide-react';
import Navbar from '@/components/Navbar';
import { useAnalytics } from '@/hooks/useAnalytics';
import { useEffect } from 'react';

export default function TemplatesPage() {
    const { data: templates, isLoading, error } = useQuery({
        queryKey: ['templates'],
        queryFn: () => api.getTemplates(),
    });
    const { logEvent } = useAnalytics();

    useEffect(() => {
        if (!error) return;
        logEvent('error_event', {
            source: 'templates_page',
            message: 'Failed to load templates',
        });
    }, [error, logEvent]);

    return (
        <div className="min-h-screen bg-gradient-to-br from-gray-50 via-blue-50/30 to-purple-50/30 dark:from-gray-950 dark:via-blue-900/10 dark:to-purple-900/10">
            <Navbar />

            <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
                <div className="mb-12">
                    <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 text-sm font-medium mb-4">
                        <Sparkles className="w-4 h-4" />
                        Contract Blueprints
                    </div>
                    <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-4">
                        Template Gallery
                    </h1>
                    <p className="text-lg text-gray-600 dark:text-gray-300 max-w-2xl">
                        Scaffold production-ready Soroban contracts in seconds. Pick a template, customise parameters, and start building.
                    </p>

                    <div className="mt-6 p-4 rounded-xl bg-gray-900 dark:bg-gray-950 border border-gray-700">
                        <div className="flex items-center gap-2 mb-2 text-gray-400 text-xs">
                            <Terminal className="w-4 h-4" />
                            <span>Quick start</span>
                        </div>
                        <code className="text-green-400 text-sm font-mono">
                            soroban-registry template list<br />
                            soroban-registry template clone token my-token --symbol TKN --initial-supply 1000000
                        </code>
                    </div>
                </div>

                {isLoading ? (
                    <TemplateGallery templates={[]} isLoading={true} />
                ) : (
                    <TemplateGallery templates={templates ?? []} isLoading={false} />
                )}
            </section>
        </div>
    );
}
