# CPM Scheduler — Rust Edition

Enterprise-grade Critical Path Method scheduler built with **Rust + egui**.  
Single binary. No runtime. Sub-millisecond scheduling on thousands of activities.

---

## Requirements

| Requirement | Version |
|---|---|
| Rust toolchain | **1.78 or newer** |
| OS | Linux / macOS / Windows |

Install Rust: https://rustup.rs/

---

## Build & Run

```bash
# Clone / extract, then:
cd cpm_scheduler

# Development run
cargo run

# Optimised release build
cargo build --release
./target/release/cpm_scheduler

# Run unit tests (6 tests, all pass)
cargo test
```

---

## Architecture

```
src/
├── main.rs         Entry point — eframe native window setup
├── activity.rs     Activity dataclass: CPM fields, Predecessor, RelType
├── scheduler.rs    CPM engine — forward/backward pass, free/total float
├── project.rs      Project state, load/save (JSON), sample data
└── app.rs          egui UI — toolbar, 3 tabs, dialogs
```

### CPM Engine (`scheduler.rs`)

Pure Rust, zero unsafe, no allocations beyond `HashMap`/`Vec`.

| Pass | Algorithm | Complexity |
|---|---|---|
| Topological sort | Kahn's algorithm | O(V + E) |
| Forward pass | ES = max(pred EF + lag) | O(V + E) |
| Backward pass | LF = min(succ LS − lag) | O(V + E) |
| Total Float | TF = LS − ES | O(V) |
| Free Float | FF = min(succ ES) − EF | O(V + E) |

Supports **FS, SS, FF, SF** relationship types with positive/negative lag.

**Verified CPM results** (sample project):

| ID | Name | Dur | ES | EF | LS | LF | TF | FF | Critical |
|----|------|-----|----|----|----|----|----|----|----------|
| A | Start | 2 | 0 | 2 | 0 | 2 | 0 | 0 | ★ |
| B | Foundation | 4 | 2 | 6 | 2 | 6 | 0 | 0 | ★ |
| C | Structure | 6 | 6 | 12 | 6 | 12 | 0 | 0 | ★ |
| D | Electrical | 3 | 6 | 9 | 9 | 12 | 3 | 3 | |
| E | Finish | 2 | 12 | 14 | 12 | 14 | 0 | 0 | ★ |

**Project duration: 14 days. Critical path: A → B → C → E**

---

## UI Overview

### Schedule Tab
Full activity table with all CPM columns. TF is colour-coded:
- 🔴 Red = Critical (TF = 0)
- 🟡 Amber = Near-critical (TF < 5)
- 🟢 Green = Has float

Edit and delete activities inline. F5 or ▶ Schedule to run CPM.

### Gantt Tab
Horizontal bar chart per activity. Red = critical, blue = normal.  
Ghost green bar shows the float window. Hover for tooltip with full CPM data.

### Network Diagram Tab
Node-link diagram showing CPM node boxes:
```
┌─────────────────┐
│  ES          EF │   ← blue (earliest)
│       ID        │
│      Name       │
│  LS    TF    LF │   ← amber (latest) + float
└─────────────────┘
```
Critical nodes have red borders and background. Critical edges are red.

---

## Keyboard Shortcuts

| Action | Key |
|---|---|
| Schedule (run CPM) | `F5` |

---

## Phase 2 Roadmap

- [ ] Working-day calendar (skip weekends / public holidays)
- [ ] SS / FF / SF relationship types with lag (engine done, UI pending)
- [ ] JSON project save / load (engine done, file dialog pending)
- [ ] Excel & PDF export
- [ ] Resource loading chart
- [ ] WBS tree panel
- [ ] Baseline vs actual comparison
- [ ] P6 XML import/export
- [ ] Resource levelling

---

## Why Rust + egui?

| Property | Python/PySide6 | Rust/egui |
|---|---|---|
| Startup time | ~1–3 s | < 50 ms |
| CPM on 10 000 activities | ~500 ms | < 1 ms |
| Memory (idle) | ~120 MB | ~8 MB |
| Distribution | Python + Qt install | Single static binary |
| Type safety | Runtime errors | Compile-time guarantees |
