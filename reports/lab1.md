# LAB 1 Reports

## Implementation Introduction

- Implement trace system call by pattern matching `_trace_request` in [`syscall module`](../os/src/syscall/process.rs).

- Add a 2D `usize` array named `syscall_count` with size (512, 256) to
`TaskMananger` class to trace number of system calls called by a task
in [`task module`](../os/src/task/mod.rs). Implement incrementation and getter methods.

- Modify `sys_call` method in [`syscall module`](../os/src/syscall/mod.rs) for
incrementing the trace array for the current task. The trace array is
indexed by the system call number and the task ID.

## 荣誉准则

我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。
我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。
我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。
我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。
我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
