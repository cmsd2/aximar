//! Ordered delivery of MCP-made changes to the desktop window.
//!
//! The embedded MCP server reports notebook changes and lifecycle events
//! through callbacks that must not block.  They queue events on one channel,
//! and a single task delivers them in the order they were queued.  For a
//! notebook change the task takes the notebook's state when it delivers the
//! event, so the window always ends up showing the current state.

use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

use aximar_core::commands::CommandEffect;
use aximar_core::notebook::Notebook;
use aximar_core::registry::NotebookRegistry;

use crate::commands::notebook::emit_notebook_state;
use crate::state::SharedAppHandle;

/// A change to show in the window.
pub enum GuiEvent {
    NotebookChanged { notebook_id: String, effect: CommandEffect },
    Lifecycle { notebook_id: String, event: String },
}

pub type GuiEventSender = mpsc::UnboundedSender<GuiEvent>;

/// Where delivered events go: the window, or a recorder in tests.
pub trait GuiEmitter: Send + Sync {
    fn notebook_state(&self, notebook_id: &str, nb: &Notebook, effect: &CommandEffect);
    fn lifecycle(&self, notebook_id: &str, event: &str);
}

/// Emits to the desktop window through the app handle.
pub struct TauriGuiEmitter(pub SharedAppHandle);

impl GuiEmitter for TauriGuiEmitter {
    fn notebook_state(&self, notebook_id: &str, nb: &Notebook, effect: &CommandEffect) {
        if let Some(handle) = self.0.get() {
            emit_notebook_state(handle, notebook_id, nb, effect);
        }
    }

    fn lifecycle(&self, notebook_id: &str, event: &str) {
        if let Some(handle) = self.0.get() {
            let _ = handle.emit(
                "notebook-lifecycle",
                serde_json::json!({ "notebook_id": notebook_id, "event": event }),
            );
        }
    }
}

/// Start the delivery task.  It runs until every sender is dropped.
pub fn spawn_gui_event_task(
    registry: Arc<Mutex<NotebookRegistry>>,
    emitter: Arc<dyn GuiEmitter>,
) -> GuiEventSender {
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(deliver(rx, registry, emitter));
    tx
}

async fn deliver(
    mut rx: mpsc::UnboundedReceiver<GuiEvent>,
    registry: Arc<Mutex<NotebookRegistry>>,
    emitter: Arc<dyn GuiEmitter>,
) {
    while let Some(event) = rx.recv().await {
        match event {
            GuiEvent::NotebookChanged { notebook_id, effect } => {
                let notebook = match registry.lock().await.get(&notebook_id) {
                    Ok(ctx) => ctx.notebook.clone(),
                    // Closed since the change: nothing left to show
                    Err(_) => continue,
                };
                let nb = notebook.lock().await;
                emitter.notebook_state(&notebook_id, &nb, &effect);
            }
            GuiEvent::Lifecycle { notebook_id, event } => {
                emitter.lifecycle(&notebook_id, &event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aximar_core::commands::NotebookCommand;
    use aximar_core::notebook::CellType;
    use std::sync::Mutex as StdMutex;

    /// Records what would reach the window, in order.
    #[derive(Default)]
    struct Recorder(StdMutex<Vec<String>>);

    impl GuiEmitter for Recorder {
        fn notebook_state(&self, notebook_id: &str, nb: &Notebook, effect: &CommandEffect) {
            self.0.lock().unwrap().push(format!(
                "{notebook_id} {} {}",
                effect.effect_name(),
                nb.cells().len()
            ));
        }

        fn lifecycle(&self, notebook_id: &str, event: &str) {
            self.0.lock().unwrap().push(format!("{notebook_id} {event}"));
        }
    }

    #[tokio::test]
    async fn delivers_every_event_in_order() {
        let registry = Arc::new(Mutex::new(NotebookRegistry::new()));
        let id = registry.lock().await.create();
        let notebook = registry.lock().await.get(&id).unwrap().notebook.clone();
        let (tx, rx) = mpsc::unbounded_channel();

        tx.send(GuiEvent::Lifecycle { notebook_id: id.clone(), event: "created".into() })
            .unwrap();
        for _ in 0..50 {
            let effect = notebook
                .lock()
                .await
                .apply(NotebookCommand::AddCell {
                    cell_type: CellType::Code,
                    input: String::new(),
                    after_cell_id: None,
                    before_cell_id: None,
                })
                .unwrap();
            tx.send(GuiEvent::NotebookChanged { notebook_id: id.clone(), effect }).unwrap();
        }
        tx.send(GuiEvent::Lifecycle { notebook_id: id.clone(), event: "active_changed".into() })
            .unwrap();
        drop(tx);

        let recorder = Arc::new(Recorder::default());
        deliver(rx, registry, recorder.clone()).await;

        let seen = recorder.0.lock().unwrap().clone();
        assert_eq!(seen.len(), 52);
        assert_eq!(seen[0], format!("{id} created"));
        assert!(seen[1..51].iter().all(|s| s.starts_with(&format!("{id} cell_added"))));
        assert_eq!(seen[51], format!("{id} active_changed"));
    }

    #[tokio::test]
    async fn skips_changes_to_a_closed_notebook() {
        let registry = Arc::new(Mutex::new(NotebookRegistry::new()));
        let keep = registry.lock().await.create();
        let gone = registry.lock().await.create();
        registry.lock().await.close(&gone).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        for notebook_id in [gone.clone(), keep.clone()] {
            tx.send(GuiEvent::NotebookChanged {
                notebook_id,
                effect: CommandEffect::NoOp { reason: "test".into() },
            })
            .unwrap();
        }
        drop(tx);

        let recorder = Arc::new(Recorder::default());
        deliver(rx, registry, recorder.clone()).await;

        let seen = recorder.0.lock().unwrap().clone();
        assert_eq!(seen, vec![format!("{keep} no_op 1")]);
    }
}
