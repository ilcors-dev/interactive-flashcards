-- V3__soft_delete_sessions.sql
ALTER TABLE sessions ADD COLUMN deleted_at INTEGER;
