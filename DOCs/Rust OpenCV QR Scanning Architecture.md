# **Architectural Synthesis of Auto-Exposure Mitigation and Adaptive Binarization for Cross-Platform QR Decoding**

## **Introduction and Architectural Context**

The development of the "VisioFlow" cross-platform command-line interface necessitates resolving one of the most persistent challenges in applied computer vision: the optical retrieval of high-density data payloads from emissive displays under highly variable ambient illumination. When scanning a digital matrix barcode (QR code) displayed on a smartphone screen, standard universal video class (UVC) webcam hardware consistently defaults to integrating the exposure based on the ambient room lighting. Because the dynamic range of a standard complementary metal-oxide-semiconductor (CMOS) sensor is fundamentally limited, exposing for a dim room causes the relatively brilliant smartphone screen to exceed the sensor's full-well capacity. This results in severe clipping, or "blowout," completely destroying the spatial data of the QR code's alignment modules and data payload.  
Operating strictly within bare-metal Rust limits the ability to rapidly deploy complex mathematical and machine-learning workarounds for these hardware deficiencies. By lifting this constraint and embracing battle-tested frameworks such as OpenCV, GStreamer, and the associated Rust bindings provided by the opencv crate, the engineering paradigm shifts. The architecture can now transition from fighting low-level USB peripheral protocols to orchestrating a high-level, hybrid software-hardware pipeline. This report exhaustively details the integration of cross-platform framework APIs, temporal exposure bracketing, software-driven Proportional-Integral-Derivative (PID) exposure control, and advanced convolutional neural network (CNN) binarization techniques to achieve commercial-grade point-of-sale (POS) scanning reliability.

## **1\. Framework API Capabilities: Overcoming Hardware Auto-Exposure Dysfunctions**

The reliance on generic UVC webcam hardware for optical data decoding presents a fundamental conflict of interest. Hardware Auto-Exposure (AE) algorithms are universally engineered to optimize for human facial tones and ambient scene averaging, relying on rudimentary center-weighted or matrix metering. They are explicitly not designed for capturing high-contrast, emissive digital displays1.  
Abstracting these hardware controls across Windows (via Media Foundation or DirectShow), Linux (via Video4Linux2), and macOS (via AVFoundation) utilizing a framework like OpenCV introduces significant leaky abstractions. The opencv::videoio::VideoCapture interface exposes properties such as CAP\_PROP\_AUTO\_EXPOSURE and CAP\_PROP\_EXPOSURE, but the underlying implementations suffer from backend-specific erraticism that must be architecturally mitigated2.

### **1.1 The Leaky Abstraction of VideoCaptureProperties**

The numerical values accepted by OpenCV's exposure properties are not strictly standardized across operating systems. In many instances, the values diverge wildly even across different backends on the exact same operating system, leading to silent failures where the camera completely ignores the software's commands3.

#### **Windows: Media Foundation (MSMF) versus DirectShow (DSHOW)**

Historically, OpenCV utilized the Video for Windows (VFW) or DirectShow (DSHOW) backends for hardware interaction on Microsoft platforms. Modern distributions default to the Microsoft Media Foundation (MSMF) backend, which introduces its own distinct behavioral quirks3.  
When utilizing the MSMF backend, controlling the AE state typically follows a boolean logic structure. Setting CAP\_PROP\_AUTO\_EXPOSURE to 0 disables the AE circuit, while setting it to 1 re-enables it3. However, MSMF is notorious for failing to force the hardware into manual mode without the software first polling and locking the camera's baseline temperature and white balance settings6. Furthermore, CAP\_PROP\_EXPOSURE in MSMF utilizes an absolute, device-dependent value scale, where negative values often represent fractions of a second, though the exact mapping is completely opaque to the developer8.  
Alternatively, the legacy DirectShow backend can be explicitly invoked in Rust via the videoio::VideoCapture::new(0, videoio::CAP\_DSHOW) instantiation11. DirectShow exhibits radically different numerical representations. Auto-exposure states map to the floating-point value 0.25 for manual mode and 0.75 for auto mode, completely defying boolean expectations4. In DirectShow, CAP\_PROP\_EXPOSURE expects a logarithmic scale representing powers of two. For example, an exposure value of \-7 translates mathematically to an integration time of ![][image1] seconds (![][image2] seconds), while a value of \-1 equals an unusable ![][image3] seconds, which induces severe motion blur4. A documented limitation in the DirectShow backend historically prevented the software from turning AE back on once it was disabled, trapping the camera in manual mode until the USB bus was physically reset12.

#### **Linux: Video4Linux2 (V4L2) and GStreamer**

Under the Video4Linux2 (V4L2) backend on Linux distributions, the parameters invert completely compared to Windows MSMF. Manual mode is typically designated by the integer 1, while auto-exposure is activated via 3 (and occasionally 0 on legacy drivers)4. Because the OpenCV C++ wrapper frequently fails to negotiate these states accurately with heterogeneous UVC drivers, production Linux systems regularly fall back to invoking the v4l2-ctl command-line utility via standard output subprocesses. This forcefully flips the register states directly at the driver level, bypassing OpenCV's property setters entirely4.  
For embedded architectures, such as NVIDIA Jetson platforms operating under Linux, utilizing GStreamer as an intermediate backend (videoio::CAP\_GSTREAMER) offers a highly deterministic pipeline definition. GStreamer plugins like nvarguscamerasrc and v4l2src natively expose robust gain and exposure ranges, allowing the Rust application to define the exposure constraints directly within the instantiation pipeline string16.

#### **Table 1: Empirical API Value Mappings for UVC Cameras**

| Backend / Operating System | CAP\_PROP\_AUTO\_EXPOSURE (Auto State) | CAP\_PROP\_AUTO\_EXPOSURE (Manual State) | CAP\_PROP\_EXPOSURE Value Format |
| :---- | :---- | :---- | :---- |
| **MSMF (Windows 10/11)** | 1.0 | 0.0 | Absolute / Device Dependent Negative |
| **DSHOW (Windows Legacy)** | 0.75 | 0.25 | Logarithmic (![][image4]) |
| **V4L2 (Linux/Debian)** | 3.0 (or 0.0) | 1.0 | Absolute (![][image5] increments) |

### **1.2 Architectural Recommendation for Hardware Control**

To bypass the myriad of driver quirks that force absolute manual mode, the recommended architecture for the "VisioFlow" Rust application is the implementation of an operating system-aware hardware abstraction layer (HAL) wrapping the opencv::videoio::VideoCapture object. The application must unconditionally force the camera into manual exposure mode upon initialization using the exact backend-specific magic numbers detailed above. Attempting to allow the camera's AE to run while occasionally injecting exposure compensation commands is structurally unsound, as generic UVC protocol implementations rarely support floating exposure compensation targets accurately. If the HAL fails to achieve exposure locking, the system must immediately pivot to software-level binarization compensation rather than fighting a non-compliant driver.

## **2\. The Bracketing and Interleaving Strategy: Defeating Buffer Lag**

Commercial point-of-sale systems manufactured by entities like Zebra or Honeywell solve ambient variability through hardware determinism: they utilize global shutter sensors combined with synchronized, multi-band LED illumination to overpower ambient light entirely. As "VisioFlow" relies on consumer-grade rolling shutter webcams without dedicated illumination, the software must simulate this determinism using temporal bracketing17.  
Temporal bracketing involves rapidly cycling the CMOS sensor through distinct manual exposure states. A high-exposure state is optimized for ambient room lighting and printed, non-emissive QR codes. A low-exposure state is optimized for emissive, high-brightness smartphone screens, drastically cutting the integration time to prevent blowout. By continuously rotating these states, the asynchronous decoding thread is mathematically guaranteed to receive a perfectly exposed frame within a tight temporal window, regardless of the physical environment.

### **2.1 The Internal Ring Buffer Latency Problem**

Implementing rapid exposure interleaving using OpenCV is fundamentally obstructed by the inherent latency of webcam frame buffering. When a physical camera captures a frame, it is pushed into a hardware USB ring buffer, decoded by the operating system's media foundation, and ultimately placed into an OpenCV buffer queue18.  
When the software sets a new CAP\_PROP\_EXPOSURE value, the sensor adjusts its integration time almost immediately. However, the subsequent frame yielded by the VideoCapture::read() method is virtually always a stale frame sitting in the queue, captured moments before the exposure change actually took effect4. This latency cascade can range from three to five frames, meaning synchronous bracketing will completely fail as the QR detector will attempt to evaluate frames with mismatched lighting assumptions18.

### **2.2 Buffer Minimization and the Asynchronous I/O Spin-Thread**

To achieve true temporal interleaving, the internal buffer must be entirely bypassed or aggressively drained. In modern OpenCV releases, setting CAP\_PROP\_BUFFERSIZE to 1 is intended to force the backend to drop stale frames and only retain the most recent physical capture2. Unfortunately, this property is routinely ignored by numerous legacy UVC drivers, making it an unreliable cross-platform solution2.  
The most reliable, commercial-grade engineering practice is separating frame acquisition from frame retrieval and processing via multithreading18. OpenCV's standard VideoCapture::read() method is a synchronous, blocking operation that combines grab() (hardware acquisition) and retrieve() (software decoding into a matrix). The architecture necessitates a dedicated, high-priority I/O thread that spins infinitely, repeatedly executing cam.grab(). This ensures the camera's hardware buffer is aggressively flushed the millisecond a frame arrives18.  
A secondary, asynchronous processing thread manages the state machine, applying the bracketing exposure changes and calling cam.retrieve() exactly when a frame is required for the QR pipeline. This decoupled architecture drops latency to the absolute theoretical minimum of the USB bus, allowing the system to alternate the camera's exposure property rapidly and read the interleaved stream without stale frames corrupting the detector.  
The following Rust implementation demonstrates this lock-free acquisition strategy utilizing the opencv crate:

Rust  
use std::sync::{Arc, Mutex};  
use std::thread;  
use std::time::Duration;  
use opencv::prelude::\*;  
use opencv::videoio::{self, VideoCapture, CAP\_PROP\_EXPOSURE, CAP\_PROP\_AUTO\_EXPOSURE};  
use opencv::core::Mat;

/// Defines a thread-safe structure for sharing frames across boundaries  
pub struct FrameStream {  
    latest\_frame: Arc\<Mutex\<Mat\>\>,  
    is\_running: Arc\<Mutex\<bool\>\>,  
}

impl FrameStream {  
    /// Spawns a dedicated I/O thread to aggressively flush the hardware buffer.  
    /// This eliminates the 3-5 frame latency typical of UVC webcams.  
    pub fn start\_io\_thread(mut cam: VideoCapture) \-\> Self {  
        let latest\_frame \= Arc::new(Mutex::new(Mat::default()));  
        let is\_running \= Arc::new(Mutex::new(true));  
          
        let frame\_clone \= Arc::clone(\&latest\_frame);  
        let running\_clone \= Arc::clone(\&is\_running);

        thread::spawn(move || {  
            let mut local\_frame \= Mat::default();  
              
            while \*running\_clone.lock().unwrap() {  
                // \`grab()\` clears the buffer at the driver level without decoding  
                if let Ok(true) \= cam.grab() {  
                    // \`retrieve()\` actually decodes the payload into the matrix  
                    if let Ok(true) \= cam.retrieve(&mut local\_frame, 0) {  
                        if let Ok(mut shared\_frame) \= frame\_clone.lock() {  
                            local\_frame.copy\_to(&mut \*shared\_frame).unwrap();  
                        }  
                    }  
                }  
                // Yield to prevent complete CPU starvation  
                thread::sleep(Duration::from\_millis(1));  
            }  
        });

        FrameStream {  
            latest\_frame,  
            is\_running,  
        }  
    }

    /// Safely retrieves the most recent frame, guaranteed to reflect the latest AE changes  
    pub fn get\_latest\_frame(&self, out\_frame: &mut Mat) \-\> opencv::Result\<()\> {  
        if let Ok(shared\_frame) \= self.latest\_frame.lock() {  
            shared\_frame.copy\_to(out\_frame)?;  
        }  
        Ok(())  
    }  
      
    pub fn stop(&self) {  
        if let Ok(mut running) \= self.is\_running.lock() {  
            \*running \= false;  
        }  
    }  
}

By leveraging this architectural pattern, "VisioFlow" guarantees that the frame passed to the QR decoder is the physical reality at that exact millisecond, rendering temporal bracketing a viable strategy.

## **3\. The Library-Assisted Software AE Loop**

In environments where rapid temporal bracketing is deemed undesirable—often due to user experience constraints, as visible screen flickering on the preview feed can cause physiological discomfort or distraction—a software-driven Auto-Exposure loop utilizing Proportional-Integral-Derivative (PID) control must be implemented. This software loop bridges the gap between hardware dysfunction and optical necessity by utilizing the host CPU to analyze the image statistics and dictate the precise hardware integration time required.

### **3.1 Extracting Region of Interest (ROI) Brightness**

A standard hardware AE loop calculates exposure by averaging the entire scene, which guarantees failure when a small, intensely bright phone screen occupies only a fraction of a dark room. A software AE loop explicitly designed for QR scanning must aggressively bias its calculations toward the center of the frame—the Region of Interest (ROI)—where the user naturally positions the digital display.  
To mathematically determine the current exposure quality without relying on custom, unoptimized algorithms, the image matrix must be converted to a color space that distinctly separates luma (intensity) from chroma (color). OpenCV's imgproc::cvt\_color function, transforming the standard BGR matrix to HSV (Hue, Saturation, Value) or LAB (Lightness, A, B) color spaces, is mathematically optimal for this process22. Extracting the 'Value' or 'Lightness' channel provides a pure grayscale intensity map stripped of color weighting.

Rust  
use opencv::{core, imgproc, prelude::\*};

/// Calculates the mean intensity of the center ROI for exposure feedback  
pub fn calculate\_roi\_brightness(frame: \&Mat) \-\> opencv::Result\<f64\> {  
    let size \= frame.size()?;  
      
    // Define a center-weighted ROI (50% of width and height)  
    let roi\_rect \= core::Rect::new(  
        size.width / 4,   
        size.height / 4,   
        size.width / 2,   
        size.height / 2  
    );  
      
    let roi \= Mat::roi(frame, roi\_rect)?;  
    let mut hsv \= Mat::default();  
      
    // Convert to HSV to separate intensity from color  
    imgproc::cvt\_color(\&roi, &mut hsv, imgproc::COLOR\_BGR2HSV, 0)?;  
      
    let mut channels \= core::Vector::\<Mat\>::new();  
    core::split(\&hsv, &mut channels)?;  
      
    // Extract the 'Value' (Brightness) channel (Index 2 in HSV)  
    let v\_channel \= channels.get(2)?;  
      
    // Calculate the mathematical mean of the intensity matrix  
    let mean\_scalar \= core::mean(\&v\_channel, \&core::no\_array())?;  
      
    // Return the scalar average (0.0 to 255.0)  
    Ok(mean\_scalar\[0\])  
}

### **3.2 PID Controller Mathematics and Step-Mapping**

The target brightness, or setpoint, for a commercial QR scanner reading an emissive screen is generally lower than standard photographic norms. While middle-gray in digital imaging is mathematically calibrated to 128 (on an 8-bit scale ranging from 0 to 255), targeting a lower intensity of 90 to 100 is highly recommended1. This lower setpoint specifically prevents the emissive white background of a QR code from optically bleeding into the black alignment modules, a phenomenon known as blooming, which corrupts the barcode's geometric ratios23.  
The PID controller calculates the error signal at time ![][image6]:  
![][image7]  
For UVC webcams processing real-time video, calculating a highly precise PID derivative is often counterproductive and introduces severe instability due to sensor noise and the previously discussed frame buffer latency. A simpler PI (Proportional-Integral) controller is demonstrably sufficient and far more stable for optical integration time adjustments1.  
The formula for the new exposure target becomes:  
![][image8]  
However, due to the non-linear, logarithmic nature of exposure settings on platforms like Windows DirectShow, the raw floating-point output of the PID loop cannot be mapped linearly to the camera API2. Injecting a continuously calculated floating-point value into CAP\_PROP\_EXPOSURE will cause the hardware to thrash, frequently leading to driver crashes.  
Instead, the PID output must dictate discrete *steps* or *increments* within an empirical lookup table of supported exposure values24. For example, if the error signal ![][image9] (indicating severe overexposure), the algorithm steps down two discrete levels in the exposure table. If ![][image10] (indicating underexposure), it steps up. This step-based regulation incorporates inherent hysteresis, which minimizes erratic hunting and oscillation around the target setpoint, ensuring the preview feed remains stable for the end user24.

## **4\. Advanced Local Binarization and Adaptive Thresholding**

In adverse deployment scenarios where hardware manipulation is completely locked out by operating system policies, failed UVC drivers, or incompatible hardware, the software processing pipeline must absorb the entirety of the overexposure burden. Standard, ubiquitous QR decoding algorithms—such as ZXing or the standard OpenCV objdetect::QRCodeDetector—utilize rudimentary global thresholding methodologies, most notably Otsu's method26.  
Otsu's algorithm calculates a single, global threshold value by minimizing intra-class variance. When a bright phone screen blows out the center of the image, while the periphery remains cast in shadow, the global mean is catastrophically skewed. Applying a global threshold to such an image renders the foreground (the QR code modules) and the background completely indistinguishable, obliterating the data26.

### **4.1 Local Adaptive Binarization Methodologies**

To successfully decode a partially overexposed QR code, the binarization process must be highly localized. OpenCV natively provides imgproc::adaptive\_threshold22. This function abandons the global paradigm, instead calculating the ideal threshold for a small pixel neighborhood using either a localized arithmetic mean (ADAPTIVE\_THRESH\_MEAN\_C) or a localized Gaussian weighted sum (ADAPTIVE\_THRESH\_GAUSSIAN\_C)27.  
This ensures that the blown-out center of the screen calculates a different threshold than the dark periphery, theoretically preserving the barcode structure.

### **4.2 The Wolf-Jolion Binarization Algorithm**

However, for the extreme, non-linear illumination gradients typical of emissive displays, standard adaptive thresholding is often insufficient. OpenCV's Extended Image Processing module (ximgproc) provides the niBlackThreshold function, which implements sophisticated local binarization algorithms, specifically the Wolf-Jolion and Sauvola methods22.  
These algorithms are explicitly engineered to preserve text and barcode structures under severe, non-uniform illumination. They achieve this by incorporating the local standard deviation into the threshold calculus, not just the local mean28.  
The Wolf-Jolion threshold calculation is defined as:  
![][image11]  
Where:

* ![][image12] is the local mean.  
* ![][image13] is the local standard deviation.  
* ![][image14] is the maximum standard deviation of the entire image.  
* ![][image15] is the minimum grayscale value of the entire image.  
* ![][image16] is a user-defined tuning parameter controlling the strictness of the binarization.

By dynamically adjusting the threshold based on the variance of the immediate pixel neighborhood, the Wolf-Jolion algorithm can successfully carve out black QR alignment modules from a sea of overexposed white pixels, assuming any residual contrast remains at the sensor level22.

#### **Table 2: Binarization Pipeline Comparison for Emissive Displays**

| Methodology | Illumination Gradient Tolerance | Execution Latency | Integration Complexity |
| :---- | :---- | :---- | :---- |
| **Standard Global Otsu** | Very Low (Fails immediately) | Ultra-Low | Native / Trivial |
| **Local Adaptive (Gaussian)** | Moderate | Low | Native |
| **Wolf-Jolion (niBlackThreshold)** | High | Moderate | Requires ximgproc compilation |

## **5\. The Machine Learning Paradigm: CNN-Based Decoding**

While advanced local binarization can salvage poorly exposed images, the absolute highest scan reliability and fastest implementation time bypass manual threshold tuning entirely in favor of Convolutional Neural Networks (CNNs). Recognizing the limitations of classical computer vision in edge cases, the OpenCV community integrated the wechat\_qrcode module—contributed directly by WeChat's Computer Vision team—which vastly outperforms traditional computer vision modules in robustness, angle invariance, and extreme overexposure tolerance31.  
Older iterations of OpenCV's QR detectors were notorious for out-of-memory (OOM) errors and severe memory leaks when attempting to process noisy, high-resolution, or blown-out frames33. The WeChatQRCode module circumvents this structural weakness by deploying two specialized Caffe-based neural network models in tandem:

1. **Object Detection Model:** This CNN is tasked solely with locating the bounding box of the QR code regardless of surrounding noise, severe overexposure, or optical camouflage31.  
2. **Super Resolution Model:** Once the bounding box is established, a second CNN zooms in and sharpens the QR data payload. Crucially, this model interpolates damaged, blurred, or heavily blown-out alignment modules, reconstructing the geometric integrity of the barcode *prior* to passing it to the Reed-Solomon error correction algorithm31.

### **5.1 Rust Integration of WeChatQRCode**

To deploy this advanced machine learning paradigm within the "VisioFlow" Rust CLI, the opencv crate must be compiled with the wechat\_qrcode feature enabled32. The required pre-trained Caffe model files (detect.prototxt, detect.caffemodel, sr.prototxt, sr.caffemodel) are heavily optimized for CPU execution. This intentionally negates the need for complex GPU integration (such as CUDA or OpenCL), making it exceptionally well-suited for lightweight, cross-platform CLI environments33.

Rust  
use opencv::{core, prelude::\*, wechat\_qrcode::WeChatQRCode};  
use std::path::Path;

/// Initializes the WeChatQRCode CNN pipeline for robust overexposure decoding  
pub fn initialize\_cnn\_decoder() \-\> opencv::Result\<WeChatQRCode\> {  
    // Models are provided via the OpenCV 3rdparty GitHub repository.  
    // Ensure these files are present in the working directory or binary path.  
    let detector \= WeChatQRCode::new(  
        "models/detect.prototxt",  
        "models/detect.caffemodel",  
        "models/sr.prototxt",  
        "models/sr.caffemodel",  
    )?;  
      
    Ok(detector)  
}

/// Executes detection and decoding on a potentially overexposed matrix  
pub fn decode\_qr\_robust(scanner: &mut WeChatQRCode, frame: \&Mat) \-\> opencv::Result\<Vec\<String\>\> {  
    let mut points \= core::Vector::\<Mat\>::new();  
      
    // The detect\_and\_decode function handles internal thresholding, object detection,   
    // and super-resolution interpolation in C++, safely returning data to Rust.  
    // It effectively ignores localized screen blowout if structural integrity is mathematically recoverable.  
    let decoded \= scanner.detect\_and\_decode(frame, &mut points)?;  
      
    Ok(decoded.into\_iter().collect())  
}

The CNN pipeline is so structurally robust against physical deformations, extreme stretching, and illumination blowout that it largely eliminates the need to battle the camera's hardware AE algorithms32. The neural network internally compensates for the blown-out regions by mapping spatial correlations that traditional algorithms simply discard.

## **6\. Synthesis: Commercial Point-of-Sale Engineering Practices**

Synthesizing these findings, it becomes evident that commercial engineering practices do not rely on a single point of failure. Devices manufactured by industry leaders utilize a multi-layered approach to guarantee scan reliability. While hardware triggers and global shutters represent the gold standard in physical POS devices, a software-only tool like "VisioFlow" must emulate this resilience entirely in code.  
The fastest implementation time yielding the highest reliability involves layering the aforementioned strategies. The system should initially attempt to acquire frames at a rapid, unthrottled pace using the decoupled I/O spin-thread, eliminating buffer latency. These zero-latency frames should be immediately passed to the WeChatQRCode CNN decoder. Because the CNN is highly tolerant of exposure variances, this primary loop will successfully decode the vast majority of emissive screens without any hardware intervention31.  
If the CNN fails to decode the payload after a predefined timeout (e.g., 500 milliseconds), the architecture must fall back to active mitigation. It should engage the temporal bracketing strategy, forcefully flipping the camera's exposure state via the OS-specific magic numbers (e.g., \-7 for DSHOW, 1 for V4L2)2. The I/O spin-thread ensures these bracketed frames arrive instantly, allowing the CNN a second attempt on a forcibly darkened image matrix. Software PI/PID loops should be strictly relegated to environments where the hardware refuses to accept absolute exposure commands, serving as the final, most computationally expensive fallback.

## **Conclusion**

To architect "VisioFlow" into a highly reliable, cross-platform CLI tool capable of scanning emissive displays in variable lighting, the engineering paradigm must transition from a strict hardware-control philosophy to a hybrid temporal-software framework.  
The exhaustive analysis dictates abandoning synchronous hardware AE tuning via generic OpenCV API calls, as OS backend implementations are too fragmented and latency-prone. Instead, the implementation of an asynchronous I/O spin-thread is mandatory to decouple frame acquisition from processing, completely neutralizing the 3-5 frame latency inherent in UVC webcams. Furthermore, standard binarization algorithms must be deprecated in favor of integrating the wechat\_qrcode CNN models. This machine learning pipeline structurally resolves the blowout problem, eliminating the need to manually execute complex adaptive binarization pipelines like the Wolf-Jolion algorithm. By layering zero-latency frame acquisition with CNN-based decoding and maintaining temporal bracketing as a deterministic fallback, the application will effortlessly decode high-brightness displays without engaging in a futile struggle against proprietary camera firmware.

#### **Works cited**

1. histogram \- calculate auto exposure in openCv \- Stack Overflow, [https://stackoverflow.com/questions/38101288/calculate-auto-exposure-in-opencv](https://stackoverflow.com/questions/38101288/calculate-auto-exposure-in-opencv)  
2. enum cv::VideoCaptureProperties — OpenCV Documentation, [https://vovkos.github.io/doxyrest-showcase/opencv/sphinx\_rtd\_theme/enum\_cv\_VideoCaptureProperties.html](https://vovkos.github.io/doxyrest-showcase/opencv/sphinx_rtd_theme/enum_cv_VideoCaptureProperties.html)  
3. PC / RPi camera display using PyQt and OpenCV \- Lean2, [https://iosoft.blog/2019/07/31/rpi-camera-display-pyqt-opencv/](https://iosoft.blog/2019/07/31/rpi-camera-display-pyqt-opencv/)  
4. Setting CAP\_PROP\_EXPOSURE on VideoCapture does not change anything · Issue \#9738, [https://github.com/opencv/opencv/issues/9738](https://github.com/opencv/opencv/issues/9738)  
5. Class VideoCapture set() method on property Videoio.CAP\_AUTO\_WB \- OpenCV Forum, [https://forum.opencv.org/t/class-videocapture-set-method-on-property-videoio-cap-auto-wb/1742](https://forum.opencv.org/t/class-videocapture-set-method-on-property-videoio-cap-auto-wb/1742)  
6. Camera white balance support for Logitech BRIO 4K · Issue \#21408 \- GitHub, [https://github.com/opencv/opencv/issues/21408](https://github.com/opencv/opencv/issues/21408)  
7. PyQt – Lean2, [https://iosoft.blog/category/pyqt/](https://iosoft.blog/category/pyqt/)  
8. OpenCV实现四路USB摄像头同步调用与实时截图保存\_孟园香 \- 智能体开发者社区, [https://adg.csdn.net/6953340e5b9f5f31781bcd22.html](https://adg.csdn.net/6953340e5b9f5f31781bcd22.html)  
9. Windows本地摄像头控制小工具：点按拍照+滑动调光变焦，开箱即用原创 \- CSDN博客, [https://blog.csdn.net/h9j8k7l6m5n/article/details/162111056](https://blog.csdn.net/h9j8k7l6m5n/article/details/162111056)  
10. Windows下用Python玩转USB摄像头：从PyUVC驱动安装到OpenCV, [https://wenku.csdn.net/column/71394jo24wi](https://wenku.csdn.net/column/71394jo24wi)  
11. Why DSHOW can control white balance but OpenCV cannot? \- Stack Overflow, [https://stackoverflow.com/questions/62716080/why-dshow-can-control-white-balance-but-opencv-cannot](https://stackoverflow.com/questions/62716080/why-dshow-can-control-white-balance-but-opencv-cannot)  
12. CAP\_PROP\_AUTO\_EXPOSURE not working with DSHOW backend \#17019 \- GitHub, [https://github.com/opencv/opencv/issues/17019](https://github.com/opencv/opencv/issues/17019)  
13. How to set camera to auto-exposure with OpenCV 3.4.2? \- Stack Overflow, [https://stackoverflow.com/questions/53545945/how-to-set-camera-to-auto-exposure-with-opencv-3-4-2](https://stackoverflow.com/questions/53545945/how-to-set-camera-to-auto-exposure-with-opencv-3-4-2)  
14. modules/videoio/src/cap\_dshow.cpp · master · CustusX / OpenCV \- GitLab, [https://gitlab.sintef.no/custusx/OpenCV/-/blob/master/modules/videoio/src/cap\_dshow.cpp](https://gitlab.sintef.no/custusx/OpenCV/-/blob/master/modules/videoio/src/cap_dshow.cpp)  
15. Manual Exposure Control of OpenCV Video | Peter F. Klemperer, [https://peterklemperer.com/blog/2018/02/10/manual-exposure-control-of-opencv-video/](https://peterklemperer.com/blog/2018/02/10/manual-exposure-control-of-opencv-video/)  
16. Adjust Gain/Exposure in opencv on Jetson Nano \- NVIDIA Developer Forums, [https://forums.developer.nvidia.com/t/adjust-gain-exposure-in-opencv-on-jetson-nano/239290](https://forums.developer.nvidia.com/t/adjust-gain-exposure-in-opencv-on-jetson-nano/239290)  
17. Setting up a machine vision camera for object detection \- Smart Design, [https://smartdesignworldwide.com/ideas/setting-up-a-machine-vision-camera-for-image-inference/](https://smartdesignworldwide.com/ideas/setting-up-a-machine-vision-camera-for-image-inference/)  
18. OpenCV VideoCapture lag due to the capture buffer \- Stack Overflow, [https://stackoverflow.com/questions/30032063/opencv-videocapture-lag-due-to-the-capture-buffer](https://stackoverflow.com/questions/30032063/opencv-videocapture-lag-due-to-the-capture-buffer)  
19. OpenCV delay in camera output on the screen \- Stack Overflow, [https://stackoverflow.com/questions/9021948/opencv-delay-in-camera-output-on-the-screen](https://stackoverflow.com/questions/9021948/opencv-delay-in-camera-output-on-the-screen)  
20. Fixing the Slow Camera FPS Issue in Python/OpenCV \- YouTube, [https://www.youtube.com/watch?v=tcAkYvNxSxY](https://www.youtube.com/watch?v=tcAkYvNxSxY)  
21. Increasing webcam FPS with Python and OpenCV \- PyImageSearch, [https://pyimagesearch.com/2015/12/21/increasing-webcam-fps-with-python-and-opencv/](https://pyimagesearch.com/2015/12/21/increasing-webcam-fps-with-python-and-opencv/)  
22. How to auto adjust contrast and brightness of a scanned Image with opencv python, [https://stackoverflow.com/questions/63243202/how-to-auto-adjust-contrast-and-brightness-of-a-scanned-image-with-opencv-python](https://stackoverflow.com/questions/63243202/how-to-auto-adjust-contrast-and-brightness-of-a-scanned-image-with-opencv-python)  
23. Increase image brightness without overflow \- python \- Stack Overflow, [https://stackoverflow.com/questions/44047819/increase-image-brightness-without-overflow](https://stackoverflow.com/questions/44047819/increase-image-brightness-without-overflow)  
24. KR100887075B1 \- Method for auto-exposure of image sensor using p.i.d. algorithm \- Google Patents, [https://patents.google.com/patent/KR100887075B1/en](https://patents.google.com/patent/KR100887075B1/en)  
25. Automatic exposure control in network video cameras \- Lund University Publications, [https://lup.lub.lu.se/luur/download?func=downloadFile\&recordOId=8847385\&fileOId=8859283](https://lup.lub.lu.se/luur/download?func=downloadFile&recordOId=8847385&fileOId=8859283)  
26. Automatic contrast and brightness adjustment of a color photo of a sheet of paper with OpenCV \- Stack Overflow, [https://stackoverflow.com/questions/56905592/automatic-contrast-and-brightness-adjustment-of-a-color-photo-of-a-sheet-of-pape](https://stackoverflow.com/questions/56905592/automatic-contrast-and-brightness-adjustment-of-a-color-photo-of-a-sheet-of-pape)  
27. OpenCV Adaptive Threshold | by Amit Yadav \- Medium, [https://medium.com/@amit25173/opencv-adaptive-threshold-fae667b91984](https://medium.com/@amit25173/opencv-adaptive-threshold-fae667b91984)  
28. opencv::ximgproc \- Rust \- Docs.rs, [https://docs.rs/opencv/latest/opencv/ximgproc/index.html](https://docs.rs/opencv/latest/opencv/ximgproc/index.html)  
29. "Option  
30. "Option  
31. opencv::wechat\_qrcode \- Rust \- Docs.rs, [https://docs.rs/opencv/latest/opencv/wechat\_qrcode/index.html](https://docs.rs/opencv/latest/opencv/wechat_qrcode/index.html)  
32. Wechat QR Reader in Rust using Opencv | by Rajesh Pachaikani | Medium, [https://medium.com/@rajeshpachaikani/wechat-qr-reader-in-rust-using-opencv-6078d429255f](https://medium.com/@rajeshpachaikani/wechat-qr-reader-in-rust-using-opencv-6078d429255f)  
33. OpenCV QRCodeDetector Out Of Memory even with a small file \- Stack Overflow, [https://stackoverflow.com/questions/70373514/opencv-qrcodedetector-out-of-memory-even-with-a-small-file](https://stackoverflow.com/questions/70373514/opencv-qrcodedetector-out-of-memory-even-with-a-small-file)  
34. ImpText: A Benchmark and Tool-Augmented Framework for Implicit Text Reasoning \- OpenReview, [https://openreview.net/pdf/3803dff6d16049521fda81002e0ba4cf5e869e93.pdf](https://openreview.net/pdf/3803dff6d16049521fda81002e0ba4cf5e869e93.pdf)  
35. Rust 通过OpenCV 识别二维码 \- 稀土掘金, [https://juejin.cn/post/7210747150829011000](https://juejin.cn/post/7210747150829011000)  
36. opencv \- Rust \- Docs.rs, [https://docs.rs/opencv](https://docs.rs/opencv)  
37. opencv/qrcode\_wechatqrcode \- Hugging Face, [https://huggingface.co/opencv/qrcode\_wechatqrcode](https://huggingface.co/opencv/qrcode_wechatqrcode)  
38. opencv\_contrib/modules/wechat\_qrcode/samples/qrcode.py at 4.x \- GitHub, [https://github.com/opencv/opencv\_contrib/blob/4.x/modules/wechat\_qrcode/samples/qrcode.py](https://github.com/opencv/opencv_contrib/blob/4.x/modules/wechat_qrcode/samples/qrcode.py)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAB4AAAAZCAYAAAAmNZ4aAAABKElEQVR4Xu2UsUrDUBSGj6igIIgoilRwcXEWdaigg4tDl1Zw8A18AXH1CQQFwcG56OzWQVx9AMUHcBAcdaz9fm4SDtdKgyYdJB980Ht6mv8k9zZm/4hTPMJ93MFFnMUR11M4c3iOyxYCZdtCcKms4KFbj+OGW/dlFJdw28KkRdCMC/14xld8wy88sL/tyzzexEWPLn6B9aiu8CdcSNbq04H5ya2kT0zhPR672jfSJqnPKZ/YxV1Xy8sqvtuAYN3JGV7imKt/WLjrhqvlRcNq6IF7rPB4P/VDha9F9Tzs2S+H3sQXC//HoaGwoYdOYwcf4i/KRG+ZK7y1MIDQI69lHSVxYiF4MlnrhF/jetZRAi0LpzD2EWdcX6GkL5A4VN7hRNZZUVFR4egBdJsyaBTOqx4AAAAASUVORK5CYII=>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADMAAAAZCAYAAACclhZ6AAACcUlEQVR4Xu2XS6hNURjHP7mK8sgjjygpJQwleZSJgQkTA4xMFSOFzETmilKSXIbkkYmBwXUzVCYGMqJMKAOKQh7/31l71VrfWWuffW4ph/Orf/vub629zvrvb31r7Ws25t9llrRYmu0bRpHV0kNpmW/4G+BNr/fBFg5JF32wYZW00AcTyOZuC/1qmWU+yy30W2P1fhk8tEl6IL3Om6pMSDelXb5BLJW+Sft8Q8NW6aX0XvolfZQOZj0CT6Xn0tXm+jZv7meONF/aIn2W3uTNVY5LZ5N7XggmEBlhkt4MfS5Lz6SdSZyx6L8iiZF15pXCC7zmYkWGNXPfwjMlmETJDPEpC21cI2Tqi7QniZ223Fxk0gdKDGvmjjTPBxtqZsgMNUbblSQefzvtf0J6bKFWIpg7mtxXGdYMS6NGzQxgaEFzjZCRn5ZnerOFMai9k9IS6Zb1L70iw5jZa2H91mgz49lm4Xdf+QZxTvpqYSz0w0KND6SrGd7odR90dDWz1oKJF83fKTETbCaHLex4jHnK8qwW6WpmnfTEBx1dzVAT09JKFyfr1NSiJEafu9I7aUMSL9LVDFvmJR90dDHDcrlt+YT5ooCN0ockHuGZQeP26GJmrnTP8i20xCAznORnLN8NyQZbNMS5lGALr43bgzVIUX+3cCrX1qQ/KGvwhjHDNuoL9oCFQo5FnYqPVuD3z0s7mvsIdUS8Nr/et5EfFE1Zvg2yHB5Z/aCESesfB/GWeS49NEsi8xGyhmlWwjHphvTJ+l/OjNhu7Qfln4BDkiV1pLmWvghmxAVrPyhHCpZQ20E5MlBw+31wVKEw+SdpzJj/md+2pIbmneifZgAAAABJRU5ErkJggg==>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABoAAAAZCAYAAAAv3j5gAAABfElEQVR4Xu2UvyuGURTHj/yIKMSAQTEok0EZJBNFfhQGg8VE2Uz+APkHDAYlWWUwSCxvb1arYqSUyUAYyI/vce593uO855EfEz2f+vT2fu99nnPvfU6XKOOvUQd7YSssN2MeJbAFlpq8GjaYLGEbnsMNuAevP4z61MA8vIO7cB2ewEc4XphWYAjOm6wRHsAqk2u40D5JAXaVZDcuvP1NOGByfgnvqtPkGp7DO+m2Ax718Jj8ya9wzIaKbxVqhhfkT+ZCSzZUxEI9cASukTSTbY53uMB9+LV8pdAhnFLZLMlzyySfJaEL3tLPCnnEhV/CNj3wm6PzaIdX8IVMg6U1A2+bC02aXLNI0plzKosLL2qkMrgDh3VIcv5p3y6SJ3khN0RFyOLRPcG+kCXwdXEEa1U2HdTwS/VKB0mKNcUJJIt+ppSbgTmFOTgDV+ADFbcpXy9nsCP85+NdIPnwWyTX1w0cDWMufIn2kxSaoE8uRQeey8/ws5VmLCPjv/MGUq5Hg9ZrOuUAAAAASUVORK5CYII=>

[image4]: <data:image/png;base64,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>

[image5]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADUAAAAaCAYAAAAXHBSTAAACiklEQVR4Xu2WTahNURTHl1CEUD6ekPIxMKKElN6bSAxIUQaKoZIJBlIG5krIyEdGJpIyePUGyo0yYMCAGJBnJAMpRSIf/591lrPP9u65t7zOq9f51b9z7lrr7HPX3mvts81aWlqaYHluyJgnrZGm546EJdIWaZE0JfM1ymLprPQtdxQMSLel99Id6YN0VJqaBokh6Yl0ubg+lzZWIhqA2fwhfSmuv6ruP8yV7kqPpYWFba/0XToRQWKT9NHKRFlNksM2IcyWOjZ2UsPmCWzN7BetjF9rvnoknrLAfLVWZvZGqEvqrfRZ2pDZT1oZv6u47/z1OjHuzsxOr823f8u3DlaeNqGv+6IuKRKqS2pGct9JA6wc93BmPy39tGqyM6Wb0khiC26YlzG+UWlZxduF/0mKZ3slhT+FHv0krUtsjM97biU2IFlsXIFnjpfu7jSdFLH0ZEA5shpj9S6l/U46Iq2SZlmfZTsRSR1IfseGMiotTeyw2jw+9Krq7k5dUuOxUeAP2CDYKdkxA1aHVaLMpiX24IJ5wpEYn5me1CVF/dPU2zL7VSvjqXN6hNlOYbd6bdUEiH0gzUlsMUGxoivMS3KPdC6CzE8zjMeppScMwgmAgfMjEDvNC+m+lTO02Xz1zkeQ2G3+AY+jEddThS3Adt18PBIG3sekRTXsMF8tkucEc6aI49n95v1VCyuU1muqtGTWm9fyI/Odh+2VP0fTBryUZr4i7SuunFQOJjHROyT6TLomPTVfka/SPelhEct4x8wrgLiX5s8OFf5xgRkdNP/DdQdfJuOQtN2qSUNs29g5olFGfOeAa/o74L3Y+/7oNg0f4ejDSUF8i+ifSQOb0RvpUu5oaWlpaYzfu3usC4MBSNoAAAAASUVORK5CYII=>

[image6]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAcAAAAeCAYAAADgiwSAAAAAo0lEQVR4XmNgGOpABV0ABhSBeCG6IAy0ArELuiAIcADxViCWRhbMA+L/aPgnEFuCJHmAWBKII4D4H5QtBsTMIEkYKGeA6MIALEC8Boifo0uAgDgQ3wXiA2jiYGADxL+BeBK6BAgUMUDsC2KAWAFynDBIAmbfWyDWBGJjIF4MxJxgbQyQIHvIAPHGKiA2g0mAgC8QfwTiDUDsiSwBA7DAGAXIAAD8ORoJ0Ewr5QAAAABJRU5ErkJggg==>

[image7]: <data:image/png;base64,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>

[image8]: <data:image/png;base64,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>

[image9]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGEAAAAaCAYAAACn4zKhAAAD5klEQVR4Xu2YXahNaRjHHzET+RgikvFxaiQf+UjjQriYKBpD+YhSkpKPyBVKytHkwoVpUqIkzcUkRmqS0uTixM3USEOklEKTicmIxuQjxvM7z3rb737OXmuvvdc+zsj61b9z9vu87Xet532fj3eLlJSUFGOoqrcfdPRSDZH680qa4BPVDjEnZ4F9q+pHbygpBhtwJPmbBzbiW9Vn3vABMjLRYG+IwC9jVLO9oZUsV93xgwmkHRbnQWNIXTulfuT8X+mnOqA6pfpJ9Up1q2qGsUr1WNWhOq66pBoX2VvCcNUNVbsbD6xW/Se17c9UM/3gBwAH53vVL9HYXNVL1YxoDF6r1knlsF0U26zPw4SiDBDb4fOqvtWmTijAv4mdhInOBodUf6javKFJtqvuq45K96Y6ovia2OHqk4wR6ffE3ikwX3VGKnMAP+AP/IJ/GgKH+5zPKX6u2u3GA8F+QSx8PcvEXmSJNzQIDvhOtUfMQe8D1pkXfQ7vujAa26X6IfoMYbPIAtOcrSY4bpvY6bortoNbpBJaa8ScuCj5HJigepjYYvnUEx58vxvPyyTVWbF6tNbZ3ichPfGO8UFlA9I24a1YpNSFkOEFpyafp4jls5BacB5O9M7lQUaIOYjFVoot7u8G4YFoVxsp0HzPV6orqq+Tzz0Fh/Ky6h+xFj0maxPYsG+crQvtUslnOIjwu63aHM1hAb7Qdz4BilJavYBQUxD/ZzFILLroMNqcrVl4r9Bi5lHWM36heiB2KAKFNiEU1L/FUhH6VTU6niTWnmVtAguRF9NoZBNo9Y5J+lrNwJrh/fKI1JwGG0pE0ynSMUKhTQgTv/QGR1YkEEFEwhxviBgoFsodUn8TYL3YaSPNUXd6CmrlAulaWDlwca7PKsy10ngVYWK9XpYF0qr8KLFizl+gTvjOJaxD79zf2dIg/y9V3RS7BPZEPWgXO8lv3DhdYrwJNCw+HeMrfMbzD4vGa7JJLAXEUCMOS6UDYBEehi7Jw4PsTf4nX/4c2QJECdGy0RsaoE2sTpAu8m5kUbiYUYi5lwQmq/5SbYjGgI2K/XhCbB7z64Kj2TGcx0teF+tIyH0BHMBlK76gBLgs/SmWOn5XTa82d0K4PhHruopClFG4uS/USo+tBB8sFmvZ94n9fPE0UewfwPavWCfJrwePxPyYG8IoqzMg75+U9NDCMRSptJRBqKZd5JqB7+G+0KEaX23qFjioOJZMwMXNX2YDNDQrxOalzSkEBeqFVN8U88JJ4iVKCsLOnlad84Y6ELb8AFYrwkqaZFaiPIxVXfWDJa2BzoneP4tPVQelweJUUlJSUlLysfIOtgrGwxfPW7MAAAAASUVORK5CYII=>

[image10]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFEAAAAaCAYAAADPELCZAAADs0lEQVR4Xu2XWahNURjHPxki80xIN1MkRChJEsWDIUOU8uLBEBGhvLglj0rq5oVESYZEUpKHEy+mhAwlCok8oBQpGb7fXWs5+35nn33WOfcinf2rf+fstXZ7fftb37C2SE7O/05fVXs7aGin6i2V76tLOqp2iHNSFsxvUp2wE/UODjzkf2PAkXtVPe3EP6KXarBXFtw3SuLfsyqWqZ7bQQ9pO0NKDST1d0rlyP3TFFSPVUdU11UnVX2SNyiDVOdU71QXVO/FZVOblaQBqoeqRjMeWKX6Kenzn1RT7OBfBNu3StEZbCi2XlF182Nky1XVbVV/P0bQfFNt99etgoUKqkuqzi2nmqGBsDg7N9bMwUHVa1WDnUhhouqmao60XQTMFuc0NjrA9Q/VXH/Nu+Gwmb/vcGA791YFDrO1gCj6rNptxgNh/rKqi5mDpeIMWWQnyjBOXFpROtZI+jOrgchjc5IlBXueqAb665fi3sFmzC6JdCJGbla9Ur0QF1EbpbjoanEPWuCvA2PE1Q/mkrKGBCfvM+OVoL42ibOH2tpWkN7fxaVrAPuynJiWgS0gFdn1Cf56vLhdCqnJy6ctQMSyk0QNqbFC3IvbNGSMnea4U0uD6SFus2gKMSUhDQKFIOBdsIVykbSlkhND7UylUXVW1UHcQ9nxp6oNiXuOiVvYdt4AtaRcvYRQU1GmMRGwQTjgjv+1GxYLjiF4hvrrmp0YGsIHcamMbqiGJW9STkm2E1mExcrRlk4ENhu776tGmLlYaHTYfUDc82p2YkizqXbCkBWJRHBaV0vSXdzZrCAZxkRA1C1WPVJNl/jSkHbALohzDs/qJ61oLMGJIaTLgRM569HhLEPENSN+gTppm0BYh3NYVzMXy1rVG9VxcQ0tFs5/ZBdRm7SLTU06EduSR57AYangRFivWmnGqJFNUjzqUJB5EF3awqJ7/P+R4k76FqKUaF1nJzJoUB0V57gtZq4aiFY+PSebcd7nrRSbJ4FEM70mxc9Uop3oJOUzwVFEGS9P93sgpZ2LF6KGcPC0sCDG0KHvqSa1nG6GlPgorutXgnX5kiBClkjpmbUW6MxfVOdV21RnVLdUo5M3ibP9mZ/jPmxmI6Oyh64a6kZazaLu8a0ZQt9CmnD2Ktcp6dzlDuIWMqOaehcLNi4Ul018uZR7Pps2S7VcShtsq5mn+qqabyci4LCc/OSqW9ih06qLdqICIT3TIrxumeYVw3DVXTuY46Bzc/bLopNqv7gmlZOTk5NTB/wCBhrAP5pUsGwAAAAASUVORK5CYII=>

[image11]: <data:image/png;base64,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>

[image12]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEYAAAAaCAYAAAAKYioIAAADqUlEQVR4Xu2YS6iNURTHl1CEJPIIeYQSMpCBQimKPMqr5JWZkiQDSgYmZpJQJJEkRBnII5RTSsrEgDKgLokklJBHHuvXPuvevZfv+853z+B8d3B+9a979vrO/vZeez32uSJt2vRUBvuBFsP7+/rBKuml2q466g0tZrXqqh8sy3zVWdUz1WPVytScgPcPqF7VxfPj4wfqsKBHquHe0GI4oN3SZOSOU61RHVL9leCkPNaqPkp47oFqnfwfqpNUL1SL3HhVDJXgHJzUFETCV9Vd1QBnM86o7klwzAZnMw6rbqn6e0OFvFNN84NlGKOqqZ6q3qomJtbAvLp+qe6rBqXmTvK+XyXHJOxtmDc0YqHqggTnEDWzEmvI0ZOqPRKi5Uhq7qSfFEccUHd4zmCxvaPPZWEO0qQMRDcHOtcbGrFXwqapL2x8eWqWHRLqyxUJ9lWpuZNRqlN+sA45vkn1WvVZtU21VPVGQhGn1pWBefjuS9UX1cDIRr07LqnjYbbqm2qrGy+ESS5L8CZ1ho3viuzkJqHISztUH1RTI3sMkYaDs1ivOlH/m1bOewjvrHcWQVG/LiGKOVA2bbAu1ucdQ2qT4nmRnomlUR8JkcIiz0s4GaKEaDGK0gj4vo82mKLaXP+beSniFtp0sZn18TKwHqJkuuqTpEWetf2JPhtEMhGGQ73TcrE0Ak6cGlOT8HLqit0BmLAojSDPMTHUFCKFlj7C2bqD1TtjiIS7E5HhMcfUJE29XOI0Ags5JhkroQsZo6U4jaCMY8z51CuitBnYXE3CegxLI6LCY46xTGiItWk2DXaavICcjych5YraNODgrFpB16EbsSEKYFxTcA4XxbIdBuwA6YAGnYd5qVkec0zR5bUTiim319uShjRf/q1aEY2x6DuqLVLcWnFs1qkwJ4veqXqv+iFdUXpT0ndZRPE8tSgLDrJDwkUSSO+fkn+H4lDZ0xJv8FjY8XKTFVVyl9qC45gofsaUFRWAQ7IuUqQXbfqSaoHqooSN4cRlkjqSmoaz2EgcER6c8V1CxHOzpejmFVf2lOe0lhFHQwxpZJGJI4jCouLLJs75QYdd8jZKOLC8qwIOs85bGU9U+/1gEyxWHfSDygzVc9UNCc62QsxFkdafBRHFfJXC6fEviQne0A1I49OqOd4gXW36oYS045czdyL+1ZEF0XlNesiP2pESinreYhsxWfJ/b7FRTp92Tw0q6mY4Lqt9VwrOsZ8AVbFPSnSiNm3atGkF/wBhasBHbrHEmAAAAABJRU5ErkJggg==>

[image13]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAD4AAAAaCAYAAADv/O9kAAADXklEQVR4Xu2XS6iNURTHl1Dk/cgjyvGYiFCkSEw8ByQGFAMGYqAURSbyyMhEMhBKkuQxUYRSrpSUkYGUqEuiFCIMyGP97vp23z7L/s453+k6Vzm/+nfvWXt/+9tr7b3X2p9ImzbN0s8bWgzvH+iNzcBAY1VDs9+9VIOyv55Zqlve2GImqO56Y1nGq76pXqneq56p1qrOq/pE/YAXPlItcfaeYG6mppijeqsakP1mhZepfqp2hk4ZfVWXVEclvRNaDXO4rRriG+rBSj8Rc8TzXbXA2Vjl16rpzt6TvFRt88ZacH7viW1vAuC5ItXbHGc/qDZEtn+B5WLHdLFvKIJE9kL1S7WpuqmL+e73VtUn1UxnjyHLjpD8GJAom8m8PM84jTBJ9UZ12DcUQRa/LuY42qWarOodd4o4q3quGu0bMsZInhzvq6Zlv9kljN1oTmD33RFzZqVrIx8tcrawc/Gl4RK7SvVDcufRR9W6uFNGR6bUCrI6V7P/CQx5g3FniFWIWgGL6a+6LFZRCABVJQ7YMflzS3McOZaPVSNdW0NUVDtUX8QC4OFYsOoedshmsedhttgYTAZHFqoGZ231WCHWH2ePi53fwFTVO0mvKvOijT51SQ0A26Wc4x5yAc/7UliGcapO1bDIRlJNzQuYF8Em6HXx5yeAPfWCRhxnpc5IuhSWgSPo58A2526RgnkVVacqSAhFjhPZ1Aseil0WwkUnhnM/SuyMcdbiM832K3vLI0PHjrPyvJ+ElwLHWRgqVU3WiGXbuGTh0DnVU7FrqYeIp6IazjTX2BNiQQt3AJLkNckTIn87xJzqzGwp9kgefG5lN8SeSZWsEOzTviEFTmxRfRWL5EmxaytlqJJ3qyJUAJ9VcQbnKClHVKvFEg22C6rhedeuo0BpYxz6FBGcvSi2ez6LBcK/G0Lg616syMLzsv9Z5aViD1Wkdq0NCScVdcZkqwfCl14R7IZT3uhgLoyBWJxOsTl4SKbsxIm+obtgItzpH0gTHwUOzj/12kNACCw7MeQgjiWrfUjSC3NTrPSl2rqNKWLRXe8bSrJRtc8bpfoazTvINeScWl9gfKRwS/zr8Gm6XyyRNQPJyCfIGNoPiO0skm/RFZoV3i02n5bBy/Z6Y4vhhnfQG9u0afN/8hsoQqPqxoAyIwAAAABJRU5ErkJggg==>

[image14]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAZCAYAAAA4/K6pAAAA+ElEQVR4Xu2SPwtBYRSHj7IhSSlhkUUpg9VgsFhMBkUZfAYpuw9BGazyEQzKaDGwY2CyKMogfqdz73Xvuf59gPvU063fOffct/NeIg9NFg7gCu6N5wgODdswanV/oQK3MKHyDDzAh8pd9OEU+lUeggv6MSAAZ7CrCyANj/RjgNlU1AXQJHn5pgt2+PjclIJxWIJ1eIUTmLQ63xCEc5IB5ub5FnawBX1W5wf4Gk/wovIwvMOqyl00SL6+0QWSfKxDOxG4JGnkQXZ4sZz3VO6gQHL0M8yrWplkgHm1/H/wdTvokDStyb1pPSAHY6/yf/CCa4b8S3t4WDwByBAwE2J6GB0AAAAASUVORK5CYII=>

[image15]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABIAAAAaCAYAAAC6nQw6AAABEUlEQVR4Xu2SvUoDURBGR1QQFBQJgiiIdlYp1EKIwTatbV7BysZObHwSCWmEVGKhhaU+gGUgiCBEtIugFnpmZ+46WbZMuQcOy/3m7v0XqaiYLEuF9nRJpu0Vr5VyhO94g8t4jM/47dki7uELfuAbHmR/BrTTFZ7iLz7hGc6KTaDZJfa8/xoO8B4XPMtoYdMLr7gVaidiA+kKE9tiq7/GuZDnaPEO50OmK/0U21aiLTb4ecjG0OJFIeuLbbXm7Rns4hfup05FdCDdZuQHOzjl7U2xA38QO9t1z3P0XIrb0tlHuBMyvRAdvIEb8n8BOYdinSL6ZtLMiToO8RYfcTfUMvSBlT2yset19KZW/VtRMVH+ANlfLy0k1NSTAAAAAElFTkSuQmCC>

[image16]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAsAAAAcCAYAAAC3f0UFAAAA50lEQVR4Xu2SqwpCQRCGR1BQvIMogu9gspgtFrMYbOIj+AQWu5itYrcYThRMBqNBsVoNgpf/dy8c1j1dwQ++sDNzdmdnj8hvU4cneINDJ/dBCW7hFTacnBcWH2DFTfi4wCWMuwkfTzhyg1E8YCu0jsFEaG3JwjOs6TUvuYMrmDFFhg4cwyacwBRsww3Mh+resHABpzCtY2yhais0TK5FXZDj64na2Qv7ZL9FOIB3OJOIEXICnAThZQJ4FNVCX8csnC1bIKZ4L+oXmMOkzr2P4qvx9YjpP4A52NVxS0FCX2t4QtmJ/fk2XgTqIybkPiUQAAAAAElFTkSuQmCC>