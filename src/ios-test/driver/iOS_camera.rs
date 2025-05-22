#[cfg(any(target_os = "ios", target_os = "macos"))]
use dispatch2::DispatchQueue;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use image::{Rgba, RgbaImage};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::__framework_prelude::NSObject;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::rc::Retained;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::{define_class, AllocAnyThread, DeclaredClass, Encoding, Message, RefEncode};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_av_foundation::{AVCaptureConnection, AVCaptureDeviceDiscoverySession, AVCaptureOutput, AVCaptureSession, AVCaptureSessionPresetMedium, AVCaptureVideoDataOutput, AVCaptureVideoDataOutputSampleBufferDelegate, AVMediaTypeVideo};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_av_foundation::{AVCaptureDeviceInput, AVCaptureDevicePosition};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_core_media::CMSampleBuffer;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_core_video::{kCVPixelBufferPixelFormatTypeKey, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferLockFlags};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_core_video::{kCVPixelFormatType_32BGRA, CVPixelBufferGetHeight, CVPixelBufferGetWidth};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_foundation::{NSArray, NSCopying, NSDictionary, NSNumber, NSString};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::slice::from_raw_parts;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::sync::Mutex;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::time::Duration;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::thread;

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Debug)]
pub enum CameraError {
    AccessDenied,
    Restricted,
    Unknown,
    WaitingForAccess,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Debug)]
pub struct ProcessorClass {
    pub last_frame: Mutex<Option<RgbaImage>>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
define_class!(

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `Processor` does not implement `Drop`.
    #[unsafe(super = NSObject)]
    #[ivars = ProcessorClass]
    #[derive(Debug)]
    struct Processor;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for Processor {}

    // SAFETY: `AVCaptureVideoDataOutputSampleBufferDelegate` has no safety requirements.
    unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for Processor {
        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        fn captureOutput_didOutputSampleBuffer_fromConnection(
            &self,
            _output: &AVCaptureOutput,
            sample_buffer: &CMSampleBuffer,
            _connection: &AVCaptureConnection,
        ) {
            println!("captureOutput called");

            let pixel_buffer = unsafe { CMSampleBuffer::image_buffer(sample_buffer) };

            if pixel_buffer.is_none() {
                eprintln!("CMSampleBufferGetImageBuffer returned None");
                return;
            }

            let pixel_buffer = pixel_buffer.unwrap();
            let height = unsafe { CVPixelBufferGetHeight(&pixel_buffer) };
            let width = unsafe { CVPixelBufferGetWidth(&pixel_buffer) };
            let bytes_per_row = unsafe { CVPixelBufferGetBytesPerRow(&pixel_buffer) };
            let size = bytes_per_row * height;

            println!(
                "Pixel buffer properties: height = {}, width = {}, bytes_per_row = {}, size = {}",
                height, width, bytes_per_row, size
            );

            use objc2_core_video::{CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress};

            let lock_result =
                unsafe { CVPixelBufferLockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0)) };
            if lock_result != 0 {
                eprintln!("Failed to lock base address of pixel buffer");
                return;
            }

            let base_address = unsafe { CVPixelBufferGetBaseAddress(&pixel_buffer) } as *const u8;

            if base_address.is_null() {
                eprintln!("Base address is null!");
                unsafe {
                    CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
                }
                return;
            }

            if size > isize::MAX as usize {
                eprintln!("Size ({}) exceeds the valid range for slices!", size);
                unsafe {
                    CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
                }
                return;
            }

            let slice = unsafe { from_raw_parts(base_address, size) };

            println!("Processing pixel data slice...");

            let mut image = RgbaImage::new(width as u32, height as u32);
            let mut pixels = image.pixels_mut();

            for y in 0..height {
                let row_start = y * bytes_per_row;
                for x in 0..width {
                    let src_index = row_start + x * 4;
                    if src_index + 3 >= slice.len() {
                        eprintln!("Source index {} out of range, skipping pixel", src_index);
                        continue;
                    }

                    let r = slice[src_index + 2];
                    let g = slice[src_index + 1];
                    let b = slice[src_index];
                    let a = slice[src_index + 3];

                    let pixel = pixels.next().unwrap();
                    *pixel = Rgba([r, g, b, a]);
                }
            }

            println!("Finished processing frame. Saving to last_frame...");

            *self.ivars().last_frame.lock().unwrap() = Some(image);

            println!("Frame saved successfully.");

            unsafe {
                CVPixelBufferUnlockBaseAddress(&pixel_buffer, CVPixelBufferLockFlags(0));
            }
        }
    }

);

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Processor {
    pub fn new() -> Retained<Self> {
        let this = Self::alloc();
        let this = this.set_ivars(ProcessorClass {
            last_frame: Mutex::new(None),
        });
        unsafe { objc2::msg_send![super(this), init] }
    }

    pub fn get_latest_frame(&self) -> Option<RgbaImage> {
        self.ivars().last_frame.lock().unwrap().clone()
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Debug)]
pub struct Camera {
    pub session: Retained<AVCaptureSession>,
    processor: Retained<Processor>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Camera {
    pub fn new() -> Self {
        unsafe {
            let camera = Camera {
                session: AVCaptureSession::new(),
                processor: Processor::new(),
            };
            camera.setup_camera();
            camera
        }
    }

    unsafe fn setup_camera(&self) {
        let device_types = NSArray::from_slice(&[objc2_av_foundation::AVCaptureDeviceTypeBuiltInWideAngleCamera]);

        let discovery_session = AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &device_types,
            AVMediaTypeVideo,
            AVCaptureDevicePosition::Front,
        );

        let devices = discovery_session.devices();

        let device = devices.into_iter().next().expect("No device at index 0");

        let input = AVCaptureDeviceInput::deviceInputWithDevice_error(&*device)
            .expect("Failed to create AVCaptureDeviceInput");

        self.session.beginConfiguration();

        self.session.setSessionPreset(AVCaptureSessionPresetMedium);

        if self.session.canAddInput(&*input) {
            self.session.addInput(&*input);
        }

        let output = AVCaptureVideoDataOutput::new();

        let pixel_format_value = NSNumber::new_u32(kCVPixelFormatType_32BGRA);

        let pixel_format_key: &NSString = &*(kCVPixelBufferPixelFormatTypeKey as *const _ as *const NSString);

        let video_settings = NSDictionary::from_slices(
            &[pixel_format_key],
            &[pixel_format_value.as_ref()],
        );

        output.setVideoSettings(Some(&video_settings));

        let queue = DispatchQueue::new("CameraQueue", None);

        output.setSampleBufferDelegate_queue(
            Some(ProtocolObject::from_ref(&*self.processor)),
            Some(&queue),
        );

        if self.session.canAddOutput(&*output) {
            self.session.addOutput(&*output);
        }

        self.session.commitConfiguration();
        self.session.startRunning();
    }

    pub fn get_latest_frame(&self) -> Option<RgbaImage> {
        let lock = self.processor.ivars().last_frame.lock().unwrap();
        if lock.is_some() {
            println!("Cloning frame from mutex.");
        }
        lock.clone()
    }
}
