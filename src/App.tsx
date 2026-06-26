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

const DEFAULT_QUERY =
  "SELECT name, type FROM sqlite_master WHERE type = 'table' ORDER BY name";

type QueryState = "idle" | "loading" | "success" | "empty" | "error";
type TablesState = "idle" | "loading" | "success" | "empty" | "error";
type SchemaState = "idle" | "loading" | "success" | "empty" | "error";
type PreviewState = "idle" | "loading" | "success" | "empty" | "error";
type EditabilityState = "idle" | "loading" | "ready";

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
  const [editabilityState, setEditabilityState] =
    useState<EditabilityState>("idle");

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
              className={`editability-status ${
                primaryKeyColumn ? "editable" : "read-only"
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
                <DataGrid columns={tablePreview.columns} rows={tablePreview.rows} />
              </>
            )}
          </section>
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
