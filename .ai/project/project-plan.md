modified: 2026-08-27

# Project Plan

## What
A multi-tenant project management SaaS purpose-built for businesses that do furniture manufacturing, turnkey interior fit-outs, and civil/architectural work — individually or blended within a single project.

## Why
Market research (`../../Comprehensive Project Management Software Market & Functionality Analysis.md`) shows a structural gap: heavy ERPs (SAP, Oracle) win on financial governance but fail at field usability; construction platforms (Procore, ACC) win at the job site but lack manufacturing depth; agile tools (monday.com, Wrike) are intuitive but too shallow for BOQ/BOM/statutory billing; localized players (Lighthouse, StrategicERP) win on regional compliance but aren't composable SaaS products. Nothing serves a business spanning these three verticals under one intuitive, fully traceable system.

## Scope & Boundaries
- In scope: a composable project/workstream model covering Design, Manufacturing/Production, Procurement, and Site Execution; multi-tenant org hierarchy (tenant → business unit/branch/factory); a generic billing/tax engine with India as the first regional profile; full audit/traceability across cost, schedule, and documents.
- Out of scope (for now): full accounting/ERP (general ledger, AP/AR, statutory filing), vendor-facing portal, deep standalone Furniture Manufacturing and Civil/Architectural verticals (beyond what Turnkey Interiors needs), multi-region billing profiles beyond India.
- Not yet decided: the technology stack (explicitly deferred by the product owner until workflow/use case and system design were finalized — now unblocked, to be discussed next).

## Major Capabilities
- Composable per-project workstreams instead of a rigid project-type enum (see `architecture.md`).
- Non-linear, concurrent workflow with explicit task dependencies and first-class Change Orders for scope changes (see `workflows.md`).
- Full transparency & traceability: audit logging and document versioning as core platform services, not per-module add-ons.
- Generic billing engine (milestone / progressive RA-style / lump-sum) driven by a pluggable regional tax/rule profile.
- Role-scoped views: internal roles see detail scoped to their business unit/workstream; an external Client Portal gives clients a curated read/approve view of their own project only.

## Assumptions
- The product will eventually be sold to other firms in this industry (multi-tenant from day one), even though the first real usage may be the founder's own operations.
- India is the first and most detailed regional compliance target; the billing/tax engine must not hardcode India-only assumptions into the core.
- "Intuitive and agile" is a hard product constraint, not just a preference — the design must actively avoid becoming as complex as the enterprise ERPs it differentiates against.

## Success Criteria
- A Turnkey Interiors project can run end-to-end (Lead → Quotation/BOQ → Design → Client Approval → Procurement → Site Execution → Installation → Handover → Milestone Billing) with every cost, schedule, and document change traceable to who/when/why.
- A client can approve a design and a change order, and view progress and invoices, through a portal without needing internal system access.
- A pure furniture order and a pure civil job can both be represented without forcing irrelevant workstreams or awkward workarounds (validated in `workflows.md` scenario walkthroughs).
- The same core data model (tenant, project, workstream, WBS/BOQ, change order, billing engine) requires no redesign when the standalone Furniture Manufacturing and Civil/Architectural verticals are built out later.

## High-Level Phases
See `roadmap.md` for the detailed phase/milestone breakdown. Summary: Planning & Evaluation (this baseline) → stack selection → Execution (MVP = Turnkey Interiors, full depth) → Testing & Bug Fixing → extend to Furniture Manufacturing and Civil/Architectural verticals.
