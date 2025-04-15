//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{copy_to_user, translated_byte_buffer};
use crate::task::current_user_token;
use crate::task::{
    change_program_brk, exit_current_and_run_next, get_syscall_count, is_user_readable,
    is_user_writable, mmap, munmap, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let tm = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // println!("tm: {}, {}", buffer[0], buffer[1]);
    let tm_ptr = &tm as *const TimeVal as *const u8;
    let tm_size = core::mem::size_of::<TimeVal>();
    let tm_bytes = unsafe { core::slice::from_raw_parts(tm_ptr, tm_size) };
    copy_to_user(current_user_token(), _ts as *mut u8, tm_bytes.to_vec());
    0
}

#[allow(clippy::vec_init_then_push)]
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            if !is_user_readable(_id) {
                return -1;
            }
            let buffers = translated_byte_buffer(current_user_token(), _id as *const u8, 1);
            info!("Trace read: {:?} from {:x}", buffers[0], _id);
            unsafe { core::ptr::read_volatile(buffers[0].as_ptr()) as isize }
        }
        1 => {
            if !is_user_writable(_id) {
                return -1;
            }
            let data_ptr = &_data as *const usize as *const u8;
            let data_size = core::mem::size_of::<usize>();
            let data_bytes = unsafe { core::slice::from_raw_parts(data_ptr, data_size) };
            info!("Trace write: {} to {}", _data, _id);
            copy_to_user(current_user_token(), _id as *mut u8, data_bytes.to_vec());
            0
        }
        2 => get_syscall_count(_id) as isize,
        _ => -1,
    }
}

pub fn sys_mmap(_start: usize, _len: usize, _prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    // Required _start page aligned
    info!(
        "sys_mmap: start: {:#x}, len: {}, prot: {:b}",
        _start, _len, _prot
    );
    if _start % PAGE_SIZE != 0 {
        error!("start page is not page aligned in mmap");
        return -1;
    }
    // Rest of _prot must be 0
    if _prot & !0x7 != 0 {
        error!("Invalid prot bits");
        return -1;
    }
    // Meaningless memory
    if _prot & 0x7 == 0 {
        error!("Invalid prot bits");
        return -1;
    }
    match mmap(_start, _len, _prot) {
        Ok(_) => 0,
        Err(str) => {
            error!("{}", str);
            -1
        }
    }
}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    // Required _start page aligned
    if _start % PAGE_SIZE != 0 {
        error!("start page is not page aligned in mmap");
        return -1;
    }
    match munmap(_start, _len) {
        Ok(_) => 0,
        Err(str) => {
            error!("{}", str);
            -1
        }
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
