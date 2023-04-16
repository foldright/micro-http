use arc_swap::ArcSwap;
use httpdate::fmt_http_date;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct DateService {
    current: Arc<ArcSwap<(SystemTime, String)>>,
    handle: tokio::task::JoinHandle<()>,
}

impl DateService {
    pub(crate) fn new() -> Self {
        let system_time = SystemTime::now();
        let http_date = fmt_http_date(system_time);

        let current = Arc::new(ArcSwap::new(Arc::new((system_time, http_date))));
        let current_arc = Arc::clone(&current);

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(700)).await;
                let system_time = SystemTime::now();
                let http_date = fmt_http_date(system_time);
                current_arc.store(Arc::new((system_time, http_date)));
            }
        });

        DateService { current, handle }
    }

    pub(crate) fn with_http_date<F>(&self, mut f: F) where F: FnMut(&str) {
        let date = &self.current.load().1;
        f(date)
    }
}

impl Drop for DateService {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
