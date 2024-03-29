// use async_lock::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use parking_lot::{Mutex as SyncMutex, RwLock as SyncRwLock};

pub type Arc<R> = std::sync::Arc<R>;
// pub type ArcAsyncRwLock<T> = Arc<AsyncRwLock<T>>;

// #[inline(always)]
// pub fn arc_async_rw_lock<T>(value: T) -> Arc<AsyncRwLock<T>> {
//     Arc::new(AsyncRwLock::new(value))
// }

pub type ArcSyncRwLock<T> = Arc<SyncRwLock<T>>;

#[inline(always)]
pub fn arc_sync_rw_lock<T>(value: T) -> Arc<SyncRwLock<T>> {
    Arc::new(SyncRwLock::new(value))
}

// pub type ArcAsyncMutex<T> = Arc<AsyncMutex<T>>;

// #[inline(always)]
// pub fn arc_async_mutex<T>(value: T) -> Arc<AsyncMutex<T>> {
//     Arc::new(AsyncMutex::new(value))
// }

pub type ArcSyncMutex<T> = Arc<SyncMutex<T>>;

#[inline(always)]
pub fn arc_sync_mutex<T>(value: T) -> Arc<SyncMutex<T>> {
    Arc::new(SyncMutex::new(value))
}

pub trait IntoSendSync {
    type SendSync: Send + Sync;

    fn into_send_sync(self) -> Self::SendSync;
}
