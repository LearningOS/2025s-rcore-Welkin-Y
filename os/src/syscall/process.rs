//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE,
    loader::get_app_data_by_name,
    mm::{copy_to_user, translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    let us = get_time_us();
    let tm = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let tm_ptr = &tm as *const TimeVal as *const u8;
    let tm_size = core::mem::size_of::<TimeVal>();
    let tm_bytes = unsafe { core::slice::from_raw_parts(tm_ptr, tm_size) };
    copy_to_user(current_user_token(), _ts as *mut u8, tm_bytes.to_vec());
    0
}

pub fn sys_mmap(_start: usize, _len: usize, _prot: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
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
    if _prot & !0x7 != 0 || _prot & 0x7 == 0 {
        error!("Invalid prot bits");
        return -1;
    }
    let control_block = current_task().unwrap();
    let mut inner = control_block.inner_exclusive_access();
    let memset = &mut inner.memory_set;
    match memset.mmap(_start, _len, _prot) {
        Ok(_) => 0,
        Err(e) => {
            error!("mmap failed: {}", e);
            -1
        }
    }
}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    // Required _start page aligned
    if _start % PAGE_SIZE != 0 {
        error!("start page is not page aligned in mmap");
        return -1;
    }
    let control_block = current_task().unwrap();
    let mut inner = control_block.inner_exclusive_access();
    let memset = &mut inner.memory_set;
    match memset.munmap(_start, _len) {
        Ok(_) => 0,
        Err(e) => {
            error!("munmap failed: {}", e);
            -1
        }
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

pub fn sys_spawn(_path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// TODO: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    info!("sys_set_priority: prio: {}", _prio);
    if _prio >= 2 {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        inner.priority = _prio as usize;
        return _prio;
    }
    -1
}
