use std::{cell::LazyCell, collections::HashMap, str::FromStr, sync::Mutex};

thread_local! {
    static CACHE_MAP: LazyCell<Mutex<HashMap<String, String>>> =
        LazyCell::new(|| Mutex::new(HashMap::new()));
}

pub fn add_data(key: &str, value: &str) {
    CACHE_MAP.with(|cache| {
        cache
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string())
    });
}

pub fn get_data<T>(key: &str) -> Option<T>
where
    T: FromStr,
{
    CACHE_MAP.with(|cache| {
        // 在闭包内部获取锁，并直接进行解析
        cache
            .lock()
            .unwrap()
            .get(key)
            .and_then(|data_str| T::from_str(data_str).ok()) // 解析成功返回 Some(T)，失败返回 None
    })
}

pub fn clear() {
    CACHE_MAP.with(|cache| cache.lock().unwrap().clear());
}
