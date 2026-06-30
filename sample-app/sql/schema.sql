-- Generated SQL schema
-- Database: sample_app v1.0
-- Schema for the RustF Tasks sample application using SQLite.
-- Dialect: SQLite
-- DO NOT EDIT - Auto-generated from schema

-- Table: task_lists
CREATE TABLE task_lists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    description TEXT,
    title VARCHAR(150) NOT NULL,
    user_id INTEGER NOT NULL
);

-- Table: tasks
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    completed_at INTEGER,
    created_at INTEGER NOT NULL,
    details TEXT,
    is_completed INTEGER NOT NULL DEFAULT 0,
    list_id INTEGER NOT NULL,
    title VARCHAR(180) NOT NULL,
    user_id INTEGER NOT NULL
);

-- Table: users
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    display_name VARCHAR(120) NOT NULL,
    email VARCHAR(150) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL
);

-- Foreign keys
-- NOTE: SQLite cannot add FKs via ALTER TABLE; declare inline instead.
-- ALTER TABLE task_lists ADD FOREIGN KEY (user_id) REFERENCES users (id);
-- NOTE: SQLite cannot add FKs via ALTER TABLE; declare inline instead.
-- ALTER TABLE tasks ADD FOREIGN KEY (list_id) REFERENCES task_lists (id);
-- NOTE: SQLite cannot add FKs via ALTER TABLE; declare inline instead.
-- ALTER TABLE tasks ADD FOREIGN KEY (user_id) REFERENCES users (id);
