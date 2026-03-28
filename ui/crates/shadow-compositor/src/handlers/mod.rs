#![cfg(target_os = "linux")]

mod compositor;
mod xdg_shell;

use smithay::{
    delegate_data_device, delegate_output, delegate_seat,
    input::dnd::{DnDGrab, DndGrabHandler, GrabType, Source},
    input::pointer::Focus,
    input::{Seat, SeatHandler, SeatState},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::Serial,
    wayland::output::OutputHandler,
    wayland::selection::data_device::{
        set_data_device_focus, DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler,
    },
    wayland::selection::SelectionHandler,
};

use crate::state::ShadowCompositor;

impl SeatHandler for ShadowCompositor {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let client = focused.and_then(|surface| self.display_handle.get_client(surface.id()).ok());
        set_data_device_focus(&self.display_handle, seat, client);
    }
}

delegate_seat!(ShadowCompositor);

impl SelectionHandler for ShadowCompositor {
    type SelectionUserData = ();
}

impl DataDeviceHandler for ShadowCompositor {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}

impl DndGrabHandler for ShadowCompositor {}
impl WaylandDndGrabHandler for ShadowCompositor {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        _icon: Option<WlSurface>,
        seat: Seat<Self>,
        serial: Serial,
        type_: GrabType,
    ) {
        match type_ {
            GrabType::Pointer => {
                let pointer = seat.get_pointer().unwrap();
                let start_data = pointer.grab_start_data().unwrap();
                let grab = DnDGrab::new_pointer(&self.display_handle, start_data, source, seat);
                pointer.set_grab(self, grab, serial, Focus::Keep);
            }
            GrabType::Touch => source.cancel(),
        }
    }
}

delegate_data_device!(ShadowCompositor);

impl OutputHandler for ShadowCompositor {}
delegate_output!(ShadowCompositor);
