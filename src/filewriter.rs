use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, ImageEncoder, Pixel};
use libharuhishot::reexport::wl_output;
#[cfg(feature = "notify")]
use notify_rust::Notification;

use crate::constenv::SAVEPATH;
#[cfg(feature = "notify")]
use crate::constenv::{FAILED_IMAGE, SUCCESSED_IMAGE, TIMEOUT};
use libharuhishot::FrameInfo;

use std::io::Write;
use std::io::{stdout, BufWriter, Cursor};
use std::time;

pub fn get_color(bufferdata: FrameInfo) {
    let mut buff = Cursor::new(Vec::new());
    PngEncoder::new(&mut buff)
        .write_image(
            &bufferdata.frame_mmap,
            bufferdata.frameformat.width,
            bufferdata.frameformat.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    let image =
        image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();
    let pixel = image.get_pixel(0, 0);
    println!(
        "RGB: R:{}, G:{}, B:{}, A:{}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
    println!(
        "16hex: #{:02x}{:02x}{:02x}{:02x}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
}

//use std::io::{stdout, BufWriter};
pub fn write_to_file(bufferdata: FrameInfo, usestdout: bool) {
    if usestdout {
        let mut buff = Cursor::new(Vec::new());
        if let Err(_e) = PngEncoder::new(&mut buff).write_image(
            &bufferdata.frame_mmap,
            bufferdata.frameformat.width,
            bufferdata.frameformat.height,
            image::ColorType::Rgba8,
        ) {
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileCopyFailed")
                .body(&format!("File failed to copy: {_e}"))
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        } else {
            let content = buff.get_ref();
            let stdout = stdout();
            let mut writer = BufWriter::new(stdout.lock());
            if let Err(_e) = writer.write_all(content) {
                #[cfg(feature = "notify")]
                let _ = Notification::new()
                    .summary("PictureWriteToStdoutFailed")
                    .body(&format!("Picture failed to write: {_e}"))
                    .icon(FAILED_IMAGE)
                    .timeout(TIMEOUT)
                    .show();
            } else {
                #[cfg(feature = "notify")]
                {
                    let _ = Notification::new()
                        .summary("Screenshot")
                        .body("Screenshot Succeed")
                        .icon(SUCCESSED_IMAGE)
                        .timeout(TIMEOUT)
                        .show();
                }
            }
        }
    } else {
        let file_name = format!(
            "{}-haruhui.png",
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        #[cfg(feature = "notify")]
        let file = SAVEPATH.join(&file_name);
        #[cfg(not(feature = "notify"))]
        let file = SAVEPATH.join(file_name);
        let filefullname = file.to_str().unwrap();
        let mut writer = std::fs::File::create(&file).unwrap();
        //let frame_mmap = &mut bufferdata.frame_mmap.unwrap();
        if PngEncoder::new(&mut writer)
            .write_image(
                &bufferdata.frame_mmap,
                bufferdata.frameformat.width,
                bufferdata.frameformat.height,
                image::ColorType::Rgba8,
            )
            .is_ok()
        {
            tracing::info!("Image saved to {}", filefullname);
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileSaved")
                .body(&format!("File saved to {}", filefullname))
                .icon(SUCCESSED_IMAGE)
                .timeout(TIMEOUT)
                .show();
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("Screenshot of screen")
                .body(&file_name)
                .icon(filefullname)
                .timeout(TIMEOUT)
                .show();
        } else {
            tracing::error!("Image failed saved to {}", filefullname);
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileSavedFailed")
                .body(&format!("File failed saved to {}", filefullname))
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
    }
}

pub fn roate_image<I: GenericImageView>(
    image: &I,
    transform: wl_output::Transform,
    width: u32,
    height: u32,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I::Pixel: 'static,
    <I::Pixel as Pixel>::Subpixel: 'static,
{
    match transform {
        wl_output::Transform::_90 => {
            let image = image::imageops::rotate90(image);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::_180 => {
            let image = image::imageops::rotate180(image);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::_270 => {
            let image = image::imageops::rotate270(image);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::Flipped => {
            let image = image::imageops::flip_horizontal(image);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::Flipped90 => {
            let filp = image::imageops::flip_horizontal(image);
            let image = image::imageops::rotate90(&filp);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::Flipped180 => {
            let filp = image::imageops::flip_horizontal(image);
            let image = image::imageops::rotate180(&filp);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        wl_output::Transform::Flipped270 => {
            let filp = image::imageops::flip_horizontal(image);
            let image = image::imageops::rotate270(&filp);
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Gaussian)
        }
        _ => image::imageops::resize(image, width, height, image::imageops::FilterType::Gaussian),
    }
}

pub fn write_to_file_mutisource(bufferdatas: Vec<FrameInfo>, usestdout: bool) {
    let mut images = Vec::new();
    for buffer in bufferdatas {
        let mut buff = Cursor::new(Vec::new());
        PngEncoder::new(&mut buff)
            .write_image(
                &buffer.frame_mmap,
                buffer.frameformat.width,
                buffer.frameformat.height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        let image =
            image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();

        //let image = roate_image(&image, buffer.transform);
        let image = roate_image(
            &image,
            buffer.transform,
            buffer.realwidth,
            buffer.realheight,
        );
        images.push(image);
    }
    if usestdout {
        let mut buff = Cursor::new(Vec::new());
        use image::imageops::overlay;
        let mut image = images[0].clone();
        for aimage in images.iter().skip(1) {
            overlay(&mut image, aimage, 0, 0);
        }
        if let Err(_e) = image.write_to(&mut buff, image::ImageFormat::Png) {
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileCopyFailed")
                .body(&format!("File failed to copy: {_e}"))
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
            return;
        };
        let content = buff.get_ref();
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        if let Err(_e) = writer.write_all(content) {
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("PictureWriteToStdoutFailed")
                .body(&format!("Picture failed to write: {_e}"))
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        } else {
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("Screenshot")
                .body("Screenshot Succeed")
                .icon(SUCCESSED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        };
    } else {
        let file_name = format!(
            "{}-haruhui.png",
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        #[cfg(feature = "notify")]
        let file = SAVEPATH.join(&file_name);
        #[cfg(not(feature = "notify"))]
        let file = SAVEPATH.join(file_name);
        let filefullname = file.to_str().unwrap();
        use image::imageops::overlay;
        let mut image = images[0].clone();
        for aimage in images.iter().skip(1) {
            overlay(&mut image, aimage, 0, 0);
        }
        if image.save(&file).is_ok() {
            tracing::info!("Image saved to {}", filefullname);
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileSaved")
                .body(&format!("File saved to {}", filefullname))
                .icon(SUCCESSED_IMAGE)
                .timeout(TIMEOUT)
                .show();
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("Screenshot of screen")
                .body(&file_name)
                .icon(filefullname)
                .timeout(TIMEOUT)
                .show();
        } else {
            tracing::error!("Image failed saved to {}", filefullname);
            #[cfg(feature = "notify")]
            let _ = Notification::new()
                .summary("FileSavedFailed")
                .body(&format!("File failed saved to {}", filefullname))
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        };
    }
}
