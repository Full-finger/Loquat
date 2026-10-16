//! Go 核心 dll 的 FFI 封装：加载、符号绑定、调用与生命周期。
//!
//! 生命周期注意：Go c-shared 的 dll 绝不能 FreeLibrary——Go 运行时的
//! 线程永不停止，卸载代码段等于让线程踩空（段错误）。所以 Library
//! 泄漏成 'static，进程退出时由操作系统回收。

use std::error::Error;
use std::ffi::{c_char, CStr, CString};

use libloading::Library;

/// 事件回调的 C ABI 签名，核心每次上报事件调用一次。
pub type EventCallback = unsafe extern "C" fn(data: *const c_char, len: i32);

type StartFn = unsafe extern "C" fn(config_path: *const c_char, cb: EventCallback) -> i32;
type SendActionFn =
    unsafe extern "C" fn(action: *const c_char, params: *const c_char, timeout_ms: i32) -> *mut c_char;
type ReloadPluginFn = unsafe extern "C" fn(name: *const c_char) -> i32;
type StatusFn = unsafe extern "C" fn() -> *mut c_char;
type FreeFn = unsafe extern "C" fn(ptr: *mut c_char);
type StopFn = unsafe extern "C" fn();

pub struct CoreDll {
    // 故意泄漏成 'static：原因见模块注释。
    // 字段本身不再被读，但留着它 dll 才不会在作用域结束时被卸载。
    #[allow(dead_code)]
    lib: &'static Library,
    start: StartFn,
    send_action: SendActionFn,
    reload_plugin: ReloadPluginFn,
    status: StatusFn,
    free: FreeFn,
    stop: StopFn,
}

impl CoreDll {
    pub unsafe fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let lib: &'static Library = Box::leak(Box::new(Library::new(path)?));
        let start = *lib.get::<StartFn>(b"LoquatStart")?;
        let send_action = *lib.get::<SendActionFn>(b"LoquatSendAction")?;
        let reload_plugin = *lib.get::<ReloadPluginFn>(b"LoquatReloadPlugin")?;
        let status = *lib.get::<StatusFn>(b"LoquatStatus")?;
        let free = *lib.get::<FreeFn>(b"LoquatFree")?;
        let stop = *lib.get::<StopFn>(b"LoquatStop")?;
        Ok(Self {
            lib,
            start,
            send_action,
            reload_plugin,
            status,
            free,
            stop,
        })
    }

    pub fn start(&self, config_path: &str, cb: EventCallback) -> Result<(), Box<dyn Error>> {
        let cfg = CString::new(config_path)?;
        let rc = unsafe { (self.start)(cfg.as_ptr(), cb) };
        if rc != 0 {
            return Err(format!("LoquatStart 返回 {rc}（0=成功，1=已启动，2=启动失败）").into());
        }
        Ok(())
    }

    /// 下发动作，返回 NapCat 应答的 JSON（失败时含 error 字段）。
    /// 返回的 String 已在 Rust 侧接管，调用方无需释放内存。
    pub fn send_action(
        &self,
        action: &str,
        params: &str,
        timeout_ms: i32,
    ) -> Result<String, Box<dyn Error>> {
        let a = CString::new(action)?;
        let p = CString::new(params)?;
        let ptr = unsafe { (self.send_action)(a.as_ptr(), p.as_ptr(), timeout_ms) };
        Ok(take_c_string(ptr, &self.free))
    }

    pub fn reload_plugin(&self, name: &str) -> i32 {
        let n = CString::new(name).expect("插件名不含 NUL");
        unsafe { (self.reload_plugin)(n.as_ptr()) }
    }

    pub fn status(&self) -> String {
        let ptr = unsafe { (self.status)() };
        take_c_string(ptr, &self.free)
    }

    pub fn stop(&self) {
        unsafe { (self.stop)() };
    }
}

// 取走 C 字符串转为 String 并释放。
fn take_c_string(ptr: *mut c_char, free: &FreeFn) -> String {
    let out = if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    };
    if !ptr.is_null() {
        unsafe { free(ptr) };
    }
    out
}
