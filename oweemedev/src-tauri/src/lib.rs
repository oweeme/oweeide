use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tauri::Emitter;
use sqlx::{Column, Row};
use ssh2::Session;
use std::net::TcpStream;

// ── Database session registry ─────────────────────────────────────────────────
enum DbPool {
    MySql(sqlx::MySqlPool),
    Postgres(sqlx::PgPool),
    Sqlite(sqlx::SqlitePool),
}

struct DbSession { pool: Arc<DbPool> }

static DB_SESSIONS: Lazy<Mutex<HashMap<String, DbSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
pub struct DbQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

fn cell_mysql(row: &sqlx::mysql::MySqlRow, i: usize) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(i)  { return v; }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<u64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<i32>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<u32>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<i16>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<u16>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<i8>, _>(i)      { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<u8>, _>(i)      { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<f32>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<bool>, _>(i)    { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return v.map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| format!("<blob {} bytes>", b.len())));
    }
    None
}


fn cell_pg(row: &sqlx::postgres::PgRow, i: usize) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(i)  { return v; }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<i32>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<i16>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<f32>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<bool>, _>(i)    { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<uuid::Uuid>, _>(i)            { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return v.map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| format!("<bytea {} bytes>", b.len())));
    }
    None
}

fn cell_sqlite(row: &sqlx::sqlite::SqliteRow, i: usize) -> Option<String> {
    if let Ok(v) = row.try_get::<Option<String>, _>(i)  { return v; }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i)     { return v.map(|x| x.to_string()); }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return v.map(|b| String::from_utf8(b.clone()).unwrap_or_else(|_| format!("<blob {} bytes>", b.len())));
    }
    None
}

async fn run_query(pool: &DbPool, sql: &str) -> Result<DbQueryResult, String> {
    match pool {
        DbPool::MySql(p) => {
            // Use raw_sql (COM_QUERY / text protocol) to avoid MariaDB binary-protocol
            // type-mismatch hangs on FLOAT, TEXT, DATE and other non-trivial columns.
            let rows = sqlx::raw_sql(sql).fetch_all(p).await.map_err(|e| e.to_string())?;
            if rows.is_empty() { return Ok(DbQueryResult { columns: vec![], rows: vec![] }); }
            let columns = rows[0].columns().iter().map(|c| c.name().to_string()).collect::<Vec<_>>();
            let result_rows = rows.iter().map(|row| (0..columns.len()).map(|i| cell_mysql(row, i)).collect()).collect();
            Ok(DbQueryResult { columns, rows: result_rows })
        }
        DbPool::Postgres(p) => {
            let rows = sqlx::query(sql).fetch_all(p).await.map_err(|e| e.to_string())?;
            if rows.is_empty() { return Ok(DbQueryResult { columns: vec![], rows: vec![] }); }
            let columns = rows[0].columns().iter().map(|c| c.name().to_string()).collect::<Vec<_>>();
            let result_rows = rows.iter().map(|row| (0..columns.len()).map(|i| cell_pg(row, i)).collect()).collect();
            Ok(DbQueryResult { columns, rows: result_rows })
        }
        DbPool::Sqlite(p) => {
            let rows = sqlx::query(sql).fetch_all(p).await.map_err(|e| e.to_string())?;
            if rows.is_empty() { return Ok(DbQueryResult { columns: vec![], rows: vec![] }); }
            let columns = rows[0].columns().iter().map(|c| c.name().to_string()).collect::<Vec<_>>();
            let result_rows = rows.iter().map(|row| (0..columns.len()).map(|i| cell_sqlite(row, i)).collect()).collect();
            Ok(DbQueryResult { columns, rows: result_rows })
        }
    }
}

async fn list_tables_pool(pool: &DbPool) -> Result<Vec<String>, String> {
    match pool {
        // SHOW TABLES is order-of-magnitude faster than querying information_schema in MySQL
        DbPool::MySql(p) => {
            let rows = sqlx::query("SHOW TABLES").fetch_all(p).await.map_err(|e| e.to_string())?;
            let mut tables: Vec<String> = rows.iter()
                .filter_map(|row| row.try_get::<String, usize>(0).ok())
                .collect();
            tables.sort_unstable();
            Ok(tables)
        }
        DbPool::Postgres(_) | DbPool::Sqlite(_) => {
            let (sql, first_col): (&str, usize) = match pool {
                DbPool::Postgres(_) => ("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name", 0),
                _                   => ("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name", 0),
            };
            let result = run_query(pool, sql).await?;
            Ok(result.rows.into_iter().filter_map(|row| row.into_iter().nth(first_col).flatten()).collect())
        }
    }
}

#[tauri::command]
async fn db_connect(id: String, driver: String, url: String) -> Result<(), String> {
    let pool = match driver.as_str() {
        "mysql" => DbPool::MySql(
            sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(std::time::Duration::from_secs(15))
                .connect(&url).await.map_err(|e| e.to_string())?
        ),
        "postgres" => DbPool::Postgres(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(std::time::Duration::from_secs(15))
                .connect(&url).await.map_err(|e| e.to_string())?
        ),
        "sqlite" => DbPool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&url).await.map_err(|e| e.to_string())?
        ),
        _ => return Err(format!("Unknown driver: {}", driver)),
    };
    let old = DB_SESSIONS.lock().unwrap().remove(&id).map(|s| s.pool);
    if let Some(p) = old {
        match p.as_ref() {
            DbPool::MySql(x)    => x.close().await,
            DbPool::Postgres(x) => x.close().await,
            DbPool::Sqlite(x)   => x.close().await,
        }
    }
    DB_SESSIONS.lock().unwrap().insert(id, DbSession { pool: Arc::new(pool) });
    Ok(())
}

#[tauri::command]
async fn db_disconnect(id: String) -> Result<(), String> {
    let old = DB_SESSIONS.lock().unwrap().remove(&id).map(|s| s.pool);
    if let Some(p) = old {
        match p.as_ref() {
            DbPool::MySql(x)    => x.close().await,
            DbPool::Postgres(x) => x.close().await,
            DbPool::Sqlite(x)   => x.close().await,
        }
    }
    Ok(())
}

#[tauri::command]
async fn db_list_tables(id: String) -> Result<Vec<String>, String> {
    let pool = { DB_SESSIONS.lock().unwrap().get(&id).ok_or("Not connected")?.pool.clone() };
    list_tables_pool(&pool).await
}

#[tauri::command]
async fn db_query(id: String, sql: String) -> Result<DbQueryResult, String> {
    let pool = { DB_SESSIONS.lock().unwrap().get(&id).ok_or("Not connected")?.pool.clone() };
    run_query(&pool, &sql).await
}

#[tauri::command]
async fn db_execute(id: String, sql: String) -> Result<u64, String> {
    let pool = { DB_SESSIONS.lock().unwrap().get(&id).ok_or("Not connected")?.pool.clone() };
    match pool.as_ref() {
        DbPool::MySql(p)     => sqlx::raw_sql(&sql).execute(p).await.map(|r| r.rows_affected()).map_err(|e| e.to_string()),
        DbPool::Postgres(p)  => sqlx::query(&sql).execute(p).await.map(|r| r.rows_affected()).map_err(|e| e.to_string()),
        DbPool::Sqlite(p)    => sqlx::query(&sql).execute(p).await.map(|r| r.rows_affected()).map_err(|e| e.to_string()),
    }
}

#[tauri::command]
async fn db_describe_table(id: String, table: String, driver: String) -> Result<DbQueryResult, String> {
    let pool = { DB_SESSIONS.lock().unwrap().get(&id).ok_or("Not connected")?.pool.clone() };
    let sql = match driver.as_str() {
        "postgres" => format!(
            "SELECT column_name AS \"Field\", data_type AS \"Type\", is_nullable AS \"Null\", \
             column_default AS \"Default\", '' AS \"Key\", '' AS \"Extra\" \
             FROM information_schema.columns WHERE table_schema='public' AND table_name='{}' ORDER BY ordinal_position",
            table
        ),
        "sqlite" => format!("PRAGMA table_info(`{}`)", table),
        _ => format!("DESCRIBE `{}`", table),
    };
    run_query(&pool, &sql).await
}

// ── PTY session registry ──────────────────────────────────────────────────────
struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
}

static PTY_SESSIONS: Lazy<Mutex<HashMap<String, PtySession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
async fn pty_create(
    id: String,
    cwd: String,
    cols: u16,
    rows: u16,
    command: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
    let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;

    let cmd = match command.as_deref() {
        Some(cli) => {
            // Run specific CLI (claude, gemini, etc.) directly
            let mut c = CommandBuilder::new(cli);
            c.cwd(&cwd);
            c.env("TERM", "xterm-256color");
            c.env("COLORTERM", "truecolor");
            c
        }
        None => {
            let mut c = CommandBuilder::new("/bin/bash");
            c.args(&["--login", "-i"]);
            c.cwd(&cwd);
            c.env("TERM", "xterm-256color");
            c.env("COLORTERM", "truecolor");
            c
        }
    };

    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(writer));
    let master = Arc::new(Mutex::new(pair.master));

    PTY_SESSIONS.lock().unwrap().insert(id.clone(), PtySession { writer: Arc::clone(&writer), master: Arc::clone(&master) });

    let app = app_handle.clone();
    let event_id = format!("pty:{}", id);
    let exit_id  = format!("pty-exit:{}", id);
    let id_clone = id.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    app.emit(&event_id, chunk).ok();
                }
            }
        }
        PTY_SESSIONS.lock().unwrap().remove(&id_clone);
        app.emit(&exit_id, "").ok();
    });

    Ok(())
}

#[tauri::command]
fn pty_write(id: String, data: String) -> Result<(), String> {
    let sessions = PTY_SESSIONS.lock().unwrap();
    if let Some(s) = sessions.get(&id) {
        let mut w = s.writer.lock().unwrap();
        w.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        w.flush().ok();
    }
    Ok(())
}

#[tauri::command]
fn pty_resize(id: String, cols: u16, rows: u16) -> Result<(), String> {
    let sessions = PTY_SESSIONS.lock().unwrap();
    if let Some(s) = sessions.get(&id) {
        s.master.lock().unwrap()
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pty_kill(id: String) {
    PTY_SESSIONS.lock().unwrap().remove(&id);
}

#[derive(Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[tauri::command]
fn list_dir(path: String, show_hidden: Option<bool>) -> Result<Vec<FileEntry>, String> {
    let show_hidden = show_hidden.unwrap_or(false);
    let dir = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut entries: Vec<FileEntry> = dir
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            let path = e.path().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Some(FileEntry { name, path, is_dir })
        })
        .collect();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(entries)
}

#[tauri::command]
fn open_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_file(path: String) -> Result<(), String> {
    if Path::new(&path).exists() {
        return Err(format!("File already exists: {}", path));
    }
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, "").map_err(|e| e.to_string())
}

#[tauri::command]
fn create_dir_cmd(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn move_to_trash(path: String) -> Result<(), String> {
    let src = Path::new(&path);
    if !src.exists() {
        return Err(format!("Path not found: {}", path));
    }

    // Build trash dirs: ~/.local/share/Trash/{files,info}
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let trash_files = format!("{}/.local/share/Trash/files", home);
    let trash_info  = format!("{}/.local/share/Trash/info",  home);
    fs::create_dir_all(&trash_files).map_err(|e| e.to_string())?;
    fs::create_dir_all(&trash_info).map_err(|e| e.to_string())?;

    let name = src.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Avoid collisions: append number if name already exists in trash
    let mut dest_name = name.clone();
    let mut counter = 1u32;
    while Path::new(&format!("{}/{}", trash_files, dest_name)).exists() {
        let stem = Path::new(&name).file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        let ext = Path::new(&name).extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        dest_name = format!("{}_{}{}", stem, counter, ext);
        counter += 1;
    }

    let dest = format!("{}/{}", trash_files, dest_name);
    let info = format!("{}/{}.trashinfo", trash_info, dest_name);

    // Write .trashinfo (freedesktop spec)
    let abs_path = src.canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(path.clone());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let date = format_trash_date(now);
    let info_content = format!("[Trash Info]\nPath={}\nDeletionDate={}\n", abs_path, date);
    fs::write(&info, info_content).map_err(|e| e.to_string())?;

    // Move to trash
    if src.is_dir() {
        // Try rename first (same filesystem), fallback to copy+delete
        if fs::rename(&path, &dest).is_err() {
            copy_dir_recursive(&path, &dest)?;
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        }
    } else {
        if fs::rename(&path, &dest).is_err() {
            fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn format_trash_date(secs: u64) -> String {
    // Format: 2024-01-15T14:30:00
    let s = secs;
    let secs_per_day = 86400u64;
    let days = s / secs_per_day;
    let time_s = s % secs_per_day;
    // Simple calculation (approximate, ignores leap years perfectly)
    let mut year = 1970u64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        year += 1;
    }
    let months = [31u64,28,31,30,31,30,31,31,30,31,30,31];
    let mut month = 0usize;
    let mut day = remaining_days;
    for (i, &m) in months.iter().enumerate() {
        if day < m { month = i; break; }
        day -= m;
    }
    let h = time_s / 3600;
    let m = (time_s % 3600) / 60;
    let s2 = time_s % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", year, month+1, day+1, h, m, s2)
}

fn copy_dir_recursive(src: &str, dst: &str) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let src_path = format!("{}/{}", src, name);
        let dst_path = format!("{}/{}", dst, name);
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn delete_entry(path: String, force: bool) -> Result<(), String> {
    let p = Path::new(&path);
    if p.is_dir() {
        let count = fs::read_dir(&path).map(|d| d.count()).unwrap_or(0);
        if count > 0 && !force {
            return Err(format!("NONEMPTY:{}", count));
        }
        fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn rename_entry(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_image_base64(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(base64_encode(&bytes))
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[(n >> 18) & 63] as char);
        result.push(CHARS[(n >> 12) & 63] as char);
        result.push(if chunk.len() > 1 { CHARS[(n >> 6) & 63] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[n & 63] as char } else { '=' });
    }
    result
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
async fn call_ai(
    provider: String,
    api_key: String,
    model: String,
    system: String,
    messages: Vec<AiMessage>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    match provider.as_str() {
        "claude" => {
            let msgs: Vec<Value> = messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content
            })).collect();

            let body = json!({
                "model": model,
                "max_tokens": 4096,
                "system": system,
                "messages": msgs
            });

            let res = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !res.status().is_success() {
                let err: Value = res.json().await.unwrap_or(json!({"error":{"message":"Unknown error"}}));
                return Err(err["error"]["message"].as_str().unwrap_or("API error").to_string());
            }

            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            Ok(data["content"][0]["text"].as_str().unwrap_or("").to_string())
        }
        "openai" => {
            let mut msgs: Vec<Value> = vec![json!({"role": "system", "content": system})];
            for m in &messages {
                msgs.push(json!({"role": m.role, "content": m.content}));
            }

            let body = json!({
                "model": model,
                "messages": msgs,
                "max_tokens": 4096
            });

            let res = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !res.status().is_success() {
                let err: Value = res.json().await.unwrap_or(json!({"error":{"message":"Unknown error"}}));
                return Err(err["error"]["message"].as_str().unwrap_or("API error").to_string());
            }

            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            Ok(data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
        }
        "gemini" => {
            // system_instruction not supported in all versions — prepend as user/model pair
            let mut msgs: Vec<Value> = vec![];
            if !system.is_empty() {
                msgs.push(json!({ "role": "user",  "parts": [{ "text": system }] }));
                msgs.push(json!({ "role": "model", "parts": [{ "text": "Understood. I will follow those instructions." }] }));
            }
            for m in &messages {
                let role = if m.role == "assistant" { "model" } else { "user" };
                msgs.push(json!({ "role": role, "parts": [{ "text": m.content }] }));
            }
            let body = json!({
                "contents": msgs,
                "generationConfig": { "maxOutputTokens": 4096 }
            });
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, api_key
            );
            let res = client.post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send().await.map_err(|e| e.to_string())?;
            if !res.status().is_success() {
                let err: Value = res.json().await.unwrap_or(json!({"error":{"message":"Unknown"}}));
                return Err(err["error"]["message"].as_str().unwrap_or("Gemini API error").to_string());
            }
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            Ok(data["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string())
        }
        "ollama" => {
            // Ollama runs locally at http://localhost:11434
            let mut msgs: Vec<Value> = vec![json!({"role":"system","content":system})];
            for m in &messages {
                msgs.push(json!({"role": m.role, "content": m.content}));
            }
            let body = json!({ "model": model, "messages": msgs, "stream": false });
            let res = client.post("http://localhost:11434/api/chat")
                .header("content-type", "application/json")
                .json(&body)
                .send().await
                .map_err(|e| format!("Ollama not running? Start with: ollama serve\n{}", e))?;
            if !res.status().is_success() {
                let txt = res.text().await.unwrap_or_default();
                return Err(format!("Ollama error: {}", txt));
            }
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            Ok(data["message"]["content"].as_str().unwrap_or("").to_string())
        }
        "deepseek" => {
            // DeepSeek uses OpenAI-compatible API
            let mut msgs: Vec<Value> = vec![json!({"role": "system", "content": system})];
            for m in &messages {
                msgs.push(json!({"role": m.role, "content": m.content}));
            }
            let body = json!({
                "model": model,
                "messages": msgs,
                "max_tokens": 4096
            });
            let res = client
                .post("https://api.deepseek.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&body)
                .send().await
                .map_err(|e| e.to_string())?;
            if !res.status().is_success() {
                let err: Value = res.json().await.unwrap_or(json!({"error":{"message":"Unknown error"}}));
                return Err(err["error"]["message"].as_str().unwrap_or("DeepSeek error").to_string());
            }
            let data: Value = res.json().await.map_err(|e| e.to_string())?;
            Ok(data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
        }
        _ => Err(format!("Unknown provider: {}", provider))
    }
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|_| "Ollama no está corriendo. Inicia con: ollama serve".to_string())?;
    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let models = data["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

#[derive(Serialize)]
pub struct SearchHit {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[tauri::command]
fn search_in_files(
    root_path: String,
    query: String,
    case_sensitive: bool,
    use_regex: bool,
    max_results: usize,
) -> Result<Vec<SearchHit>, String> {
    let mut hits = Vec::new();
    let skip_dirs = ["node_modules", ".git", "target", "dist", ".cache", "__pycache__"];
    let text_exts = ["js","ts","tsx","jsx","vue","php","go","py","rs","html","htm","css","scss",
                     "sass","json","yaml","yml","toml","xml","md","txt","sql","sh","bash","env",
                     "c","cpp","h","hpp","java","rb","swift","kt","cs"];

    fn walk(
        dir: &str, query: &str, case_sensitive: bool, skip_dirs: &[&str],
        text_exts: &[&str], hits: &mut Vec<SearchHit>, max: usize,
    ) {
        if hits.len() >= max { return; }
        let entries = match fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
        for entry in entries.flatten() {
            if hits.len() >= max { return; }
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();
            if path.is_dir() {
                if name.starts_with('.') { continue; } // skip .git, .node_modules, etc.
                if skip_dirs.contains(&name.as_str()) { continue; }
                walk(&path_str, query, case_sensitive, skip_dirs, text_exts, hits, max);
            } else {
                let ext = path.extension().map(|e| e.to_str().unwrap_or("")).unwrap_or("");
                if !text_exts.contains(&ext) { continue; }
                if let Ok(content) = fs::read_to_string(&path_str) {
                    for (i, line) in content.lines().enumerate() {
                        if hits.len() >= max { return; }
                        let matched = if case_sensitive {
                            line.contains(query)
                        } else {
                            line.to_lowercase().contains(&query.to_lowercase())
                        };
                        if matched {
                            hits.push(SearchHit {
                                file: path_str.clone(),
                                line: i + 1,
                                text: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Ignore use_regex for now (simple contains search)
    let _ = use_regex;
    walk(&root_path, &query, case_sensitive, &skip_dirs, &text_exts, &mut hits, max_results);
    Ok(hits)
}

#[tauri::command]
fn open_in_file_manager(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string())
}

#[tauri::command]
fn path_join(base: String, segment: String) -> String {
    Path::new(&base).join(&segment).to_string_lossy().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            list_dir,
            get_home_dir,
            path_join,
            create_file,
            create_dir_cmd,
            delete_entry,
            rename_entry,
            read_image_base64,
            call_ai,
            list_ollama_models,
            pty_create,
            pty_write,
            pty_resize,
            pty_kill,
            move_to_trash,
            search_in_files,
            open_in_file_manager,
            db_connect,
            db_disconnect,
            db_list_tables,
            db_query,
            db_execute,
            db_describe_table,
            remote_connect,
            remote_disconnect,
            remote_list_dir,
            remote_read_file,
            remote_write_file,
            remote_delete,
            remote_mkdir,
            remote_rename,
            lint_file,
            http_request,
            lsp_start,
            lsp_complete,
            lsp_diagnostics,
            lsp_notify_open,
            lsp_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ─── Remote (FTP / SFTP) ─────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct RemoteEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

struct SftpSession {
    session: Mutex<Session>,
    #[allow(dead_code)]
    root: String,
}

struct FtpSession {
    stream: Mutex<suppaftp::FtpStream>,
    #[allow(dead_code)]
    root: String,
}

enum RemoteConn {
    Sftp(SftpSession),
    Ftp(FtpSession),
}

static REMOTE: Lazy<Mutex<HashMap<String, Arc<RemoteConn>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn parse_ftp_list(line: &str, parent: &str) -> Option<RemoteEntry> {
    let cols: Vec<&str> = line.split_ascii_whitespace().collect();
    if cols.len() < 9 { return None; }
    let is_dir = cols[0].starts_with('d');
    let size = cols[4].parse::<u64>().unwrap_or(0);
    let name = cols[8..].join(" ");
    if name == "." || name == ".." { return None; }
    let path = if parent == "/" { format!("/{}", name) }
               else { format!("{}/{}", parent.trim_end_matches('/'), name) };
    Some(RemoteEntry { name, path, is_dir, size })
}

#[tauri::command]
async fn remote_connect(
    id: String, protocol: String, host: String, port: u16,
    user: String, pass: String, root: String,
) -> Result<(), String> {
    REMOTE.lock().unwrap().remove(&id);
    tokio::task::spawn_blocking(move || {
        let conn: RemoteConn = match protocol.as_str() {
            "sftp" => {
                let tcp = TcpStream::connect(format!("{}:{}", host, port))
                    .map_err(|e| format!("TCP: {}", e))?;
                let mut sess = Session::new().map_err(|e| e.to_string())?;
                sess.set_tcp_stream(tcp);
                sess.handshake().map_err(|e| format!("Handshake: {}", e))?;
                if sess.userauth_agent(&user).is_err() {
                    if !pass.is_empty() {
                        sess.userauth_password(&user, &pass)
                            .map_err(|e| format!("Auth: {}", e))?;
                    } else {
                        return Err("Auth failed: no password and no SSH agent key loaded".into());
                    }
                }
                if !sess.authenticated() { return Err("Authentication failed".into()); }
                RemoteConn::Sftp(SftpSession { session: Mutex::new(sess), root })
            }
            "ftp" => {
                let mut stream = suppaftp::FtpStream::connect(format!("{}:{}", host, port))
                    .map_err(|e| format!("FTP connect: {}", e))?;
                stream.login(&user, &pass).map_err(|e| format!("FTP login: {}", e))?;
                RemoteConn::Ftp(FtpSession { stream: Mutex::new(stream), root })
            }
            p => return Err(format!("Unknown protocol: {}", p)),
        };
        REMOTE.lock().unwrap().insert(id, Arc::new(conn));
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_disconnect(id: String) -> Result<(), String> {
    REMOTE.lock().unwrap().remove(&id);
    Ok(())
}

#[tauri::command]
async fn remote_list_dir(id: String, path: String) -> Result<Vec<RemoteEntry>, String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        let mut entries = match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                sftp.readdir(Path::new(&path)).map_err(|e| e.to_string())?
                    .into_iter().filter_map(|(p, stat)| {
                        let name = p.file_name()?.to_string_lossy().to_string();
                        if name == "." || name == ".." { return None; }
                        Some(RemoteEntry {
                            is_dir: stat.is_dir(), size: stat.size.unwrap_or(0),
                            path: p.to_string_lossy().to_string(), name,
                        })
                    }).collect::<Vec<_>>()
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                ftp.list(Some(&path)).map_err(|e| e.to_string())?
                   .iter().filter_map(|l| parse_ftp_list(l, &path)).collect()
            }
        };
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_read_file(id: String, path: String) -> Result<String, String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                let mut file = sftp.open(Path::new(&path)).map_err(|e| e.to_string())?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(&buf).to_string())
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                let cursor = ftp.retr_as_buffer(&path).map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(cursor.get_ref()).to_string())
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_write_file(id: String, path: String, content: String) -> Result<(), String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                let mut file = sftp.create(Path::new(&path)).map_err(|e| e.to_string())?;
                file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
                Ok(())
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                ftp.put_file(&path, &mut std::io::Cursor::new(content.as_bytes()))
                   .map_err(|e| e.to_string())?;
                Ok(())
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_delete(id: String, path: String, is_dir: bool) -> Result<(), String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                if is_dir { sftp.rmdir(Path::new(&path)).map_err(|e| e.to_string()) }
                else { sftp.unlink(Path::new(&path)).map_err(|e| e.to_string()) }
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                if is_dir { ftp.rmdir(&path).map_err(|e| e.to_string()) }
                else { ftp.rm(&path).map_err(|e| e.to_string()) }
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_mkdir(id: String, path: String) -> Result<(), String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                sftp.mkdir(Path::new(&path), 0o755).map_err(|e| e.to_string())
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                ftp.mkdir(&path).map_err(|e| e.to_string())
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_rename(id: String, old_path: String, new_path: String) -> Result<(), String> {
    let conn = REMOTE.lock().unwrap().get(&id).cloned()
        .ok_or_else(|| "Not connected".to_string())?;
    tokio::task::spawn_blocking(move || {
        match &*conn {
            RemoteConn::Sftp(s) => {
                let sess = s.session.lock().unwrap();
                let sftp = sess.sftp().map_err(|e| e.to_string())?;
                sftp.rename(Path::new(&old_path), Path::new(&new_path), None)
                    .map_err(|e| e.to_string())
            }
            RemoteConn::Ftp(f) => {
                let mut ftp = f.stream.lock().unwrap();
                ftp.rename(&old_path, &new_path).map_err(|e| e.to_string())
            }
        }
    }).await.map_err(|e| e.to_string())?
}

// ─── Linting ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Default, Clone)]
pub struct LintError {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub message: String,
    pub severity: String,
}

#[tauri::command]
async fn lint_file(language: String, content: String) -> Vec<LintError> {
    tokio::task::spawn_blocking(move || run_lint(&language, &content))
        .await
        .unwrap_or_default()
}

fn run_lint(language: &str, content: &str) -> Vec<LintError> {
    match language {
        "python"     => lint_python(content),
        "javascript" | "typescript" => lint_js(content, language),
        "php"        => lint_php(content),
        "rust"       => lint_rust(content),
        _            => vec![],
    }
}

fn tmp_path(ext: &str) -> std::path::PathBuf {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("_owee_lint_{}.{}", ms, ext))
}

fn lint_python(content: &str) -> Vec<LintError> {
    use std::process::{Command, Stdio};
    let script = concat!(
        "import ast,sys\n",
        "src=sys.stdin.buffer.read().decode('utf-8','replace')\n",
        "try:\n",
        "    compile(src,'<stdin>','exec')\n",
        "except SyntaxError as e:\n",
        "    print(f'ERR:{e.lineno or 1}:{e.offset or 0}:{e.msg}')\n",
        "    sys.exit(1)\n"
    );
    let mut child = match Command::new("python3")
        .args(["-c", script])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn() { Ok(c) => c, Err(_) => return vec![] };

    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(content.as_bytes()); }
    let out = match child.wait_with_output() { Ok(o) => o, Err(_) => return vec![] };
    if out.status.success() { return vec![]; }

    String::from_utf8_lossy(&out.stdout).lines()
        .filter_map(|l| l.strip_prefix("ERR:"))
        .filter_map(|r| {
            let p: Vec<&str> = r.splitn(3, ':').collect();
            if p.len() == 3 {
                let ln = p[0].parse::<usize>().unwrap_or(1).saturating_sub(1);
                let col = p[1].parse::<usize>().unwrap_or(1).saturating_sub(1);
                Some(LintError { line: ln, col, end_line: ln, end_col: 1000,
                    message: p[2].to_string(), severity: "error".to_string() })
            } else { None }
        }).collect()
}

fn lint_js(content: &str, language: &str) -> Vec<LintError> {
    let ext = if language == "typescript" { "ts" } else { "js" };
    let tmp = tmp_path(ext);
    if fs::write(&tmp, content).is_err() { return vec![]; }
    let out = std::process::Command::new("node").arg("--check").arg(&tmp).output();
    let _ = fs::remove_file(&tmp);
    let out = match out { Ok(o) if !o.status.success() => o, _ => return vec![] };
    let stderr = String::from_utf8_lossy(&out.stderr);
    parse_node_errors(&stderr)
}

fn parse_node_errors(stderr: &str) -> Vec<LintError> {
    let lines: Vec<&str> = stderr.lines().collect();
    let mut errors = vec![];
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("SyntaxError:") || t.starts_with("ReferenceError:") || t.starts_with("TypeError:") {
            let msg = t.to_string();
            // Scan backwards for location line "path:line" or "path:line:col"
            for prev in lines[..i].iter().rev() {
                if prev.starts_with('/') || prev.starts_with('.') {
                    let parts: Vec<&str> = prev.splitn(4, ':').collect();
                    if parts.len() >= 2 {
                        if let Ok(ln) = parts[1].trim().parse::<usize>() {
                            let col = parts.get(2).and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(1);
                            errors.push(LintError {
                                line: ln.saturating_sub(1), col: col.saturating_sub(1),
                                end_line: ln.saturating_sub(1), end_col: 1000,
                                message: msg.clone(), severity: "error".to_string(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }
    errors
}

fn lint_php(content: &str) -> Vec<LintError> {
    let tmp = tmp_path("php");
    if fs::write(&tmp, content).is_err() { return vec![]; }
    let out = std::process::Command::new("php")
        .args(["-l", "-d", "display_errors=1", tmp.to_str().unwrap_or("")])
        .output();
    let _ = fs::remove_file(&tmp);
    let out = match out { Ok(o) => o, Err(_) => return vec![] };
    let combined = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    combined.lines()
        .filter(|l| (l.contains("Parse error") || l.contains("Fatal error") || l.contains("Warning:"))
            && l.contains(" on line "))
        .filter_map(|l| {
            let n = l.split(" on line ").last()?.trim().parse::<usize>().ok()?;
            let msg = l.split(" in ").next().unwrap_or(l).trim().to_string();
            let sev = if l.contains("Warning:") { "warning" } else { "error" };
            Some(LintError { line: n.saturating_sub(1), col: 0, end_line: n.saturating_sub(1),
                end_col: 1000, message: msg, severity: sev.to_string() })
        }).collect()
}

fn lint_rust(content: &str) -> Vec<LintError> {
    let tmp = tmp_path("rs");
    if fs::write(&tmp, content).is_err() { return vec![]; }
    let out = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--error-format=json", "--emit=metadata", "-o", "/dev/null"])
        .arg(&tmp).output();
    let _ = fs::remove_file(&tmp);
    let _ = fs::remove_file(std::env::temp_dir().join("librust_check.rmeta"));
    let out = match out { Ok(o) => o, Err(_) => return vec![] };
    String::from_utf8_lossy(&out.stderr).lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            let level = v["level"].as_str()?.to_string();
            if level != "error" && level != "warning" { return None; }
            let message = v["message"].as_str().unwrap_or("").to_string();
            let span = v["spans"].as_array()?.first()?;
            let ln = span["line_start"].as_u64().unwrap_or(1) as usize;
            let col = span["column_start"].as_u64().unwrap_or(1) as usize;
            let end_ln = span["line_end"].as_u64().unwrap_or(ln as u64) as usize;
            let end_col = span["column_end"].as_u64().unwrap_or(col as u64) as usize;
            Some(LintError { line: ln.saturating_sub(1), col: col.saturating_sub(1),
                end_line: end_ln.saturating_sub(1), end_col, message, severity: level })
        }).collect()
}

// ─── HTTP Client ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub time_ms: u64,
}

#[tauri::command]
async fn http_request(
    method: String,
    url: String,
    headers: std::collections::HashMap<String, String>,
    body: Option<String>,
) -> Result<HttpResponse, String> {
    use reqwest::Client;
    let client = Client::builder()
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| e.to_string())?;

    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req = client.request(m, &url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    if let Some(b) = body {
        if !b.is_empty() { req = req.body(b); }
    }

    let start = std::time::Instant::now();
    let res = req.send().await.map_err(|e| e.to_string())?;
    let time_ms = start.elapsed().as_millis() as u64;

    let status = res.status().as_u16();
    let status_text = res.status().canonical_reason().unwrap_or("").to_string();
    let mut hdrs = std::collections::HashMap::new();
    for (k, v) in res.headers() {
        hdrs.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
    }
    let body_bytes = res.bytes().await.map_err(|e| e.to_string())?;
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    Ok(HttpResponse { status, status_text, headers: hdrs, body: body_str, time_ms })
}

// ─── LSP Bridge ───────────────────────────────────────────────────────────────

use std::io::{BufRead as LspBufRead, BufReader as LspBufReader};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrd};
use std::sync::mpsc;

#[derive(Serialize, Clone, Debug, Default)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct LspDiagnosticItem {
    pub message: String,
    pub severity: u32,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

struct LspSession {
    stdin: Mutex<std::process::ChildStdin>,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, mpsc::Sender<Value>>>,
    diagnostics: Mutex<HashMap<String, Vec<LspDiagnosticItem>>>,
    initialized: std::sync::atomic::AtomicBool,
}

static LSP_SESSIONS: Lazy<Mutex<HashMap<String, Arc<LspSession>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn lsp_binary(language: &str) -> Option<&'static str> {
    match language {
        "typescript" | "javascript" => Some("typescript-language-server"),
        "vue"    => Some("vue-language-server"),
        "go"     => Some("gopls"),
        "rust"   => Some("rust-analyzer"),
        "php"    => Some("phpactor"),
        "python" => Some("pylsp"),
        _ => None,
    }
}

fn lsp_args(language: &str) -> Vec<&'static str> {
    match language {
        "typescript" | "javascript" | "vue" => vec!["--stdio"],
        "php" => vec!["language-server"],
        _ => vec![],
    }
}

fn lsp_lang_id(language: &str) -> &'static str {
    match language {
        "vue" => "vue",
        "typescript" => "typescript",
        "javascript" => "javascript",
        "go" => "go", "rust" => "rust",
        "php" => "php", "python" => "python",
        _ => "plaintext",
    }
}

fn lsp_write_msg(stdin: &mut std::process::ChildStdin, msg: &str) -> Result<(), String> {
    let frame = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);
    stdin.write_all(frame.as_bytes()).map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn lsp_read_msg(reader: &mut LspBufReader<std::process::ChildStdout>) -> Option<Value> {
    let mut content_len = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 { return None; }
        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if line.is_empty() { break; }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_len = rest.trim().parse().unwrap_or(0);
        }
    }
    if content_len == 0 { return None; }
    let mut body = vec![0u8; content_len];
    use std::io::Read as IoR;
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn lsp_start_reader(mut reader: LspBufReader<std::process::ChildStdout>, session: Arc<LspSession>) {
    std::thread::spawn(move || {
        while let Some(msg) = lsp_read_msg(&mut reader) {
            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                let tx = session.pending.lock().unwrap().remove(&id);
                if let Some(tx) = tx { let _ = tx.send(msg); }
            } else if msg.get("method").and_then(|v| v.as_str()) == Some("textDocument/publishDiagnostics") {
                if let Some(params) = msg.get("params") {
                    let uri = params["uri"].as_str().unwrap_or("").to_string();
                    let diags: Vec<LspDiagnosticItem> = params["diagnostics"]
                        .as_array().unwrap_or(&vec![]).iter()
                        .filter_map(|d| {
                            let range = d.get("range")?;
                            Some(LspDiagnosticItem {
                                message:    d["message"].as_str().unwrap_or("").to_string(),
                                severity:   d["severity"].as_u64().unwrap_or(1) as u32,
                                start_line: range["start"]["line"].as_u64().unwrap_or(0) as u32,
                                start_col:  range["start"]["character"].as_u64().unwrap_or(0) as u32,
                                end_line:   range["end"]["line"].as_u64().unwrap_or(0) as u32,
                                end_col:    range["end"]["character"].as_u64().unwrap_or(0) as u32,
                            })
                        }).collect();
                    session.diagnostics.lock().unwrap().insert(uri, diags);
                }
            }
        }
    });
}

#[tauri::command]
async fn lsp_start(language: String, root_path: String) -> Result<bool, String> {
    {
        let sessions = LSP_SESSIONS.lock().unwrap();
        if sessions.contains_key(&language) { return Ok(true); }
    }
    let binary = lsp_binary(&language).ok_or_else(|| format!("No LSP para {}", language))?;
    let args   = lsp_args(&language);

    let mut child = std::process::Command::new(binary)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("{binary} no encontrado: {e}. Instálalo primero."))?;

    let stdin  = child.stdin.take().ok_or("no stdin")?;
    let stdout = LspBufReader::new(child.stdout.take().ok_or("no stdout")?);

    let session = Arc::new(LspSession {
        stdin:       Mutex::new(stdin),
        next_id:     AtomicU64::new(1),
        pending:     Mutex::new(HashMap::new()),
        diagnostics: Mutex::new(HashMap::new()),
        initialized: std::sync::atomic::AtomicBool::new(false),
    });

    lsp_start_reader(stdout, session.clone());

    // Send initialize
    let req_id   = session.next_id.fetch_add(1, AtomicOrd::SeqCst);
    let root_uri = format!("file://{}", root_path);
    let init = json!({
        "jsonrpc": "2.0", "id": req_id, "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": { "completionItem": { "snippetSupport": false, "documentationFormat": ["plaintext"] } },
                    "publishDiagnostics": { "relatedInformation": false }
                }
            }
        }
    }).to_string();

    let (tx, rx) = mpsc::channel::<Value>();
    session.pending.lock().unwrap().insert(req_id, tx);
    { let mut stdin = session.stdin.lock().unwrap(); lsp_write_msg(&mut stdin, &init)?; }

    // Wait for initialize response (blocking, called in spawn_blocking context is fine)
    tokio::task::spawn_blocking(move || {
        let _ = rx.recv_timeout(std::time::Duration::from_secs(10));
    }).await.ok();

    // Send initialized notification
    let notif = json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }).to_string();
    { let mut stdin = session.stdin.lock().unwrap(); lsp_write_msg(&mut stdin, &notif)?; }
    session.initialized.store(true, AtomicOrd::SeqCst);

    LSP_SESSIONS.lock().unwrap().insert(language, session);
    std::mem::forget(child); // Keep process alive
    Ok(true)
}

#[tauri::command]
async fn lsp_complete(
    language: String,
    file_path: String,
    content: String,
    line: u32,
    col: u32,
) -> Result<Vec<LspCompletionItem>, String> {
    let session = LSP_SESSIONS.lock().unwrap().get(&language).cloned();
    let session = match session { Some(s) => s, None => return Ok(vec![]) };
    if !session.initialized.load(AtomicOrd::SeqCst) { return Ok(vec![]); }

    let uri     = format!("file://{}", file_path);
    let lang_id = lsp_lang_id(&language);

    // Sync document state
    static DOC_VERSION: AtomicU64 = AtomicU64::new(1);
    let ver = DOC_VERSION.fetch_add(1, AtomicOrd::SeqCst);
    let did_change = json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": uri, "version": ver, "languageId": lang_id },
            "contentChanges": [{ "text": content }]
        }
    }).to_string();

    let req_id = session.next_id.fetch_add(1, AtomicOrd::SeqCst);
    let comp_req = json!({
        "jsonrpc": "2.0", "id": req_id, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": col },
            "context": { "triggerKind": 1 }
        }
    }).to_string();

    let (tx, rx) = mpsc::channel::<Value>();
    session.pending.lock().unwrap().insert(req_id, tx);
    {
        let mut stdin = session.stdin.lock().unwrap();
        lsp_write_msg(&mut stdin, &did_change)?;
        lsp_write_msg(&mut stdin, &comp_req)?;
    }

    let response = tokio::task::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_millis(1500)).ok()
    }).await.ok().flatten();

    let Some(resp) = response else { return Ok(vec![]); };

    let items_arr = if let Some(arr) = resp["result"].as_array() {
        arr.to_vec()
    } else if let Some(arr) = resp["result"]["items"].as_array() {
        arr.to_vec()
    } else {
        return Ok(vec![]);
    };

    Ok(items_arr.iter().take(60).filter_map(|item| {
        let label = item["label"].as_str()?.to_string();
        Some(LspCompletionItem {
            label,
            kind: item["kind"].as_u64().map(|k| k as u32),
            detail: item["detail"].as_str().map(|s| s.to_string()),
            insert_text: item["insertText"].as_str().map(|s| s.to_string()),
        })
    }).collect())
}

#[tauri::command]
async fn lsp_diagnostics(language: String, file_path: String) -> Result<Vec<LspDiagnosticItem>, String> {
    let session = LSP_SESSIONS.lock().unwrap().get(&language).cloned();
    let session = match session { Some(s) => s, None => return Ok(vec![]) };
    let uri = format!("file://{}", file_path);
    let result = session.diagnostics.lock().unwrap().get(&uri).cloned().unwrap_or_default();
    Ok(result)
}

#[tauri::command]
async fn lsp_notify_open(language: String, file_path: String, content: String) -> Result<(), String> {
    let session = LSP_SESSIONS.lock().unwrap().get(&language).cloned();
    let session = match session { Some(s) => s, None => return Ok(()) };
    if !session.initialized.load(AtomicOrd::SeqCst) { return Ok(()); }
    let uri     = format!("file://{}", file_path);
    let lang_id = lsp_lang_id(&language);
    let msg = json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": { "textDocument": { "uri": uri, "languageId": lang_id, "version": 1, "text": content } }
    }).to_string();
    let mut stdin = session.stdin.lock().unwrap();
    lsp_write_msg(&mut stdin, &msg)
}

#[tauri::command]
async fn lsp_stop(language: String) -> Result<(), String> {
    let session = LSP_SESSIONS.lock().unwrap().remove(&language);
    if let Some(s) = session {
        if let Ok(mut stdin) = s.stdin.lock() {
            let shutdown = json!({ "jsonrpc": "2.0", "id": 999999, "method": "shutdown", "params": null }).to_string();
            let _ = lsp_write_msg(&mut stdin, &shutdown);
            let exit = json!({ "jsonrpc": "2.0", "method": "exit", "params": null }).to_string();
            let _ = lsp_write_msg(&mut stdin, &exit);
        }
    }
    Ok(())
}
