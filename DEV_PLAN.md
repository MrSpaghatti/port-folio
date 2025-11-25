# Port-Folio: Development Plan

## Project Overview
**Port-Folio** is a Terminal User Interface (TUI) network monitor built in Rust. It aims to provide real-time visibility into active network connections while adding an educational layer powered by local AI/heuristics to explain *what* connections are doing and *why* they might be risky.

## Core Features
1.  **Network Monitoring:** Real-time list of active TCP/UDP connections (Process ID, Local Address, Remote Address, State).
2.  **TUI Dashboard:** A clean, interactive terminal interface using `ratatui`.
3.  **Process Inspection:** Detailed view of the process associated with a connection.
4.  **AI/Heuristic Analysis:**
    -   Analyze suspicious ports or IPs.
    -   Provide educational context (e.g., "Port 445 is SMB, exposing this to the internet is risky because...").
5.  **Packet Sniffer (Optional/Advanced):** Basic packet capture for selected streams.

## Tech Stack
-   **Language:** Rust
-   **UI:** `ratatui` (TUI framework), `crossterm`.
-   **Async Runtime:** `tokio`.
-   **System Info:** `sysinfo` (for process/system details).
-   **Network:** `netstat` parsing or platform-specific socket reading.
-   **AI/LLM:** `ollama-rs` or `rust-bert` (for local inference).

## Milestones

### Phase 1: The Skeleton (MVP)
-   [ ] Set up Rust project structure.
-   [ ] Implement basic TUI layout (Process List, Details Pane, Log Pane).
-   [ ] Fetch and display a static list of active network connections.

### Phase 2: Live Data & Interaction
-   [ ] Implement real-time refreshing of connection data.
-   [ ] Add keyboard navigation (up/down to select connections).
-   [ ] Integrate `sysinfo` to resolve PIDs to Process Names/Paths.

### Phase 3: The "Doctor" (Analysis Layer)
-   [ ] Implement a "Analyze" button/shortcut.
-   [ ] Integrate a basic heuristic engine (Port 80 = HTTP, 22 = SSH).
-   [ ] (Optional) Integrate LLM client for generating descriptions of unknown ports/IPs.

### Phase 4: Polish & Refine
-   [ ] Color coding (Red for public IPs, Green for local).
-   [ ] Search/Filter functionality.
-   [ ] Documentation and Help screen.

## User Experience Goal
The user should feel like they have a "network x-ray" that doesn't just show data, but teaches them how to interpret it.
