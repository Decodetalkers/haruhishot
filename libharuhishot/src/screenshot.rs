use std::{
    ops::Deref,
    os::fd::OwnedFd,
    sync::{Arc, RwLock},
};

use image::ColorType;
use memmap2::MmapMut;
use wayland_client::{
    WEnum,
    protocol::{wl_output::WlOutput, wl_shm},
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::FailureReason, ext_image_copy_capture_manager_v1::Options,
};

use crate::{
    HaruhiShotState,
    haruhierror::HaruhiError,
    state::{CaptureInfo, CaptureState, FrameInfo},
    utils::Size,
};

use std::os::fd::{AsFd, AsRawFd};
use std::{
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on successful execution and seal it's operations.
        match memfd::memfd_create(
            c"wayshot",
            memfd::MFdFlags::MFD_CLOEXEC | memfd::MFdFlags::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd.as_fd(),
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::ENOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/wayshot-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            // O_CREAT = Create file if does not exist.
            // O_EXCL = Error if create and file exists.
            // O_RDWR = Open for reading and writing.
            // O_CLOEXEC = Close on successful execution.
            // S_IRUSR = Set user read permission bit .
            // S_IWUSR = Set user write permission bit.
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd.as_raw_fd()) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(errno) => return Err(std::io::Error::from(errno)),
                },
            },
            Err(nix::errno::Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/wayshot-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub color_type: ColorType,
}

impl HaruhiShotState {
    pub fn shot_single_output(&mut self, output: &WlOutput) -> Result<ImageInfo, HaruhiError> {
        let mut event_queue = self.take_event_queue();
        let img_manager = self.output_image_manager();
        let capture_manager = self.image_copy_capture_manager();
        let qh = self.qhandle();

        let source = img_manager.create_source(output, qh, ());
        let info = Arc::new(RwLock::new(FrameInfo::default()));
        let session =
            capture_manager.create_session(&source, Options::PaintCursors, qh, info.clone());

        let capture_info = CaptureInfo::new();
        let frame = session.create_frame(qh, capture_info.clone());
        event_queue.blocking_dispatch(self).unwrap();
        let qh = self.qhandle();

        let shm = self.shm();
        let info = info.read().unwrap();

        let Size { width, height } = info.size();
        let WEnum::Value(frame_format) = info.format() else {
            return Err(HaruhiError::NotSupportFormat);
        };
        if !matches!(
            frame_format,
            wl_shm::Format::Xbgr2101010
                | wl_shm::Format::Abgr2101010
                | wl_shm::Format::Argb8888
                | wl_shm::Format::Xrgb8888
                | wl_shm::Format::Xbgr8888
        ) {
            return Err(HaruhiError::NotSupportFormat);
        }
        let frame_bytes = 4 * height * width;
        let mem_fd = create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);
        mem_file.set_len(frame_bytes as u64).unwrap();

        let stride = 4 * width;

        let shm_pool = shm.create_pool(mem_file.as_fd(), (width * height * 4) as i32, qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            frame_format,
            qh,
            (),
        );
        frame.attach_buffer(&buffer);
        frame.capture();

        loop {
            event_queue
                .blocking_dispatch(self)
                .map_err(HaruhiError::DispatchError)?;
            let info = capture_info.read().unwrap();
            match info.state() {
                CaptureState::Succeeded => {
                    break;
                }
                CaptureState::Failed(info) => match info {
                    WEnum::Value(reason) => match reason {
                        FailureReason::Stopped => {
                            return Err(HaruhiError::CaptureFailed("Stopped".to_owned()));
                        }

                        FailureReason::BufferConstraints => {
                            return Err(HaruhiError::CaptureFailed("BufferConstraints".to_owned()));
                        }
                        FailureReason::Unknown | _ => {
                            return Err(HaruhiError::CaptureFailed("Unknown".to_owned()));
                        }
                    },
                    WEnum::Unknown(code) => {
                        return Err(HaruhiError::CaptureFailed(format!(
                            "Unknown reason, code : {code}"
                        )));
                    }
                },
                CaptureState::Pedding => {}
            }
        }

        self.reset_event_queue(event_queue);

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        let converter = crate::convert::create_converter(frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageInfo {
            data: frame_mmap.deref().into(),
            width,
            height,
            color_type,
        })
    }
}
