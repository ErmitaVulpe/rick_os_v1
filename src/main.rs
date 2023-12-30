#![no_main]
#![no_std]

extern crate alloc;

use alloc::vec;
use uefi::{prelude::*, CStr16};
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo, RegularFile};


const VIDEO_PATH: &CStr16 = cstr16!("\\VIDEO_BYTES");
const RAW_WIDTH: usize = 384;
const RAW_HEIGHT: usize = 216;
const RAW_BYTES_PER_FRAME: usize = ((RAW_WIDTH * RAW_HEIGHT) >> 1) * 3 ;


struct VideoReader {
    // Frame count of the embedded video
    file_handle: RegularFile,
    frames_read: usize,
    frame_count: usize,
    buffer: [u8; RAW_BYTES_PER_FRAME],
}

impl VideoReader {
    fn new(mut file_handle: RegularFile) -> Self {
        let frame_count = file_handle
            .get_boxed_info::<FileInfo>()
            .unwrap()
            .file_size() as usize / RAW_BYTES_PER_FRAME;

        VideoReader {
            file_handle,
            frames_read: 0,
            frame_count,
            buffer: [0; RAW_BYTES_PER_FRAME],
        }
    }

    // Rewind to the start of the video
    fn rewind(&mut self) {
        self.file_handle.set_position(0).unwrap();
        self.frames_read = 0;
    }

    // Read the yuv420p bytes for the next frame and give a reference to a buffer
    fn next_frame(&mut self) -> &[u8; RAW_BYTES_PER_FRAME] {
        if self.frames_read >= self.frame_count {
            self.rewind()
        }

        self.file_handle.set_position((self.frames_read * RAW_BYTES_PER_FRAME) as u64)
            .unwrap();
        self.file_handle.read(&mut self.buffer).unwrap();
        self.frames_read += 1;
        &self.buffer
    }
}


#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    let boot_services = system_table.boot_services();


    // Get protocol to the simple file system
    let mut simple_file_system = boot_services.get_image_file_system(
        boot_services.image_handle())
        .unwrap();
    let mut root_dir = simple_file_system
        .open_volume()
        .unwrap();
    // Get a handle to the video file
    let file_handle = root_dir
        .open(VIDEO_PATH, FileMode::Read, FileAttribute::SYSTEM)
        .unwrap()
        .into_regular_file()
        .unwrap();

    let mut video_reader = VideoReader::new(file_handle);


    // Activate the Graphics Output Protocol
    let gop_handle = boot_services
        .get_handle_for_protocol::<GraphicsOutput>()
        .unwrap();
    let mut gop = boot_services
        .open_protocol_exclusive::<GraphicsOutput>(gop_handle)
        .unwrap();

    // Get the resolution of the display
    let (display_width, display_height) = gop.current_mode_info().resolution();
    let display_res = display_width * display_height;


    let mut blt_buffer = vec![BltPixel::new(0, 0, 0); display_res];
    loop {
        // Read the next frame and convert it to rgb24 (BltPixel)
        let frame = video_reader.next_frame();
        yuv420p_to_rgb24(frame, display_width, display_height, &mut blt_buffer);

        // Write to the screen
        gop.blt(BltOp::BufferToVideo {
            buffer: &blt_buffer,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (display_width, display_height),
        })
        .unwrap();
    }
}


pub fn yuv420p_to_rgb24(
    yuv_buffer: &[u8],
    target_width: usize,
    target_height: usize,
    blt_buffer: &mut [BltPixel],
) {
    for y in 0..target_height {
        // mapped x and y refer to the x and y in the original buffer for a display pixel
        let mapped_y = ((y as f32 / target_height as f32) * RAW_HEIGHT as f32) as usize;
        for x in 0..target_width {
            let mapped_x = ((x as f32 / target_width as f32) * RAW_WIDTH as f32) as usize;

            // Get yuv values from the original buffer
            let yv = yuv_buffer[mapped_y * RAW_WIDTH + mapped_x] as f32;
            let uv = yuv_buffer[RAW_WIDTH * RAW_HEIGHT + (mapped_y/2 * RAW_WIDTH/2 + mapped_x/2)] as f32;
            let vv = yuv_buffer[RAW_WIDTH * RAW_HEIGHT + RAW_WIDTH / 2 * RAW_HEIGHT / 2 + (mapped_y/2 * RAW_WIDTH/2 + mapped_x/2)] as f32;

            // Calculate rgb values
            let r = yv + 1.402 * (vv - 128.0);
            let g = yv - 0.3441 * (uv - 128.0) - 0.7141 * (vv - 128.0);
            let b = yv + 1.772 * (uv - 128.0);

            // Modify output buffer pixel
            let pix_ref = &mut blt_buffer[y * target_width + x];
            pix_ref.red = r as u8;
            pix_ref.green = g as u8;
            pix_ref.blue = b as u8;
        }
    }
}
