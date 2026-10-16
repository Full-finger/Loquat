// loquat 宿主（Rust）：进程入口。
// 加载 Go 核心 dll（配置中心 / 插件监督 / 消息队列都在里面），
// 收事件，调 Python 业务（pyo3 嵌入），发动作。

use std::error::Error;
use std::ffi::{c_char, CStr, CString};
use std::sync::{mpsc, OnceLock};

use libloading::Library;
use pyo3::prelude::*;
use pyo3::types::PyModule;

static EVENT_TX: OnceLock<mpsc::Sender<Vec<u8>>> = OnceLock::new();

const QA_SCRIPT: &str = "extensions/qa.py";

type EventCallback = unsafe extern "C" fn(data: *const c_char, len: i32);
type StartFn = unsafe extern "C" fn(config_path: *const c_char, cb: EventCallback) -> i32;
type SendActionFn =
    unsafe extern "C" fn(action: *const c_char, params: *const c_char, timeout_ms: i32) -> *mut c_char;
type FreeFn = unsafe extern "C" fn(ptr: *mut c_char);
type StopFn = unsafe extern "C" fn();

// Go 核心的事件回调：只拷贝字节进通道就返回，别在回调里干重活。
extern "C" fn on_event(data: *const c_char, len: i32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) };
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(bytes.to_vec());
    }
}

struct CoreDll {
    _lib: Library, // 必须保持存活，导出的函数指针才有效
    start: StartFn,
    send_action: SendActionFn,
    free: FreeFn,
    _stop: StopFn,
}

impl CoreDll {
    unsafe fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let lib = Library::new(path)?;
        let start = *lib.get::<StartFn>(b"LoquatStart")?;
        let send_action = *lib.get::<SendActionFn>(b"LoquatSendAction")?;
        let free = *lib.get::<FreeFn>(b"LoquatFree")?;
        let stop = *lib.get::<StopFn>(b"LoquatStop")?;
        Ok(Self {
            _lib: lib,
            start,
            send_action,
            free,
            _stop: stop,
        })
    }

    fn start(&self, config_path: &str) -> Result<(), Box<dyn Error>> {
        let cfg = CString::new(config_path)?;
        let rc = unsafe { (self.start)(cfg.as_ptr(), on_event) };
        if rc != 0 {
            return Err(format!("LoquatStart 返回 {rc}（0=成功，1=已启动，2=启动失败）").into());
        }
        Ok(())
    }

    // 返回 NapCat 应答的 JSON（失败时含 error 字段）。调用方无需释放内存。
    fn send_action(
        &self,
        action: &str,
        params: &str,
        timeout_ms: i32,
    ) -> Result<String, Box<dyn Error>> {
        let a = CString::new(action)?;
        let p = CString::new(params)?;
        let ptr = unsafe { (self.send_action)(a.as_ptr(), p.as_ptr(), timeout_ms) };
        let out = if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
        };
        if !ptr.is_null() {
            unsafe { (self.free)(ptr) };
        }
        Ok(out)
    }
}

// Python 业务：加载 qa.py，转发 handle 调用。
struct Business {
    module: Py<PyModule>,
}

impl Business {
    fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("读业务脚本失败 {path}: {e}"))?;
        Python::attach(|py| {
            let file = CString::new(path)?;
            let name = CString::new("qa")?;
            let code = CString::new(source)?;
            let module = PyModule::from_code(py, &code, &file, &name)?;
            Ok(Self { module: module.into() })
        })
    }

    // 调 qa.handle(text) -> list[str]
    fn handle(&self, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        Python::attach(|py| {
            let replies = self.module.bind(py).call_method1("handle", (text,))?;
            Ok(replies.extract()?)
        })
    }
}

// 从事件 JSON 里取出 MVP 关心的字段。
struct Incoming {
    message_type: String,
    chat_id: i64,
    text: String,
}

fn parse_incoming(payload: &[u8]) -> Option<Incoming> {
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    if v.get("post_type")?.as_str()? != "message" {
        return None;
    }
    let message_type = v.get("message_type")?.as_str()?.to_string();
    let chat_id = match message_type.as_str() {
        "private" => v.get("user_id")?.as_i64()?,
        "group" => v.get("group_id")?.as_i64()?,
        _ => return None,
    };
    Some(Incoming {
        message_type,
        chat_id,
        text: v.get("raw_message")?.as_str()?.to_string(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let dll = unsafe { CoreDll::load("bin/core.dll")? };
    println!("[rust] core.dll 已加载");

    let business = Business::load(QA_SCRIPT)?;
    println!("[rust] 业务已加载: {QA_SCRIPT}");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    EVENT_TX.set(tx).expect("事件通道只能初始化一次");

    dll.start("config/loquat.yaml")?;
    println!("[rust] 核心已启动，等待事件（Ctrl+C 退出）");

    for payload in rx {
        let Some(msg) = parse_incoming(&payload) else {
            continue;
        };
        println!("[rust] 消息: [{}] {}", msg.message_type, msg.text);

        let replies = match business.handle(&msg.text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[rust] 业务处理出错: {e}");
                continue;
            }
        };

        let action = match msg.message_type.as_str() {
            "private" => "send_private_msg",
            "group" => "send_group_msg",
            _ => continue,
        };
        for reply in replies {
            let params = if action == "send_private_msg" {
                serde_json::json!({ "user_id": msg.chat_id, "message": reply })
            } else {
                serde_json::json!({ "group_id": msg.chat_id, "message": reply })
            };
            let resp = dll.send_action(action, &params.to_string(), 15_000)?;
            println!("[rust] 已回复: {reply} resp={resp}");
        }
    }
    Ok(())
}
