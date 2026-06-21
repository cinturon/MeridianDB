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




function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [myMsg, setMyMsg] = useState("");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [sqlitePing, setSQLitePing] = useState<boolean>(false);

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
    </main>
  );
}

export default App;
