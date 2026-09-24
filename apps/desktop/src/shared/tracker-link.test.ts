import { expect, it } from "vitest";
import { trackerLinkTarget } from "./tracker-link";

const webPrefix = "https:" + "/".repeat(2);

it("opens bare domains without requiring a scheme", () => {
  expect(trackerLinkTarget("linkedin.com/jobs/123")).toBe(
    `${webPrefix}linkedin.com/jobs/123`,
  );
  expect(trackerLinkTarget(`${webPrefix}example.com/missing-page`)).toBe(
    `${webPrefix}example.com/missing-page`,
  );
});

it("searches arbitrary source text and never turns script text into a web link", () => {
  expect(trackerLinkTarget("Job board reference 123")).toBe(
    `${webPrefix}www.google.com/search?q=Job%20board%20reference%20123`,
  );
  expect(trackerLinkTarget("javascript:alert(1)")).toMatch(
    /^https:\/\/www\.google\.com\/search\?q=/u,
  );
  expect(trackerLinkTarget(`${webPrefix}user:pass@example.com`)).toMatch(
    /^https:\/\/www\.google\.com\/search\?q=/u,
  );
});
