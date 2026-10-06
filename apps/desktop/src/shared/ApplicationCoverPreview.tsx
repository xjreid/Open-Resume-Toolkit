import { invokeDesktop as invoke } from "./desktop-client";
import type * as Wire from "@ort/contracts/wire";
import { useEffect, useState } from "react";
import type { PdfPreview } from "@ort/contracts/pdf";
import { PdfCanvas } from "./PdfPreview";

export function ApplicationCoverPreview({
  revision,
  text,
}: {
  revision?: number;
  text: string;
}) {
  const [preview, setPreview] = useState<PdfPreview | null>(null);
  const [status, setStatus] = useState("Loading preview…");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    setFailed(false);
    if (revision === undefined) {
      setStatus("Save the cover letter before opening its PDF preview.");
      return;
    }
    void invoke("preview_application_pdf", {
      expectedRevision: revision,
      kind: "cover_letter",
    })
      .then((result) => {
        if (!active) return;
        if (!result.ok || !result.value || result.value.revision !== revision) {
          throw new Error("Preview unavailable");
        }
        setPreview(result.value);
      })
      .catch(() => {
        if (active) {
          setFailed(true);
          setStatus(
            "The PDF preview could not be loaded. Close it and try View again after the files are ready.",
          );
        }
      });
    return () => {
      active = false;
    };
  }, [revision]);
  return (
    <section aria-label="Cover letter PDF preview">
      {status && <p role={failed ? "alert" : "status"}>{status}</p>}
      {preview && !failed && (
        <PdfCanvas
          preview={preview}
          minimal
          onReady={() => setStatus("")}
          onError={() => {
            setFailed(true);
            setStatus(
              "The PDF could not be displayed. Close it and try View again.",
            );
          }}
        />
      )}
      <div className="visually-hidden">
        <h2>Cover letter text</h2>
        <p>{text}</p>
      </div>
    </section>
  );
}
