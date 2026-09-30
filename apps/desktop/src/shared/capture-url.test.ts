import { expect, it } from "vitest";
import { sanitizeCaptureUrl } from "./capture-url";

it("removes credentials, fragments and tracking fields from a reviewed source URL", () => {
  expect(
    sanitizeCaptureUrl(
      "https" +
        "://user:pass@example.test/job?role=dev&utm_source=x&token=secret#apply",
    ),
  ).toBe("https" + "://example.test/job?role=dev");
  expect(sanitizeCaptureUrl(" ")).toBe("");
  expect(
    sanitizeCaptureUrl(
      "https" +
        "://example.test/job?jobId=42&API_KEY=secret&SESSIONID=secret&refresh_token=secret&password=secret&ref=jobs",
    ),
  ).toBe("https" + "://example.test/job?jobId=42&ref=jobs");
  expect(() => sanitizeCaptureUrl("file:///tmp/job")).toThrow();
  expect(() =>
    sanitizeCaptureUrl(`https${"://"}example.test/?q=${"x".repeat(4096)}`),
  ).toThrow();
});
