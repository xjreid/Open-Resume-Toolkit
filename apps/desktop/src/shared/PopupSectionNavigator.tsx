import type { ResumeDocument } from "@ort/contracts/resume";
import { ResumeSectionNavigator } from "./ResumeSectionNavigator";

export function PopupSectionNavigator({
  document,
  disabled,
  onChange,
}: {
  document: ResumeDocument;
  disabled: boolean;
  onChange: (document: ResumeDocument) => void;
}) {
  return (
    <ResumeSectionNavigator
      document={document}
      disabled={disabled}
      className="application-popup__navigator"
      canUndoRemoval={false}
      onChange={(update) => onChange(update(document))}
    />
  );
}
