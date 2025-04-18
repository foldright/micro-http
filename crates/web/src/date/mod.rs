//! HTTP date header value management service.
//!
//! This module provides a service for efficiently managing and updating HTTP date header values
//! in a concurrent environment. It updates the date string periodically to avoid repeated
//! date string formatting operations in high-concurrency scenarios.

use arc_swap::ArcSwap;
use http::HeaderValue;
use httpdate::fmt_http_date;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

mod date_service_decorator;

pub use date_service_decorator::DateServiceDecorator;

/// A service that maintains and periodically updates the current HTTP date string.
///
/// This service runs a background task that updates the date string every 700ms,
/// providing an efficient way to access formatted HTTP date strings without
/// formatting them on every request.
pub struct DateService {
    current: Arc<ArcSwap<(SystemTime, HeaderValue)>>,
    handle: tokio::task::JoinHandle<()>,
}

static DATE_SERVICE: Lazy<DateService> = Lazy::new(|| DateService::new_with_update_interval(Duration::from_millis(800)));

impl DateService {

    /// Returns a reference to the global singleton instance of `DateService`.
    ///
    /// This method provides access to a shared `DateService` instance that can be used
    /// across the application to efficiently handle HTTP date headers.
    ///
    /// # Returns
    /// A static reference to the global `DateService` instance.
    pub fn get_global_instance() -> &'static DateService {
        &DATE_SERVICE
    }

    /// Creates a new `DateService` instance.
    ///
    /// This method initializes the service with the current system time and starts
    /// a background task that updates the date string every 700ms.
    ///
    /// # Returns
    /// Returns a new `DateService` instance with the background update task running.
    fn new_with_update_interval(update_interval: Duration) -> Self {
        let system_time = SystemTime::now();
        let http_date = fmt_http_date(system_time);
        let date_value = HeaderValue::try_from(http_date).expect("http_date should not fail");

        let current = Arc::new(ArcSwap::from_pointee((system_time, date_value)));
        let current_arc = Arc::clone(&current);

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(update_interval).await;
                let system_time = SystemTime::now();
                let http_date = fmt_http_date(system_time);
                let date_value = HeaderValue::try_from(http_date).expect("http_date should not fail");
                current_arc.store(Arc::new((system_time, date_value)));
            }
        });

        DateService { current, handle }
    }

    /// Provides access to the current HTTP date string through a callback function.
    ///
    /// This method allows safe access to the current date string without exposing
    /// the internal synchronization mechanisms.
    pub(crate) fn with_http_date<F>(&self, mut f: F)
    where
        F: FnMut(&HeaderValue),
    {
        let date = &self.current.load().1;
        f(date)
    }
}

/// Implements the `Drop` trait to ensure the background task is properly cleaned up
/// when the `DateService` is dropped.
impl Drop for DateService {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
