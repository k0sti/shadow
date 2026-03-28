use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    task::Waker,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::protocol::{HostMessage, RenderPayload, RuntimeMessage};

pub enum RuntimeUpdate {
    Render(RenderPayload),
    Exited(String),
}

pub struct TsRuntime {
    child: Child,
    event_file: File,
    event_file_path: PathBuf,
    render_file_path: PathBuf,
    receiver: Receiver<RuntimeMessage>,
    last_stderr: Arc<Mutex<Option<String>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl TsRuntime {
    pub fn launch() -> io::Result<(Self, RenderPayload)> {
        let (event_file_path, event_file) = create_event_mailbox()?;
        let render_file_path = create_render_mailbox()?;
        eprintln!(
            "shadow-blitz-demo: event mailbox {} render mailbox {}",
            event_file_path.display(),
            render_file_path.display()
        );
        let script = script_path();
        let mut child = Command::new("deno")
            .args(["run", "--quiet", "--no-prompt"])
            .arg(format!("--allow-read={}", event_file_path.display()))
            .arg(format!("--allow-write={}", render_file_path.display()))
            .arg(script)
            .arg(&event_file_path)
            .arg(&render_file_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        eprintln!(
            "shadow-blitz-demo: launched Deno runtime pid {}",
            child.id()
        );
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("missing deno stderr"))?;
        let (sender, receiver) = mpsc::channel();
        let waker = Arc::new(Mutex::new(None::<Waker>));
        let thread_waker = waker.clone();
        let render_path = render_file_path.clone();
        let last_stderr = Arc::new(Mutex::new(None::<String>));
        let stderr_state = last_stderr.clone();

        thread::spawn(move || {
            let mut last_snapshot = String::new();

            let wake = || {
                let maybe_waker = thread_waker.lock().ok().and_then(|guard| guard.clone());
                if let Some(waker) = maybe_waker {
                    waker.wake();
                }
            };

            loop {
                let snapshot = fs::read_to_string(&render_path).unwrap_or_default();
                if !snapshot.trim().is_empty() && snapshot != last_snapshot {
                    last_snapshot = snapshot.clone();
                    match serde_json::from_str::<RuntimeMessage>(&snapshot) {
                        Ok(message) => {
                            if sender.send(message).is_err() {
                                break;
                            }
                            wake();
                        }
                        Err(error) => eprintln!("shadow-blitz-demo: bad runtime message: {error}"),
                    }
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(mut slot) = stderr_state.lock() {
                    *slot = Some(trimmed.to_string());
                }
                eprintln!("shadow-blitz-demo runtime: {trimmed}");
            }
        });

        let mut runtime = Self {
            child,
            event_file,
            event_file_path,
            render_file_path,
            receiver,
            last_stderr,
            waker,
        };
        let initial = runtime.wait_for_render()?;
        Ok((runtime, initial))
    }

    pub fn register_waker(&mut self, waker: &Waker) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(waker.clone());
        }
    }

    pub fn send(&mut self, message: HostMessage) -> io::Result<()> {
        let mut bytes = serde_json::to_vec(&message)
            .map_err(|error| io::Error::other(format!("encode host message: {error}")))?;
        bytes.push(b'\n');
        self.event_file.write_all(&bytes)?;
        self.event_file.flush()
    }

    pub fn drain_update(&mut self) -> Option<RuntimeUpdate> {
        let mut latest = None;

        while let Ok(message) = self.receiver.try_recv() {
            match message {
                RuntimeMessage::Log { message } if !message.is_empty() => {
                    eprintln!("shadow-blitz-demo: {message}");
                }
                RuntimeMessage::Log { .. } => {}
                RuntimeMessage::Render(render) => {
                    latest = Some(RuntimeUpdate::Render(render));
                }
            }
        }

        if latest.is_some() {
            return latest;
        }

        if let Ok(Some(status)) = self.child.try_wait() {
            let reason = exit_reason(
                &self.last_stderr,
                &format!("the Deno runtime exited: {status}"),
            );
            eprintln!("shadow-blitz-demo: Deno runtime exited: {reason}");
            return Some(RuntimeUpdate::Exited(reason));
        }

        None
    }

    fn wait_for_render(&mut self) -> io::Result<RenderPayload> {
        let start = Instant::now();
        let timeout = Duration::from_secs(30);

        loop {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for Deno render",
                ));
            }

            match self.receiver.recv_timeout(remaining) {
                Ok(RuntimeMessage::Log { message }) if !message.is_empty() => {
                    eprintln!("shadow-blitz-demo: {message}");
                }
                Ok(RuntimeMessage::Log { .. }) => {}
                Ok(RuntimeMessage::Render(render)) => {
                    eprintln!("shadow-blitz-demo: received initial Deno render");
                    return Ok(render);
                }
                Err(error) => {
                    log_render_mailbox(&self.render_file_path);
                    if let Some(status) = self.child.try_wait()? {
                        return Err(io::Error::other(format!(
                            "runtime exited before first render: {}",
                            exit_reason(
                                &self.last_stderr,
                                &format!("the Deno runtime exited: {status}")
                            )
                        )));
                    }
                    return Err(io::Error::other(format!(
                        "failed waiting for Deno render: {error}"
                    )));
                }
            }
        }
    }
}

impl Drop for TsRuntime {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.event_file_path);
        let _ = fs::remove_file(&self.render_file_path);
    }
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/app.ts")
}

fn create_event_mailbox() -> io::Result<(PathBuf, File)> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!(
        "shadow-blitz-demo-{}-{unique}.events",
        std::process::id()
    ));
    fs::write(&path, [])?;
    let file = OpenOptions::new().append(true).open(&path)?;
    Ok((path, file))
}

fn create_render_mailbox() -> io::Result<PathBuf> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!(
        "shadow-blitz-demo-{}-{unique}.render",
        std::process::id()
    ));
    fs::write(&path, [])?;
    Ok(path)
}

fn log_render_mailbox(path: &PathBuf) {
    match fs::read_to_string(path) {
        Ok(snapshot) => {
            eprintln!(
                "shadow-blitz-demo: render mailbox snapshot bytes={}",
                snapshot.len()
            );
            if !snapshot.trim().is_empty() {
                let preview = snapshot.chars().take(160).collect::<String>();
                eprintln!("shadow-blitz-demo: render mailbox preview {preview}");
            }
        }
        Err(error) => eprintln!(
            "shadow-blitz-demo: failed reading render mailbox {}: {error}",
            path.display()
        ),
    }
}

fn exit_reason(stderr: &Arc<Mutex<Option<String>>>, default_reason: &str) -> String {
    stderr
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_else(|| default_reason.to_string())
}
