# Safe Cell Editing

MeridianDB supports editing **one cell at a time** in table preview rows. The workflow is designed so nothing hits SQLite until the user has reviewed the change, and the backend validates and wraps the write in a transaction.

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
| UI | `src/App.tsx` | Draft state, confirmation panel, `invoke("update_cell", …)` |
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
- **No undo** — a committed edit persists until changed again.
- **Zero-row updates succeed** — if the PK value does not match any row, `rows_updated` is `0` and no data changes.

## Tests

Cell edit behavior is covered in `src-tauri/src/database.rs` under `test_update_cell_*`, including read-back after commit, validation rejection, NOT NULL rollback, and sequential commits.
