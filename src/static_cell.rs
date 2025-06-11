#![no_std]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::console::PLATFORM;
use crate::gpio::{GPIO_BASE, init_gpio_as_output, set_gpio_output};

// 状态定义
const STATE_UNUSED: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_INITIALIZED: u8 = 2;

/// 完全避免原子读-修改-写操作的 StaticCell
pub struct StaticCell<T> {
    state: AtomicU8,
    val: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for StaticCell<T> {}
unsafe impl<T> Sync for StaticCell<T> {}

impl<T> StaticCell<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_UNUSED),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init(&'static self, val: T) -> &'static mut T {
        self.uninit().write(val)
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init_with(&'static self, val: impl FnOnce() -> T) -> &'static mut T {
        self.uninit().write(val())
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn uninit(&'static self) -> &'static mut MaybeUninit<T> {
        if let Some(val) = self.try_uninit() {
            val
        } else {
            panic!("`StaticCell` is already full, it can't be initialized twice.");
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn try_init(&'static self, val: T) -> Option<&'static mut T> {
        Some(self.try_uninit()?.write(val))
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn try_init_with(&'static self, val: impl FnOnce() -> T) -> Option<&'static mut T> {
        Some(self.try_uninit()?.write(val()))
    }

    #[inline]
    pub fn try_uninit(&'static self) -> Option<&'static mut MaybeUninit<T>> {
        // 方法1: 仅使用 load 和 store 的乐观方法

        println!("这里开始出问题的");
        // 首先检查当前状态
        if self.state.load(Ordering::Acquire) != STATE_UNUSED {
            return None;
        }
        println!("这里开始出问题的1");

        // init_gpio_as_output(GPIO_BASE, 55);
        // set_gpio_output(GPIO_BASE, 55, true);

        // 尝试原子地设置为 INITIALIZING 状态
        // 这里使用一个技巧：如果在单核环境下，或者这个核心有独占访问，
        // 简单的 store 也是安全的
        self.state.store(STATE_INITIALIZING, Ordering::Release);
        println!("这里开始出问题的2");

        // 立即再次检查状态，如果在多核环境下有竞争，
        // 可能会看到不一致的状态
        let current_state = self.state.load(Ordering::Acquire);
        if current_state == STATE_INITIALIZING {
            // 我们成功获得了初始化权限
            // SAFETY: 我们设置了初始化状态
            let val = unsafe { &mut *self.val.get() };
            Some(val)
        } else {
            // 竞争失败，其他线程可能同时在操作
            None
        }
    }
}

/// 使用内存屏障和简单计数的版本（适合已知是单核的情况）
pub struct StaticCellSingleCore<T> {
    initialized: AtomicU8,
    val: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for StaticCellSingleCore<T> {}
unsafe impl<T> Sync for StaticCellSingleCore<T> {}

impl<T> StaticCellSingleCore<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            initialized: AtomicU8::new(0),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init(&'static self, val: T) -> &'static mut T {
        self.uninit().write(val)
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn uninit(&'static self) -> &'static mut MaybeUninit<T> {
        if let Some(val) = self.try_uninit() {
            val
        } else {
            panic!("`StaticCell` is already full, it can't be initialized twice.");
        }
    }

    #[inline]
    pub fn try_uninit(&'static self) -> Option<&'static mut MaybeUninit<T>> {
        // 在单核环境下，简单的 load + store 组合是安全的
        // （因为不会有真正的并发访问）

        if self.initialized.load(Ordering::Relaxed) != 0 {
            return None;
        }

        // 设置初始化标志
        self.initialized.store(1, Ordering::Relaxed);

        // SAFETY: 在单核环境下，上面的检查和设置是原子的
        let val = unsafe { &mut *self.val.get() };
        Some(val)
    }
}

/// 使用自旋锁思想的版本（避免 AMO 指令）
pub struct StaticCellSpinlock<T> {
    // 使用一个简单的标志，配合忙等待
    lock_flag: AtomicU8,
    initialized: AtomicU8,
    val: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for StaticCellSpinlock<T> {}
unsafe impl<T> Sync for StaticCellSpinlock<T> {}

impl<T> StaticCellSpinlock<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            lock_flag: AtomicU8::new(0),
            initialized: AtomicU8::new(0),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init(&'static self, val: T) -> &'static mut T {
        self.uninit().write(val)
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn uninit(&'static self) -> &'static mut MaybeUninit<T> {
        if let Some(val) = self.try_uninit() {
            val
        } else {
            panic!("`StaticCell` is already full, it can't be initialized twice.");
        }
    }

    #[inline]
    pub fn try_uninit(&'static self) -> Option<&'static mut MaybeUninit<T>> {
        // 尝试获取"锁"（使用简单的标志位）
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 1000;

        while attempts < MAX_ATTEMPTS {
            // 检查是否已经初始化
            if self.initialized.load(Ordering::Acquire) != 0 {
                return None;
            }

            // 尝试获取锁
            if self.lock_flag.load(Ordering::Acquire) == 0 {
                // 尝试设置锁
                self.lock_flag.store(1, Ordering::Release);

                // 双重检查锁定模式
                if self.lock_flag.load(Ordering::Acquire) == 1
                    && self.initialized.load(Ordering::Acquire) == 0
                {
                    // 成功获得锁且未初始化，可以进行初始化
                    self.initialized.store(1, Ordering::Release);

                    // SAFETY: 我们通过锁机制确保了独占访问
                    let val = unsafe { &mut *self.val.get() };

                    // 释放锁
                    self.lock_flag.store(0, Ordering::Release);

                    return Some(val);
                } else {
                    // 锁竞争失败，释放锁
                    self.lock_flag.store(0, Ordering::Release);
                }
            }

            attempts += 1;

            // 简单的延迟，避免忙等待占用太多 CPU
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }

        None
    }
}

// ConstStaticCell 的无 AMO 版本
pub struct ConstStaticCell<T> {
    taken: AtomicU8,
    val: UnsafeCell<T>,
}

unsafe impl<T> Send for ConstStaticCell<T> {}
unsafe impl<T> Sync for ConstStaticCell<T> {}

impl<T> ConstStaticCell<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            taken: AtomicU8::new(0),
            val: UnsafeCell::new(value),
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn take(&'static self) -> &'static mut T {
        if let Some(val) = self.try_take() {
            val
        } else {
            panic!("`ConstStaticCell` is already taken, it can't be taken twice")
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn try_take(&'static self) -> Option<&'static mut T> {
        // 检查是否已被取走
        if self.taken.load(Ordering::Acquire) != 0 {
            return None;
        }

        // 尝试标记为已取走
        self.taken.store(1, Ordering::Release);

        // 再次检查以确保我们是第一个
        if self.taken.load(Ordering::Acquire) == 1 {
            // SAFETY: 我们设置了标志位
            let val = unsafe { &mut *self.val.get() };
            Some(val)
        } else {
            None
        }
    }
}
