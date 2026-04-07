use std::{
    io::{self, Read, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
};

use shadow_ui_core::control::ControlRequest;
use smithay::reexports::calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction};

use crate::ShadowGuestCompositor;

pub fn init_listener(event_loop: &mut EventLoop<ShadowGuestCompositor>) -> io::Result<PathBuf> {
    let path = control_socket_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }

    let listener = UnixListener::bind(&path)?;
    listener.set_nonblocking(true)?;

    event_loop
        .handle()
        .insert_source(
            Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, state| {
                loop {
                    let mut stream = match unsafe { listener.get_mut() }.accept() {
                        Ok((stream, _)) => stream,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                        Err(error) => {
                            tracing::warn!(
                                "[shadow-guest-compositor] control-accept failed: {error}"
                            );
                            break;
                        }
                    };

                    let mut request = String::new();
                    match stream.read_to_string(&mut request) {
                        Ok(_) => {
                            let Some(request) = ControlRequest::parse(&request) else {
                                tracing::warn!(
                                    "[shadow-guest-compositor] ignoring malformed control request"
                                );
                                continue;
                            };
                            match state.handle_control_request(request) {
                                Ok(response) => {
                                    if let Err(error) = stream.write_all(response.as_bytes()) {
                                        tracing::warn!(
                                            "[shadow-guest-compositor] control-response failed: {error}"
                                        );
                                    }
                                }
                                Err(error) => {
                                    let _ = stream.write_all(
                                        format!("error={}\n", error.to_string().replace('\n', " "))
                                            .as_bytes(),
                                    );
                                    tracing::warn!(
                                        "[shadow-guest-compositor] control-request failed: {error}"
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            let _ = stream.write_all(
                                format!("error={}\n", error.to_string().replace('\n', " "))
                                    .as_bytes(),
                            );
                            tracing::warn!(
                                "[shadow-guest-compositor] control-read failed: {error}"
                            );
                        }
                    }
                }

                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

    Ok(path)
}

fn control_socket_path() -> PathBuf {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    runtime_dir.join(shadow_ui_core::control::COMPOSITOR_CONTROL_SOCKET)
}
