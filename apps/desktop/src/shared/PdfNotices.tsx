import apache from "../../../../docs/dependencies/pdf/Apache-2.0.txt?raw";
import assetLicense from "../../../../docs/dependencies/pdf/typst-assets-LICENSE.txt?raw";
import assetNotice from "../../../../docs/dependencies/pdf/typst-assets-NOTICE.txt?raw";

export function PdfNotices() {
  return (
    <details>
      <summary>PDF renderer and font licenses</summary>
      <p>
        Typst 0.15.1: Copyright Typst GmbH. PDF.js 6.3.289: Copyright Mozilla
        Foundation. Both use Apache License 2.0. Bundled Libertinus Serif fonts
        use SIL Open Font License 1.1; original font names are unchanged. Full
        upstream asset notices are preserved below, including unused asset
        families.
      </p>
      <details>
        <summary>Apache License 2.0</summary>
        <pre className="license-text">{apache}</pre>
      </details>
      <details>
        <summary>Typst assets license</summary>
        <pre className="license-text">{assetLicense}</pre>
      </details>
      <details>
        <summary>Typst assets notices and font licenses</summary>
        <pre className="license-text">{assetNotice}</pre>
      </details>
    </details>
  );
}
