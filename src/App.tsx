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

type TablesState = "idle" | "loading" | "success" | "empty" | "error";

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [databasePath, setDatabasePath] = useState("");
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [tablesState, setTablesState] = useState<TablesState>("idle");
  const [tablesError, setTablesError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppInfo>("get_app_info").then(setAppInfo);
  }, []);

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

    try {
      const result = await invoke<TableInfo[]>("list_tables", { path });
      setTables(result);

      if (result.length === 0) {
        setTablesState("empty");
      } else {
        setTablesState("success");
      }
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
          Open a SQLite file and list the user-created tables inside it.
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
            placeholder="e.g. funny_test_data.sqlite"
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
                  <li key={table.name}>
                    <span className="table-name">{table.name}</span>
                    <span className="table-type">{table.table_type}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      </section>
    </main>
  );
}

export default App;
