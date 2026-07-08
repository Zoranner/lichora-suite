//! Frame publication through IPC v2 frame channels.

use anyhow::{ensure, Result};

const MAX_WIDTH: usize = 2560;
const MAX_HEIGHT: usize = 1440;
const BYTES_PER_PIXEL: usize = 4;
const MAX_PIXEL_BUFFER_SIZE: usize = MAX_WIDTH * MAX_HEIGHT * BYTES_PER_PIXEL;

pub struct CaptureModule {
    frame_channel: ipc::FrameChannel,
    frame_sequence: u64,
    width: i32,
    height: i32,
    frame_buffer: Vec<u8>,
}

impl CaptureModule {
    pub fn new_frame_channel_only(
        session_id: &str,
        browser_id: &str,
        width: i32,
        height: i32,
    ) -> Result<Self> {
        let frame_channel = open_or_create_frame_channel(session_id, browser_id)?;
        Ok(Self {
            frame_channel,
            frame_sequence: 0,
            width,
            height,
            frame_buffer: Vec::new(),
        })
    }

    pub fn write_paint_frame(
        &mut self,
        width: i32,
        height: i32,
        pixels: &[u8],
        _dirty_rects: &[(i32, i32, i32, i32)],
    ) -> Result<bool> {
        ensure!(width > 0 && height > 0, "invalid capture size");
        let pixel_data_size = width as usize * height as usize * BYTES_PER_PIXEL;
        ensure!(
            pixel_data_size == pixels.len(),
            "pixel buffer length does not match capture size"
        );
        ensure!(
            pixel_data_size <= MAX_PIXEL_BUFFER_SIZE,
            "pixel buffer exceeds capture slot size"
        );

        self.frame_sequence = self.frame_sequence.saturating_add(1);
        self.frame_buffer.clear();
        self.frame_buffer.extend_from_slice(pixels);

        let published = self
            .frame_channel
            .publish_full(
                self.frame_sequence,
                width as u32,
                height as u32,
                &self.frame_buffer,
            )?
            .published();
        if published {
            self.width = width;
            self.height = height;
        }
        Ok(published)
    }

    #[allow(dead_code)]
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

fn open_or_create_frame_channel(session_id: &str, browser_id: &str) -> Result<ipc::FrameChannel> {
    let directory = ipc_directory();
    let spec = ipc::frame_channel_spec(session_id, browser_id);
    ipc::FrameChannel::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::OpenExisting)
        .or_else(|_| {
            ipc::FrameChannel::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::Create)
        })
        .map_err(Into::into)
}

fn ipc_directory() -> std::path::PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("LichoraIpc"))
}

#[cfg(test)]
mod tests {
    use super::CaptureModule;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn publishes_frame_channel_without_legacy_capture_file() {
        let _guard = env_lock().lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("EBI_IPC_DIR", temp.path());
        let session_id = uuid::Uuid::new_v4().to_string();
        let browser_id = uuid::Uuid::new_v4().to_string();
        let mut module =
            CaptureModule::new_frame_channel_only(&session_id, &browser_id, 2, 2).unwrap();
        let pixels = vec![9u8, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255];

        assert!(module.write_paint_frame(2, 2, &pixels, &[]).unwrap());

        let spec = ipc::frame_channel_spec(&session_id, &browser_id);
        let mut reader =
            ipc::FrameChannel::open_in_dir(temp.path(), &spec, ipc::ChannelOpenMode::OpenExisting)
                .unwrap();
        let mut buffer = vec![0u8; pixels.len()];
        let copied = reader.try_copy_latest(&mut buffer).unwrap();

        assert_eq!(1, copied.sequence);
        assert_eq!(2, copied.width);
        assert_eq!(2, copied.height);
        assert_eq!(pixels, buffer);
        std::env::remove_var("EBI_IPC_DIR");
    }

    #[test]
    fn preserves_browser_frame_alpha() {
        let _guard = env_lock().lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("EBI_IPC_DIR", temp.path());
        let session_id = uuid::Uuid::new_v4().to_string();
        let browser_id = uuid::Uuid::new_v4().to_string();
        let mut module =
            CaptureModule::new_frame_channel_only(&session_id, &browser_id, 2, 1).unwrap();
        let pixels = vec![10u8, 20, 30, 0, 40, 50, 60, 128];

        assert!(module.write_paint_frame(2, 1, &pixels, &[]).unwrap());

        let spec = ipc::frame_channel_spec(&session_id, &browser_id);
        let mut reader =
            ipc::FrameChannel::open_in_dir(temp.path(), &spec, ipc::ChannelOpenMode::OpenExisting)
                .unwrap();
        let mut buffer = vec![0u8; pixels.len()];
        reader.try_copy_latest(&mut buffer).unwrap();

        assert_eq!(pixels, buffer);
        std::env::remove_var("EBI_IPC_DIR");
    }
}
