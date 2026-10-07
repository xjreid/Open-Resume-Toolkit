# M3 direct-provider catalog review

Review date: September 15, 2026. Catalog ID: `2026-09-15.1`.

The bundled catalog was checked against the providers' official model and
pricing documentation before its development baseline was signed:

| Provider/model | Standard synchronous text rates represented by ORT | Official sources |
| --- | --- | --- |
| OpenAI `gpt-5.6-terra` | $2.00 input, $0.20 cached input, and $12.00 output per million tokens | [model/pricing page](https://developers.openai.com/api/docs/models/gpt-5.6-terra), [model comparison](https://developers.openai.com/api/docs/models/compare) |
| Anthropic `claude-sonnet-5` | $2.00 input and $10.00 output per million tokens | [model overview](https://platform.claude.com/docs/en/models/overview), [pricing](https://platform.claude.com/docs/en/about-claude/pricing) |
| Google `gemini-3.6-flash` | $0.75 input, $0.075 cached input, and $3.75 output (including thinking) per million tokens through December 31, 2026 | [model page](https://ai.google.dev/gemini-api/docs/models/gemini-3.6-flash), [pricing](https://ai.google.dev/gemini-api/docs/pricing) |

The OpenAI entry does not assign a cache-write rate because its official model
table lists none. ORT's Gemini normalizer keeps candidate and thinking token
counts distinguishable, then prices both at the documented output rate. The
Gemini entry ends at `2027-01-01T00:00:00Z`; the catalog itself expires just
before then so the promotional price cannot silently continue.

These entries deliberately do not flatten batch/flex/priority, long-context,
cache storage, tools, grounding, taxes, credits, promotions outside the stated
window, or account-specific terms into a misleading universal rate. ORT's
current synthetic connection request uses none of those optional billed
features and its input bound is below OpenAI's long-context threshold.

The exact catalog bytes are signed with the public key in `packages/catalog`.
The development signing private key was generated ephemerally and discarded.
The protected publishing workflow and durable release catalog key remain M7
distribution work; this record does not claim production key custody.
