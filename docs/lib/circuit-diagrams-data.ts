import type { CircuitDiagram } from './circuit-diagram';

import nox30Raw from '../../knowledge/models/amps/diagrams/nox30.diagram.json5';
import dumblerRaw from '../../knowledge/models/amps/diagrams/dumbler.diagram.json5';
import sheriff800Raw from '../../knowledge/models/amps/diagrams/sheriff800.diagram.json5';
import muffinRaw from '../../knowledge/models/pedals/fuzz/diagrams/muffin.diagram.json5';
import minotaurRaw from '../../knowledge/models/pedals/overdrive/diagrams/minotaur.diagram.json5';
import monarchRaw from '../../knowledge/models/pedals/overdrive/diagrams/monarch.diagram.json5';
import godessOneRaw from '../../knowledge/models/pedals/distortion/diagrams/godess-one.diagram.json5';
import dartfordRaw from '../../knowledge/models/pedals/modulation/diagrams/dartford.diagram.json5';

export const circuitDiagrams = {
  nox30: parseDiagram(nox30Raw),
  muffin: parseDiagram(muffinRaw),
  minotaur: parseDiagram(minotaurRaw),
  monarch: parseDiagram(monarchRaw),
  'godess-one': parseDiagram(godessOneRaw),
  dartford: parseDiagram(dartfordRaw),
  dumbler: parseDiagram(dumblerRaw),
  sheriff800: parseDiagram(sheriff800Raw),
} satisfies Record<string, CircuitDiagram>;

export type CircuitDiagramModel = keyof typeof circuitDiagrams;

function parseDiagram(source: string): CircuitDiagram {
  return JSON.parse(toJson(source)) as CircuitDiagram;
}

function toJson(source: string): string {
  return quoteObjectKeys(removeTrailingCommas(convertSingleQuotedStrings(stripComments(source))));
}

function stripComments(source: string): string {
  let output = '';
  let quote: '"' | "'" | null = null;
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];
    const next = source[index + 1];

    if (quote) {
      output += char;
      if (escaped) {
        escaped = false;
      } else if (char === '\\') {
        escaped = true;
      } else if (char === quote) {
        quote = null;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      output += char;
      continue;
    }

    if (char === '/' && next === '/') {
      while (index < source.length && source[index] !== '\n') index += 1;
      output += '\n';
      continue;
    }

    if (char === '/' && next === '*') {
      index += 2;
      while (index < source.length && !(source[index] === '*' && source[index + 1] === '/')) {
        index += 1;
      }
      index += 1;
      continue;
    }

    output += char;
  }

  return output;
}

function convertSingleQuotedStrings(source: string): string {
  let output = '';
  let quote: '"' | "'" | null = null;
  let buffer = '';
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];

    if (!quote) {
      if (char === "'") {
        quote = "'";
        buffer = '';
      } else if (char === '"') {
        quote = '"';
        output += char;
      } else {
        output += char;
      }
      continue;
    }

    if (quote === '"') {
      output += char;
      if (escaped) {
        escaped = false;
      } else if (char === '\\') {
        escaped = true;
      } else if (char === '"') {
        quote = null;
      }
      continue;
    }

    if (escaped) {
      buffer += char;
      escaped = false;
    } else if (char === '\\') {
      escaped = true;
    } else if (char === "'") {
      output += JSON.stringify(buffer);
      quote = null;
    } else {
      buffer += char;
    }
  }

  return output;
}

function removeTrailingCommas(source: string): string {
  return source.replace(/,\s*([}\]])/g, '$1');
}

function quoteObjectKeys(source: string): string {
  return source.replace(/([{,]\s*)([A-Za-z_$][\w$-]*)(\s*:)/g, '$1"$2"$3');
}
