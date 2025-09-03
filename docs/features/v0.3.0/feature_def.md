# Feature Definition: [Workflow Builder]

## 1. Summary

## 2. User Goal

- Who is this feature for?
  - experienced users

## 3. UI Behavior

- **Location:** Where will it appear in the terminal UI?
- **Appearance:** How should it look (panel, popup, inline text, progress bar, etc.)?
- **Interaction:** What inputs trigger it (keys, mouse, resize)?

\*Example:

- Appears as a single line at the bottom of the screen.
- Displays mode (Normal/Search) and last action.
- Updates in real time as the user types or presses keys.\*

## 4. State Changes

- What new data must be tracked?
- Do we need new enums, structs, or fields in `AppState`?
- How does this affect existing state?

_Example:_

```rust
enum AppMode {
    Normal,
    Search(String), // holds current query
}
```

## 5. Acceptance Criteria

## 6. Scope
