use std::{
    ops::{Add, Sub},
    sync::OnceLock,
};

use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_protocols::{
    ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    xdg::xdg_output::zv1::client::zxdg_output_v1::ZxdgOutputV1,
};

/// Describe the size
#[derive(Debug, Default, Clone, Copy)]
pub struct Size<T = i32>
where
    T: Default,
{
    pub width: T,
    pub height: T,
}

/// Describe the position
#[derive(Debug, Default, Clone, Copy)]
pub struct Position<T = i32>
where
    T: Default,
{
    pub x: T,
    pub y: T,
}

impl<T> Sub for Position<T>
where
    T: Sub<Output = T> + Default,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T> Add for Position<T>
where
    T: Add<Output = T> + Default,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub position: Position,
    pub size: Size,
}
#[derive(Debug, Clone, Copy)]
pub struct ClipRegion {
    pub relative_region_real: Region,
    pub relative_region_wl: Region,
    pub display_region: Region,
}

impl ClipRegion {
    pub fn absolute_position_real(&self) -> Position {
        let position = self.display_region.position;
        position + self.relative_region_real.position
    }
    pub fn relative_position_real(&self) -> Position {
        self.relative_region_real.position
    }
    pub fn relative_position_wl(&self) -> Position {
        self.relative_region_wl.position
    }
    pub fn clip_size_wl(&self) -> Size {
        self.relative_region_wl.size
    }
    pub fn clip_size_real(&self) -> Size {
        self.relative_region_real.size
    }
}

/// contain the output and their messages
#[derive(Debug, Clone)]
pub struct WlOutputInfo {
    pub(crate) output: WlOutput,
    pub(crate) size: Size,
    pub(crate) logical_size: Size,
    pub(crate) position: Position,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) xdg_output: OnceLock<ZxdgOutputV1>,
    pub(crate) transform: wl_output::Transform,
    pub(crate) scale: i32,
}

impl WlOutputInfo {
    /// The name of the output or maybe the screen?
    pub fn name(&self) -> &str {
        &self.name
    }

    /// get the description
    pub fn description(&self) -> &str {
        &self.description
    }
    /// get the wl_output
    pub fn wl_output(&self) -> &WlOutput {
        &self.output
    }

    /// get the position
    pub fn position(&self) -> Position {
        self.position
    }

    /// get the logical size
    pub fn logical_size(&self) -> Size {
        self.logical_size
    }
    pub(crate) fn new(output: WlOutput) -> Self {
        Self {
            output,
            position: Position::default(),
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

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) title: String,
    pub(crate) app_id: String,
    pub(crate) identifier: String,
    pub(crate) active: bool,
}

impl TopLevel {
    pub(crate) fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: "".to_owned(),
            app_id: "".to_owned(),
            identifier: "".to_owned(),
            active: true,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn id_and_title(&self) -> String {
        format!("{} {}", self.app_id(), self.title())
    }

    pub fn handle(&self) -> &ExtForeignToplevelHandleV1 {
        &self.handle
    }

    pub fn active(&self) -> bool {
        self.active
    }
}
