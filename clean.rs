#![no_std]
#![no_main]
pub mod console;
mod gpio;
#[macro_use]
mod log;
mod simple_once;
mod static_cell;
mod time_driver;

use core::{arch::asm, mem::forget, ptr::NonNull};

use ::log::{error, info};
use aclint::SifiveClint;
use console::{PLATFORM, Uart16550Wrap};
use embassy_executor::Executor;
use embassy_time::{Duration, Timer};
use fast_trap::{FastContext, FastResult, FlowContext, FreeTrapStack};
use gpio::{GPIO_BASE, init_gpio_as_output, set_gpio_output, toggle_gpio};
// use log::*;
use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{mcause, mepc, mtval, mtvec},
};
#[doc(hidden)]
async fn __run_simple_task() {
    loop {
        println!("虽然打个印");
        Timer::after(Duration::from_secs(2)).await;
    }
}

fn run_simple() -> ::embassy_executor::SpawnToken<impl Sized> {
    println!("spawn one 3");
    const fn __task_pool_get<F, Args, Fut>(
        _: F,
    ) -> &'static ::embassy_executor::raw::TaskPool<Fut, POOL_SIZE>
    where
        F: ::embassy_executor::_export::TaskFn<Args, Fut = Fut>,
        Fut: ::core::future::Future + 'static,
    {
        unsafe { &*POOL.get().cast() }
    }
    println!("spawn one 4");
    const POOL_SIZE: usize = 3;
    static POOL: ::embassy_executor::_export::TaskPoolHolder<
        { ::embassy_executor::_export::task_pool_size::<_, _, _, POOL_SIZE>(__run_simple_task) },
        { ::embassy_executor::_export::task_pool_align::<_, _, _, POOL_SIZE>(__run_simple_task) },
    > = unsafe {
        ::core::mem::transmute(::embassy_executor::_export::task_pool_new::<
            _,
            _,
            _,
            POOL_SIZE,
        >(__run_simple_task))
    };
    println!("spawn one 6");
    unsafe {
        println!("spawn one 7");
        let pool_get = __task_pool_get(__run_simple_task);
        println!("spawn one 8");
        pool_get._spawn_async_fn(move || __run_simple_task())
    }
}
// #[embassy_executor::task]
// async fn run_simple() {
//     loop {
//         Timer::after(Duration::from_secs(2)).await;
//     }
// }

#[embassy_executor::task]
async fn run_gpio() {
    // 初始化 GPIO5 作为输出
    println!("Hello, world!7");
    init_gpio_as_output(GPIO_BASE, 55);

    loop {
        // 切换 LED 状态验证 Embassy 运行
        unsafe {
            toggle_gpio(GPIO_BASE, 55);
        }

        // 1s延迟
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

pub struct SifiveClintWrap {
    inner: *const SifiveClint,
}

impl SifiveClintWrap {
    pub const fn new(base: usize) -> Self {
        Self {
            inner: base as *const SifiveClint,
        }
    }
}

impl SifiveClintWrap {
    #[inline(always)]
    fn read_mtime(&self) -> u64 {
        let result = unsafe { (*self.inner).read_mtime() };
        result
    }

    #[inline(always)]
    fn write_mtime(&self, val: u64) {
        unsafe { (*self.inner).write_mtime(val) }
    }

    #[inline(always)]
    fn read_mtimecmp(&self, hart_idx: usize) -> u64 {
        unsafe { (*self.inner).read_mtimecmp(hart_idx) }
    }

    #[inline(always)]
    fn write_mtimecmp(&self, hart_idx: usize, val: u64) {
        unsafe { (*self.inner).write_mtimecmp(hart_idx, val) }
    }

    #[inline(always)]
    fn read_msip(&self, hart_idx: usize) -> bool {
        unsafe { (*self.inner).read_msip(hart_idx) }
    }

    #[inline(always)]
    fn set_msip(&self, hart_idx: usize) {
        unsafe { (*self.inner).set_msip(hart_idx) }
    }

    #[inline(always)]
    fn clear_msip(&self, hart_idx: usize) {
        unsafe { (*self.inner).clear_msip(hart_idx) }
    }
}

static mut EXECUTOR: Option<Executor> = None;

unsafe impl Sync for SifiveClintWrap {}
unsafe impl Send for SifiveClintWrap {}

static mut CLINT: SifiveClintWrap = SifiveClintWrap::new(0x2000000);

pub extern "C" fn fast_handler(
    mut ctx: FastContext,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> FastResult {
    // Save mepc into context
    let epc = mepc::read();
    ctx.regs().pc = epc;

    let save_regs = |ctx: &mut FastContext| {
        ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
    };
    let cause = mcause::read();
    println!("TRAP: cause{:?}, code={:#x}", cause.cause(), cause.code());

    match cause.cause().try_into() {
        Ok(cause) => {
            match cause {
                // Handle MTimer
                Trap::Interrupt(Interrupt::MachineTimer) => {
                    // 会导致中断委托给S态，因而在embassy这里应该不做处理？
                    crate::time_driver::timer_interrupt_handler();

                    save_regs(&mut ctx);
                    ctx.restore()
                }
                // Handle other traps
                trap => unsupported_trap(Some(trap)),
            }
        }
        Err(err) => {
            println!("Failed to parse mcause: {:?}", err);
            unsupported_trap(None);
        }
    }
}
pub fn unsupported_trap(trap: Option<Trap<Interrupt, Exception>>) -> ! {
    println!("-----------------------------");
    println!("trap:    {trap:?}");
    println!("mepc:    {:#018x}", mepc::read());
    println!("mtval:   {:#018x}", mtval::read());
    println!("-----------------------------");
    panic!("Stopped with unsupported trap")
}

const STACK_SIZE: usize = 16 * 1024;

// #[repr(C, align(128))]
#[unsafe(link_section = ".bss.stack")]
// static mut HART0_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut HART0_STACK: Stack = Stack([0; STACK_SIZE]);

#[repr(C, align(128))]
pub(crate) struct Stack([u8; STACK_SIZE]);

impl Stack {
    fn load_as_stack(&'static mut self) {
        let range = self.0.as_ptr_range();

        // Create and load trap stack, forgetting it to avoid drop
        forget(
            FreeTrapStack::new(
                range.start as usize..range.end as usize,
                |_| {}, // Empty callback
                unsafe { NonNull::new_unchecked(&mut FlowContext::ZERO) },
                fast_handler,
            )
            .unwrap()
            .load(),
        );
    }
}

fn clear_bss() {
    unsafe extern "C" {
        fn start_bss();
        fn end_bss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(
            start_bss as usize as *mut u8,
            end_bss as usize - start_bss as usize,
        )
        .fill(0);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
extern "C" fn _start(argc: usize, argv: usize) -> ! {
    clear_bss();

    unsafe {
        asm!(
            "la sp, {stack} + {stack_size}",
            stack = sym HART0_STACK,
            stack_size = const STACK_SIZE,
        )
        // asm!("la sp, {0}", sym HART0_STACK);
        // asm!("addi sp, sp, {}", const STACK_SIZE);
    }

    #[allow(static_mut_refs)]
    unsafe {
        PLATFORM.console = Some(Uart16550Wrap::<u32>::new(0x10000000))
    };
    println!("可以了么？");

    time_driver::init();
    #[allow(static_mut_refs)]
    unsafe {
        HART0_STACK.load_as_stack()
    };

    unsafe { mtvec::write(fast_trap::trap_entry as _, mtvec::TrapMode::Direct) };

    let executor_new = Executor::new();
    unsafe {
        EXECUTOR = Some(executor_new);
        #[allow(static_mut_refs)]
        if let Some(executor) = EXECUTOR.as_mut() {
            println!("Hello, world!5");
            executor.run(|spawner| {
                println!("Hello, world!6");
                spawner.spawn(run_simple()).unwrap()
            });
        }
    };

    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use ::riscv::register::*;
    // error!("Hart {} {info}", current_hartid());
    println!("{info}");
    println!("-----------------------------");
    println!("mcause:  {:?}", mcause::read().cause());
    println!("mepc:    {:#018x}", mepc::read());
    println!("mtval:   {:#018x}", mtval::read());
    println!("-----------------------------");
    println!("System shutdown scheduled due to RustSBI panic");
    loop {}
}
