import { useEffect, useRef } from "react";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { ResumeDocument } from "@ort/contracts/resume";

export function PdfCanvas({ base64 }: { base64: string }) {
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let cancelled = false;
    let worker: Worker | null = null;
    let pdfWorker: import("pdfjs-dist").PDFWorker | null = null;
    let loading: import("pdfjs-dist").PDFDocumentLoadingTask | null = null;
    const host = container.current;
    if (!host) return;
    host.replaceChildren();
    void (async () => {
      try {
        const engine = await import("pdfjs-dist");
        if (cancelled) return;
        worker = new Worker(workerUrl, { type: "module" });
        pdfWorker = engine.PDFWorker.create({ port: worker });
        const binary = atob(base64);
        const bytes = Uint8Array.from(binary, (character) =>
          character.charCodeAt(0),
        );
        loading = engine.getDocument({
          worker: pdfWorker,
          data: bytes,
          disableFontFace: true,
          useSystemFonts: false,
          useWorkerFetch: false,
          useWasm: false,
          stopAtErrors: true,
          enableXfa: false,
          isOffscreenCanvasSupported: false,
          isImageDecoderSupported: false,
          maxImageSize: 2_000_000,
          canvasMaxAreaInBytes: 8_000_000,
        });
        const pdf = await loading.promise;
        for (let index = 1; index <= pdf.numPages && !cancelled; index += 1) {
          const page = await pdf.getPage(index);
          const viewport = page.getViewport({ scale: 1.2 });
          const canvas = document.createElement("canvas");
          canvas.width = Math.ceil(viewport.width);
          canvas.height = Math.ceil(viewport.height);
          canvas.setAttribute("aria-label", `PDF page ${index}`);
          host.append(canvas);
          const context = canvas.getContext("2d");
          if (context)
            await page.render({ canvas, canvasContext: context, viewport })
              .promise;
        }
      } catch {
        if (!cancelled)
          host.textContent = "Preview unavailable. Download remains available.";
      }
    })();
    return () => {
      cancelled = true;
      void loading?.destroy();
      pdfWorker?.destroy();
      worker?.terminate();
      host.replaceChildren();
    };
  }, [base64]);
  return (
    <div
      className="application-pdf-pages"
      ref={container}
      aria-label="PDF preview"
    />
  );
}

export function ResumeFields({
  document,
  onChange,
}: {
  document: ResumeDocument;
  onChange: (value: ResumeDocument) => void;
}) {
  function edit(
    sectionIndex: number,
    entryIndex: number,
    field: "heading" | "subheading" | "location" | "dateRange",
    value: string,
  ) {
    onChange({
      ...document,
      sections: document.sections.map((section, si) =>
        si === sectionIndex
          ? {
              ...section,
              entries: section.entries.map((entry, ei) =>
                ei === entryIndex
                  ? {
                      ...entry,
                      [field]: value,
                      ...(field === "dateRange" && value.trim()
                        ? { dates: [] }
                        : {}),
                    }
                  : entry,
              ),
            }
          : section,
      ),
    });
  }
  return (
    <div className="application-editor">
      <h3>Structured resume content</h3>
      <label>
        Title
        <input
          value={document.title}
          onChange={(event) =>
            onChange({ ...document, title: event.target.value })
          }
        />
      </label>
      <h4>Contact</h4>
      {(
        [
          ["fullName", "Full name"],
          ["email", "Email"],
          ["phone", "Phone"],
          ["location", "Location"],
        ] as const
      ).map(([field, label]) => (
        <label key={field}>
          {label}
          <input
            value={document.contact[field]}
            onChange={(event) =>
              onChange({
                ...document,
                contact: { ...document.contact, [field]: event.target.value },
              })
            }
          />
        </label>
      ))}
      {document.contact.links.map((link, index) => (
        <div key={link.id ?? index} className="application-entry">
          <label>
            Contact link {index + 1} label
            <input
              value={link.label}
              onChange={(event) =>
                onChange({
                  ...document,
                  contact: {
                    ...document.contact,
                    links: document.contact.links.map((item, position) =>
                      position === index
                        ? { ...item, label: event.target.value }
                        : item,
                    ),
                  },
                })
              }
            />
          </label>
          <label>
            Contact link {index + 1} URL
            <input
              value={link.url}
              onChange={(event) =>
                onChange({
                  ...document,
                  contact: {
                    ...document.contact,
                    links: document.contact.links.map((item, position) =>
                      position === index
                        ? { ...item, url: event.target.value }
                        : item,
                    ),
                  },
                })
              }
            />
          </label>
        </div>
      ))}
      {document.sections.map((section, si) => (
        <section key={section.id}>
          <label>
            Section heading
            <input
              value={section.heading}
              onChange={(event) =>
                onChange({
                  ...document,
                  sections: document.sections.map((item, index) =>
                    index === si
                      ? { ...item, heading: event.target.value }
                      : item,
                  ),
                })
              }
            />
          </label>
          {section.entries.map((entry, ei) => (
            <div key={entry.id} className="application-entry">
              {(
                ["heading", "subheading", "location", "dateRange"] as const
              ).map((field) => (
                <label key={field}>
                  {field === "dateRange" && entry.dates?.length
                    ? "Date range (replaces structured dates)"
                    : field}
                  <input
                    value={entry[field]}
                    onChange={(event) =>
                      edit(si, ei, field, event.target.value)
                    }
                  />
                </label>
              ))}
              {entry.fields.map((field, fi) => (
                <label key={field.id}>
                  {field.label || "Field"}
                  <textarea
                    value={field.value}
                    onChange={(event) =>
                      onChange({
                        ...document,
                        sections: document.sections.map((part, partIndex) =>
                          partIndex === si
                            ? {
                                ...part,
                                entries: part.entries.map((item, itemIndex) =>
                                  itemIndex === ei
                                    ? {
                                        ...item,
                                        fields: item.fields.map(
                                          (value, valueIndex) =>
                                            valueIndex === fi
                                              ? {
                                                  ...value,
                                                  value: event.target.value,
                                                }
                                              : value,
                                        ),
                                      }
                                    : item,
                                ),
                              }
                            : part,
                        ),
                      })
                    }
                  />
                </label>
              ))}
              {entry.bullets.map((bullet, bi) => (
                <label key={bullet.id}>
                  Bullet {bi + 1}
                  <textarea
                    value={bullet.text}
                    onChange={(event) =>
                      onChange({
                        ...document,
                        sections: document.sections.map((part, partIndex) =>
                          partIndex === si
                            ? {
                                ...part,
                                entries: part.entries.map((item, itemIndex) =>
                                  itemIndex === ei
                                    ? {
                                        ...item,
                                        bullets: item.bullets.map(
                                          (value, valueIndex) =>
                                            valueIndex === bi
                                              ? {
                                                  ...value,
                                                  text: event.target.value,
                                                }
                                              : value,
                                        ),
                                      }
                                    : item,
                                ),
                              }
                            : part,
                        ),
                      })
                    }
                  />
                </label>
              ))}
              {entry.links.map((link, li) => (
                <div key={link.id ?? li}>
                  <label>
                    Entry link {li + 1} label
                    <input
                      value={link.label}
                      onChange={(event) =>
                        onChange({
                          ...document,
                          sections: document.sections.map((part, partIndex) =>
                            partIndex === si
                              ? {
                                  ...part,
                                  entries: part.entries.map(
                                    (item, itemIndex) =>
                                      itemIndex === ei
                                        ? {
                                            ...item,
                                            links: item.links.map(
                                              (value, linkIndex) =>
                                                linkIndex === li
                                                  ? {
                                                      ...value,
                                                      label: event.target.value,
                                                    }
                                                  : value,
                                            ),
                                          }
                                        : item,
                                  ),
                                }
                              : part,
                          ),
                        })
                      }
                    />
                  </label>
                  <label>
                    Entry link {li + 1} URL
                    <input
                      value={link.url}
                      onChange={(event) =>
                        onChange({
                          ...document,
                          sections: document.sections.map((part, partIndex) =>
                            partIndex === si
                              ? {
                                  ...part,
                                  entries: part.entries.map(
                                    (item, itemIndex) =>
                                      itemIndex === ei
                                        ? {
                                            ...item,
                                            links: item.links.map(
                                              (value, linkIndex) =>
                                                linkIndex === li
                                                  ? {
                                                      ...value,
                                                      url: event.target.value,
                                                    }
                                                  : value,
                                            ),
                                          }
                                        : item,
                                  ),
                                }
                              : part,
                          ),
                        })
                      }
                    />
                  </label>
                </div>
              ))}
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}
