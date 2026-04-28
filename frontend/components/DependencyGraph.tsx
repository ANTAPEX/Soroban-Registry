'use client';

import React, { forwardRef, useImperativeHandle, useRef } from 'react';
import * as d3 from 'd3';
import { GraphNode, GraphEdge } from '@/types';
import { useDependencyGraph } from './graph/DependencyGraph/useDependencyGraph';
import { GraphSVG } from './graph/DependencyGraph/GraphSVG';

export interface DependencyGraphHandle {
    zoomIn: () => void;
    zoomOut: () => void;
    resetZoom: () => void;
    focusOnNode: (id: string) => void;
}

interface DependencyGraphProps {
    nodes: GraphNode[];
    edges: GraphEdge[];
    searchQuery?: string;
    dependentCounts?: Map<string, number>;
    onNodeClick?: (node: GraphNode | null) => void;
    selectedNode?: GraphNode | null;
}

const DependencyGraph = forwardRef<DependencyGraphHandle, DependencyGraphProps>(
    function DependencyGraph({ nodes, edges, dependentCounts = new Map(), onNodeClick, selectedNode, searchQuery = "" }, ref) {
        
        const {
            svgRef,
            zoomRef,
            gRef,
            tooltip,
            setTooltip,
            edgeTooltip,
            setEdgeTooltip,
            highlightedChain,
            pinnedRef,
            resolvedTheme
        } = useDependencyGraph(nodes, edges, dependentCounts, selectedNode, onNodeClick);

        const containerRef = useRef<HTMLDivElement>(null);
        const isLargeGraph = nodes.length > 200;
        const isVeryLargeGraph = nodes.length > 500;

        useImperativeHandle(ref, () => ({
            zoomIn: () => {
                if (svgRef.current && zoomRef.current) {
                    d3.select(svgRef.current).transition().call(zoomRef.current.scaleBy, 1.3);
                }
            },
            zoomOut: () => {
                if (svgRef.current && zoomRef.current) {
                    d3.select(svgRef.current).transition().call(zoomRef.current.scaleBy, 1 / 1.3);
                }
            },
            resetZoom: () => {
                if (svgRef.current && zoomRef.current) {
                    const rect = svgRef.current.getBoundingClientRect();
                    d3.select(svgRef.current).transition().call(
                        zoomRef.current.transform,
                        d3.zoomIdentity.translate(rect.width / 2, rect.height / 2).scale(1)
                    );
                }
            },
            focusOnNode: (id: string) => {
                // Focus logic
            }
        }));

        if (nodes.length === 0) {
            return (
                <div className="w-full h-full flex items-center justify-center bg-muted/30">
                    <p className="text-muted-foreground text-sm">No nodes to display.</p>
                </div>
            );
        }

        return (
            <div className="w-full h-full relative" ref={containerRef}>
                <GraphSVG 
                    nodes={nodes}
                    edges={edges}
                    dependentCounts={dependentCounts}
                    resolvedTheme={resolvedTheme}
                    svgRef={svgRef}
                    gRef={gRef}
                    zoomRef={zoomRef}
                    setTooltip={setTooltip}
                    setEdgeTooltip={setEdgeTooltip}
                    selectedNode={selectedNode}
                    onNodeClick={onNodeClick}
                    pinnedRef={pinnedRef}
                    isLargeGraph={isLargeGraph}
                    isVeryLargeGraph={isVeryLargeGraph}
                />

                {isVeryLargeGraph && (
                    <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 pointer-events-none">
                        <div className="bg-amber-900/80 backdrop-blur border border-amber-700/50 rounded-lg px-3 py-1.5 text-xs text-amber-200">
                            Large graph — labels hidden for performance
                        </div>
                    </div>
                )}

                {tooltip && (
                    <div 
                        className="pointer-events-none absolute z-40 bg-background/95 backdrop-blur-xl border border-border rounded-xl px-3 py-2.5 shadow-2xl text-xs"
                        style={{ left: tooltip.x + 14, top: tooltip.y - 10 }}
                    >
                        <p className="font-semibold text-foreground mb-1">{tooltip.node.name}</p>
                        <div className="space-y-0.5 text-muted-foreground">
                            <div className="flex justify-between gap-4">
                                <span>Network</span>
                                <span className="text-foreground capitalize">{tooltip.node.network}</span>
                            </div>
                            <div className="flex justify-between gap-4">
                                <span>Dependents</span>
                                <span className="text-foreground">{tooltip.dependents}</span>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        );
    }
);

export default DependencyGraph;