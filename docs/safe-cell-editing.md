# Safe Cell Editing

MeridianDB supports editing **one cell at a time** in table preview rows. The workflow is designed so nothing hits SQLite until the user has reviewed the change, and the backend validates and wraps the write in a transaction. Recorded edits can be **safely undone** from the Change History panel when the live cell still matches the edit (see [Safe undo workflow](#safe-undo-workflow) below).

## End-to-end flow

```text
Click cell → draft in UI → Save → confirm panel → update_cell command
  → validate → transaction → UPDATE → commit → refresh preview
```

1. **Editability check** — When a table is selected, the app calls `get_primary_key_column`. Only tables with exactly **one** primary-key column are editable. Tables with no PK or composite PKs are read-only.
2. **Draft** — Clicking a non-PK cell opens an inline editor. React stores a `DraftCellEdit` (table, PK column/value, target column, original and new values). Nothing is written to the database yet.
3. **Save (review)** — Save opens a confirmation panel showing table, column, row identity, and before/after values. The backend is **not** called at this step.
4. **Confirm save** — Confirm invokes the `update_cell` Tauri command with a snake_case `CellEditRequest` payload.
5. **Validation** — `validate_cell_edit_request` checks required fields, table existence, PK match, non-PK target column, and that the column exists. Failures return `Err` before any transaction starts.
6. **Transaction** — `update_cell` runs a parameterized `UPDATE` inside `unchecked_transaction()`, then `commit()` on success. Execute failures roll back automatically.
7. **Refresh** — On success, the draft clears and the table preview reloads. On failure, the draft stays open and an error message is shown.

## Example

Open `../test_data.sqlite`, select **employees**, click the **name** cell on the row where **id** is `3`, change `Ada` to `Ada Lovelace`, click **Save**, review the confirmation panel, then click **Confirm save**. The preview refreshes with the new value.

## Key files

| Layer | File | Role |
|-------|------|------|
| UI | `src/App.tsx` | Draft state, save/undo confirmation, change history, `invoke` calls |
| UI | `src/DataGrid.tsx` | Inline edit control; disables input while confirmation is open |
| Command | `src-tauri/src/lib.rs` | `update_cell` command wrapper |
| Database | `src-tauri/src/database.rs` | `validate_cell_edit_request`, `update_cell` (transaction) |
| Models | `src-tauri/src/models.rs` | `CellEditRequest`, `CellEditResult` |

## IPC note

Top-level Tauri args use camelCase from JavaScript (`tableName`). Nested structs deserialized by serde use **snake_case** field names (`table_name`, `target_column`, `new_value`).

## Current limitations

- **One cell per edit** — no multi-cell or row-level editing.
- **Simple primary key only** — exactly one PK column; composite or missing PK tables are read-only.
- **Primary key cells cannot be edited** — the PK identifies the row; changing it is blocked.
- **Text input in the UI** — all values are edited as strings; type coercion is SQLite's responsibility.
- **Table preview only** — the query workbench is read-only.
- **Zero-row updates succeed** — if the PK value does not match any row, `rows_updated` is `0` and no data changes.

## Safe undo workflow

Undo is **another database write**, not a magic rollback. MeridianDB treats it like a cell edit: preview first, validate safety, confirm in the UI, then run a transactional `UPDATE`.

### End-to-end flow

```text
Change History → Preview undo → undo_preview command
  → read live cell → stale check → UI status banner
    → Confirm undo → undo_cell command
      → validate_undo_safety → transaction → UPDATE → history insert → commit
        → refresh preview + history
```

1. **History entry** — Each successful `update_cell` appends a row to `meridian_change_history` with the table, PK, column, new value, and original value. The UI loads entries (with `id`) via `get_change_history`.
2. **Preview (read-only)** — **Preview undo** on a history row calls `undo_preview`. The backend loads the history entry, reads the live cell, and builds an `UndoPreview` with `current_value`, `restored_value`, and `is_safe_to_undo`.
3. **Stale check** — Undo is safe only when the live cell still equals the history entry's **new value** (the value written by that edit). If someone changed the cell again, the preview is **unsafe** and undo is blocked.
4. **UI confirmation** — Safe previews show a confirm step with plain-language before/after values. Unsafe previews show a warning only; there is no confirm path.
5. **Execution** — Confirm calls `undo_cell`, which runs `validate_undo_safety` first, then a parameterized `UPDATE` that restores `original_value` from the history entry. The update and a new history row commit in one transaction.
6. **Refresh** — On success, the preview clears, the table preview reloads, and change history refreshes. A success banner appears in the history panel. On failure, the preview stays open and an error banner is shown.

### Example

Continuing the edit example above: after changing **employees.name** for **id** `3` from `Ada` to `Ada Lovelace`, open **Change History** for `employees`, click **Preview undo** on that entry, then **Confirm undo** twice (preview step, then dialog). The name returns to `Ada`, history gains a second entry describing the restore, and the status banner shows success.

To see a blocked undo: edit the same cell again (`Ada Lovelace` → something else), then preview undo on the **first** history entry. The banner warns that the cell changed; confirm is not offered.

### Key undo files

| Layer | File | Role |
|-------|------|------|
| UI | `src/App.tsx` | Preview/confirm flow, status banner, `invoke("undo_preview" \| "undo_cell", …)` |
| Command | `src-tauri/src/lib.rs` | `undo_preview`, `undo_cell` command wrappers |
| Database | `src-tauri/src/database.rs` | `undo_preview`, `validate_undo_safety`, `undo_cell` (transaction) |
| Models | `src-tauri/src/models.rs` | `UndoPreview`, `UndoResult`, `ChangeHistoryEntry` |

### Undo limitations

- **One history entry at a time** — undo restores a single recorded edit; there is no multi-step undo stack in the UI.
- **Stale data blocks undo** — if the cell changed after the history entry, execution is rejected.
- **Same PK rules as editing** — undo targets one row via the stored primary key value.
- **History is append-only** — the original history row is kept; undo adds a new row rather than deleting the edit record.
- **Re-undoing the same entry fails** — after a successful undo, the cell no longer matches that entry's `new_value`, so a second undo on the same id is treated as stale.
- **Table-scoped history in the UI** — the change history panel filters by the selected table.

## Tests

Cell edit behavior is covered in `src-tauri/src/database.rs` under `test_update_cell_*`, including read-back after commit, validation rejection, NOT NULL rollback, and sequential commits.

Undo behavior is covered under `test_undo_preview_*`, `test_validate_undo_safety_*`, and `test_undo_cell_*`, including safe restore, stale rejection, history recording, and transactional rollback when history insert fails.
