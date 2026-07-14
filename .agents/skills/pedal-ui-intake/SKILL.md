---
name: pedal-ui-intake
description: Run a structured, interactive intake before designing or generating a Greybound pedal UI. Use explicitly when the user wants a new pedal faceplate, a pedal control layout, or an image-generation prompt for a pedal.
---

# Pedal UI intake

Create one model-specific layout only after the user has approved the proposed
placement. Preserve the shared mechanical template; do not edit it unless the
user explicitly asks to change the standard.

## Start

Read these files before asking questions:

- `AGENTS.md`
- `docs/templates/pedal-standard-1200x2172.svg`
- `docs/ai-asset-generation.md`
- `docs/render-specs.md`

Reuse answers already given in the current task. Ask only for information that
is still missing. Ask one topic at a time; do not send a long unstructured
questionnaire.

## Intake order

1. **Identity:** model id, displayed model name, intended pedal family, and
   whether its labels are baked into the faceplate or drawn by the UI.
2. **Controls:** every control's label, widget type, audio role, and whether it
   is dynamic. Confirm the bypass LED and footswitch behavior.
3. **Layout:** the desired arrangement, component sizes, and any reference
   pedal or sketch. Translate vague placement into proposed coordinates; do
   not silently invent final coordinates.
4. **Typography:** label text, label size, casing, colour, and whether labels
   are printed into the final faceplate. For each control, specify the label
   baseline relative to its anchor and ensure label zones do not overlap.
5. **Art direction:** enclosure material, finish, palette, decorative motifs,
   brand/model typography, and reference images or existing Greybound pedals.
6. **Output:** confirm whether the user wants only the layout, a generation
   prompt, or an image generation too.

## Layout proposal

Before creating files or images, present a compact placement table with:

- canvas dimensions;
- body, side-I/O rectangle, and footswitch-cross references from the shared
  template;
- each control's anchor `(x, y)`, visual diameter, and label baseline;
- every printed label and its bounding zone;
- any deliberate exception to the shared mechanical standard.

Ask for explicit approval of this table. Revise it until approved.

## Approved output

Create `docs/templates/layouts/<model-id>.svg` from the shared template after
approval. Keep visual guides in clearly marked SVG groups and state that they
must not appear in the final PNG. The model-specific overlay must contain the
exact control anchors, control-size circles, label baselines, label bounding
zones, and printed label text.

Then write a generation prompt that includes the approved static artwork and
explicitly excludes all guide lines, crosses, circles, boxes, placeholder text,
and crop marks. Generate an image only when the user explicitly requests it.

Do not alter Rust render specs or production assets unless the user asks to
integrate the approved layout.

## Completion

Report the approved layout path and the prompt. State whether the faceplate
uses baked or UI-drawn labels and list any remaining choices.
