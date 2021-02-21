use crate::libusb::async_device::AsyncDevice;
use crate::libusb::context::Context;
use crate::libusb::device_handle::DeviceHandle;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct AsyncContext {
    context: Arc<Context>,
    running_atomic: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl AsyncContext {
    pub fn start(context: Context) -> AsyncContext {
        Self::with_arc(Arc::new(context))
    }
    pub fn with_arc(context: Arc<Context>) -> AsyncContext {
        let job_context = context.clone();
        let is_running = Arc::new(AtomicBool::new(true));
        let running_atomic = is_running.clone();
        let job = move || {
            while is_running.load(Ordering::Relaxed) {
                job_context
                    .handle_events_timeout(std::time::Duration::from_secs(1))
                    .expect("libusb handle events error");
            }
        };
        let handle = std::thread::spawn(job);
        AsyncContext {
            context,
            running_atomic,
            thread: Some(handle),
        }
    }
    pub fn context_ref(&self) -> &Context {
        &self.context
    }
    pub fn context_arc(&self) -> Arc<Context> {
        self.context.clone()
    }
    /// WARNING!!: If the device belongs to another context, async operations on that device will
    /// just block. This function is a no-op just to make sure a `AsyncContext` is running. It does
    /// not check to make sure it owns the handle. Proceed at own risk.
    pub fn make_async_device(&self, handle: DeviceHandle) -> AsyncDevice {
        AsyncDevice { handle }
    }
}
impl Drop for AsyncContext {
    fn drop(&mut self) {
        self.running_atomic.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            handle.join().expect("async context panicked")
        }
    }
}
