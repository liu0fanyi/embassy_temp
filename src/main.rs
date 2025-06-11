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
// use simple_once::SimpleOnce;
// use spin::Once;
// use spin::Mutex;
// use static_cell::StaticCell;

// #[macro_use]
// extern crate log;

// #[embassy_executor::task]
// async fn run() {
//     loop {
//         Timer::after(Duration::from_millis(1_000)).await;
//         info!("spawn 1 second");
//     }
// }

// #[embassy_executor::task]
// async fn run_another() {
//     loop {
//         Timer::after(Duration::from_secs(5)).await;
//         info!("spawn 5 second");
//     }
// }

// #[doc(hidden)]
// async fn __run_simple_task() {
//     // init_gpio_as_output(GPIO_BASE, 55);
//     set_gpio_output(GPIO_BASE, 55, false);
//     let mut count = 0;
//     loop {
//         println!("虽然打个印n:{}", count);
//         count += 1;
//         // unsafe { toggle_gpio(GPIO_BASE, 55) };
//         Timer::after(Duration::from_secs(2)).await;
//         if count % 2 == 0 {
//             set_gpio_output(GPIO_BASE, 55, true);
//         } else {
//             set_gpio_output(GPIO_BASE, 55, false);
//         }
//     }
// }

// fn run_simple() -> ::embassy_executor::SpawnToken<impl Sized> {
//     println!("spawn one 3");
//     const fn __task_pool_get<F, Args, Fut>(
//         _: F,
//     ) -> &'static ::embassy_executor::raw::TaskPool<Fut, POOL_SIZE>
//     where
//         F: ::embassy_executor::_export::TaskFn<Args, Fut = Fut>,
//         Fut: ::core::future::Future + 'static,
//     {
//         unsafe { &*POOL.get().cast() }
//     }
//     println!("spawn one 4");
//     const POOL_SIZE: usize = 3;
//     // #[unsafe(link_section = ".data")]
//     static POOL: ::embassy_executor::_export::TaskPoolHolder<
//         { ::embassy_executor::_export::task_pool_size::<_, _, _, POOL_SIZE>(__run_simple_task) },
//         { ::embassy_executor::_export::task_pool_align::<_, _, _, POOL_SIZE>(__run_simple_task) },
//     > = unsafe {
//         ::core::mem::transmute(::embassy_executor::_export::task_pool_new::<
//             _,
//             _,
//             _,
//             POOL_SIZE,
//         >(__run_simple_task))
//     };
//     println!("spawn one 6");
//     unsafe {
//         println!("spawn one 7");
//         let pool_get = __task_pool_get(__run_simple_task);
//         println!("spawn one 8");
//         pool_get._spawn_async_fn(move || __run_simple_task())
//     }
// }
#[embassy_executor::task]
async fn run_simple() {
    loop {
        Timer::after(Duration::from_secs(2)).await;
        println!("Hello, run_simple!");
    }
}

#[embassy_executor::task]
async fn run_gpio() {
    // 初始化 GPIO5 作为输出
    println!("Hello, world!run_gpio");
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
        // println!("Hello, world!16");
        let result = unsafe { (*self.inner).read_mtime() };
        // println!("Hello, world!17");
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

// static EXECUTOR: StaticCell<Executor> = StaticCell::new();
// static EXECUTOR: SimpleOnce<Executor> = SimpleOnce::new();
static mut EXECUTOR: Option<Executor> = None;

unsafe impl Sync for SifiveClintWrap {}
unsafe impl Send for SifiveClintWrap {}

// static CLINT: StaticCell<Mutex<SifiveClintWrap>> = StaticCell::new();
// static mut CLINT: Option<SifiveClintWrap> = Option::None;
// static CLINT: AtomicPtr<SifiveClintWrap> = AtomicPtr::new(core::ptr::null_mut());

// pub fn init_clint(base: usize) {
//     let clint = Box::leak(Box::new(SifiveClintWrap::new(base)));
//     CLINT.store(clint, Ordering::SeqCst);
// }

// pub fn get_clint() -> Option<&'static SifiveClintWrap> {
//     let ptr = CLINT.load(Ordering::SeqCst);
//     if ptr.is_null() {
//         None
//     } else {
//         Some(unsafe { &*ptr })
//     }
// }
// 使用StaticCell来静态分配CLINT实例
// static CLINT: StaticCell<SifiveClintWrap> = StaticCell::new();
// static CLINT: SimpleOnce<SifiveClintWrap> = SimpleOnce::new();
static mut CLINT: SifiveClintWrap = SifiveClintWrap::new(0x2000000);

// 全局访问接口
// pub fn init_clint(base: usize) {
//     CLINT.call_once(|| SifiveClintWrap::new(base));
// }

// pub fn get_clint() -> &'static SifiveClintWrap {
//     println!("Hello, world!15");
//     CLINT.get().expect("CLINT not initialized")
// }

// pub fn init_clint(base: usize) {
//     let _ = CLINT.init(SifiveClintWrap::new(base));
// }
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
                // Handle Msoft
                // Trap::Interrupt(Interrupt::MachineSoft) => {
                //     save_regs(&mut ctx);
                //     handler::msoft_handler(ctx)
                // }
                // Handle MTimer
                Trap::Interrupt(Interrupt::MachineTimer) => {
                    // 会导致中断委托给S态，因而在embassy这里应该不做处理？
                    crate::time_driver::timer_interrupt_handler();

                    save_regs(&mut ctx);
                    ctx.restore()
                }
                // Handle SBI calls
                // Trap::Exception(Exception::SupervisorEnvCall) => {
                //     handler::sbi_call_handler(ctx, a1, a2, a3, a4, a5, a6, a7)
                // }
                // Handle illegal instructions
                // Trap::Exception(Exception::IllegalInstruction) => {
                //     if mstatus::read().mpp() == mstatus::MPP::Machine {
                //         panic!("Cannot handle illegal instruction exception from M-MODE");
                //     }
                //     save_regs(&mut ctx);
                //     ctx.continue_with(handler::illegal_instruction_handler, ())
                // }
                // Trap::Exception(Exception::LoadMisaligned) => {
                //     save_regs(&mut ctx);
                //     ctx.continue_with(handler::load_misaligned_handler, ())
                // }
                // Trap::Exception(Exception::StoreMisaligned) => {
                //     save_regs(&mut ctx);
                //     ctx.continue_with(handler::store_misaligned_handler, ())
                // }
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

// #[inline]
// unsafe fn stack_top() -> usize {
//     unsafe {
//         let stack_ptr = HART0_STACK.as_ptr() as usize;
//         stack_ptr + STACK_SIZE
//     }
// }

const STACK_SIZE: usize = 16 * 1024;

// #[repr(C, align(128))]
#[unsafe(link_section = ".bss.stack")]
// static mut HART0_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut HART0_STACK: Stack = Stack([0; STACK_SIZE]);

#[repr(C, align(128))]
pub(crate) struct Stack([u8; STACK_SIZE]);

impl Stack {
    // #[inline]
    // pub fn hart_context(&mut self) -> &mut HartContext {
    //     unsafe { &mut *self.0.as_mut_ptr().cast() }
    // }

    /// Initializes stack for trap handling.
    /// - Sets up hart context.
    /// - Creates and loads FreeTrapStack with the stack range.
    fn load_as_stack(&'static mut self) {
        // let hart = self.hart_context();
        // let context_ptr = hart.context_ptr();
        // hart.init();

        // Get stack memory range.
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
    init_gpio_as_output(GPIO_BASE, 55);
    set_gpio_output(GPIO_BASE, 55, true);

    // let log_result = Logger::init();

    // match log_result {
    //     Ok(_) => println!("ok了"),
    //     Err(e) => println!("出啥事了:{:?}", e),
    // }

    println!("可以了么？");

    // println!("Hello, world!");

    // let clint = CLINT.init(SifiveClintWrap::new(0x2000000));
    //
    // init_clint(0x2000000);
    // println!("Hello, world!0");

    time_driver::init();
    // println!("Hello, world!1");

    // unsafe { CLINT = Some(SifiveClintWrap::new(0x2000000)) };
    // //u-boot/arch/riscv/dts/jh7110.dtsi, clint 0x2000000
    // // CLINT.init(SifiveClintWrap::new(0x2000000));
    // // let clint = CLINT.init(Mutex::new(SifiveClintWrap::new(0x2000000)));

    // // time_driver::init();
    // info!("init 0 time");

    // let misa = riscv::register::misa::read();
    // info!("hart 0 MISA: ---{:?}---", misa);

    // let misa_val: usize;
    // unsafe {
    //     core::arch::asm!("csrr {}, misa", out(reg) misa_val);
    // }
    // info!("hart 0 MISA: -----0x{:x}-----", misa_val);
    #[allow(static_mut_refs)]
    unsafe {
        HART0_STACK.load_as_stack()
    };
    // println!("Hello, world!2");

    unsafe { mtvec::write(fast_trap::trap_entry as _, mtvec::TrapMode::Direct) };
    // println!("Hello, world!3");

    let executor_new = Executor::new();
    // println!("Hello, world!4");

    // init_gpio_as_output(GPIO_BASE, 55);
    // set_gpio_output(GPIO_BASE, 55, true);
    // let executor = EXECUTOR.init(executor_new);
    unsafe {
        EXECUTOR = Some(executor_new);
        #[allow(static_mut_refs)]
        if let Some(executor) = EXECUTOR.as_mut() {
            println!("Hello, world!5");

            // let mut executor = Executor::new();
            // info!("init 0 executor");
            // info!("init 0 executor 1");

            executor.run(|spawner| {
                println!("Hello, world!6");
                // info!("spawn one ");
                // info!("spawn one 1");

                // let token = run_simple();
                // info!("spawn one 2");
                // match spawner.spawn(token) {
                //     Ok(_) => info!("spawn one ok"),
                //     Err(e) => info!("spawn one err{}", e),
                // }
                // info!("spawn one 20");
                // match spawner.spawn(run()) {
                //     Ok(_) => info!("spawn one ok"),
                //     Err(e) => info!("spawn one err{}", e),
                // }
                // info!("spawn two");
                // spawner.spawn(run_another()).unwrap();
                // info!("spawn three");
                spawner.spawn(run_gpio()).unwrap();
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
    // error!("-----------------------------");
    // error!("mcause:  {:?}", mcause::read().cause());
    // error!("mepc:    {:#018x}", mepc::read());
    // error!("mtval:   {:#018x}", mtval::read());
    // error!("-----------------------------");
    // error!("System shutdown scheduled due to RustSBI panic");
    loop {}
}
