//! Rhai scripting integration via soushi.
//!
//! Loads user scripts from `~/.config/shashin/scripts/*.rhai` and exposes
//! image viewer functions: `shashin.open`, `shashin.zoom`, `shashin.rotate`,
//! `shashin.export`.

use std::collections::HashMap;
use std::path::PathBuf;

use soushi::ScriptEngine;

/// Script event hooks that scripts can define.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptEvent {
    /// Fired when the application starts.
    OnStart,
    /// Fired when the application is about to quit.
    OnQuit,
    /// Fired on every key press.
    OnKey,
}

/// Manages the Rhai scripting engine and user scripts for shashin.
pub struct ScriptManager {
    engine: ScriptEngine,
    hooks: HashMap<ScriptEvent, Vec<soushi::rhai::AST>>,
    named_scripts: HashMap<String, soushi::rhai::AST>,
    scripts_dir: PathBuf,
}

impl ScriptManager {
    /// Create a new script manager and register shashin-specific functions.
    #[must_use]
    pub fn new() -> Self {
        let mut engine = ScriptEngine::new();
        engine.register_builtin_log();
        engine.register_builtin_env();
        engine.register_builtin_string();

        Self::register_shashin_functions(&mut engine);

        let scripts_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("shashin")
            .join("scripts");

        let mut manager = Self {
            engine,
            hooks: HashMap::new(),
            named_scripts: HashMap::new(),
            scripts_dir,
        };

        manager.load_scripts();
        manager
    }

    /// Register shashin-specific functions with the scripting engine.
    fn register_shashin_functions(engine: &mut ScriptEngine) {
        engine.register_fn("shashin_open", |path: &str| -> String {
            tracing::info!(path, "script: shashin.open");
            format!("opened: {path}")
        });

        engine.register_fn("shashin_zoom", |level: f64| -> String {
            tracing::info!(level, "script: shashin.zoom");
            format!("zoom: {level}")
        });

        engine.register_fn("shashin_rotate", |degrees: i64| -> String {
            tracing::info!(degrees, "script: shashin.rotate");
            format!("rotated: {degrees}")
        });

        engine.register_fn("shashin_export", |path: &str, format: &str| -> String {
            tracing::info!(path, format, "script: shashin.export");
            format!("exported to {path} as {format}")
        });
    }

    /// Load all scripts from the scripts directory.
    fn load_scripts(&mut self) {
        if !self.scripts_dir.is_dir() {
            tracing::debug!(
                path = %self.scripts_dir.display(),
                "scripts directory does not exist, skipping"
            );
            return;
        }

        match self.engine.load_scripts_dir(&self.scripts_dir) {
            Ok(names) => {
                tracing::info!(count = names.len(), "loaded shashin scripts");
                for name in &names {
                    self.compile_named_script(name);
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to load scripts");
            }
        }
    }

    /// Compile and store a named script for later execution.
    fn compile_named_script(&mut self, name: &str) {
        let path = self.scripts_dir.join(format!("{name}.rhai"));
        if let Ok(source) = std::fs::read_to_string(&path) {
            match self.engine.compile(&source) {
                Ok(ast) => {
                    self.named_scripts.insert(name.to_string(), ast);
                }
                Err(e) => {
                    tracing::error!(script = name, error = %e, "failed to compile script");
                }
            }
        }
    }

    /// Register a hook script for a given event.
    pub fn register_hook(&mut self, event: ScriptEvent, script: &str) {
        match self.engine.compile(script) {
            Ok(ast) => {
                self.hooks.entry(event).or_default().push(ast);
            }
            Err(e) => {
                tracing::error!(event = ?event, error = %e, "failed to compile hook");
            }
        }
    }

    /// Fire all hooks registered for a given event.
    pub fn fire_event(&self, event: ScriptEvent) {
        if let Some(scripts) = self.hooks.get(&event) {
            for ast in scripts {
                if let Err(e) = self.engine.eval_ast(ast) {
                    tracing::error!(event = ?event, error = %e, "hook script failed");
                }
            }
        }
    }

    /// Run a named script by file stem.
    pub fn run_script(&self, name: &str) -> Result<soushi::rhai::Dynamic, soushi::SoushiError> {
        if let Some(ast) = self.named_scripts.get(name) {
            self.engine.eval_ast(ast)
        } else {
            let path = self.scripts_dir.join(format!("{name}.rhai"));
            self.engine.eval_file(&path)
        }
    }

    /// Access the underlying script engine.
    #[must_use]
    pub fn engine(&self) -> &ScriptEngine {
        &self.engine
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_script_manager() {
        let _mgr = ScriptManager::new();
    }

    #[test]
    fn register_and_fire_hook() {
        let mut mgr = ScriptManager::new();
        mgr.register_hook(ScriptEvent::OnStart, r#"log_info("on_start fired")"#);
        mgr.fire_event(ScriptEvent::OnStart);
    }

    #[test]
    fn shashin_open_callable() {
        let mgr = ScriptManager::new();
        let result = mgr.engine().eval(r#"shashin_open("/tmp/test.png")"#).unwrap();
        assert!(result.into_string().unwrap().contains("opened"));
    }

    #[test]
    fn shashin_zoom_callable() {
        let mgr = ScriptManager::new();
        let result = mgr.engine().eval("shashin_zoom(2.0)").unwrap();
        assert!(result.into_string().unwrap().contains("zoom"));
    }

    #[test]
    fn shashin_rotate_callable() {
        let mgr = ScriptManager::new();
        let result = mgr.engine().eval("shashin_rotate(90)").unwrap();
        assert!(result.into_string().unwrap().contains("rotated"));
    }

    #[test]
    fn shashin_export_callable() {
        let mgr = ScriptManager::new();
        let result = mgr.engine().eval(r#"shashin_export("/tmp/out.jpg", "jpeg")"#).unwrap();
        assert!(result.into_string().unwrap().contains("exported"));
    }

    #[test]
    fn run_nonexistent_script_errors() {
        let mgr = ScriptManager::new();
        assert!(mgr.run_script("nonexistent_script_12345").is_err());
    }
}
