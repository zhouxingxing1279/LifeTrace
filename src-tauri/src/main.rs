#![allow(linker_messages)]
// 发布版作为 GUI 应用运行，不弹出控制台窗口（开发版保留终端便于看日志）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lifetrace_lib::run();
}
