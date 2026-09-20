import anthropicLogo from "../assets/providers/anthropic.svg";
import geminiLogo from "../assets/providers/gemini.png";
import openAiLogo from "../assets/providers/openai.svg";

export type AiProvider = "openai" | "anthropic" | "gemini";

export const providerName = (provider: AiProvider) =>
  provider === "openai"
    ? "OpenAI"
    : provider === "anthropic"
      ? "Anthropic"
      : "Gemini";

const providerLogos: Record<AiProvider, string> = {
  openai: openAiLogo,
  anthropic: anthropicLogo,
  gemini: geminiLogo,
};

export function ProviderLogo({ provider }: { provider: AiProvider }) {
  return <img src={providerLogos[provider]} alt="" aria-hidden="true" />;
}

export function keyDisplayName(key: {
  name?: string | null;
  provider: AiProvider;
  removed?: boolean;
}) {
  if (key.name) return key.name;
  if (key.removed) return "Archived key";
  return `${providerName(key.provider)} key`;
}

export function formatKeyCreatedAt(createdAt: string | null) {
  if (!createdAt) return "Date unavailable";
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) return "Date unavailable";
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(date);
}
