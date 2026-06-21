import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { AppError, parseAppError, displayAppErrorMessage } from "./errors";

export interface AppInfo {
  name: string;
  version: string;
  description: string;
}

export interface Note {
  id: number;
  title: string;
  content: string;
}

export interface DatabaseHealth {
  sqlite_available: boolean;
  sample_query_passed: boolean;
  message: string | null;
}

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [myMsg, setMyMsg] = useState("");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [sqlitePing, setSQLitePing] = useState<boolean>(false);
  const [note, setNote] = useState<Note | null>(null);
  const [databaseHealth, setDatabaseHealth] = useState<DatabaseHealth | null>(null);
  const [databaseOpen, setDatabaseOpen] = useState<boolean>(false);

  async function openDatabase(path: string) {
    try {
      const result = await invoke("open_database", { path }) as DatabaseHealth;
      setDatabaseOpen(result.sqlite_available as boolean);
    } catch (err) {
      setError(parseAppError(err));
      setDatabaseOpen(false);
    }
  }
  async function getDatabaseHealth() {
    try {
      const health = await invoke("get_database_health");
      setDatabaseHealth(health as DatabaseHealth);
    } catch (err) {
      setError(parseAppError(err));
    }
  }

  async function createNote() {
    try {
      const note = await invoke("create_note", { title: "Test Note", content: "Test Content" });
      setNote(note as Note);
    } catch (err) {
      setError(parseAppError(err));
    }
  }
  async function pingSQLite() {
    try {
      await invoke("ping_sqlite_command");
      setSQLitePing(true);
    } catch (err) {
      setError(parseAppError(err));
      setSQLitePing(false);
    }
  }
  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  async function myCommand() {
    setMyMsg(await invoke("my_command"));
  }

  async function getAppInfo() {
    setAppInfo(await invoke("get_app_info"));
  }

  async function checkDatabaseSupport() {
    try {
      await invoke("check_database_support");
      setError(null);
    } catch (err) {
      setError(parseAppError(err));
    }
  }

  useEffect(() => {
    getAppInfo();
    pingSQLite();
    createNote();
    getDatabaseHealth();
  }, []);

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
          myCommand();
          checkDatabaseSupport();
          openDatabase("funny_test_data.sqlite");

        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg}</p>
      <p>{myMsg}</p>
      {appInfo && (
        <>
          <p>Name: {appInfo.name}</p>
          <p>Version: {appInfo.version}</p>
          <p>Description: {appInfo.description}</p>
        </>
      )}
      {error && (
        <>
          <p>Error: {displayAppErrorMessage(error)}</p>
        </>
      )}
      {sqlitePing && (
        <>
          <p>SQLite Ping: Success</p>
        </>
      )}
      {note && (
        <>
          <p>Note: {note.title}</p>
          <p>Note: {note.content}</p>
        </>
      )}
      {databaseHealth && (
        <>
          <p>Database Health: {databaseHealth.message}</p>
          <p>SQLite Available: {databaseHealth.sqlite_available ? "Yes" : "No"}</p>
          <p>Sample Query Passed: {databaseHealth.sample_query_passed ? "Yes" : "No"}</p>
        </>
      )}
      {databaseOpen && (
        <>
          <p>Database Open: Success</p>
        </>
      )}
    </main>
  );
}

export default App;
