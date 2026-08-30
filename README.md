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

Open Resume Toolkit helps a person maintain one master resume, tailor it to a deliberately captured job description, create optional cover letters and application answers, export documents locally, and track completed applications. User content remains on the user's computer. The project operates no product account system, subscription, hosted database, cloud document store, or centrally funded AI service.
