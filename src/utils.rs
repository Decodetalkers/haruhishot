use std::sync::OnceLock;

use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::{
    ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    xdg::xdg_output::zv1::client::zxdg_output_v1::ZxdgOutputV1,
};

#[derive(Debug, Default)]
pub struct Size<T = i32>
where
    T: Default,
{
    pub width: T,
    pub height: T,
}

#[derive(Debug, Default)]
pub struct Position<T = i32>
where
    T: Default,
{
    pub x: T,
    pub y: T,
}

#[derive(Debug)]
pub struct WlOutputInfo {
    pub(crate) output: WlOutput,
    pub(crate) size: Size,
    pub(crate) logical_size: Size,
    pub(crate) position: Position,
    pub(crate) logical_position: Position,
    pub(crate) name: String,
    pub(crate) xdg_output: OnceLock<ZxdgOutputV1>,
}

impl WlOutputInfo {
    pub(crate) fn new(output: WlOutput) -> Self {
        Self {
            output,
            position: Position::default(),
            logical_position: Position::default(),
            size: Size::default(),
            logical_size: Size::default(),
            name: "".to_owned(),
            xdg_output: OnceLock::new(),
        }
    }
}

#[derive(Debug)]
pub struct TopLevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) title: String,
}

impl TopLevel {
    pub(crate) fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: "".to_string(),
        }
    }
}
