use image::{codecs::png::PngEncoder, ImageEncoder};
use image::{GenericImage, GenericImageView, ImageBuffer, Pixel, Primitive};

use crate::wlrbackend::BufferData;
use crate::constenv::SAVEPATH;

use std::io::Write;
use std::io::{stdout, BufWriter, Cursor};
use std::time;

//use std::io::{stdout, BufWriter};
pub fn write_to_file(bufferdata: BufferData, usestdout: bool) {
    if usestdout {
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        PngEncoder::new(&mut writer)
            .write_image(
                &bufferdata.frame_mmap.unwrap(),
                bufferdata.width,
                bufferdata.height,
                image::ColorType::Rgba8,
            )
            .unwrap()
    } else {
        let file = format!(
            "{}-haruhui.png",
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let file = SAVEPATH.join(file);
        let mut writer = std::fs::File::create(file).unwrap();
        //let frame_mmap = &mut bufferdata.frame_mmap.unwrap();
        PngEncoder::new(&mut writer)
            .write_image(
                &bufferdata.frame_mmap.unwrap(),
                bufferdata.width,
                bufferdata.height,
                image::ColorType::Rgba8,
            )
            .unwrap()
    }
}

pub fn write_to_file_fullscreen(bufferdatas: Vec<BufferData>, usestdout: bool) {
    let mut images = Vec::new();
    for buffer in bufferdatas {
        let mut buff = Cursor::new(Vec::new());
        PngEncoder::new(&mut buff)
            .write_image(
                &buffer.frame_mmap.unwrap(),
                buffer.width,
                buffer.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        images.push(
            image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap(),
        );
    }
    if usestdout {
        let mut buff = Cursor::new(Vec::new());
        h_concat(&images)
            .write_to(&mut buff, image::ImageFormat::Png)
            .unwrap();
        let content = buff.get_ref();
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write(content).unwrap();
    } else {
        let file = format!(
            "{}-haruhui.png",
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let file = SAVEPATH.join(file);
        h_concat(&images).save(file).unwrap();
    }
}
fn h_concat<I, P, S>(images: &[I]) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    // The final width is the sum of all images width.
    let img_width_out: u32 = images.iter().map(|im| im.width()).sum();

    // The final height is the maximum height from the input images.
    let img_height_out: u32 = images.iter().map(|im| im.height()).max().unwrap_or(0);

    // Initialize an image buffer with the appropriate size.
    let mut imgbuf = image::ImageBuffer::new(img_width_out, img_height_out);
    let mut accumulated_width = 0;

    // Copy each input image at the correct location in the output image.
    for img in images {
        imgbuf.copy_from(img, accumulated_width, 0).unwrap();
        accumulated_width += img.width();
    }

    imgbuf
}
