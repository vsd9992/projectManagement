# Decision: Projects are composable (Workstreams), not a rigid project-type enum

**Decision**: A Project does not have a fixed `project_type` (manufacturing / interiors / civil). Instead, a Project enables any combination of four Workstreams: Design, Manufacturing/Production, Procurement, Site Execution.

**Basis**: Explicit answer to "are furniture manufacturing, turnkey interiors, and civil/architectural run as one integrated business, separate business lines, or mixed/case-by-case?" — answered "Mixed / case-by-case."

**Why**: A rigid project-type enum cannot represent a project that starts as pure civil and later needs a manufacturing workstream, or a turnkey project that blends all three business lines under one project record. The composable model handles all cases (pure manufacturing, pure civil, full turnkey) through the same entity without special-casing project types throughout the system, and allows new workstream types to be added later without redesigning the Project entity.

**Consequences/constraints**: All downstream design (WBS tagging, scheduling, permissions, dashboards) must key off enabled Workstreams, not a project type field. See `.ai/project/architecture.md` § Project Model for the full mechanism. Risk: this flexibility can be over-built beyond what MVP needs — see `.ai/project/risks.md` risk #2.
