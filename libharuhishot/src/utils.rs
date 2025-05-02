use std::sync::OnceLock;

use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_protocols::{
    ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    xdg::xdg_output::zv1::client::zxdg_output_v1::ZxdgOutputV1,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Size<T = i32>
where
    T: Default,
{
    pub width: T,
    pub height: T,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Position<T = i32>
where
    T: Default,
{
    pub x: T,
    pub y: T,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct WlOutputInfo {
    pub(crate) output: WlOutput,
    pub(crate) size: Size,
    pub(crate) logical_size: Size,
    pub(crate) position: Position,
    pub(crate) logical_position: Position,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) xdg_output: OnceLock<ZxdgOutputV1>,
    pub(crate) transform: wl_output::Transform,
    pub(crate) scale: i32,
}

impl WlOutputInfo {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn output(&self) -> &WlOutput {
        &self.output
    }
    pub(crate) fn new(output: WlOutput) -> Self {
        Self {
            output,
            position: Position::default(),
            logical_position: Position::default(),
            size: Size::default(),
            logical_size: Size::default(),
            name: "".to_owned(),
            description: "".to_owned(),
            xdg_output: OnceLock::new(),
            transform: wl_output::Transform::Normal,
            scale: 1,
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
