# MeridianDB Architecture

MeridianDB is a desktop SQLite database explorer built with Tauri, Rust, React, and TypeScript. The app is split into four layers. Each layer has a clear job and a clear limit on what it should not do.

## Data Flow

```text
UI → Tauri Command → Rust Service → SQLite → Rust Model → UI
```

A user action starts in the React UI. The UI calls a Tauri command. The command delegates to Rust service code (database logic). The service reads or writes SQLite and maps results into shared Rust models. Those models serialize back across the boundary to the UI.

## Frontend

**Location:** `src/` (for example `App.tsx`, `App.css`)

**Responsible for:**
- Rendering the database explorer UI
- Capturing user input (paths, table selection, buttons)
- Calling Tauri commands with `invoke`
- Showing loading, success, error, and empty states

**Should not:**
- Open SQLite files or run SQL directly
- Encode database rules or business logic
- Know about Rust module layout beyond command names and typed responses

## Commands

**Location:** `src-tauri/src/lib.rs` today; may grow into dedicated command modules later

**Responsible for:**
- Exposing typed operations the frontend can call (`greet`, `my_command`, and future commands like `list_tables`)
- Translating frontend requests into Rust function calls
- Returning serializable results (strings, structs, or friendly errors) to the UI

**Should not:**
- Contain large blocks of SQL or row-mapping logic (that belongs in the database layer)
- Render UI or manage React state
- Grow into a catch-all file as features arrive—commands stay thin

## Database

### Why SQLite

MeridianDB uses SQLite because it is an embedded, file-based database—users open local `.db` files without running a separate database server, and `rusqlite` with bundled SQLite keeps the desktop app portable across platforms.

**Location:** `src-tauri/src/database.rs` (`DatabaseService`, queries, and file-backed SQLite helpers)

**Responsible for:**
- Opening SQLite connections (in-memory for tests, file paths for real databases)
- Running queries such as `SELECT 1`, listing tables from `sqlite_master`, and inspecting schemas with `PRAGMA table_info`
- Validating and executing cell edits inside explicit transactions (see [Safe Cell Editing](safe-cell-editing.md))
- Mapping SQLite rows into Rust structs
- Returning `Result` values that commands can turn into user-facing errors

**Should not:**
- Know about React components or frontend state
- Be invoked directly from TypeScript
- Mix UI concerns (for example formatting a table for display—that is frontend work on data the command already returned)

### DatabaseService

`DatabaseService` is a small struct that gives database behavior one clear home as MeridianDB grows. It is not a framework—just a tidy workbench for related SQLite operations.

**Role today:**
- Owns a `rusqlite::Connection` opened from a filesystem path (`DatabaseService::new(path)`)
- Exposes behavior as small methods, for example `ping_sqlite()` and `db_health_check()`
- Returns shared models such as `DatabaseHealth` that commands serialize to the frontend

**How commands use it:**

```text
invoke → command → DatabaseService::new(path) → method → Result<Model, AppError> → UI
```

Tauri commands stay thin: they construct or receive a service, call one method, and return the result. Commands such as `open_database`, `ping_sqlite_command`, and `get_database_health` delegate to `DatabaseService` instead of calling scattered free functions.

**Intentionally not included yet:**
- Traits, generics, or dependency-injection containers
- Global or long-lived app state holding a connection across commands
- Table browsing or schema inspection (later lessons)

As more operations arrive (list tables, inspect schema), they belong on `DatabaseService` or as helpers the service calls—not duplicated in `lib.rs`.

## Safe cell editing

MeridianDB's first write path edits a single preview cell through draft state, user confirmation, backend validation, and an explicit SQLite transaction. See [Safe Cell Editing](safe-cell-editing.md) for the full workflow, limitations, and key files.

## Models

**Location:** `src-tauri/src/models.rs`

**Responsible for:**
- Defining shared data shapes such as `AppInfo`, `DatabaseHealth`, `TableInfo`, and `ColumnInfo`
- Using `serde` so structs can cross the Tauri boundary as JSON-like values for TypeScript
- Keeping field names and types stable so the frontend can display results predictably

**Should not:**
- Run SQL or hold database connections (serializable result types only; connection-holding service code belongs in the database layer)
- Contain UI layout or styling logic
- Become a dumping ground for unrelated helpers—models describe data, not behavior

## Errors

**Location:** `src-tauri/src/errors.rs` on the backend; `src/errors.tsx` on the frontend for the matching TypeScript types

**Responsible for:**
- Defining `AppError`, an app-level enum for expected failures such as `Message(String)` and `NotImplemented(String)`
- Serializing errors back through Tauri when a command returns `Result<T, AppError>`
- Giving commands and services a shared vocabulary for user-facing failures instead of panicking or using `unwrap`

**Should not:**
- Represent programmer bugs (logic mistakes, index errors)—those are bugs to fix, not variants to display
- Contain UI formatting logic; the frontend decides how to present an error message to the user
- Swallow errors silently; callers return `Result` and let the command boundary decide what crosses into the UI

### Error flow

When something goes wrong in an expected way:

```text
UI → invoke("command") → Command returns Err(AppError) → serde → frontend catch → readable message
```

Commands that can fail return `Result`. On success, the UI gets the `Ok` value. On failure, `invoke` rejects and the frontend parses the serialized `AppError` variant and displays it.

## Planned Rust Layout

As MeridianDB grows, backend code will split along these lines:

```text
src-tauri/src/
  lib.rs       # Tauri setup and command registration
  main.rs      # App entrypoint
  models.rs    # Shared serializable structs
  errors.rs    # Friendly app-level errors
  database.rs  # DatabaseService, SQLite queries, and helpers
```

This map is intentionally small. It names where code goes before the forest gets dense—not every future feature, just enough for the next lessons to have a home.
