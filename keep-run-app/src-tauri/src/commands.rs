use std::sync::{Arc, Mutex};

use chrono::{DateTime, Datelike, Local};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

// ============= 城市经纬度表 =============
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityCoord {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
}

pub fn city_registry() -> Vec<CityCoord> {
    vec![
        CityCoord { name: "兰州市".into(), lat: 36.0611, lon: 103.8343, tz: "Asia/Shanghai".into() },
        CityCoord { name: "北京市".into(), lat: 39.9042, lon: 116.4074, tz: "Asia/Shanghai".into() },
        CityCoord { name: "上海市".into(), lat: 31.2304, lon: 121.4737, tz: "Asia/Shanghai".into() },
        CityCoord { name: "广州市".into(), lat: 23.1291, lon: 113.2644, tz: "Asia/Shanghai".into() },
        CityCoord { name: "深圳市".into(), lat: 22.5431, lon: 114.0579, tz: "Asia/Shanghai".into() },
        CityCoord { name: "成都市".into(), lat: 30.5728, lon: 104.0668, tz: "Asia/Shanghai".into() },
        CityCoord { name: "重庆市".into(), lat: 29.5630, lon: 106.5516, tz: "Asia/Shanghai".into() },
        CityCoord { name: "杭州市".into(), lat: 30.2741, lon: 120.1551, tz: "Asia/Shanghai".into() },
        CityCoord { name: "南京市".into(), lat: 32.0603, lon: 118.7969, tz: "Asia/Shanghai".into() },
        CityCoord { name: "武汉市".into(), lat: 30.5928, lon: 114.3055, tz: "Asia/Shanghai".into() },
        CityCoord { name: "西安市".into(), lat: 34.3416, lon: 108.9398, tz: "Asia/Shanghai".into() },
        CityCoord { name: "长沙市".into(), lat: 28.2282, lon: 112.9388, tz: "Asia/Shanghai".into() },
    ]
}

pub fn find_city(name: &str) -> Option<CityCoord> {
    let key = name.trim();
    city_registry().into_iter().find(|c| c.name.contains(key) || key.contains(&c.name))
}

// ============= Open-Meteo 天气 API =============
#[derive(Debug, Deserialize)]
struct OpenMeteoResp {
    current: Current,
}
#[derive(Debug, Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: u16,
    wind_speed_10m: f64,
    relative_humidity_2m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    pub weather: String,
    pub temp: i32,
    pub wind_kmh: f64,
    pub humidity: Option<f64>,
    pub city: String,
}

fn describe(code: u16) -> &'static str {
    match code {
        0 => "晴",
        1 => "晴间多云",
        2 => "多云",
        3 => "阴",
        45 | 48 => "雾",
        51 | 53 | 55 => "毛毛雨",
        61 => "小雨",
        63 => "中雨",
        65 => "大雨",
        66 | 67 => "冻雨",
        71 => "小雪",
        73 => "中雪",
        75 => "大雪",
        77 => "雪粒",
        80 => "阵雨",
        81 => "中阵雨",
        82 => "强阵雨",
        85 => "阵雪",
        86 => "强阵雪",
        95 => "雷暴",
        96 | 99 => "雷暴伴冰雹",
        _ => "未知",
    }
}

pub async fn fetch_weather(client: &Client, city: &CityCoord) -> anyhow::Result<Weather> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code,wind_speed_10m,relative_humidity_2m&timezone={}",
        city.lat, city.lon, city.tz
    );
    let resp: OpenMeteoResp = client.get(&url).send().await?.json().await?;
    Ok(Weather {
        weather: describe(resp.current.weather_code).to_string(),
        temp: resp.current.temperature_2m.round() as i32,
        wind_kmh: resp.current.wind_speed_10m,
        humidity: resp.current.relative_humidity_2m,
        city: city.name.clone(),
    })
}

// ============= 配置持久化 =============
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub city: String,
    pub user: String,
    pub weather_auto: bool,
    pub last_weather: Option<Weather>,
    pub last_fetch_at: Option<DateTime<Local>>,
    pub distance: f32,
    pub pace_secs: u32,
    pub shape: String,
    pub style: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            city: "兰州市".into(),
            user: "儒雅的_Keeperze4b".into(),
            weather_auto: true,
            last_weather: None,
            last_fetch_at: None,
            distance: 1.7,
            pace_secs: 373,
            shape: "loop".into(),
            style: "city".into(),
        }
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    let dir = directories::ProjectDirs::from("app", "keep-run", "KeepRun")?;
    let p = dir.config_dir().join("config.json");
    std::fs::create_dir_all(dir.config_dir()).ok()?;
    Some(p)
}

pub fn load_config() -> AppConfig {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &AppConfig) -> anyhow::Result<()> {
    let p = config_path().ok_or_else(|| anyhow::anyhow!("cannot resolve config path"))?;
    std::fs::write(p, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

// ============= State =============
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub client: Client,
}

// ============= Commands =============
#[tauri::command]
pub async fn list_cities() -> Result<Vec<CityCoord>, String> {
    Ok(city_registry())
}

#[tauri::command]
pub async fn get_now() -> Result<serde_json::Value, String> {
    let now = Local::now();
    let weekday = match now.weekday() {
        chrono::Weekday::Mon => "周一",
        chrono::Weekday::Tue => "周二",
        chrono::Weekday::Wed => "周三",
        chrono::Weekday::Thu => "周四",
        chrono::Weekday::Fri => "周五",
        chrono::Weekday::Sat => "周六",
        chrono::Weekday::Sun => "周日",
    };
    Ok(serde_json::json!({
        "iso": now.to_rfc3339(),
        "date": now.format("%Y-%m-%d").to_string(),
        "time": now.format("%H:%M").to_string(),
        "hms": now.format("%H:%M:%S").to_string(),
        "weekday": weekday,
        "unix": now.timestamp()
    }))
}

#[tauri::command]
pub async fn get_weather(
    state: tauri::State<'_, AppState>,
    city: Option<String>,
) -> Result<Weather, String> {
    let cfg = state.config.lock().unwrap().clone();
    let name = city.unwrap_or(cfg.city);
    let coord = find_city(&name).ok_or_else(|| format!("未找到城市: {}", name))?;
    let w = fetch_weather(&state.client, &coord)
        .await
        .map_err(|e| e.to_string())?;
    let mut c = state.config.lock().unwrap();
    c.last_fetch_at = Some(Local::now());
    c.city = coord.name.clone();
    c.last_weather = Some(w.clone());
    let _ = save_config(&c);
    Ok(w)
}

#[tauri::command]
pub async fn refresh_weather(state: tauri::State<'_, AppState>) -> Result<Weather, String> {
    let cfg = state.config.lock().unwrap().clone();
    get_weather(state, Some(cfg.city)).await
}

#[tauri::command]
pub async fn load_my_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_my_config(
    state: tauri::State<'_, AppState>,
    cfg: AppConfig,
) -> Result<(), String> {
    let mut s = state.config.lock().unwrap();
    *s = cfg.clone();
    save_config(&s).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn random_seed() -> Result<String, String> {
    let mut buf = [0u8; 6];
    rand::rng().fill_bytes(&mut buf);
    const ALPHA: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";
    const DIGIT: &[u8; 10] = b"0123456789";
    let pick = |arr: &[u8], v: u8| arr[(v as usize) % arr.len()] as char;
    Ok(format!(
        "keep-{}{}{}{}{}{}",
        pick(ALPHA, buf[0]),
        pick(ALPHA, buf[1]),
        pick(DIGIT, buf[2]),
        pick(ALPHA, buf[3]),
        pick(DIGIT, buf[4]),
        pick(ALPHA, buf[5]),
    ))
}

// ============= 后台定时任务 =============
pub fn spawn_clock(app: tauri::AppHandle) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            tick.tick().await;
            let now = Local::now();
            let weekday = match now.weekday() {
                chrono::Weekday::Mon => "周一",
                chrono::Weekday::Tue => "周二",
                chrono::Weekday::Wed => "周三",
                chrono::Weekday::Thu => "周四",
                chrono::Weekday::Fri => "周五",
                chrono::Weekday::Sat => "周六",
                chrono::Weekday::Sun => "周日",
            };
            let payload = serde_json::json!({
                "date": now.format("%Y-%m-%d").to_string(),
                "time": now.format("%H:%M:%S").to_string(),
                "weekday": weekday,
                "unix": now.timestamp()
            });
            let _ = app.emit("clock-tick", payload);
        }
    });
}

pub fn spawn_weather_refresh(app: tauri::AppHandle, state: AppState) {
    tokio::spawn(async move {
        // 启动先刷新一次
        {
            let cfg = state.config.lock().unwrap().clone();
            if cfg.weather_auto {
                if let Some(city) = find_city(&cfg.city) {
                    if let Ok(w) = fetch_weather(&state.client, &city).await {
                        let mut c = state.config.lock().unwrap();
                        c.last_weather = Some(w.clone());
                        c.last_fetch_at = Some(Local::now());
                        let _ = save_config(&c);
                        let _ = app.emit("weather-updated", serde_json::to_value(&w).ok());
                    }
                }
            }
        }
        // 每 30 分钟自动刷新一次天气
        let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        loop {
            tick.tick().await;
            let cfg = state.config.lock().unwrap().clone();
            if !cfg.weather_auto {
                continue;
            }
            if let Some(city) = find_city(&cfg.city) {
                if let Ok(w) = fetch_weather(&state.client, &city).await {
                    let mut c = state.config.lock().unwrap();
                    c.last_weather = Some(w.clone());
                    c.last_fetch_at = Some(Local::now());
                    let _ = save_config(&c);
                    let _ = app.emit("weather-updated", serde_json::to_value(&w).ok());
                }
            }
        }
    });
}

// ============= 入口 =============
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = load_config();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("KeepRun/0.1 (+app.keep.run)")
        .build()
        .expect("build reqwest client");
    let state = AppState {
        config: Arc::new(Mutex::new(cfg)),
        client,
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_cities,
            get_now,
            get_weather,
            refresh_weather,
            load_my_config,
            save_my_config,
            random_seed,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let st = app.state::<AppState>();
            spawn_clock(handle.clone());
            spawn_weather_refresh(
                handle,
                AppState {
                    config: st.config.clone(),
                    client: st.client.clone(),
                },
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
