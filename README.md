# Open Resume Toolkit planning workspace

## Status

This folder defines **Open Resume Toolkit**: a free, open-source, local-first desktop application with companion Chrome and Edge extensions.

This planning workspace is self-contained. A reviewer should not need another repository, product plan, or historical document to understand the approved product direction and future implementation work.

## Start here

Read [Plan_Index.md](Plan_Index.md). It contains the reading order, authority rules, current decisions, document catalog, and change-control rules.

For a fast product review, read:

1. [Plan index](Plan_Index.md)
2. [Product scope and principles](<Product Plans/Product_Scope_and_Principles.md>)
3. [Core workflows](<Product Plans/Core_Workflows.md>)
4. [Product states and operations](<Product Plans/Product_States_and_Operations.md>)
5. [Local data and document model](<Product Plans/Local_Data_and_Document_Model.md>)
6. [Configuration limits and defaults](<Product Plans/Configuration_Limits_and_Defaults.md>)
7. [Release scope and open decisions](<Product Plans/Release_Scope_and_Open_Decisions.md>)

## Workspace boundaries

- Approved product behavior belongs in `Product Plans/`.
- Future code-level design belongs in `Implementation Plans/`.
- Future visual identity, themes, and document-template styling belong in `Aesthetic/`.
- Unresolved choices and validation work belong only in `Product Plans/Release_Scope_and_Open_Decisions.md`.
- Technical or aesthetic documents may implement approved behavior but may not silently redefine it.

## Product summary

Open Resume Toolkit helps a person maintain one master resume, tailor it to a deliberately captured job description, create optional cover letters and application answers, use a personal OpenAI/Anthropic/Gemini API key or eligible ChatGPT/Codex subscription, compare models and local usage, set optional spend/quota guardrails, export documents locally, and track completed applications. User content, AI activity, and guardrail records remain on the user's computer. The project operates no ORT-owned account system or subscription, hosted database, cloud document store, or centrally funded AI service.

## License, attribution, and official releases

Open Resume Toolkit is licensed under the GNU General Public License version 3 only (`GPL-3.0-only`). Commercial use, modification, and redistribution are permitted subject to the GPL.

- [`LICENSE`](LICENSE) contains the unmodified GPLv3 text.
- [`NOTICE`](NOTICE) identifies the copyright holders and canonical source repository.
- [`ADDITIONAL_TERMS.md`](ADDITIONAL_TERMS.md) contains the GPLv3 Section 7(b) attribution term applicable to identified project material.
- [`TRADEMARKS.md`](TRADEMARKS.md) explains how modified and third-party distributions must distinguish themselves from official releases.

The canonical source repository is <https://github.com/xjreid/Open-Resume-Toolkit>. Official binary channels and signing status will be documented before release; source availability alone does not make a third-party build official.

## Community and security

- Read the [contribution guidelines](.github/CONTRIBUTING.md) before proposing a change. Substantial code contributions are deferred until the recorded architecture and licensing decisions are finalized.
- Participation is governed by the [Code of Conduct](.github/CODE_OF_CONDUCT.md).
- Report vulnerabilities through the private process in the [security policy](.github/SECURITY.md), never through a public issue containing sensitive details.
