import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { CircuitDiagramViewer } from './circuit-diagram-viewer';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    CircuitDiagram: CircuitDiagramViewer,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
