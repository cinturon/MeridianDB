import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { parseAppError, displayAppErrorMessage } from "./errors";
import { DataGrid } from "./DataGrid";

export interface AppInfo {
  name: string;
  version: string;
  description: string;
}

export interface TableInfo {
  name: string;
  table_type: string;
}

export interface ColumnInfo {
  cid: number;
  name: string;
  data_type: string;
  not_null: boolean;
  default_value: string | null;
  primary_key: boolean;
}

export interface TablePreview {
  columns: string[];
  rows: string[][];
  limit: number;
}

export interface QueryResult {
  columns: string[];
  rows: string[][];
  row_count: number;
  duration_ms: number | null;
}

export interface DraftCellEdit {
  tableName: string;
  primaryKeyColumn: string;
  primaryKeyValue: string;
  targetColumn: string;
  originalValue: string | null;
  newValue: string | null;
}

export interface CellEditResult {
  rows_updated: number;
}

export interface ChangeHistoryEntry {
  timestamp: string;
  cell_edit_request: CellEditRequest;
}

export interface CellEditRequest {
  table_name: string;
  primary_key_column: string;
  primary_key_value: string;
  target_column: string;
  new_value: string | null;
  original_value: string | null;
}


const DEFAULT_QUERY =
  "SELECT name, type FROM sqlite_master WHERE type = 'table' ORDER BY name";

type QueryState = "idle" | "loading" | "success" | "empty" | "error";
type TablesState = "idle" | "loading" | "success" | "empty" | "error";
type SchemaState = "idle" | "loading" | "success" | "empty" | "error";
type PreviewState = "idle" | "loading" | "success" | "empty" | "error";
type EditabilityState = "idle" | "loading" | "ready";
type ChangeHistoryState = "idle" | "loading" | "success" | "empty" | "error";

function formatCellDisplay(value: string | null): string {
  if (value === null || value === "") {
    return "—";
  }
  return value;
}

function formatHistoryTimestamp(timestamp: string): string {
  const seconds = Number(timestamp);
  if (Number.isNaN(seconds)) {
    return timestamp;
  }
  return new Date(seconds * 1000).toLocaleString();
}

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [databasePath, setDatabasePath] = useState("");
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [tablesState, setTablesState] = useState<TablesState>("idle");
  const [tablesError, setTablesError] = useState<string | null>(null);
  const [selectedTable, setSelectedTable] = useState<TableInfo | null>(null);
  const [tableSchema, setTableSchema] = useState<ColumnInfo[]>([]);
  const [schemaState, setSchemaState] = useState<SchemaState>("idle");
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const [tablePreview, setTablePreview] = useState<TablePreview | null>(null);
  const [previewState, setPreviewState] = useState<PreviewState>("idle");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [queryState, setQueryState] = useState<QueryState>("idle");
  const [queryError, setQueryError] = useState<string | null>(null);
  const [querySql, setQuerySql] = useState(DEFAULT_QUERY);
  const [primaryKeyColumn, setPrimaryKeyColumn] = useState<string | null>(null);
  const [editabilityState, setEditabilityState] = useState<EditabilityState>("idle");
  const [draftCellEdit, setDraftCellEdit] = useState<DraftCellEdit | null>(null);
  const [draftError, setDraftError] = useState<string | null>(null);
  const [affectedRows, setAffectedRows] = useState<number>(0);
  const [showSaveConfirm, setShowSaveConfirm] = useState(false);
  const [saving, setSaving] = useState(false);
  const [changeHistory, setChangeHistory] = useState<ChangeHistoryEntry[]>([]);
  const [changeHistoryState, setChangeHistoryState] = useState<ChangeHistoryState>("idle");
  const [changeHistoryError, setChangeHistoryError] = useState<string | null>(null);

  function handleRequestSave() {
    if (!draftCellEdit) {
      return;
    }
    setDraftError(null);
    setShowSaveConfirm(true);
  }

  function handleCancelSaveConfirm() {
    setShowSaveConfirm(false);
  }

  async function handleConfirmSave() {
    if (!draftCellEdit || saving) {
      return;
    }
    setSaving(true);
    const path = databasePath.trim();
    let saveSucceeded = false;

    try {
      const result = await invoke<CellEditResult>("update_cell", {
        path,
        request: {
          table_name: draftCellEdit.tableName,
          primary_key_column: draftCellEdit.primaryKeyColumn,
          primary_key_value: draftCellEdit.primaryKeyValue,
          target_column: draftCellEdit.targetColumn,
          new_value: draftCellEdit.newValue,
          original_value: draftCellEdit.originalValue,
        },
      });
      setShowSaveConfirm(false);
      setAffectedRows(result.rows_updated);
      setDraftError(null);
      clearDraftCellEdit();
      saveSucceeded = true;
    } catch (error) {
      const parsed = parseAppError(error);
      setDraftError(parsed ? displayAppErrorMessage(parsed) : "Could not save draft.");
    } finally {
      setSaving(false);
    }

    if (!saveSucceeded) {
      return;
    }

    if (selectedTable) {
      try {
        await handleSelectTable(selectedTable);
      } catch {
        setPreviewState("error");
        setPreviewError(
          "Save succeeded, but the table preview could not be refreshed.",
        );
      }
    }

    if (selectedTable) {
      await loadChangeHistory(path, selectedTable.name);
    }
  }

  function clearDraftCellEdit() {
    setDraftCellEdit(null);
    setShowSaveConfirm(false);
  }

  function handleStartCellEdit(
    targetColumn: string,
    primaryKeyValue: string,
    value: string | null,
  ) {
    if (!selectedTable || !primaryKeyColumn) {
      return;
    }

    setShowSaveConfirm(false);
    setDraftCellEdit({
      tableName: selectedTable.name,
      primaryKeyColumn,
      primaryKeyValue,
      targetColumn,
      originalValue: value,
      newValue: value,
    });
  }

  function handleDraftChange(newValue: string) {
    setDraftCellEdit((draft) => (draft ? { ...draft, newValue } : null));
  }

  async function loadChangeHistory(path: string, tableName: string | null = null) {
    setChangeHistoryState("loading");
    setChangeHistoryError(null);

    try {
      const history = await invoke<ChangeHistoryEntry[]>("get_change_history", {
        path,
        tableName,
      });
      setChangeHistory(history);
      setChangeHistoryState(history.length === 0 ? "empty" : "success");
    } catch (err) {
      const parsed = parseAppError(err);
      setChangeHistoryState("error");
      setChangeHistoryError(
        parsed ? displayAppErrorMessage(parsed) : "Could not load change history.",
      );
      setChangeHistory([]);
    }
  }

  useEffect(() => {
    invoke<AppInfo>("get_app_info").then(setAppInfo);
  }, []);

  async function handleSelectTable(table: TableInfo) {
    const path = databasePath.trim();
    if (!path) {
      setSchemaState("error");
      setSchemaError("Enter a database path before inspecting a table.");
      return;
    }

    setSelectedTable(table);
    setSchemaState("loading");
    setSchemaError(null);
    setTableSchema([]);
    setPreviewState("loading");
    setPreviewError(null);
    setTablePreview(null);
    setPrimaryKeyColumn(null);
    setEditabilityState("loading");
    clearDraftCellEdit();

    await Promise.all([
      invoke<string | null>("get_primary_key_column", {
        path,
        tableName: table.name,
      })
        .then((pk) => {
          setPrimaryKeyColumn(pk);
          setEditabilityState("ready");
        })
        .catch(() => {
          setPrimaryKeyColumn(null);
          setEditabilityState("ready");
        }),
      invoke<ColumnInfo[]>("inspect_table_schema", {
        path,
        tableName: table.name,
      })
        .then((columns) => {
          setTableSchema(columns);
          setSchemaState(columns.length === 0 ? "empty" : "success");
        })
        .catch((err) => {
          const parsed = parseAppError(err);
          setSchemaState("error");
          setSchemaError(
            parsed
              ? displayAppErrorMessage(parsed)
              : "Could not inspect table schema.",
          );
          setTableSchema([]);
        }),
      invoke<TablePreview>("preview_table", {
        path,
        tableName: table.name,
      })
        .then((preview) => {
          setTablePreview(preview);
          setPreviewState(preview.rows.length === 0 ? "empty" : "success");
        })
        .catch((err) => {
          const parsed = parseAppError(err);
          setPreviewState("error");
          setPreviewError(
            parsed ? displayAppErrorMessage(parsed) : "Could not load table preview.",
          );
          setTablePreview(null);
        }),
    ]);

    await loadChangeHistory(path, table.name);
  }

  async function handleListTables(event: React.FormEvent) {
    event.preventDefault();

    const path = databasePath.trim();
    if (!path) {
      setTablesState("error");
      setTablesError("Enter a path to a SQLite database file.");
      setTables([]);
      return;
    }

    setTablesState("loading");
    setTablesError(null);
    setSelectedTable(null);
    setTableSchema([]);
    setSchemaState("idle");
    setSchemaError(null);
    setTablePreview(null);
    setPreviewState("idle");
    setPreviewError(null);
    setPrimaryKeyColumn(null);
    setEditabilityState("idle");
    clearDraftCellEdit();
    setChangeHistory([]);
    setChangeHistoryState("idle");
    setChangeHistoryError(null);

    try {
      const result = await invoke<TableInfo[]>("list_tables", { path });
      setTables(result);
      setTablesState(result.length === 0 ? "empty" : "success");
    } catch (err) {
      const parsed = parseAppError(err);
      setTablesState("error");
      setTablesError(
        parsed ? displayAppErrorMessage(parsed) : "Could not list tables.",
      );
      setTables([]);
    }
  }

  async function handleRunQuery(event: React.FormEvent) {
    event.preventDefault();

    const path = databasePath.trim();
    const sql = querySql.trim();
    if (!path) {
      setQueryState("error");
      setQueryError("Enter a database path before running a query.");
      setQueryResult(null);
      return;
    }
    if (!sql) {
      setQueryState("error");
      setQueryError("Enter a SELECT query before running.");
      setQueryResult(null);
      return;
    }

    setQueryState("loading");
    setQueryError(null);
    setQueryResult(null);

    try {
      const result = await invoke<QueryResult>("query", { path, sql });
      setQueryResult(result);
      setQueryState(result.row_count === 0 ? "empty" : "success");
    } catch (err) {
      const parsed = parseAppError(err);
      setQueryState("error");
      setQueryError(
        parsed ? displayAppErrorMessage(parsed) : "Could not run query.",
      );
      setQueryResult(null);
    }
  }

  return (
    <main className="container">
      <header className="app-header">
        <img src="/favicon.png" className="app-logo" alt="MeridianDB logo" />
        <div>
          <h1>{appInfo?.name ?? "MeridianDB"}</h1>
          {appInfo && (
            <p className="app-subtitle">
              v{appInfo.version} — {appInfo.description}
            </p>
          )}
        </div>
      </header>

      <section className="explorer-panel" aria-labelledby="explorer-heading">
        <h2 id="explorer-heading">Tables</h2>
        <p className="panel-hint">
          Open a SQLite file, list tables, then click a table to inspect its schema and preview rows.
        </p>

        <form className="path-form" onSubmit={handleListTables}>
          <label htmlFor="database-path" className="sr-only">
            Database path
          </label>
          <input
            id="database-path"
            type="text"
            value={databasePath}
            onChange={(e) => setDatabasePath(e.currentTarget.value)}
            placeholder="e.g. ../test_data.sqlite"
            disabled={tablesState === "loading"}
          />
          <button type="submit" disabled={tablesState === "loading"}>
            {tablesState === "loading" ? "Listing…" : "List Tables"}
          </button>
        </form>

        <div className="tables-result" aria-live="polite">
          {tablesState === "idle" && (
            <p className="status-message">Enter a database path and click List Tables.</p>
          )}

          {tablesState === "loading" && (
            <p className="status-message">Loading tables…</p>
          )}

          {tablesState === "error" && tablesError && (
            <p className="status-message error">{tablesError}</p>
          )}

          {tablesState === "empty" && (
            <p className="status-message">No tables found in this database.</p>
          )}

          {tablesState === "success" && (
            <>
              <p className="table-count">
                {tables.length} table{tables.length === 1 ? "" : "s"} found
              </p>
              <ul className="table-list">
                {tables.map((table) => (
                  <li
                    key={table.name}
                    className={
                      selectedTable?.name === table.name ? "selected" : undefined
                    }
                    onClick={() => handleSelectTable(table)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        handleSelectTable(table);
                      }
                    }}
                    role="button"
                    tabIndex={0}
                  >
                    <span className="table-name">{table.name}</span>
                    <span className="table-type">{table.table_type}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>

        {selectedTable && (
          <section
            className="schema-panel"
            aria-labelledby="schema-heading"
            aria-live="polite"
          >
            <h3 id="schema-heading">Schema: {selectedTable.name}</h3>

            <p
              className={`editability-status ${primaryKeyColumn ? "editable" : "read-only"
                }`}
              aria-live="polite"
            >
              {editabilityState === "loading" && "Checking editability…"}
              {editabilityState === "ready" && primaryKeyColumn && (
                <>
                  Editable: primary key <code>{primaryKeyColumn}</code> detected
                </>
              )}
              {editabilityState === "ready" && !primaryKeyColumn && (
                <>Read-only: no simple primary key detected</>
              )}
            </p>

            {schemaState === "loading" && (
              <p className="status-message">Loading columns…</p>
            )}

            {schemaState === "error" && schemaError && (
              <p className="status-message error">{schemaError}</p>
            )}

            {schemaState === "empty" && (
              <p className="status-message">No columns found for this table.</p>
            )}

            {schemaState === "success" && (
              <div className="column-table-wrap">
                <table className="column-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Type</th>
                      <th>Not null</th>
                      <th>Primary key</th>
                      <th>Default</th>
                    </tr>
                  </thead>
                  <tbody>
                    {tableSchema.map((column) => (
                      <tr key={column.cid}>

                        <td className="column-name">{column.name}</td>
                        <td>{column.data_type}</td>
                        <td>{column.not_null ? "Yes" : "No"}</td>
                        <td>{column.primary_key ? "Yes" : "No"}</td>
                        <td className="column-default">
                          {column.default_value ?? "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        )}

        {selectedTable && (
          <section
            className="preview-panel"
            aria-labelledby="preview-heading"
            aria-live="polite"
          >
            <h3 id="preview-heading">Preview: {selectedTable.name}</h3>

            {previewState === "loading" && (
              <p className="status-message">Loading rows…</p>
            )}

            {previewState === "error" && previewError && (
              <p className="status-message error">{previewError}</p>
            )}

            {previewState === "empty" && (
              <p className="status-message">No rows found in this table.</p>
            )}

            {previewState === "success" && tablePreview && (
              <>
                <p className="preview-meta">
                  Showing {tablePreview.rows.length} of up to {tablePreview.limit} rows
                </p>
                {affectedRows > 0 && (
                  <p className="status-message">
                    {affectedRows} row{affectedRows === 1 ? "" : "s"} affected
                  </p>
                )}
                {draftError && (
                  <p className="status-message error">{draftError}</p>
                )}
                {draftCellEdit &&
                  draftCellEdit.tableName === selectedTable.name &&
                  !showSaveConfirm && (
                    <p className="preview-meta draft-edit-hint">
                      Draft edit on <code>{draftCellEdit.targetColumn}</code>
                      {draftCellEdit.newValue !== draftCellEdit.originalValue
                        ? " (unsaved changes)"
                        : " (not saved)"}
                    </p>
                  )}
                {showSaveConfirm && draftCellEdit && (
                  <div
                    className="save-confirm-panel"
                    role="dialog"
                    aria-labelledby="save-confirm-heading"
                    aria-live="polite"
                  >
                    <h4 id="save-confirm-heading">Confirm cell change</h4>
                    <p className="save-confirm-summary">
                      Change <code>{draftCellEdit.targetColumn}</code> in{" "}
                      <code>{draftCellEdit.tableName}</code> where{" "}
                      <code>{draftCellEdit.primaryKeyColumn}</code> is{" "}
                      <code>{draftCellEdit.primaryKeyValue}</code>?
                    </p>
                    <dl className="save-confirm-details">
                      <div>
                        <dt>Original value</dt>
                        <dd>{formatCellDisplay(draftCellEdit.originalValue)}</dd>
                      </div>
                      <div>
                        <dt>New value</dt>
                        <dd>{formatCellDisplay(draftCellEdit.newValue)}</dd>
                      </div>
                    </dl>
                    <div className="save-confirm-actions">
                      <button
                        type="button"
                        className="save-confirm-button"
                        onClick={handleConfirmSave}
                        disabled={saving}
                      >
                        {saving ? "Saving…" : "Confirm save"}
                      </button>
                      <button
                        type="button"
                        className="save-confirm-cancel"
                        onClick={handleCancelSaveConfirm}
                        disabled={saving}
                      >
                        Back to editing
                      </button>
                    </div>
                  </div>
                )}
                <DataGrid
                  columns={tablePreview.columns}
                  rows={tablePreview.rows}
                  editable={editabilityState === "ready" && primaryKeyColumn != null}
                  primaryKeyColumn={primaryKeyColumn}
                  draftCellEdit={draftCellEdit}
                  saveConfirmPending={showSaveConfirm}
                  onStartEdit={(_rowIndex, columnName, primaryKeyValue, value) =>
                    handleStartCellEdit(columnName, primaryKeyValue, value)
                  }
                  onDraftChange={handleDraftChange}
                  onCancelEdit={clearDraftCellEdit}
                  onSaveDraft={handleRequestSave}
                />
              </>
            )}
          </section>
        )}
      </section>

      <section
        className="change-history-panel"
        aria-labelledby="change-history-heading"
        aria-live="polite"
      >
        <h2 id="change-history-heading">
          {selectedTable
            ? `Change History: ${selectedTable.name}`
            : "Change History"}
        </h2>
        <p className="panel-hint">
          {selectedTable
            ? `Recent edits for ${selectedTable.name}, newest first.`
            : "Select a table to see edits for that table only."}
        </p>

        {changeHistoryState === "idle" && (
          <p className="status-message">
            {tablesState === "success"
              ? "Select a table to view its change history."
              : "List tables on a database, then select a table."}
          </p>
        )}

        {changeHistoryState === "loading" && (
          <p className="status-message">Loading change history…</p>
        )}

        {changeHistoryState === "error" && changeHistoryError && (
          <p className="status-message error">{changeHistoryError}</p>
        )}

        {changeHistoryState === "empty" && selectedTable && (
          <p className="status-message">
            No edits recorded yet for table {selectedTable.name}.
          </p>
        )}

        {changeHistoryState === "success" && (
          <ul className="change-history-list">
            {changeHistory.map((entry, index) => (
              <li key={`${entry.timestamp}-${index}`} className="change-history-item">
                <p className="change-history-meta">
                  <time dateTime={entry.timestamp}>
                    {formatHistoryTimestamp(entry.timestamp)}
                  </time>
                  {" · "}
                  <span className="change-history-table">
                    {entry.cell_edit_request.table_name}
                  </span>
                  {" · "}
                  <code>{entry.cell_edit_request.target_column}</code>
                </p>
                <dl className="change-history-values">
                  <div>
                    <dt>Old value</dt>
                    <dd>{formatCellDisplay(entry.cell_edit_request.original_value)}</dd>
                  </div>
                  <div>
                    <dt>New value</dt>
                    <dd>{formatCellDisplay(entry.cell_edit_request.new_value)}</dd>
                  </div>
                </dl>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="workbench-panel" aria-labelledby="workbench-heading">
        <h2 id="workbench-heading">Query</h2>
        <p className="panel-hint">
          Run a read-only SELECT against the open database path.
        </p>

        <form className="query-form" onSubmit={handleRunQuery}>
          <label htmlFor="query-sql" className="sr-only">
            SQL query
          </label>
          <textarea
            id="query-sql"
            className="query-input"
            value={querySql}
            onChange={(e) => setQuerySql(e.currentTarget.value)}
            rows={4}
            spellCheck={false}
            disabled={queryState === "loading"}
          />
          <button type="submit" disabled={queryState === "loading"}>
            {queryState === "loading" ? "Running…" : "Run Query"}
          </button>
        </form>

        <div className="query-result" aria-live="polite">
          {queryState === "idle" && (
            <p className="status-message">
              Enter SQL and click Run Query to see results.
            </p>
          )}

          {queryState === "loading" && (
            <p className="status-message">Running query…</p>
          )}

          {queryState === "error" && queryError && (
            <p className="status-message error">{queryError}</p>
          )}

          {queryState === "empty" && (
            <p className="status-message">Query returned no rows.</p>
          )}

          {queryState === "success" && queryResult && (
            <>
              <p className="preview-meta">
                {queryResult.row_count} row{queryResult.row_count === 1 ? "" : "s"}
                {queryResult.duration_ms != null &&
                  ` · ${queryResult.duration_ms} ms`}
              </p>
              <DataGrid columns={queryResult.columns} rows={queryResult.rows} />
            </>
          )}
        </div>
      </section>
    </main>
  );
}

export default App;
