'use client';

import { circuitDiagrams, type CircuitDiagramModel } from '@/lib/circuit-diagrams-data';
import type { CircuitDiagram, CircuitDiagramNode } from '@/lib/circuit-diagram';
import { useEffect, useMemo, useRef, useState } from 'react';

type NodeRect = {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
};

type CircuitDiagramViewerProps = {
  model: CircuitDiagramModel;
};

const NODE_WIDTH = 150;
const NODE_HEIGHT = 74;

export function CircuitDiagramViewer({ model }: CircuitDiagramViewerProps) {
  const diagram = circuitDiagrams[model];
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [selectedId, setSelectedId] = useState(diagram.nodes[0]?.id);
  const rects = useMemo(() => nodeRects(diagram), [diagram]);
  const selected = diagram.nodes.find((node) => node.id === selectedId) ?? diagram.nodes[0];

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const context = canvas.getContext('2d');
    if (!context) return;

    drawDiagram(context, diagram, rects, selected?.id);
  }, [diagram, rects, selected?.id]);

  return (
    <section className="circuit-diagram" aria-label={`${diagram.title} visualization`}>
      <div className="circuit-diagram__header">
        <div>
          <div className="circuit-diagram__eyebrow">Circuit graph</div>
          <h3>{diagram.title}</h3>
        </div>
        <div className="circuit-diagram__meta">
          <span>{diagram.status}</span>
          <span>{diagram.sourceOfTruth}</span>
        </div>
      </div>

      <div className="circuit-diagram__body">
        <div className="circuit-diagram__canvas-wrap">
          <canvas
            ref={canvasRef}
            width={diagram.renderer?.preferredWidth ?? 1400}
            height={diagram.renderer?.preferredHeight ?? 720}
            className="circuit-diagram__canvas"
            onClick={(event) => {
              const canvas = canvasRef.current;
              if (!canvas) return;
              const bounds = canvas.getBoundingClientRect();
              const scaleX = canvas.width / bounds.width;
              const scaleY = canvas.height / bounds.height;
              const x = (event.clientX - bounds.left) * scaleX;
              const y = (event.clientY - bounds.top) * scaleY;
              const hit = [...rects]
                .reverse()
                .find((rect) => x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height);
              if (hit) setSelectedId(hit.id);
            }}
          />
        </div>

        <NodeInspector node={selected} />
      </div>
    </section>
  );
}

function NodeInspector({ node }: { node?: CircuitDiagramNode }) {
  if (!node) return null;

  return (
    <aside className="circuit-diagram__inspector">
      <div className="circuit-diagram__eyebrow">{node.kind}</div>
      <h4>{node.label}</h4>

      {node.hardware?.role ? (
        <div className="circuit-diagram__detail">
          <strong>Hardware</strong>
          <p>{node.hardware.role}</p>
          {node.hardware.confidence ? <span>Confidence: {node.hardware.confidence}</span> : null}
        </div>
      ) : null}

      {node.emulation ? (
        <div className="circuit-diagram__detail">
          <strong>Emulation</strong>
          {node.emulation.library?.length ? <p>{node.emulation.library.join(', ')}</p> : null}
          {node.emulation.algorithm ? <p>{node.emulation.algorithm}</p> : null}
        </div>
      ) : null}

      <div className="circuit-diagram__detail">
        <strong>SPICE export</strong>
        {node.spice ? (
          <p>
            {node.spice.primitive}
            {node.spice.name} {node.spice.nodes.join(' ')}
            {node.spice.value ? ` ${node.spice.value}` : ''}
            {node.spice.model ? ` ${node.spice.model}` : ''}
          </p>
        ) : (
          <p>Documentation-only node.</p>
        )}
      </div>
    </aside>
  );
}

function nodeRects(diagram: CircuitDiagram): NodeRect[] {
  return diagram.nodes.map((node) => ({
    id: node.id,
    x: node.layout?.x ?? 0,
    y: node.layout?.y ?? 0,
    width: node.layout?.width ?? NODE_WIDTH,
    height: node.layout?.height ?? NODE_HEIGHT,
  }));
}

function drawDiagram(
  context: CanvasRenderingContext2D,
  diagram: CircuitDiagram,
  rects: NodeRect[],
  selectedId?: string,
) {
  const width = diagram.renderer?.preferredWidth ?? 1400;
  const height = diagram.renderer?.preferredHeight ?? 720;
  context.clearRect(0, 0, width, height);
  context.fillStyle = '#fbfaf7';
  context.fillRect(0, 0, width, height);

  drawGrid(context, width, height);
  drawGroups(context, diagram, rects);
  drawEdges(context, diagram, rects);
  for (const node of diagram.nodes) {
    const rect = rects.find((item) => item.id === node.id);
    if (rect) drawNode(context, node, rect, node.id === selectedId);
  }
}

function drawGrid(context: CanvasRenderingContext2D, width: number, height: number) {
  context.save();
  context.strokeStyle = 'rgba(31, 41, 55, 0.06)';
  context.lineWidth = 1;
  for (let x = 0; x <= width; x += 40) {
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, height);
    context.stroke();
  }
  for (let y = 0; y <= height; y += 40) {
    context.beginPath();
    context.moveTo(0, y);
    context.lineTo(width, y);
    context.stroke();
  }
  context.restore();
}

function drawGroups(context: CanvasRenderingContext2D, diagram: CircuitDiagram, rects: NodeRect[]) {
  context.save();
  for (const group of diagram.groups ?? []) {
    const groupRects = group.nodes
      .map((id) => rects.find((rect) => rect.id === id))
      .filter((rect): rect is NodeRect => Boolean(rect));
    if (groupRects.length === 0) continue;

    const left = Math.min(...groupRects.map((rect) => rect.x)) - 28;
    const top = Math.min(...groupRects.map((rect) => rect.y)) - 44;
    const right = Math.max(...groupRects.map((rect) => rect.x + rect.width)) + 28;
    const bottom = Math.max(...groupRects.map((rect) => rect.y + rect.height)) + 28;

    roundedRect(context, left, top, right - left, bottom - top, 14);
    context.fillStyle = 'rgba(232, 225, 214, 0.42)';
    context.fill();
    context.strokeStyle = 'rgba(120, 113, 108, 0.22)';
    context.stroke();
    context.fillStyle = '#57534e';
    context.font = '600 18px ui-sans-serif, system-ui';
    context.fillText(group.label, left + 18, top + 28);
  }
  context.restore();
}

function drawEdges(context: CanvasRenderingContext2D, diagram: CircuitDiagram, rects: NodeRect[]) {
  context.save();
  context.strokeStyle = '#64748b';
  context.fillStyle = '#64748b';
  context.lineWidth = 2.2;

  for (const edge of diagram.edges) {
    const from = rects.find((rect) => rect.id === edge.from.split('.')[0]);
    const to = rects.find((rect) => rect.id === edge.to.split('.')[0]);
    if (!from || !to) continue;

    const startX = from.x + from.width;
    const startY = from.y + from.height / 2;
    const endX = to.x;
    const endY = to.y + to.height / 2;
    const midX = startX + Math.max(40, (endX - startX) / 2);

    context.beginPath();
    context.moveTo(startX, startY);
    context.bezierCurveTo(midX, startY, midX, endY, endX, endY);
    context.stroke();

    context.beginPath();
    context.moveTo(endX, endY);
    context.lineTo(endX - 9, endY - 5);
    context.lineTo(endX - 9, endY + 5);
    context.closePath();
    context.fill();
  }
  context.restore();
}

function drawNode(context: CanvasRenderingContext2D, node: CircuitDiagramNode, rect: NodeRect, selected: boolean) {
  context.save();
  roundedRect(context, rect.x, rect.y, rect.width, rect.height, 12);
  context.fillStyle = node.spice ? '#ecfdf5' : '#ffffff';
  context.fill();
  context.lineWidth = selected ? 4 : 1.5;
  context.strokeStyle = selected ? '#0f766e' : '#cbd5e1';
  context.stroke();

  context.fillStyle = colorForKind(node.kind);
  roundedRect(context, rect.x + 12, rect.y + 12, 16, 16, 4);
  context.fill();

  context.fillStyle = '#111827';
  context.font = '700 17px ui-sans-serif, system-ui';
  wrapText(context, node.label, rect.x + 36, rect.y + 26, rect.width - 46, 19, 2);

  context.fillStyle = '#64748b';
  context.font = '500 13px ui-sans-serif, system-ui';
  context.fillText(node.spice ? 'SPICE partial' : node.kind, rect.x + 14, rect.y + rect.height - 14);
  context.restore();
}

function colorForKind(kind: string) {
  if (kind.includes('triode') || kind.includes('bjt') || kind.includes('gain')) return '#ef4444';
  if (kind.includes('tone') || kind.includes('filter') || kind.includes('highpass')) return '#2563eb';
  if (kind.includes('power') || kind.includes('transformer') || kind.includes('supply')) return '#7c3aed';
  if (kind.includes('diode')) return '#f59e0b';
  return '#0f766e';
}

function roundedRect(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
  context.beginPath();
  context.moveTo(x + radius, y);
  context.lineTo(x + width - radius, y);
  context.quadraticCurveTo(x + width, y, x + width, y + radius);
  context.lineTo(x + width, y + height - radius);
  context.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  context.lineTo(x + radius, y + height);
  context.quadraticCurveTo(x, y + height, x, y + height - radius);
  context.lineTo(x, y + radius);
  context.quadraticCurveTo(x, y, x + radius, y);
  context.closePath();
}

function wrapText(
  context: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxWidth: number,
  lineHeight: number,
  maxLines: number,
) {
  const words = text.split(' ');
  let line = '';
  let lineCount = 0;

  for (const word of words) {
    const next = line ? `${line} ${word}` : word;
    if (context.measureText(next).width > maxWidth && line) {
      context.fillText(line, x, y + lineCount * lineHeight);
      line = word;
      lineCount += 1;
      if (lineCount >= maxLines) return;
    } else {
      line = next;
    }
  }

  if (line && lineCount < maxLines) context.fillText(line, x, y + lineCount * lineHeight);
}
