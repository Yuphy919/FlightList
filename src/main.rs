use actix_files::Files;
use actix_web::{get, App, HttpServer, HttpResponse, Responder};
use serde::Serialize;
use reqwest;
use serde_json::Value;
use chrono::Local;
use regex::Regex;
use actix_web::error::ErrorInternalServerError;

#[get("/api/flights")]
async fn flights_api() -> Result<impl Responder, actix_web::Error> {
    #[derive(Serialize)]
    struct Flight {
        flight_status: String,
        arrival_time: String,
        place_of_departure: String,
        aircraft_code: String,
        terminal: String,
    }

    // データ取得
    let url = "https://tokyo-haneda.com/app_resource/flight/data/int/hdacfarv.json";
    let resp = reqwest::get(url).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(e)
    })?.text().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(e)
    })?;

    let json: Value = serde_json::from_str(&resp).map_err(|e| {
        actix_web::error::ErrorInternalServerError(e)
    })?;

    // 今日の日付
    let today = Local::now().format("%Y/%m/%d").to_string();

    // 正規表現の準備
    let date_re = Regex::new(r"\d{4}/\d{2}/\d{2}").map_err(ErrorInternalServerError)?;
    let time_re = Regex::new(r"\b([01]?\d|2[0-3]):[0-5]\d\b").map_err(ErrorInternalServerError)?;

    let mut rows: Vec<Flight> = Vec::new();

    // フライト情報を処理
    if let Some(flights) = json["flight_info"].as_array() {

        for flight_row in flights {
            let note = flight_row["備考和名称"].as_str().unwrap_or("");

            if note.contains("到着済み") {
                continue;
            }

            let time_str = arrival_time(flight_row["定刻"].as_str().unwrap_or(""),flight_row["変更時刻"].as_str().unwrap_or(""));

            if let Some(date_caps) = date_re.captures(time_str.as_str()) {

                if date_caps.get(0).map(|m| m.as_str()) != Some(&today) {
                    continue;
                }

                rows.push(Flight {
                    flight_status: flight_row["備考訳名称"]["ja"].as_str().unwrap_or("").to_string(),
                    arrival_time: time_re.captures(time_str.as_str())
                        .and_then(|cap| cap.get(0))
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    place_of_departure: flight_row["出発地空港和名称"].as_str().unwrap_or("").to_string(),
                    aircraft_code: flight_row["機種コード"].as_str().unwrap_or("").to_string(),
                    terminal: flight_row["ターミナル区分"].as_str().unwrap_or("").to_string(),
                });

            }
        }

    }

    // 時刻でソート（最初の行はヘッダーなので除外して sort）
    rows.sort_by(|a, b| a.arrival_time.cmp(&b.arrival_time));

    //返却
    Ok(HttpResponse::Ok().json(rows))

}

fn arrival_time(time: &str, change_time: &str) -> String {
    if !change_time.is_empty() {
        change_time.to_string()
    } else {
        time.to_string()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // staticフォルダの中身を `/` で配信
            .service(flights_api)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
