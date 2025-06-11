// 详见u-boot/arch/riscv/include/asm/arch-jh7110/gpio.h
// GPIO 控制器基地址
pub const GPIO_BASE: usize = 0x13040000;
const GPIOA_BASE: usize = 0x17020000;

// 基于 U-Boot 的准确寄存器偏移
const GPIO_DOEN: usize = 0x0;
const GPIO_DOUT: usize = 0x40;
const GPIO_DIN: usize = 0x80;
const GPIO_CONFIG: usize = 0x120;

// 掩码定义
const GPIO_DOEN_MASK: u32 = 0x3f;
const GPIO_DOUT_MASK: u32 = 0x7f;
const GPIO_DIN_MASK: u32 = 0x7f;

// 辅助宏的 Rust 实现
fn gpio_offset(gpio: u32) -> usize {
    ((gpio >> 2) << 2) as usize
}

fn gpio_shift(gpio: u32) -> u32 {
    (gpio & 0x3) << 3
}

// 修改寄存器位的辅助函数
fn clrsetbits_le32(addr: *mut u32, clr_mask: u32, set_mask: u32) {
    let current = unsafe { addr.read_volatile() };
    let new_value = (current & !clr_mask) | set_mask;
    unsafe { addr.write_volatile(new_value) };
}

// 基于 U-Boot 宏的 GPIO 操作函数
unsafe fn sys_iomux_doen(gpio_base: usize, gpio: u32, oen: u32) {
    let addr = (gpio_base + gpio_offset(gpio)) as *mut u32;
    let shift = gpio_shift(gpio);
    clrsetbits_le32(addr, GPIO_DOEN_MASK << shift, oen << shift);
}

unsafe fn sys_iomux_dout(gpio_base: usize, gpio: u32, gpo: u32) {
    let addr = (gpio_base + GPIO_DOUT + gpio_offset(gpio)) as *mut u32;
    let shift = gpio_shift(gpio);
    clrsetbits_le32(
        addr,
        GPIO_DOUT_MASK << shift,
        (gpo & GPIO_DOUT_MASK) << shift,
    );
}

fn sys_iomux_din_read(gpio_base: usize, gpio: u32) -> bool {
    let addr = (gpio_base + GPIO_DIN + ((gpio >> 5) * 4) as usize) as *mut u32;
    let value = unsafe { addr.read_volatile() };
    ((value >> (gpio & 0x1F)) & 0x1) != 0
}

// 高级封装函数
pub fn init_gpio_as_output(gpio_base: usize, gpio: u32) {
    // 设置为输出模式 (oen = 0)
    unsafe { sys_iomux_doen(gpio_base, gpio, 0) };
    // 初始输出低电平
    unsafe { sys_iomux_dout(gpio_base, gpio, 0) };
}

pub fn set_gpio_output(gpio_base: usize, gpio: u32, high: bool) {
    let value = if high { 1 } else { 0 };
    unsafe { sys_iomux_dout(gpio_base, gpio, value) };
}

pub unsafe fn toggle_gpio(gpio_base: usize, gpio: u32) {
    static mut GPIO_STATES: [bool; 64] = [false; 64];

    if gpio < 64 {
        GPIO_STATES[gpio as usize] = !GPIO_STATES[gpio as usize];
        set_gpio_output(gpio_base, gpio, GPIO_STATES[gpio as usize]);
    }
}
