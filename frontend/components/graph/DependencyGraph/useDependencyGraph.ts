'use client';

import { useState, useRef, useEffect, useCallback, useImperativeHandle } from 'react';
import * as d3 from 'd3';
import { useTheme } from '@/hooks/useTheme';
import { GraphNode, GraphEdge } from '@/types';

export interface SimNode extends d3.SimulationNodeDatum {
    id: string;
    data: GraphNode;
    radius: number;
}

export interface SimLink extends d3.SimulationLinkDatum<SimNode> {
    data: GraphEdge;
}

export interface TooltipState {
    x: number;
    y: number;
    node: GraphNode;
    dependents: number;
}

export interface EdgeTooltipState {
    x: number;
    y: number;
    edge: GraphEdge;
    sourceName: string;
    targetName: string;
}

export function useDependencyGraph(
    nodes: GraphNode[],
    edges: GraphEdge[],
    dependentCounts: Map<string, number>,
    selectedNode: GraphNode | null,
    onNodeClick?: (node: GraphNode | null) => void
) {
    const { resolvedTheme } = useTheme();
    const svgRef = useRef<SVGSVGElement>(null);
    const zoomRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
    const gRef = useRef<d3.Selection<SVGGElement, unknown, null, undefined> | null>(null);
    
    const [tooltip, setTooltip] = useState<TooltipState | null>(null);
    const [edgeTooltip, setEdgeTooltip] = useState<EdgeTooltipState | null>(null);
    const [highlightedChain, setHighlightedChain] = useState<{ nodes: Set<string>; edges: Set<string> } | null>(null);
    const pinnedRef = useRef<Set<string>>(new Set());

    const computeChain = useCallback((startNodeId: string) => {
        const chainNodes = new Set<string>([startNodeId]);
        const chainEdges = new Set<string>();
        
        const outEdges = new Map<string, string[]>();
        const inEdges = new Map<string, string[]>();
        
        edges.forEach(e => {
            const src = typeof e.source === 'string' ? e.source : (e.source as any).id;
            const tgt = typeof e.target === 'string' ? e.target : (e.target as any).id;
            
            if (!outEdges.has(src)) outEdges.set(src, []);
            if (!inEdges.has(tgt)) inEdges.set(tgt, []);
            outEdges.get(src)!.push(tgt);
            inEdges.get(tgt)!.push(src);
        });

        let queue: string[] = [startNodeId];
        while (queue.length > 0) {
            const current = queue.shift()!;
            (outEdges.get(current) || []).forEach(target => {
                if (!chainNodes.has(target)) {
                    chainNodes.add(target);
                    chainEdges.add(`${current}-${target}`);
                    queue.push(target);
                } else if (!chainEdges.add(`${current}-${target}`)) {
                    chainEdges.add(`${current}-${target}`);
                }
            });
        }
        
        queue = [startNodeId];
        while (queue.length > 0) {
            const current = queue.shift()!;
            (inEdges.get(current) || []).forEach(source => {
                if (!chainNodes.has(source)) {
                    chainNodes.add(source);
                    chainEdges.add(`${source}-${current}`);
                    queue.push(source);
                } else if (!chainEdges.add(`${source}-${current}`)) {
                    chainEdges.add(`${source}-${current}`);
                }
            });
        }
        
        return { nodes: chainNodes, edges: chainEdges };
    }, [edges]);

    useEffect(() => {
        if (selectedNode) {
            setHighlightedChain(computeChain(selectedNode.id));
        } else {
            setHighlightedChain(null);
        }
    }, [selectedNode, computeChain]);

    return {
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
    };
}
