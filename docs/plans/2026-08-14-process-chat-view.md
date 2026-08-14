# Process Chat View Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Enter open a dedicated process chat view that supports follow-up questions and returns to the live process list with Esc.

**Architecture:** Keep the process monitor as the default view and add a small in-memory chat mode to the existing `App`. The selected process remains the evidence snapshot for each request, while completed question/answer pairs are retained only for display during the current session.

**Tech Stack:** Rust, Tokio, Crossterm, Ratatui, existing local OpenAI-compatible streaming engine.

---

### Task 1: Add chat view state and transitions

**Files:** Modify `src/app.rs`

Add monitor/chat mode state, chat turn storage, and transitions: Enter or `a` opens chat; Enter submits a question in chat; Esc returns to the monitor; `q` remains typeable in chat.

### Task 2: Render the dedicated chat view

**Files:** Modify `src/app.rs`

Render chat history, the streaming answer, current input, and keyboard instructions in a full-screen layout. Keep the existing process table rendering unchanged in monitor mode.

### Task 3: Rebuild and verify

**Files:** None

Run `cargo fmt`, `cargo test`, and `cargo build` so the executable launched from `target/debug/whytop` contains the changes.
