import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { parseAppError, displayAppErrorMessage } from "./errors";

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

type TablesState = "idle" | "loading" | "success" | "empty" | "error";
type SchemaState = "idle" | "loading" | "success" | "empty" | "error";
type PreviewState = "idle" | "loading" | "success" | "empty" | "error";

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

    try {
      const columns = await invoke<ColumnInfo[]>("inspect_table_schema", {
        path,
        tableName: table.name,
      });
      setTableSchema(columns);
      setSchemaState(columns.length === 0 ? "empty" : "success");
    } catch (err) {
      const parsed = parseAppError(err);
      setSchemaState("error");
      setSchemaError(
        parsed ? displayAppErrorMessage(parsed) : "Could not inspect table schema.",
      );
      setTableSchema([]);
    }

    try {
      const preview = await invoke<TablePreview>("preview_table", {
        path,
        tableName: table.name,
      });
      setTablePreview(preview);
      setPreviewState(preview.rows.length === 0 ? "empty" : "success");
    } catch (err) {
      const parsed = parseAppError(err);
      setPreviewState("error");
      setPreviewError(
        parsed ? displayAppErrorMessage(parsed) : "Could not load table preview.",
      );
      setTablePreview(null);
    }
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
                <div className="preview-table-wrap">
                  <table className="preview-table">
                    <thead>
                      <tr>
                        {tablePreview.columns.map((column) => (
                          <th key={column}>{column}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {tablePreview.rows.map((row, rowIndex) => (
                        <tr key={rowIndex}>
                          {row.map((cell, cellIndex) => (
                            <td key={`${rowIndex}-${cellIndex}`}>{cell}</td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}
          </section>
        )}
      </section>
    </main>
  );
}

export default App;
