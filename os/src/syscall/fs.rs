//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{copy_to_user, translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if _fd >= inner.fd_table.len() {
        error!("sys_fstat failed: bad file descriptor");
        return -1;
    }
    if let Some(file) = &inner.fd_table[_fd] {
        let file = file.clone();
        drop(inner);
        match file.stat() {
            Ok(stat) => {
                info!("{:?}", stat);
                let st_ptr = &stat as *const Stat as *const u8;
                let st_size = core::mem::size_of::<Stat>();
                let st_bytes = unsafe { core::slice::from_raw_parts(st_ptr, st_size) };
                copy_to_user(current_user_token(), _st as *mut u8, st_bytes.to_vec());
                return 0;
            }
            Err(e) => {
                error!("sys_fstat failed: {}", e);
                return -1;
            }
        }
    }
    -1
}

pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);
    //if old_name == new_name  return -1
    //create _new_name
    //stat.hard_link+1
    let _task = current_task().unwrap();
    let token = current_user_token();
    let old_path = translated_str(token, _old_name);
    let new_path = translated_str(token, _new_name);
    if old_path == new_path {
        error!("Cannot link a file to itself");
        return -1;
    }
    if let Some(os_inode) = open_file(old_path.as_str(), OpenFlags::RDWR) {
        match os_inode.linkat(&new_path) {
            Ok(_) => 0,
            Err(e) => {
                error!("sys_linkat failed: {}", e);
                -1
            }
        }
    } else {
        -1
    }
}

pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_unlinkat", current_task().unwrap().pid.0);
    let _task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, _name);
    if let Some(os_inode) = open_file(path.as_str(), OpenFlags::RDWR) {
        match os_inode.unlinkat(&path) {
            Ok(_) => 0,
            Err(e) => {
                error!("sys_unlinkat failed: {}", e);
                -1
            }
        }
    } else {
        error!("sys_unlinkat failed: {}", path);
        -1
    }
}
