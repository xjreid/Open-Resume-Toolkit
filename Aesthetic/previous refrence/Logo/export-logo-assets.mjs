import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const sharp = require("sharp");

const logoDirectory = path.dirname(fileURLToPath(import.meta.url));
const exportDirectory = path.join(logoDirectory, "exports");

const iconSizes = [16, 20, 24, 32, 48, 64, 128, 256, 512, 1024];
const reversedIconSizes = [16, 24, 32, 48, 64, 128, 256, 512];
const horizontalWidths = [160, 240, 320, 480, 640, 960];
const stackedWidths = [128, 256, 512, 1024];

const iconSource = path.join(logoDirectory, "open-frame-icon.svg");
const reversedIconSource = path.join(
  logoDirectory,
  "open-frame-icon-reversed.svg",
);
const horizontalSource = path.join(
  logoDirectory,
  "open-frame-lockup-horizontal.svg",
);
const reversedHorizontalSource = path.join(
  logoDirectory,
  "open-frame-lockup-horizontal-reversed.svg",
);
const stackedSource = path.join(
  logoDirectory,
  "open-frame-lockup-stacked.svg",
);

await Promise.all([
  fs.mkdir(path.join(exportDirectory, "icons"), { recursive: true }),
  fs.mkdir(path.join(exportDirectory, "reversed"), { recursive: true }),
  fs.mkdir(path.join(exportDirectory, "lockups"), { recursive: true }),
]);

for (const size of iconSizes) {
  await sharp(iconSource)
    .resize(size, size)
    .png()
    .toFile(path.join(exportDirectory, "icons", `open-frame-${size}.png`));
}

for (const size of reversedIconSizes) {
  await sharp(reversedIconSource)
    .resize(size, size)
    .png()
    .toFile(
      path.join(exportDirectory, "reversed", `open-frame-reversed-${size}.png`),
    );
}

for (const width of horizontalWidths) {
  await sharp(horizontalSource)
    .resize({ width })
    .png()
    .toFile(
      path.join(
        exportDirectory,
        "lockups",
        `open-frame-horizontal-${width}.png`,
      ),
    );

  await sharp(reversedHorizontalSource)
    .resize({ width })
    .png()
    .toFile(
      path.join(
        exportDirectory,
        "reversed",
        `open-frame-horizontal-reversed-${width}.png`,
      ),
    );
}

for (const width of stackedWidths) {
  await sharp(stackedSource)
    .resize({ width })
    .png()
    .toFile(
      path.join(
        exportDirectory,
        "lockups",
        `open-frame-stacked-${width}.png`,
      ),
    );
}

const manifest = {
  source: {
    icon: "../open-frame-icon.svg",
    iconReversed: "../open-frame-icon-reversed.svg",
    horizontal: "../open-frame-lockup-horizontal.svg",
    horizontalReversed: "../open-frame-lockup-horizontal-reversed.svg",
    stacked: "../open-frame-lockup-stacked.svg",
  },
  icons: iconSizes,
  reversedIcons: reversedIconSizes,
  horizontalWidths,
  stackedWidths,
  color: "#102A4C",
  generatedBy: "export-logo-assets.mjs",
};

await fs.writeFile(
  path.join(exportDirectory, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
  "utf8",
);
