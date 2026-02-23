import { useState } from 'react';
import {
    Search, ZoomIn, ZoomOut, Maximize2, Download, FileImage,
    GitBranch, Circle, Sparkles, ChevronUp, ChevronDown,
    Keyboard, ChevronLeft, ChevronRight, BarChart2
} from 'lucide-react';

interface GraphControlsProps {
    searchQuery: string;
    onSearchChange: (q: string) => void;
    networkFilter: string;
    onNetworkFilterChange: (n: string) => void;
    demoMode: boolean;
    onDemoModeChange: (v: boolean) => void;
    demoNodeCount: number;
    onDemoNodeCountChange: (v: number) => void;
    totalNodes: number;
    totalEdges: number;
    criticalCount: number;
    searchMatchCount: number;
    searchMatchIndex: number;
    onPrevMatch: () => void;
    onNextMatch: () => void;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onResetZoom: () => void;
    onExportSVG: () => void;
    onExportPNG: () => void;
    onPanUp?: () => void;
    onPanDown?: () => void;
    onPanLeft?: () => void;
    onPanRight?: () => void;
    // per-network node counts for the stats panel
    networkCounts?: { mainnet: number; testnet: number; futurenet: number; other: number };
}

export default function GraphControls({
    searchQuery,
    onSearchChange,
    networkFilter,
    onNetworkFilterChange,
    demoMode,
    onDemoModeChange,
    demoNodeCount,
    onDemoNodeCountChange,
    totalNodes,
    totalEdges,
    criticalCount,
    searchMatchCount,
    searchMatchIndex,
    onPrevMatch,
    onNextMatch,
    onZoomIn,
    onZoomOut,
    onResetZoom,
    onExportSVG,
    onExportPNG,
    onPanUp,
    onPanDown,
    onPanLeft,
    onPanRight,
    networkCounts,
}: GraphControlsProps) {
    const [statsOpen, setStatsOpen] = useState(false);
    return (
        <>
            {/* Top-left: Search + Filters */}
            <div className="absolute top-4 left-4 z-30 flex flex-col gap-3 max-w-xs" role="region" aria-label="Graph search and filters">
                {/* Search */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden">
                    <div className="relative flex items-center">
                        <Search className="absolute left-3 w-4 h-4 text-gray-500" />
                        <input
                            id="graph-search"
                            type="search"
                            value={searchQuery}
                            onChange={(e) => onSearchChange(e.target.value)}
                            onKeyDown={(e) => { if (e.key === 'Enter' && searchMatchCount > 0) onNextMatch(); }}
                            placeholder="Search contracts…"
                            aria-label="Search graph nodes"
                            aria-controls="graph-search-status"
                            className="w-full pl-9 pr-3 py-2.5 bg-transparent text-sm text-white placeholder-gray-500 focus:outline-none focus-visible:ring-1 focus-visible:ring-blue-500 rounded"
                        />
                        {/* Live region for search result announcement */}
                        <span id="graph-search-status" aria-live="polite" className="sr-only">
                            {searchQuery && searchMatchCount > 0
                                ? `${searchMatchCount} match${searchMatchCount !== 1 ? "es" : ""} found, showing ${searchMatchIndex + 1}`
                                : searchQuery && searchMatchCount === 0 ? "No matches found" : ""}
                        </span>
                        {searchQuery && searchMatchCount > 0 && (
                            <div className="flex items-center gap-0.5 pr-1.5 shrink-0">
                                <span className="text-xs text-gray-400 tabular-nums px-1">
                                    {searchMatchIndex + 1}/{searchMatchCount}
                                </span>
                                <button
                                    onClick={onPrevMatch}
                                    className="p-0.5 text-gray-400 hover:text-white transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                                    aria-label="Previous search match"
                                    title="Previous match"
                                >
                                    <ChevronUp className="w-3.5 h-3.5" />
                                </button>
                                <button
                                    onClick={onNextMatch}
                                    className="p-0.5 text-gray-400 hover:text-white transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                                    aria-label="Next search match"
                                    title="Next match"
                                >
                                    <ChevronDown className="w-3.5 h-3.5" />
                                </button>
                            </div>
                        )}
                        {searchQuery && searchMatchCount === 0 && (
                            <span className="text-xs text-red-400 pr-2.5 shrink-0">No results</span>
                        )}
                    </div>
                </div>

                {/* Filters */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl space-y-3">
                    <div>
                        <label className="text-xs text-gray-400 mb-1 block">Network</label>
                        <select
                            id="graph-network-filter"
                            value={networkFilter}
                            onChange={(e) => onNetworkFilterChange(e.target.value)}
                            className="w-full px-3 py-1.5 rounded-lg bg-gray-800 border border-gray-700 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-blue-500"
                        >
                            <option value="">All Networks</option>
                            <option value="mainnet">Mainnet</option>
                            <option value="testnet">Testnet</option>
                            <option value="futurenet">Futurenet</option>
                        </select>
                    </div>

                    {/* Demo Mode */}
                    <div className="border-t border-gray-800 pt-3">
                        <label className="flex items-center gap-2 cursor-pointer">
                            <input
                                id="graph-demo-toggle"
                                type="checkbox"
                                checked={demoMode}
                                onChange={(e) => onDemoModeChange(e.target.checked)}
                                className="rounded border-gray-600 text-blue-600 focus:ring-blue-500 bg-gray-800"
                            />
                            <span className="text-sm text-gray-300 flex items-center gap-1.5">
                                <Sparkles className="w-3.5 h-3.5 text-amber-400" />
                                Demo Mode
                            </span>
                        </label>
                        {demoMode && (
                            <div className="mt-2">
                                <label className="text-xs text-gray-500 mb-1 block">Contracts: {demoNodeCount.toLocaleString()}</label>
                                <input
                                    id="graph-demo-count"
                                    type="range"
                                    min={50}
                                    max={10000}
                                    step={50}
                                    value={demoNodeCount}
                                    onChange={(e) => onDemoNodeCountChange(Number(e.target.value))}
                                    className="w-full h-1.5 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
                                />
                                <div className="flex justify-between text-[10px] text-gray-600 mt-0.5">
                                    <span>50</span>
                                    <span>10,000</span>
                                </div>
                            </div>
                        )}
                    </div>
                </div>

                {/* Legend */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl">
                    <p className="text-xs text-gray-400 mb-2 font-medium">Legend</p>
                    <div className="space-y-1.5 text-xs">
                        <p className="text-[10px] text-gray-500 uppercase tracking-wider mb-1">Network</p>
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-green-500 fill-green-500" />
                            <span className="text-gray-300">Mainnet</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-blue-500 fill-blue-500" />
                            <span className="text-gray-300">Testnet</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <Circle className="w-3 h-3 text-purple-500 fill-purple-500" />
                            <span className="text-gray-300">Futurenet</span>
                        </div>
                        <div className="border-t border-gray-800 my-1" />
                        <p className="text-[10px] text-gray-500 uppercase tracking-wider mb-1">Node Size</p>
                        <div className="flex items-center gap-2 pt-0.5">
                            <div className="w-3 h-3 rounded-full border-2 border-amber-400" />
                            <span className="text-gray-300">Critical (≥5 deps)</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-gray-400 mx-0.5" />
                            <span className="text-gray-400">Larger = more dependents</span>
                        </div>
                        <div className="border-t border-gray-800 my-1" />
                        <p className="text-[10px] text-gray-500 uppercase tracking-wider mb-1">Edges</p>
                        <div className="flex items-center gap-2">
                            <GitBranch className="w-3 h-3 text-gray-400" />
                            <span className="text-gray-400">Arrow = dependency direction</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Top-right: Graph Stats panel (collapsible) */}
            <div className="absolute top-4 right-4 z-30">
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden">
                    {/* Header row — always visible */}
                    <button
                        id="graph-stats-toggle"
                        onClick={() => setStatsOpen((o) => !o)}
                        className="flex items-center gap-3 px-4 py-2.5 w-full hover:bg-gray-800/50 transition-colors"
                        aria-expanded={statsOpen}
                        aria-controls="graph-stats-body"
                        aria-label="Toggle graph statistics panel"
                    >
                        <BarChart2 className="w-3.5 h-3.5 text-gray-400 shrink-0" />
                        <div className="flex items-center gap-3 flex-1">
                            <div className="text-center">
                                <div className="text-sm font-bold text-white leading-none">{totalNodes.toLocaleString()}</div>
                                <div className="text-[9px] text-gray-500 uppercase tracking-wider">Nodes</div>
                            </div>
                            <div className="w-px h-6 bg-gray-700" />
                            <div className="text-center">
                                <div className="text-sm font-bold text-white leading-none">{totalEdges.toLocaleString()}</div>
                                <div className="text-[9px] text-gray-500 uppercase tracking-wider">Edges</div>
                            </div>
                            <div className="w-px h-6 bg-gray-700" />
                            <div className="text-center">
                                <div className="text-sm font-bold text-amber-400 leading-none">{criticalCount}</div>
                                <div className="text-[9px] text-gray-500 uppercase tracking-wider">Critical</div>
                            </div>
                        </div>
                        <ChevronDown
                            className={`w-3 h-3 text-gray-500 transition-transform duration-200 ${statsOpen ? "rotate-180" : ""}`}
                        />
                    </button>

                    {/* Expandable body — network breakdown */}
                    {statsOpen && (
                        <div id="graph-stats-body" className="border-t border-gray-700/50 px-4 py-3 space-y-2">
                            <p className="text-[10px] text-gray-500 uppercase tracking-wider mb-2">Network Breakdown</p>
                            {[
                                { label: "Mainnet", color: "bg-green-500", count: networkCounts?.mainnet ?? 0 },
                                { label: "Testnet", color: "bg-blue-500", count: networkCounts?.testnet ?? 0 },
                                { label: "Futurenet", color: "bg-purple-500", count: networkCounts?.futurenet ?? 0 },
                                { label: "Other", color: "bg-gray-500", count: networkCounts?.other ?? 0 },
                            ].map(({ label, color, count }) => count > 0 ? (
                                <div key={label} className="flex items-center gap-2">
                                    <div className={`w-2 h-2 rounded-full ${color} shrink-0`} />
                                    <span className="text-[11px] text-gray-400 flex-1">{label}</span>
                                    <span className="text-[11px] font-mono text-gray-300">{count.toLocaleString()}</span>
                                    <div className="w-16 bg-gray-800 rounded-full h-1 overflow-hidden">
                                        <div
                                            className={`h-1 rounded-full ${color}`}
                                            style={{ width: `${Math.round((count / Math.max(totalNodes, 1)) * 100)}%` }}
                                        />
                                    </div>
                                </div>
                            ) : null)}
                            <div className="border-t border-gray-800 pt-2 mt-1">
                                <div className="flex justify-between text-[10px]">
                                    <span className="text-gray-500">Avg edges/node</span>
                                    <span className="text-gray-400 font-mono">
                                        {totalNodes > 0 ? (totalEdges / totalNodes).toFixed(1) : "0.0"}
                                    </span>
                                </div>
                                {networkFilter !== "all" && (
                                    <div className="flex justify-between text-[10px] mt-1">
                                        <span className="text-gray-500">Active filter</span>
                                        <span className="text-blue-400 font-mono capitalize">{networkFilter}</span>
                                    </div>
                                )}
                            </div>
                        </div>
                    )}
                </div>
            </div>

            {/* Bottom-left: Keyboard shortcut hints */}
            <div className="absolute bottom-4 left-4 z-30" role="complementary" aria-label="Keyboard shortcuts reference">
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl p-3 shadow-2xl">
                    <div className="flex items-center gap-1.5 mb-2">
                        <Keyboard className="w-3 h-3 text-gray-400" />
                        <p className="text-[10px] text-gray-400 font-medium uppercase tracking-wider">Shortcuts</p>
                    </div>
                    <div className="space-y-1 text-[10px] text-gray-500">
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Zoom in/out</span>
                            <span className="font-mono bg-gray-800 px-1 rounded">+ / -</span>
                        </div>
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Pan</span>
                            <span className="font-mono bg-gray-800 px-1 rounded">↑ ↓ ← →</span>
                        </div>
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Reset view</span>
                            <span className="font-mono bg-gray-800 px-1 rounded">R</span>
                        </div>
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Deselect</span>
                            <span className="font-mono bg-gray-800 px-1 rounded">Esc</span>
                        </div>
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Drag</span>
                            <span className="text-gray-500">move nodes</span>
                        </div>
                        <div className="flex justify-between gap-4">
                            <span className="text-gray-400">Double-click</span>
                            <span className="text-gray-500">pin / unpin</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Bottom-right: Zoom + Export controls */}
            <div className="absolute bottom-4 right-4 z-30 flex flex-col gap-2" role="group" aria-label="Graph view controls">
                {/* Pan d-pad */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden p-1" role="group" aria-label="Pan controls">
                    <div className="grid grid-cols-3 gap-0.5 w-[90px]">
                        <div />
                        <button
                            id="graph-pan-up"
                            onClick={onPanUp}
                            className="flex items-center justify-center h-7 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                            aria-label="Pan up"
                            title="Pan up (↑)"
                        >
                            <ChevronUp className="w-3.5 h-3.5" />
                        </button>
                        <div />
                        <button
                            id="graph-pan-left"
                            onClick={onPanLeft}
                            className="flex items-center justify-center h-7 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                            aria-label="Pan left"
                            title="Pan left (←)"
                        >
                            <ChevronLeft className="w-3.5 h-3.5" />
                        </button>
                        <button
                            id="graph-reset-view"
                            onClick={onResetZoom}
                            className="flex items-center justify-center h-7 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                            aria-label="Reset view to fit all nodes"
                            title="Reset view (R)"
                        >
                            <Maximize2 className="w-3 h-3" />
                        </button>
                        <button
                            id="graph-pan-right"
                            onClick={onPanRight}
                            className="flex items-center justify-center h-7 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                            aria-label="Pan right"
                            title="Pan right (→)"
                        >
                            <ChevronRight className="w-3.5 h-3.5" />
                        </button>
                        <div />
                        <button
                            id="graph-pan-down"
                            onClick={onPanDown}
                            className="flex items-center justify-center h-7 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors rounded focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                            aria-label="Pan down"
                            title="Pan down (↓)"
                        >
                            <ChevronDown className="w-3.5 h-3.5" />
                        </button>
                        <div />
                    </div>
                </div>

                {/* Zoom controls */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden" role="group" aria-label="Zoom controls">
                    <button
                        id="graph-zoom-in"
                        onClick={onZoomIn}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                        aria-label="Zoom in (+ key)"
                        title="Zoom in (+)"
                    >
                        <ZoomIn className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" role="separator" />
                    <button
                        id="graph-zoom-out"
                        onClick={onZoomOut}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                        aria-label="Zoom out (- key)"
                        title="Zoom out (-)"
                    >
                        <ZoomOut className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" role="separator" />
                    <button
                        id="graph-reset-zoom"
                        onClick={onResetZoom}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                        aria-label="Reset zoom (R key)"
                        title="Reset zoom (R)"
                    >
                        <Maximize2 className="w-4 h-4" />
                    </button>
                </div>

                {/* Export controls */}
                <div className="bg-gray-900/90 backdrop-blur-xl border border-gray-700/50 rounded-xl shadow-2xl overflow-hidden" role="group" aria-label="Export controls">
                    <button
                        id="graph-export-svg"
                        onClick={onExportSVG}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                        aria-label="Export graph as SVG file"
                        title="Export as SVG"
                    >
                        <Download className="w-4 h-4" />
                    </button>
                    <div className="border-t border-gray-800" role="separator" />
                    <button
                        id="graph-export-png"
                        onClick={onExportPNG}
                        className="flex items-center justify-center w-10 h-10 text-gray-400 hover:text-white hover:bg-gray-800 transition-colors focus-visible:ring-1 focus-visible:ring-blue-500 focus:outline-none"
                        aria-label="Export graph as PNG image"
                        title="Export as PNG"
                    >
                        <FileImage className="w-4 h-4" />
                    </button>
                </div>
            </div>
        </>
    );
}
