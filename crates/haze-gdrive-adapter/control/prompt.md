# W1-GDA-FAN-IN-ARCHITECTURE-ACCEPTED-HOLD

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: orchestrator-hold
Phase: GDA-FAN-IN-ARCHITECTURE-ACCEPTED-HOLD

Do not implement, merge, change draft state, rewrite history, modify sibling branches, or begin provider/runtime/transport/deployment work.

The GDrive fan-in architecture is accepted by report blob `14c427880e1201d851cdc9ee04b9cd0e83334de4`.

Pinned boundaries:
- one standalone long-running GDrive Adapter process;
- API + Server own authenticated public HTTP contracts;
- Storage owns durable state through Server/API-mediated access;
- direct adapter database access is forbidden;
- GDrive Adapter owns Google/OAuth/provider lifecycle and scheduling;
- Deployment service wiring remains blocked until runtime, transport, persistence, live OAuth/provider lifecycle, status and fake integration are accepted.

The first executable owner slot is Storage phase `STOR-GDA-P1-DURABLE-STATE`.

Wait for Orchestrator assignment.
