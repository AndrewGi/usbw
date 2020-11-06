use crate::libusb::context::Context;
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
                job_context.handle_events();
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
}
impl Drop for AsyncContext {
    fn drop(&mut self) {
        self.running_atomic.store(false, Ordering::SeqCst);
        self.thread
            .take()
            .map(|h| h.join().expect("async context paniced"));
    }
}
