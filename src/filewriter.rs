use image::{codecs::png::PngEncoder, ImageEncoder};
use image::{GenericImage, GenericImageView, ImageBuffer, Pixel, Primitive};
#[cfg(feature = "notify")]
use notify_rust::Notification;

use crate::constenv::SAVEPATH;
#[cfg(feature = "notify")]
use crate::constenv::{FAILED_IMAGE, SUCCESSED_IMAGE, TIMEOUT};
use crate::wlrbackend::BufferData;

use std::io::Write;
use std::io::{stdout, BufWriter, Cursor};
use std::time;

//use std::io::{stdout, BufWriter};
pub fn write_to_file(bufferdata: BufferData, usestdout: bool) {
    if usestdout {
        let mut buff = Cursor::new(Vec::new());
        if let Err(_e) = PngEncoder::new(&mut buff).write_image(
            &bufferdata.frame_mmap.unwrap(),
            bufferdata.width,
            bufferdata.height,
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

                    // TO SLOW, I think before should deirectroy write to wl-copy
                    // After can use command line under
                    //let image = image::load_from_memory_with_format(
                    //    buff.get_ref(),
                    //    image::ImageFormat::Png,
                    //)
                    //.unwrap();

                    //let _ = Notification::new()
                    //    .summary("Screenshot")
                    //    .body("Your Screenshot is")
                    //    .image_data(
                    //        notify_rust::Image::from_rgba(
                    //            image.width() as i32,
                    //            image.height() as i32,
                    //            image.as_rgba8().unwrap().to_vec(),
                    //        )
                    //        .unwrap(),
                    //    )
                    //    .timeout(TIMEOUT)
                    //    .show();
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
                &bufferdata.frame_mmap.unwrap(),
                bufferdata.width,
                bufferdata.height,
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

pub fn write_to_file_mutisource(bufferdatas: Vec<BufferData>, usestdout: bool) {
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
        let image =
            image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();
        let image = image::imageops::resize(
            &image,
            buffer.realwidth as u32,
            buffer.realheight as u32,
            image::imageops::FilterType::Gaussian,
        );
        images.push(image);
    }
    if usestdout {
        let mut buff = Cursor::new(Vec::new());
        let image = h_concat(&images);
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
            //#[cfg(feature = "notify")]
            //let _ = Notification::new()
            //    .summary("Screenshot")
            //    .body("Your Screenshot is")
            //    .image_data(
            //        notify_rust::Image::from_rgba(
            //            image.width() as i32,
            //            image.height() as i32,
            //            image.as_ref().to_vec(),
            //        )
            //        .unwrap(),
            //    )
            //    .timeout(TIMEOUT)
            //    .show();
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

        if h_concat(&images).save(&file).is_ok() {
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
