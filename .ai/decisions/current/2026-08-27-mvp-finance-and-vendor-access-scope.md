# Decision: MVP ships a minimal Finance role and no vendor-facing portal

**Decision**: (a) MVP includes a minimal Finance role/view — raise and track milestone invoices, mark paid, track retention — not a full ledger/AP-AR module. (b) Procurement is purely internal-facing in MVP; vendors/subcontractors get no portal or login access.

**Basis**: Direct answers given when resolving the plan's open questions: "minimal Finance role/view in the MVP" and "procurement purely internal-facing."

**Why**: Milestone billing cannot function with zero finance participation (someone has to raise/track invoices), but a full accounting system (general ledger, AP/AR, statutory filing) is out of proportion for the MVP's purpose of proving the core workflow model. Similarly, vendor collaboration features (PO acknowledgment, delivery status from the vendor side) add real scope without being necessary to validate the core Turnkey Interiors workflow — internal staff can track PO status manually against vendor communication for now.

**Consequences/constraints**: `.ai/project/requirements.md` and `.ai/project/roadmap.md` (M5) scope Finance and Procurement accordingly. A full Finance & Admin module and a vendor-facing portal are both listed under "After MVP" in `roadmap.md` — not scheduled, but expected future work once the core model is validated.
