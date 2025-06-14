//! Embassy time driver implementation using RustSBI's IPI interface

use crate::console::PLATFORM;
use core::{
    cell::RefCell,
    panic,
    sync::atomic::{AtomicU64, Ordering},
};
use critical_section::{Impl, Mutex, with};
use embassy_time::TICK_HZ;
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;

// use crate::{CLINT, SifiveClintWrap, get_clint};
use crate::CLINT;
// use rustsbi::Timer;

struct RustSbiCriticalSection;
critical_section::set_impl!(RustSbiCriticalSection);

unsafe impl Impl for RustSbiCriticalSection {
    unsafe fn acquire() -> critical_section::RawRestoreState {
        let mstatus = riscv::register::mstatus::read();
        unsafe { riscv::register::mstatus::clear_mie() };
        mstatus.bits()
    }

    unsafe fn release(restore_state: critical_section::RawRestoreState) {
        if restore_state & (1 << 3) != 0 {
            unsafe { riscv::register::mstatus::set_mie() };
        }
    }
}

// 引入RustSBI相关类型和接口
// use crate::{platform::PLATFORM, sbi::ipi::clear_mtime};

// const CLINT_FREQ_HZ: u64 = 51_200_000; // 51.2MHz, stg apb clock
const CLINT_FREQ_HZ: u64 = 4_000_000; // 51.2MHz, stg apb clock
// const EMBASSY_TICK_HZ: u64 = 5_120_000; // 5.12MHz from Cargo.toml feature tick-hz-1_000_000
const FREQ_RATIO: u64 = CLINT_FREQ_HZ / TICK_HZ;

struct MachineTimeDriver {
    queue: Mutex<RefCell<Queue>>,
    next_alarm: AtomicU64,
}

embassy_time_driver::time_driver_impl!(static DRIVER: MachineTimeDriver = MachineTimeDriver {
    queue: Mutex::new(RefCell::new(Queue::new())),
    next_alarm: AtomicU64::new(u64::MAX),
});

impl MachineTimeDriver {
    pub fn init(&self) {
        // println!("Hello, world!10");
        let current_time = Self::read_time();
        // println!("Hello, world!11");
        self.next_alarm.store(current_time, Ordering::Relaxed);
        // println!("Hello, world!12");

        // 启用机器定时器中断
        unsafe {
            riscv::register::mie::set_mtimer();
        }
        // println!("Hello, world!13");
    }

    fn read_time() -> u64 {
        // 通过SbiIpi读取时间
        // let low = ipi.get_time() as u64;
        // let high = ipi.get_timeh() as u64;
        // (high << 32) | low
        // if let Some(clint) = get_clint() {
        //     return clint.read_mtime();
        // }
        // 0

        // println!("Hello, world!14");
        #[allow(static_mut_refs)]
        unsafe {
            CLINT.read_mtime()
        }
    }

    fn set_timer(&self, when_ticks: u64) {
        // 使用RustSBI的Timer接口设置定时器
        // ipi.set_timer(when_ticks);
        // if let Some(clint) = unsafe { &mut CLINT } {
        // clint.set_msip()
        #[allow(static_mut_refs)]
        unsafe {
            CLINT.write_mtimecmp(0, when_ticks)
        };
        unsafe {
            riscv::register::mip::clear_stimer();
        }
        // Enable machine timer interrupt.
        unsafe {
            riscv::register::mie::set_mtimer();
        }
        // }
    }

    pub fn handle_timer_interrupt(&self) {
        // clear_mtime();
        // if let Some(clint) = unsafe { &mut CLINT } {
        #[allow(static_mut_refs)]
        unsafe {
            CLINT.write_mtimecmp(0, u64::MAX)
        };
        // }
        with(|cs| {
            let now = Self::read_time();
            let mut queue = self.queue.borrow_ref_mut(cs);
            let next_alarm = queue.next_expiration(now);

            if next_alarm != u64::MAX {
                self.set_timer(next_alarm);
                self.next_alarm.store(next_alarm, Ordering::Relaxed);
            } else {
                self.next_alarm.store(u64::MAX, Ordering::Relaxed);
            }
        })
    }
}

impl Driver for MachineTimeDriver {
    fn now(&self) -> u64 {
        let current_ticks = Self::read_time();

        current_ticks / FREQ_RATIO
    }

    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        with(|cs| {
            let mut queue = self.queue.borrow_ref_mut(cs);

            if queue.schedule_wake(at, waker) {
                let now = Self::read_time();
                let next = queue.next_expiration(now);
                self.set_timer(next);
                self.next_alarm.store(at, Ordering::Relaxed);
            }
        })
    }
}

pub fn init() {
    // println!("Hello, world!8");
    DRIVER.init();
    // println!("Hello, world!9");

    // info!("mstatus = {:x}", riscv::register::mstatus::read().bits());
    // info!("mie = {:x}", riscv::register::mie::read().bits());
    // info!("mip = {:x}", riscv::register::mip::read().bits());
}

pub fn timer_interrupt_handler() {
    DRIVER.handle_timer_interrupt();
}

pub fn is_timer_interrupt_pending() -> bool {
    use riscv::register::mip;
    mip::read().mtimer()
}

// #[cfg(feature = "defmt")]
// impl defmt::Format for MachineTimeDriver {
//     fn format(&self, f: defmt::Formatter) {
//         defmt::write!(
//             f,
//             "MachineTimeDriver {{ next_alarm: {} }}",
//             self.next_alarm.load(Ordering::Relaxed)
//         )
//     }
// }
