use core::sync::atomic::{AtomicBool, Ordering};

pub struct SimpleOnce<T> {
    initialized: AtomicBool,
    value: core::cell::UnsafeCell<core::mem::MaybeUninit<T>>,
}

unsafe impl<T: Sync> Sync for SimpleOnce<T> {}
unsafe impl<T: Send> Send for SimpleOnce<T> {}

impl<T> SimpleOnce<T> {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()),
        }
    }

    pub fn call_once(&self, f: impl FnOnce() -> T) -> &T {
        // 简单检查是否已初始化
        if self.initialized.load(Ordering::Acquire) {
            return unsafe { &*(*self.value.get()).as_ptr() };
        }

        // 初始化逻辑
        let val = f();
        unsafe {
            (*self.value.get()).write(val);
        }
        self.initialized.store(true, Ordering::Release);

        unsafe { &*(*self.value.get()).as_ptr() }
    }

    pub fn get(&self) -> Option<&T> {
        if self.initialized.load(Ordering::Acquire) {
            Some(unsafe { &*(*self.value.get()).as_ptr() })
        } else {
            None
        }
    }
}
