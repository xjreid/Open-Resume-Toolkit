import {
  DOCUMENT_STYLE_TEMPLATES,
  type DocumentStyle,
} from "@ort/contracts/export";

export const SELECTABLE_DOCUMENT_STYLES = [
  "technical",
  "professional",
  "modern",
] as const satisfies readonly DocumentStyle[];

export const DOCUMENT_STYLE_LABELS: Record<DocumentStyle, string> = {
  plain: "Plain layout v1",
  technical: "Technical / Engineering",
  professional: "Professional / Business",
  modern: "Modern / Marketing & Sales",
};

export const DOCUMENT_STYLE_DESCRIPTIONS: Record<DocumentStyle, string> = {
  plain: "The original plain layout, retained for historical previews.",
  technical: "Compact single-column layout with restrained headings.",
  professional: "Traditional headings and more space between details.",
  modern: "Larger name and muted teal headings in a single-column layout.",
};

export function pdfStyleLabel(templateId: string): string {
  const style = (Object.keys(DOCUMENT_STYLE_TEMPLATES) as DocumentStyle[]).find(
    (style) => DOCUMENT_STYLE_TEMPLATES[style].pdf === templateId,
  );
  return style ? DOCUMENT_STYLE_LABELS[style] : "Unrecognized historical style";
}
