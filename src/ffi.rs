//! Go 核心 dll 的 FFI 封装：加载、符号绑定、调用与生命周期。
//!
//! 生命周期注意：Go c-shared 的 dll 绝不能 FreeLibrary——Go 运行时的
//! 线程永不停止，卸载代码段等于让线程踩空（段错误）。所以 Library
//! 泄漏成 'static，进程退出时由操作系统回收。

use std::error::Error;
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, OnceLock};

use libloading::Library;

/// 事件回调的 C ABI 签名，核心每次上报事件调用一次。
pub type EventCallback = unsafe extern "C" fn(data: *const c_char, len: i32);

type StartFn = unsafe extern "C" fn(config_path: *const c_char, cb: EventCallback) -> i32;
type SendActionFn = unsafe extern "C" fn(
    platform: *const c_char,
    action: *const c_char,
    params: *const c_char,
    timeout_ms: i32,
) -> *mut c_char;
type SendMessageFn = unsafe extern "C" fn(
    platform: *const c_char,
    chat_kind: *const c_char,
    chat_id: *const c_char,
    segments: *const c_char,
    timeout_ms: i32,
) -> *mut c_char;
type InjectEventFn = unsafe extern "C" fn(
    platform: *const c_char,
    text: *const c_char,
    at_bot: i32,
    mode: *const c_char,
) -> *mut c_char;
type AdaptersConfigFn = unsafe extern "C" fn() -> *mut c_char;
type ApplyAdaptersConfigFn = unsafe extern "C" fn(config_json: *const c_char) -> *mut c_char;
type ReloadPluginFn = unsafe extern "C" fn(name: *const c_char) -> i32;
type StatusFn = unsafe extern "C" fn() -> *mut c_char;
type SystemStatusFn = unsafe extern "C" fn() -> *mut c_char;
type LogPipeFn = unsafe extern "C" fn() -> usize;
type FreeFn = unsafe extern "C" fn(ptr: *mut c_char);
type StopFn = unsafe extern "C" fn();

static CORE_DLL: OnceLock<Arc<CoreDll>> = OnceLock::new();

/// 把核心实例登记为全局能力源，供注入 Python 的桥函数使用。
pub fn set_core_dll(dll: Arc<CoreDll>) {
    let _ = CORE_DLL.set(dll);
}

pub(crate) fn core_dll() -> Option<&'static Arc<CoreDll>> {
    CORE_DLL.get()
}

pub struct CoreDll {
    // 故意泄漏成 'static：原因见模块注释。
    // 字段本身不再被读，但留着它 dll 才不会在作用域结束时被卸载。
    #[allow(dead_code)]
    lib: &'static Library,
    start: StartFn,
    send_action: SendActionFn,
    send_message: SendMessageFn,
    inject_event: InjectEventFn,
    adapters_config: AdaptersConfigFn,
    apply_adapters_config: ApplyAdaptersConfigFn,
    reload_plugin: ReloadPluginFn,
    status: StatusFn,
    system_status: SystemStatusFn,
    log_pipe: LogPipeFn,
    free: FreeFn,
    stop: StopFn,
}

// 安全性：字段只有 'static 库引用和函数指针；方法都只是经函数指针调用，
// Go 侧各导出自身并发安全，允许 Arc 跨线程共享（Python 桥与主循环同时调用）。
unsafe impl std::marker::Send for CoreDll {}
unsafe impl std::marker::Sync for CoreDll {}

impl CoreDll {
    pub unsafe fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        // 126（找不到模块）多半是启动目录不对：宿主用相对路径找 bin/、config/、
        // extensions/、webui/dist，报错时带上当前目录，方便定位
        let lib: &'static Library = Box::leak(Box::new(Library::new(path).map_err(|e| {
            let cwd = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<未知>".into());
            format!("加载 {path} 失败: {e}（宿主按相对路径找产物；当前目录: {cwd}，请在项目根目录启动）")
        })?));
        let start = *lib.get::<StartFn>(b"LoquatStart")?;
        let send_action = *lib.get::<SendActionFn>(b"LoquatSendAction")?;
        let send_message = *lib.get::<SendMessageFn>(b"LoquatSendMessage")?;
        let inject_event = *lib.get::<InjectEventFn>(b"LoquatInjectEvent")?;
        let adapters_config = *lib.get::<AdaptersConfigFn>(b"LoquatAdaptersConfig")?;
        let apply_adapters_config =
            *lib.get::<ApplyAdaptersConfigFn>(b"LoquatApplyAdaptersConfig")?;
        let reload_plugin = *lib.get::<ReloadPluginFn>(b"LoquatReloadPlugin")?;
        let status = *lib.get::<StatusFn>(b"LoquatStatus")?;
        let system_status = *lib.get::<SystemStatusFn>(b"LoquatSystemStatus")?;
        let log_pipe = *lib.get::<LogPipeFn>(b"LoquatLogPipe")?;
        let free = *lib.get::<FreeFn>(b"LoquatFree")?;
        let stop = *lib.get::<StopFn>(b"LoquatStop")?;
        Ok(Self {
            lib,
            start,
            send_action,
            send_message,
            inject_event,
            adapters_config,
            apply_adapters_config,
            reload_plugin,
            status,
            system_status,
            log_pipe,
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
        platform: &str,
        action: &str,
        params: &str,
        timeout_ms: i32,
    ) -> Result<String, Box<dyn Error>> {
        let pl = CString::new(platform)?;
        let a = CString::new(action)?;
        let p = CString::new(params)?;
        let ptr = unsafe { (self.send_action)(pl.as_ptr(), a.as_ptr(), p.as_ptr(), timeout_ms) };
        Ok(take_c_string(ptr, &self.free))
    }

    /// 统一发送：段列表（protojson 形态的 JSON 数组）交核心翻译成平台消息。
    /// 成功返回 {"messageId": "..."}，失败返回 {"error": "..."}。
    pub fn send_message(
        &self,
        platform: &str,
        chat_kind: &str,
        chat_id: &str,
        segments_json: &str,
        timeout_ms: i32,
    ) -> Result<String, Box<dyn Error>> {
        let pl = CString::new(platform)?;
        let k = CString::new(chat_kind)?;
        let id = CString::new(chat_id)?;
        let s = CString::new(segments_json)?;
        let ptr = unsafe {
            (self.send_message)(pl.as_ptr(), k.as_ptr(), id.as_ptr(), s.as_ptr(), timeout_ms)
        };
        Ok(take_c_string(ptr, &self.free))
    }

    /// 适配器配置（JSON）：当前配置 + 各平台运行状态。
    pub fn adapters_config(&self) -> String {
        let ptr = unsafe { (self.adapters_config)() };
        take_c_string(ptr, &self.free)
    }

    /// 应用适配器配置：写盘 + 热应用。成功返回说明文本，失败含 error 字段。
    pub fn apply_adapters_config(&self, config_json: &str) -> Result<String, Box<dyn Error>> {
        let c = CString::new(config_json)?;
        let ptr = unsafe { (self.apply_adapters_config)(c.as_ptr()) };
        Ok(take_c_string(ptr, &self.free))
    }

    /// 注入一条沙箱消息（WebUI 沙箱选项卡的输入通路）。
    pub fn inject_event(&self, platform: &str, text: &str, at_bot: bool, mode: &str) -> Result<String, Box<dyn Error>> {
        let pl = CString::new(platform)?;
        let t = CString::new(text)?;
        let m = CString::new(mode)?;
        let ptr = unsafe { (self.inject_event)(pl.as_ptr(), t.as_ptr(), at_bot as i32, m.as_ptr()) };
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

    /// 系统状态 JSON（宿主机指标 + 核心自身状态），由 Go 核心采集。
    pub fn system_status(&self) -> Result<String, Box<dyn Error>> {
        let ptr = unsafe { (self.system_status)() };
        Ok(take_c_string(ptr, &self.free))
    }

    /// 核心日志管道的读端句柄（Windows HANDLE）；0 表示不可用。
    /// 须在 start 之前调用：它是首次 FFI，顺带完成 Go 运行时初始化。
    pub fn log_pipe(&self) -> usize {
        unsafe { (self.log_pipe)() }
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
