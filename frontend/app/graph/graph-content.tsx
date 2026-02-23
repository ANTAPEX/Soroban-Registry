'use client';

import { useQuery } from '@tanstack/react-query';
import { api, GraphNode, GraphEdge } from '@/lib/api';
import DependencyGraph from '@/components/DependencyGraph';
import GraphControls from '@/components/GraphControls';
import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { AlertCircle, Sparkles, ExternalLink, X } from 'lucide-react';
import { useAnalytics } from '@/hooks/useAnalytics';
import type { DependencyGraphHandle } from '@/components/DependencyGraph';

// Generate synthetic demo data for testing at scale
function generateDemoData(nodeCount: number): { nodes: GraphNode[]; edges: GraphEdge[] } {
    const networks: ('mainnet' | 'testnet' | 'futurenet')[] = ['mainnet', 'testnet', 'futurenet'];
    const categories = ['DeFi', 'NFT', 'DAO', 'Oracle', 'Bridge', 'DEX', 'Lending', 'Staking', 'Wallet', 'Token'];
    const tagOptions = ['soroban', 'stellar', 'defi', 'amm', 'lending', 'governance', 'token', 'nft', 'oracle', 'bridge'];
    const nameAdjectives = ['Swift', 'Quantum', 'Solar', 'Stellar', 'Bright', 'Nova', 'Cosmic', 'Nebula', 'Astral', 'Lunar'];
    const nameNouns = ['Swap', 'Vault', 'Pool', 'Bridge', 'Oracle', 'Token', 'Lend', 'Stake', 'DAO', 'Mint'];

    const nodes: GraphNode[] = [];
    for (let i = 0; i < nodeCount; i++) {
        const adj = nameAdjectives[Math.floor(Math.random() * nameAdjectives.length)];
        const noun = nameNouns[Math.floor(Math.random() * nameNouns.length)];
        const tagCount = 1 + Math.floor(Math.random() * 3);
        const tags: string[] = [];
        for (let t = 0; t < tagCount; t++) {
            const tag = tagOptions[Math.floor(Math.random() * tagOptions.length)];
            if (!tags.includes(tag)) tags.push(tag);
        }
        nodes.push({
            id: `demo-${i}`,
            contract_id: `C${Array.from({ length: 55 }, () => 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'[Math.floor(Math.random() * 32)]).join('')}`,
            name: `${adj}${noun}${i > 0 ? i : ''}`,
            network: networks[Math.floor(Math.random() * networks.length)],
            is_verified: Math.random() > 0.6,
            category: categories[Math.floor(Math.random() * categories.length)],
            tags,
        });
    }

    // Create edges — power-law distribution: some nodes get many dependents
    const edges: GraphEdge[] = [];
    const edgeCount = Math.min(nodeCount * 2, nodeCount * (nodeCount - 1) / 2);
    const edgeSet = new Set<string>();
    for (let i = 0; i < edgeCount; i++) {
        // Bias towards lower-index nodes as targets to create hub nodes
        const sourceIdx = Math.floor(Math.random() * nodeCount);
        const targetIdx = Math.floor(Math.pow(Math.random(), 2) * nodeCount);
        if (sourceIdx === targetIdx) continue;
        const key = `${sourceIdx}-${targetIdx}`;
        if (edgeSet.has(key)) continue;
        edgeSet.add(key);
        edges.push({
            source: nodes[sourceIdx].id,
            target: nodes[targetIdx].id,
            dependency_type: Math.random() > 0.7 ? 'imports' : 'calls',
        });
    }

    return { nodes, edges };
}

export function GraphContent() {
    const [networkFilter, setNetworkFilter] = useState<string>('');
    const [searchQuery, setSearchQuery] = useState('');
    const [demoMode, setDemoMode] = useState(false);
    const [demoNodeCount, setDemoNodeCount] = useState(200);
    const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
    const [searchMatchIndex, setSearchMatchIndex] = useState(0);
    const graphRef = useRef<DependencyGraphHandle | null>(null);
    const { logEvent } = useAnalytics();

    const { data: apiData, isLoading, error } = useQuery({
        queryKey: ['contract-graph', networkFilter],
        queryFn: () => api.getContractGraph(networkFilter || undefined),
        enabled: !demoMode,
    });

    useEffect(() => {
        if (!error) return;
        logEvent('error_event', {
            source: 'graph_page',
            message: 'Failed to load contract graph data',
            network_filter: networkFilter || 'all',
        });
    }, [error, networkFilter, logEvent]);

    useEffect(() => {
        if (!searchQuery.trim()) return;
        logEvent('search_performed', {
            keyword: searchQuery.trim(),
            source: 'graph_page',
            network_filter: networkFilter || 'all',
            demo_mode: demoMode,
        });
    }, [searchQuery, networkFilter, demoMode, logEvent]);

    const demoData = useMemo(
        () => (demoMode ? generateDemoData(demoNodeCount) : null),
        [demoMode, demoNodeCount]
    );

    // Apply client-side network filtering for demo mode
    const filteredDemoData = useMemo(() => {
        if (!demoData || !networkFilter) return demoData;
        const filteredNodes = demoData.nodes.filter((n) => n.network === networkFilter);
        const nodeIds = new Set(filteredNodes.map((n) => n.id));
        const filteredEdges = demoData.edges.filter(
            (e) => nodeIds.has(e.source) && nodeIds.has(e.target)
        );
        return { nodes: filteredNodes, edges: filteredEdges };
    }, [demoData, networkFilter]);

    const graphData = demoMode ? filteredDemoData : apiData;

    // Safe nodes/edges (API may return missing or non-array values)
    const nodes = useMemo(
        () => (graphData && Array.isArray(graphData.nodes) ? graphData.nodes : []),
        [graphData]
    );
    const edges = useMemo(
        () => (graphData && Array.isArray(graphData.edges) ? graphData.edges : []),
        [graphData]
    );

    // Compute dependent counts (how many nodes depend on this one = in-edges)
    const dependentCounts = useMemo(() => {
        const counts = new Map<string, number>();
        for (const edge of edges) {
            counts.set(edge.target, (counts.get(edge.target) || 0) + 1);
        }
        return counts;
    }, [edges]);

    // Compute dependency counts (how many nodes this one depends on = out-edges)
    const dependencyCounts = useMemo(() => {
        const counts = new Map<string, number>();
        for (const edge of edges) {
            counts.set(edge.source, (counts.get(edge.source) || 0) + 1);
        }
        return counts;
    }, [edges]);

    const criticalCount = useMemo(() => {
        let count = 0;
        dependentCounts.forEach((v) => { if (v >= 5) count++; });
        return count;
    }, [dependentCounts]);

    // Per-network node counts for the stats panel
    const networkCounts = useMemo(() => {
        const counts = { mainnet: 0, testnet: 0, futurenet: 0, other: 0 };
        for (const node of nodes) {
            const n = node.network?.toLowerCase() ?? "";
            if (n === "mainnet") counts.mainnet++;
            else if (n === "testnet") counts.testnet++;
            else if (n === "futurenet") counts.futurenet++;
            else counts.other++;
        }
        return counts;
    }, [nodes]);

    const handleNodeClick = useCallback((node: GraphNode | null) => {
        setSelectedNode(node);
    }, []);

    // Search match navigation
    const searchMatches = useMemo(() => {
        if (!searchQuery || nodes.length === 0) return [];
        const q = searchQuery.toLowerCase();
        return nodes
            .filter((n) => n.name.toLowerCase().includes(q) || n.contract_id.toLowerCase().includes(q))
            .map((n) => n.id);
    }, [searchQuery, nodes]);

    // Reset match index when query or matches change
    useEffect(() => {
        setSearchMatchIndex(0);
    }, [searchQuery]);

    // Auto-focus on the active match
    useEffect(() => {
        if (searchMatches.length > 0 && graphRef.current) {
            graphRef.current.focusOnNode(searchMatches[searchMatchIndex] || searchMatches[0]);
        }
    }, [searchMatches, searchMatchIndex]);

    const handlePrevMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i - 1 + searchMatches.length) % searchMatches.length);
    }, [searchMatches.length]);

    const handleNextMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i + 1) % searchMatches.length);
    }, [searchMatches.length]);

    // Keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Ignore when typing in inputs
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
            const g = graphRef.current;
            if (!g) return;
            switch (e.key) {
                case '=':
                case '+':
                    e.preventDefault();
                    g.zoomIn();
                    break;
                case '-':
                    e.preventDefault();
                    g.zoomOut();
                    break;
                case 'r':
                case 'R':
                    e.preventDefault();
                    g.resetZoom();
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    g.panUp();
                    break;
                case 'ArrowDown':
                    e.preventDefault();
                    g.panDown();
                    break;
                case 'ArrowLeft':
                    e.preventDefault();
                    g.panLeft();
                    break;
                case 'ArrowRight':
                    e.preventDefault();
                    g.panRight();
                    break;
                case 'Escape':
                    setSelectedNode(null);
                    break;
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, []);

    if (!demoMode && isLoading) {
        return (
            <div className="flex items-center justify-center h-[calc(100vh-4rem)] bg-background">
                <div className="text-center">
                    <div className="inline-block w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin mb-4" />
                    <p className="text-muted-foreground text-sm">Loading graph data…</p>
                </div>
            </div>
        );
    }

    if (!demoMode && error) {
        return (
            <div className="relative h-[calc(100vh-4rem)] overflow-hidden bg-background">
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center bg-background/95 backdrop-blur-xl border border-border rounded-xl p-10 shadow-2xl max-w-md">
                        <AlertCircle className="w-12 h-12 text-amber-500 mx-auto mb-4" />
                        <h3 className="text-xl font-semibold text-foreground mb-2">Backend API unavailable</h3>
                        <p className="text-muted-foreground mb-6 text-sm">
                            Could not load contract graph data. You can still explore the visualization using Demo Mode with synthetic data.
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="px-6 py-2.5 bg-primary hover:opacity-90 text-primary-foreground rounded-lg font-medium transition-opacity inline-flex items-center gap-2"
                        >
                            <Sparkles className="w-4 h-4" />
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="relative h-[calc(100vh-4rem)] overflow-hidden bg-background">
            {/* Graph Canvas — render actual graph (empty when no data) */}
            <div className="w-full h-full bg-muted/50 relative">
                <DependencyGraph
                    ref={graphRef}
                    nodes={nodes}
                    edges={edges}
                    searchQuery={searchQuery}
                    dependentCounts={dependentCounts}
                    onNodeClick={handleNodeClick}
                    selectedNode={selectedNode}
                />
            </div>

            {/* Controls Overlay */}
            <GraphControls
                searchQuery={searchQuery}
                onSearchChange={setSearchQuery}
                networkFilter={networkFilter}
                onNetworkFilterChange={setNetworkFilter}
                demoMode={demoMode}
                onDemoModeChange={setDemoMode}
                demoNodeCount={demoNodeCount}
                onDemoNodeCountChange={setDemoNodeCount}
                totalNodes={nodes.length}
                totalEdges={edges.length}
                criticalCount={criticalCount}
                searchMatchCount={searchMatches.length}
                searchMatchIndex={searchMatchIndex}
                onPrevMatch={handlePrevMatch}
                onNextMatch={handleNextMatch}
                onZoomIn={() => graphRef.current?.zoomIn()}
                onZoomOut={() => graphRef.current?.zoomOut()}
                onResetZoom={() => graphRef.current?.resetZoom()}
                onExportSVG={() => graphRef.current?.exportSVG()}
                onExportPNG={() => graphRef.current?.exportPNG()}
                onPanUp={() => graphRef.current?.panUp()}
                onPanDown={() => graphRef.current?.panDown()}
                onPanLeft={() => graphRef.current?.panLeft()}
                onPanRight={() => graphRef.current?.panRight()}
                networkCounts={networkCounts}
            />

            {/* Selected Node Panel */}
            {selectedNode && (
                <div className="absolute bottom-4 left-4 z-30 w-80 bg-background/95 backdrop-blur-xl border border-border rounded-xl p-5 shadow-2xl">
                    {/* Header */}
                    <div className="flex items-start justify-between mb-2">
                        <div className="flex-1 min-w-0 pr-2">
                            <h3 className="font-semibold text-foreground text-base truncate">{selectedNode.name}</h3>
                            <p className="text-[10px] text-muted-foreground font-mono truncate mt-0.5">{selectedNode.contract_id}</p>
                        </div>
                        <button
                            onClick={() => setSelectedNode(null)}
                            className="text-muted-foreground hover:text-foreground transition-colors shrink-0 p-1 rounded hover:bg-muted focus-visible:ring-1 focus-visible:ring-primary focus:outline-none"
                            aria-label="Close panel"
                        >
                            <X className="w-4 h-4" />
                        </button>
                    </div>

                    {/* Stats row */}
                    <div className="grid grid-cols-3 gap-2 mb-3 mt-3">
                        <div className="bg-muted/70 rounded-lg p-2 text-center">
                            <div className="text-lg font-bold text-foreground">{dependentCounts.get(selectedNode.id) || 0}</div>
                            <div className="text-[10px] text-muted-foreground">Dependents</div>
                        </div>
                        <div className="bg-muted/70 rounded-lg p-2 text-center">
                            <div className="text-lg font-bold text-foreground">{dependencyCounts.get(selectedNode.id) || 0}</div>
                            <div className="text-[10px] text-muted-foreground">Dependencies</div>
                        </div>
                        <div className="bg-muted/70 rounded-lg p-2 text-center">
                            <div className={`text-sm font-bold ${selectedNode.is_verified ? 'text-green-600 dark:text-green-400' : 'text-muted-foreground'}`}>
                                {selectedNode.is_verified ? '✓' : '—'}
                            </div>
                            <div className="text-[10px] text-muted-foreground">Verified</div>
                        </div>
                    </div>

                    {/* Details */}
                    <div className="space-y-1.5 text-sm">
                        <div className="flex justify-between">
                            <span className="text-muted-foreground">Network</span>
                            <span className={`font-medium ${selectedNode.network === 'mainnet' ? 'text-green-600 dark:text-green-400' :
                                selectedNode.network === 'testnet' ? 'text-blue-600 dark:text-blue-400' : 'text-purple-600 dark:text-purple-400'
                                }`}>{selectedNode.network}</span>
                        </div>
                        {selectedNode.category && (
                            <div className="flex justify-between">
                                <span className="text-muted-foreground">Type</span>
                                <span className="text-foreground">{selectedNode.category}</span>
                            </div>
                        )}
                    </div>

                    {/* Tags */}
                    {selectedNode.tags.length > 0 && (
                        <div className="pt-2 mt-2 border-t border-border">
                            <div className="flex flex-wrap gap-1">
                                {selectedNode.tags.map((tag) => (
                                    <span key={tag} className="px-1.5 py-0.5 bg-primary/10 text-primary border border-primary/20 rounded text-[10px]">
                                        {tag}
                                    </span>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Link */}
                    <a
                        href={`/contracts/${selectedNode.contract_id}`}
                        className="mt-3 flex items-center justify-center gap-1.5 w-full py-1.5 bg-primary/10 hover:bg-primary/20 border border-primary/30 text-primary rounded-lg text-xs font-medium transition-colors focus-visible:ring-1 focus-visible:ring-primary focus:outline-none"
                    >
                        <ExternalLink className="w-3 h-3" />
                        View Contract Details
                    </a>
                </div>
            )}

            {/* Empty State */}
            {!demoMode && nodes.length === 0 && !isLoading && (
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center bg-background/90 backdrop-blur-xl border border-border rounded-xl p-10">
                        <Sparkles className="w-12 h-12 text-primary mx-auto mb-4" />
                        <h3 className="text-xl font-semibold text-foreground mb-2">No contracts yet</h3>
                        <p className="text-muted-foreground mb-4">
                            Publish some contracts or enable Demo Mode to explore the graph
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="px-6 py-2.5 bg-primary hover:opacity-90 text-primary-foreground rounded-lg font-medium transition-opacity"
                        >
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}
