//! The app-lifecycle engine: spawn app servers, probe them to readiness, open
//! their window shell and tear everything down (tree kill) when asked.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use nucleus_launcher_core::{AppMeta, LaunchKind};

use crate::ports::free_port;
use crate::probe::READY_TIMEOUT;
use crate::process::{kill_tree, random_token, spawn_command, spawn_node};
use crate::window::open_app_window;

/// What the engine tells the UI, drained once per frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The app process is starting.
    Starting(String),
    /// The app is up and (for servers) answering. `url` is empty for
    /// command-kind apps.
    Ready(String, String),
    /// Launch or readiness probe failed.
    Failed(String, String),
    /// Stopped (by the user, or because the window/process exited on its own).
    Stopped(String),
}

/// Fixed engine options resolved once at startup.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// `where node` result or `NUCLEUS_NODE`; `None` → warn the user, and
    /// spawn falls back to the literal `"node"`.
    pub node_path: Option<String>,
    /// `assets/nucleus-shell.exe` when built.
    pub shell_exe: Option<PathBuf>,
    /// `assets/nucleus.ico` when present.
    pub icon_ico: Option<PathBuf>,
    /// `%APPDATA%\Nucleus\window-profiles`.
    pub window_profiles_dir: PathBuf,
    /// Shell window size, e.g. `(1440, 900)`.
    pub window_size: (u16, u16),
}

/// A live app the engine is supervising.
struct Running {
    id: String,
    /// The app process: node for servers, the command for kind=command.
    node: Option<Child>,
    /// The shell window process (server/url kinds with a window).
    window: Option<Child>,
    /// Pid used to tree-kill the primary process (node/command or window).
    pid: Option<u32>,
    /// Separate pid for the shell window, so closing it kills node too.
    window_pid: Option<u32>,
    port: Option<u16>,
    url: Option<String>,
}

impl Running {
    /// Tree-kill every tracked process; never panics.
    fn kill_all(&self) {
        if let Some(pid) = self.pid {
            kill_tree(pid);
        }
        if let Some(window_pid) = self.window_pid
            && self.pid != Some(window_pid)
        {
            kill_tree(window_pid);
        }
    }
}

#[derive(Clone)]
pub struct Runtime {
    inner: Arc<Mutex<RuntimeInner>>,
    tx: Sender<Event>,
}

struct RuntimeInner {
    config: RuntimeConfig,
    running: Vec<Running>,
}

impl Runtime {
    /// Create the engine and the event channel the UI must drain.
    pub fn new(config: RuntimeConfig) -> (Runtime, Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let inner = Arc::new(Mutex::new(RuntimeInner {
            config,
            running: Vec::new(),
        }));
        (Runtime { inner, tx }, rx)
    }

    /// Launch an app; no-op (Ok) when the id is already running.
    ///
    /// Mirrors the TS reference ordering: `starting` first, then spawn +
    /// probe; the window opens once **ready**; `ready` fires.
    pub fn launch(&self, app: &AppMeta) -> Result<(), String> {
        let id = app.id().to_string();
        let name = app.name().to_string();
        let spec = app
            .launch_spec()
            .ok_or_else(|| format!("app `{id}` has no launch spec"))?;

        {
            let guard = self.inner.lock().unwrap();
            if guard.running.iter().any(|entry| entry.id == id) {
                return Ok(()); // already running
            }
        }

        let config = self.inner.lock().unwrap().config.clone();
        let profile_dir = config.window_profiles_dir.join(&id);
        let notify = |event: Event| {
            let _ = self.tx.send(event);
        };

        notify(Event::Starting(id.clone()));

        match spec.kind {
            LaunchKind::Command => {
                let command = spec
                    .command
                    .clone()
                    .unwrap_or_else(|| node_or_literal(&config));
                let child = spawn_command(&command, &spec.args, &app.dir).map_err(|err| {
                    notify(Event::Failed(id.clone(), err.to_string()));
                    format!("failed to spawn `{command}`: {err}")
                })?;
                let pid = child.id();
                let id_ready = id.clone();
                self.push_entry(Running {
                    id: id.clone(),
                    node: Some(child),
                    window: None,
                    pid: Some(pid),
                    window_pid: None,
                    port: None,
                    url: None,
                });
                notify(Event::Ready(id_ready, String::new()));
            }
            LaunchKind::Server => {
                let port = free_port(spec.port).map_err(|err| {
                    notify(Event::Failed(
                        id.clone(),
                        format!("cannot pick a port: {err}"),
                    ));
                    format!("cannot pick a port for `{id}`: {err}")
                })?;
                let command = spec
                    .command
                    .clone()
                    .unwrap_or_else(|| node_or_literal(&config));
                let token = random_token();
                let child =
                    spawn_node(&command, &spec.args, &app.dir, port, &token).map_err(|err| {
                        notify(Event::Failed(
                            id.clone(),
                            format!("cannot spawn node: {err}"),
                        ));
                        format!("failed to spawn node for `{id}`: {err}")
                    })?;
                let pid = child.id();
                // The token rides in the URL only for the first load; the app
                // swaps it for an HttpOnly cookie and redirects to the clean URL.
                let url = format!("http://127.0.0.1:{port}/?t={token}");
                let want_window = spec.window.unwrap_or(true);
                let ready_path = spec.ready_path.clone().unwrap_or_else(|| "/".to_string());

                self.push_entry(Running {
                    id: id.clone(),
                    node: Some(child),
                    window: None,
                    pid: Some(pid),
                    window_pid: None,
                    port: Some(port),
                    url: Some(url.clone()),
                });

                // Probe in the background so the UI stays responsive; the
                // window opens only once the server answers.
                let runtime = self.clone();
                std::thread::spawn(move || {
                    match crate::probe::wait_ready_with_timeout(
                        port,
                        &ready_path,
                        READY_TIMEOUT,
                        &token,
                    ) {
                        Ok(()) => {
                            let id_ready = runtime.ready_window(
                                &id,
                                &name,
                                &url,
                                want_window,
                                &profile_dir,
                                &config,
                            );
                            if let Some(id_ready) = id_ready {
                                let _ = runtime.tx.send(Event::Ready(id_ready, url));
                            }
                        }
                        Err(message) => runtime.fail(id, message),
                    }
                });
            }
            LaunchKind::Url => {
                let url = spec.url.clone().ok_or_else(|| {
                    notify(Event::Failed(id.clone(), "launch.url is empty".into()));
                    format!("app `{id}` has kind=url but no url")
                })?;
                let want_window = spec.window.unwrap_or(true);
                let window = if want_window {
                    open_app_window(
                        &url,
                        &name,
                        config.icon_ico.as_deref(),
                        &profile_dir,
                        config.window_size,
                        config.shell_exe.as_deref(),
                    )
                } else {
                    None
                };
                let window_pid = window.as_ref().map(Child::id);
                let id_ready = id.clone();
                self.push_entry(Running {
                    id: id.clone(),
                    node: None,
                    window,
                    pid: window_pid,
                    window_pid,
                    port: None,
                    url: Some(url.clone()),
                });
                notify(Event::Ready(id_ready, url));
            }
        }
        Ok(())
    }

    /// Stop one app: tree-kill its process and window, emit `Stopped`.
    pub fn stop(&self, id: &str) -> bool {
        let entry = {
            let mut guard = self.inner.lock().unwrap();
            let Some(position) = guard.running.iter().position(|entry| entry.id == id) else {
                return false;
            };
            guard.running.remove(position)
        };
        entry.kill_all();
        let _ = self.tx.send(Event::Stopped(entry.id.clone()));
        true
    }

    /// Stop everything (app exit hook + runtime drop).
    pub fn stop_all(&self) {
        let ids = self.running_ids();
        for id in ids {
            self.stop(&id);
        }
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.inner
            .lock()
            .unwrap()
            .running
            .iter()
            .any(|entry| entry.id == id)
    }

    pub fn running_count(&self) -> usize {
        self.inner.lock().unwrap().running.len()
    }

    /// Ids of running apps, in launch order.
    pub fn running_ids(&self) -> Vec<String> {
        let guard = self.inner.lock().unwrap();
        guard.running.iter().map(|entry| entry.id.clone()).collect()
    }

    /// Live details for the inspector: `(url, port)` when the app is running.
    pub fn running_info(&self, id: &str) -> Option<(Option<String>, Option<u16>)> {
        let guard = self.inner.lock().unwrap();
        guard
            .running
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| (entry.url.clone(), entry.port))
    }

    /// Reap exited processes so a closed window or a crashed node auto-stops
    /// the app. Call once per UI frame.
    pub fn reconcile(&self) {
        let mut guard = self.inner.lock().unwrap();
        let mut index = 0;
        let mut stopped = Vec::new();
        while index < guard.running.len() {
            let node_exited = exited(&mut guard.running[index].node);
            let window_exited = exited(&mut guard.running[index].window);
            if node_exited || window_exited {
                let entry = guard.running.remove(index);
                entry.kill_all();
                stopped.push(entry.id);
            } else {
                index += 1;
            }
        }
        drop(guard);
        for id in stopped {
            let _ = self.tx.send(Event::Stopped(id));
        }
    }

    /// Register a running app. Done before the readiness probe so the UI can
    /// already show the "running" state.
    fn push_entry(&self, entry: Running) {
        self.inner.lock().unwrap().running.push(entry);
    }

    /// Probe succeeded: open the shell window (if wanted) and return the app
    /// id so the probe thread can emit `Ready`. Returns `None` if the app was
    /// stopped in the meantime.
    fn ready_window(
        &self,
        id: &str,
        name: &str,
        url: &str,
        want_window: bool,
        profile_dir: &Path,
        config: &RuntimeConfig,
    ) -> Option<String> {
        let mut guard = self.inner.lock().unwrap();
        let position = guard.running.iter().position(|entry| entry.id == id)?;
        let entry = &mut guard.running[position];
        if want_window
            && let Some(window) = open_app_window(
                url,
                name,
                config.icon_ico.as_deref(),
                profile_dir,
                config.window_size,
                config.shell_exe.as_deref(),
            )
        {
            let window_pid = window.id();
            entry.window = Some(window);
            if entry.pid.is_none() {
                entry.pid = Some(window_pid);
            }
            entry.window_pid = Some(window_pid);
        }
        Some(entry.id.clone())
    }

    /// A launch/probe failure: kill any half-started processes, forget the
    /// app, emit `Failed`.
    fn fail(&self, id: String, message: String) {
        let entry = {
            let mut guard = self.inner.lock().unwrap();
            guard
                .running
                .iter()
                .position(|entry| entry.id == id)
                .map(|position| guard.running.remove(position))
        };
        if let Some(entry) = entry {
            entry.kill_all();
        }
        let _ = self.tx.send(Event::Failed(id, message));
    }
}

/// `try_wait` → true when the process has exited (or failed to start).
fn exited(child: &mut Option<Child>) -> bool {
    child
        .as_mut()
        .map(|child| child.try_wait().ok().flatten().is_some())
        .unwrap_or(false)
}

fn node_or_literal(config: &RuntimeConfig) -> String {
    config
        .node_path
        .clone()
        .unwrap_or_else(|| "node".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleus_launcher_core::{AppSource, parse_manifest};

    fn config() -> RuntimeConfig {
        RuntimeConfig {
            node_path: None,
            shell_exe: None,
            icon_ico: None,
            window_profiles_dir: std::env::temp_dir().join("nucleus-test-profiles"),
            window_size: (100, 100),
        }
    }

    fn url_app() -> AppMeta {
        AppMeta {
            manifest: parse_manifest(
                r#"{"id":"nucleus.url","name":"Url","version":"0.1.0","coreApiVersion":"1","launch":{"kind":"url","url":"http://127.0.0.1:9/"}}"#,
            )
            .expect("valid"),
            dir: std::env::temp_dir(),
            manifest_path: std::env::temp_dir().join("nucleus.app.json"),
            source: AppSource::Repo,
            installed: None,
            catalog: None,
        }
    }

    #[test]
    fn url_launch_reports_starting_then_ready_then_stopped() {
        let (runtime, rx) = Runtime::new(config());
        // No shell binary configured → falls back to `cmd /c start` which we
        // never track, yet the app is still reported ready.
        runtime.launch(&url_app()).expect("url launch succeeds");
        assert!(runtime.is_running("nucleus.url"));

        assert_eq!(
            rx.try_recv().unwrap(),
            Event::Starting("nucleus.url".into())
        );
        match rx.try_recv().unwrap() {
            Event::Ready(id, url) => {
                assert_eq!(id, "nucleus.url");
                assert_eq!(url, "http://127.0.0.1:9/");
            }
            other => panic!("expected Ready, got {other:?}"),
        }

        assert!(runtime.stop("nucleus.url"));
        assert!(!runtime.is_running("nucleus.url"));
        assert!(matches!(rx.try_recv().unwrap(), Event::Stopped(id) if id == "nucleus.url"));
    }

    #[test]
    fn stop_unknown_id_returns_false() {
        let (runtime, _rx) = Runtime::new(config());
        assert!(!runtime.stop("nucleus.nope"));
    }

    #[test]
    fn stop_all_clears() {
        let (runtime, _rx) = Runtime::new(config());
        runtime.launch(&url_app()).unwrap();
        assert_eq!(runtime.running_count(), 1);
        runtime.stop_all();
        assert_eq!(runtime.running_count(), 0);
    }
}
